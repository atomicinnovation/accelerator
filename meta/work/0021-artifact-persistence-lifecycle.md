---
title: "Artifact persistence lifecycle"
type: adr-creation-task
status: todo
---

# ADR Ticket: Artifact persistence lifecycle

## Summary

In the context of review and other skill outputs being ephemeral, we decided
for the principle that every skill producing structured output must write to
`meta/`, with `meta/reviews/plans/` and `meta/reviews/prs/` for persistent
review artifacts and `meta/tmp/` kept purely ephemeral, to achieve a complete
audit trail, cross-session handoff, and a clean semantic boundary (tmp is
always safe to delete, reviews are always committed), accepting additional I/O
and reversal of earlier "don't write to file" guidelines.

## Context and Forces

- Review skills (`review-pr`, `review-plan`) discarded structured outputs to
  conversation only — lost after session ends
- Other skills already write persistent artifacts: `research-codebase` writes
  to `meta/research/`, `create-plan` writes to `meta/plans/`
- Skills cannot access another skill's conversation history
- Cross-team collaboration requires artifacts that persist beyond a single
  session
- `meta/tmp/` was mixing ephemeral working data (diffs, patches) with review
  outputs, creating confusion about what to keep
- The plugin's core architectural principle is filesystem as message bus

## Decision Drivers

- Complete audit trail: every significant output should be recoverable
- Cross-session handoff: teammates and future sessions should access prior
  outputs
- Clean semantic boundary: clear distinction between ephemeral and persistent
- Consistency with existing artifact patterns (research, plans)

## Considered Options

1. **Conversation-only** — Outputs stay in conversation. Lost after session.
2. **All in meta/tmp/** — Mix ephemeral and persistent. Unclear what to keep.
3. **Persistent artifacts + ephemeral tmp** — Reviews go to
   `meta/reviews/{plans,prs}/` as numbered, never-replaced files with
   appendable re-reviews. Diffs, patches, and working data stay in
   `meta/tmp/` (gitignored, always safe to delete).

## Decision

We will adopt the principle that every skill producing structured output
valuable to a different team member or future session must write to `meta/`.
Reviews are persisted to `meta/reviews/plans/` and `meta/reviews/prs/` as
self-contained documents with machine-parseable frontmatter, per-lens results
as markdown sections (not separate files), and appendable re-review history.
`meta/tmp/` is kept purely ephemeral. This reverses the explicit "don't write
to file" guidelines in both review skills.

## Consequences

### Positive
- Complete audit trail for all significant skill outputs
- Cross-session and cross-team visibility of review results
- Clean semantic boundary: `meta/tmp/` is always safe to delete,
  `meta/reviews/` is always committed
- Consistent with existing research and plan artifact patterns
- Enables downstream skills to consume review artifacts

### Negative
- Additional file I/O and disk usage
- Reversal of earlier guidelines requires updating review skill instructions
- Numbered, never-replaced files accumulate over time

### Neutral
- Review artifacts follow the immutable-file-with-appendable-re-reviews pattern,
  distinct from the date-prefixed pattern used by research and plans

## Source References

- `meta/research/2026-03-18-meta-management-strategy.md` — Filesystem-as-
  shared-memory principle and review persistence strategy
