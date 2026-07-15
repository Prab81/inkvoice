# CONTEXT.md — Append-Only Decision & Discovery Log

> The WHY of the project. Future sessions read this to avoid re-litigating settled
> questions and re-hitting known landmines. Append at the TOP. Never edit old entries.
>
> LOG: architectural decisions (+ rejected alternatives), dependency choices, non-obvious
> bug root causes, integration quirks, scope changes, perf/security findings.
> DON'T LOG: obvious implementation details, debugging dead-ends, preferences without rationale.

---

## [YYYY-MM-DD] — Project Bootstrapped from Blueprint
**Type:** Decision
**Impact:** High

### Context
Project initialized using the Project Execution Blueprint.

### Decision / Finding
Adopted the blueprint workflow: doc-driven execution, session-state checkpointing,
evidence-based verification, one-story-in-flight.

### Rationale
Ensures consistent, high-quality execution across sessions and across models.

### Implications
All sessions must follow CLAUDE.md protocols. SESSION_STATE.md is the resume point.

---
