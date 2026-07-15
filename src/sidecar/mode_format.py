"""Writing-mode formatting — the last pass over a finalized transcript.

The shell sends the active writing mode with each "end" message; modes
here only change STRUCTURE/STYLE, never wording — this is the rule-based
slice of STORY-006 (tone adaptation). Modes are selected manually on the
recording pill, not inferred from the focused app. Inspired by FluidVoice's
per-app prompt sets (github.com/altic-dev/FluidVoice), but implemented
independently: their enhancement runtime is closed-source and the app is
GPLv3, so nothing is ported from it.

Deliberately conservative, like cleanup.py: every rule here fires only on
a high-confidence pattern, because a wrong "correction" in someone's real
email is worse than no formatting at all.

A fourth mode, "rewrite", exists alongside these three (STORY-004's local-
LLM slice) but isn't handled by `apply_mode` — it needs a network call to
Ollama, which doesn't belong in this pure-function module. asr_server.py
calls `ai_rewrite.rewrite()` directly when that mode is active, after
`apply_mode` has already run (a no-op for "rewrite", same as "dictation").
"""
import re

MODES = ("dictation", "email", "message", "rewrite")

# "dear john" / "hi Sarah," / "hello Dr Smith." at the very start of the
# transcript.
_GREETING_WORD_RE = re.compile(r"^\s*(dear|hi|hello|hey)[ ,]+", re.IGNORECASE)

# Common English words that can follow a greeting word but are clearly the
# start of the MESSAGE, not a name ("Hi, I wanted to ask…", "Hey there").
_NOT_NAMES = {
    "i", "i'm", "i've", "we", "you", "there", "all", "everyone", "again",
    "this", "that", "the", "a", "an", "it", "just", "so", "please",
}

_SIGNOFFS = (
    "best regards", "kind regards", "warm regards", "warmest regards",
    "regards", "sincerely", "yours sincerely", "yours truly",
    "best wishes", "many thanks", "thank you", "thanks", "cheers", "best",
)
# Sign-off at the END of the text, optionally followed by a 1-2 word name.
_SIGNOFF_RE = re.compile(
    r"[,.]?\s+(" + "|".join(re.escape(s) for s in _SIGNOFFS) + r")"
    r"[ ,]*([A-Za-z][A-Za-z'-]*(?:\s+[A-Za-z][A-Za-z'-]*)?)?[.!]?\s*$",
    re.IGNORECASE,
)


def _title(words: str) -> str:
    return " ".join(w[:1].upper() + w[1:] for w in words.split())


def _looks_like_name(candidate: str) -> bool:
    words = candidate.split()
    return bool(words) and all(w.lower() not in _NOT_NAMES for w in words)


def _match_greeting(text: str):
    """Returns (salutation, name, body) or None.

    Where a multi-word name ENDS is ambiguous without punctuation ("hello
    Dr Smith the results are in" — nothing marks 'Smith' as the last name
    word), so use the ASR's own casing as the boundary: consume up to 3
    consecutive Capitalized words after the greeting. Parakeet produces
    real casing, making this reliable in practice. Fallback: after "dear"
    specifically (which is unambiguous — nobody starts email CONTENT with
    "dear"), accept a single lowercase word as the name.
    """
    m = _GREETING_WORD_RE.match(text)
    if not m:
        return None
    salutation = m.group(1).capitalize()
    rest = text[m.end():]
    words = rest.split()
    if not words:
        return None

    name_words = []
    for w in words[:3]:
        core = w.rstrip(",.")
        if core and core[0].isupper() and core.lower() not in _NOT_NAMES:
            name_words.append(core)
            if w != core:  # trailing , or . explicitly ends the name
                break
        else:
            break

    if not name_words and salutation == "Dear":
        core = words[0].rstrip(",.")
        if core and core.lower() not in _NOT_NAMES:
            name_words = [core]

    if not name_words:
        return None
    # Body = everything after the consumed name words in the original rest,
    # located by exact word spans rather than reconstructed lengths.
    spans = [w.span() for w in re.finditer(r"\S+", rest)]
    body = rest[spans[len(name_words) - 1][1]:].lstrip(" ,.")
    return salutation, _title(" ".join(name_words)), body


def format_email(text: str) -> str:
    """'dear john i am writing…' -> 'Dear John,\\n\\nI am writing…'
    and '…thanks john' -> '…\\n\\nThanks,\\nJohn'."""
    g = _match_greeting(text)
    if g:
        salutation, name, body = g
        if body:
            body = body[0].upper() + body[1:]
        text = f"{salutation} {name},\n\n{body}"

    m = _SIGNOFF_RE.search(text)
    if m and (m.group(2) is None or _looks_like_name(m.group(2))):
        signoff = m.group(1)
        signoff = signoff[0].upper() + signoff[1:].lower()
        name = _title(m.group(2)) if m.group(2) else None
        body = text[: m.start()].rstrip()
        if not body.endswith((".", "!", "?")):
            body += "."
        tail = f"\n\n{signoff},"
        if name:
            tail += f"\n{name}"
        text = body + tail

    return text


_SENTENCE_END_RE = re.compile(r"[.!?]")


def format_message(text: str) -> str:
    """Chat style: a short single-sentence message loses its formal
    trailing period ('On my way.' -> 'On my way'). Multi-sentence text is
    left alone — mid-text punctuation is still doing real work there."""
    stripped = text.rstrip()
    if stripped.endswith(".") and not stripped.endswith(".."):
        # Only if the '.' at the end is the ONLY sentence terminator.
        if not _SENTENCE_END_RE.search(stripped[:-1]):
            return stripped[:-1]
    return text


def apply_mode(text: str, mode: str) -> str:
    if not text:
        return text
    if mode == "email":
        return format_email(text)
    if mode == "message":
        return format_message(text)
    return text  # "dictation" and anything unrecognized: unchanged
