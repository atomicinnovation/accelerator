---
work_item_id: "0044"
title: "Spike: Confirm List-Screen Scope Decisions"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: spike
status: draft
priority: high
parent: ""
tags: [design, spike, scope, kanban, lifecycle]
---

# 0044: Spike: Confirm List-Screen Scope Decisions

**Type**: Spike
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Confirm with stakeholders the prototype's list-screen scope decisions: whether the lifecycle-index sort-mode reduction and filter-input removal are intentional simplifications, and whether the prototype's three-column kanban board is illustrative versus prescriptive (since ADR-0024 mandates configurable columns).

## Context

The current app's lifecycle-index sort affordance is a three-mode toggle (Recent / Oldest / Completeness) plus a free-text filter input. The prototype's lifecycle-index uses a two-segment pill control (`Updated` / `Completeness`) and no filter input — i.e. one fewer sort mode and no filter at all. The current filter is the only way to narrow large lifecycle lists by slug substring.

The current app's kanban columns are derived dynamically from the server-side `kanban-config` endpoint (configurable per-workspace), while the prototype's `kanban-view` shows a fixed three-column board (Todo / In progress / Done) with a `live` chip and `N total` count in the page subtitle. ADR-0024 *Configurable Kanban Column Set* mandates configurable columns, so the prototype's three-column board cannot be taken as prescriptive — but the gap analysis still requires confirmation that we preserve dynamic columns while adopting the prototype's `live` / `N total` page-subtitle treatment.

Reference screenshot: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/kanban-view.png`.

## Requirements

This spike must produce confirmed answers to the following questions:

1. **Lifecycle-index sort-mode reduction**: Is reducing from three sort modes (Recent / Oldest / Completeness) to two (Updated / Completeness) an intentional simplification?
2. **Lifecycle-index filter-input removal**: Is removing the free-text filter input intentional? If yes, what alternative is provided for narrowing large lifecycle lists?
3. **Kanban scope**: Confirm that the prototype's three-column board is illustrative — the redesign retains dynamic columns per ADR-0024, while adopting the prototype's `live` chip and `N total` page-subtitle treatment.

Time-box: ~0.5 day to gather stakeholder confirmation and document outcomes.

## Acceptance Criteria

- [ ] Each of the three questions above has a written, stakeholder-confirmed answer.
- [ ] Outcomes are captured either in updates to the gap-analysis document or as ADRs.
- [ ] Where the answer affects an existing work item (e.g. lifecycle-index control redesign), the affected item is updated or a new follow-up is created.

## Open Questions

- Is there appetite to extend ADR-0024 (or write a new ADR) to cover the kanban page-subtitle treatment if it is non-trivial?

## Dependencies

- Blocked by: none.
- Blocks: any future "lifecycle-index control redesign" or "kanban subtitle" work items.

## Assumptions

- ADR-0024's configurable-column requirement is non-negotiable; the spike is confirming the scope of the *visual* changes around the column set, not the column set itself.

## Technical Notes

- This is a research-only spike; the technical answer for the kanban question is already constrained by ADR-0024.

## Drafting Notes

- Bundled three "confirm with stakeholders" questions into one spike because they all concern list-screen scope decisions surfaced together in the gap analysis.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- ADR-0024 Configurable Kanban Column Set
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/kanban-view.png`
