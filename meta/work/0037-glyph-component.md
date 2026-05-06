---
work_item_id: "0037"
title: "Glyph Component"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: ""
tags: [design, frontend, components]
---

# 0037: Glyph Component

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Implement a Glyph component that renders a per-doc-type icon at multiple sizes with per-doc-type fill colours, and thread it through every doc-type reference across the app — Sidebar nav, Page header eyebrows, kanban cards, lifecycle cards, timeline steps, and activity-feed rows.

## Context

The current app has no doc-type icon system — items are distinguished by text labels alone. The prototype defines a `Glyph` component that renders a square doc-type icon at multiple sizes (16/24/32px) with per-doc-type fill colours: red for Decision, orange for Research, blue for Plan, purple for Plan review, green for Validation, teal for PR, mauve for PR review, red-pin for Note, dark-red for Work item.

The prototype embeds Glyph inside nav items, page eyebrows, kanban cards, timeline steps, lifecycle cards, and activity items — making it a load-bearing shared component that downstream redesigns (Sidebar, Library page wrapper, kanban card enrichment, lifecycle hexchain, activity feed) all depend on.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `library-view.png`, `kanban-view.png`, `lifecycle-cluster-detail.png`.

## Requirements

- Implement a `Glyph` component accepting a `docType` prop and a `size` prop (16, 24, or 32 px).
- Implement icon assets and per-doc-type fill colours for at least the nine doc types enumerated in the prototype (Decision, Research, Plan, Plan review, Validation, PR, PR review, Note, Work item) — and any other doc types currently surfaced in the app's twelve list views.
- Source colours from the new `--ac-*` token layer rather than hard-coded hex values.
- Provide rendered exports / examples sufficient for downstream consumers to thread Glyph through Sidebar nav, Page header eyebrows, kanban cards, lifecycle cards, timeline steps, and activity-feed rows.

## Acceptance Criteria

- [ ] Given a Glyph is rendered with `docType="decision"` at any of the three supported sizes, when it paints, then a red square doc-type icon appears at the requested size.
- [ ] Glyph renders correctly for every doc-type currently surfaced in the app's twelve list views.
- [ ] Glyph fill colours resolve via `--ac-*` tokens and swap correctly between light and dark theme.
- [ ] Glyph is consumed correctly by Sidebar nav, page header eyebrows, kanban cards, lifecycle cards, timeline steps, and activity-feed rows (verified once those consumers are updated by their own work items).

## Open Questions

- Are there doc types in the current app not enumerated in the prototype's nine that need additional icons / colours assigned?
- Should Glyph support an `accessible-label` prop for screen readers, given the current app distinguishes items by text labels alone?

## Dependencies

- Blocked by: 0033 (token system, for `--ac-*` colour tokens).
- Blocks: 0036 (Sidebar nav, activity feed), 0040 (kanban card enrichment, lifecycle hexchain), 0041 (Library page header eyebrows), 0042 (Templates view).

## Assumptions

- The icon assets for each doc type can be derived from the prototype source or commissioned; the work item assumes asset creation is in scope.

## Technical Notes

- The prototype uses CSS classes like `.ac-glyph` with per-doc-type modifier classes; a React component wrapping that pattern is the obvious shape.

## Drafting Notes

- Kept Glyph as its own story rather than folding it into any one consumer because the gap analysis identifies six surfaces that consume it.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `library-view.png`, `kanban-view.png`, `lifecycle-cluster-detail.png`
- Related: 0033, 0036, 0040, 0041, 0042
