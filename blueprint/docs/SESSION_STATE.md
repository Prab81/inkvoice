# SESSION_STATE.md — Live Resume Point

> **Purpose:** a brand-new session (any model, zero context) must be able to read this
> file and continue the work within two minutes. This file is OVERWRITTEN at every
> `/checkpoint` — it reflects NOW, not history (history lives in BUILD_LOG.md and CONTEXT.md).
>
> Update triggers: story step completed · every ~30 min · before risky/large operations ·
> session end (via /handoff). Staleness here is the #1 project risk.

---

**Last checkpoint:** YYYY-MM-DD HH:MM
**Checkpointed by:** [model name / session note]
**Project phase:** [e.g., Phase 2 of EXECUTION_PLAN.md — core pipeline]

## Current Story
**STORY-000:** [title] — Status: [In Progress]
**Plan step:** [which step of the story's written plan we are on, e.g., "step 3 of 5"]

## Exact Next Action
> The single most specific instruction possible. Not "continue the feature" but
> "In `src/foo/bar.py`, implement `parse_header()` per plan step 3; then run
> `pytest tests/test_bar.py -k header` — 2 of its 4 tests currently fail as expected."

[NEXT ACTION HERE]

## State of the Working Tree
- Uncommitted changes: [none | list files and why]
- Last commit: `[hash] [STORY-ID] message`
- Branch: [main | feature/...]

## Open Problems / Unknowns
- [Anything unresolved: failing test, undecided approach, question awaiting user]

## Discovered This Session (gotchas the next session must know)
- [e.g., "The vendor SDK silently truncates payloads > 1MB — see CONTEXT.md 2026-07-07"]

## Working Commands (verified this session)
```powershell
# build:    [command]
# test:     [command]
# run:      [command]
```

## Do NOT
- [Session-specific warnings, e.g., "don't touch config.yaml — user edits it manually"]
