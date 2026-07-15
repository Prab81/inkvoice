"""AI Rewrite — the local-LLM slice of STORY-004 (Auto-Edit), exposed as its
own writing mode alongside Dictation/Email/Message rather than folded into
the rule-based cleanup pipeline.

Why a separate mode instead of "always on": the rule-based passes in
cleanup.py/mode_format.py are deterministic and near-instant; an LLM call
is neither — even warm, it costs a few hundred ms to a few seconds, and it
can in principle change wording in ways the user didn't say. Making it an
explicit, user-selected mode (like Wispr Flow's and FluidVoice's own
enhancement layers, which this is InkVoice's answer to) keeps the default
experience fast and predictable, and puts the risk of "the AI changed my
words" only where the user opted into it.

Talks to a local Ollama instance (127.0.0.1:11434) — already running on
the dev machine with several models pulled, so no new install/download.
`qwen3:8b` was chosen over the larger pulled models (qwen2.5:14b,
qwen3:30b, gpt-oss:20b) purely for latency: this is a proofreading pass,
not a reasoning task, and warm-model latency measured at ~120-600ms for
sentence-length input vastly beats any real gain from a bigger model here.
`"think": false` disables Qwen3's chain-of-thought — without it the model
spends time on hidden reasoning tokens the user would just be waiting on.

Same resilience philosophy as the rest of the sidecar (save_debug_utterance,
log_latency, log_history): a failure here must never take down real
dictation. Any error, timeout, or unreachable Ollama falls back to the
text unchanged — Rewrite mode degrades to Dictation mode, silently, rather
than blocking the user's transcript.
"""
import json
import urllib.error
import urllib.request

OLLAMA_URL = "http://127.0.0.1:11434/api/generate"
MODEL = "qwen3:8b"
TIMEOUT_S = 12.0  # generous: covers a cold model load; falls back after

_SYSTEM_PROMPT = (
    "You correct grammar, word order, and awkward phrasing in dictated "
    "speech. Preserve the speaker's meaning, tone, and every fact — do not "
    "summarize, add information, or change the register (casual stays "
    "casual, formal stays formal). Output ONLY the corrected text, with no "
    "preamble, quotes, or explanation. If the text is already correct, "
    "return it unchanged."
)


def rewrite(text: str) -> str:
    """Grammar/fluency pass via the local Ollama model. Returns `text`
    unchanged on any failure (connection refused, timeout, malformed
    response, empty output) — never raises."""
    if not text or not text.strip():
        return text
    payload = {
        "model": MODEL,
        "system": _SYSTEM_PROMPT,
        "prompt": text,
        "stream": False,
        "think": False,
        "options": {"temperature": 0.3},
    }
    try:
        req = urllib.request.Request(
            OLLAMA_URL,
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=TIMEOUT_S) as resp:
            data = json.loads(resp.read())
        rewritten = data.get("response", "").strip()
        return rewritten if rewritten else text
    except (urllib.error.URLError, TimeoutError, OSError, json.JSONDecodeError, KeyError) as e:
        print(f"ai_rewrite: falling back to unmodified text (non-fatal): {e}", flush=True)
        return text
