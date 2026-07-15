# ARCHITECTURE.md — System Design

> Living document: the WHAT of the system as it is NOW (history lives in CONTEXT.md).
> Update when: components are added/changed, data models change, integrations change,
> or folder structure changes materially.

**Last Updated:** YYYY-MM-DD

## Overview
[One paragraph: what the system is and its core approach]

## Component Map
```
[ASCII diagram or list of major components and how they connect]
```

## Components
### [Component Name]
- **Responsibility:** [one concern]
- **Location:** `src/...`
- **Depends on:** [components, external services]
- **Key decisions:** [link CONTEXT.md entries by date]

## Data Models
[Core entities, their fields, and relationships]

## External Integrations
| Service | Purpose | Auth | Quirks (see CONTEXT.md) |
|---------|---------|------|--------------------------|

## Cross-Cutting Concerns
- **Error handling:** [strategy]
- **Logging:** [strategy]
- **Configuration:** [where config lives, how secrets are handled]
- **Testing strategy:** [unit/integration/scenario split]
