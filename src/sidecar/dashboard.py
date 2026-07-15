"""Local dashboard for InkVoice — history, personal dictionary, latency.

A tiny stdlib HTTP server (no new dependencies — the package must stay
portable, including to a future Mac port where the Win32 shell won't run
but this will) bound to localhost only, running as a daemon thread inside
the ASR sidecar process. Serves a single-page UI plus a small JSON API
over the sidecar's existing on-disk state:

    GET  /                       dashboard.html
    GET  /api/history            newest-first utterance history (history.jsonl)
    GET  /api/stats              lifetime totals across ALL history (not capped)
    GET  /api/latency            per-utterance latency records (latency_log.jsonl)
    GET  /api/dictionary         {"terms": [...]}
    POST /api/dictionary         {"term": "..."} -> add
    POST /api/dictionary/delete  {"term": "..."} -> remove

Dictionary writes go straight to personal_dictionary.json; the decoder
reloads that file every utterance already, so additions take effect on the
very next dictation with no restart or signaling (STORY-007 AC1's capture
flow, with AC2's add-from-history living in the UI).

Architecture note: this deliberately is NOT the Tauri shell from the
original plan. A localhost page served by the sidecar needs zero new
runtime, works on any OS with a browser, and can be replaced by (or
embedded in) a real shell later — the JSON API is the part worth keeping.
"""
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

DASH_HOST, DASH_PORT = "127.0.0.1", 43918

_HERE = Path(__file__).resolve().parent
HISTORY_PATH = _HERE / "history.jsonl"
LATENCY_PATH = _HERE / "latency_log.jsonl"
DICT_PATH = _HERE / "personal_dictionary.json"
HTML_PATH = _HERE / "dashboard.html"

HISTORY_LIMIT = 500  # newest entries served; the file itself is never truncated


def _read_jsonl(path: Path, limit: int | None = None) -> list[dict]:
    if not path.exists():
        return []
    entries = []
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                continue  # one corrupt line must not hide the rest
    except OSError:
        return []
    if limit is not None:
        entries = entries[-limit:]
    entries.reverse()  # newest first
    return entries


def _lifetime_stats() -> dict:
    """Aggregate over the FULL history file, not the HISTORY_LIMIT window
    the UI displays — lifetime WPM must reflect actual lifetime usage, not
    just whatever's currently loaded on the page."""
    count = 0
    total_words = 0
    total_audio_s = 0.0
    if HISTORY_PATH.exists():
        try:
            for line in HISTORY_PATH.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                count += 1
                total_words += len((entry.get("cleaned") or "").split())
                total_audio_s += entry.get("audio_s") or 0.0
        except OSError:
            pass
    wpm = (total_words / (total_audio_s / 60.0)) if total_audio_s > 0 else 0.0
    return {
        "dictations": count,
        "words": total_words,
        "audio_s": round(total_audio_s, 1),
        "wpm": round(wpm, 1),
    }


def _read_terms() -> list[str]:
    if not DICT_PATH.exists():
        return []
    try:
        data = json.loads(DICT_PATH.read_text(encoding="utf-8"))
        return list(data.get("terms", []))
    except (json.JSONDecodeError, OSError):
        return []


def _write_terms(terms: list[str]) -> None:
    DICT_PATH.write_text(
        json.dumps({"terms": terms}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


class _Handler(BaseHTTPRequestHandler):
    def _json(self, obj, status: int = 200) -> None:
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):  # noqa: N802 (BaseHTTPRequestHandler API)
        if self.path in ("/", "/index.html"):
            try:
                body = HTML_PATH.read_bytes()
            except OSError:
                self._json({"error": "dashboard.html missing"}, 500)
                return
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/api/history":
            self._json(_read_jsonl(HISTORY_PATH, HISTORY_LIMIT))
        elif self.path == "/api/stats":
            self._json(_lifetime_stats())
        elif self.path == "/api/latency":
            self._json(_read_jsonl(LATENCY_PATH))
        elif self.path == "/api/dictionary":
            self._json({"terms": _read_terms()})
        else:
            self._json({"error": "not found"}, 404)

    def do_POST(self):  # noqa: N802
        try:
            length = int(self.headers.get("Content-Length", 0))
            payload = json.loads(self.rfile.read(length) or b"{}")
            term = str(payload.get("term", "")).strip()
        except (json.JSONDecodeError, ValueError):
            self._json({"error": "bad request"}, 400)
            return
        if not term:
            self._json({"error": "empty term"}, 400)
            return

        terms = _read_terms()
        if self.path == "/api/dictionary":
            if term.lower() not in (t.lower() for t in terms):
                terms.append(term)
                _write_terms(terms)
            self._json({"terms": terms})
        elif self.path == "/api/dictionary/delete":
            kept = [t for t in terms if t.lower() != term.lower()]
            if len(kept) != len(terms):
                _write_terms(kept)
            self._json({"terms": kept})
        else:
            self._json({"error": "not found"}, 404)

    def log_message(self, fmt, *args):  # silence per-request stderr noise
        pass


def start_dashboard() -> None:
    """Start the dashboard server on a daemon thread. Never raises — the
    dashboard is a convenience and must not take down dictation."""
    try:
        server = ThreadingHTTPServer((DASH_HOST, DASH_PORT), _Handler)
    except OSError as e:
        print(f"dashboard unavailable (port {DASH_PORT} busy?): {e}", flush=True)
        return
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"dashboard: http://{DASH_HOST}:{DASH_PORT}", flush=True)
