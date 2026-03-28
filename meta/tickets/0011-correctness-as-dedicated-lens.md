---
title: "Correctness as a dedicated lens"
type: adr-creation-task
status: todo
---

# ADR Ticket: Correctness as a dedicated lens

## Summary

In the context of continued coverage gaps for logical validity, reversing an
earlier exclusion, we decided for a dedicated correctness lens reviewing as a
formal verifier to achieve explicit coverage of logic errors that fall between
code quality and test coverage, accepting three-way boundary statements.

## Context and Forces

- An earlier gap analysis deliberately excluded correctness as a standalone lens
- Continued experience revealed coverage gaps for logical validity, boundary
  conditions, and state management errors
- Logic errors fall in a gap between code quality (which focuses on
  maintainability) and test coverage (which focuses on whether tests exist)
- Neither the code quality nor test coverage lens is well-positioned to evaluate
  whether code is *logically correct*
- Correctness becomes one of the "core four" always-consider lenses, reflecting
  its fundamental importance

## Decision Drivers

- Coverage gap: logical validity was not explicitly evaluated by any lens
- The "formal verifier" perspective is distinct from maintainability (code
  quality) and test existence (test coverage)
- Correctness is universally relevant — it belongs in the "core four"
- Reversing the earlier exclusion requires clear boundary definitions

## Considered Options

1. **Keep excluded** — Rely on code quality and test coverage to catch logic
   errors. Leaves a demonstrated gap.
2. **Fold into code quality** — Expand code quality to cover correctness.
   Broadens an already broad lens and mixes maintainability with logical
   soundness.
3. **Standalone correctness lens** — Review as a formal verifier evaluating
   logical soundness, boundary conditions, state transitions, invariant
   preservation, and error handling completeness.

## Decision

We will create a dedicated correctness lens with the persona of a formal
verifier. It evaluates: logical soundness (assuming single-threaded execution),
boundary conditions, state machine transitions, invariant preservation, and
error handling completeness. The boundary: correctness = logical soundness of
implementation; code quality = maintainability and readability; test coverage =
whether tests catch defects. Correctness joins the "core four" always-consider
set.

## Consequences

### Positive
- Explicit coverage of the logic error gap
- The formal verifier persona catches issues other lenses miss
- Core four status ensures correctness is always evaluated

### Negative
- Three-way boundary statements needed between correctness, code quality, and
  test coverage
- Adds another lens to the catalogue
- Reverses a prior deliberate decision (requires documenting why)

### Neutral
- The "assuming single-threaded execution" constraint places concurrency
  correctness in the performance lens instead

## Source References

- `meta/plans/2026-03-15-new-review-lenses.md` — Correctness lens design and
  core four designation
- `meta/research/2026-03-15-review-lens-optimal-structure.md` — Earlier
  exclusion noted in Open Questions
