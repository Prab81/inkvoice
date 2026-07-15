"""InkVoice ASR sidecar.

Long-running process: loads TWO models once, then serves streaming
transcription over a localhost TCP socket using a JSON-lines protocol. One
client (the shell) at a time.

Dual-model architecture (M2): a single offline model (Parakeet-TDT) redecoding
the growing buffer every ~250ms was the M1/M2 approach, and it worked, but it
has two structural problems found via live testing and a head-to-head spike
against a real streaming model:
  1. Every partial is a full independent re-decode, so already-shown words
     visibly flicker/revise every cycle — it never looks "settled" the way
     live dictation should.
  2. Per-partial compute grows with utterance length (redecoding more audio
     each time), so latency isn't flat.
A real streaming model (cache-aware incremental decoding, e.g. Zipformer)
fixes both: near-zero per-chunk latency, and zero revisions once a token is
emitted (confirmed in spikes/streaming_zipformer). But head-to-head accuracy
testing (spikes/streaming_zipformer/zipformer_accent_eval equivalent) showed
it's measurably worse than Parakeet on names/jargon, and it has no built-in
punctuation/casing at all (LibriSpeech-style all-caps output) — a real
regression if it replaced Parakeet outright.

So: BOTH models run. The streaming model drives live partials (what you see
while talking — smoothness is what matters there, and its accuracy weakness
is transient/self-correcting from the user's perspective since it's not the
committed text). Parakeet still produces the FINAL text once per utterance
(same one-shot call as before) — same accuracy and free punctuation/casing
as before, and actually cheaper now since it's no longer redecoding
periodically during the utterance.

Protocol (one JSON object per line):
  shell -> sidecar:
    {"type": "begin"}                      start a new utterance
    {"type": "audio", "pcm": "<base64>"}   float32 mono 16kHz samples
    {"type": "end", "mode": "dictation"}   finalize current utterance; mode
                                           in {dictation,email,message,rewrite} picks
                                           the writing-mode formatting pass
    {"type": "ping"}
  sidecar -> shell:
    {"type": "ready"}                      models loaded, accepting audio
    {"type": "partial", "text": "..."}     from the streaming model, live
    {"type": "final", "text": "..."}       from Parakeet, once per utterance
    {"type": "pong"}
"""
import base64
import json
import socket
import sys
import time
from pathlib import Path

import numpy as np
import sherpa_onnx
import soundfile as sf

from cleanup import clean_transcript
from dashboard import start_dashboard
from ai_rewrite import rewrite as ai_rewrite
from mode_format import apply_mode
from personal_dictionary import apply_personal_dictionary, load_dictionary

HOST, PORT = "127.0.0.1", 43917
SAMPLE_RATE = 16000

# FOUND LIVE: Parakeet-TDT here runs as a full re-decode, not a cache-aware
# streaming model. Decoding the *entire* session buffer in one shot (the
# original approach) silently degrades to empty/garbled output once the
# buffer gets long (multi-minute sessions) rather than erroring. The
# original fix bounded finalize() to only decode the trailing 20s of
# audio — simple, but it silently dropped everything before that on any
# dictation longer than 20s (found live: 3 real dictations at 20.9s/
# 49.8s/31.8s/42.3s each lost everything but their last ~20s).
#
# Real fix: decode in bounded chunks AS audio arrives and stitch the raw
# per-chunk transcripts together, so no single decode call ever risks the
# degradation zone regardless of total utterance length. Prefer cutting a
# chunk during a detected pause (near-silence) so we don't split a word
# across two decode calls — but never let a chunk grow past
# CHUNK_HARD_MAX_S even mid-speech, to guarantee we stay well clear of the
# ~20s zone where the model itself is known to degrade.
CHUNK_SOFT_TRIGGER_S = 15.0
CHUNK_HARD_MAX_S = 19.0
SILENCE_TAIL_S = 0.3
SILENCE_RMS_THRESHOLD = 0.015

def _repo_root() -> Path:
    # When frozen by PyInstaller, __file__ is inside a temp extraction dir.
    # Use the exe's own directory as the anchor instead; fall back to the
    # source-tree layout (parents[2] of this .py file) for dev runs.
    import sys
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parents[2]

_ROOT = _repo_root()

DEFAULT_OFFLINE_MODEL_DIR = (
    _ROOT / "spikes" / "m0_asr" / "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"
)
DEFAULT_STREAMING_MODEL_DIR = (
    _ROOT / "spikes" / "streaming_zipformer" / "sherpa-onnx-streaming-zipformer-en-2023-06-21"
)
# Chose the 2023-06-21 variant (LibriSpeech+GigaSpeech) over 2023-06-26
# (LibriSpeech only): head-to-head on our own accent/jargon test set and a
# real Bluetooth-mic recording, 06-21 was substantially more accurate
# ("Kubernetes" and "call mom" correct vs. garbled) at the same latency —
# GigaSpeech's broader, noisier training data generalizes better to real
# mic audio than LibriSpeech's clean audiobook narration.

# FOUND LIVE (M2): a garbled transcript ("oooooooose...") couldn't be
# diagnosed after the fact because nothing but the (also lossy) text log
# survived — the raw audio that produced it was gone. Every finalized
# utterance is now dumped to disk (audio + both transcript stages) so a
# future weird output can actually be reproduced and debugged, not just
# guessed at. Capped and rotated since this runs unattended.
DEBUG_AUDIO_DIR = _ROOT / "src" / "sidecar" / "debug_audio"
DEBUG_AUDIO_MAX_FILES = 40

# STORY-002 AC3: latency has been measured extensively via ad-hoc spikes
# during development, but never logged as a standing per-session record —
# this is that. One JSON line per finalized utterance, append-only.
# Deliberately not capped/rotated like debug_audio: these lines are tiny
# (no audio payload, just a handful of numbers), so unlike raw WAV dumps
# there's no meaningful disk-growth concern to guard against yet.
LATENCY_LOG_PATH = _ROOT / "src" / "sidecar" / "latency_log.jsonl"

# STORY-014: durable dictation history (text only — the rotating debug_audio
# dump already covers audio for debugging). One JSON line per finalized
# utterance with both transcript stages, so the dashboard can show a
# raw-vs-cleaned diff and offer copy-raw revert long after the utterance.
HISTORY_PATH = _ROOT / "src" / "sidecar" / "history.jsonl"


def _append_jsonl(path: Path, entry: dict, label: str) -> None:
    try:
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(entry) + "\n")
    except OSError as e:
        # Same philosophy as save_debug_utterance: logging must never take
        # down real dictation.
        print(f"{label} write failed (non-fatal): {e}", flush=True)


def log_latency(entry: dict) -> None:
    _append_jsonl(LATENCY_LOG_PATH, entry, "latency log")


def log_history(entry: dict) -> None:
    _append_jsonl(HISTORY_PATH, entry, "history log")


def save_debug_utterance(samples: np.ndarray, raw_text: str, cleaned_text: str) -> None:
    try:
        DEBUG_AUDIO_DIR.mkdir(exist_ok=True)
        stamp = time.strftime("%Y%m%d-%H%M%S")
        base = DEBUG_AUDIO_DIR / f"{stamp}"
        sf.write(str(base.with_suffix(".wav")), samples, SAMPLE_RATE)
        base.with_suffix(".txt").write_text(
            f"raw:     {raw_text}\ncleaned: {cleaned_text}\n", encoding="utf-8"
        )
        existing = sorted(DEBUG_AUDIO_DIR.glob("*.wav"))
        for old in existing[:-DEBUG_AUDIO_MAX_FILES]:
            old.unlink(missing_ok=True)
            old.with_suffix(".txt").unlink(missing_ok=True)
    except OSError as e:
        # Debug capture must never take down real dictation.
        print(f"debug audio capture failed (non-fatal): {e}", flush=True)


def build_offline_recognizer(model_dir: Path) -> sherpa_onnx.OfflineRecognizer:
    return sherpa_onnx.OfflineRecognizer.from_transducer(
        encoder=str(model_dir / "encoder.int8.onnx"),
        decoder=str(model_dir / "decoder.int8.onnx"),
        joiner=str(model_dir / "joiner.int8.onnx"),
        tokens=str(model_dir / "tokens.txt"),
        num_threads=4,
        sample_rate=SAMPLE_RATE,
        feature_dim=80,
        model_type="nemo_transducer",
    )


def build_streaming_recognizer(model_dir: Path) -> sherpa_onnx.OnlineRecognizer:
    return sherpa_onnx.OnlineRecognizer.from_transducer(
        tokens=str(model_dir / "tokens.txt"),
        encoder=str(model_dir / "encoder-epoch-99-avg-1.int8.onnx"),
        decoder=str(model_dir / "decoder-epoch-99-avg-1.int8.onnx"),
        joiner=str(model_dir / "joiner-epoch-99-avg-1.int8.onnx"),
        num_threads=4,
        sample_rate=SAMPLE_RATE,
        feature_dim=80,
        decoding_method="greedy_search",
    )


def _is_silence(tail: np.ndarray) -> bool:
    if len(tail) == 0:
        return False
    return float(np.sqrt(np.mean(tail**2))) < SILENCE_RMS_THRESHOLD


class UtteranceDecoder:
    def __init__(self, offline_rec: sherpa_onnx.OfflineRecognizer, online_rec: sherpa_onnx.OnlineRecognizer):
        self.offline_rec = offline_rec
        self.online_rec = online_rec
        self.buf = np.zeros(0, dtype=np.float32)  # full utterance, for debug dumps only
        self.chunk_buf = np.zeros(0, dtype=np.float32)  # audio since the last flushed chunk
        self.raw_chunks: list[str] = []  # completed chunks' raw offline-decoded text, in order
        self.online_stream = online_rec.create_stream()
        self.last_partial = ""
        self.t_begin: float | None = None
        self.t_first_partial: float | None = None

    def reset(self):
        self.buf = np.zeros(0, dtype=np.float32)
        self.chunk_buf = np.zeros(0, dtype=np.float32)
        self.raw_chunks = []
        self.online_stream = self.online_rec.create_stream()
        self.last_partial = ""
        self.t_begin = None
        self.t_first_partial = None

    def _decode_offline(self, samples: np.ndarray) -> str:
        try:
            s = self.offline_rec.create_stream()
            s.accept_waveform(SAMPLE_RATE, samples)
            self.offline_rec.decode_stream(s)
            return s.result.text
        except Exception as e:
            # Never fail silently: a bad decode must be visible, not an
            # empty string the caller can't distinguish from "no speech".
            print(f"offline decode error on {len(samples)/SAMPLE_RATE:.1f}s input: {e}",
                  flush=True)
            return ""

    def _flush_chunk(self) -> None:
        """Offline-decode whatever's in chunk_buf and append its raw text
        to raw_chunks, then clear chunk_buf. A no-op on an empty buffer so
        it's safe to call unconditionally from finalize()."""
        if len(self.chunk_buf) == 0:
            return
        text = self._decode_offline(self.chunk_buf)
        if text:
            self.raw_chunks.append(text)
        self.chunk_buf = np.zeros(0, dtype=np.float32)

    def feed(self, samples: np.ndarray) -> str | None:
        """Append audio; stream it through the online model for an instant,
        stable partial (near-zero latency, tokens don't revise — see module
        docstring). Also accumulates chunk_buf for chunked offline decoding
        (see CHUNK_SOFT_TRIGGER_S/CHUNK_HARD_MAX_S above) so long dictations
        get their whole content transcribed, not just a trailing window."""
        self.buf = np.concatenate([self.buf, samples])
        self.chunk_buf = np.concatenate([self.chunk_buf, samples])
        self.online_stream.accept_waveform(SAMPLE_RATE, samples)
        while self.online_rec.is_ready(self.online_stream):
            self.online_rec.decode_stream(self.online_stream)
        partial = self.online_rec.get_result(self.online_stream)

        chunk_len_s = len(self.chunk_buf) / SAMPLE_RATE
        if chunk_len_s >= CHUNK_HARD_MAX_S:
            self._flush_chunk()
        elif chunk_len_s >= CHUNK_SOFT_TRIGGER_S:
            tail = self.chunk_buf[-int(SAMPLE_RATE * SILENCE_TAIL_S):]
            if _is_silence(tail):
                self._flush_chunk()

        if partial == self.last_partial:
            return None
        if self.t_first_partial is None:
            self.t_first_partial = time.monotonic()
        self.last_partial = partial
        return partial

    def finalize(self, mode: str = "dictation") -> tuple[str, str]:
        """Stitch all completed chunks' raw text plus whatever's left in
        chunk_buf into the full utterance text, then run cleanup ONCE over
        the complete stitched text (not per chunk) — filler/self-correction/
        punctuation rules need the whole utterance in view, not a fragment.
        The writing mode (from the shell's "end" message) applies last, on
        top of the fully cleaned text — see mode_format.py.

        Returns (cleaned, raw) — raw is kept for the history log so the
        dashboard can diff what the AI changed and offer revert-to-raw.
        """
        if len(self.buf) == 0:
            self.reset()
            return "", ""
        self._flush_chunk()  # decode the tail — always well under the hard cap
        if self.raw_chunks:
            text = " ".join(self.raw_chunks)
        elif self.last_partial:
            # Belt-and-braces: no chunk produced any text (e.g. a very
            # short utterance) — fall back to the streaming model's last
            # partial rather than emitting nothing.
            text = self.last_partial
        else:
            text = ""
        cleaned = clean_transcript(text)
        # Reloaded fresh each utterance (cheap — a handful of names) so
        # editing personal_dictionary.json takes effect without restarting
        # the sidecar; no hot-reload watcher, just a cheap re-read.
        cleaned = apply_personal_dictionary(cleaned, load_dictionary())
        cleaned = apply_mode(cleaned, mode)
        if mode == "rewrite":
            cleaned = ai_rewrite(cleaned)
        save_debug_utterance(self.buf, text, cleaned)
        self.reset()
        return cleaned, text


def send(conn: socket.socket, obj: dict):
    conn.sendall((json.dumps(obj) + "\n").encode())


def serve(offline_rec: sherpa_onnx.OfflineRecognizer, online_rec: sherpa_onnx.OnlineRecognizer):
    dec = UtteranceDecoder(offline_rec, online_rec)
    # warmup so the first real utterance doesn't pay graph-init cost
    dec._decode_offline(np.zeros(SAMPLE_RATE, dtype=np.float32))
    dec.reset()

    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((HOST, PORT))
    srv.listen(1)
    print(f"listening on {HOST}:{PORT}", flush=True)

    while True:
        conn, _ = srv.accept()
        dec.reset()
        send(conn, {"type": "ready"})
        f = conn.makefile("r", encoding="utf-8")
        try:
            for line in f:
                msg = json.loads(line)
                t = msg["type"]
                if t == "audio":
                    pcm = np.frombuffer(
                        base64.b64decode(msg["pcm"]), dtype=np.float32)
                    partial = dec.feed(pcm)
                    if partial is not None:
                        send(conn, {"type": "partial", "text": partial})
                elif t == "begin":
                    dec.reset()
                    dec.t_begin = time.monotonic()
                elif t == "end":
                    # Capture these before finalize(), which calls reset()
                    # internally and would clear them first.
                    audio_s = len(dec.buf) / SAMPLE_RATE
                    chunk_count = len(dec.raw_chunks) + (1 if len(dec.chunk_buf) else 0)
                    t_begin, t_first_partial = dec.t_begin, dec.t_first_partial
                    t_end_received = time.monotonic()
                    text, raw_text = dec.finalize(mode=msg.get("mode", "dictation"))
                    t_final = time.monotonic()
                    log_latency({
                        "ts": time.time(),
                        "audio_s": round(audio_s, 2),
                        "chunks": chunk_count,
                        "time_to_first_partial_ms": (
                            round((t_first_partial - t_begin) * 1000, 1)
                            if t_begin and t_first_partial else None
                        ),
                        "finalize_ms": round((t_final - t_end_received) * 1000, 1),
                        "total_session_ms": (
                            round((t_final - t_begin) * 1000, 1) if t_begin else None
                        ),
                    })
                    if text or raw_text:
                        log_history({
                            "ts": time.time(),
                            "audio_s": round(audio_s, 2),
                            "raw": raw_text,
                            "cleaned": text,
                        })
                    send(conn, {"type": "final", "text": text})
                elif t == "ping":
                    send(conn, {"type": "pong"})
        except (ConnectionError, json.JSONDecodeError, OSError) as e:
            print(f"client disconnected: {e}", flush=True)
        finally:
            conn.close()


def main():
    offline_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OFFLINE_MODEL_DIR
    online_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_STREAMING_MODEL_DIR
    if not offline_dir.is_dir():
        sys.exit(f"offline model dir not found: {offline_dir}")
    if not online_dir.is_dir():
        sys.exit(f"streaming model dir not found: {online_dir}")
    start_dashboard()
    print("loading models...", flush=True)
    offline_rec = build_offline_recognizer(offline_dir)
    online_rec = build_streaming_recognizer(online_dir)
    serve(offline_rec, online_rec)


if __name__ == "__main__":
    main()
