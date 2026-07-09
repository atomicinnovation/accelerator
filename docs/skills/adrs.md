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

### <img src="https://api.iconify.design/ph/scroll-bold.svg?color=%237c3aed" width="18" align="center" alt=""> `/create-adr [topic or description]`

Interactively create an architecture decision record (ADR). Walks through context
gathering, options analysis, and consequence documentation.

*Pass `--supersedes ADR-NNNN` when the new decision replaces an accepted one —
the superseded ADR stays immutable and is marked, rather than edited.*

### <img src="https://api.iconify.design/ph/export-bold.svg?color=%237c3aed" width="18" align="center" alt=""> `/extract-adrs [research doc paths...]`

Extract architecture decision records from existing meta documents (research,
plans).

*Leave the paths empty to scan all. Use when decisions are buried in research or
planning documents and need to be captured formally; it proposes candidates for
you to select before writing.*

### <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%237c3aed" width="18" align="center" alt=""> `/review-adr [path to ADR]`

Review an architecture decision record for quality and completeness, then accept,
reject, or suggest revisions.

*Enforces ADR immutability: only **proposed** ADRs can be modified, and an
accepted ADR can only transition to superseded or deprecated (use
`--deprecate reason`).*

ADRs follow an append-only lifecycle: once accepted, an ADR's content becomes
immutable. To revise a decision, create a new ADR that supersedes the original.
