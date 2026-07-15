# Context Log

## [2026-07-04] — Competitive Research: Wispr Flow & Voice Dictation Category
**Type:** Discovery
**Impact:** High

### Context
Kicking off the InkVoice project (a Wispr Flow–class AI voice dictation product). Before writing a PRD, researched Wispr Flow's actual feature set, pricing, tech approach, and — critically — its documented weaknesses, plus the broader competitive landscape (Superwhisper, Aqua Voice, MacWhisper, Willow Voice, Dragon, Talon, Otter.ai) and current ASR state-of-the-art benchmarks.

### Decision / Finding
Key findings that shaped the PRD:
- **Privacy incident:** Wispr Flow was found silently screenshotting the active window and sending it to cloud servers via an undisclosed "Context Awareness" feature (surfaced on Reddit); the company initially banned the user who exposed it before the CTO apologized. This is reflected in a stark rating gap: Trustpilot 2.7/5 vs. App Store 4.8/5 vs. Product Hunt 4.9/5 — experienced users trust it far less than new users.
- **Latency gap:** Official marketing claims <700ms P99 latency; real-world user reports describe 1-2 seconds perceived latency. No public confirmation of the underlying ASR model; LLM cleanup confirmed (via a Baseten case study with Wispr's CTO) to use a fine-tuned Llama model served via TensorRT-LLM.
- **Platform/power-user gaps:** No Linux, iPad, Chromebook, or VM/remote-desktop support. Command Mode (real editing-by-voice) is Pro-tier gated and less capable than Talon's grammar-based system. No offline/on-device mode at all — always cloud-processed.
- **Resource footprint:** ~800MB RAM / 8% CPU idle on Windows (Electron-based), described by users as "incredibly heavy," ~4x competitors.
- **Compatibility bugs:** Breaks in WSL terminals, Cursor's terminal, silently alters VS Code accessibility settings, conflicts with Windows Terminal keybindings.
- **Pricing:** $15/mo (Pro monthly) / $12/mo annual, Team $10-12/user/mo, Enterprise custom. No lifetime/one-time-purchase option, which some reviewers cite as a reason to prefer alternatives (e.g., Superwhisper's $249.99 lifetime, MacWhisper's one-time €59-64).
- **State of the art ASR (as of research date):** Nvidia Canary-Qwen (5.63% WER) and Parakeet-TDT (6.32% WER) and AssemblyAI Universal-3 Pro (4.5-5.9% WER) all beat Whisper-class WER on clean English. Deepgram streaming achieves ~250ms latency. Apple's on-device SpeechAnalyzer (iOS 26) is 55% faster than Whisper large-v3-turbo with zero network round-trip, demonstrating on-device is now performance-viable, not just a privacy compromise.

### Rationale
Rather than building a straight feature clone, we chose to position InkVoice against Wispr Flow's specific, documented failure points (trust, latency-vs-marketing honesty, platform coverage, power-user depth) because these are durable, hard-to-copy differentiators — a feature-for-feature clone would just become a slower follower in a category the incumbent already dominates on brand awareness and funding ($700M valuation, reportedly raising toward $2B).

### Implications
- PRD (@docs/PRD.md) sets explicit, measurable bars (WER <6%, latency <300ms P50, idle memory <150MB) benchmarked directly against the researched competitive data, not arbitrary targets.
- On-device processing is scoped as a first-class v1 mode, not a post-launch add-on — this has architecture implications (need an ASR model small enough to run on-device at target latency; see @docs/ARCHITECTURE.md open questions).
- Command Mode is scoped to be Talon-competitive, which is a larger engineering lift than Wispr's simpler "transforms" — flagged as a scope/timeline risk in STORIES.md for the P0 stories under that epic.
- Linux support is a deliberate v1 wedge (zero major competitors currently support it) — adds platform-testing surface area early, which is a real cost worth tracking.
- No architecture or vendor decisions (ASR model choice, Command Mode engine build-vs-license, desktop shell framework) have been made yet — these are logged as Open Questions in the PRD and must be resolved before implementation begins per the Task Execution Protocol in CLAUDE.md.

---

## [2026-07-04] — ASR Model Selection: Parakeet-TDT-1.1B over Canary-Qwen
**Type:** Decision
**Impact:** High

### Context
Needed to resolve the ASR foundation model open question from the initial PRD. Two open-weight candidates were compared: NVIDIA Parakeet-TDT and NVIDIA Canary-Qwen (SALM architecture, FastConformer encoder + Qwen LLM decoder). Target dev/reference hardware is an RTX 5070 Ti (16GB VRAM), which initially raised the question of whether GPU headroom made Canary-Qwen's higher resource needs a non-issue.

### Decision / Finding
Selected **Parakeet-TDT-1.1B** as the primary ASR engine for both on-device and cloud modes. Canary-Qwen was rejected despite scoring marginally better on WER (5.63% vs. Parakeet's ~6.3%).

### Rationale
The deciding factor was architecture, not compute budget. Parakeet-TDT is a transducer model that streams tokens incrementally as audio arrives — a structural fit for the <300ms P50 latency requirement (NREQ-001) and streaming partial-results requirement (STORY-002). Canary-Qwen's LLM-decoder design needs a chunk of audio before the LLM can decode, and its RTFx (throughput) is far lower than Parakeet's due to LLM decoding overhead — this is a design mismatch with live dictation that more GPU headroom does not fix. The 5070 Ti's 16GB does, however, justify running the larger Parakeet-TDT-1.1B variant instead of the leaner 0.6B, closing most of the accuracy gap to Canary-Qwen while preserving the streaming behavior.

### Implications
- @docs/ARCHITECTURE.md updated: Parakeet-TDT-1.1B is now the named model for both on-device (GPU via CUDA/TensorRT, with a CPU/ONNX sherpa-onnx fallback for non-NVIDIA hardware) and cloud inference paths.
- @docs/PRD.md Open Questions list reduced accordingly; the ASR model line item is resolved and logged under Decisions Made.
- A new open item was added: the CPU/ONNX fallback path (for Apple Silicon, AMD/Intel GPUs, CPU-only machines) has not been benchmarked yet and needs a latency/accuracy spike before it's trusted as the universal non-NVIDIA on-device fallback.
- Canary-Qwen remains worth revisiting only if InkVoice ever adds a batch/offline transcription feature (e.g., meeting transcription) — explicitly a non-goal for v1 per the PRD, so this is not scheduled.

---

## [2026-07-04] — Execution Plan: Scope Correction, Stack Defaults, Model Routing
**Type:** Decision
**Impact:** High

### Context
Moving from planning to execution. The PRD roadmap (5 platforms, enterprise tier, monetization) is a funded-team scope, not a buildable sequence for this environment (solo build, Windows 11 dev machine, RTX 5070 Ti). Two open questions (desktop shell, LLM cleanup model) still blocked implementation, and the user requested a model-routing strategy for token-efficient execution plus a checkpoint/compaction discipline.

### Decision / Finding
Wrote @docs/EXECUTION_PLAN.md. Key decisions:
1. **Windows-first milestone ladder (M0–M4):** M0 ASR feasibility spike as a hard go/no-go gate before any app code; then core dictation MVP → cleanup/personalization → power-user layer → hardening. macOS/Linux/mobile/browser and Epics F/G (monetization, enterprise) deferred to M5+.
2. **Desktop shell:** Tauri (Rust + system webview) with a Python sidecar process for GPU inference. Rust owns hotkey capture and text insertion (SendInput/UI Automation). Electron rejected on the NREQ-003 footprint budget; Python-only rejected because the shell/UI shouldn't carry the ML runtime's memory.
3. **LLM cleanup:** rule-based pass in M1; local small instruct model (Qwen3-4B-class via llama.cpp, sharing the GPU) in M2. Cloud cleanup deferred until a backend exists.
4. **Command Mode:** minimal in-house grammar in M3; Talon interop evaluated only if that proves insufficient.
5. **Model routing:** Sonnet orchestrates (planning, integration, review, decisions); Opus is escalation-only for hard/expensive-to-reverse problems, each escalation justified in BUILD_LOG.md; Haiku handles low-complexity token-heavy work (boilerplate, repetitive tests, fixtures, doc drafting, log analysis) — the role the user called "Hermes," which is not an available model tier here.
6. **Checkpoint/compaction protocol:** BUILD_LOG.md `Current State` is the durable resume point; checkpoints at every milestone/story completion and proactively when a session nears the context-summarization region, so compaction never loses load-bearing state. Exact-token-count compaction triggering is not controllable — the protocol makes compaction lossless instead.

### Rationale
Risk-first sequencing: local streaming ASR latency is the single existential risk, so it's validated before any app code. Sonnet-as-orchestrator over Opus: orchestration is routing/judgment over moderate context, where Sonnet is ~5x cheaper with equivalent outcomes; Opus's depth advantage only pays on single hard problems, hence escalation-only.

### Implications
- @docs/BUILD_LOG.md and @docs/TEST_SCENARIOS.md created; both are now mandatory checkpoint artifacts.
- PRD Open Questions on shell framework is resolved; pricing and compliance-cert-order questions remain open but are M5+ concerns.
- STORIES.md priorities unchanged, but execution order now follows EXECUTION_PLAN.md milestones, not story priority alone (e.g., P0 enterprise stories deliberately deferred).

---

## [2026-07-04] — M0 Spike Results: GO. Model revised to 0.6B-v3; audio-capture stack finding
**Type:** Decision | Discovery
**Impact:** High

### Context
M0 feasibility spike executed on the reference machine (Windows 11, RTX 5070 Ti, Python 3.11 + sherpa-onnx). Chunked pseudo-streaming harness (`spikes/m0_asr/spike_latency.py`, 250ms chunks, 20s sliding window).

### Decision / Finding
1. **GO — latency gate passed with large margin, on CPU alone.** Test WAVs: P50=54ms/P99=102ms. Live mic: P50=90ms/P99=148ms with an accurate transcript. GPU (CUDA/TensorRT) is now an optimization lever, not a requirement.
2. **ASR model decision revised: Parakeet-TDT-0.6B-v3 int8 ONNX is the MVP model**, superseding the earlier 1.1B choice. Reasons: no published ONNX export of the 1.1B exists (NeMo, its native runtime, is Linux-first and impractical on native Windows); 0.6B-v3 passes latency on CPU with qualitatively strong multilingual accuracy (en/de/es with punctuation/casing). 1.1B remains a post-M1 accuracy upgrade candidate (own ONNX export or WSL2/TensorRT serving). PRD Decisions entry on 1.1B is superseded by this entry.
3. **Discovery — PortAudio cannot capture from Bluetooth LE Audio microphones on Windows.** The user's Bose QC Ultra (LE Audio) mic delivered pure silence through every PortAudio path (MME/DirectSound/WASAPI, both 16kHz and native 48kHz), while Windows' own sound test and ffmpeg's DirectShow capture worked fine (RMS 0.032 vs 0.0001). Root cause is in PortAudio's handling of LE Audio endpoints, not permissions (mic consent verified Allow) or hardware.

### Rationale
Ship-what-runs: 0.6B-v3 is the only Parakeet variant with a supported Windows inference path today, and it clears every M0 bar. Deferring the WER numeric baseline (TS-M0-03) to M1 is acceptable because latency was the existential risk; accuracy is qualitatively validated and will be measured against a proper recording set once M1's capture pipeline can collect one.

### Implications
- **The production audio-capture layer must not use PortAudio.** The Rust core should capture via WASAPI directly (e.g., cpal or windows-rs) and must be explicitly tested against LE Audio Bluetooth mics — this exact class of device is a documented Wispr Flow failure mode ("Bluetooth mic freezes"), so handling it robustly is a differentiator. Added to ARCHITECTURE constraints.
- M1 can begin: scaffold Tauri shell + Python sidecar, wire hotkey → capture → streaming decode → text insertion.
- `spikes/m0_asr/` retained as the reference harness; the int8 model directory (~640MB) is the model asset M1 will load.

---

## [2026-07-04] — M1 Vertical Slice: Two Live Bugs, Both Fixed
**Type:** Discovery | Bug
**Impact:** High

### Context
Built the first end-to-end M1 vertical slice: `src/sidecar/asr_server.py` (persistent ASR process, TCP JSON-lines protocol wrapping the M0-validated chunked-redecode approach) and `src/shell/` (Rust: hotkey via `RegisterHotKey` with a fallback candidate list, mic capture via `cpal`, text insertion via `SendInput`). Tested live against real speech and a real Bluetooth headset (the same Bose QC Ultra from M0), including recovering from an actual machine crash mid-session.

### Decision / Finding
**Bug 1 — audio stream died silently at startup, no recovery.** The shell originally opened one `cpal` WASAPI stream for the app's entire lifetime. It died within ~1 second every single launch (`device no longer available`) — timing strongly suggests a race with Windows switching the Bluetooth headset from A2DP (music) to HFP (mic) profile the moment the stream opened. After that, the app looked alive (hotkey worked, "recording" state toggled) but captured nothing, forever — an entirely silent failure mode. This was masked initially by an unrelated issue (a machine crash reset the OS default input device to a non-functional NVIDIA Broadcast virtual mic, which produces near-silent audio even when "working").

**User requirement surfaced during the fix:** the audio layer must recover not just from a one-time startup race but from a Bluetooth mic going offline and coming back online repeatedly, at any point including mid-recording.

**Fix — single dedicated audio-owner thread.** `cpal::Stream` wraps a COM object and is deliberately `!Send` — it can never move between threads once created, which ruled out an initial design where a background "reconnector" thread would build a new stream and hand it to the main thread. The corrected design: one thread owns capture for the app's entire lifetime, holds the `Stream` locally, and receives only lightweight `Start`/`Stop` commands over a channel. It retries on a timer (400ms) whenever it should be recording but has no live stream — which uniformly covers first-open races, the stream's own error callback firing mid-recording (treated identically to needing a fresh open), and any number of repeated off/on cycles, all through the same retry loop rather than special-cased recovery logic.

**Bug 2 — `SendInput` batch-size race dropped/scrambled characters.** One utterance (out of several) was transcribed correctly by the ASR (confirmed in the sidecar log) but arrived in Notepad garbled ("But and I'm trying to build..." → "ut anbuild..."). The whole utterance had been sent as a single large `SendInput` batch; the receiving window's message queue can't always keep up with a large burst of synthetic Unicode key events, which is a known class of Win32 input-injection issue, not specific to this app. Fixed by chunking into groups of 6 characters with a 4ms gap between groups, and logging whenever `SendInput`'s return value reports fewer events accepted than sent (so silent drops become visible instead of just occasionally-wrong text).

### Rationale
Both fixes follow the same principle: prefer a design that structurally can't silently fail over one that happens to work most of the time. The single-owner-thread audio design isn't just a bug fix, it's the right architecture given `Stream`'s `!Send` constraint — worth encoding as a standing note for whoever touches audio capture next (already commented in `main.rs`).

### Implications
- `src/shell/src/main.rs`'s audio-capture section and `type_text` now carry inline comments explaining why the design is shaped this way — read those before "simplifying" either.
- This is a genuine differentiator opportunity: "Bluetooth mic freezes" is a named, documented Wispr Flow complaint (@docs/CONTEXT.md entry 1). InkVoice's shell now self-heals through this class of failure by construction, not as a patch — worth calling out in future positioning/marketing material once there's a product to market.
- STORY-001 and STORY-002 acceptance criteria are substantially validated by this live test (real hotkey, real streaming partials, real text insertion into a real app) but not yet formally marked Done — see STORIES.md status.
- Remaining before M1 can close: re-verify the SendInput fix and the mid-recording reconnect path live (not yet re-tested after the fixes), then run the terminal/IDE compatibility pass (STORY-015) before declaring the vertical slice solid.

---

## [2026-07-04] — M2 Cleanup Pipeline: Live False Positive Destroyed Real Content
**Type:** Bug | Discovery
**Impact:** High

### Context
Pulled forward the M2 rule-based cleanup pass (STORY-003/004/005/REQ-019: verbal punctuation commands, filler removal, self-correction collapsing, list formatting) ahead of finishing M1 hardening, per user request, since dictation quality can't really be evaluated without it. Implemented as `src/sidecar/cleanup.py`, applied only to final transcripts (never partials, since these transforms need full-sentence context and would cause visible flicker on partials). Shipped with 15 passing unit tests, then tested live against extended natural dictation (several minutes across multiple sessions).

### Decision / Finding
The list-formatting heuristic (v1: any 2 distinct ordinal words anywhere in the text → treat as an enumerated list) fired on ordinary narrative speech and **destroyed real content**, not just misformatted it. Live example: dictating a description of a story-generation app feature got mangled from correct prose into a bogus 2-item list with words silently dropped ("speakers to" vanished entirely). Root cause: "first", "then", "next", "finally" are extremely common ordinary English words in narration, not just list markers — "any 2 distinct ordinal words" was far too weak a signal on continuous natural dictation (as opposed to the artificial "first... second... third..." test inputs it was designed against).

**Fix, deliberately conservative:** (1) dropped "next"/"then"/"finally"/"lastly" from the trigger set entirely — they're the worst offenders for false-triggering in prose; (2) require the remaining numeric ordinals (first..tenth) to appear in **strictly increasing order** (first, then second, then third...) — a real spoken list reliably satisfies this, coincidental prose usage essentially never does; (3) raised the minimum count from 2 to 3. Added regression tests reproducing the exact failure shape from the live log, plus an out-of-order case, so this class of bug can't silently return.

Separately confirmed via sidecar logs: several sessions in the same test run produced empty final transcripts. Checked for decode errors (none found) — these were legitimate "no confidently-decodable speech" results (hesitant "uh"/pauses), not a bug. One real, already-documented limitation did surface: a 27.2s continuous session was correctly truncated to its trailing 20s per the known MAX_WINDOW_S scope-out.

### Rationale
A false positive in a "helpful formatting" feature is qualitatively worse than a missed opportunity to format — it silently corrupts what the user actually said, which is a correctness bug, not a quality-of-life gap. The fix trades recall (fewer real lists get auto-formatted, e.g. ones using "next"/"then" instead of numeric ordinals) for precision (near-zero chance of corrupting ordinary prose), which is the right tradeoff for a transform that runs unconditionally on every dictation.

### Implications
- `cleanup.py`'s list-detection section now carries an inline comment explaining the false-positive history — read it before loosening the heuristic again.
- This is a concrete argument for the already-planned M2 local-LLM cleanup pass (EXECUTION_PLAN.md): an LLM with actual sentence understanding won't confuse "the first two" (a quantity phrase) with a list marker the way surface regex does. The rule-based pass should be treated as a conservative floor, not the final answer, for exactly this class of ambiguity.
- No STORIES.md status change yet — STORY-003/004/005 remain In Progress; this was a fix within already-in-progress work, not a completion.

---

## [2026-07-04] — Accent Testing, Hotwords Investigation, Personal Dictionary v1
**Type:** Discovery | Decision | Bug
**Impact:** Medium

### Context
User reported a garbled live transcript ("...oooooooose around...") and asked (1) why, (2) for accent/voice-type robustness testing, and (3) about training the model to their own voice/name ("Vihan" was inconsistently recognized as Vihal/Vihar/V Han across partials, though the earlier live session's final transcript did resolve correctly to "Vihan").

### Decision / Finding
**The garbled session itself couldn't be root-caused** — the shell had been restarted several times since (each restart truncates `shell.log`, its only record), so the raw audio and exact ASR output no longer exist. This is a real gap, not just bad luck: there was no durable record of a session's audio to replay. Fixed going forward: `asr_server.py` now dumps every finalized utterance's raw audio (WAV) plus its raw/cleaned transcript to `src/sidecar/debug_audio/`, rotating to keep the most recent 40. Any future garbled output is now reproducible and debuggable, not just describable secondhand.

**Accent/voice-type testing:** only two TTS voices are installed on this machine (Microsoft Hazel, en-GB; Microsoft Zira, en-US) — a limited proxy for real human accent diversity, but useful for a quick smoke test. Ran 4 sentences (name recognition, list dictation, technical jargon, self-correction) through both voices via the actual pipeline (ASR + cleanup). Findings:
- "Vihan" transcribed correctly by both synthetic voices — suggests the earlier live jitter was natural speech variability (pacing/mic), not a systemic model defect.
- Zira's voice: "Kubernetes" → "Cuba Ernets" — confirms the known, expected technical-jargon weak spot.
- **New bug found:** Zira's "p.m." broke `collapse_self_corrections` — the abbreviation's internal period was treated as a sentence boundary, so a correction cue ("actually") ended up isolated from the text it should have overridden, and the trailing space-restoration step further mangled "p.m." into "p. m.". Fixed by protecting known abbreviations (a.m./p.m./u.s./Mr./Mrs./Ms./Dr./vs./etc.) with a placeholder before sentence-splitting, restored after. Regression tests added.

**Hotwords / model-level personalization investigated and found non-trivial:** confirmed live that sherpa-onnx's `hotwords_file` mechanism works with this model, but only under `decoding_method="modified_beam_search"` (not our default `greedy_search`), and — critically — hotword entries must be pre-tokenized into the model's SentencePiece subword vocabulary (confirmed via `tokens.txt`: pieces like `▁s`, `er`, `▁p`, not whole words). The original tokenizer artifact needed to do that encoding isn't bundled in this ONNX export. This is a real, scoped task (locate/obtain the matching tokenizer, or a sherpa-onnx conversion utility), not a quick wire-up.

**Shipped instead, as a working interim:** `src/sidecar/personal_dictionary.py` — a small JSON word list (seeded with "Vihan") plus a conservative fuzzy-correction pass applied after cleanup: only capitalized tokens (proper-noun heuristic) that are a close-but-not-exact match (`difflib` ratio ≥ 0.6) to a dictionary entry get swapped to the correct spelling. 5 unit tests passing, including a check that an unrelated capitalized word ("Vikram") is *not* falsely corrected.

### Rationale
Model-level hotwords are the architecturally "correct" fix and should still be pursued (better precision, no reliance on the ASR having gotten *close* to the right word already) — but they require infrastructure (the tokenizer) we don't currently have, and the user's actual problem (name recognition) needed a working answer now. Fuzzy post-correction is a legitimate, if less powerful, interim: it directly fixes the reported symptom with a small, testable, easily-audited module.

### Implications
- STORY-007 (Personal Dictionary) is now partially implemented (fuzzy correction slice) but the full spec (auto-learning from corrections, hotwords-based model biasing, UI for managing entries) remains open. Track "obtain Parakeet-TDT-0.6B-v3 tokenizer artifact for true hotwords support" as a follow-up — see also the already-tracked 1.1B upgrade path, since both involve sourcing model artifacts beyond the bundled ONNX export.
- Accent/voice testing via synthetic TTS is a real but limited tool — only 2 voices available locally, both female, no non-native-English-accent coverage. Real human accent testing (or a larger TTS voice pack) is needed before claiming accent robustness with any confidence; noted as a gap, not resolved.
- Debug audio capture (`src/sidecar/debug_audio/`) is now a standing diagnostic tool — check it first for any future "why did it transcribe this weirdly" report, before re-deriving from text logs alone.

---

## [2026-07-04] — Overlay Rendered as a Black Rectangle: Colorkey Transparency Replaced with Real Alpha
**Type:** Bug | Decision
**Impact:** Medium

### Context
User reported the recording-indicator overlay showed as a plain black rectangle with square corners — no cream color, no waveform. The original implementation used plain GDI drawing (raw `GetDC`/`BitBlt`, no `WM_PAINT` validation cycle) plus `SetLayeredWindowAttributes`/`LWA_COLORKEY` to punch out a near-black "transparent key" color.

### Decision / Finding
Built a standalone screenshot-based test harness (`src/bin/overlay_test.rs`, runs the overlay with synthetic amplitude data with no mic/hotkey needed) specifically to get visual ground truth rather than debug blind. Instrumented every GDI call in the paint path with success/failure logging — every single call reported success (`FillRgn` for both the transparent-key fill and the cream pill fill, `BitBlt`, `SetLayeredWindowAttributes` all returned success codes), yet the window still rendered as solid black with hard corners. This means the color-key compositing mechanism itself wasn't reliably picking up the drawn content when combined with a manual `GetDC`/`BitBlt` painting cycle outside the normal `WM_PAINT`/`BeginPaint`/`EndPaint` flow — a known-fragile combination, confirmed here rather than assumed.

**Fix:** replaced the entire rendering path with `UpdateLayeredWindow` and a manually-managed 32bpp top-down DIB section with real per-pixel alpha — the API Win32 actually provides for exactly this use case. GDI draws RGB only into the DIB; a post-pass sets alpha=255 for any pixel that isn't the untouched (0,0,0,0) background (valid since none of the overlay's drawing colors are pure black). Verified fixed via the same screenshot harness: cream rounded pill, layered waveform, and correct compositing against whatever's behind it (confirmed with a real desktop screenshot showing background content bleeding through as expected around the pill).

### Rationale
Rather than keep patching the color-key approach on faith, building a way to actually *see* the result (screenshot harness) turned "seems broken" into "confirmed broken via instrumented API calls" and then "confirmed fixed via a visual before/after" — the right sequence for a rendering bug that's otherwise impossible to reason about blind.

### Implications
- `src/bin/overlay_test.rs` is a reusable tool for any future overlay visual work — run it and screenshot rather than relying on live user reports for iteration.
- The module doc comment in `overlay.rs` now records why `UpdateLayeredWindow` was chosen over color-key, so this doesn't get "simplified" back to the broken approach later.
- Edges are still hard-cut (alpha is 0 or 255, no antialiasing) — matches the originally-accepted tradeoff for a "quick native overlay," just via a mechanism that actually works. Antialiased edges would need a coverage-based alpha computation at the pill boundary — not done, not currently needed.

---

## [2026-07-04] — Dual-Model ASR Architecture: Streaming Zipformer for Partials, Parakeet for Finals
**Type:** Decision | Discovery
**Impact:** High

### Context
User assessed the product's current smoothness/speed at "1/10" against Wispr Flow and asked whether a bigger offline model (Parakeet-1.1B) or a Qwen-based model would help. Root-caused the actual problem first: Parakeet-TDT is an *offline* model; the sidecar fakes streaming by re-decoding the entire growing buffer from scratch every 250ms (M1 approach). This means every partial is an independent full inference — visible words flicker/revise constantly — and per-partial compute grows with utterance length. Neither a bigger offline model nor a heavier LLM-decoder model (Qwen-Audio, Canary-Qwen) fixes this; both would make the redecode slower, not smoother.

### Decision / Finding
Spiked a genuinely streaming architecture: sherpa-onnx's streaming Zipformer models (cache-aware incremental decoding — the encoder carries state forward, the decoder extends rather than redoes). Measured via `spikes/streaming_zipformer/spike_streaming.py`: **P50 latency 0.2ms and zero revisions** (a token, once emitted, never changes) — versus Parakeet's chunked-redecode ~90ms with constant flicker.

**Accuracy trade-off found and resolved through iteration:**
1. First candidate (`2023-06-26`, LibriSpeech-only training) was fast and stable but measurably worse than Parakeet on our own accent/jargon test set (Vihan→VIGREN, Kubernetes→CUBANETTIS, "Meet me at"→"MEAT MADE A") and has **no built-in punctuation/casing at all** (LibriSpeech-style all-caps output) — a real regression if it replaced Parakeet outright.
2. On a real Bluetooth-mic recording (not clean TTS), this model's live partial output was notably rough ("OR BY MARBLE I GUESS TEN THIGHS ILL WON'T LET ME KNOW...") — a bigger domain gap on real-world audio than on studio-quality test files.
3. Tried a second candidate (`2023-06-21`, LibriSpeech+GigaSpeech training — broader, noisier source data) at the same latency: substantially better on both the accent test set (Kubernetes and "call mom" now correct) and the real mic recording ("I MARRIBLE I GUES TEN SECONDS OVER LET ME KNOW IF YOU WANT TO RECORDED AND TESTED THIS ONE" — still imperfect but far closer to the actual speech). Selected this variant.

**Resulting architecture — both models run, each doing what it's good at:**
- **Streaming Zipformer (2023-06-21)** drives the live partial display — near-instant, stable, no flicker. This is what fixes the perceived smoothness/speed complaint, since it's what's visible while the user is talking.
- **Parakeet-TDT-0.6B-v3** still produces the FINAL committed text, once per utterance (same one-shot call as the old `finalize()`) — same accuracy and free punctuation/casing as before. This is actually *cheaper* than the M1/M2 design since Parakeet no longer redecodes periodically during the utterance, only once at the end.

Confirmed live via the sidecar's own protocol (`test_dual_model.py`-style test): first partial at ~40ms, final text unaffected and correct ("I guess ten seconds over, let me know if you've already recorded and tested.").

### Rationale
This is the textbook right-tool-for-the-job split: use the model built for live incremental feedback where liveness matters (partials), and the model with better raw accuracy where correctness matters (the committed final text the user actually keeps). Neither model alone was the right answer; combining them was.

### Implications
- `src/sidecar/asr_server.py` rewritten: `UtteranceDecoder` now holds both an `OfflineRecognizer` (Parakeet) and an `OnlineRecognizer` (streaming Zipformer); `feed()` streams through the online model every audio chunk (no more `PARTIAL_INTERVAL_S` throttling — it's cheap enough to run on every chunk), `finalize()` unchanged in spirit (one-shot Parakeet decode over the buffered audio).
- Partial text is now uppercase, unpunctuated (Zipformer's native output) while final text remains properly cased/punctuated (Parakeet's). This asymmetry is intentional but should be called out in any future UI work on the overlay/partial display so it doesn't read as a bug.
- `spikes/streaming_zipformer/` now holds two model variants; `2023-06-21` is the one actually wired into the sidecar. `2023-06-26` kept for reference/comparison, not deleted.
- Open follow-up: STORY-007's hotwords blocker (needs a SentencePiece tokenizer artifact) applies to whichever model — worth checking if the Zipformer models ship a usable `bpe.model` (the 06-26 download included one; 06-21 did not) as a *lower-effort* path to real contextual biasing than sourcing Parakeet's tokenizer, since it would only need to improve the live partial, not the final text's accuracy.

---

## [2026-07-04] — Hotkey Letter Collision (Ctrl+Alt+I) and Shell Not Running
**Type:** Bug
**Impact:** Medium

### Context
User reported pressing the hotkey caused an "I" to appear in italics in whatever app was focused, no overlay appeared, and nothing was recorded.

### Decision / Finding
Checked live process state: **the shell (`inkvoice-shell.exe`) was not running at all** — it had died or was never relaunched after an earlier restart. With no process registering the global hotkey, Ctrl+Alt+I fell straight through to the focused app, which — like most rich-text editors — binds plain Ctrl+I to italics; the extra Alt didn't stop the app's own accelerator from matching. This single root cause explained all three symptoms (unintercepted keystroke, no overlay, no recording), not three separate bugs.

Also changed the hotkey away from a letter key on general principle, per the user's own suggestion: **Ctrl+Alt+I → Ctrl+Alt+`** (backtick). Letter keys are always at risk of collision with some app's text-formatting accelerator (I=italic, B=bold, U=underline are extremely common bindings); backtick essentially never is. Kept Ctrl+Alt+Space as the first candidate and added Ctrl+Shift+\` as a further fallback.

### Rationale
Relaunching fixed the immediate symptom, but the hotkey change is a durability improvement independent of that — even when the app *is* running, a letter-key global hotkey is one dropped RegisterHotKey call (or one app with its own low-level keyboard hook) away from this exact symptom recurring.

### Implications
- No code bug was found in the hotkey/audio/overlay pipeline itself — this was a process-lifecycle issue (something killed or never restarted the shell) compounded by a fragile hotkey choice.
- Worth eventually adding a supervisor/watchdog (auto-restart on crash) once the app moves past manual dev-loop restarts — not needed yet, but this is the second time a "why isn't anything working" report traced back to a silently-dead process.

---

## [2026-07-04] — STORY-015 Compatibility Testing: SendInput Fully Blocked on Windows Terminal
**Type:** Discovery | Decision
**Impact:** High

### Context
Started the M3 terminal/IDE compatibility regression pass (STORY-015) — one of the named, documented Wispr Flow failure modes ("breaks in WSL terminals... conflicts with Windows Terminal") that InkVoice is meant to avoid by construction. Built `src/bin/sendinput_test.rs`, a standalone harness sharing the real `type_text` code path (`src/text_insert.rs`, extracted from `main.rs` for this purpose) that launches a target app and types a test string into it, independent of the mic/hotkey pipeline.

### Decision / Finding
**Windows Terminal rejects all synthetic input from InkVoice with `ERROR_ACCESS_DENIED` (Win32 error 5).** Confirmed at two levels: (1) the full dictated-text `SendInput` call (Unicode character events) reported 0/12 events accepted every time; (2) a follow-up test using a plain VK-based keypress (no `KEYEVENTF_UNICODE`, the kind of event a literal physical keyboard would generate) got the *same* `ACCESS_DENIED` — ruling out a workaround that swaps injection style, and also ruling out a clipboard+Ctrl+V-paste workaround, since simulating that keypress goes through the same blocked API. The automation session also couldn't `Stop-Process` the Windows Terminal process at all ("Access is denied") — a second, independent symptom of the same protection boundary. This is consistent with Windows Terminal's MSIX/Store packaging running inside an AppContainer sandbox, which restricts what unpackaged desktop processes (like InkVoice) can inject into it — a known Win32 platform behavior, not a bug in our code.

**Classic console hosts are unaffected.** The same test against `cmd.exe` launched directly (not routed through Windows Terminal) accepted every event with no errors — confirming the block is specific to AppContainer-sandboxed targets, not a general problem with console-hosted apps or with our `type_text` implementation.

**Caveat surfaced but not fully verified:** Windows 11 has a system-wide "Default Terminal Application" setting (Settings → For Developers) that, when set to "Windows Terminal" (a common modern default), routes even direct `cmd.exe`/`powershell.exe` launches through the same sandboxed host. This machine's `cmd.exe` test succeeding suggests that setting is currently on the legacy console host here — but that means the successful result is machine-configuration-dependent, not a guarantee it'll hold on every user's Windows 11 install.

### Rationale
Diagnosing to a specific Win32 error code (`ACCESS_DENIED` via `GetLastError`) rather than stopping at "it doesn't work" was worth the extra few minutes — it turned a vague compatibility complaint into a specific, well-understood platform restriction with a known class of fix, rather than something that might get "tuned around" with retries or timing changes (it won't; access denial isn't a timing issue).

### Implications
- **Real fix path identified, not yet implemented:** `WriteConsoleInput`, a Win32 API purpose-built for programmatic console input (writes directly into a console's input buffer rather than going through the OS-wide synthetic-input pipeline that AppContainer blocks). This is the standard mechanism automation tools use to reach sandboxed consoles. Scoped as its own implementation task, not folded into this testing pass.
- Until that's built, **InkVoice cannot dictate into Windows Terminal at all** on a system where it's the default terminal host — this is a real, currently-open gap, not a false alarm, and should be treated as release-blocking for the developer persona (Dana) per the PRD, same severity as the compatibility issues already fixed for Bluetooth mics and SendInput batching.
- `src/bin/sendinput_test.rs` and `src/text_insert.rs` (extracted from `main.rs`) are reusable for testing any future target app without needing the full mic/hotkey pipeline running.
- STORY-015's remaining scope (WSL terminal, VS Code/Cursor if installed — Cursor isn't installed on this dev machine) still needs testing; not done in this pass.

---

## [2026-07-04] — Windows Terminal Input Block: WriteConsoleInput Also Fails; Accepted as a Documented Limitation
**Type:** Discovery | Decision
**Impact:** Medium

### Context
Follow-up to the `SendInput`/`ACCESS_DENIED` finding above. `WriteConsoleInput` (a console-specific injection API, bypassing the OS-wide synthetic-input pipeline that blocked `SendInput`) was identified as the standard fix for reaching sandboxed consoles and tested directly rather than assumed to work.

### Decision / Finding
**`WriteConsoleInput` also fails, and at an earlier stage than expected.** Built `src/bin/writeconsoleinput_test.rs`: focuses the target terminal, walks the foreground process's descendant tree, and tries `AttachConsole` + `WriteConsoleInputW` against every candidate PID. With a real Windows Terminal window focused (3 PowerShell tabs open):
- `WindowsTerminal.exe` and all three `OpenConsole.exe` (ConPTY host) processes: `ERROR_INVALID_HANDLE` — they don't expose an attachable classic console.
- All three `powershell.exe` processes (the actual shells, which do own a console): **`ERROR_ACCESS_DENIED` on `AttachConsole` itself** — blocked before even reaching the point of writing input. This is a different, earlier failure than the `SendInput` block, which strongly suggests the ConPTY/Windows Terminal ecosystem deliberately restricts cross-process console attachment as its own security boundary, not merely as a side-effect of blocking keystroke injection.

Both identified fix paths are now confirmed dead ends via direct testing, not assumption. A third avenue (UI Automation — a COM-based accessibility channel, architecturally unrelated to keyboard/console injection) was identified but not attempted: Windows Terminal supports UIA for Narrator compatibility, but whether it supports text *insertion* (vs. read-only access for screen readers) is unverified, and implementing COM UIA interop from Rust is a materially bigger lift than either approach tried so far.

**Decision: accept this as a documented platform limitation for now, not a bug to keep chasing.** Two confirmed independent blocking mechanisms (input injection AND console attachment) is a strong signal this is an intentional security boundary, not an oversight — the kind of restriction unlikely to have an easy userland workaround. Revisit only if UI Automation's write support is confirmed cheaply, or if this becomes a hard adoption blocker.

### Rationale
Continuing to probe alternative injection techniques without a clear signal that the next one would work risks sinking disproportionate effort into a single compatibility edge case. Documenting the limitation honestly (as Wispr Flow itself does for its own terminal compatibility gaps) is the more defensible product decision than an open-ended debugging spiral.

### Implications
- STORY-015 (@docs/STORIES.md) stays **Blocked**, but the blocker is now "known platform limitation, workaround not found" rather than "fix path identified, not yet implemented" — a meaningfully different status; don't re-attempt `WriteConsoleInput` without new information.
- `src/bin/sendinput_test.rs` and `src/bin/writeconsoleinput_test.rs` remain as reusable diagnostic tools if this is revisited later (e.g., to test a UI Automation approach, or to re-check if a future Windows/Windows Terminal update changes this behavior).
- User-facing implication (for whenever there's a UI to surface it in): InkVoice should detect when the focused window is Windows Terminal and show a clear, honest message ("dictation isn't supported in Windows Terminal — try a classic console window instead") rather than silently failing, once there's a UI surface capable of that kind of feedback.

---

## [2026-07-04] — Command Mode MVP (STORY-009)
**Type:** Decision
**Impact:** Medium

### Context
Moved to M3's flagged highest-risk item after the Windows Terminal investigation concluded. Per the PRD's Open Questions, this story required a build-vs-license decision (in-house grammar vs. Talon interop) before implementation.

### Decision / Finding
Built in-house, deliberately narrow — not attempting a real grammar/intent parser. Key design choices, each a scope-narrowing decision made explicitly rather than discovered as a limitation later:
1. **Mode switching via a second hotkey (Ctrl+Alt+9), not a voice wake-phrase.** Detecting "command mode" spoken aloud reliably (vs. it appearing in normal dictated prose) is a real speech-understanding problem; a second hotkey sidesteps it entirely at the cost of one more keybinding to remember.
2. **Command matching is fixed substring checks** (`command_mode.rs::match_command`), same conservative philosophy as `cleanup.py`'s cue-phrase matching — longest/most-specific phrase checked first, no match found means no action (a wrong action is worse than no action).
3. **Reused the existing dictation pipeline wholesale** — same hotkey-toggle/audio-capture/ASR/overlay path as prose dictation, differing only in what happens to the final transcript (matched against commands and dispatched as key combos, instead of typed). This kept the implementation small: no new audio or ASR code, just a `Mode` enum threaded through.
4. **"New line" dispatches a real `VK_RETURN` keypress**, not the Unicode LF character prose-mode cleanup produces — a direct, deliberate application of the STORY-015 finding that terminals need real Enter keypresses, not literal LF characters, to submit/act.

Sentence-level select/delete is a same-line approximation (Home → Shift+End) since there's no OS-level "sentence" boundary concept to hook into; paragraph-level commands aren't implemented at all for the same reason plus added ambiguity about what a "paragraph" means in a plain text field.

### Rationale
Talon's grammar-based architecture is the more capable long-term answer for this category (flagged as such in the PRD from the start), but standing up an equivalent from scratch or integrating Talon itself is a materially larger effort than the rest of M3 combined. Shipping a narrow, honestly-scoped MVP now (and documenting exactly where it's thin) is more valuable than blocking Command Mode entirely on that larger decision.

### Implications
- `command_mode.rs`'s matcher is unit-tested (3 tests, pure logic, no OS dependency) but the dispatch side (real `SendInput` key combos) has not been tested live yet — unlike prose dictation, there's no live-voice validation of this feature so far.
- The fixed command list is a real ceiling: adding a new command means editing source code, not a config file. STORY-009's AC3 (user-extensible grammar) is explicitly not met and not attempted in this pass.
- If the fixed phrase set proves too limiting in practice (ambiguous overlaps with normal dictation content, users wanting custom commands), the honest next step is evaluating Talon interop for real, not extending the substring-matching approach indefinitely.

---

## [2026-07-04] — Wispr Flow Running Concurrently; Dashboard Feature Analysis
**Type:** Discovery
**Impact:** Low

### Context
User asked whether having Wispr Flow also installed and running was interfering with InkVoice testing. Checked directly rather than speculating.

### Decision / Finding
**Confirmed:** Wispr Flow was running as 11 separate processes (Electron multi-process architecture), including a dedicated `audio.mojom.AudioService` utility process — meaning it keeps a background microphone-capture service alive continuously, not just while actively dictating. This is a real, if likely modest, resource-contention factor. Quit at the user's request; InkVoice's own hotkey/mic pipeline is unaffected by this (registration and capture succeed independent of Wispr Flow's state), but two apps both negotiating the same Bluetooth mic in the background is a plausible amplifier of the kind of profile-switching race already fixed earlier this session.

**One hypothesis tested and disproven:** speculated Wispr Flow held the `Ctrl+Alt+Space` hotkey (which has shown "unavailable" in every launch log all session). Re-tested after fully quitting Wispr Flow — still unavailable. Something else on this machine holds it (not investigated further; our fallback to Ctrl+Alt+\` already works, so this isn't blocking anything).

User also shared a screenshot of Wispr Flow's "Insights" dashboard for future feature ideas (not to build now). Notable elements: usage analytics (words/minute with a percentile ranking, a "fixes made" transparency panel listing exact corrections applied, per-destination-app usage breakdown), a daily-usage streak calendar (gamification), and left-nav features beyond core dictation — Style (tone presets, maps to existing STORY-006), Transforms (likely LLM rewrite macros), Scratchpad (standalone capture area), Snippets (maps to existing STORY-011), and social sharing of stats.

### Rationale
Testing hygiene matters when debugging perceived performance/smoothness — a competing app's background services are a confound worth eliminating before drawing conclusions, even though our specific smoothness fix (the dual-model ASR architecture) was already validated independently via isolated latency measurements unaffected by this.

### Implications
- No code changes from this entry — process/environment hygiene note plus a future-feature backlog seed.
- The "fixes made" transparency panel idea is a strong fit for InkVoice's existing architecture (we already persist raw-vs-cleaned text per utterance in `debug_audio/`) and reinforces the privacy/trust positioning from the original competitive research, rather than competing with it — worth prioritizing over purely cosmetic ideas (streaks, sharing) if/when an Insights-style UI is built.
- No new stories created yet — revisit and formalize into STORIES.md when platform/UI milestones (M5+) are reached, per the user's "future phases" framing.

---

## [2026-07-04] — Prose Dictation Accidentally Triggered Ctrl+N (New Window) Mid-Typing
**Type:** Bug
**Impact:** Medium

### Context
User reported that after a normal Prose-mode dictation (Ctrl+Alt+\`, no Command Mode involved), new Notepad windows kept appearing mid-typing, each containing part of the dictated text — text that should have gone into one window ended up split across several.

### Decision / Finding
Diagnosed from the symptom shape rather than reproduced directly: this is the signature of an accidental **Ctrl+N ("new window")** firing partway through `type_text`'s character stream. Working theory: the physical Ctrl+Alt held to trigger the Ctrl+Alt+\` hotkey hadn't fully released by the time synthetic typing began, so when a character containing an "N" landed, Windows (or Notepad's own accelerator handling) combined it with the still-settling real Ctrl into a New Window command — splitting the rest of the typed text into the freshly-opened window.

**Fix:** rather than chase the exact timing race (hard to pin down precisely and inherently timing-dependent), `type_text` now explicitly sends `KEYUP` for every modifier key (Ctrl, Alt, Shift, both Win keys) before typing a single character. This removes the entire class of "hotkey's modifiers bled into the typed text" bug regardless of the precise underlying mechanism, rather than fixing one specific race condition.

### Rationale
A blanket "release everything first" is safer and more robust than a narrower fix tied to one hypothesized race, especially since the exact interaction between `KEYEVENTF_UNICODE` injection and physically-held modifier keys isn't something we can fully verify without deep Win32 internals access. Cheap to do (a handful of `KEYUP` events, no user-visible cost) and directly addresses the reported symptom.

### Implications
- Not yet re-verified live by the user — this is a plausible, low-risk fix for the reported symptom, not a confirmed root-cause fix (no direct reproduction was captured before applying it).
- `command_mode.rs`'s `dispatch()` was NOT given the same treatment in this pass (no report of the same symptom there yet) — if Command Mode shows similar accidental-shortcut behavior later, apply the same `release_all_modifiers` pattern there too (would need making the function `pub` and shared, currently private to `text_insert.rs`).

---

## [2026-07-04] — Overlay v2: Live Partial Text, Soft Anti-Aliased Waveform, Amplitude Smoothing
**Type:** Bug | Decision
**Impact:** Medium

### Context
User reported two issues after live-testing the overlay with real speech: (1) the live partial transcript was completely invisible — it only ever printed to a hidden dev console, never reached any user-facing surface; (2) the waveform looked noticeably less impressive live than in the reference video/screenshot the visual design was built from.

### Decision / Finding
**Root cause of (2), diagnosed rather than assumed:** the rendering pipeline had only been validated against a clean synthetic sine wave (`overlay_test.rs`'s test data). Real microphone RMS amplitude is noisy frame-to-frame; feeding that directly into the same renderer produces a visibly jittery, less elegant curve than the clean test suggested — the renderer wasn't broken, the input signal driving it in the synthetic test was unrepresentative of real audio.

**Three changes, each targeting a distinct part of the pipeline:**
1. **Temporal amplitude smoothing (EMA)** applied to incoming samples before they enter the history buffer — directly addresses the noisy-real-audio problem at the source, before it ever reaches rendering.
2. **Soft anti-aliased brush strokes replacing raw `Polyline`** — GDI's `Polyline` is hard-edged/aliased; waveform curves are now drawn with a custom soft circular-brush stamping routine operating directly on the pixel buffer (coverage-based alpha blending), plus the amplitude history is upsampled via linear interpolation before drawing so curve segments are short and read as smooth rather than a chain of sharp angles. This also required reworking how the pill's base fill and alpha channel are established — previously alpha was set via an "if GDI touched this pixel" heuristic (`RGB != 0`), which doesn't compose with partial-coverage AA blending; now alpha is set directly from a `point_in_pill` geometry test (standard rounded-rect containment: distance to the nearest clamped corner-circle-center), decoupled from whatever GDI/manual drawing happens to touch.
3. **Live partial text added inside the pill** via GDI `DrawTextW`, fed from a new `Sender<String>` channel threaded from `overlay::spawn()` through to the shell's reader thread (previously the reader thread only ever printed partials to console — this is the fix for complaint (1)). First attempt at text rendering used `CLEARTYPE_QUALITY` + normal font weight and came out too faint/thin to read against the cream background in a screenshot check; switched to `ANTIALIASED_QUALITY` + semibold (600) weight, confirmed legible via the same screenshot-testing method used for the earlier black-rectangle bug.

Pill dimensions increased (320×84 → 420×140) to fit a text line below the waveform without cramping either.

### Rationale
Screenshot-verify, don't assume — the same lesson from the original black-rectangle bug applied again here: the first text-rendering attempt "should have" worked by any reasonable reading of the GDI calls, but only checking the actual rendered pixels caught that it didn't read clearly. Both the smoothing and AA-stroke changes are also each independently testable/observable via the existing `overlay_test.rs` harness before ever touching the live app.

### Implications
- `overlay::spawn()`'s signature changed (now returns a 3-tuple: handle, amplitude sender, text sender) — any other caller besides `main.rs` and `overlay_test.rs` (there are none currently) would need updating too.
- The soft-brush AA drawing is O(strokes × segments × brush-area) per frame — cheap at this window size (420×140, 3 strokes, ~90×4 upsampled points) but worth remembering if the overlay ever grows significantly larger or the frame rate needs to increase.
- Not yet re-confirmed by the user against real live speech (verified via the synthetic-data test harness + screenshot only, same as the original build) — the real test is whether it looks as good with an actual voice driving it, given that gap is exactly what prompted this round of fixes.

---

## [2026-07-04] — Ctrl+Alt Hotkeys Reinterpreted as AltGr, Opening Notepad Settings
**Type:** Bug
**Impact:** High

### Context
User reported that pressing Ctrl+Alt+\` to stop dictation opened Notepad's Settings window instead of the final text landing — and Notepad never received the typed text at all.

### Decision / Finding
**Likely root cause: AltGr reinterpretation.** On many non-US keyboard layouts, the Ctrl+Alt chord is treated by Windows as the AltGr key, which the OS/app can reinterpret as producing an entirely different character or triggering a different shortcut than the literal key combo pressed. Modern Windows/WinUI apps (including the new Windows 11 Notepad) commonly bind Ctrl+, to Settings — a plausible AltGr-reinterpreted target for Ctrl+Alt+\`. This would mean **every Ctrl+Alt-based hotkey candidate tried so far** (Ctrl+Alt+Space, Ctrl+Alt+I, Ctrl+Alt+\`) has carried this same latent risk on this keyboard layout, not just this one instance.

**Fix:** reordered hotkey candidates to prefer **Ctrl+Shift** combos over Ctrl+Alt — Ctrl+Shift is never reinterpreted as AltGr on any keyboard layout. Dictation is now Ctrl+Shift+\`, Command Mode is Ctrl+Shift+9; Ctrl+Alt variants remain in the candidate list only as a last-resort fallback if Ctrl+Shift combos are ever unavailable.

### Rationale
Rather than patch around this one specific collision (e.g., picking yet another single Ctrl+Alt+key and hoping it doesn't also collide), the structural fix is avoiding the whole AltGr-ambiguous modifier chord — it protects against every future Ctrl+Alt candidate having the same latent risk, not just the one just discovered.

### Implications
- Positive side-finding: the user's report also confirmed the overlay v2 live-partial-text fix IS working ("some text coming up in the white graphics with waveform") — that part of the previous round's work is validated.
- Not yet re-verified live — this is a plausible, well-reasoned fix for the reported symptom, not a confirmed-via-reproduction root cause (same caveat pattern as the earlier Ctrl+N/modifier-release fix).
- If Ctrl+Shift hotkeys show similar unexpected behavior, the AltGr theory would need revisiting — Ctrl+Shift has no equivalent ambiguity on any known layout, so a recurrence there would point to a different cause entirely.

---

## [2026-07-06] — Long Dictations Lost Everything But the Last ~20s
**Type:** Bug | Decision
**Impact:** High

### Context
User reported 3 long dictations (confirmed via sidecar logs: 20.9s, 49.8s, 31.8s, 42.3s) each only captured the trailing portion of what they said. Root cause was a known, already-documented limitation: `finalize()` bounded the final Parakeet decode to only the trailing `MAX_WINDOW_S` (20s) of buffered audio, because decoding an entire long buffer in one shot had been found live to silently degrade to empty/garbled output. That mitigation worked for the degradation bug but as a side effect, silently dropped everything before the last 20s on any longer dictation — never surfaced to the user beyond a console log line (`finalize: session was Xs, only trailing 20s transcribed`).

### Decision / Finding
Replaced the single-trailing-window approach with **chunked decoding + stitching**: `UtteranceDecoder` now decodes audio in bounded chunks as it arrives (`_flush_chunk`), rather than one decode call over the whole/trailing buffer at the end. A chunk is flushed once it reaches `CHUNK_SOFT_TRIGGER_S` (15s) **and** a brief trailing silence is detected (RMS below `SILENCE_RMS_THRESHOLD` over the last `SILENCE_TAIL_S`) — preferring a natural pause boundary so words aren't split across two decode calls. If no pause is found, a hard ceiling (`CHUNK_HARD_MAX_S`, 19s) forces a flush anyway, keeping every single decode call safely clear of the ~20s zone where the model is known to degrade, regardless of utterance length or how continuously the user is speaking. `finalize()` now joins all chunks' raw text with the tail chunk, then runs the existing single cleanup pass (fillers/self-correction/punctuation) over the *complete* stitched text once — not per chunk, since those rules need the whole utterance in view.

### Rationale
The original 20s cap fixed the model-degradation bug but introduced a worse, silent one (data loss beyond 20s) — a classic "moved the bug, didn't fix it" situation. Chunking removes the length limit structurally rather than just raising the cap (which would only delay the same failure at a longer duration). Pause-preferred cutting is a pragmatic middle ground given effort constraints: true VAD-based segmentation would be more robust but a much bigger build; a simple RMS-based silence check on the chunk's own trailing audio is cheap, requires no new dependencies, and meaningfully reduces (though doesn't eliminate) mid-word cut risk. The hard ceiling exists so a user who never pauses (continuous run-on speech) still can't reintroduce the original degradation bug.

### Implications
- `MAX_WINDOW_S` is gone, replaced by `CHUNK_SOFT_TRIGGER_S`/`CHUNK_HARD_MAX_S` — any other code referencing the old constant would break (checked: nothing else did).
- Known remaining seam risk: if a chunk boundary lands mid-word despite the pause preference (e.g., continuous speech past the hard ceiling), that word may be misrecognized at the boundary rather than dropped — an accuracy risk, not a data-loss risk, and a reasonable trade given the alternative was losing entire sentences.
- Self-correction/cue-phrase detection (STORY-005) could theoretically miss a correction phrase that straddles a chunk boundary since chunks are decoded independently before stitching — an existing edge case, not newly introduced, and cleanup still runs on the fully stitched text so this only matters if the cue phrase itself is split mid-word by the boundary (same low-probability case as the general seam risk above).
- Not yet live-tested against a real long dictation (existing `test_cleanup.py`/`test_personal_dictionary.py` suites pass unchanged, 20+5 tests — those don't exercise this new chunking path directly since it lives in `asr_server.py`, which has no dedicated test file). Real validation is a live 30-60s+ dictation.
- Found and cleaned up in passing: two stray/orphaned `asr_server.py` processes were both bound (attempting to bind) to the sidecar's port from earlier session restarts — killed both and relaunched one clean instance.

---

## [2026-07-06] — Push-to-Talk Added Alongside Toggle (Not Replacing It)
**Type:** Decision
**Impact:** Medium

### Context
STORY-001 AC1 called for both push-to-talk (hold) and toggle (tap-to-start/tap-to-stop) modes, but only toggle had been built. User explicitly confirmed both should coexist rather than one replacing the other, and that the existing Ctrl+Shift+\` toggle binding should stay as-is.

### Decision / Finding
Added push-to-talk as **Ctrl+Shift+Space**, a separate chord from the toggle hotkeys, always available alongside them. Mechanism had to differ from the toggle hotkeys' `RegisterHotKey`/`WM_HOTKEY`: that API only ever fires once on press, with no matching "key released" message — there's no way to detect hold-and-release with it. Push-to-talk instead uses a `WH_KEYBOARD_LL` low-level keyboard hook, which sees real key-down/key-up pairs for any key system-wide, installed on the main thread (it requires a message loop pumping on the installing thread to receive callbacks, and the existing `GetMessageW` loop already provides that — no extra thread needed).

The hook watches for Space while Ctrl+Shift are held (checked via `GetAsyncKeyState`, not hook-tracked modifier state, since that reflects true physical key state regardless of message-queue timing): on the first key-down edge (guarding against repeat-key auto-repeat firing many key-down events while held) it posts a custom thread message (`WM_APP_PTT_DOWN`) into the same message queue the main loop already pumps; on key-up it posts `WM_APP_PTT_UP`. The hook swallows the Space keystroke in both directions when Ctrl+Shift are held (returns non-zero instead of calling `CallNextHookEx`) — otherwise holding the chord would leak a literal space character into the focused app, and some apps bind Ctrl+Shift+Space to their own shortcut (e.g. paste-without-formatting).

`Ctrl+Shift+Space` was removed from the toggle hotkey's fallback candidate list (it was there as a secondary candidate) to avoid the two mechanisms ever contending for the same physical chord.

Refactored the previously-inline start/stop recording logic (duplicated between the toggle branch's start and stop arms) into two shared functions, `start_recording`/`stop_recording`, called from both the toggle and push-to-talk code paths — avoids drift between the two if either changes later. A `ptt_session: bool` flag distinguishes "this recording was started by push-to-talk" from "started by the toggle hotkey," so a stray release doesn't stop a toggle-started session and vice versa (attempting to toggle-stop a push-to-talk session prints a message pointing at the correct release action instead).

### Rationale
A low-level keyboard hook is more invasive than `RegisterHotKey` (it sees every keystroke system-wide, and Windows has strict timing requirements — a slow hook callback risks being silently unhooked or causing input lag) — chosen because it's the only Win32 mechanism that exposes real key-up events for an arbitrary combo; there is no less invasive API for this specific need.

### Implications
- Built and compiles clean; hook installs successfully on relaunch (confirmed via startup log, process stays responsive afterward). Not yet live-tested with an actual held keypress — the real test is whether holding Ctrl+Shift+Space starts recording promptly and releasing it stops promptly, and whether a plain unmodified Space keypress elsewhere is unaffected (should be, since the hook only intercepts Space when Ctrl+Shift are also down).
- The hook callback must stay fast (no blocking work) per Windows' LL-hook timing contract — current implementation only does an atomic swap and a `PostThreadMessageW` call, both fast; the actual start/stop work happens later in the main loop when the posted message is processed, not inside the hook itself.

---

## [2026-07-06] — "period" as Content Corrupted by Verbal-Command Matching; Duplicate Sidecar Processes Found
**Type:** Bug | Discovery
**Impact:** Medium

### Context
User reported a long dictation "failed... after initial 20s." Investigation via `debug_audio/*.txt` (the raw+cleaned dump every finalized utterance produces) showed the chunked-decode fix from the prior session is actually working correctly — all 3 recent dictations, including a ~30s one, captured their full content start to finish, no truncation. The reported "failure" was something else: the ordinary noun phrase "over a **period** of time" came out as "over a**.** of time" in the cleaned output.

### Decision / Finding
**Root cause:** `apply_verbal_commands` in `cleanup.py` rewrote every standalone occurrence of "period" to a literal "." via a context-blind regex (`\bperiod\b`) — correct when "period" is used as a punctuation command, wrong when used as an ordinary noun ("a period of time", "grace period", "trial period"). Same risk existed for comma/colon/semicolon (e.g. "colon cancer", "comma-separated").

**Fix:** added a determiner guard — a punctuation command is essentially never preceded by an article/determiner ("a period", "the comma"); nobody dictates "a period" meaning "insert a full stop." Added negative lookbehind assertions for a/an/the/this/that/one/some/every/each immediately before each of these single-word command patterns. 3 new regression tests added (23 total in `test_cleanup.py`, all passing) covering: the exact live-found case, a parallel comma case, and confirming the command still fires correctly when NOT preceded by a determiner.

**Separate discovery while investigating:** two `asr_server.py` processes were found racing for the sidecar's port — one from the project's dedicated venv (started by me), one launched via an entirely different, system-wide `uv`-managed Python interpreter, with no scheduled task, startup entry, or `.vscode/tasks.json` found to explain what launched the second one. The `uv`-interpreter one won the port race and was the one actually serving the shell. Likely explains earlier stray-process sightings this session too. Killed both, confirmed the port was free, relaunched exactly one instance from the project's venv.

### Rationale
Same category of fix as the earlier "p.m." sentence-boundary bug (STORY-003) and the documented STORY-004 AC3 limitation ("legitimate use of a filler/command word as content is not mangled — not tested against this specific case") — this is exactly that limitation, now found live and fixed for this specific instance. The determiner guard is a cheap surface-level heuristic, not real semantic disambiguation (matching this module's stated design: rule-based, deliberately conservative, not a full NLP understanding pass), but it directly closes the reproduced failure with no new false-negative risk found in testing.

### Implications
- Other verbal-command words could still misfire in less common phrasings the determiner guard doesn't cover (e.g. "grace period" — no determiner immediately before "period" there, so it would still be corrupted: "grace." — not yet found live, but a known-plausible gap in this heuristic). Revisit if it surfaces.
- Not yet re-verified live with a fresh "period as content" dictation — verified via the unit test suite and the historical debug-audio evidence only.
- **CORRECTION (same day):** the "two racing sidecar processes" framing above was a misdiagnosis. `spikes/m0_asr/venv/pyvenv.cfg` shows this venv was created with `uv`'s managed Python 3.11.15 as its base/`home` interpreter. On this setup, launching `venv\Scripts\python.exe` produces a parent process that re-execs into that base interpreter as a child — confirmed via `Get-CimInstance Win32_Process`'s `ParentProcessId`: the "uv-path" process's parent was literally the "venv-path" process from the same launch. There was only ever one logical sidecar instance; what looked like two competing processes fighting for the port was just its normal parent/child pair. No actual duplicate-process bug exists here — retracting the "cause still unexplained" note above.

---

## [2026-07-06] — Long-Dictation Text Arrived Incomplete in Notepad (Typing Layer, Not ASR)
**Type:** Bug
**Impact:** High

### Context
Immediately after the chunked-decode and "period" fixes above, user tested another long dictation (~550 characters, into Notepad) and again saw only a fragment land — this time a mid-sentence cutoff ("...ultimate objective th"). Investigation via the sidecar log showed the ASR/cleanup pipeline produced the fully correct, complete final text this time (confirmed byte-for-byte against what the user quoted as missing) — so unlike the two previous incidents this session, this failure was NOT in transcription. It was in `type_text`, the step that actually inserts the finalized text into the focused window.

### Decision / Finding
`type_text`'s own `SendInput` return-value check reported full success for every group — no dropped-event warning logged — meaning the drop happened *after* the OS accepted the synthetic input, in Notepad's own processing of a long sustained keystroke burst. This is the same race already documented in `type_text`'s docstring ("seen once out of several runs... not a deterministic failure"), but the 6-chars/4ms pacing had only ever been validated against short test utterances, not several hundred characters of continuous typing — a much longer burst gives that race far more chances to actually manifest.

Two changes:
1. **Widened the pacing safety margin** in `text_insert.rs`: `GROUP_SIZE` 6→4, `GROUP_DELAY` 4ms→8ms — roughly triples total typing time for a given utterance, giving the receiving app's message pump more breathing room per burst.
2. **Made the push-to-talk keyboard hook ignore its own injected traffic.** While investigating, noticed a real design gap unrelated to (but compounding) the above: `keyboard_hook_proc` (added earlier this session, see the push-to-talk entry) receives a synchronous callback for *every* keyboard event system-wide, including events `type_text` itself injects via `SendInput` — roughly 1 in 6 characters of ordinary English is a space, so a long dictation means hundreds of extra synchronous hook hops added to the OS input pipeline for the app's own typing traffic, layered on top of whatever Notepad's own message pump was already straining to keep up with. Real physical key holds carry no `LLKHF_INJECTED`/`LLKHF_LOWER_IL_INJECTED` flag; synthetic ones do. The hook now checks these flags and bails immediately (`CallNextHookEx`) for any injected event — conceptually correct regardless (push-to-talk should only ever react to a real physical hold), and removes a plausible contributing factor to the typing-drop race becoming more likely.

### Rationale
Couldn't definitively prove which of the two changes (or both together) caused the observed drop — `SendInput` reporting success gives no visibility into what Notepad's own message loop actually did with each event. Both changes are independently well-reasoned (one hardens the known documented race for longer bursts, the other closes a real correctness gap in the hook's design) and neither has a plausible downside, so applying both together rather than trying to isolate which one "was the real fix" via more live trial-and-error.

### Implications
- This is now the third distinct bug found across three consecutive long-dictation attempts this session (20s truncation → "period" corruption → typing dropout) — each was a different layer of the pipeline (ASR windowing, cleanup rules, text insertion), not the same bug reappearing. Long dictations specifically have been the most effective way to surface issues this session; worth continuing to test with genuinely long, natural speech rather than short phrases.
- Not yet re-verified live — the real test is another ~500+ character dictation into Notepad landing completely.

---

## [2026-07-06] — Live transcript moved from overlay pill into the document itself
**Type:** Pivot
**Impact:** High

### Context
STORY-002 AC2 was implemented as live partial text rendered inside the recording overlay pill. Live user feedback: the pill text is not useful ("better to remove that text from there") — committed text only ever appeared in the document after stopping, which made dictation feel non-realtime despite sub-ms partial latency.

### Decision / Finding
Partials are now live-typed directly into the focused window as they arrive (Prose mode only), via a per-utterance prefix-diff (streaming Zipformer partials are append-only in practice, so this is nearly always "type the new suffix"). On final, the raw live text is erased with backspaces and replaced wholesale by the cleaned Parakeet transcript — the user chose this over keeping the raw text (which would lose punctuation/casing/cleanup) or a bigger overlay. Raw partials are lowercased with first-letter capitalization before typing, since the streaming model emits caseless text. The overlay is waveform-only again (height 140→96).

### Rationale
- Wholesale erase-and-replace over minimal diffing at finalization: casing/punctuation differences mean the diff would nearly always start at character 0 anyway; simpler and equally disruptive.
- `send_backspaces` force-releases modifiers first: during push-to-talk the user physically holds Ctrl+Shift, and Ctrl+Backspace is delete-word in most edit controls — each 1-char retraction would have eaten a word.
- No live typing in Command mode — the utterance is an instruction, not content.

### Implications
- Known accepted limitation: if the user moves the caret/focus mid-dictation, backspace retraction lands at the wrong spot and corrupts text. Fixing this properly needs UI Automation/TSF-based insertion (tracked implicitly under STORY-015's compatibility umbrella).
- The final replace is a burst of backspaces + retype; for very long dictations this is visually loud and slow (~8ms per 4 chars each way). If it feels bad live, consider anchoring on the word-level common prefix (case-insensitive) to shrink the rewrite.
- Not yet live-tested (TS-002-04..08 Ready).

---

## [2026-07-06] — ROLLBACK: live document typing triggered target-app shortcuts
**Type:** Bug | Pivot
**Impact:** High

### Context
The same-day pivot to live-typing partials into the focused document (previous entry) failed its first live test: with toggle mode active (Ctrl+Shift+`), one word appeared and then Notepad commands/shortcuts fired repeatedly instead of text. User asked for a rollback to final-only typing. Rolled back within the hour; the waveform-only overlay half of the change was kept.

### Decision / Finding
Live typing is out until the injection mechanism is safe. What we know:
- Failure mode: injected keystrokes interpreted as accelerator shortcuts by the target app, repeatedly — not a one-off race.
- It happened even in TOGGLE mode, where the user releases Ctrl+Shift+` before speaking. So it is NOT purely "user physically holds PTT modifiers during typing".
- Leading hypothesis: the per-partial cadence is the differentiator vs. final-only typing (which has worked for weeks with the same `type_text`). Each partial burst called `release_all_modifiers()` (5 synthetic modifier keyups) followed by Unicode char events, many times per second, interleaved with real user key activity and with the just-released hotkey chord. Candidate mechanisms: (a) hotkey modifiers still physically settling when the first partial bursts arrive; (b) synthetic modifier keyups colliding with the async key state mid-stream so some chars land while the app still sees Ctrl down; (c) `send_backspaces`' VK_BACK with stale modifier state becoming Ctrl+Backspace or similar.

### Rationale
Roll back rather than debug forward: the user was actively using dictation and the feature made the tool destructive (fired editor commands). Final-only typing is known-good.

### Implications
Constraints for any retry of live typing:
- Gate every injection burst on `GetAsyncKeyState` showing Ctrl/Alt/Win physically UP; if held, buffer the delta and flush when released. Don't rely on synthetic modifier keyups.
- Wait for the hotkey chord to be fully released before the first live burst.
- Consider abandoning keystroke injection for live text entirely — UI Automation TextPattern / TSF insertion has no accelerator ambiguity at all.
- A test harness that replays a real partial cadence into a focused app (like `sendinput_test` but streaming) would let this be validated without live dictation.

---

## [2026-07-06] — Rollback retest "failed" because the shell was launched from a sandboxed session
**Type:** Discovery
**Impact:** Medium

### Context
After the live-typing rollback, the retest showed NO text arriving in Notepad at all — worse than before the whole change. But the sidecar debug dumps showed the utterance captured and finalized perfectly ("Testing, testing, testing…" at 17:28:36), and a probe confirmed the shell held the Ctrl+Shift+` hotkey. Everything worked except the final SendInput.

### Decision / Finding
The failing instance had been relaunched by Claude from its sandboxed tool session. The sandbox's job object strips UI-injection rights from child processes: hotkey registration, audio capture, and TCP to the sidecar all work normally, but `SendInput` is silently discarded (reports success, nothing arrives). Integrity level looks normal (Medium), so nothing about the process hints at the restriction. Relaunched unsandboxed; the rollback code itself was never actually broken.

### Implications
- Any "transcribes fine but types nothing" symptom: first check how the shell process was launched, before touching pipeline code.
- The reported "Notepad settings opened on Ctrl+Shift+` at stop" remains unexplained-but-suspicious: while the shell holds that hotkey, the keypress cannot reach Notepad. Most likely a nearby combo was pressed (e.g. Ctrl+Shift+' — apostrophe, not backtick — on this layout); watch whether it recurs now that typing works.

---

## [2026-07-06] — Root cause found: typed text becomes accelerator chords while stop-hotkey modifiers are still physically held
**Type:** Bug
**Impact:** High

### Context
User screenshot showed Notepad Settings opening "on stopping the dictation". Notepad Settings is Ctrl+, — and dictated text contains commas. The final transcript is typed within milliseconds of the Ctrl+Shift+` stop press, while the user still physically holds Ctrl.

### Decision / Finding
`release_all_modifiers`' synthetic keyups are insufficient against physically held keys: the held key's auto-repeat re-asserts key-down ~30ms later, so injected characters intermittently land with Ctrl active and become accelerator chords (comma → Ctrl+, → Settings). This is also the retroactive root cause of the rolled-back live-typing failure (continuous injection while the hotkey chord was held → continuous shortcut firing). Fix: `wait_for_physical_modifier_release()` in text_insert.rs — poll GetAsyncKeyState for Ctrl/Alt/Shift/Win physically up (3s deadline as hang-guard) before any injection; applied to both `type_text` and Command Mode's `send_combo` (where a held Shift would have turned Ctrl+Z into Ctrl+Shift+Z).

### Implications
- The 2026-07-06 rollback entry's "retry constraints" hypothesis (a) is confirmed as the mechanism. A future live-typing retry becomes viable again IF every injection burst is gated the same way (buffer partials while modifiers are held, flush on release).
- Why it rarely bit before: whether the final decode finishes before the user releases the chord is a race; short utterances finalize fast enough to lose it.

---

## [2026-07-07] — Dashboard shipped as a sidecar-served localhost page, not the planned Tauri shell
**Type:** Decision
**Impact:** High

### Context
User asked for a Wispr-Flow-style dashboard (not just the recording pill), plus the personal-dictionary capture flow and latency-log surfacing. The original EXECUTION_PLAN called for a Tauri shell for tray UI/settings/history.

### Decision / Finding
Built it as a single-page dashboard served by the Python sidecar itself (stdlib http.server, 127.0.0.1:43918, daemon thread): dictionary CRUD API writing personal_dictionary.json (picked up next-utterance since the decoder reloads per utterance), history.jsonl store appended at finalize, latency stats from the existing latency_log.jsonl. No new dependencies, no new process.

### Rationale
- Zero new runtime and fully cross-platform — significant given the user's stated intent to run on a Mac at the office: the sidecar+dashboard will run there as-is, while any Tauri/Win32 shell work would not.
- The JSON API is shell-agnostic: a future Tauri (or native) shell can embed or replace the page and keep the API.
- Tauri remains the right call for tray/settings/native-feel later; this decision defers, not rejects, it.

### Implications
- Dashboard lives and dies with the sidecar process; if the sidecar is down the page is unreachable (acceptable — no sidecar means no dictation to inspect either).
- history.jsonl is unbounded (text-only lines, tiny); revisit rotation only if it ever matters.
- STORY-014 "revert" is clipboard-copy of the raw transcript — InkVoice cannot rewrite text already typed into another app.

---

## [2026-07-07] — Portable package ships models by download, not by bundling
**Type:** Decision
**Impact:** Medium

### Context
User wanted the portable zip much smaller for moving to a Mac/office laptop and sharing with colleagues. The 629 MB was almost entirely two int8-quantized ONNX model files (622 MB + 179 MB) — quantized weights are near-uniform-entropy bytes, so max zip compression only recovered ~2%.

### Decision / Finding
Stopped bundling models in the zip. `fetch_models.py` downloads both from their public k2-fsa/sherpa-onnx GitHub release (verified reachable, same models already validated during M0/M2) on first launch, extracts, and keeps only the int8 files (discarding the fp32 siblings in the release archive) into `models/parakeet` and `models/zipformer`. Both launchers call it automatically when `models/` is absent. Zip dropped to 0.2 MB.

### Rationale
- No practical alternative shrinks quantized weights further without changing the models themselves (a real accuracy tradeoff, not attempted here).
- Downloading from the upstream release rather than re-hosting keeps this dependency-free and avoids the maintenance burden of hosting large binaries anywhere.

### Implications
- First run now requires internet access (one-time, ~800MB); every run after that is fully offline, same as before.
- If k2-fsa ever removes/renames these specific release assets, `fetch_models.py`'s URLs would need updating — low risk (versioned, stable release names) but worth knowing if a colleague reports a 404 on first run.

---

## [2026-07-08] — Writing modes + live transcript in the pill; GDI alpha-hole discovery
**Type:** Decision | Bug
**Impact:** High

### Context
User asked to (1) bring back live text as-you-speak, acceptable in the pill this time, (2) add selectable writing modes (dictation/email/message) with mode-appropriate output formatting, citing FluidVoice as inspiration.

### Decision / Finding
- Live transcript is DISPLAY-ONLY in the pill (attempt 3). Attempt 2's failure class (injected keystrokes becoming app shortcuts) is structurally impossible when nothing is injected until final. Pill grew to 380×150: ribbons / two-line tail-trimmed transcript / clickable mode chip.
- Modes are a manual, per-recording selection on the pill — the MVP slice of STORY-006's app-aware tone adaptation. Formatting is rule-based in the sidecar (mode_format.py), applied after cleanup+dictionary, mode carried in the "end" message. Command-mode recordings force "dictation" so formatting never corrupts command matching. Multi-word greeting names are bounded using the ASR's own capitalization ("hello Dr Smith the results…" — the lowercase word ends the name), which works because Parakeet emits real casing.
- FluidVoice review: concepts adopted (live-preview overlay, per-context formatting), no code — their enhancement runtime is closed-source and the app is GPLv3 (adopting code would relicense InkVoice).
- Clickability: WS_EX_TRANSPARENT removed (per-pixel alpha hit-testing routes clicks on opaque pill pixels to us), WS_EX_NOACTIVATE retained — chip clicks never move focus from the dictation target.

### Rationale
Rule-based before LLM, consistent with the M2 cleanup decision: modes needed a transport + UI + formatting seam regardless of engine; the LLM can replace mode_format.py's internals later without touching the seam.

### Implications
- FOUND LIVE: GDI text on a per-pixel-alpha layered window ZEROES the alpha of touched pixels — glyphs become transparent holes and "text color" is whatever is behind the window. Every prior pill-text rendering only looked right by background luck. Fix: GdiFlush + restore alpha=255 across the pill interior after GDI drawing. Any future GDI drawing on the overlay must repeat this.
- The pill now occludes clicks over its area while recording (no longer click-through) — accepted; it only exists during recording.
- history.jsonl/debug dumps store the MODE-FORMATTED text as "cleaned" — the dashboard diff view therefore also shows mode formatting as AI edits, which is accurate and reinforces transparency.

---

## [2026-07-08] — Long finals now insert via clipboard paste, not simulated typing
**Type:** Bug | Decision
**Impact:** High

### Context
Third live incident of long-dictation corruption at the typing layer (M1: dropped chars; later: truncation; now: a ~75-char stretch of a clean 470-char final replaced by a run of repeated "y"s). Each time the sidecar dump proved the transcript was correct before injection, and each time SendInput reported success — the failure is in target apps' handling of sustained synthetic keystroke bursts, and pacing tweaks only shrink the window.

### Decision / Finding
Stop simulating typing for long text. `insert_text` now routes finals ≥100 chars through clipboard paste (save existing clipboard text → set ours → single Ctrl+V after physical-modifier release → restore previous text after 300ms). Short finals keep per-character typing: it works in more contexts (no paste-shortcut assumption) and never disturbs the clipboard for the common quick-phrase case. This mirrors what commercial dictation tools do.

### Implications
- Non-text clipboard content (images, file lists) is NOT preserved across a paste-insert — only CF_UNICODETEXT is saved/restored. Documented; full multi-format preservation is real extra surface if it ever matters.
- Apps that don't bind Ctrl+V to paste (rare; some terminals) get the paste shortcut anyway for long text — Windows Terminal is already a documented full block (STORY-015), and classic consoles do support Ctrl+V on Win10+.
- If a target app reads the clipboard lazily (>300ms after the keystroke), the restore could race the paste — not observed; revisit the delay before assuming.

---

## [2026-07-09] — Dictation hotkey moved off backtick entirely: OEM punctuation keys are unsafe for RegisterHotKey
**Type:** Bug
**Impact:** High

### Context
User reported hitting Notepad's menu/settings immediately on pressing the stop hotkey (Ctrl+Shift+`) — the same AltGr-adjacent symptom from an earlier finding, but this time on the "AltGr-safe" Ctrl+Shift combo that was supposed to have fixed it. A screenshot showed Notepad's Alt-key KeyTip badges (F/E/V over File/Edit/View) — proof a raw Alt keystroke reached Notepad even though the registered hotkey has no Alt component at all.

### Decision / Finding
Root cause: the machine has two keyboard layouts installed (en-AU, en-GB) with Windows' default "different input method per app window" active. `RegisterHotKey`'s matching for OEM punctuation virtual-key codes (VK_OEM_3 = backtick/grave) is resolved against the ACTIVE LAYOUT OF THE FOREGROUND THREAD at the moment of the keypress, not the layout active at registration time — a long-documented Windows quirk. US and UK layouts diverge enough on that key that producing it under Notepad's active layout apparently synthesizes an extra Alt component, which no longer matches the registered Ctrl+Shift+VK_OEM_3 combo — so Windows never intercepts it, and the raw keystroke (with its stray Alt) falls straight through to Notepad, immediately, before any app code runs.

Fixed by moving the dictation toggle's primary candidate to **Ctrl+Shift+F9**. Virtual-key codes for function keys are identical across every keyboard layout — there's no character being produced, so this whole bug class is structurally impossible on an F-key. Backtick combos remain as last-resort fallbacks only. Command Mode's toggle (Ctrl+Shift+9, a digit — already comparatively safe) got a same-class Ctrl+Shift+F8 fallback for free, deliberately avoiding F10 since plain F10 is itself Windows' "activate menu bar" key.

### Rationale
Two prior fixes on this exact hotkey (moving off a letter key, then off Ctrl+Alt) treated symptoms of the same underlying issue: any OEM punctuation key is layout-fragile. Function keys close the whole class rather than the next specific instance of it.

### Implications
- Packaged docs (dist/InkVoice/README.md) updated to the new hotkey. The dist package's bundled `.exe` is a stale build from before this fix (and before several other 2026-07-08/09 changes) — needs a rebuild+recopy before it's handed to anyone.
- If a future hotkey candidate is ever needed, prefer function keys or digits; avoid comma/period/backtick/bracket-class VK_OEM_* codes.

---

## [2026-07-09] -- AI Rewrite shipped as an opt-in mode, not an always-on Auto-Edit pass
**Type:** Decision
**Impact:** High

### Context
User asked to build out STORY-004's local-LLM grammar/fluency correction, explicitly as a 4th writing mode alongside Dictation/Email/Message rather than an always-on pass. This resolves EXECUTION_PLAN M2's long-standing "local LLM Auto-Edit" line item, which had been rule-based-only until now.

### Decision / Finding
Ollama was already installed and running on the dev machine with several models pulled (qwen3:8b/30b, qwen2.5:14b-instruct, qwen3-coder:30b, gpt-oss:20b) -- no new install or download needed. Chose qwen3:8b over the larger pulled models purely for latency: this is a proofreading task, not a reasoning task, and warm-model latency measured 120-600ms for sentence-length input, vastly outperforming any quality gain a bigger model would offer here. "think": false disables Qwen3s chain-of-thought, which would otherwise burn time on hidden reasoning tokens the user is just waiting on.

Implemented as , called from s finalize() only when mode == "rewrite", after the existing rule-based cleanup+dictionary+mode_format passes have already run. Same resilience contract as the rest of the sidecar (save_debug_utterance, log_latency): any failure (Ollama down, timeout, malformed response, empty output) falls back to the already-cleaned text unchanged -- Rewrite mode silently degrades to Dictation mode rather than blocking or crashing.

### Rationale
PRD/EXECUTION_PLAN originally framed this as an always-on Auto-Edit pass. Opt-in was chosen instead because an LLM call is neither instant (even warm, real network+inference latency) nor risk-free (it can reword things no rule-based pass would) -- making it an explicit mode keeps the default experience fast and predictable, and confines "the AI changed my wording" risk to only the mode a user deliberately selected. This also reuses the exact mode-chip mechanism already built for STORY-006, rather than a new toggle/setting.

### Implications
- Cold-start cost: the very first Rewrite-mode utterance after Ollama has been idle pays a one-time model-load cost (~18s measured) before the fast path kicks in. Not yet mitigated (e.g. a keep-alive ping) -- worth revisiting if it surfaces as a real first-use complaint.
- Ollama being a genuinely external dependency (not bundled, not guaranteed present on a colleague's machine) needs to be called out in the portable-package docs before this ships anywhere beyond the dev machine.
- Tests deliberately don't assert on real model output (non-deterministic); the resilience/fallback contract is what's covered. Real-quality verification is the end-to-end manual check logged in CHANGELOG.

---

## [2026-07-09] — AI Rewrite shipped as an opt-in mode, not an always-on Auto-Edit pass
**Type:** Decision
**Impact:** High

### Context
User asked to build out STORY-004's local-LLM grammar/fluency correction, explicitly as a 4th writing mode alongside Dictation/Email/Message rather than an always-on pass. This resolves EXECUTION_PLAN M2's long-standing "local LLM Auto-Edit" line item, which had been rule-based-only until now.

### Decision / Finding
Ollama was already installed and running on the dev machine with several models pulled (qwen3:8b/30b, qwen2.5:14b-instruct, qwen3-coder:30b, gpt-oss:20b) — no new install or download needed. Chose qwen3:8b over the larger pulled models purely for latency: this is a proofreading task, not a reasoning task, and warm-model latency measured 120-600ms for sentence-length input, vastly outperforming any quality gain a bigger model would offer here. `"think": false` disables Qwen3's chain-of-thought, which would otherwise burn time on hidden reasoning tokens the user is just waiting on.

Implemented as `src/sidecar/ai_rewrite.py`, called from `asr_server.py`'s finalize() only when mode == "rewrite", after the existing rule-based cleanup+dictionary+mode_format passes have already run. Same resilience contract as the rest of the sidecar (save_debug_utterance, log_latency): any failure (Ollama down, timeout, malformed response, empty output) falls back to the already-cleaned text unchanged — Rewrite mode silently degrades to Dictation mode rather than blocking or crashing.

### Rationale
PRD/EXECUTION_PLAN originally framed this as an always-on Auto-Edit pass. Opt-in was chosen instead because an LLM call is neither instant (even warm, real network+inference latency) nor risk-free (it can reword things no rule-based pass would) — making it an explicit mode keeps the default experience fast and predictable, and confines "the AI changed my wording" risk to only the mode a user deliberately selected. This also reuses the exact mode-chip mechanism already built for STORY-006, rather than a new toggle/setting.

### Implications
- Cold-start cost: the very first Rewrite-mode utterance after Ollama has been idle pays a one-time model-load cost (~18s measured) before the fast path kicks in. Not yet mitigated (e.g. a keep-alive ping) — worth revisiting if it surfaces as a real first-use complaint.
- Ollama being a genuinely external dependency (not bundled, not guaranteed present on a colleague's machine) needs to be called out in the portable-package docs before this ships anywhere beyond the dev machine.
- Tests deliberately don't assert on real model output (non-deterministic); the resilience/fallback contract is what's covered. Real-quality verification is the end-to-end manual check logged in CHANGELOG.

---
