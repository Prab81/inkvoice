# Build Log

> Running, durable state of the build. The `Current State` section must always be sufficient for a fresh session (or post-compaction context) to resume work with zero conversation memory. Dated entries below are append-only history.

---

## Current State

**Milestone:** M3 started (terminal/IDE compatibility regression pass, STORY-015). Found a real, currently-unresolved blocker: Windows Terminal rejects all synthetic input. STORIES.md brought up to date after being stale all session.
**Last checkpoint:** 2026-07-04 — Checkpoint 10.

**Hotkeys are now Ctrl+Shift+\` (dictation) and Ctrl+Shift+9 (Command Mode)** — moved off Ctrl+Alt entirely (AltGr reinterpretation risk on non-US keyboard layouts opened Notepad Settings instead of typing text — see CONTEXT.md). Update any test instructions accordingly; Ctrl+Alt+I and Ctrl+Alt+\` are both stale references now.

**KNOWN LIMITATION, ACCEPTED (STORY-015):** InkVoice cannot dictate into Windows Terminal. Two independent mechanisms tried and confirmed blocked: `SendInput` (`ACCESS_DENIED`) and `WriteConsoleInput` (`AttachConsole` itself denied against the owning shell process). Classic console hosts (`cmd.exe` launched directly) are unaffected. User decided (2026-07-04) to document this as a platform limitation rather than keep debugging — UI Automation is the only untried avenue, unverified and not attempted. Do not re-open without new information.

**Command Mode MVP shipped (STORY-009):** second hotkey (Ctrl+Alt+9) toggles a hands-free editing mode; fixed phrase matching dispatches real key combos (undo/redo, select/delete word or same-line "sentence", go-to-start/end, new line). Built and compiles clean, hotkeys register, 3 unit tests passing on the matcher — **not yet live-voice-tested.**

**Overlay v2 shipped:** live partial text now visible inside the pill (was previously invisible — only ever printed to a hidden console); waveform redrawn with soft anti-aliased brush strokes + temporal amplitude smoothing (EMA) to fix real-mic jitter that a clean synthetic test hadn't revealed. Verified via `overlay_test` + screenshot; **not yet confirmed by the user against real live speech.**

**Locked decisions (full rationale in CONTEXT.md):**
- ASR: **dual-model**, not single-model. Streaming Zipformer (`2023-06-21`, LibriSpeech+GigaSpeech) drives live partials (~0.2ms latency, zero flicker); Parakeet-TDT-0.6B-v3 int8 still produces the final committed text once per utterance (unchanged accuracy/punctuation, now cheaper since it no longer redecodes periodically). Neither a bigger offline model (1.1B) nor a Qwen-based model would have fixed the smoothness problem — it was an offline-vs-streaming architecture mismatch, not a model-size problem. Both run CPU-only; GPU untested but unnecessary so far.
- Shell audio: single dedicated owner thread for the `cpal::Stream` (required — it's `!Send`), self-retrying Start/Stop design; survives BT mic drop/reconnect mid-recording.
- Text insertion: `SendInput` chunked in groups of 6 chars with a 4ms gap (fixes an observed character-drop race).
- Cleanup pipeline: rule-based only so far (verbal commands, self-correction collapsing, filler removal, list formatting — deliberately conservative after a false-positive incident, entry on "M2 Cleanup Pipeline"). Local-LLM cleanup pass still not started.
- Personalization: fuzzy dictionary correction shipped as an interim (entry on "Accent Testing..."); true model-level hotwords confirmed technically possible but blocked on obtaining the model's original tokenizer artifact.
- Orchestration: Sonnet (this session); Opus escalation-only; Haiku/Hermes for low-complexity token-heavy work (none delegated yet — all work so far has needed design judgment inline).

**What exists, concretely:**
- Full docs set: PRD, STORIES (24 stories, all Backlog — none formally marked In Progress/Done yet despite substantial work against STORY-001/002/003/004/005/007; a status pass is overdue), ARCHITECTURE, CONTEXT, CHANGELOG, EXECUTION_PLAN, TEST_SCENARIOS.
- `spikes/m0_asr/`: M0 latency harness (reference methodology), venv, model files.
- `src/sidecar/asr_server.py`: persistent ASR server (TCP JSON-lines: `begin`/`audio`/`end` → `ready`/`partial`/`final`). **Now dual-model:** every audio chunk streams through the online Zipformer for an instant partial; Parakeet decodes the buffered audio once at `end` for the final (bounded to a 20s trailing window — utterances >20s only have their trailing ~20s transcribed, documented, not yet solved). Partial text is uppercase/unpunctuated (Zipformer's native output); final text is properly cased/punctuated (Parakeet's) — intentional asymmetry, not a bug.
- `src/sidecar/cleanup.py`: rule-based transcript cleanup, applied to finals only. 20 unit tests passing.
- `src/sidecar/personal_dictionary.py` + `personal_dictionary.json` (seeded with "Vihan"): fuzzy proper-noun correction. 5 unit tests passing.
- `src/sidecar/debug_audio/`: every finalized utterance's raw audio + raw/cleaned transcript, rotating (last 40) — added so future bad transcripts are reproducible, not just describable.
- `src/shell/` (Rust, `inkvoice-shell`): hotkey (Ctrl+Alt+I — Ctrl+Alt+Space was taken on this machine), WASAPI capture via cpal with self-healing reconnect, SendInput text insertion, and `overlay.rs` — a borderless cream-pill recording indicator with a live layered waveform, bottom-center of screen, spawned/stopped in lockstep with recording. Rendering uses `UpdateLayeredWindow` + a manual 32bpp alpha DIB (an initial color-key-based approach rendered as a solid black rectangle — see Checkpoint 7 — and was replaced). `src/bin/overlay_test.rs`: standalone visual-debug harness (synthetic waveform, no mic/hotkey needed) — use this plus a screenshot for any future overlay visual work.
- Git repo initialized; `.gitignore` covers venvs/models/build artifacts/debug audio. No commits made (not requested).
- `tests/`: `test_cleanup.py` (20 passing), `test_personal_dictionary.py` (5 passing).

**M0 final results (CPU only, no GPU used):** files P50=54ms/P99=102ms; live mic P50=90ms/P99=148ms.

**Next action:** User to confirm the overlay now looks/behaves right live (fixed and visually verified via screenshot harness — see Checkpoint 7 — but not yet confirmed by the user against the real running app with real speech). Also still open: a dedicated live re-verification of the SendInput chunking fix and a real mid-recording Bluetooth off/on cycle (code paths exist, not isolated in a clean test). STORIES.md status is stale and should be updated to reflect actual progress.

**Bugs found live and fixed during M1 (see CONTEXT.md entry 5 for full detail):**
1. Opening the WASAPI stream once at app startup raced Windows' Bluetooth A2DP→HFP profile switch and died silently. **Fix:** redesigned to a single dedicated audio-owner thread (`cpal::Stream` is `!Send`, so it must be created/held/dropped on one thread) that takes lightweight Start/Stop commands and self-retries whenever it wants to be recording but has no live stream — this also transparently handles a Bluetooth mic dropping and coming back **mid-recording**, per explicit user requirement.
2. `SendInput` sending an entire utterance as one large batch could drop/scramble characters in the target window (observed once). **Fix:** chunked into groups of 6 characters with a 4ms gap between groups, plus a warning log if `SendInput`'s return value ever reports fewer events accepted than sent.

**Open problems / risks:**
- SendInput chunking fix not yet re-verified live (next action).
- Mid-recording Bluetooth off/on reconnect implemented but not yet verified live under an actual disconnect (only the empty-stream-at-startup case was observed and is now handled by the same code path).
- 1.1B accuracy upgrade path (own ONNX export or WSL2/TensorRT) unscheduled — revisit after M1.
- Long-dictation (>20s continuous, no pause) transcription truncation is a known, documented gap — not a regression, a scoped-out capability.
- Wayland insertion approach remains unvalidated (not blocking M0–M3).

---

## History

### [2026-07-04] — Checkpoint 0: Planning complete
- Competitive research on Wispr Flow + category done (see CONTEXT.md entry 1).
- PRD v1.0 drafted: 20 REQs, 9 NREQs, 5 personas, measurable "great" bars (WER <6%, latency <300ms P50, <150MB idle).
- 24 stories across 7 epics created, all Backlog.
- ASR model decision made: Parakeet-TDT-1.1B over Canary-Qwen (streaming transducer vs. batch-oriented LLM decoder; CONTEXT.md entry 2).
- Execution plan written: Windows-first milestone ladder M0–M4, model-routing table (Sonnet orchestrates, Opus escalation-only, Haiku for token-heavy grunt work), checkpoint/compaction protocol.
- Shell + cleanup-layer open questions resolved by default: Tauri + Python sidecar; rule-based → local LLM cleanup (CONTEXT.md entry 3).
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 1: M0 spike, file-based latency gate PASS
- Spike env built: Python 3.11 venv + sherpa-onnx (CPU wheel), Parakeet-TDT-0.6B-v3 int8 ONNX downloaded/extracted.
- `spike_latency.py` written: chunked pseudo-streaming (250ms chunks, 20s sliding window cap), reports P50/P90/P99/RTFx vs. 300ms gate.
- Result on model test WAVs, **CPU only**: P50=54ms P90=87ms P99=102ms RTFx=4.3 → **PASS**. en/de/es transcribed correctly with punctuation.
- Implication: GPU (CUDA/TensorRT) is an optimization, not a requirement — de-risks the non-NVIDIA on-device fallback (NREQ-007) significantly.
- Deviation noted: 0.6B-v3 used instead of locked 1.1B (no published ONNX; NeMo poor on native Windows). Resolution deferred to M0 close-out.
- Remaining for M0: live-mic run, WER baseline set.
- Hermes delegation route confirmed: `hermes -z "<prompt>"` (headless one-shot); EXECUTION_PLAN routing table updated.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 3: M1 vertical slice live, two bugs found and fixed
- Scaffolded `src/sidecar/asr_server.py` (streaming ASR over TCP JSON-lines) and `src/shell/` (Rust cargo project, cpal capture, SendInput typing, RegisterHotKey with fallback candidates). Git repo initialized.
- First live test: hotkey worked, but zero audio reached the sidecar — stream opened once at startup died within ~1s (`device no longer available`), silently, no recovery. Root cause: WASAPI stream open raced a Bluetooth A2DP→HFP profile switch. Compounded by a machine crash mid-session that also reset Windows' default input device to a non-functional NVIDIA Broadcast virtual mic.
- User explicitly required robustness to repeated Bluetooth mic off/on cycles, not just the one-time startup race. Redesigned audio capture around a single dedicated owner thread (required — `cpal::Stream` is `!Send` and can't move across threads once created) that self-retries on a timer whenever it wants to record but has no live stream, and treats a stream's own error callback the same way a "Stop" would — just drop and let the retry loop reopen. This handles first-open races, mid-recording drops, and repeated cycles all through one code path.
- Second live test: full pipeline worked — accurate streaming transcription with live self-correction, correct final transcript. One utterance came through the sidecar correctly but was typed into Notepad garbled ("But and I'm trying to build..." arrived as "ut anbuild..."). Diagnosed as a `SendInput` batch-size race, not an ASR problem (the log's `final` text was already correct). Fixed by chunking `SendInput` into groups of 6 chars with a 4ms gap, plus logging if the OS reports fewer events accepted than sent.
- Next: re-verify the SendInput fix and the mid-recording Bluetooth reconnect path live.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 5: Recording-indicator overlay
- User referenced a video (couldn't be watched directly — WebFetch only returns page metadata for YouTube, not frames) then shared a screenshot: a cream rounded pill with a layered, hand-drawn-looking waveform. Asked user to pick implementation approach (AskUserQuestion) between a quick native GDI overlay now vs. bringing up the full planned Tauri shell early; user chose the quick native overlay.
- Built `src/shell/src/overlay.rs`: borderless, click-through (WS_EX_TRANSPARENT), non-activating, always-on-top popup window using plain GDI + `SetLayeredWindowAttributes`/`LWA_COLORKEY` for transparency (not true per-pixel alpha — simpler, hard-edged pill boundary as a known tradeoff). Runs on its own dedicated thread (window handles are thread-affine); spawned on recording start, stopped on recording stop via a `PostThreadMessageW`-based stop signal.
- Waveform: live RMS amplitude computed in the existing audio callback, streamed to the overlay thread over a new per-recording channel threaded through `AudioCmd::Start(Sender<f32>)`; drawn as 3 overlapping offset/phased polylines to approximate the reference's layered look without real alpha blending.
- Compile fixes needed: added `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_System_Threading` features; `SetLayeredWindowAttributes`/`LWA_COLORKEY` actually live in `Win32_UI_WindowsAndMessaging`, not `Win32_Graphics_Gdi` (user32, not gdi32) — first build attempt guessed wrong.
- Built and launched successfully; **not yet visually confirmed by the user against the running app** — built from a static screenshot only.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 6: Accent testing, hotwords investigation, Personal Dictionary v1
- User reported a garbled transcript ("...oooooooose around...") and asked for accent/voice-type robustness testing plus voice/name personalization. Root cause of that specific garble could NOT be determined — shell.log had been overwritten by later restarts, no raw audio survived. Fixed going forward: `save_debug_utterance()` in `asr_server.py` now dumps every finalized utterance's audio + raw/cleaned transcript to `src/sidecar/debug_audio/` (rotating, last 40).
- Accent proxy test: only 2 TTS voices available on this machine (Hazel/en-GB, Zira/en-US) — real accent diversity untested, flagged as a gap. Ran 4 sentences through the actual pipeline. "Vihan" name recognized correctly by both synthetic voices (suggests earlier live jitter was natural speech variability, not a model defect). Zira mis-heard "Kubernetes" as "Cuba Ernets" (expected jargon gap).
- **Found and fixed a real bug:** Zira's "p.m." broke `collapse_self_corrections` — internal abbreviation periods were treated as sentence boundaries, isolating a correction cue from the text it should have overridden, and mangling "p.m." into "p. m." via the trailing space-restoration step. Fixed with an abbreviation-protection placeholder swap; 2 regression tests added (20 total passing in `test_cleanup.py`).
- Investigated sherpa-onnx `hotwords_file` (true model-level biasing) for personalization: confirmed it works, but requires `decoding_method="modified_beam_search"` (not our `greedy_search` default) and hotword entries pre-tokenized into the model's SentencePiece vocabulary — the tokenizer artifact needed for that isn't bundled in this ONNX export. Scoped as a real follow-up, not implemented now.
- Shipped `src/sidecar/personal_dictionary.py` instead: a JSON term list (seeded "Vihan") plus conservative fuzzy correction (capitalized tokens only, `difflib` ratio ≥ 0.6) applied after cleanup. 5 unit tests passing, including a check against false-correcting an unrelated name ("Vikram").
- Restarted sidecar/shell clean with all fixes; no import errors.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 7: Overlay black-rectangle bug found and fixed
- User reported the overlay rendered as a solid black rectangle, no cream color, no waveform.
- Built `src/bin/overlay_test.rs` — a standalone harness that shows the overlay with synthetic sine-wave amplitude data, no mic/hotkey needed — specifically to get a screenshot-based visual ground truth instead of debugging blind.
- Instrumented every GDI call in the paint path; all reported success (`FillRgn` x2, `BitBlt`, `SetLayeredWindowAttributes`), yet the screenshot still showed solid black with square corners — the color-key compositing approach (raw `GetDC`/`BitBlt` painting, no `WM_PAINT` validation cycle) wasn't reliably compositing despite every call individually succeeding.
- Replaced the rendering path with `UpdateLayeredWindow` + a manually-managed 32bpp top-down DIB section with real per-pixel alpha (GDI draws RGB only; a post-pass sets alpha=255 wherever a pixel isn't the untouched zeroed background). Verified fixed via the same screenshot harness: correct cream pill, layered waveform, correct compositing against the desktop behind it.
- Rebuilt the main `inkvoice-shell` binary (shares `overlay.rs`) and relaunched sidecar+shell; **user confirmed working live** (screenshot: correct cream pill, layered waveform, working transparency).
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 8: Dual-model ASR architecture (streaming Zipformer + Parakeet)
- User rated current smoothness/speed "1/10" vs. Wispr Flow and asked whether Parakeet-1.1B or a Qwen model would help. Diagnosed root cause first: Parakeet is offline; our sidecar fakes streaming via full redecode of the growing buffer every 250ms — that's why partials flicker and why latency scales with utterance length. Neither a bigger offline model nor a Qwen-based one (heavier, still offline) would fix this.
- Downloaded and spiked sherpa-onnx's streaming Zipformer (`spikes/streaming_zipformer/spike_streaming.py`): true cache-aware incremental decode. Result: P50=0.2ms, **zero revisions** (tokens never change once emitted) vs. Parakeet's ~90ms with constant flicker.
- First candidate model (`2023-06-26`, LibriSpeech-only) was fast/stable but worse than Parakeet on our accent/jargon test set and has no punctuation/casing at all. On a real Bluetooth-mic recording it was notably rough. Tried `2023-06-21` (LibriSpeech+GigaSpeech, broader training data): substantially more accurate on both the accent test set (Kubernetes, "call mom" now correct) and the real mic recording, same latency. Selected this variant.
- Rewrote `asr_server.py`: `UtteranceDecoder` now runs both models — Zipformer streams every audio chunk for the live partial (no more throttling; it's cheap enough to run continuously), Parakeet still does a one-shot decode at `end` for the final (unchanged accuracy/punctuation, now actually cheaper — no periodic redecode during the utterance).
- Verified end-to-end via the sidecar's own protocol on a real recording: first partial at ~40ms (from ~90ms+ before), final text unaffected and correct.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 9: Shell not running (root cause) + hotkey moved off a letter key
- User reported Ctrl+Alt+I causing an "I" to appear in italics in the focused app, no overlay, nothing recorded.
- Checked live process state: `inkvoice-shell.exe` was not running at all (died or never relaunched). With nothing registering the global hotkey, the keystroke fell through to the focused app, which — like most rich-text editors — binds plain Ctrl+I to italics. One root cause explained all three symptoms.
- Changed hotkey candidates: Ctrl+Alt+Space (unchanged, first choice) → **Ctrl+Alt+\`** (backtick, not I) → Ctrl+Shift+Space → Ctrl+Shift+\`. Rationale: any letter-key global hotkey risks collision with some app's text-formatting accelerator; backtick essentially never is bound to one.
- Rebuilt, relaunched sidecar+shell, confirmed `inkvoice-shell.exe` actually running this time (checked via `Get-Process`, not just log output) before handing back to user.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 10: M3 started — STORY-015 compatibility pass finds a real blocker
- User confirmed Ctrl+Alt+Tab can't be used (probed and confirmed: reserved by Windows for the task switcher). Probed a batch of alternatives with a throwaway `hotkey_probe` binary; Ctrl+Alt+\` confirmed genuinely free (not just free-because-our-own-app-holds-it).
- Started M3's terminal/IDE compatibility regression suite (STORY-015). Extracted `type_text` into `src/text_insert.rs` (shared between the shell and a new standalone test binary) and built `src/bin/sendinput_test.rs` to test the real insertion code path against arbitrary target apps without needing the mic/hotkey pipeline running.
- **Found a real, unresolved P0 blocker:** Windows Terminal rejects `SendInput` entirely (`GetLastError` = `ACCESS_DENIED`), confirmed for both Unicode character injection and plain VK-based keypresses — ruling out tuning fixes and clipboard-paste workarounds (both go through the same blocked API). Root cause: MSIX/AppContainer sandboxing. Classic `cmd.exe` (launched directly, not via Windows Terminal) is unaffected — confirms this is sandboxing-specific, not a console-hosting problem generally.
- Identified but did not implement the fix: `WriteConsoleInput`, a console-specific Win32 API that bypasses the blocked synthetic-input pipeline.
- Updated STORIES.md (stale all session) to reflect real status: STORY-001/002/003/004/005/007 moved to In Progress with honest per-AC notes; STORY-015 moved to **Blocked** with the finding as its blocker.
- User given a 3-way choice on priority (fix now / design Command Mode first / test WSL first); chose fix now.
- Built `writeconsoleinput_test`: confirmed `WriteConsoleInput` is ALSO blocked — `AttachConsole` itself returns `ACCESS_DENIED` against the actual owning shell process (a different, earlier failure than the SendInput block), strongly suggesting deliberate ConPTY/Windows-Terminal security isolation rather than an incidental gap.
- Presented the user a second 3-way choice (try UI Automation / document as limitation / suggest a terminal-setting change); chose to document and move on.
- STORY-015 blocker text and CONTEXT.md updated to reflect final status: known, accepted platform limitation, not an actively-worked bug.
- Designed and confirmed scope for STORY-009 (Command Mode MVP) with the user before coding: second hotkey for mode switch (not voice wake-phrase), fixed phrase matching (not a real grammar), core verb set (select/delete/undo/redo/navigate/new-line).
- Built `src/shell/src/command_mode.rs` (matcher + SendInput key-combo dispatch) and wired it into `main.rs`: added a second hotkey (Ctrl+Alt+9), a `Mode` enum shared via `Arc<Mutex<>>` between the message loop and reader thread, and branched the reader thread's final-transcript handling on mode (type vs. match-and-dispatch). Extracted `register_first_free` as a reusable fn (was becoming duplicated between the two hotkey registrations).
- 3 unit tests added for the matcher (pure logic, no OS dependency) — all passing. Compiles clean, both hotkeys register successfully on relaunch. Live voice test not yet performed.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 11: Overlay v2 — live partial text + soft anti-aliased waveform
- User reported live partial text was completely invisible (only ever printed to hidden dev console) and the waveform looked much less impressive live than the reference video it was built from.
- Diagnosed the waveform gap: rendering was only ever validated against a clean synthetic sine wave; real mic RMS is frame-to-frame noisy, which the same renderer faithfully reproduced as visible jitter — the renderer wasn't broken, the test signal wasn't representative.
- Rewrote `overlay.rs`: (1) EMA temporal smoothing on incoming amplitude samples before they enter history; (2) waveform strokes redrawn as soft anti-aliased brush stamps directly on the pixel buffer (replacing hard-edged `Polyline`), with linear-interpolation upsampling of the amplitude history for smoother curves; (3) alpha/base-fill reworked to use a `point_in_pill` geometry test instead of the old "did GDI touch this pixel" heuristic, needed for AA blending to compose correctly; (4) live partial text added via `DrawTextW`, fed through a new `Sender<String>` channel threaded from `overlay::spawn()` (now returns a 3-tuple) through the shell's reader thread.
- First text-rendering attempt (CLEARTYPE_QUALITY, normal weight) came out too faint to read — caught via the same screenshot-verification method used for the original black-rectangle bug, not assumed correct. Fixed with ANTIALIASED_QUALITY + semibold weight; confirmed legible via screenshot.
- Pill resized 320×84 → 420×140 to fit the text line. Rebuilt, relaunched sidecar+shell successfully.
- Not yet confirmed by the user against real live speech (verified via synthetic test data only, same caveat as the original overlay build).
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 12: Ctrl+Alt hotkeys reinterpreted as AltGr — moved to Ctrl+Shift
- User tested overlay v2: confirmed live partial text IS now visible (progress). But reported that pressing Ctrl+Alt+\` to stop opened Notepad's Settings window instead of typing the final text — Notepad never received it.
- Diagnosed likely root cause: Ctrl+Alt is interpreted as AltGr on non-US keyboard layouts, and the OS can reinterpret the whole combo as a different character/shortcut — modern Windows apps commonly bind Ctrl+, to Settings, a plausible AltGr target. This means every Ctrl+Alt hotkey tried so far (including the earlier Ctrl+Alt+I fix) carried this same latent risk.
- Reordered hotkey candidates in `main.rs` to prefer Ctrl+Shift combos (never reinterpreted as AltGr on any layout) over Ctrl+Alt. Dictation: Ctrl+Shift+\`. Command Mode: Ctrl+Shift+9. Ctrl+Alt variants kept only as last-resort fallback.
- Rebuilt, relaunched — both hotkeys registered successfully as Ctrl+Shift combos.
- Not yet re-verified live; this is a well-reasoned fix for the reported symptom, not a confirmed-via-reproduction root cause.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 4: M2 cleanup pass implemented, live false positive fixed
- Implemented `src/sidecar/cleanup.py` (verbal commands, self-correction collapsing, filler removal, list formatting) per STORY-003/004/005/REQ-019, pulled forward from M2 into current work at user's request. Wired into `asr_server.py`'s `finalize()`. 15 initial unit tests passing.
- Live test (several minutes of natural continuous dictation) found a real bug: list-detection triggered on ordinary prose ("the first two... and then...") and destroyed real content by chopping it into a bogus list, dropping words. Not a formatting quibble — a correctness bug.
- Fixed: dropped "next/then/finally/lastly" from the trigger set (worst prose false-triggers), require 3+ numeric ordinals (first..tenth) in strictly increasing order to fire at all. Added regression tests reproducing the exact failure and an out-of-order case. 18 tests passing.
- Checked sidecar logs for the several empty-final sessions in the same test run — no decode errors; legitimate no-confident-speech results, not a bug. One known limitation reconfirmed live: 27.2s session correctly truncated to trailing 20s.
- Sidecar and shell restarted with the fix; not yet re-tested live.
- Opus escalations this period: none.

### [2026-07-04] — Checkpoint 2: M0 close-out — GO
- Live-mic test initially blocked: Bose QC Ultra (Bluetooth LE Audio) mic silent through all PortAudio paths (MME/DS/WASAPI, 16k and native 48k); Windows mic consent verified Allow; Windows' own sound test worked. ffmpeg DirectShow capture succeeded (RMS 0.032) → **PortAudio, not hardware/permissions, is the broken layer for LE Audio mics.**
- Live-mic latency+accuracy on the ffmpeg capture: P50=90ms P90=136ms P99=148ms, RTFx=2.8, transcript accurate. **GATE PASS.**
- M0 verdict: **GO.** Model decision revised to Parakeet-TDT-0.6B-v3 int8 (CONTEXT.md entry 4); WER numeric baseline deferred to M1.
- TEST_SCENARIOS: TS-M0-01 Pass, TS-M0-02 Pass, TS-M0-03 Deferred.
- Opus escalations this period: none.
