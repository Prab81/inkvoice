"""Rule-based transcript cleanup — M2, rule-based phase.

Applied to a FINAL transcript only, never to partials: every transform here
needs full-sentence context (a correction cue can invalidate everything
before it; a list cue only makes sense once the whole utterance is known),
and re-running these on every partial would make the live text visibly
flicker as words get provisionally stripped and restored.

Pipeline order matters and is deliberate:
  1. verbal commands   (produce literal punctuation the later stages read)
  2. self-correction   (must run before filler-stripping collapses the very
                         cue words — e.g. "um, actually" — it depends on)
  3. filler removal
  4. list formatting    (needs the cleaned sentence, not raw filler-laden text)

KNOWN LIMITATION (M2 rule-based phase): these are heuristics over surface
text, not a semantic understanding of what was said. They will misfire on
legitimate use of a cue word as content (e.g. dictating the sentence
"Actually, I love this plan." mid-document with no prior clause to correct).
The planned local-LLM cleanup pass (M2, next phase) uses context the way an
editor would; this rule-based pass is deliberately conservative in its
place. Every stage is unit-tested (tests/test_cleanup.py) against both
the intended case and a plausible false-positive case.
"""
import re

# --- 1. Verbal commands -----------------------------------------------------

# FOUND LIVE: "over a period of time" (ordinary noun usage) came out of
# cleanup as "over a. of time" — the bare regex below has no context
# awareness and rewrites *every* standalone "period" to "." regardless of
# whether it's a punctuation command or content. Same risk applies to
# comma/colon/semicolon (e.g. "grace period", "trial period", "colon
# cancer", "comma-separated"). A punctuation command is essentially never
# preceded by a determiner ("a period", "the comma") — nobody dictates
# "a period" meaning "insert a full stop" — so guarding on that is a cheap,
# safe signal to tell command usage from content usage without needing real
# semantic understanding.
_DETERMINER_GUARD = (
    r"(?<!\ba\s)(?<!\ban\s)(?<!\bthe\s)(?<!\bthis\s)(?<!\bthat\s)"
    r"(?<!\bone\s)(?<!\bsome\s)(?<!\bevery\s)(?<!\beach\s)"
)

_VERBAL_COMMANDS = [
    (r"\bnew paragraph\b", "\n\n"),
    (r"\bnew line\b", "\n"),
    (_DETERMINER_GUARD + r"\bquestion mark\b", "?"),
    (_DETERMINER_GUARD + r"\bexclamation (point|mark)\b", "!"),
    (_DETERMINER_GUARD + r"\bcomma\b", ","),
    (_DETERMINER_GUARD + r"\bperiod\b", "."),
    (_DETERMINER_GUARD + r"\bcolon\b", ":"),
    (_DETERMINER_GUARD + r"\bsemicolon\b", ";"),
]


def apply_verbal_commands(text: str) -> str:
    for pattern, replacement in _VERBAL_COMMANDS:
        text = re.sub(pattern, replacement, text, flags=re.IGNORECASE)
    # Collapse " ," -> "," etc. left by a substitution, then re-tidy spacing,
    # including stray spaces the substitution leaves around inserted newlines
    # (the original words' surrounding spaces are outside the replacement).
    text = re.sub(r"\s+([,.!?;:])", r"\1", text)
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n[ \t]+", "\n", text)
    text = re.sub(r"[ \t]+", " ", text)
    return text.strip()


# --- 2. Self-correction collapsing ------------------------------------------

# Cue phrases that mean "discard what I said before this point, use what
# follows instead". Ordered longest-first so multi-word cues aren't shadowed
# by a shorter substring (e.g. "no wait" before "wait" if "wait" existed).
_CORRECTION_CUES = [
    "scratch that",
    "no wait",
    "i mean",
    "actually",
    "sorry",
]
_CORRECTION_RE = re.compile(
    r"[,.]?\s*\b(" + "|".join(re.escape(c) for c in _CORRECTION_CUES) + r")\b[,.]?\s*",
    re.IGNORECASE,
)

# FOUND LIVE (M2, accent testing): abbreviations like "p.m." have an
# internal period that the sentence-splitter below was treating as a real
# sentence boundary, which (a) isolated a correction cue from the text it
# should have overridden — "Meet me at 2 p.m., actually make it 3 p.m."
# failed to collapse at all — and (b) got mangled into "p. m." by the
# trailing space-restoration step. Fix: swap the internal period for a
# placeholder before splitting, and restore it after rejoining, so these
# abbreviations are atomic as far as sentence-boundary detection is concerned.
_ABBREV_RE = re.compile(r"\b(a\.m|p\.m|u\.s|mr|mrs|ms|dr|vs|etc)\.", re.IGNORECASE)
_ABBREV_PLACEHOLDER = "\x01"


def _protect_abbreviations(text: str) -> str:
    return _ABBREV_RE.sub(lambda m: m.group(0).replace(".", _ABBREV_PLACEHOLDER), text)


def _restore_abbreviations(text: str) -> str:
    return text.replace(_ABBREV_PLACEHOLDER, ".")


def collapse_self_corrections(text: str) -> str:
    """Keep only the text after the LAST correction cue in each sentence.

    Operates per-sentence (split on '.', '!', '?', '\\n') so a correction in
    one sentence doesn't eat earlier, unrelated sentences.
    """
    text = _protect_abbreviations(text)
    parts = re.split(r"([.!?\n])", text)  # keep delimiters
    out = []
    for i in range(0, len(parts), 2):
        sentence = parts[i]
        delim = parts[i + 1] if i + 1 < len(parts) else ""
        matches = list(_CORRECTION_RE.finditer(sentence))
        if matches:
            sentence = sentence[matches[-1].end():]
        out.append(sentence + delim)
    joined = "".join(out).strip()
    # A correction cue can consume the space that normally follows sentence
    # punctuation (it sat right after that space) — restore it so sentences
    # don't get glued together, without touching intentional "X.\nY" breaks.
    joined = re.sub(r"([.!?])(?=\S)", r"\1 ", joined)
    return _restore_abbreviations(joined)


# --- 3. Filler word removal --------------------------------------------------

_FILLERS = r"\b(um+|uh+|erm+|hm+)\b"
_FILLER_RE = re.compile(_FILLERS, re.IGNORECASE)


def remove_fillers(text: str) -> str:
    text = _FILLER_RE.sub("", text)
    # Fillers leave orphaned punctuation/spacing behind ("Hi, um, there" ->
    # "Hi, , there"); tidy comma runs and doubled spaces without touching
    # intentional punctuation elsewhere.
    text = re.sub(r"\s*,\s*,", ",", text)
    text = re.sub(r"[ \t]+", " ", text)
    text = re.sub(r"\s+([,.!?;:])", r"\1", text)
    text = re.sub(r",\s*([.!?])", r"\1", text)
    return text.strip()


# --- 4. List formatting -------------------------------------------------------
#
# FOUND LIVE (M2): the first version of this triggered on ordinary narrative
# speech ("I want the first two ... and then ...") and destroyed real content
# by chopping it into a bogus 2-item list. Root cause: "first"/"then" etc.
# are common prose words, not just list markers, so "any 2 distinct ordinal
# words" was far too weak a signal on continuous dictation. A false positive
# here is strictly worse than doing nothing (it corrupts the transcript), so
# this is deliberately conservative: only the numeric ordinals count (drop
# "next"/"then"/"finally"/"lastly" — the most prose-common offenders), and
# they must appear in strictly increasing order (first, then second, then
# third...) with at least 3 of them. A real enumerated list said out loud
# reliably satisfies this; coincidental prose usage essentially never does.

_ORDINAL_WORDS = [
    "first", "second", "third", "fourth", "fifth", "sixth",
    "seventh", "eighth", "ninth", "tenth",
]
_ORDINAL_INDEX = {w: i for i, w in enumerate(_ORDINAL_WORDS)}
_ORDINAL_RE = re.compile(
    r"\b(" + "|".join(_ORDINAL_WORDS) + r")\b,?\s*",
    re.IGNORECASE,
)
_MIN_LIST_ITEMS = 3


def format_lists(text: str) -> str:
    candidates = list(_ORDINAL_RE.finditer(text))

    # Keep only a strictly increasing subsequence (first < second < third...)
    # — a real spoken list says these in order; prose repeating or jumping
    # backward ("first... my second point is... going back to my first...")
    # is not treated as a list at all, to stay on the conservative side.
    kept, last_idx = [], -1
    for m in candidates:
        idx = _ORDINAL_INDEX[m.group(1).lower()]
        if idx > last_idx:
            kept.append(m)
            last_idx = idx

    if len(kept) < _MIN_LIST_ITEMS:
        return text  # not confidently a list — leave prose untouched

    preamble = text[:kept[0].start()].strip(" ,.")
    items, cursor = [], kept[0].end()
    for m in kept[1:]:
        items.append(text[cursor:m.start()].strip(" ,."))
        cursor = m.end()
    items.append(text[cursor:].strip(" ,."))
    items = [i for i in items if i]
    list_text = "\n".join(f"{i + 1}. {item}" for i, item in enumerate(items))
    return f"{preamble}\n{list_text}" if preamble else list_text


# --- Pipeline -----------------------------------------------------------------

def clean_transcript(raw_text: str) -> str:
    if not raw_text:
        return raw_text
    text = apply_verbal_commands(raw_text)
    text = collapse_self_corrections(text)
    text = remove_fillers(text)
    text = format_lists(text)
    return text
