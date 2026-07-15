# Architecture: InkVoice

> Living document. Update when components, data models, or integrations change materially.
> Current status: **pre-implementation** — this reflects the target architecture agreed alongside the PRD, not built code. Open questions marked ⚠ must be resolved before implementation starts (see @docs/PRD.md Open Questions).

## System Overview

```
                        ┌─────────────────────────────┐
                        │        Desktop Client        │
                        │  (macOS / Windows / Linux)    │
                        │                               │
   Mic ──▶ Audio Capture│──▶ VAD ──▶ ASR Engine ─┐       │
                        │            (local or    │      │
                        │             streaming)  ▼      │
                        │                    Partial/Final│
                        │                    Transcript   │
                        │                         │       │
                        │                         ▼       │
                        │              LLM Cleanup Layer  │
                        │        (Auto-Edit, tone, dict.) │
                        │                         │       │
                        │                         ▼       │
                        │            OS Text Insertion     │
                        │        (Accessibility/UIA/AT-SPI)│
                        └─────────────┬───────────────────┘
                                      │ (settings sync, cloud-mode inference,
                                      │  org policy, telemetry — opt-in only)
                                      ▼
                        ┌─────────────────────────────┐
                        │         Backend Services      │
                        │  Auth · Sync · Billing ·       │
                        │  Admin/SSO/SCIM · Cloud ASR/   │
                        │  LLM inference (opt-in) ·       │
                        │  Zero-retention proxy mode      │
                        └─────────────────────────────┘
```

## Core Components

### 1. Audio Capture & VAD
Captures microphone input, runs voice-activity detection to segment speech, and streams audio frames to the ASR engine. **Constraint (M0 finding):** on Windows, capture must go through WASAPI directly (e.g., cpal/windows-rs from the Rust core) — PortAudio-based capture delivers silence from Bluetooth LE Audio microphones (verified against a Bose QC Ultra; see CONTEXT.md entry 4). LE Audio headsets are a mandatory device class in the capture test matrix. Runs identically whether the downstream ASR is local or cloud — the client decides *where* to send frames based on the user's selected mode (STORY-012).

### 2. ASR Engine (dual-mode)
- **Model: Parakeet-TDT-1.1B** (NVIDIA, open-weight transducer architecture) is the primary ASR engine for both on-device and cloud modes. Decided over Canary-Qwen-2.5B: Canary-Qwen's FastConformer+LLM-decoder ("SALM") design is built for offline/batch accuracy, not incremental streaming, and its lower RTFx (LLM decoding overhead) doesn't meet the <300ms latency bar regardless of available GPU headroom. Parakeet-TDT's transducer architecture emits tokens incrementally, which is what streaming partial results actually require.
- **On-device mode:** Parakeet-TDT-1.1B, GPU-accelerated via CUDA/TensorRT on NVIDIA hardware (reference dev target: RTX 5070 Ti, 16GB) for full local processing with zero network round-trip. **Fallback path:** CPU inference via ONNX export (sherpa-onnx runtime) for non-NVIDIA hardware (Apple Silicon, AMD/Intel GPUs, CPU-only machines) — needs a latency/accuracy spike to confirm it holds the target on CPU (see Open Questions).
- **Cloud mode:** same Parakeet-TDT-1.1B model served from backend inference (opt-in), used when a user selects cloud mode, or automatically on hardware that can't meet the latency bar locally.
- Both modes emit the same partial/final transcript event stream to the client so downstream components are mode-agnostic.

### 3. LLM Cleanup Layer
Consumes raw ASR transcript + lightweight context (destination app identity, user's Personal Dictionary, selected tone preset) and produces cleaned text: filler removal, self-correction resolution, punctuation/formatting, tone adaptation. Runs as a smaller on-device model in on-device mode, or a cloud model in cloud mode — never triggers screen/window content capture to do its job (STORY-013/NREQ-009 hard constraint).

### 4. Personal Dictionary & Snippet Store
Local-first data store (synced to backend for cross-device access, encrypted at rest) holding learned vocabulary, corrections history, and snippet macros. Import/export as portable JSON (STORY-020).

### 5. OS Text Insertion Layer
Platform-specific adapters that insert finalized text into the focused control:
- macOS: Accessibility API
- Windows: UI Automation
- Linux: AT-SPI (X11) / portal-based input (Wayland) ⚠ — Wayland sandboxing may constrain global hotkey and insertion approaches; needs a spike before STORY-017 is scheduled.

### 6. Command Mode Grammar Engine
Separate mode from prose dictation (STORY-009). Parses recognized speech against a grammar of navigation/editing commands and dispatches OS-level actions (cursor movement, selection, delete, undo). Extensible via user-defined command grammars. Build-vs-license decision (e.g., Talon-style interop) is an open question — see PRD.

### 7. Backend Services
- **Auth & Sync:** account management, cross-device settings/dictionary sync.
- **Billing:** Free/Pro/Lifetime/Team/Enterprise tier enforcement (STORY-021).
- **Admin/SSO/SCIM:** enterprise identity integration and org policy enforcement (STORY-022, STORY-023).
- **Cloud Inference:** hosts cloud-mode ASR/LLM for users/orgs that opt in; supports a **zero-retention proxy mode** where audio/transcript are processed in-memory only and never persisted (STORY-024).
- **Telemetry:** opt-in only; explicitly excludes audio/transcript content by default (NREQ-005).

### 8. Browser Extension & Mobile Clients
Thin clients reusing the same ASR/LLM cleanup backend contracts as desktop, adapted to platform constraints (web page text-field injection; iOS keyboard extension / Android IME sandboxing limits).

## Data Model (high level)

- **User** — account, tier, org membership, selected processing mode (on-device/cloud/auto)
- **Organization** — Team/Enterprise entity, seats, SSO config, policy settings (force-on-device, zero-retention, shared dictionary)
- **DictionaryEntry** — term, pronunciation hints, correction source, scope (personal/org-shared)
- **Snippet** — trigger phrase, template body, variable placeholders, scope
- **DictationSession** — timestamped raw transcript, AI-edited transcript, destination app, processing mode used, latency metrics (for internal benchmarking against NREQ-001/002)
- **CommandGrammar** — user-defined command phrase → action mapping

## Key Non-Functional Constraints Driving Architecture
- Latency budget (NREQ-001) forces the on-device path to exist as a true first-class path, not a stripped-down fallback — this is why ASR/LLM cleanup are architected as dual-mode from the start rather than cloud-first-then-optimize.
- Resource footprint (NREQ-003) rules out a heavy Electron-only shell as the sole desktop implementation; native or lightweight-runtime shells (e.g., Tauri, or platform-native UI with a shared core engine) should be evaluated ⚠.
- Zero-retention/compliance requirements (NREQ-005, STORY-024) mean the backend must support a distinct inference code path with no persistence layer touched at all, auditable independently of the standard (retained) path.

## Open Architecture Questions (blocking implementation)
See @docs/PRD.md Open Questions for the authoritative list. Summary:
1. Command Mode grammar engine: build vs. license/interop.
2. Desktop shell framework choice given the <150MB idle memory target.
3. Wayland text-insertion approach for Linux.
4. Spike: confirm Parakeet-TDT-1.1B CPU/ONNX (sherpa-onnx) fallback path holds acceptable latency/accuracy on non-NVIDIA hardware before committing it as the universal on-device fallback.
