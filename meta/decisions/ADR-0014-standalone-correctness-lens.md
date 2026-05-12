---
adr_id: ADR-0014
date: "2026-04-17T15:15:33+01:00"
author: Toby Clemson
status: accepted
tags: [review-system, lenses, correctness]
---

# ADR-0014: Standalone Correctness Lens

**Date**: 2026-04-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

The review system (ADR-0002, ADR-0003) uses specialist lenses selected per
review. The initial gap analysis against industry frameworks — Google
engineering practices, OWASP, ISO 25010 — flagged **functional correctness**
as a recognised review dimension but deliberately excluded it from the lens
catalogue, recommending it be "left implicit" as a cross-cutting concern that
every reviewer checks by default
(`meta/research/codebase/2026-02-22-review-lens-gap-analysis.md`). The reasoning: making
correctness an explicit lens risked overlap with test coverage (edge cases) and
code quality (error handling).

Subsequent experience with the review system showed the opposite: leaving
correctness implicit produced a coverage gap for logical validity, boundary
conditions, and state-management errors. These defects fall between the
existing lenses — code quality evaluates maintainability and readability,
test coverage evaluates whether defects would be caught by tests, but neither
is positioned to evaluate whether the code is *logically sound* in the first
place. An implementation can be clean and well-tested against its author's
mental model, yet still contain off-by-one errors, uncovered enum branches,
missing null handling, or invariant violations that no lens is structurally
focused on finding.

The catalogue expansion in ADR-0004 added six new lenses and established a
"core four" (Architecture, Code Quality, Test Coverage, Correctness) always
included by the orchestrator. That decision presupposed correctness being
present as a lens — this ADR documents the reversal of the original exclusion
and defines the lens's scope and boundaries.

## Decision Drivers

- **Demonstrated coverage gap**: logical validity, boundary conditions, and
  state-management errors fall between code quality (maintainability) and test
  coverage (defect detection) and were not being caught reliably under the
  implicit-correctness approach
- **Distinct reviewer persona (PBR principle)**: formal-verifier analysis —
  case enumeration, invariant preservation, boundary evaluation, state
  transition validity — is a different analytical stance from the
  maintainability mindset of code quality or the defect-detection mindset of
  test coverage; bundling them degrades all three
- **Universal relevance**: unlike domain-specific lenses that apply to a
  subset of changes, logical correctness is relevant to every non-trivial
  change and belongs in the core four alongside architecture, code quality,
  and test coverage
- **Reversing the earlier exclusion requires explicit boundaries**: the
  original rejection warned of overlap with test coverage and code quality;
  adding the lens is only defensible with clean three-way boundary statements
  that address that concern directly (additional boundaries with adjacent
  lenses — performance, database, safety — are required for catalogue
  completeness but are secondary to the reversal justification)

## Considered Options

1. **Keep correctness implicit** — Retain the original exclusion: rely on
   code quality and test coverage to catch logic errors as a side effect of
   their primary focus, with a brief reminder in the reviewer agent's
   behavioural conventions. Avoids adding to the catalogue and preserves the
   original gap-analysis judgement. Leaves the demonstrated coverage gap
   unaddressed — neither lens is structurally positioned to evaluate logical
   soundness, and cross-cutting reminders have not produced reliable
   correctness findings in practice.

2. **Fold correctness into code quality** — Expand the code quality lens
   with a logical-correctness Core Responsibility covering case completeness,
   boundary conditions, and invariant preservation. Avoids adding a lens.
   Broadens code quality past a coherent scope — it already owns complexity,
   design principles, readability, error-handling structure, and
   observability; adding formal-verifier analysis conflates the
   maintainability mindset with logical soundness and repeats the overload
   pattern ADR-0013 explicitly rejected for the architecture lens.

3. **Fold correctness into test coverage** — Extend test coverage to
   evaluate whether the code handles logical edge cases, not just whether
   tests exist for them. Keeps correctness adjacent to where edge-case
   thinking already happens. Inverts the test coverage lens's purpose —
   it evaluates whether tests would catch defects, not whether defects
   exist; asking one lens to do both conflates "is this tested?" with "is
   this correct?" and makes findings harder to act on.

4. **Standalone correctness lens** — A dedicated lens with a formal-verifier
   persona covering three Core Responsibilities: logical correctness and
   invariant preservation (case completeness, arithmetic safety, pre/post
   conditions); boundary conditions and edge cases (off-by-one, null
   propagation, empty/max collections); and state management and transition
   validity (state machine soundness, initialisation completeness, atomic
   mutations). Gives logical soundness its own reviewer persona with clean
   three-way boundaries against code quality (maintainability) and test
   coverage (defect detection). Requires explicit boundary statements and
   joins the core four, adding one entry to the always-run set.

## Decision

We will add a standalone correctness lens that evaluates logical soundness
with the reviewer persona of a **formal verifier** — checking whether the
code's logic is sound under all valid inputs and state transitions. Its scope
covers three Core Responsibilities: **logical correctness and invariant
preservation** (case completeness in conditionals and enum/switch handling,
arithmetic safety including overflow/underflow/precision, loop-invariant
soundness, pre/postcondition maintenance across function boundaries);
**boundary conditions and edge cases** (empty/single/max collections, null
and undefined propagation, off-by-one in loops and indexing, locale- and
unicode-sensitive operations); and **state management and transition
validity** (state machine soundness, atomic mutations where required,
initialisation completeness, cleanup in all code paths including error
paths).

The lens joins the **core four** established in ADR-0004 — Architecture,
Code Quality, Test Coverage, and Correctness — and is always included by the
orchestrator (consistent with ADR-0004's core-four policy of never skipping
these lenses), reflecting logical soundness being universally relevant rather
than domain-specific.

We draw explicit boundaries with adjacent lenses:

- **Code quality** retains maintainability, readability, design principles,
  and error-handling *structure*. Correctness assesses whether the logic is
  *sound*; code quality assesses whether it is *well-expressed*. Error
  handling sits at this seam: whether errors are well-structured and
  well-logged belongs to code quality; whether every code path (including
  error paths) preserves invariants belongs to correctness.
- **Test coverage** retains whether tests exist, whether they exercise the
  right cases, and whether they would catch regressions. Correctness
  assesses whether the *code itself* is correct, independent of whether any
  test would catch incorrectness. The seam is directional: correctness may
  identify a logic error that no test covers; test coverage may identify a
  missing test without requiring the code to be wrong.
- **Performance** (ADR-0011) retains concurrency safety — race conditions,
  deadlocks, lock contention. Correctness scopes its analysis to logical
  soundness *assuming single-threaded execution*. The boundary is the
  threading model: anything that requires reasoning about interleaving
  belongs to performance; anything provable without it belongs to
  correctness.
- **Database** retains SQL and query-logic correctness. The carve-out
  matters because the word "correctness" easily drifts into SQL territory
  without an explicit boundary — this lens evaluates the surrounding
  application logic, not query construction or result-set semantics.
- **Safety** (ADR-0013) retains safeguards against accidental harm —
  destructive-operation protection, blast-radius containment, fail-safe
  defaults. Correctness assesses whether the implementation logic is sound;
  safety assesses whether that logic has guardrails when it goes wrong.

## Consequences

### Positive

- The logical-validity coverage gap is closed with a dedicated reviewer
  persona — reviews now surface a class of defects (off-by-one errors,
  uncovered enum branches, missing null handling, broken invariants,
  unsound state transitions) that were falling between code quality and
  test coverage under the implicit-correctness approach.
- The formal-verifier persona produces findings the other lenses are
  structurally unable to produce — case enumeration and invariant-based
  reasoning are analytical stances distinct from maintainability review
  and test-strategy review.
- Core-four inclusion guarantees correctness is evaluated on every
  non-trivial review, providing a consistent logical-soundness baseline
  alongside architecture, code quality, and test coverage.
- Reviewer output aligns with ISO 25010's Functional Suitability and with
  industry frameworks (e.g., Google engineering practices, Augment Code's
  Logic & Correctness pillar) that treat correctness as a primary review
  dimension.

### Negative

- Reverses a prior deliberate exclusion
  (`meta/research/codebase/2026-02-22-review-lens-gap-analysis.md`). The original
  concerns — overlap with test coverage on edge cases and with code quality
  on error handling — must be actively managed through the boundary
  statements, and the boundaries must be maintained as adjacent lenses
  evolve.
- The error-handling seam between correctness (every path preserves
  invariants) and code quality (errors are well-structured and well-logged)
  is subtle — reviewers must label consistently or consumers will see
  duplicate findings.
- The test-coverage seam ("is this correct?" vs "is this tested?") will
  occasionally surface both lenses flagging the same underlying edge case
  from different angles; the deduplication layer (ADR-0009) must handle
  this gracefully.
- Every existing lens must add correctness to its "What NOT to Do" list —
  the linear boundary-list maintenance cost ADR-0004 already flagged,
  incurred again.

### Neutral

- Concurrency correctness (race conditions, deadlocks, lock contention) is
  scoped to the performance lens under the single-threaded-execution
  constraint. This is a deliberate partitioning, not a gap — reasoning
  about interleaving requires a different analytical stance and belongs
  with the lens that already owns it.
- The correctness lens shares the same structural invariants as other
  lenses (ADR-0003) — six sections, conditional applicability sub-groups,
  severity tiers, and output format. No new lens-infrastructure patterns
  are introduced.
- Available to both PR and plan reviews via the generic reviewer agent
  (ADR-0005); no orchestrator-level changes beyond adding it to the core
  four, which ADR-0004 already anticipated.
- Core-four membership means correctness does not compete with
  domain-specific lenses for the selection cap — it's always included,
  freeing the selection logic to rank only the remaining slots.

## References

- `meta/research/codebase/2026-02-22-review-lens-gap-analysis.md` — Original gap
  analysis that excluded correctness as "a cross-cutting concern best left
  implicit" (the exclusion this ADR reverses)
- `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md` — Open
  Questions noting correctness as a deliberately excluded coverage gap
- `meta/plans/2026-03-15-new-review-lenses.md` — Implementation plan
  (Phase 3) defining the correctness lens scope, formal-verifier persona,
  three Core Responsibilities, and boundary statements
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` —
  Three-layer review architecture the lens sits within
- `meta/decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md` —
  Lens design principles the correctness lens conforms to
- `meta/decisions/ADR-0004-lens-catalogue-expansion-with-bounded-selection-and-core-set.md` —
  Catalogue expansion decision that identified correctness as one of six
  new lenses and established its core-four status
- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md` —
  Generic reviewer pattern that makes adding a lens a skill-only change
- `meta/decisions/ADR-0009-dual-gate-deduplication-with-spatial-proximity-and-semantic-similarity.md` —
  Deduplication that handles the test-coverage seam
- `meta/decisions/ADR-0011-standalone-performance-lens.md` — Sibling
  decision; holds concurrency correctness under the threading-model
  boundary
- `meta/decisions/ADR-0013-standalone-safety-lens.md` — Sibling decision;
  precedent for carving out a standalone lens with explicit boundaries
