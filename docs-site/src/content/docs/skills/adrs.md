---
title: Architecture Decision Records (ADRs)
---

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

- [`create-adr`](../reference/skills/decisions/create-adr.md) drafts a
  decision record interactively — context gathering, options analysis,
  consequence documentation. Pass `--supersedes ADR-NNNN` when the new
  decision replaces an accepted one.
- [`extract-adrs`](../reference/skills/decisions/extract-adrs.md) scans
  existing meta documents (research, plans) for decisions made in
  passing and proposes candidates to formalise. Use it when decisions
  are buried in prose rather than starting from scratch.
- [`review-adr`](../reference/skills/decisions/review-adr.md) reviews a
  proposed ADR for quality and completeness, then accepts, rejects, or
  suggests revisions.

ADRs follow an append-only lifecycle: once accepted, an ADR's content
becomes immutable — `review-adr` enforces this, allowing an accepted ADR
to transition only to superseded or deprecated (`--deprecate reason`).
To revise a decision, create a new ADR that supersedes the original. See
[The meta/ directory](../reference/meta-directory.md) for the full
lifecycle.
