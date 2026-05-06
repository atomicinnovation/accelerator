---
work_item_id: "0041"
title: "Library Page Wrapper and Overview Hub"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, library]
---

# 0041: Library Page Wrapper and Overview Hub

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce a structured Page wrapper (eyebrow row, h1, subtitle/count, right-aligned actions including sort + filter pills) for library type views, and add a Library overview hub at `/library` that replaces the current redirect with phase-grouped doc-type cards showing icon, label, count, and latest-document preview.

## Context

The current app's `LibraryTypeView` renders a sortable table where sort is driven by clicking column headers, and there is no filter UI. The prototype renders the same table with two extra page-level controls — an `.ac-sort-btn` pill ("Recently modified") and a `.ac-filter` pill — plus a structured `Page` wrapper with eyebrow row, `<h1>`, subtitle/count, and right-aligned actions.

The current app's `/library` route redirects straight to `/library/decisions`, so there is no library-overview hub. The prototype provides a `library-overview` screen that groups the twelve doc types by lifecycle phase (Define / Build / Ship / Remember), with each group containing per-doc-type cards that show an icon, the doc-type label, the doc count, and a "latest · {title}" preview.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view.png` (overview hub), `library-decisions.png`, `library-type-view.png` (page wrapper on type views).

## Requirements

### Page wrapper

- Implement a `Page` wrapper component providing an eyebrow row (small uppercase label with optional Glyph), `<h1>`, subtitle/count line, and a right-aligned actions slot.
- Apply the `Page` wrapper to `LibraryTypeView`, replacing the bare `<h1>`.
- In the actions slot, render a sort-button pill (e.g. "Recently modified") and a filter-button pill, providing an alternative entry point to sorting and a new filter affordance.
- Preserve the existing column-header click sort as a secondary entry point (or replace it — this is an open question).

### Overview hub

- Replace the `/library` → `/library/decisions` redirect with a new Library overview screen.
- Group the twelve doc types by lifecycle phase (Define / Build / Ship / Remember).
- Render each doc type as a drillable card showing icon (Glyph), label, count, and "latest · {title}" preview line.

## Acceptance Criteria

- [ ] Given the user navigates to `/library`, when the page renders, then the new overview hub appears (not a redirect).
- [ ] The overview hub groups the twelve doc types by lifecycle phase (Define / Build / Ship / Remember).
- [ ] Each doc-type card on the overview hub shows the doc-type Glyph, label, current document count, and the title of the most recently modified document.
- [ ] Clicking a doc-type card navigates to that doc type's list view.
- [ ] Given the user navigates to a library type view, when the page renders, then the new Page wrapper provides an eyebrow row, h1, subtitle/count line, and right-aligned actions.
- [ ] The library type view exposes a sort-button pill and a filter-button pill in the actions area.
- [ ] Clicking the sort pill opens a sort-mode chooser; clicking the filter pill opens a filter input or chooser.

## Open Questions

- Should column-header click-sort be retained as a secondary entry point, or should sorting be driven exclusively from the new sort pill?
- What is the filter UX — a dropdown, a free-text input, or a faceted filter panel? The prototype shows only the pill, not the activated state.
- What query backs the "latest · {title}" preview on each overview card — is it derived from the existing per-type list endpoint, or does it need a new aggregation endpoint?

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph), 0038 (Chip — used in subtitle/count).
- Blocks: 0042 (Templates view also uses the Page wrapper).

## Assumptions

- The overview hub's doc-type cards drill into existing list views; no new detail behaviour is introduced.

## Technical Notes

- Consider whether `Page` should be a generic wrapper consumed by other screens (lifecycle index, kanban) or is library-specific. The gap analysis describes it as page-header structure broadly applicable.

## Drafting Notes

- Treated Page wrapper + overview hub as one story because both touch `/library` routing and the page chrome, and the overview hub is the natural debut of the Page wrapper.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view.png`, `library-decisions.png`, `library-type-view.png`
- Related: 0033, 0037, 0038, 0042
