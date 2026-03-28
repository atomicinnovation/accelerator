---
adr_id: ADR-0009
date: "2026-03-30T00:55:06+00:00"
author: Toby Clemson
status: accepted
tags: [review-system, deduplication, aggregation, orchestration]
---

# ADR-0009: Dual-Gate Deduplication with Spatial Proximity and Semantic Similarity

**Date**: 2026-03-30
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system's three-layer architecture (ADR-0002) runs multiple specialist
agents in parallel, each producing structured findings (ADR-0006) that the
orchestrator must aggregate into a unified review. When the lens catalogue
includes 13 or more lenses, independently-produced findings frequently reference
the same or nearby code locations.

Several forces shape the deduplication approach. Multiple lenses may
independently flag the same underlying issue — for example, both security and
code quality might flag an unvalidated input on the same line. Conversely,
findings from different lenses may reference nearby lines but describe entirely
unrelated concerns — a security finding about missing authentication on line 42
and a code quality finding about naming on line 44 are distinct issues that
happen to be spatially close. Naive proximity-based deduplication would
incorrectly merge such unrelated findings, losing the distinct perspectives that
multi-lens review is designed to provide. Meanwhile, under-deduplication creates
noise and the appearance of finding inflation, undermining user trust in the
review output.

This applies to both PR review (where proximity means same file, same side, and
overlapping line ranges) and plan review (where proximity means overlapping plan
section references). The deduplication algorithm is implemented by the
orchestrator layer and is lens-agnostic.

## Decision Drivers

- **Accuracy**: only truly duplicate findings — the same concern identified by
  different lenses — should be merged
- **Precision over recall**: better to keep a near-duplicate than to incorrectly
  merge unrelated findings, since distinct findings are easier to resolve
  individually
- **Lens-agnostic implementation**: the orchestrator applies the same
  deduplication logic regardless of which lenses produced the findings
- **Consistent user experience**: reviews should neither feel inflated with
  duplicates nor stripped of legitimate multi-lens perspectives

## Considered Options

1. **No deduplication** — Present all findings as-is. Preserves every lens
   perspective but creates noise when multiple lenses flag the same concern,
   producing inflated finding counts that undermine user trust.

2. **Proximity-only deduplication** — Merge findings that reference the same
   file and nearby lines (within 3 lines for PR review) or overlapping plan
   sections (for plan review). Fast and simple to implement, but merges
   unrelated findings that happen to be spatially close — destroying the
   distinct perspectives that justify running multiple lenses.

3. **Dual-gate deduplication (proximity + semantic similarity)** — Require both
   spatial proximity (same file, same side, lines within 3 of each other for PR;
   overlapping section references for plan) and semantic similarity (same
   underlying concern from different lens perspectives). When in doubt, keep
   findings separate.

## Decision

We will require both spatial proximity and semantic similarity for finding
deduplication, with a "when in doubt, keep separate" default.

For PR review, spatial proximity means same file, same diff side, and line
numbers within 3 of each other. For plan review, it means overlapping or
identical plan section references. Spatial proximity is a necessary
precondition — findings in different files or distant lines are never candidates
for merging.

Semantic similarity means the findings address the same underlying concern from
different lens perspectives. Two findings about input validation — one from
security, one from code quality — are semantically similar. A security finding
about authentication and a code quality finding about naming on an adjacent line
are not, despite spatial proximity.

When findings pass both gates and are merged, the orchestrator combines the
bodies with lens attribution, takes the highest severity, takes the highest
confidence, and notes all contributing lenses in the title. When either gate
fails, or when semantic similarity is ambiguous, findings remain separate.

## Consequences

### Positive

- Unrelated findings at nearby locations are preserved as separate items —
  multi-lens perspectives on distinct issues are maintained
- The "when in doubt, keep separate" default is conservative and safe — false
  negatives (missed deduplication) are less harmful than false positives
  (incorrectly merged findings)
- The same dual-gate algorithm applies identically to both PR and plan review
  aggregation, differing only in the definition of spatial proximity

### Negative

- Some genuine duplicates may remain separate when semantic similarity is
  ambiguous — users may occasionally see near-duplicates
- Semantic similarity assessment relies on the orchestrator's judgement, which
  cannot be precisely specified as a deterministic rule — this is inherently
  fuzzy
- The 3-line proximity threshold for PR review is a heuristic that may not suit
  all cases (e.g., related findings separated by a larger block of unchanged
  code)

### Neutral

- The deduplication algorithm operates on the structured output defined in
  ADR-0006 — it depends on per-finding `lens`, `severity`, `confidence`, and
  location fields being present

## References

- `meta/plans/2026-02-22-pr-review-inline-comments.md` — PR review
  deduplication algorithm (Step 4, item 4)
- `meta/plans/2026-02-22-review-plan-alignment.md` — Plan review deduplication
  algorithm (Phase 4, Step 4, item 3)
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  architecture defining the orchestrator layer that implements deduplication
- `meta/decisions/ADR-0006-structured-agent-output-contract-with-context-specific-schemas.md`
  — Structured output contract providing the fields deduplication operates on
