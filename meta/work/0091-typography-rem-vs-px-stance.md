---
work_item_id: "0091"
title: "Typography rem-vs-px stance review"
date: "2026-05-23T16:30:00+00:00"
author: Toby Clemson
type: work-item
kind: task
status: draft
priority: low
parent: ""
tags: [design, frontend, tokens, typography, accessibility]
---

# 0091: Typography rem-vs-px stance review

**Type**: Spike
**Status**: Backlog
**Priority**: Low
**Author**: Toby Clemson

## Summary

Revisit ADR-0036's px-anchored stance for `--size-*` tokens. The 0075
migration anchors every typography token to px values for token-value
determinism, trading user-controllable root-font-size scaling.
Investigate whether the accessibility trade-off matters in practice
and whether heading-tier tokens (or the entire `--size-*` family)
should be reintroduced as rem.

## Context

Created alongside the 0075 migration plan as the durable tracker for
the accessibility regression that ADR-0036 documents as a known
consequence. Specifically:

- The `MarkdownRenderer` H1 migration (Phase 2 of 0075) changes
  `font-size: 1.75rem` to `var(--size-h3)` (`28px`). At default
  browser font-size this is computed-identical, but a user who
  customises browser default font-size for accessibility loses
  font-size scaling for the H1.
- The full `--size-*` family (post-0075) is px-anchored. The same
  trade-off applies to every consumer.
- Browser-level zoom is unaffected; only font-size-only scaling is
  lost.

ADR-0036 §Decision documents the px-anchored stance as deliberate,
and §Consequences flags the accessibility trade-off as the principal
known cost. This work item exists so the trade-off is not "shipped
and forgotten".

## Acceptance Criteria

- [ ] **AC1.** Investigate real-world impact: is there user data,
  internal feedback, or an external accessibility audit signal that
  the px-anchored stance affects users? Capture findings in a
  research artefact under `meta/research/`.
- [ ] **AC2.** Decide one of:
  - **Keep px-anchored** — document the decision in a new ADR
    (supersession or amendment of ADR-0036 §Decision) with the
    user-impact evidence from AC1.
  - **Reintroduce rem for headings** — propose a token-shape change
    (e.g. `--size-h*` tokens become rem; `--size-body` and below stay
    px) in a new ADR, with a migration plan as a sibling work item.
  - **Reintroduce rem family-wide** — propose a full token-shape
    change in a new ADR, with a comprehensive migration plan.
- [ ] **AC3.** Spike output is either a new ADR (decision recorded)
  or a child story (decision deferred again with explicit reasoning).

## Decisions

(None yet — this is a spike.)

## Dependencies

- Blocked by: 0075 (typography size-scale consumption — must land
  first so the px-anchored stance is in production for evaluation).

## Assumptions

- The accessibility regression may not materially affect users; the
  spike should not presume the answer.
- Browser-level zoom remains the dominant scaling mechanism;
  font-size-only scaling is a niche accessibility preference.

## References

- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
  — origin of the px-anchored stance.
- `meta/work/0075-typography-size-scale-consumption.md` — landed the
  migration this work item evaluates.
- `meta/plans/2026-05-23-0075-typography-size-scale-consumption.md` —
  plan that introduces the px-anchored stance.
