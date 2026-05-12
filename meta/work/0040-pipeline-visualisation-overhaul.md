---
work_item_id: "0040"
title: "Pipeline Visualisation Overhaul"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, lifecycle, kanban]
---

# 0040: Pipeline Visualisation Overhaul

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Extend the eight-stage pipeline visualisation into three rendering modes — full hexchain, compact strip, and micro stagedots — and surface them appropriately on lifecycle cluster detail (hexchain header), lifecycle index (compact strip on cluster cards), and kanban (micro stagedots on cards). Enrich the WorkItemCard with stagedots, linked-artifact count, and namespaced IDs.

## Context

The current app's `PipelineDots` is a single component rendering an eight-dot completeness row inside lifecycle cluster cards. The prototype renders the eight-stage pipeline three different ways:

- A full `.ac-hexchain` (linked stage tiles with labels) on lifecycle cluster detail.
- A compact strip (`.ac-lcard__pipe`) on lifecycle cluster cards in the index.
- A micro `.ac-stagedots` row embedded in kanban cards.

The lifecycle cluster detail screen also gets a hexchain header strip *above* the existing vertical timeline of pipeline stages, sharing pipeline data with the per-stage cards below.

The current app's `WorkItemCard` (the draggable kanban card) shows the work-item ID, mtime, title, and an optional `frontmatter.type` field. The prototype's `.ac-kcard` adds an embedded `.ac-stagedots` micro-pipeline showing each card's lifecycle completeness, an `.ac-kcard__links` "N linked" relation count, and richer ID prefixing (e.g. `PROJ-NNNN`, `ENG-NNNN`, `META-NNNN`).

Reference screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/lifecycle-cluster-detail.png` (hexchain header), `kanban-view.png` (micro stagedots on cards), `main-light.png` / `library-view.png` (compact strip on lifecycle cards in the index).

## Requirements

- Refactor `PipelineDots` (or introduce a new `Pipeline` component) to support three rendering modes:
  - `mode="hexchain"` — linked stage tiles with labels.
  - `mode="compact"` — compact strip (`.ac-lcard__pipe`).
  - `mode="stagedots"` — micro stagedots row.
- All three modes share the same eight-stage pipeline data source.
- Lifecycle cluster detail screen (`/lifecycle/{slug}`) renders a hexchain header strip above the existing per-stage timeline, sharing pipeline data with the timeline cards below.
- Lifecycle cluster cards in the index render the compact strip mode.
- Kanban cards (`WorkItemCard`) embed the micro stagedots row.
- Extend `WorkItemCard` to add an `.ac-kcard__links` "N linked" relation count showing the number of linked artifacts.
- Render namespaced work-item IDs (e.g. `PROJ-NNNN`, `ENG-NNNN`, `META-NNNN`) on `WorkItemCard`, alongside the existing chrome (mtime, title, optional `frontmatter.type`).

## Acceptance Criteria

- [ ] Given the user opens `/lifecycle/{slug}`, when the page renders, then a hexchain header strip appears above the existing vertical timeline, sharing pipeline data with the timeline cards.
- [ ] Given the user opens the lifecycle index, when each cluster card renders, then the compact pipeline strip is visible on each card.
- [ ] Given the user opens the kanban view, when each WorkItemCard renders, then a micro stagedots row is embedded in the card.
- [ ] Given a WorkItemCard has linked artifacts, when the card renders, then a "N linked" relation count is displayed.
- [ ] Given a work item has a namespaced ID (e.g. `PROJ-0042`), when its WorkItemCard renders, then the namespaced ID is rendered in full.
- [ ] All three pipeline modes consume the same data source and update consistently when stage data changes.

## Open Questions

- What is the data source for the "N linked" relation count — is it derived from the existing related-artifacts query (Targets / Referenced by / Same lifecycle), or does it require a new endpoint?
- Are the namespaced ID prefixes (`PROJ`, `ENG`, `META`) configurable per workspace, or fixed?
- Should the hexchain header on lifecycle cluster detail be interactive (e.g. clicking a stage scrolls the timeline to it)?

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph component for kanban card eyebrows), 0038 (Chip component).
- Blocks: none.

## Assumptions

- The existing eight-stage pipeline data model is sufficient for all three rendering modes; no schema changes are required.
- The ID-namespacing mechanism aligns with the work-item id_pattern configuration (`work.id_pattern`).

## Technical Notes

- The current `PipelineDots` component is the natural place to start the refactor — extending it to a `mode` prop is less invasive than introducing a new component hierarchy.

## Drafting Notes

- The gap analysis describes pipeline 3-modes (Component Drift), WorkItemCard enrichment (Component Drift), and lifecycle hexchain header (Screen Drift) as separate items, but they all share the same pipeline component output and are coupled by data flow. Treated as one story.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/lifecycle-cluster-detail.png`, `kanban-view.png`, `main-light.png`, `library-view.png`
- Related: 0033, 0037, 0038
