---
adr_id: ADR-0015
date: "2026-04-17T16:33:16+00:00"
author: Toby Clemson
status: accepted
tags: [review, lenses, plan-review, test-coverage, code-quality]
---

# ADR-0015: Standalone Test-Coverage Lens

**Date**: 2026-04-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

The three-layer review system (ADR-0002) runs two orchestrators: `review-plan`
and `review-pr`. Each selects review lenses from a shared catalogue (ADR-0004)
and delegates to the generic reviewer agent (ADR-0005) with lens-specific
knowledge injected at runtime.

Plan review and PR review had evolved asymmetrically. PR review was built with
a dedicated test-coverage lens from the outset — evaluating actual test *code*
in the diff. Plan review was not: testing concerns were absorbed into the code
quality lens, which owned a combined "Testability and Testing Strategy"
responsibility covering both design-for-testability (dependency injection,
interface abstractions) and testing strategy (test pyramid, edge case
identification, mock strategy).

Earlier alignment research concluded test coverage was PR-only because plans
have no test code to review. That reasoning conflated two distinct targets:
test *code* (reviewable only on a PR) and testing *strategy* (reviewable from a
plan's prose). Testing-strategy review is analogous to how the security lens
reviews security *design* in plans versus security *code* in PRs — the lens is
meaningful in both contexts, with different subjects.

Overloading code quality with testing strategy also blurred a lens boundary:
*design-for-testability* is a property of the production code and belongs with
maintainability concerns, while *testing strategy* is a property of the test
plan and belongs with coverage concerns. Keeping both inside code quality made
the lens less focused and made the plan review catalogue structurally
different from PR review without a principled reason.

## Decision Drivers

- **Catalogue symmetry between plan and PR review** — a shared lens catalogue
  is easier to reason about; asymmetries should exist only where the artefact
  genuinely differs (e.g., diff anchoring), not where the concept applies
  equally.
- **Shift-left detection of testing gaps** — missing test provisions,
  inadequate edge-case coverage, and poor mock strategy are cheaper to fix at
  the plan stage than after code is written.
- **Single-concern lenses** — following the precedent set by ADR-0011
  (standalone performance), ADR-0013 (standalone safety), and ADR-0014
  (standalone correctness), each lens should own one evaluative concern
  rather than bundling related-but-distinct concerns.
- **Clean boundary between testability and testing strategy** —
  *design-for-testability* (a code property) and *testing strategy* (a
  test-plan property) are distinct concerns that should be assessed by
  distinct lenses.
- **Analogy to the security lens** — security reviews design in plans and code
  in PRs; test coverage should follow the same pattern rather than being
  treated as uniquely PR-only.

## Considered Options

1. **Status quo — code-quality-lens owns testability and testing strategy; no
   test-coverage-lens.** A single lens covers all testing-related concerns for
   both plan and PR reviews. Avoids adding a lens to the catalogue but keeps
   a single lens responsible for both design-for-testability and testing
   strategy, without distinguishing the two.

2. **Test-coverage-lens scoped to PR review only.** The lens exists as a
   standalone skill, but only `review-pr` selects it; `review-plan` continues
   to route testing concerns through code quality. Matches the earlier
   PR-only reasoning from alignment research but produces catalogue asymmetry
   and denies plan reviewers the shift-left detection benefit.

3. **Standalone test-coverage-lens shared by both orchestrators.** One lens
   skill (`skills/review/lenses/test-coverage-lens/SKILL.md`) carries guidance
   for coverage adequacy, assertion quality, and test architecture — the same
   knowledge applies whether the subject is planned testing provisions (plan
   context) or actual test code (PR context). Both `review-plan` and
   `review-pr` select it from the shared catalogue. `code-quality-lens`
   retains testability-as-design only, with a reciprocal boundary note
   pointing to `test-coverage-lens`.

## Decision

We will maintain a standalone `test-coverage-lens` as a first-class entry in
the shared review lens catalogue, selectable by both `review-plan` and
`review-pr` orchestrators. The lens owns testing strategy, coverage adequacy,
assertion quality, and test architecture — applied to planned testing
provisions when invoked from plan review and to actual test code when invoked
from PR review.

We will keep `code-quality-lens` focused on testability as a *code design
property* (dependency injection, interface abstractions, independent
testability) and remove testing-strategy responsibilities from it. Both lens
skills will carry reciprocal boundary notes so the split is discoverable from
either side.

This ADR retrospectively documents a split already implemented in the lens
skills and both orchestrators; it captures the rationale so future lens-
boundary work has a durable reference. It also revises the earlier
alignment-research conclusion that test coverage was PR-only; that reasoning
conflated test code with testing strategy, and the latter is evaluable from
a plan's prose alone.

## Consequences

### Positive

- **Shift-left detection of testing gaps.** Missing test provisions,
  inadequate edge-case coverage, and fragile mock strategy surface at the plan
  stage, before code is written and rework is expensive.
- **Symmetric catalogue across plan and PR review.** Both orchestrators draw
  from the same lens set; asymmetries are confined to output format and
  anchoring — not to which concerns are evaluated.
- **Single-concern lenses.** `code-quality-lens` focuses on maintainability
  and design; `test-coverage-lens` focuses on test strategy and effectiveness.
  Each lens is easier to author, tune, and critique.
- **Clean, discoverable boundary.** Reciprocal boundary notes in both lens
  SKILL.md files make the testability-vs-testing-strategy split explicit,
  reducing the chance of overlapping or contradictory findings.
- **Consistent with prior lens-split ADRs.** Follows the precedent of ADR-0011
  (standalone performance), ADR-0013 (standalone safety), and ADR-0014
  (standalone correctness), reinforcing a coherent lens-design philosophy.

### Negative

- **More lens-boundary maintenance.** The code-quality / test-coverage seam
  must be policed as both lenses evolve; responsibilities can drift back
  toward overlap without vigilance.
- **Lens catalogue grows.** One more lens for orchestrators to consider during
  selection, slightly increasing selection-phase reasoning cost.
- **Single lens, two subjects.** The shared lens must accommodate both
  planned testing provisions and actual test code; authors of the lens need
  to keep guidance applicable to both contexts without becoming vague.

### Neutral

- **Historical decision reframed, not reversed.** The earlier PR-only
  conclusion targeted test *code*; this ADR makes testing *strategy* an
  explicit in-scope concern for plan review. The original reasoning remains
  correct within its narrower scope.
- **No change to the generic reviewer agent or output contract.** Lens
  injection (ADR-0005) and the structured output schema (ADR-0006) already
  accommodate a shared lens across orchestrators.

## References

- `meta/work/0012-test-coverage-as-plan-review-lens.md` — Source work item for
  this ADR
- `meta/decisions/ADR-0002-three-layer-review-architecture.md` — Three-layer
  review system (orchestrator / specialist / knowledge)
- `meta/decisions/ADR-0004-lens-catalogue-expansion-with-bounded-selection-and-core-set.md`
  — Shared lens catalogue and core-set semantics
- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md`
  — Runtime lens injection that enables a single lens skill to serve both
  orchestrators
- `meta/decisions/ADR-0006-structured-agent-output-contract-with-context-specific-schemas.md`
  — Output schema the test-coverage lens emits under
- `meta/decisions/ADR-0011-standalone-performance-lens.md` — Precedent for
  splitting a concern out of code quality
- `meta/decisions/ADR-0013-standalone-safety-lens.md` — Most recent sibling
  lens-split ADR establishing the same extraction pattern
- `meta/decisions/ADR-0014-standalone-correctness-lens.md` — Precedent for a
  standalone lens alongside test coverage and code quality
- `meta/plans/2026-02-22-review-plan-alignment.md` — Implementation plan that
  extracted testing strategy from `plan-code-quality-reviewer`
- `meta/research/2026-02-22-review-plan-pr-alignment.md` — Earlier alignment
  research whose PR-only conclusion this ADR reframes
- `skills/review/lenses/test-coverage-lens/SKILL.md` — Current lens skill
- `skills/review/lenses/code-quality-lens/SKILL.md` — Reciprocal
  boundary-note location
