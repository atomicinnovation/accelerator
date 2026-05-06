---
work_item_id: "0038"
title: "Generic Chip Component"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, components]
---

# 0038: Generic Chip Component

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement a generic Chip component with five named variants (`--green`, `--indigo`, `--amber`, `--neutral`, `--sm`) so status pills can be reused across kanban cards, lifecycle cards, library tables, and templates indicators rather than being open-coded per surface.

## Context

The current app's chip vocabulary is limited to `FrontmatterChips` (definition-list pairs surfaced from a document's frontmatter) and a malformed-frontmatter banner. The prototype defines a generic `Chip` component with five named variants:

- `--green` for done/accepted
- `--indigo` for in progress / live
- `--amber` for approve-with-changes
- `--neutral`
- `--sm` (small size variant)

These are used across kanban cards, lifecycle cards, library tables, and templates tier indicators.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/kanban-view.png`, `library-decisions.png`, `library-type-view.png`, `lifecycle-cluster-detail.png`, `templates-view.png`.

## Requirements

- Implement a `Chip` component accepting a `variant` prop (`green` / `indigo` / `amber` / `neutral`) and a `size` prop (default / `sm`).
- Source variant colours from the new `--ac-*` token layer.
- Replace open-coded status pills across kanban cards, lifecycle cards, library tables, and templates indicators with the new Chip component.
- The existing `FrontmatterChips` component continues to surface frontmatter key/value pairs, but its inner chip rendering should consume the generic Chip variants where applicable.

## Acceptance Criteria

- [ ] Given a Chip is rendered with `variant="green"`, when it paints, then it uses the green semantic palette and is interpreted as "done/accepted".
- [ ] Each of the five variants (`green`, `indigo`, `amber`, `neutral`, `sm`) renders as specified in the prototype.
- [ ] Chip variant colours swap correctly between light and dark theme.
- [ ] `grep` for open-coded inline chip styles across kanban / lifecycle / library / templates surfaces returns no results once the migration is complete.

## Open Questions

- Are there status values in the current app (e.g. additional workflow states) that don't map cleanly onto the four colour variants and require new variants?
- Should Chip support an icon slot (e.g. for the green pulse on `live` indicators)?

## Dependencies

- Blocked by: 0033 (token system).
- Blocks: 0040 (kanban card enrichment uses Chip), 0041 (Library page header subtitle/count uses Chip), 0042 (Templates view tier presence uses Chip).

## Assumptions

- The five variants enumerated in the gap analysis are exhaustive for current needs.

## Technical Notes

- The prototype uses CSS class modifiers (`.ac-chip--green`, `.ac-chip--sm`); a React component with className composition is the natural shape.

## Drafting Notes

- Kept Chip as its own story rather than folding it into any one consumer because four distinct surfaces consume it.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/kanban-view.png`, `library-decisions.png`, `library-type-view.png`, `lifecycle-cluster-detail.png`, `templates-view.png`
- Related: 0033, 0040, 0041, 0042
