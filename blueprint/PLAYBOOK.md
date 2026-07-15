# PLAYBOOK — Getting Frontier-Class Execution From Any Model

> This is the philosophy behind the blueprint. The CLAUDE.md template enforces the rules;
> this file explains WHY they exist, so future sessions (and future humans) keep them honest.
>
> Origin: distilled from working sessions with Claude Fable 5. The goal is that a Sonnet or
> Opus session following this playbook is indistinguishable in output quality from those
> sessions — because the quality came from the process, not just the model.

---

## The Core Insight

A model's raw intelligence sets the ceiling on any *single step*. But project outcomes are
determined by the *accumulation* of steps: whether progress persists, whether errors get
caught, whether scope stays controlled, whether session #14 knows what session #3 decided.

Strong models compensate for missing structure by holding more in their heads.
The blueprint removes the need to compensate: **externalize the head.**

Ten principles follow. Each maps to a concrete mechanism in the blueprint.

---

## 1. Files Are Memory. Context Is Scratch.

Anything that lives only in the context window is already lost — compaction, session end,
or a crash will take it. The context window is a workbench, not a filing cabinet.

**Mechanism:** `docs/SESSION_STATE.md` is the living brain-dump: current story, exact next
step, open problems, recent discoveries, commands that work. `/checkpoint` refreshes it.
The test of a good checkpoint: *could a brand-new session with zero context read
SESSION_STATE.md and continue within two minutes?*

## 2. Plan in Writing, Then Execute the Writing

Weaker planning shows up as mid-task drift: the model starts coding, discovers a snag,
improvises, and ends somewhere nobody chose. A written plan converts improvisation into a
visible decision: either follow the plan or *explicitly amend it in writing*.

**Mechanism:** `/plan-story` produces a plan inside the story entry (files to touch, order
of operations, test scenarios, risks) BEFORE any code. `/execute` treats the plan as a
checklist. Deviations get one line in the plan: `[AMENDED] <what changed and why>`.

## 3. Small Verified Loops Beat Big Confident Leaps

The failure mode of every model — including frontier ones — is building 500 lines on top of
an unverified assumption. The cheaper the loop (change → run → observe), the earlier the
assumption breaks, the less gets rebuilt.

**Mechanism:** `/execute` mandates the loop: implement ONE plan step → run the relevant
test/command → record the result → next step. Never two unverified steps in a row.

## 4. Evidence, Not Confidence

Models overclaim completion. "This should work now" is not a status; it is a hypothesis.
The antidote is mechanical: a claim of Done must be accompanied by pasted evidence —
test output, a command result, an observed behavior.

**Mechanism:** `/verify-story` refuses to tick an acceptance criterion without naming the
scenario that was run and what was observed. TEST_SCENARIOS.md rows have an Evidence
column. If it wasn't run, it isn't Done — it's In Progress.

## 5. One Story In Flight

Parallel half-done work is where context dies. Every open thread multiplies what the next
session must reconstruct.

**Mechanism:** STORIES.md allows exactly one story `In Progress` per work stream. Found a
bug while working? If it's not blocking the current story, it becomes a Backlog story
(30 seconds to file) — not a detour.

## 6. Keep the Main Context Lean — Delegate Exploration

Reading 40 files to find something burns the context that execution needs. Exploration is
high-volume, low-retention work: perfect for a subagent whose context is disposable.

**Mechanism:** For any search/survey ("where is X handled?", "how do other modules do Y?"),
spawn an Explore/general-purpose agent and keep only its conclusion. The main session's
context is reserved for the current story's files and the plan.

## 7. Fresh Sessions Beat Stretched Sessions

After heavy compaction, a session runs on summaries of summaries — reasoning quality
degrades invisibly. A fresh session reading well-maintained docs reasons on primary truth.

**Mechanism:** When a session gets long, `/checkpoint` + `/handoff`, then start fresh with
`/resume`. This costs two minutes and buys back full reasoning quality. Never fear ending
a session: if the docs are current, nothing is lost — that's the whole design.

## 8. Scope Changes Are Logged Transactions

Silent scope drift is unrecoverable: months later nobody knows what was agreed. Every
requirement change is acknowledged, impact-assessed, written into PRD + stories, and logged.

**Mechanism:** `/new-requirement` runs the protocol: acknowledge → list impacted stories →
update PRD → revise stories (mark `[REVISED]`, never delete) → log in CONTEXT.md → ask
whether in-flight work should pause.

## 9. Decisions Outlive Sessions — Log the WHY

Code shows *what*; only the log shows *why* — which alternatives were rejected and what
constraint forced the choice. Without it, future sessions re-litigate settled questions or,
worse, "fix" deliberate choices.

**Mechanism:** CONTEXT.md is append-only. Any decision with alternatives, any non-obvious
bug root-cause, any integration quirk gets an entry. Reading it is step one of `/resume`.

## 10. The Docs Are the Project. Code Is an Output.

If STORIES.md is stale, the project's dashboard lies. If SESSION_STATE.md is stale, the
project cannot survive a session boundary. Documentation isn't overhead added after the
work — it is the mechanism by which multi-session work is possible at all.

**Mechanism:** `/handoff` is a hard gate at session end: stories current, context logged,
changelog updated, session state accurate. A session that skips `/handoff` has silently
taxed the next session.

---

## Model-Specific Tuning

The blueprint works unchanged on all models. Adjust only the *grain size*:

| | Opus-class | Sonnet-class |
|---|---|---|
| Plan step size | Can take larger steps per loop iteration | Smaller steps; verify more often |
| Checkpoint cadence | Every 45 min / before big ops | Every 30 min / before big ops |
| Ambiguity | May resolve minor ambiguity itself (log it) | Should ask, or pick the conservative option and log it |
| Subagent use | For exploration | For exploration AND for isolated mechanical subtasks |
| Self-review | Re-read own diff before verify | Re-read own diff AND re-run scenarios from scratch |

The rule of thumb: **the less capable the model, the smaller the loop and the more often
the state hits disk.** Nothing else changes.

---

## Anti-Patterns (all models)

- ❌ "Let me just quickly also fix..." → file a story instead.
- ❌ Marking Done because the code "looks right" → run the scenario.
- ❌ Answering "what's the status?" from memory → read STORIES.md and SESSION_STATE.md.
- ❌ A 3-hour session with no checkpoint → one crash from losing the thread.
- ❌ Re-explaining a past decision from scratch → link the CONTEXT.md entry.
- ❌ TODO comments in code → they die there; stories don't.
- ❌ Deleting or rewriting old ACs/decisions → mark `[REVISED]`; history is data.
