---
type: work-item
id: "0138"
title: "Static-Analysis-Driven Refactoring Workflow"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: epic
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0138: Static-Analysis-Driven Refactoring Workflow

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Consider adding a refactoring workflow driven by static analyses and code
quality metrics, so refactoring opportunities are surfaced and prioritised from
objective signals.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Framed as a consideration; captured
as an epic to hold the workflow's design and child deliverables.

## Requirements

- Define a refactoring workflow that consumes static-analysis output and
  code-quality metrics.
- Surface, prioritise, and (optionally) drive refactoring actions from those
  signals.

## Acceptance Criteria

- [ ] A refactoring workflow exists that uses static analyses / code-quality
      metrics to identify and prioritise refactoring opportunities.

## Open Questions

- Which static-analysis tools / metrics feed the workflow?
- Does the workflow propose, plan, or apply refactorings?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: repo-wide linting/formatting/static-analysis work.

## Assumptions

- Builds on existing per-language linting / static-analysis tooling in the repo.

## Technical Notes

- Could integrate with existing check tasks across the four toolchains.

## Drafting Notes

- Treated as an epic per the user's instruction; the source phrased it as a
  "consider" item, so scope and whether to proceed remain open.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
