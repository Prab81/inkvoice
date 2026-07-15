# Stories

> Organized by epic. Every REQ in @docs/PRD.md has at least one story. All stories start as Backlog until picked up.

---

## Epic A: Core Dictation Engine

### STORY-001: System-wide dictation via hotkey
**Status:** In Progress
**Priority:** P0
**PRD Ref:** REQ-001
**Last Updated:** 2026-07-09

**As a** user,
**I want** to press a hotkey and dictate into whatever text field is focused,
**So that** I can write in any application without switching tools.

#### Acceptance Criteria
- [x] AC1: Push-to-talk (hold) and toggle (tap-to-start/tap-to-stop) modes both configurable — both now implemented and coexist: Ctrl+Shift+8 (Prose)/Ctrl+Shift+9 (Command) remain toggle hotkeys, Ctrl+Shift+Space added as a hold-to-talk chord for Prose mode via a low-level keyboard hook (`RegisterHotKey` has no key-release event, so push-to-talk couldn't be built on the same mechanism as the toggle hotkeys); "configurable" here means both modes are simultaneously available, not yet a user-facing settings toggle (no settings UI exists at all yet)
- [~] AC2: Works in at least: browser text fields, native text editors, Slack/email clients, IDEs, terminal emulators — confirmed working (Notepad, classic console hosts); confirmed **blocked** on Windows Terminal (`ACCESS_DENIED`, AppContainer sandboxing — see CONTEXT.md); browsers/Slack/email/IDEs not yet tested
- [~] AC3: Hotkey is user-remappable and does not conflict with common OS/app shortcuts by default — not user-configurable yet (hardcoded fallback candidate list); moved off a letter key (Ctrl+Alt+I), then off Ctrl+Alt (AltGr reinterpretation on non-US layouts), then off the backtick key entirely (2026-07-09: leaked into Notepad on a machine with two keyboard layouts installed — RegisterHotKey's OEM-punctuation matching depends on the foreground thread's active layout at keypress time; confirmed via Notepad's Alt-KeyTip badges appearing even though our combo has no Alt in it) to Ctrl+Shift+F9 (function-key VK codes don't vary by layout), then off F9 too same day — it worked, but most laptop keyboards route bare F-keys to hardware/media functions by default, requiring an extra Fn hold every time. Primary is now **Ctrl+Shift+8**: no Fn needed, and digits are just as layout-stable as function keys for this user's installed layouts (en-AU/en-GB share an identical QWERTY digit row — only OEM punctuation diverged). Not yet a settings-configurable remap.

#### Technical Notes
Must not require per-app plugins — needs OS-level text-insertion API (Accessibility API on macOS, UI Automation on Windows, AT-SPI/portal on Linux). **Windows-specific finding:** `SendInput` alone is insufficient — MSIX/AppContainer-sandboxed apps (Windows Terminal being the flagship example) reject it outright. `WriteConsoleInput` is the identified fix path for console targets specifically, not yet implemented. **Second Windows-specific finding:** OEM punctuation keys (backtick, comma, etc.) are unsafe choices for `RegisterHotKey` on any machine with multiple keyboard layouts installed — function keys don't have this problem.

#### Change History
- [2026-07-04] Created from REQ-001
- [2026-07-04] M1 vertical slice built and live-tested: hotkey (RegisterHotKey with fallback candidates), WASAPI capture, SendInput text insertion. Found and fixed: audio stream startup race with Bluetooth profile switching, SendInput batch-drop race, hotkey/letter-key collision. Found and NOT yet fixed: Windows Terminal blocks SendInput entirely (STORY-015 finding).
- [2026-07-06] Added push-to-talk (Ctrl+Shift+Space, hold-to-record) alongside the existing toggle hotkeys, per explicit user decision to keep both rather than replace either. Implemented via `WH_KEYBOARD_LL` low-level keyboard hook (`main.rs::keyboard_hook_proc`) since `RegisterHotKey` never fires a "released" event. Refactored the shared start/stop recording logic (previously inline in the toggle branch) into `start_recording`/`stop_recording` functions so both the toggle and push-to-talk paths call the same code. Built cleanly, all three hotkeys (toggle dictation, toggle Command Mode, push-to-talk) register successfully on relaunch. Not yet live-tested with a real held key.
- [2026-07-09] Found live: Ctrl+Shift+\` (the AltGr-safe replacement from the earlier finding) still leaked straight into Notepad on this specific machine, immediately on keypress. Root cause: two keyboard layouts installed (en-AU, en-GB) with Windows' default per-app-window input method switching; RegisterHotKey's backtick matching silently stopped working once Notepad's active layout diverged. Fixed by moving the dictation toggle to Ctrl+Shift+F9 (layout-independent); backtick combos kept only as last-resort fallback candidates. Verified via captured stdout: registers as F9 with no fallback.
- [2026-07-09] Found live (same day): F9 required holding Fn on the user's laptop keyboard — worked, but an unwelcome extra key every time. Moved primary to Ctrl+Shift+8 (no Fn row involved); F9 kept as first fallback. Verified via captured stdout: registers as 8 with no fallback, and a real dictation successfully typed into Notepad on this combo.

---

### STORY-002: Real-time streaming partial transcription
**Status:** In Progress
**Priority:** P0
**PRD Ref:** REQ-002, NREQ-001
**Last Updated:** 2026-07-06

**As a** user,
**I want** to see words appear as I speak, not just after I stop,
**So that** dictation feels instantaneous and I can self-correct in the moment.

#### Acceptance Criteria
- [x] AC1: Partial results render within 300ms (P50) of the corresponding speech — dual-model architecture (streaming Zipformer for partials) achieves ~0.2ms P50, far under budget; see CONTEXT.md "Dual-Model ASR Architecture"
- [REVISED] AC2 (old): Partial results are visually distinguishable from finalized text until confirmed — was implemented as live text inside the recording overlay pill; user rejected this live ("text on the wave modal is not useful") — the live transcript belongs in the actual document, not the pill
- [REVISED] AC2 (attempt 2): live-typing partials into the focused document — implemented and ROLLED BACK same day: live test showed injected keystrokes triggering target-app shortcuts repeatedly (see CONTEXT.md 2026-07-06 rollback entry). Any future attempt must gate injection on modifier keys being physically up.
- [ ] AC2 (attempt 3, current): live transcript returned to the recording pill 2026-07-08, display-only (no injection — the shortcut failure class is structurally impossible): pill expanded to 380×150 with a two-line wrapping transcript that tail-trims to the newest words. Verified via synthetic harness + screen capture; not yet live-verified by the user.
- [x] AC3: Latency is measured and logged per session for internal benchmarking against NREQ-001 — standing per-utterance log shipped (`latency_log.jsonl`: time-to-first-partial, finalize, total), surfaced as P50/P99 stat cards on the local dashboard (2026-07-07)

#### Change History
- [2026-07-04] Created from REQ-002, NREQ-001
- [2026-07-04] M0 spike validated feasibility (chunked Parakeet redecode, P50~90ms). M2: replaced with a genuinely streaming model (Zipformer) after finding the redecode approach caused constant partial flicker and latency that grew with utterance length — see CONTEXT.md. Real streaming partial now near-instant and stable (zero revisions).
- [2026-07-06] [REVISED] AC2 reworked after live feedback: partial text in the overlay pill was only readable in a cramped single line and the user only ever saw committed text after stopping — replaced with live typing into the focused document itself (prefix-diff per partial, wholesale erase-and-replace with the cleaned final on stop; raw partials lowercased + first-letter-capitalized since the streaming model has no casing). Overlay reverted to waveform-only. Known limitation: moving the caret mid-dictation corrupts the retraction (documented, accepted for MVP).
- [2026-07-06] ROLLED BACK live typing after live test failure: only one word ever landed and the target app (Notepad) interpreted the injected keystroke stream as shortcuts, repeatedly triggering commands, even in toggle mode (Ctrl+Shift+`). Reverted to final-only typing; overlay stays waveform-only (that half of the change was kept). Root cause not fully diagnosed — see CONTEXT.md for hypotheses and constraints on any retry.
- [2026-07-06] Found live: dictations longer than 20s only had their trailing ~20s finalized — the rest was silently dropped (a side effect of the earlier fix for long-buffer decode degradation). Replaced the single-trailing-window final decode with chunked decoding + stitching (`asr_server.py`'s `UtteranceDecoder`) — bounded ~15-19s chunks, pause-preferred cut points, stitched before the single cleanup pass. Removes the length limit structurally. Not yet live-tested against a real long dictation.

---

### STORY-003: Auto punctuation, capitalization, and formatting
**Status:** In Progress
**Priority:** P0
**PRD Ref:** REQ-003
**Last Updated:** 2026-07-04

**As a** user,
**I want** punctuation and paragraph structure inferred automatically,
**So that** I don't have to say "comma" and "period" for every sentence.

#### Acceptance Criteria
- [x] AC1: Sentence boundaries and standard punctuation inferred from pause/prosody patterns — the final-text model (Parakeet) produces this natively; verified across multiple live and synthetic tests
- [x] AC2: Explicit verbal commands ("new line", "period", "question mark") still supported as overrides — implemented in `cleanup.py::apply_verbal_commands`, unit tested
- [x] AC3: List/paragraph structure inferred when user dictates enumerable items — implemented in `cleanup.py::format_lists`, deliberately conservative (3+ strictly-increasing numeric ordinals required) after a live false-positive destroyed real content; regression-tested

#### Change History
- [2026-07-04] Created from REQ-003
- [2026-07-04] Rule-based cleanup pipeline shipped (`src/sidecar/cleanup.py`). Live testing found and fixed: a list-detection false positive that corrupted ordinary prose, and an abbreviation-period bug ("p.m.") that broke sentence-boundary detection. 20 unit tests passing.

---

### STORY-004: AI Auto-Edit filler and self-correction removal
**Status:** In Progress
**Priority:** P0
**PRD Ref:** REQ-004
**Last Updated:** 2026-07-09

**As a** user,
**I want** filler words and self-corrections cleaned up automatically,
**So that** my dictated text reads as if I wrote it carefully, not spoke it live.

#### Acceptance Criteria
- [x] AC1: Common filler words ("um", "uh", "like") removed by default — implemented (`remove_fillers`), not yet toggleable (no settings UI exists)
- [x] AC2: Self-corrections ("meet at 2, actually make it 3") resolve to the corrected version only — implemented (`collapse_self_corrections`), including an abbreviation-period edge case found and fixed live
- [~] AC3: Edge case: legitimate use of a filler word as content is not mangled, user can view raw transcript — raw transcript IS preserved (`debug_audio/*.txt`) but not exposed to the user in any UI; **found live and partially fixed:** "over a period of time" was corrupted to "over a. of time" by the verbal-command matcher treating content-usage "period" as the punctuation command; fixed with a determiner guard (a/an/the/etc. immediately before the word signals content, not command) for period/comma/colon/semicolon — closes the reproduced case, but less common phrasings without a determiner (e.g. "grace period") remain a known gap in this heuristic
- [x] AC4 (new, the local-LLM slice EXECUTION_PLAN's M2 always called for): grammar/fluency correction beyond rule-based fillers/self-correction — shipped 2026-07-09 as a 4th writing mode, **Rewrite**, rather than an always-on pass. `src/sidecar/ai_rewrite.py` calls a local Ollama instance (already installed/running on this machine — no new dependency) with `qwen3:8b`, `think: false` for latency. Warm-model latency measured 120-600ms for sentence-length input; falls back to unmodified text on any error/timeout/unreachable-Ollama (never blocks or crashes dictation).

#### Design Note
PRD/EXECUTION_PLAN framed this as an always-on Auto-Edit pass; shipped instead as an explicit opt-in mode (matching the user's request and STORY-006's mode-chip pattern). Rationale: an LLM call is neither instant nor risk-free (it can reword things the rule-based passes never would), so making it opt-in keeps the default (Dictation/Email/Message) fast and predictable, and confines "the AI changed my wording" risk to only where the user asked for it.

#### Change History
- [2026-07-04] Created from REQ-004
- [2026-07-04] Rule-based pass shipped as part of the M2 cleanup pipeline (see STORY-003 change history — same module/commit).
- [2026-07-09] Local-LLM grammar/fluency pass shipped as a 4th writing mode ("Rewrite") on the recording pill, using an already-running local Ollama instance (`qwen3:8b`, thinking disabled). 7 new resilience-focused unit tests (fallback on connection error/timeout/malformed response — real model output isn't asserted on, by design). Verified end-to-end with a real Ollama call (not just mocks): "me and him was going... we was late" → "Me and him were going... we were late.". Not yet live-tested by the user via the actual pill/hotkey flow.

---

### STORY-005: Real-time retroactive correction
**Status:** In Progress
**Priority:** P1
**PRD Ref:** REQ-019
**Last Updated:** 2026-07-04

**As a** user,
**I want** to say "no, I meant X" immediately after a misrecognition and have it applied retroactively,
**So that** I don't have to manually select and retype the wrong word.

#### Acceptance Criteria
- [x] AC1: Correction phrase pattern detected within the same dictation session — `collapse_self_corrections` detects cue phrases (actually, scratch that, no wait, I mean, sorry) within an utterance
- [~] AC2: Correction applies to the most recent matching span, not the whole document — applies within the current utterance/sentence only (correct scope for what's built so far); "whole document" scope doesn't apply yet since there's no persistent document/history view (STORY-014, not built)
- [ ] AC3: Edge case: ambiguous correction prompts disambiguation — not built; current behavior always takes the last cue's match, no disambiguation UI

#### Change History
- [2026-07-04] Created from REQ-019
- [2026-07-04] Cue-phrase detection shipped as part of the M2 cleanup pipeline (see STORY-003 change history). This is the rule-based slice — real-time (mid-utterance, before finalization) correction as originally envisioned would need the streaming model to support retroactive edits, not yet designed.

---

## Epic B: Personalization & Context

### STORY-006: App-context-aware tone adaptation
**Status:** In Progress
**Priority:** P1
**PRD Ref:** REQ-005
**Last Updated:** 2026-07-08

**As a** user,
**I want** my dictation tone to adapt automatically to the app I'm writing in,
**So that** emails sound formal and chat messages sound casual without manual editing.

#### Acceptance Criteria
- [~] AC1: Tone presets exist for at least: email, chat/IM, code comments, documents — MVP slice shipped 2026-07-08: manual writing modes (Dictation / Email / Message) selected via a clickable chip on the recording pill; rule-based formatting only (email greeting "dear john…" → "Dear John,\n\n…" + sign-off structuring; message mode strips the formal trailing period on short messages). Code-comments and documents presets, plus LLM tone rewrite, not yet built.
- [ ] AC2: Per-app override is visible and user-configurable — mode is manual per-recording, not per-app; auto-detection from destination app identity is the follow-up
- [x] AC3: No screen-content capture is used to achieve this — destination app identity only. Trivially satisfied by the MVP (no context capture of any kind — mode is user-selected)

#### Technical Notes
Rules in `src/sidecar/mode_format.py`, applied after cleanup+dictionary; mode travels in the "end" message (Command-mode recordings always send "dictation" so formatting can't corrupt command matching). Multi-word greeting names use the ASR's own capitalization as the name/content boundary. Concept parallels FluidVoice's per-app prompt sets, but implemented independently — FluidVoice's enhancement runtime is closed-source and the app is GPLv3 (no code ported).

#### Change History
- [2026-07-04] Created from REQ-005
- [2026-07-08] MVP slice shipped: manual modes on the pill + rule-based email/message formatting (15 unit tests). App-aware auto-selection and LLM tone deferred.

---

### STORY-007: Personal Dictionary with auto-learning
**Status:** In Progress
**Priority:** P0
**PRD Ref:** REQ-006
**Last Updated:** 2026-07-04

**As a** user,
**I want** names, jargon, and acronyms I use to be learned automatically,
**So that** I don't have to manually maintain a dictionary for accuracy to improve.

#### Acceptance Criteria
- [~] AC1: Manual corrections captured and applied automatically, persisting across sessions — in-app capture flow shipped 2026-07-07 (dashboard dictionary manager: add/remove terms, applies from the very next dictation since the decoder reloads the JSON per utterance); fully *automatic* learning from user corrections still not built
- [ ] AC2: Explicit "add to dictionary" action available from history view — shipped 2026-07-07 on the dashboard (per-history-item "Add term to dictionary…", pre-filled from selected text); not yet live-verified by the user
- [~] AC3: Dictionary entries apply across all apps and are exportable — apply across all apps (language-agnostic to destination), yes; exportable in the STORY-020 sense (a dedicated export flow), no — though the underlying format is already a plain portable JSON file

#### Technical Notes
Shipped as **fuzzy post-decode correction**, not true model-level personalization: sherpa-onnx's `hotwords_file` mechanism was confirmed working but requires entries pre-tokenized into the model's SentencePiece vocabulary, and the tokenizer artifact needed for that isn't bundled with either ASR model currently in use. `src/sidecar/personal_dictionary.py` instead does a conservative fuzzy match (capitalized tokens only, similarity ≥ 0.6) against a small JSON term list, applied after cleanup. Real hotwords-based biasing remains open — see CONTEXT.md "Accent Testing, Hotwords Investigation."

#### Change History
- [2026-07-04] Created from REQ-006
- [2026-07-04] Shipped fuzzy dictionary correction v1 (`personal_dictionary.py`), seeded with the user's own name after live testing showed name-recognition jitter. 5 unit tests passing.

---

### STORY-008: Multi-language support with auto-detection
**Status:** Backlog
**Priority:** P1
**PRD Ref:** REQ-007
**Last Updated:** 2026-07-04

**As a** multilingual user,
**I want** the app to detect and support 50+ languages, including switching mid-session,
**So that** I can dictate in whichever language I'm currently thinking in.

#### Acceptance Criteria
- [ ] AC1: Language auto-detected from speech within the first few words of a session
- [ ] AC2: Manual language override available and remembered per-app
- [ ] AC3: Mid-session language switch handled without restarting dictation

#### Change History
- [2026-07-04] Created from REQ-007

---

## Epic C: Command Mode & Developer Experience

### STORY-009: Grammar-based Command Mode
**Status:** In Progress
**Priority:** P0
**PRD Ref:** REQ-008
**Last Updated:** 2026-07-04

**As a** power user,
**I want** a hands-free command mode for navigation and editing (not just prose dictation),
**So that** I can operate my computer by voice for extended hands-free sessions.

#### Acceptance Criteria
- [x] AC1: Mode switch between prose dictation and Command Mode is explicit, not ambiguous inference — implemented as a **second hotkey** (Ctrl+Alt+9), not a voice wake-phrase (deliberate scope decision: voice-triggered mode switching is unreliable to detect and adds real complexity — see Technical Notes)
- [~] AC2: Supports core commands: select/delete/navigate by word, sentence, paragraph; undo/redo — word/sentence-level select/delete, undo/redo, go-to-start/end, new line all implemented; **paragraph-level not implemented** (no reliable OS-level "paragraph" concept to hook); sentence-level is a same-line approximation (Home→Shift+End), won't span multiple lines correctly
- [ ] AC3: Command grammar is user-extensible — not built; current command set is a fixed list in `command_mode.rs`, not user-configurable

#### Technical Notes
This is the highest-risk/highest-effort epic — evaluate build vs. license/interop with Talon's grammar approach before committing engineering time (see Open Questions in PRD). **MVP decision:** built in-house rather than evaluating Talon interop, scoped deliberately narrow — fixed substring-based phrase matching (not a real grammar/intent parser), reusing the existing dictation pipeline (same hotkey-toggle/audio-capture/ASR path, branching only on what happens to the final text). Talon-style grammar remains a future upgrade if the fixed phrase set proves too limiting in practice.

#### Change History
- [2026-07-04] Created from REQ-008
- [2026-07-04] MVP shipped: `src/shell/src/command_mode.rs` (phrase matching + SendInput key-combo dispatch) wired into `main.rs` via a second hotkey and a shared `Mode` state read by the reader thread. 3 unit tests passing on the matcher logic. Not yet tested live (built and compiles, hotkeys register cleanly, but no live voice test performed yet).

---

### STORY-010: Voice-coding primitives
**Status:** Backlog
**Priority:** P1
**PRD Ref:** REQ-009
**Last Updated:** 2026-07-04

**As a** developer,
**I want** to dictate identifiers in camelCase/snake_case/PascalCase and speak symbols by name,
**So that** I can write code and comments by voice without fighting the formatter.

#### Acceptance Criteria
- [ ] AC1: Case-style commands ("camel case get user name") produce correctly formatted identifiers
- [ ] AC2: Common symbols (brackets, operators) speakable by name and inserted correctly
- [ ] AC3: A "code comment mode" preset applies code-appropriate tone/formatting automatically

#### Change History
- [2026-07-04] Created from REQ-009

---

### STORY-011: Snippet / macro library
**Status:** Backlog
**Priority:** P2
**PRD Ref:** REQ-013
**Last Updated:** 2026-07-04

**As a** user,
**I want** to trigger frequently-used templates by voice,
**So that** I don't have to redictate boilerplate text.

#### Acceptance Criteria
- [ ] AC1: User can record a text snippet and assign a voice trigger phrase
- [ ] AC2: Snippets support variable placeholders (e.g., recipient name) filled at insertion time
- [ ] AC3: Snippet library is exportable/importable (see STORY-020)

#### Change History
- [2026-07-04] Created from REQ-013

---

## Epic D: Privacy & Trust

### STORY-012: On-device processing mode
**Status:** Backlog
**Priority:** P0
**PRD Ref:** REQ-010, NREQ-007
**Last Updated:** 2026-07-04

**As a** privacy-sensitive user,
**I want** a mode where all audio processing happens on my device,
**So that** my voice and text never leave my machine.

#### Acceptance Criteria
- [ ] AC1: On-device mode achieves core dictation (ASR + basic formatting) with zero network connectivity
- [ ] AC2: A persistent, unambiguous UI indicator shows whether current processing is local or cloud
- [ ] AC3: Edge case: features that require cloud (e.g., an advanced cleanup model not available on-device) degrade gracefully with a clear message, never silently fall back to cloud

#### Change History
- [2026-07-04] Created from REQ-010, NREQ-007

---

### STORY-013: No undisclosed screen/context capture
**Status:** Backlog
**Priority:** P0
**PRD Ref:** REQ-011, NREQ-009
**Last Updated:** 2026-07-04

**As a** user,
**I want** confidence that the app never captures my screen content without my explicit, visible consent,
**So that** I can trust it with sensitive work (directly responds to the Wispr Flow screenshot-capture incident).

#### Acceptance Criteria
- [ ] AC1: Any screen/window content capture capability is off by default
- [ ] AC2: If enabled, a persistent on-screen indicator is visible for the entire duration capture is active
- [ ] AC3: An in-app, user-readable data-flow log records every instance of non-audio data capture, timestamped and per-app

#### Change History
- [2026-07-04] Created from REQ-011, NREQ-009

---

### STORY-014: Dictation history with diff and revert
**Status:** In Progress
**Priority:** P1
**PRD Ref:** REQ-012
**Last Updated:** 2026-07-07

**As a** user,
**I want** to see my dictation history and compare AI-edited output against my raw transcript,
**So that** I can verify the AI didn't change my meaning and revert if needed.

#### Acceptance Criteria
- [x] AC1: History is searchable and grouped by time period — dashboard history view: full-text search over raw+cleaned, grouped Today/Yesterday/date
- [x] AC2: Diff view shows raw transcript vs. AI-edited output — implemented as an inline word-level diff (del/ins highlighting) rather than side-by-side columns; reads better for sentence-length utterances
- [x] AC3: One-click revert to raw transcript available from history — "Copy raw (revert)" button copies the raw transcript to the clipboard; InkVoice cannot rewrite text already inserted into another app, so clipboard is the honest revert mechanism
- [ ] AC4: Live-verified by the user against real dictations

#### Technical Notes
History persisted by the sidecar to `history.jsonl` (text only; audio stays in the rotating debug_audio dump). Served by the local dashboard (`dashboard.py` on 127.0.0.1:43918).

#### Change History
- [2026-07-04] Created from REQ-012
- [2026-07-07] Shipped on the local dashboard: history store + search + time grouping + inline raw-vs-cleaned diff + copy-raw revert. AC2 wording "side by side" satisfied via inline diff (deliberate — noted above). Not yet live-verified.

---

### STORY-015: Compatibility hardening for terminals and IDEs
**Status:** Blocked
**Priority:** P0
**PRD Ref:** NREQ-008, NREQ-006
**Last Updated:** 2026-07-04
**Blocker:** Windows Terminal blocks all synthetic input. Two independent injection mechanisms tried and confirmed dead ends: `SendInput` (`ACCESS_DENIED`, AppContainer/MSIX sandboxing) and `WriteConsoleInput` (`AttachConsole` itself returns `ACCESS_DENIED` against the owning shell process). Accepted as a documented platform limitation for now, not an actively-worked bug — see CONTEXT.md. Only remaining untried avenue is UI Automation (unverified whether it supports text insertion, not just read-only screen-reader access), not attempted. Revisit only on new information.

**As a** developer,
**I want** InkVoice to never break my terminal keybindings or silently alter my OS accessibility settings,
**So that** I can run it alongside my existing dev environment without side effects.

#### Acceptance Criteria
- [ ] AC1: Verified non-interference with WSL terminals, Cursor's integrated terminal, and Windows Terminal default keybindings — **Windows Terminal: FAILS outright** (dictation cannot insert text at all, not a keybinding conflict but a total input block); classic console hosts (cmd.exe launched directly) work correctly; WSL and Cursor not yet tested (Cursor not installed on dev machine)
- [ ] AC2: App never modifies OS-level accessibility settings without explicit user action — not yet tested
- [ ] AC3: Regression test suite covers this compatibility set before every release — `src/bin/sendinput_test.rs` built as a reusable standalone harness (launches any target app, runs the real `type_text` path, reports exact Win32 error codes); not yet wired into an automated CI-style suite, currently run manually

#### Technical Notes
Directly addresses a named, documented Wispr Flow failure mode — treat as a release-blocking test gate, not a nice-to-have. **Update:** this story is currently failing its own AC1 for the single most common Windows terminal app. See docs/CONTEXT.md "STORY-015 Compatibility Testing" for full diagnosis (confirmed via `GetLastError` = `ERROR_ACCESS_DENIED`, both for Unicode character injection and plain VK keypresses).

#### Change History
- [2026-07-04] Created from NREQ-008, NREQ-006
- [2026-07-04] Built `sendinput_test` harness; ran against Windows Terminal and classic `cmd.exe`. Found and fully diagnosed a hard blocker on Windows Terminal (AppContainer sandboxing rejects `SendInput` entirely). Status moved to Blocked pending a `WriteConsoleInput`-based fix.
- [2026-07-04] Built `writeconsoleinput_test` harness; confirmed `WriteConsoleInput` is ALSO blocked (`AttachConsole` itself denied against the owning shell process, an earlier/different failure than the SendInput block). Decided to accept as a documented platform limitation rather than continue debugging; moving to M3's next item (Command Mode design).

---

## Epic E: Platform Coverage

### STORY-016: macOS and Windows desktop clients (v1 launch)
**Status:** Backlog
**Priority:** P0
**PRD Ref:** REQ-014, NREQ-003
**Last Updated:** 2026-07-04

**As a** user,
**I want** a lightweight native desktop app on macOS and Windows,
**So that** I get full functionality without the resource overhead competitors have.

#### Acceptance Criteria
- [ ] AC1: Idle memory usage under 150MB, idle CPU under 2%
- [ ] AC2: Feature parity between macOS and Windows clients at launch
- [ ] AC3: Auto-update mechanism in place

#### Change History
- [2026-07-04] Created from REQ-014, NREQ-003

---

### STORY-017: Linux client (X11 + Wayland)
**Status:** Backlog
**Priority:** P1
**PRD Ref:** REQ-014
**Last Updated:** 2026-07-04

**As a** Linux-using developer,
**I want** a first-class InkVoice client,
**So that** I'm not excluded from the category the way I am by every current competitor.

#### Acceptance Criteria
- [ ] AC1: Functional on both X11 and Wayland session types
- [ ] AC2: Global hotkey and text-insertion work via portal APIs where direct access is restricted (Wayland)
- [ ] AC3: Feature parity with macOS/Windows for core dictation (Command Mode may lag per platform API availability — tracked separately if so)

#### Change History
- [2026-07-04] Created from REQ-014

---

### STORY-018: iOS and Android mobile clients
**Status:** Backlog
**Priority:** P2
**PRD Ref:** REQ-014
**Last Updated:** 2026-07-04

**As a** mobile user,
**I want** system keyboard-level dictation on my phone,
**So that** I can use InkVoice anywhere I type, not just at my desk.

#### Acceptance Criteria
- [ ] AC1: Implemented as a custom keyboard (iOS keyboard extension / Android IME)
- [ ] AC2: Core dictation and Personal Dictionary sync from desktop
- [ ] AC3: Works within platform mic-permission and background-processing constraints

#### Change History
- [2026-07-04] Created from REQ-014

---

### STORY-019: Browser extension
**Status:** Backlog
**Priority:** P2
**PRD Ref:** REQ-014
**Last Updated:** 2026-07-04

**As a** user,
**I want** a browser extension for Chromium and Firefox,
**So that** dictation works reliably in web-app text fields even where OS-level insertion is unreliable.

#### Acceptance Criteria
- [ ] AC1: Works in common web text fields (contenteditable, textarea, rich text editors like Gmail/Notion)
- [ ] AC2: Respects site permissions and does not activate mic without explicit user action
- [ ] AC3: Shares dictionary/settings sync with desktop client

#### Change History
- [2026-07-04] Created from REQ-014

---

### STORY-020: Dictionary/snippet import-export
**Status:** Backlog
**Priority:** P2
**PRD Ref:** REQ-020
**Last Updated:** 2026-07-04

**As a** user or team admin,
**I want** to import/export personal dictionaries and snippets,
**So that** I can back them up or share them across a team.

#### Acceptance Criteria
- [ ] AC1: Export produces a portable file format (e.g., JSON)
- [ ] AC2: Import validates and merges without duplicating existing entries
- [ ] AC3: Team admin can push a shared dictionary to all seats (ties to STORY-022)

#### Change History
- [2026-07-04] Created from REQ-020

---

## Epic F: Monetization

### STORY-021: Free, Pro subscription, and lifetime license tiers
**Status:** Backlog
**Priority:** P0
**PRD Ref:** REQ-018
**Last Updated:** 2026-07-04

**As a** prospective user,
**I want** a free tier to try the product and a one-time-purchase option alongside subscription,
**So that** I'm not forced into a subscription I'm unsure about (addresses a documented pricing complaint against Wispr Flow).

#### Acceptance Criteria
- [ ] AC1: Free tier has a clearly communicated weekly/monthly word cap with graceful cutoff messaging
- [ ] AC2: Pro subscription unlocks unlimited words and premium features (Command Mode, on-device mode, priority support)
- [ ] AC3: Lifetime license tier available at a fixed one-time price, clearly scoped (e.g., major-version updates included, feature-tier equivalence to Pro)

#### Change History
- [2026-07-04] Created from REQ-018

---

## Epic G: Enterprise

### STORY-022: Admin console for Team/Enterprise
**Status:** Backlog
**Priority:** P1
**PRD Ref:** REQ-015
**Last Updated:** 2026-07-04

**As an** IT/security admin,
**I want** a console to manage seats, view usage, and enforce policy,
**So that** I can safely roll out InkVoice across my organization.

#### Acceptance Criteria
- [ ] AC1: Seat provisioning/deprovisioning from the console
- [ ] AC2: Org-wide policy toggles including "force on-device mode" and "disable cloud fallback"
- [ ] AC3: Usage analytics visible per seat (word volume, feature adoption) without exposing dictated content

#### Change History
- [2026-07-04] Created from REQ-015

---

### STORY-023: SSO (SAML) and SCIM provisioning
**Status:** Backlog
**Priority:** P1
**PRD Ref:** REQ-016
**Last Updated:** 2026-07-04

**As an** enterprise IT admin,
**I want** SAML SSO and SCIM user provisioning,
**So that** InkVoice integrates with our existing identity provider and offboarding process.

#### Acceptance Criteria
- [ ] AC1: SAML SSO login flow functional with at least one major IdP (Okta, Azure AD) tested
- [ ] AC2: SCIM provisioning/deprovisioning reflected in the admin console within a defined SLA
- [ ] AC3: Deprovisioned users lose access immediately, including on already-authenticated devices

#### Change History
- [2026-07-04] Created from REQ-016

---

### STORY-024: Configurable data retention / zero-retention mode
**Status:** Backlog
**Priority:** P0
**PRD Ref:** REQ-017, NREQ-005
**Last Updated:** 2026-07-04

**As an** enterprise compliance officer,
**I want** to configure a zero-retention mode where no audio or transcript is stored server-side,
**So that** we can adopt InkVoice under strict data-handling requirements (legal, healthcare, finance).

#### Acceptance Criteria
- [ ] AC1: Zero-retention mode processes audio in-memory only, with no server-side persistence post-response
- [ ] AC2: Setting is org-wide enforced from the admin console, not per-user overridable
- [ ] AC3: Compliance documentation (SOC 2 Type II report, data flow diagrams) available to enterprise customers on request

#### Change History
- [2026-07-04] Created from REQ-017, NREQ-005

---
