---
adr_id: ADR-0027
date: "2026-05-17T20:13:38+00:00"
author: Toby Clemson
status: accepted
tags: [artifacts, persistence, reviews, meta-directory]
---

# ADR-0027: Persist structured skill outputs to meta/

**Date**: 2026-05-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

The plugin's workflows are organised into clear-context phases: distinct
steps that depend on filesystem state as their input and output, rather
than on a shared conversation. Within a single session multiple skills
may share context, but across phase boundaries — and across sessions or
team members — the filesystem is the message bus.

Two patterns coexisted:

- `research-codebase` writes to `meta/codebase/` and `create-plan`
  writes to `meta/plans/` — outputs persist and are recoverable later.
- `review-pr` and `review-plan` carried explicit "don't write to file"
  guidelines — structured outputs lived only in conversation and were
  lost as soon as the next phase started or the session ended.

At the same time, `meta/tmp/` was mixing two concerns: ephemeral working
data (diffs, patches, scratch) and outputs that subsequent phases or
team members would want. This made it unclear what was safe to delete.

Cross-phase handoff, cross-session continuity, and re-review workflows
all require artifacts that outlive the conversation that produced them.

## Decision Drivers

- Complete audit trail: every significant skill output should be
  recoverable later
- Cross-phase, cross-session, and cross-team handoff: subsequent phases,
  future sessions, and teammates must be able to read prior outputs
- Historical record: a richer corpus of persisted artifacts helps future
  sessions understand why the codebase is designed the way it is and
  what historic decisions were made
- Clean semantic boundary: ephemeral vs persistent storage must be
  unambiguous
- Consistency with existing artifact patterns already used by
  `research-codebase` and `create-plan`

## Considered Options

1. **Conversation-only outputs** — Skills emit structured results to the
   conversation and nowhere else. Lost at the next phase boundary.
2. **Everything in `meta/tmp/`** — Both ephemeral working data and review
   outputs share one directory. Persistence depends on someone
   remembering to copy files elsewhere.
3. **Persistent artifacts under `meta/`, ephemeral under `meta/tmp/`** —
   Skills producing structured output write to `meta/`. Reviews
   specifically land in `meta/reviews/plans/` and `meta/reviews/prs/`
   as numbered, never-replaced files with appendable re-review history.
   `meta/tmp/` stays purely ephemeral.

## Decision

We will adopt the principle that every skill producing structured output
valuable to a later phase, future session, or another team member must
write to `meta/`. Reviews are persisted to `meta/reviews/plans/` and
`meta/reviews/prs/` as self-contained documents. `meta/tmp/` is kept
purely ephemeral.

This reverses the explicit "don't write to file" guidelines previously
present in `review-pr` and `review-plan`.

### Artifact shape

The review artifact format — machine-parseable frontmatter, per-lens
results as markdown sections (not separate files), and appendable
re-review history — is an implementation detail of the review skills
rather than part of this decision, and is specified in the originating
plan referenced below.

## Consequences

### Positive

- Complete audit trail for all significant skill outputs
- Cross-phase, cross-session, and cross-team visibility of review results
- Persisted artifacts form a historical record that helps future
  sessions understand prior reasoning and decisions
- Clean semantic boundary: `meta/tmp/` is always safe to delete,
  `meta/reviews/` is always committed
- Consistent with existing `research-codebase` and `create-plan` patterns
- Enables downstream skills to consume review artifacts as inputs

### Negative

- Additional file I/O and disk usage on every review run
- Reversal of earlier guidelines requires updating review skill
  instructions

### Neutral

- Review artifacts follow an immutable-file-with-appendable-re-reviews
  pattern, distinct from the date-prefixed pattern used by research and
  plans

## References

- `meta/research/codebase/2026-03-18-meta-management-strategy.md` —
  Filesystem-as-shared-memory principle and review persistence strategy
- `meta/plans/2026-03-22-persist-review-artifacts.md` — Plan that
  introduced persistent review artifacts; specifies the review artifact
  format
- `meta/decisions/ADR-0001-context-isolation-principles.md` —
  Clear-context-phase foundation
- `meta/decisions/ADR-0006-structured-agent-output-contract-with-context-specific-schemas.md` —
  Defines what "structured output" means for agents whose results this
  ADR persists
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md` —
  Precedent for `meta/tmp/` as an ephemeral handoff location
- `meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md` —
  Establishes the ephemeral/persistent boundary that this ADR sharpens
