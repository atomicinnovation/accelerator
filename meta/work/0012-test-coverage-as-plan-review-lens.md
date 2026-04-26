---
title: "Test coverage as a plan review lens"
type: adr-creation-task
status: done
---

# ADR Ticket: Test coverage as a plan review lens

## Summary

In the context of aligning plan and PR review, reversing an earlier PR-only
decision, we decided to add a plan-test-coverage reviewer evaluating testing
*strategy* (pyramid, edge cases, mock strategy) to achieve earlier detection of
testing gaps before code is written, accepting boundary redraw with the code
quality lens.

## Context and Forces

- Test coverage was initially considered PR-only because it reviews actual test
  code
- Plans describe testing strategy but lack dedicated review of that strategy
- Catching testing gaps at the plan stage is cheaper than catching them after
  code is written
- The code quality lens previously owned testability-as-design-quality
  (dependency injection, interface abstractions)
- A plan-test-coverage lens evaluates a different aspect: testing *strategy*
  rather than test *code*

## Decision Drivers

- Shift-left principle: catch testing gaps before code is written
- Testing strategy (pyramid planning, edge case identification, mock strategy)
  is evaluable from a plan alone
- Clear distinction between testability-as-design (code quality) and testing
  strategy (test coverage)

## Considered Options

1. **Keep test coverage PR-only** — Plans aren't reviewed for testing strategy.
   Misses early detection opportunity.
2. **Fold into plan code quality review** — Overloads code quality with strategy
   evaluation.
3. **Standalone plan-test-coverage reviewer** — Evaluates testing strategy in
   plans: test pyramid planning, edge case identification, test architecture
   approach, mock strategy.

## Decision

We will add a plan-test-coverage reviewer that evaluates testing *strategy* in
plans. The code quality lens retains testability-as-design-quality (dependency
injection, interface abstractions). The test coverage lens owns testing strategy
(pyramid, edge cases, mock strategy). This reverses the earlier PR-only
decision.

## Consequences

### Positive
- Testing gaps caught at plan stage before code is written
- Clean boundary: design for testability (code quality) vs testing strategy
  (test coverage)
- Plan reviews become more comprehensive

### Negative
- Boundary redraw between code quality and test coverage requires updating both
  lenses
- An additional agent for plan reviews

### Neutral
- The PR test coverage lens continues to review actual test code
- The plan test coverage lens reviews testing strategy described in prose

## Source References

- `meta/plans/2026-02-22-review-plan-alignment.md` — Addition of
  plan-test-coverage reviewer and boundary redraw
