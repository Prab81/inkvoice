"""Unit tests for src/sidecar/personal_dictionary.py.

Run: python tests/test_personal_dictionary.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src" / "sidecar"))

from personal_dictionary import apply_personal_dictionary  # noqa: E402

passed = failed = 0


def check(label, actual, expected):
    global passed, failed
    if actual == expected:
        passed += 1
    else:
        failed += 1
        print(f"FAIL [{label}]\n  expected: {expected!r}\n  actual:   {actual!r}")


TERMS = ["Vihan"]

check(
    "fixes a close misrecognition",
    apply_personal_dictionary("His name is Vihar and he plays tennis.", TERMS),
    "His name is Vihan and he plays tennis.",
)
check(
    "leaves an exact match untouched",
    apply_personal_dictionary("His name is Vihan.", TERMS),
    "His name is Vihan.",
)
check(
    "does not touch lowercase words (proper-noun heuristic)",
    apply_personal_dictionary("vihan is not corrected when lowercase", TERMS),
    "vihan is not corrected when lowercase",
)
check(
    "leaves unrelated capitalized words alone",
    apply_personal_dictionary("Vikram went to the store.", TERMS),
    "Vikram went to the store.",
)
check(
    "no dictionary terms is a no-op",
    apply_personal_dictionary("Anything goes here.", []),
    "Anything goes here.",
)

print(f"\n{passed} passed, {failed} failed")
sys.exit(1 if failed else 0)
