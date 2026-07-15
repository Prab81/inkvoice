"""Writing-mode formatter tests (STORY-006 MVP slice)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src" / "sidecar"))

from mode_format import apply_mode, format_email, format_message


# --- email: greeting ---------------------------------------------------------

def test_email_greeting_basic():
    assert format_email("dear john I am writing to inform you about the delay") == (
        "Dear John,\n\nI am writing to inform you about the delay"
    )


def test_email_greeting_already_cased_with_comma():
    assert format_email("Dear Sarah, thanks for the quick turnaround") == (
        "Dear Sarah,\n\nThanks for the quick turnaround"
    )


def test_email_greeting_multiword_name():
    # Parakeet produces real casing; the capitalization boundary is what
    # separates the name from the message ("…Smith" vs "the results…").
    assert format_email("hello Dr Smith the results came back today") == (
        "Hello Dr Smith,\n\nThe results came back today"
    )


def test_email_greeting_not_a_name_left_alone():
    text = "Hi I wanted to ask about the meeting"
    assert format_email(text) == text


def test_email_hey_there_left_alone():
    text = "Hey there the build is green"
    assert format_email(text) == text


# --- email: sign-off ---------------------------------------------------------

def test_email_signoff_with_name():
    out = format_email("The report is attached. Regards John")
    assert out == "The report is attached.\n\nRegards,\nJohn"


def test_email_signoff_without_name():
    out = format_email("Let me know if that works. Thanks.")
    assert out.endswith("\n\nThanks,")


def test_email_signoff_mid_sentence_not_matched():
    text = "Please give my regards to John and the team"
    assert format_email(text) == text


def test_email_greeting_and_signoff_together():
    out = format_email("dear priya the invoice is ready best regards prabuddh")
    assert out.startswith("Dear Priya,\n\n")
    assert out.endswith("\n\nBest regards,\nPrabuddh")


# --- message -----------------------------------------------------------------

def test_message_strips_single_trailing_period():
    assert format_message("On my way.") == "On my way"


def test_message_keeps_multi_sentence_punctuation():
    text = "Running late. Start without me."
    assert format_message(text) == text


def test_message_keeps_question_mark():
    assert format_message("Are you coming?") == "Are you coming?"


# --- dispatch ----------------------------------------------------------------

def test_apply_mode_dictation_untouched():
    text = "dear john I am writing to you."
    assert apply_mode(text, "dictation") == text


def test_apply_mode_unknown_mode_untouched():
    text = "Some text."
    assert apply_mode(text, "haiku") == text


def test_apply_mode_empty():
    assert apply_mode("", "email") == ""
