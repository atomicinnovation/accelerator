---
id: "0096"
title: "Templates View Auto-Discovers Available Templates"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: in-progress
kind: story
priority: medium
tags: [visualiser, templates, frontend]
last_updated: "2026-06-02T12:11:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["work-item:0042", "work-item:0089"]
---

# 0096: Templates View Auto-Discovers Available Templates

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The templates view does not show the full set of available templates; the
displayed list is incomplete and drifts out of sync with reality. The view
should automatically stay up to date based on the contents of the
`templates/` directory, so newly added templates appear without manual
intervention.

## Context

The current templates view appears to rely on a list that is not derived
from the `templates/` directory, so it omits templates and requires manual
upkeep to stay current.

## Requirements

- The templates view enumerates templates from the `templates/` directory
  as its source of truth.
- Adding or removing a template file is reflected in the view without a
  separate, hand-maintained registration step.

## Acceptance Criteria

- [ ] Every template present in `templates/` appears in the templates view.
- [ ] A newly added template file appears in the view without editing a
  separate list.
- [ ] A removed template file no longer appears in the view.

## Open Questions

- Is discovery performed at build time (static generation over
  `templates/`) or at runtime?
- Does this depend on or land after 0042's templates view redesign?

## Dependencies

- Related: 0042 (templates view redesign), 0089 (templates preview
  whitespace fix), 0029 (template management subcommand surface).

## Drafting Notes

- Captured as a stub without interactive enrichment. Requirements,
  acceptance criteria, and the build-time-vs-runtime question need
  refinement before promoting from `draft` to `ready`.

## References

- Related: 0042, 0089, 0029
