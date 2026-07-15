# TEST_SCENARIOS.md — Scenario-Based Verification

> Every story's ACs map to scenario rows here. /plan-story adds rows (Status: Pending →
> Ready once the how-to-run is defined). /verify-story executes them and fills Evidence.
> An AC may only be ticked when its scenario row shows Pass WITH evidence.
>
> Statuses: Pending (defined, not yet runnable) · Ready (runnable) · Pass · Fail · Blocked

## STORY-001: [Title]

| # | Scenario | Type | How to run | Status | Evidence (output/observation + date) |
|---|----------|------|------------|--------|--------------------------------------|
| 1.1 | [Happy path description] | Happy | `pytest tests/... -k ...` | Pending | |
| 1.2 | [Error state description] | Error | [command or manual steps] | Pending | |
| 1.3 | [Edge case description] | Edge | [command or manual steps] | Pending | |

<!-- Add one section per story. Never delete rows; a superseded scenario gets [REVISED]. -->
