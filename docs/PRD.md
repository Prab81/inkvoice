# PRD: InkVoice — AI Voice Dictation for Every App
**Version:** 1.0
**Last Updated:** 2026-07-04
**Status:** Draft

## Problem Statement
Typing is the bottleneck between thought and text. Voice dictation has existed for decades (Dragon, OS-level dictation) but never crossed into daily habitual use because of three unsolved problems: **accuracy** on real speech (filler words, self-correction, jargon), **latency** that breaks the illusion of "thinking out loud," and **friction** of switching apps/formats. Wispr Flow proved there's a market willing to pay $12-20/mo to solve this, but our research (@docs/CONTEXT.md, 2026-07-04 entry) surfaced three unresolved gaps in the incumbent:

1. **Trust gap** — Flow's cloud-only architecture led to an undisclosed screenshot-capture incident; Trustpilot rating (2.7/5) is far below App Store rating (4.8/5), meaning experienced/burned users trust it less than new users.
2. **Performance gap** — Marketing claims <700ms latency; real-world reports are 1-2s. Resource footprint is heavy (~800MB RAM idle, Electron-based).
3. **Power-user / platform gap** — No Linux support, no real "command mode" for hands-free computer control (only basic text transforms), weak handling of code/technical jargon without manual dictionary work, no offline capability at all.

**Who has this problem:** knowledge workers with RSI/wrist strain, fast thinkers who type slower than they talk, non-native English speakers who write better than they type, developers who want voice-driven coding, and privacy-sensitive professionals (legal, healthcare, finance) who cannot send raw audio/screen content to a third-party cloud by default.

## What "Good" Looks Like (Category Bar)
Derived from competitive research across Wispr Flow, Superwhisper, Aqua Voice, Talon, Dragon, and current ASR benchmarks:

| Dimension | Bar for "good" | Bar for "great" (our target) |
|---|---|---|
| Word error rate (clean English) | <10% | <6% (match Parakeet-TDT/Canary-Qwen class models) |
| Perceived latency (speech end → text appears) | <1s | <300ms (streaming partial results) |
| Privacy | Cloud processing disclosed | On-device mode available; zero screenshot/context capture without explicit opt-in and visible indicator |
| Personalization | Manual dictionary | Auto-learns vocabulary from usage + one-shot correction propagation |
| Platform coverage | macOS + Windows | macOS, Windows, **Linux**, iOS, Android, browser extension |
| Power-user control | Text formatting transforms | Full grammar-based Command Mode (Talon-class) for hands-free navigation/editing, plus voice-coding primitives |
| Resource footprint | <500MB idle | <150MB idle, native (not Electron) where feasible |
| Enterprise readiness | SOC 2, SSO | SOC 2 Type II, ISO 27001, HIPAA, SSO/SCIM, admin console, configurable zero-retention mode |
| Pricing | Subscription only | Subscription + one-time/lifetime tier (addresses documented price complaint) |

## Goals
- [ ] G1: Ship a system-wide dictation product (macOS + Windows first) that a user can dictate into any text field, with auto-formatting and punctuation, in private beta within 1 quarter.
- [ ] G2: Beat Wispr Flow's real-world latency (target <300ms perceived) and WER (<6%) on benchmark and internal jargon-heavy test sets.
- [ ] G3: Offer a genuine privacy-first mode — on-device ASR + on-device LLM cleanup — as a first-class, not degraded, experience.
- [ ] G4: Build a Command Mode power users (especially developers) prefer over Talon/Wispr's transforms for hands-free editing and voice-driven code dictation.
- [ ] G5: Reach parity+ on platform coverage including Linux, which every major competitor excludes.
- [ ] G6: Establish enterprise trust (SOC 2 Type II, transparent data handling, admin controls) from day one of the paid enterprise tier, not retrofitted after an incident.

## Non-Goals
- Not building a general-purpose voice assistant (no "Hey InkVoice, what's the weather" — this is a dictation/editing tool, not Siri/Alexa).
- Not building meeting transcription/summarization as a primary use case (Otter.ai's territory) — may be a later add-on, not v1.
- Not building custom hardware (wearables/BCI) — software only.
- Not supporting VMs/remote desktop environments in v1 (matches current category limitation; revisit post-launch).
- Not attempting real-time translation dictation in v1 (dictate in language A, output language B) — single-language-at-a-time dictation only.

## User Personas
- **Dana, Developer (power user):** Writes code and Slack messages all day, has mild RSI, wants to dictate comments, commit messages, and prose into her IDE/terminal without breaking her keybindings. Values a real Command Mode and correct handling of technical jargon/camelCase/identifiers.
- **Miguel, Sales/CS Professional:** Writes 100+ emails and CRM notes a day. Wants fast, accurate, tone-adapted dictation (formal for client emails, casual for Slack) with zero setup burden.
- **Priya, Non-native English Writer:** Speaks better English than she types, uses dictation to draft docs and reports faster and with more natural phrasing, needs strong accent robustness and self-correction handling.
- **Alex, Privacy-Sensitive Professional (legal/healthcare/finance):** Cannot use tools that send raw audio or screen content to third-party clouds without explicit, auditable consent. Needs an on-device mode and/or a zero-retention enterprise agreement to adopt at all.
- **Jordan, IT/Security Admin (enterprise buyer):** Evaluates the tool for company-wide rollout. Needs SSO/SCIM, admin console, usage visibility, and compliance certifications before approving purchase.

## Requirements

### Functional
- REQ-001: System-wide dictation activated by a configurable hotkey (push-to-talk and toggle modes) that inserts text into the currently focused field in any application.
- REQ-002: Real-time streaming transcription with partial results displayed as the user speaks (not just after they stop).
- REQ-003: Automatic punctuation, capitalization, and paragraph/list formatting inferred from speech patterns (pauses, verbal cues like "new line", "period").
- REQ-004: AI Auto-Edit pass that removes filler words ("um", "uh"), false starts, and self-corrections ("meet at 2... actually make it 3" → "meet at 3") before finalizing text.
- REQ-005: App-context-aware tone adaptation (e.g., formal register for email clients, casual for chat apps) based on the destination application, with a visible per-app override.
- REQ-006: Personal Dictionary that auto-learns names, jargon, and acronyms from user corrections and explicit additions, applied globally across all apps and languages.
- REQ-007: Support for 50+ languages with automatic language detection and the ability to switch language mid-session.
- REQ-008: Command Mode — a grammar-based hands-free mode for navigation and text editing (e.g., "select last sentence", "delete that", "go to end of paragraph") distinct from prose dictation mode.
- REQ-009: Voice-coding primitives for developers: dictate identifiers in camelCase/snake_case/PascalCase on command, dictate symbols/punctuation by name, and a "code comment mode" preset.
- REQ-010: On-device processing mode (ASR + LLM cleanup) selectable per-user, with a persistent visible indicator of whether audio is being processed locally or in the cloud.
- REQ-011: No screen/window content capture of any kind without explicit, per-app, revocable opt-in — and a persistent on-screen indicator whenever such capture is active.
- REQ-012: Dictation history log (searchable, time-grouped) with the ability to view a diff between raw transcript and AI-edited output, and one-click revert to raw transcript.
- REQ-013: Snippet/macro library — voice-triggered insertion of frequently used templates (signatures, boilerplate responses).
- REQ-014: Cross-platform apps for macOS, Windows, Linux (X11 + Wayland), iOS, and Android, plus a browser extension for Chromium/Firefox-based text fields.
- REQ-015: Admin console for Team/Enterprise tiers: seat management, usage analytics, org-wide dictionary sharing, policy enforcement (e.g., force on-device mode, disable cloud fallback).
- REQ-016: SSO (SAML) and SCIM provisioning for Enterprise tier.
- REQ-017: Configurable data retention policy per organization, including a zero-retention mode where no audio or transcript is stored server-side post-processing.
- REQ-018: Free tier with a generous but capped word allowance; Pro subscription tier with unlimited words; one-time/lifetime license tier as an alternative to subscription.
- REQ-019: Real-time correction: user can speak a correction immediately after a misrecognition ("no I meant...") and have it retroactively applied without manual text selection.
- REQ-020: Import/export of personal dictionary and snippets for backup and team-sharing.

### Non-Functional
- NREQ-001: **Latency** — P50 perceived latency (speech end to text rendered) under 300ms; P99 under 600ms, measured end-to-end including network for cloud mode.
- NREQ-002: **Accuracy** — Word error rate under 6% on a standard clean-English benchmark (e.g., LibriSpeech test-clean equivalent) and under 12% on an internal jargon/accented-speech test set, re-measured every release.
- NREQ-003: **Resource footprint** — Idle memory usage under 150MB on desktop clients; CPU usage under 2% idle.
- NREQ-004: **Reliability** — Dictation session crash rate under 0.1%; local draft buffering so no in-progress dictation is lost on app crash or network drop.
- NREQ-005: **Privacy & Security** — SOC 2 Type II and ISO 27001 certification before general enterprise availability; HIPAA-eligible configuration for healthcare customers; no telemetry containing audio or transcript content without explicit opt-in; all cloud audio in transit and at rest encrypted.
- NREQ-006: **Accessibility** — Full compatibility with screen readers and OS accessibility APIs; the app itself must not silently alter a user's existing OS accessibility settings (a documented Wispr Flow complaint).
- NREQ-007: **Offline capability** — On-device mode must function with zero network connectivity for core dictation (streaming ASR + local formatting), with graceful degradation messaging if a cloud-only feature (e.g., a large cleanup model) is unavailable offline.
- NREQ-008: **Compatibility** — Must not break or conflict with terminal emulators (including WSL, Cursor's integrated terminal, Windows Terminal keybindings) — a specific, named failure mode of the incumbent.
- NREQ-009: **Transparency** — Any data capture beyond raw microphone audio (e.g., screen content for context) must be off by default, clearly disclosed in-product before first use, and independently auditable via an in-app data-flow log.

## Open Questions
- [ ] Do we build Command Mode's grammar engine in-house or license/interop with Talon's open grammar approach? (owner: eng lead)
- [ ] Pricing exact numbers for lifetime tier and enterprise floor? (owner: product/business, due: before pricing page ships)
- [ ] Which compliance certification to pursue first — SOC 2 Type II vs. HIPAA — given target enterprise segment? (owner: product, due: before enterprise GA)

## Decisions Made
- [2026-07-04] Product will be positioned explicitly against Wispr Flow's three documented weaknesses (privacy incident, latency-vs-marketing gap, platform/power-user gaps) rather than as a generic feature clone. Rationale: differentiation on trust and performance is more durable than feature parity alone, per competitive research in @docs/CONTEXT.md.
- [2026-07-04] On-device processing will be a first-class mode, not a fallback. Rationale: privacy is the single sharpest documented complaint against the market leader; Apple's on-device SpeechAnalyzer benchmarks show on-device is now performance-competitive, removing the historical excuse for cloud-only design.
- [2026-07-04] Linux support included in v1 platform scope. Rationale: every major competitor (Flow, Superwhisper, MacWhisper, Willow) excludes Linux; it is an open, low-competition wedge into the developer segment we're already targeting via Command Mode.
- [2026-07-04] ASR model selected: **Parakeet-TDT-1.1B**, GPU-accelerated (CUDA/TensorRT) as the primary engine for both on-device and cloud modes, with a CPU/ONNX (sherpa-onnx) fallback path for non-NVIDIA hardware. Rationale: Parakeet's transducer architecture streams tokens incrementally, meeting the <300ms latency bar (NREQ-001); Canary-Qwen was rejected despite a slightly better WER because its LLM-decoder architecture is built for offline/batch accuracy, not streaming, and GPU headroom (target dev hardware: RTX 5070 Ti, 16GB) doesn't resolve that architectural mismatch. The 1.1B variant (rather than 0.6B) was chosen because target GPU headroom allows it, closing most of the accuracy gap to Canary-Qwen while preserving streaming behavior.
- [2026-07-04] Desktop shell: Tauri (Rust + system webview) with a Python GPU-inference sidecar; Electron rejected on footprint (NREQ-003). Execution scope corrected to a Windows-first milestone ladder (M0 ASR spike → M4 hardening) with platform expansion and Epics F/G deferred to M5+. Full plan and model-routing strategy in @docs/EXECUTION_PLAN.md; rationale in @docs/CONTEXT.md.

## Future Phase Ideas (not yet scoped into stories)
> Candidates surfaced from competitive observation, not yet committed — formalize into STORIES.md when a relevant platform/UI milestone (M5+) is reached.
- **Insights/analytics dashboard** (from Wispr Flow's Insights screen): words-per-minute with a percentile ranking, per-destination-app usage breakdown, a "fixes made" transparency panel showing exact corrections applied. The transparency panel is a strong fit — InkVoice already persists raw-vs-cleaned text per utterance (`debug_audio/`) and it reinforces the privacy/trust positioning rather than competing with it. Prioritize this over purely cosmetic ideas below.
- **Daily-usage streak calendar** (gamification, GitHub-heatmap style) — cheap once there's a UI and a local usage log; lower priority than the transparency panel.
- **Scratchpad**: a standalone text-capture area independent of any target app, for dictating notes with nowhere specific to send them yet.
- **Social sharing of usage stats** — viral growth mechanic; if built, must be opt-in only per our privacy stance, not on-by-default.
- (Style/Transforms/Snippets equivalents already tracked as STORY-006/likely-future/STORY-011 respectively — not new ideas, just confirmed as validated-by-competitor-existence.)
