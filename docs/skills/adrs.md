# Architecture Decision Records (ADRs)

ADR skills capture architectural decisions that emerge from research and
planning:

```
research-codebase  → create-plan → implement-plan
       ↓                  ↓
meta/research/codebase/ meta/plans/
       ↓                  ↓
  extract-adrs ←──────────┘
       ↓
  meta/decisions/
       ↓
  review-adr → accepted ADRs inform future research & planning
```

| Skill            | Usage                                      | Description                                                |
|------------------|--------------------------------------------|------------------------------------------------------------|
| **create-adr**   | `/accelerator:create-adr [topic]`          | Interactively create an ADR with context gathering         |
| **extract-adrs** | `/accelerator:extract-adrs [doc paths...]` | Extract decisions from existing meta documents into ADRs   |
| **review-adr**   | `/accelerator:review-adr [path to ADR]`    | Review proposed ADRs; accept, reject, or suggest revisions |

ADRs follow an append-only lifecycle: once accepted, an ADR's content becomes
immutable. To revise a decision, create a new ADR that supersedes the original.
