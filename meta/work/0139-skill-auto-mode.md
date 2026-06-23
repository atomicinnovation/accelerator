---
type: work-item
id: "0139"
title: "Auto-Mode for Relevant Skills"
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
external_id: PP-160
---

# 0139: Auto-Mode for Relevant Skills

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add an auto-mode to all relevant skills, allowing them to run with reduced
interactivity / automated decisions where appropriate.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Many skills are currently
interactive; an auto-mode would let them proceed without per-step prompting.

## Requirements

- Define a consistent auto-mode contract across relevant skills.
- Identify which skills should support auto-mode and how it changes their
  behaviour.

## Acceptance Criteria

- [ ] Relevant skills support an auto-mode that runs with reduced interactivity
      following a consistent convention.

## Open Questions

- Which skills are "relevant" for auto-mode?
- How is auto-mode invoked (flag, config, argument)?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- Auto-mode is an opt-in alternative to the default interactive flow.

## Technical Notes

- Should be a shared convention rather than per-skill bespoke handling.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
