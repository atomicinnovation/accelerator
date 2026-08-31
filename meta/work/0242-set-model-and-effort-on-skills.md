---
type: "work-item"
id: "0242"
title: "Set Model And Effort On Skills"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["skills", "config"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0242: Set Model And Effort On Skills

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Allow a skill to declare the model and/or reasoning effort it should run
with, so heavier skills can request a more capable model and lighter ones a
cheaper one.

## Context

Captured in the further-ideas backlog. Skills currently run under whatever
model the session uses; per-skill tuning would match cost to task.

## Requirements

- A skill can declare a preferred model and/or effort level.
- Invocation honours the declared model/effort where the harness allows.

## Acceptance Criteria

- [ ] A skill declaring a model/effort runs under that model/effort.
- [ ] A skill declaring neither behaves as today.

## Open Questions

- Is model/effort selection honourable from within a skill, or must it be
  set by the harness at spawn time?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
