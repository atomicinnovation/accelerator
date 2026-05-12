---
work_item_id: "0042"
title: "Templates View Redesign"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: low
parent: ""
tags: [design, frontend, templates]
---

# 0042: Templates View Redesign

**Type**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Redesign the templates index to surface per-tier presence inline on each row (with a "winning" highlight), and add a sha256 etag header to the rendered tier-preview pane on the detail screen.

## Context

The current app's templates view (`/library/templates/{name}`) renders three stacked tier panels with an active-tier marker plus the rendered template body. The prototype's `templates-view` adds an inline tier-presence row on the index list (each tier shown with a `present`-state colour and a "winning" highlight), expands a chosen template into a stacked tier card alongside a separate right pane that renders the template body with a `sha256-…` etag header.

Reference screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/templates-view.png`.

## Requirements

- On the templates index list, render an inline per-tier presence indicator for each row, showing each tier with a `present`-state colour and a "winning" highlight indicating which tier is active.
- On the templates detail screen, retain the stacked tier card but render the template body in a separate right-hand pane.
- Add a `sha256-…` etag header above the rendered tier-preview pane.
- Preserve the existing active-tier marker semantics — the redesign surfaces tier presence and winner more explicitly without changing the underlying tier model.

## Acceptance Criteria

- [ ] Given the user navigates to the templates index, when each row renders, then a per-tier presence row is visible inline showing each tier in a `present`-state colour with the winning tier highlighted.
- [ ] Given the user opens a template detail screen, when the page renders, then the stacked tier card is on the left and the rendered template body is in a separate right-hand pane.
- [ ] The right-hand pane shows a `sha256-…` etag header above the rendered template body.
- [ ] The etag value matches the sha256 of the resolved template content for the winning tier.

## Open Questions

- Where does the sha256 etag come from — is it computed client-side from the rendered content, or is it returned by the backend on the templates endpoint?
- Should the etag be selectable / copyable for use in cache-validation workflows?

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph for template-type icons), 0038 (Chip for tier-presence indicators), 0041 (Page wrapper consistency).
- Blocks: none.

## Assumptions

- The existing tier model (three stacked tiers with an active-tier marker) is preserved; only the presentation changes.

## Technical Notes

- The backend may need to expose sha256 etags on the templates endpoint if not already done.

## Drafting Notes

- Marked as low priority because it's a single screen redesign with no shared-component dependencies that other items need.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/templates-view.png`
- Related: 0033, 0037, 0038, 0041
