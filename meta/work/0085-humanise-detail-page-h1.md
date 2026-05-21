---
work_item_id: "0085"
title: "Humanise Detail-Page H1 Across All Doc Kinds"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: low
parent: ""
tags: [design, frontend, detail-page]
---

# 0085: Humanise Detail-Page H1 Across All Doc Kinds

**Type**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Apply a single canonical rule for how the detail-page H1 is derived,
producing a humanised title on every doc kind — including
work-item-review, plan-review, and validation pages, which currently
render the raw slug.

## Context

The current app surfaces a humanised `entry.title` on plan, research,
decision, design-inventory, design-gap, and work-item detail pages, but
renders the raw slug as H1 on work-item-review, plan-review, and
validation pages (the `0042-templates-view-redesign-review-1` form).
The prototype consistently uses a humanised H1 across all twelve doc
kinds.

## Requirements

- Define a single derivation rule for the detail-page H1 covering all
  doc kinds:
  - Prefer `frontmatter.title` when present.
  - Fall back to a humanised form of the slug (split on hyphens,
    title-cased, with numeric ID prefixes kept as-is or formatted).
- Apply the rule uniformly to every detail-page route, including
  work-item-review, plan-review, and validation.

## Acceptance Criteria

- [ ] H1 on every detail-page route renders a humanised title; no doc
  kind renders the raw slug as H1.
- [ ] The derivation rule is documented in the relevant component or a
  shared helper.
- [ ] Existing tests assert humanised H1 output for at least one
  example per doc kind.

## Open Questions

- For review/validation pages, should the H1 include the parent
  artefact's title (e.g. "Review 1 of 0042 Templates View Redesign"),
  or only the review's own title?

## Dependencies

- Blocked by: 0041 (Page wrapper standardised across detail routes).
- Blocks: none.

## Assumptions

- Frontmatter `title` is reliably populated on existing review and
  validation documents; if missing, the slug-humanisation fallback
  handles it.

## Technical Notes

- The current humanisation lives per-route; consolidating into a
  shared helper is the natural shape.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0041
