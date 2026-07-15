# Project Execution Blueprint

> **This folder is the canonical blueprint.** Copy it into any new project to get
> Fable-class execution discipline from ANY Claude model (Sonnet, Opus, or future models).
>
> The blueprint does not depend on model intelligence. It depends on **structure**:
> externalized state, small verified loops, and mandatory documentation. A less capable
> model following this structure will outperform a more capable model working ad hoc.

---

## What's In Here

```
blueprint/
├── README.md               # ← You are here. How to use the blueprint.
├── PLAYBOOK.md             # The operating philosophy. Read once, internalize forever.
├── CLAUDE.md               # Template → copy to new project root. Session bootstrap.
├── new-project.ps1         # Bootstrap script: copies blueprint into a target folder.
├── docs/                   # Document templates → copy to <project>/docs/
│   ├── SESSION_STATE.md    # THE key file: live handoff state for long-running work
│   ├── PRD.md              # Product requirements template
│   ├── STORIES.md          # User stories template
│   ├── EXECUTION_PLAN.md   # Phased build plan template
│   ├── CONTEXT.md          # Append-only decision log template
│   ├── ARCHITECTURE.md     # System design template
│   ├── TEST_SCENARIOS.md   # Scenario-based verification template
│   ├── BUILD_LOG.md        # Per-session build journal template
│   └── CHANGELOG.md        # What shipped and why
└── .claude/
    └── skills/             # Slash-command skills → copy to <project>/.claude/skills/
        ├── kickoff/        # /kickoff       — idea → PRD → stories → plan
        ├── resume/         # /resume        — session-start ritual (load state, continue)
        ├── plan-story/     # /plan-story    — plan one story before touching code
        ├── execute/        # /execute       — disciplined implement loop
        ├── checkpoint/     # /checkpoint    — save state NOW (cheap, run often)
        ├── verify-story/   # /verify-story  — run scenarios, tick ACs, mark Done
        ├── handoff/        # /handoff       — end-of-session wrap-up
        └── new-requirement/# /new-requirement — change management protocol
```

## Starting a New Project

### Option A — script (from this folder)
```powershell
.\new-project.ps1 -Target "V:\AI\MyNewProject"
```

### Option B — manual
1. Create the project folder.
2. Copy `CLAUDE.md` to the project root.
3. Copy `docs/` to `<project>/docs/`.
4. Copy `.claude/` to `<project>/.claude/`.
5. `git init` and make the first commit: `[SETUP] Bootstrap from blueprint`.
6. Open Claude Code in the project and run `/kickoff <describe your idea>`.

## Daily Rhythm (any model, any session)

| When | Command | What happens |
|------|---------|--------------|
| Session starts | `/resume` | Loads SESSION_STATE, STORIES, CONTEXT; states what's next; continues |
| New idea / project | `/kickoff` | PRD → stories → execution plan → test scenarios |
| Before coding a story | `/plan-story STORY-ID` | Written plan, files to touch, test scenarios, risks |
| While coding | `/execute STORY-ID` | Small loop: implement → test → record. Repeats. |
| Every 30–45 min OR before anything big | `/checkpoint` | SESSION_STATE.md updated so any future session can resume |
| Story complete | `/verify-story STORY-ID` | Runs every scenario, demands evidence, ticks ACs |
| Requirements change | `/new-requirement` | Impact analysis, PRD/story revision, logged |
| Session ends | `/handoff` | All docs current, state saved, clean stopping point |

## The One Rule That Matters Most

**If it isn't written in a file, it doesn't exist.**
Context windows compact, sessions end, models change. Files survive.
Every session must be able to die at any moment and be resumed by a fresh
session — possibly a different model — with zero loss.

Read `PLAYBOOK.md` for the full philosophy.
