# CLAUDE.md — Project Intelligence File

> Loaded by Claude Code at the start of every session. These instructions are MANDATORY
> and apply to every Claude model (Sonnet, Opus, or newer). Follow them exactly.
> Detailed specs live in `docs/`. Keep this file under 300 lines.

---

## 📁 Project Structure

```
project-root/
├── CLAUDE.md                  # ← You are here. Session bootstrap.
├── docs/
│   ├── SESSION_STATE.md       # LIVE resume point — read FIRST, update via /checkpoint
│   ├── PRD.md                 # Product requirements (source of truth for scope)
│   ├── STORIES.md             # All user stories — the live project dashboard
│   ├── EXECUTION_PLAN.md      # Phased build order across stories
│   ├── CONTEXT.md             # Append-only log of decisions & discoveries (the WHY)
│   ├── ARCHITECTURE.md        # System design, data models, component map (the WHAT)
│   ├── TEST_SCENARIOS.md      # Scenario table driving /verify-story
│   ├── BUILD_LOG.md           # Per-session journal of what was built
│   └── CHANGELOG.md           # What shipped and why
├── src/                       # Application source
├── tests/                     # Tests mirroring src/ structure
├── spikes/                    # Throwaway experiments (never imported by src/)
└── .claude/skills/            # Project slash commands (see Skills section)
```

---

## 🚀 Session Start (MANDATORY — do this before anything else)

1. Read `docs/SESSION_STATE.md` — the exact resume point from the last session.
2. Read `docs/STORIES.md` — find the story marked `In Progress`.
3. Skim the latest entries of `docs/CONTEXT.md` — recent decisions.
4. State to the user in 2–3 sentences: where the project stands and what you will do next.
5. If SESSION_STATE.md conflicts with what you find in the code, TRUST THE CODE, say so,
   and fix SESSION_STATE.md.

Never assume prior knowledge. Never answer status questions from memory — read the files.
The `/resume` skill automates this ritual.

## 💾 Context Survival Rules (the most important section)

Your context window WILL be compacted or the session WILL end — plan for it constantly:

- **Checkpoint often.** Run the `/checkpoint` discipline (update `docs/SESSION_STATE.md`)
  after completing any story step, every ~30 minutes of work, and ALWAYS before: large
  refactors, long-running commands, risky operations, or when the conversation feels long.
- **The checkpoint test:** a brand-new session must be able to read SESSION_STATE.md and
  continue within two minutes. Include: current story + step, exact next action, open
  problems, working commands, and gotchas discovered this session.
- **Files are memory; context is scratch.** Any discovery, decision, or plan that lives
  only in conversation is already lost. Write it down when it happens, not "later".
- **Delegate exploration.** For codebase searches or surveys, use a subagent (Explore /
  general-purpose Task) and keep only the conclusion. Reserve your own context for the
  current story's files.
- **Prefer fresh sessions.** If heavily compacted, checkpoint, tell the user a fresh
  session will reason better, and make the handoff seamless via SESSION_STATE.md.

## 🚦 Task Execution Protocol

For ANY implementation request:

1. **Identify the story** — find the STORY-ID in `docs/STORIES.md` or create one first.
   No requirement exists without a story. Ask "which story is this?" if unclear.
2. **Check scope** — confirm it's in `docs/PRD.md`. If not → run the change protocol below.
3. **Plan in writing** — for anything non-trivial, write the plan into the story's
   Technical Notes (files to touch, ordered steps, risks) and add scenario rows to
   `docs/TEST_SCENARIOS.md` BEFORE coding. Get user confirmation for large plans.
4. **Execute in small verified loops** — implement ONE plan step → run the relevant
   test/command → record the result → next step. NEVER stack two unverified steps.
   Deviating from the plan? Amend the plan in writing first (`[AMENDED] why`).
5. **Verify with evidence** — run `/verify-story STORY-ID`: execute every scenario in
   TEST_SCENARIOS.md for the story. An AC may only be ticked with named evidence
   (test output, command result, observed behavior). "Should work" = NOT Done.
6. **Update docs** — story status + ACs, CONTEXT.md entry if a decision was made,
   one line in CHANGELOG.md, BUILD_LOG.md session entry.
7. **Checkpoint** — refresh SESSION_STATE.md.

**One story In Progress at a time.** Notice an unrelated bug or idea mid-story? File it as
a Backlog story (30 seconds) and return to the current story. Do not detour.

## 🔄 Requirement Change Protocol

Never silently absorb a change. When requirements shift:

1. Acknowledge the change explicitly.
2. List the impacted stories by ID.
3. Update `docs/PRD.md`.
4. Revise impacted stories — mark superseded ACs `[REVISED]`, never delete.
5. Log what/why/impact in `docs/CONTEXT.md`.
6. Ask: "Should any In Progress work be paused or reworked?"

The `/new-requirement` skill runs this protocol.

## ⚙️ Development Standards

### Code
- Tests alongside every feature — untested code is unfinished code.
- Explicit over clever. Functions do one thing; files group one concern.
- No `TODO` comments in committed code — convert to stories.
- Match the existing style of the file you're editing.
- Experiments go in `spikes/`, never imported by `src/`.

### Git
- Commit messages: `[STORY-ID] What and why`.
- One logical change per commit. Commit at every green verification point —
  small commits are checkpoints too.
- Never commit secrets. Never force-push without explicit user instruction.

### Testing & Evidence
- Run tests before marking any story Done.
- Every AC edge case has a TEST_SCENARIOS.md row with Status and Evidence columns.
- If a test fails, report the actual output — never paper over it.

## 📖 Story Format (docs/STORIES.md)

```markdown
### STORY-[ID]: [Short Title]
**Status:** Backlog | In Progress | Done | Blocked | Revised
**Priority:** P0 | P1 | P2   **PRD Ref:** REQ-[ID]   **Last Updated:** YYYY-MM-DD

**As a** [user], **I want** [capability], **so that** [benefit].

#### Acceptance Criteria
- [ ] AC1: [Specific, testable condition]
- [ ] AC2: Edge case or error state handled

#### Technical Notes / Plan
[Written plan from /plan-story: files, ordered steps, risks]

#### Change History
- [YYYY-MM-DD] Created from REQ-001
```

Rules: revise, never delete (`[REVISED]`). Update status the moment work starts/stops.
Blocked stories get a `Blocker:` line with owner.

## 🗂️ Context Log (docs/CONTEXT.md)

Append-only. Log: architectural decisions (with rejected alternatives), dependency
choices, non-obvious bug root causes, integration quirks, scope changes, performance and
security findings. Do NOT log: obvious implementation details, dead-end debugging steps.
Entry format: dated header, Type, Impact, Context, Decision, Rationale, Implications.

## 🧰 Skills (run as slash commands)

| Command | Purpose |
|---------|---------|
| `/kickoff <idea>` | New project/feature: PRD → stories → execution plan → scenarios |
| `/resume` | Session-start ritual: load state, announce position, continue work |
| `/plan-story <ID>` | Write the implementation plan for one story before coding |
| `/execute <ID>` | Disciplined implement loop for the planned story |
| `/checkpoint` | Update SESSION_STATE.md NOW (cheap — run liberally) |
| `/verify-story <ID>` | Run all scenarios with evidence; tick ACs; mark Done |
| `/new-requirement` | Requirement change protocol |
| `/handoff` | End-of-session gate: all docs current, clean stopping point |

## 🚫 Hard Rules

- **Never guess at requirements** — ask before building.
- **Never silently change scope** — surface, log, confirm.
- **Never claim Done without evidence** — run the scenario, paste the result.
- **Never leave SESSION_STATE.md or STORIES.md stale** — they ARE the project.
- **Never let context be the only copy of anything.**
- **Never re-litigate logged decisions** — reference CONTEXT.md; reopen only explicitly.
- **Keep this file under 300 lines** — detail lives in docs/.

---

*Bootstrapped from the Project Execution Blueprint. Philosophy: blueprint/PLAYBOOK.md.*
