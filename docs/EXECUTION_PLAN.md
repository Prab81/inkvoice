# Execution Plan: InkVoice

**Last Updated:** 2026-07-04
**Status:** Approved plan, pre-M0
**Companion docs:** @docs/PRD.md (what/why), @docs/ARCHITECTURE.md (how), @docs/BUILD_LOG.md (running state), @docs/STORIES.md (story tracking)

---

## 1. Plan Review — Corrections to the Original Roadmap

The PRD as written is a funded-team roadmap. For execution here, three corrections:

1. **Windows-first, single-platform MVP.** Dev/reference hardware is a Windows 11 machine with an RTX 5070 Ti. macOS/Linux/mobile/browser (STORY-016 partial, 017, 018, 019) move to later milestones. Linux remains a strategic v1.x wedge but is not in the build critical path.
2. **Defer monetization and enterprise entirely** (Epics F, G — STORY-021..024). Nothing in Free/Pro/SSO/admin-console matters until core dictation is proven. These stories stay Backlog, untouched.
3. **Sequence by risk, not by epic.** The single existential risk is the latency/accuracy of local streaming Parakeet on real hardware. That gets validated first (M0 spike) before any app code is written.

Two remaining open questions are now resolved with pragmatic defaults (logged in CONTEXT.md):
- **Desktop shell:** Tauri (Rust + system webview) for tray UI/settings/history, with a **Python sidecar process** for GPU inference (NeMo/onnxruntime + CUDA). Rust core owns hotkey capture and text insertion (SendInput/UI Automation). Rationale: Python is unavoidable for the ML runtime; keeping it in a sidecar isolates its memory from the shell and keeps the UI layer under the footprint budget. Electron rejected per NREQ-003.
- **LLM cleanup layer (MVP):** rule-based pass in M1 (verbal commands, basic filler stripping); local small instruct model (e.g., Qwen3-4B-class via llama.cpp, sharing the 5070 Ti) added in M2 for Auto-Edit/self-correction/tone. Cloud LLM cleanup deferred until there's a backend at all.
- **Command Mode engine:** build a minimal in-house grammar (M3), evaluate Talon interop only if in-house proves insufficient. Licensing talks are not a solo-build activity.

## 2. Milestones

| # | Milestone | Stories | Exit criteria (checkpoint gate) |
|---|-----------|---------|--------------------------------|
| **M0** | **ASR feasibility spike** | (pre-story) | Parakeet-TDT-1.1B streaming locally on the 5070 Ti; measured partial-result latency < 300ms P50; WER sanity-checked on a small test set. **Go/no-go gate** — if it fails, model choice reopens before anything else is built. |
| **M1** | **Core dictation MVP (Windows)** | STORY-001, 002, 003 (partial: model punctuation + verbal commands) | Hotkey push-to-talk → streaming partials → finalized text inserted into Notepad, browser, VS Code, Windows Terminal. Tray app runs. Latency instrumented per session. |
| **M2** | **Cleanup + personalization** | STORY-003 (full), 004, 005 (basic), 007, 014 | Local LLM Auto-Edit (fillers, self-corrections), personal dictionary with correction learning, history view with raw-vs-edited diff and revert. |
| **M3** | **Power user layer** | STORY-009 (minimal grammar), 010, 015 | Command Mode core verbs (select/delete/navigate/undo), case-style dictation for identifiers, terminal-compatibility regression suite green (WSL, Windows Terminal, Cursor). |
| **M4** | **Hardening + on-device guarantee** | STORY-012, 013, 006 (full), 008 | Fully offline operation verified; local/cloud indicator (trivially "local" for now); data-flow log; multi-language pass; footprint audit vs. NREQ-003. |
| **M5+** | **Platform expansion & product** | STORY-016 (macOS), 017, 011, 020, then Epics F/G | Scoped when M4 ships. Not planned in detail now — planning it now would be speculation. |

Each milestone ends with a **checkpoint** (see §4).

## 3. Model Routing Strategy

**Orchestrator: Sonnet (main loop — this session).** Sonnet does all planning, architecture, integration, review of subagent output, and doc upkeep. Opus is not the orchestrator: orchestration is mostly routing and judgment over moderate context, which Sonnet handles at ~1/5 the cost; Opus's edge is depth on a single hard problem, so it's used as a targeted escalation, not a resident.

Note: "Hermes" refers to the locally installed **Nous Research Hermes Agent** (`V:\Hermes\hermes-agent`), invocable headlessly via `hermes -z "<prompt>"`. It shares the low-complexity/token-heavy band with Haiku: use Hermes when the task benefits from running fully outside this session's context/token budget (bulk generation, long log analysis); use Haiku subagents when the task needs this session's file state or harness integration. All delegated output is reviewed by the orchestrator before merge.

| Complexity band | Work type | Model | Why |
|---|---|---|---|
| **Critical/hard** | Latency-critical audio pipeline, streaming ASR integration, concurrency bugs, Command Mode grammar design, thorny native-API debugging (UIA/SendInput edge cases) | **Sonnet inline; escalate to Opus** only after a failed Sonnet attempt or when a design decision is expensive to reverse | Opus is the escalation valve, not the default — pay for depth only where wrong answers are costly |
| **Standard** | Tray UI, settings screens, history view, dictionary store, sidecar IPC plumbing, ordinary feature code + its tests | **Sonnet** (inline or subagent) | Core competency zone; subagent when the task is self-contained and >~30k tokens of context would otherwise pile into the main loop |
| **Low-complexity, token-heavy** | Boilerplate/scaffolding, repetitive test cases from a written spec, fixtures/mock data, bulk mechanical refactors, CHANGELOG/BUILD_LOG drafting, log-file analysis, doc formatting passes | **Haiku subagents** (the "Hermes" role) | These burn tokens on volume, not reasoning — cheapest tier, spec written by orchestrator, output reviewed by orchestrator |
| **Search/recon** | "Where is X", API-surface exploration, dependency docs lookup | **Explore agent / Haiku** | Read-heavy, zero design judgment needed |

**Routing rules of thumb:**
- The orchestrator writes the spec; a cheaper model never invents requirements.
- Haiku output that touches the latency-critical path gets a Sonnet review before merge; pure boilerplate doesn't.
- One escalation to Opus requires a one-line justification in BUILD_LOG.md (keeps cost drift visible).
- Prefer inline work over subagents when context is already loaded — spawning re-derives context and usually costs more than it saves.

## 4. Checkpoint & Compaction Protocol

Context compaction is managed by the harness (cannot be manually triggered at an exact token count), so the protocol makes compaction **lossless** rather than trying to control its timing:

1. **BUILD_LOG.md is the durable state.** Its `## Current State` section is updated at every checkpoint and whenever direction changes — it must always be sufficient for a fresh session to resume with zero conversation memory.
2. **Checkpoint = end of each milestone, each completed story, or any session pause.** At a checkpoint, in order:
   - BUILD_LOG.md: dated entry (what was built, what's verified, what's next, open problems)
   - STORIES.md: status + AC checkboxes updated
   - CHANGELOG.md: one line per shipped change
   - CONTEXT.md: entry only if an architectural/design decision was made
   - TEST_SCENARIOS.md: scenarios added/updated for the story per the Task Execution Protocol
3. **Long-conversation hygiene:** when a session runs long (approaching the point where summarization kicks in, ~the 250k region), proactively write a checkpoint even mid-story, so the summary never carries load-bearing state that isn't also on disk.
4. **Doc updates are Haiku-band work** (drafted cheap, reviewed by orchestrator) except CONTEXT.md decision entries, which the orchestrator writes directly.

## 5. Immediate Next Actions
1. M0 spike: environment setup (CUDA/PyTorch/NeMo or onnxruntime-gpu), pull Parakeet-TDT-1.1B, build a minimal streaming loop against the mic, measure latency. → Go/no-go.
2. On go: scaffold repo (`src/` layout for Tauri shell + Python sidecar), init git, first BUILD_LOG entry.
3. Begin STORY-001/002 (they land together — hotkey capture and streaming pipeline are one vertical slice).
