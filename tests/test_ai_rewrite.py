"""AI Rewrite tests (STORY-004 local-LLM slice).

Deliberately does NOT assert on real model output (non-deterministic, and
would make the test suite depend on Ollama being installed/running). What
IS tested and matters: the resilience contract — rewrite() must never
raise and must fall back to the original text on any failure, since a
broken LLM call must not take down real dictation (same philosophy as
save_debug_utterance/log_latency elsewhere in the sidecar).
"""
import json
import sys
import urllib.error
from pathlib import Path
from unittest.mock import patch, MagicMock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src" / "sidecar"))

import ai_rewrite  # noqa: E402


def test_empty_text_short_circuits_without_network_call():
    with patch("ai_rewrite.urllib.request.urlopen") as mock_urlopen:
        assert ai_rewrite.rewrite("") == ""
        assert ai_rewrite.rewrite("   ") == "   "
        mock_urlopen.assert_not_called()


def test_connection_refused_falls_back_to_original_text():
    with patch("ai_rewrite.urllib.request.urlopen", side_effect=urllib.error.URLError("refused")):
        text = "this has bad grammar"
        assert ai_rewrite.rewrite(text) == text


def test_timeout_falls_back_to_original_text():
    with patch("ai_rewrite.urllib.request.urlopen", side_effect=TimeoutError):
        text = "some dictated text"
        assert ai_rewrite.rewrite(text) == text


def test_malformed_json_falls_back_to_original_text():
    mock_resp = MagicMock()
    mock_resp.read.return_value = b"not json"
    mock_resp.__enter__.return_value = mock_resp
    with patch("ai_rewrite.urllib.request.urlopen", return_value=mock_resp):
        text = "original text"
        assert ai_rewrite.rewrite(text) == text


def test_empty_model_response_falls_back_to_original_text():
    mock_resp = MagicMock()
    mock_resp.read.return_value = json.dumps({"response": "  "}).encode()
    mock_resp.__enter__.return_value = mock_resp
    with patch("ai_rewrite.urllib.request.urlopen", return_value=mock_resp):
        text = "original text"
        assert ai_rewrite.rewrite(text) == text


def test_successful_response_is_used_and_stripped():
    mock_resp = MagicMock()
    mock_resp.read.return_value = json.dumps({"response": "  Corrected text.  "}).encode()
    mock_resp.__enter__.return_value = mock_resp
    with patch("ai_rewrite.urllib.request.urlopen", return_value=mock_resp):
        assert ai_rewrite.rewrite("corrected text") == "Corrected text."


def test_request_payload_disables_thinking_and_sets_model():
    captured = {}

    def fake_urlopen(req, timeout):
        captured["body"] = json.loads(req.data)
        mock_resp = MagicMock()
        mock_resp.read.return_value = json.dumps({"response": "ok"}).encode()
        mock_resp.__enter__.return_value = mock_resp
        return mock_resp

    with patch("ai_rewrite.urllib.request.urlopen", side_effect=fake_urlopen):
        ai_rewrite.rewrite("some text")

    assert captured["body"]["model"] == ai_rewrite.MODEL
    assert captured["body"]["think"] is False
    assert captured["body"]["stream"] is False
    assert captured["body"]["prompt"] == "some text"
