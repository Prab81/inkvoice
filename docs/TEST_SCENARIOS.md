# Test Scenarios

> Populated per story when work begins (per CLAUDE.md Task Execution Protocol). Status: `Pending` (implementation not yet defined) → `Ready` (runnable) → `Pass`/`Fail`.

| ID | Story | Scenario | Type | Status |
|----|-------|----------|------|--------|
| TS-M0-01 | (M0 spike) | Parakeet-TDT (0.6B-v3 int8 ONNX) loads and transcribes test WAVs with plausible output | happy path | **Pass** (2026-07-04: en/de/es correct incl. punctuation) |
| TS-M0-02 | (M0 spike) | Chunked partial-decode latency <300ms P50 | perf gate | **Pass** (files: P50=54ms; live mic: P50=90ms P99=148ms, CPU only, transcript accurate) |
| TS-M0-03 | (M0 spike) | WER on a ~30-utterance internal test set (clean + jargon) recorded as baseline | perf baseline | Deferred to M1 (needs a collected recording set; latency + qualitative accuracy validated instead) |
| TS-001-01 | STORY-001 | Hold hotkey, speak, release — text appears in focused Notepad window | happy path | Pending |
| TS-001-02 | STORY-001 | Toggle mode: tap to start, tap to stop, text inserted on stop | happy path | **Pass** (2026-07-06: live-verified in Notepad after the physical-modifier-release fix; no shortcut misfires) |
| TS-001-03 | STORY-001 | Dictation into VS Code editor, browser textarea, Windows Terminal — all receive correct text, no keybinding interference | compatibility | Pending |
| TS-001-04 | STORY-001 | Hotkey pressed with no text field focused — graceful no-op or clear feedback, no crash | error state | Pending |
| TS-002-01 | STORY-002 | Partial words render while still speaking; visually distinct until finalized | happy path | Superseded by TS-002-04..08 (AC2 revised: live text goes into the document, not the overlay) |
| TS-002-04 | STORY-002 | Speak into focused Notepad — raw text appears in the document while still speaking (lowercased, first letter capitalized) | happy path | **Fail** (2026-07-06: one word landed, then injected keystrokes triggered Notepad shortcuts repeatedly — feature rolled back) |
| TS-002-05 | STORY-002 | On stop, live raw text is fully erased and replaced by the cleaned punctuated final; no leftover or duplicated characters | happy path | Retired (live typing rolled back) |
| TS-002-06 | STORY-002 | Push-to-talk (Ctrl+Shift physically held) during live typing — no Ctrl+char shortcuts fire, no Ctrl+Backspace word deletions | edge case | **Fail** (2026-07-06: shortcuts fired even in toggle mode — feature rolled back) |
| TS-002-07 | STORY-002 | Command mode utterance — nothing live-typed into the document; command still dispatches on final | edge case | Retired (live typing rolled back) |
| TS-002-08 | STORY-002 | Overlay pill shows waveform only — no transcript text, no "Listening…" placeholder | happy path | Ready (this half of the change was kept) |
| TS-002-02 | STORY-002 | Per-session latency metrics logged and retrievable | instrumentation | Pending |
| TS-002-03 | STORY-002 | Mid-utterance app crash of sidecar — shell recovers, no OS input left in stuck state | edge case | Pending |

| TS-007-01 | STORY-007 | Add a term on the dashboard, dictate a near-miss of it — correction applies without sidecar restart | happy path | Ready |
| TS-007-02 | STORY-007 | Remove a term — no longer applied on next dictation | happy path | Ready |
| TS-014-01 | STORY-014 | Dictate, open dashboard — utterance appears in history under "Today" with time and duration | happy path | Ready |
| TS-014-02 | STORY-014 | "Show changes" renders word-level diff when cleanup edited the text; hidden when raw == cleaned | happy path | Ready |
| TS-014-03 | STORY-014 | "Copy raw (revert)" puts the raw transcript on the clipboard | happy path | Ready |
| TS-014-04 | STORY-014 | Search box filters history across raw and cleaned text | happy path | Ready |
| TS-014-05 | STORY-014 | Dashboard unreachable (sidecar down) or empty logs — page loads with empty states, no errors | error state | Ready |
| TS-002-09 | STORY-002 | Speak — live transcript appears in the pill within ~300ms, wraps to 2 lines, tail-trims on long utterances | happy path | Ready |
| TS-006-01 | STORY-006 | Click the pill's mode chip — label cycles Dictation → Email → Message; focus stays in the target app | happy path | Ready |
| TS-006-02 | STORY-006 | Email mode: "dear john I am writing…" → typed text starts "Dear John," + blank line + capitalized paragraph | happy path | Ready |
| TS-006-03 | STORY-006 | Email mode: utterance ending "regards john" → sign-off on its own lines | happy path | Ready |
| TS-006-04 | STORY-006 | Message mode: short single sentence loses trailing period; multi-sentence text untouched | happy path | Ready |
| TS-006-05 | STORY-006 | Command mode recording with Email selected on the chip — command matching unaffected (mode forced to dictation) | edge case | Ready |
| TS-004-01 | STORY-004 | Rewrite mode: grammatically broken dictation ("me and him was going") is corrected on stop, meaning/tone preserved | happy path | Ready |
| TS-004-02 | STORY-004 | Rewrite mode with Ollama stopped/unreachable — falls back to the rule-cleaned text unchanged, no crash, no hang beyond the timeout | error state | Ready |
| TS-004-03 | STORY-004 | Rewrite mode chip cycle (4th position) — label reads "Rewrite"; wire value sent to sidecar is "rewrite" | happy path | Ready |
| TS-004-04 | STORY-004 | Rewrite mode on already-correct text — output is unchanged or near-identical, no unwanted rewording/summarizing | edge case | Ready |

<!-- Scenarios for M2+ stories added when those stories move to In Progress. -->
