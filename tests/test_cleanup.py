"""Unit tests for src/sidecar/cleanup.py — rule-based transcript cleanup.

Run: python tests/test_cleanup.py
No pytest dependency — plain assertions, since this is a small, stable
module and the M2 rule-based phase is expected to be replaced/augmented by
an LLM cleanup pass later (see docs/EXECUTION_PLAN.md).
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src" / "sidecar"))

from cleanup import (  # noqa: E402
    apply_verbal_commands,
    clean_transcript,
    collapse_self_corrections,
    format_lists,
    remove_fillers,
)

passed = failed = 0


def check(label: str, actual, expected):
    global passed, failed
    if actual == expected:
        passed += 1
    else:
        failed += 1
        print(f"FAIL [{label}]\n  expected: {expected!r}\n  actual:   {actual!r}")


# --- verbal commands ---------------------------------------------------------
check("verbal: period", apply_verbal_commands("this is a test period"), "this is a test.")
check("verbal: comma + question mark",
      apply_verbal_commands("are you coming comma or not question mark"),
      "are you coming, or not?")
check("verbal: new line", apply_verbal_commands("hello new line world"), "hello\nworld")
check("verbal: new paragraph",
      apply_verbal_commands("done new paragraph next section"), "done\n\nnext section")
check(
    # Regression: found live — "over a period of time" (ordinary noun
    # usage) came out as "over a. of time" because the old regex rewrote
    # every standalone "period" with no context check at all.
    "verbal: 'period' as content after a determiner is not a command",
    apply_verbal_commands("the understanding of a person's voice over a period of time"),
    "the understanding of a person's voice over a period of time",
)
check(
    "verbal: 'a comma' as content is not a command",
    apply_verbal_commands("please use a comma here"),
    "please use a comma here",
)
check(
    "verbal: period still works as a command right after content (no determiner)",
    apply_verbal_commands("this is a test period next sentence"),
    "this is a test. next sentence",
)

# --- self-correction ---------------------------------------------------------
check("correction: actually",
      collapse_self_corrections("meet at 2, actually make it 3"), "make it 3")
check("correction: scratch that",
      collapse_self_corrections("send it to Bob scratch that send it to Alice"),
      "send it to Alice")
check("correction: none present (prose unchanged)",
      collapse_self_corrections("this is a normal sentence"), "this is a normal sentence")
check("correction: only affects its own sentence",
      collapse_self_corrections("First sentence is fine. Second, actually third one."),
      "First sentence is fine. third one.")
check(
    # Regression: found via accent testing (Zira/en-US voice) — "p.m."'s
    # internal period broke sentence-boundary detection, which both failed
    # to collapse the correction AND mangled "p.m." into "p. m.".
    "correction: abbreviation periods (p.m.) don't break sentence detection",
    collapse_self_corrections(
        "Meet me at 2 p.m. Actually make it 3 p.m. instead"
    ),
    "make it 3 p.m. instead",
)
check(
    "correction: abbreviation survives untouched with no correction present",
    collapse_self_corrections("The meeting is at 2 p.m. See you then."),
    "The meeting is at 2 p.m. See you then.",
)

# --- filler removal -----------------------------------------------------------
check("fillers: um/uh stripped", remove_fillers("so um I think uh this works"),
      "so I think this works")
check("fillers: leaves legit words alone", remove_fillers("hum along with the tune"),
      "hum along with the tune")

# --- list formatting -----------------------------------------------------------
check(
    "lists: three ordinals -> numbered",
    format_lists("first buy milk second walk the dog third call mom"),
    "1. buy milk\n2. walk the dog\n3. call mom",
)
check(
    "lists: single ordinal is prose, left alone",
    format_lists("this is my first day at the job"),
    "this is my first day at the job",
)
check(
    "lists: two ordinals only (below min 3) left alone",
    format_lists("I came in first and she came in second"),
    "I came in first and she came in second",
)
check(
    # Regression: exact shape of a live false positive (M2) that destroyed
    # real content — "first" used as an ordinary word in narration, with no
    # real enumerated list, must NOT trigger list formatting.
    "lists: narrative 'first'/'then' usage is not a list",
    format_lists(
        "and when I'm telling a story, I want the first two to create sound "
        "effects, and then I want to hear insects making sound"
    ),
    "and when I'm telling a story, I want the first two to create sound "
    "effects, and then I want to hear insects making sound",
)
check(
    "lists: out-of-order ordinals is not treated as a list",
    format_lists("going back to my first point, my second point, my first point again"),
    "going back to my first point, my second point, my first point again",
)

# --- full pipeline ------------------------------------------------------------
check(
    "pipeline: filler + correction + punctuation",
    clean_transcript("um meet at 2 comma actually make it 3 period"),
    "make it 3.",
)
check(
    "pipeline: dictated list with fillers",
    clean_transcript("okay so um first buy milk second uh walk the dog third call mom"),
    "okay so\n1. buy milk\n2. walk the dog\n3. call mom",
)
check("pipeline: empty input", clean_transcript(""), "")

print(f"\n{passed} passed, {failed} failed")
sys.exit(1 if failed else 0)
