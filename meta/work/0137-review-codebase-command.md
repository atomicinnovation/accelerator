---
type: work-item
id: "0137"
title: "/review-codebase Command"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-158
---

# 0137: /review-codebase Command

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add a `/review-codebase` command that reviews a codebase (as opposed to a diff
or PR) through the plugin's review lenses.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The plugin already has review skills
for PRs and plans; this extends reviewing to a whole codebase.

## Requirements

- Add a `/review-codebase` skill that runs a multi-lens review over a codebase.
- Define its scope (whole repo, a path, a component) and output.

## Acceptance Criteria

- [ ] Invoking `/review-codebase` produces a multi-lens review of the targeted
      codebase scope.

## Open Questions

- What scope does it accept (whole repo, directory, component)?
- Which lenses apply, and how does it relate to the existing PR/plan review
  skills?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: refactoring workflow driven by static analyses.

## Assumptions

- Reuses the existing generic reviewer agent and lens machinery.

## Technical Notes

- Should follow the established review-skill architecture (orchestrator +
  lens-specific reviewer agents).

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
