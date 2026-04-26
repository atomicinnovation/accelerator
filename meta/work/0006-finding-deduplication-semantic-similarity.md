---
title: "Finding deduplication requires semantic similarity"
type: adr-creation-task
status: done
---

# ADR Ticket: Finding deduplication requires semantic similarity

## Summary

In the context of aggregating findings from multiple parallel agents, facing
the risk of merging unrelated findings at nearby locations, we decided that
deduplication requires both spatial proximity and semantic similarity to achieve
accurate consolidation, accepting that some near-duplicates may remain when in
doubt.

## Context and Forces

- Multiple lenses may independently flag the same underlying issue (e.g.,
  security and code quality both flag an unvalidated input)
- Findings from different lenses may reference nearby lines but describe
  completely different concerns
- Naive proximity-based deduplication would incorrectly merge unrelated findings
- Over-aggressive deduplication loses valuable multi-lens perspectives
- Under-deduplication creates noise and the appearance of finding inflation

## Decision Drivers

- Accuracy: only truly duplicate findings should be merged
- Precision over recall: better to keep a near-duplicate than to incorrectly
  merge unrelated findings
- Practical implementability in the orchestrator
- Consistent user experience across reviews

## Considered Options

1. **No deduplication** — Present all findings as-is. Maximum information but
   noisy.
2. **Proximity-only** — Merge findings referencing the same file and nearby
   lines. Fast but merges unrelated findings.
3. **Semantic similarity + proximity** — Require both spatial proximity (same
   file, lines within 3 of each other) and semantic similarity (same underlying
   concern from different lenses). When in doubt, keep separate.

## Decision

We will require both spatial proximity (same file, lines within 3 of each
other) and semantic similarity (same underlying concern assessed from different
lens perspectives) for deduplication. The principle is "when in doubt, keep
separate" — prioritizing precision over deduplication completeness.

## Consequences

### Positive
- Unrelated findings at nearby locations are preserved as separate items
- Multi-lens perspectives on distinct issues are maintained
- The "when in doubt, keep separate" principle is conservative and safe

### Negative
- Some genuine duplicates may remain separate if semantic similarity is unclear
- Semantic similarity assessment adds complexity to the orchestrator
- The 3-line proximity threshold is a heuristic that may not suit all cases

### Neutral
- The approach applies identically to both PR and plan review aggregation

## Source References

- `meta/plans/2026-02-22-pr-review-inline-comments.md` — Deduplication
  algorithm for PR review inline comments
- `meta/plans/2026-02-22-review-plan-alignment.md` — Deduplication approach
  for plan review findings
