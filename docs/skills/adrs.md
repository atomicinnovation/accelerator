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

### `/create-adr`

**What it does** — Interactively create an architecture decision record (ADR).
Walks through context gathering, options analysis, and consequence
documentation.

**How to use it** — `/create-adr [topic or description] [--supersedes ADR-NNNN]`

**Advice & guidelines** — Pass `--supersedes ADR-NNNN` when the new decision
replaces an accepted one — the superseded ADR stays immutable and is marked,
rather than edited.

### `/extract-adrs`

**What it does** — Extract architecture decision records from existing meta
documents (research, plans).

**How to use it** — `/extract-adrs [research doc paths...]` (leave
empty to scan all)

**Advice & guidelines** — Use when decisions are buried in research or planning
documents and need to be captured formally; it proposes candidates for you to
select before writing.

### `/review-adr`

**What it does** — Review an architecture decision record for quality and
completeness, then accept, reject, or suggest revisions.

**How to use it** — `/review-adr [path to ADR] [--deprecate reason]`

**Advice & guidelines** — Enforces ADR immutability: only *proposed* ADRs can be
modified, and an accepted ADR can only transition to superseded or deprecated
(use `--deprecate`).

ADRs follow an append-only lifecycle: once accepted, an ADR's content becomes
immutable. To revise a decision, create a new ADR that supersedes the original.
