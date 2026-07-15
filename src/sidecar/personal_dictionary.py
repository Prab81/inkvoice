"""Personal dictionary — fuzzy post-decode name/term correction.

STORY-007 in full (auto-learning, org-shared, import/export) is not built
yet. This is the practical slice that ships now: a small user-maintained
word list, applied as a fuzzy correction pass on top of the ASR output.

Why fuzzy post-correction instead of model-level hotwords (the "proper"
fix): sherpa-onnx's hotwords_file *does* work with this model (confirmed
live), but it requires entries pre-tokenized into the model's SentencePiece
subword vocabulary, which needs the original tokenizer artifact — not
bundled in this ONNX export (see docs/CONTEXT.md). Getting that is a
tracked follow-up, not a quick fix. This module gets a working result today
without it: it looks at each capitalized word/short phrase the ASR
produced and, if it's a close-but-not-exact match to a dictionary entry,
swaps in the correct spelling.

Deliberately conservative like the rest of the M2 rule-based pass: only
capitalized tokens are considered (proper-noun heuristic, avoids "correcting"
ordinary words), and only close matches above a similarity floor are swapped
— a miss stays as the ASR's raw guess rather than risk a wrong "correction".
"""
import difflib
import json
import re
from pathlib import Path

DEFAULT_DICT_PATH = Path(__file__).resolve().parent / "personal_dictionary.json"
SIMILARITY_THRESHOLD = 0.6  # difflib ratio; tuned against the false-positive
                             # risk of "correcting" an unrelated real word


def load_dictionary(path: Path = DEFAULT_DICT_PATH) -> list[str]:
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        return list(data.get("terms", []))
    except (json.JSONDecodeError, OSError):
        return []


_WORD_RE = re.compile(r"[A-Za-z']+")


def apply_personal_dictionary(text: str, terms: list[str]) -> str:
    if not terms or not text:
        return text

    # Match against multi-word terms too (e.g. a full name), longest first
    # so "Vihan Kumar" is tried before "Vihan" alone would partially match.
    terms_sorted = sorted(terms, key=len, reverse=True)

    def best_match(candidate: str) -> str | None:
        best, best_score = None, 0.0
        for term in terms_sorted:
            if candidate.lower() == term.lower():
                return None  # already correct, nothing to do
            score = difflib.SequenceMatcher(None, candidate.lower(), term.lower()).ratio()
            if score > best_score:
                best, best_score = term, score
        return best if best_score >= SIMILARITY_THRESHOLD else None

    def replace_word(m: re.Match) -> str:
        word = m.group(0)
        if not word[0].isupper():
            return word  # proper-noun heuristic: leave lowercase words alone
        match = best_match(word)
        return match if match else word

    return _WORD_RE.sub(replace_word, text)
