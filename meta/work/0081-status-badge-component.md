---
work_item_id: "0081"
title: "StatusBadge — Map Both Status and Verdict to Chip Tone"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, components, chips]
---

# 0081: StatusBadge — Map Both Status and Verdict to Chip Tone

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce a `StatusBadge` component (or extend `FrontmatterChips`) that
maps both `status` and `verdict` frontmatter keys to a coloured chip
tone using a shared status-tone map, so review and validation pages
signal their outcome at a glance.

## Context

The prototype's `StatusBadge` maps both `status` and `verdict`
frontmatter keys to a coloured chip tone (`Accepted` → green,
`Draft` → amber, `pass` → green, and so on). The current app's
`FrontmatterChips` only colours the `status` key via
`statusToChipVariant` and renders the `verdict` key as neutral — the
validation detail page renders the `pass` verdict as a neutral chip with
no semantic colour.

## Requirements

- Extend the chip-tone mapping so both `status` and `verdict` values
  resolve to the same coloured chip variants.
- Canonical `verdict` value tones: `pass` → green, `fail` → red,
  `approve-with-changes` → amber, neutral / unknown → neutral.
- Apply across plan-review, work-item-review, and validation detail
  pages where `verdict` appears.
- Build on 0038's `Chip` primitive — no new variant required beyond
  what 0038 already ships.

## Acceptance Criteria

- [ ] `statusToChipVariant` (or an equivalent helper) accepts both
  `status` and `verdict` values and returns the matching `Chip` variant.
- [ ] On plan-review, work-item-review, and validation detail pages,
  the `verdict` chip renders with the appropriate coloured variant
  (no longer neutral).
- [ ] Mapping is exhaustive over the canonical verdict values; an
  unmapped value falls back to neutral.

## Open Questions

- Are the canonical verdict values `pass` / `fail` /
  `approve-with-changes` exhaustive, or do other review domains use
  different verdict vocabularies?

## Dependencies

- Blocked by: 0038 (Chip primitive shipped).
- Blocks: none.

## Assumptions

- The same chip variants from 0038 cover all verdict tones — no new
  variant is needed.

## Technical Notes

- `statusToChipVariant` lives in `frontend/src/api/status-variant.ts`.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0038
