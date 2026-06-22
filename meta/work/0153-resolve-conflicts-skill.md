---
type: work-item
id: "0153"
title: "/resolve-conflicts VCS Skill"
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
---

# 0153: /resolve-conflicts VCS Skill

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add a `/resolve-conflicts` skill to the VCS skill group to assist with resolving
merge/rebase conflicts.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The VCS skill group abstracts over
jj and git; conflict resolution is a common, fiddly operation.

## Requirements

- Add a `/resolve-conflicts` skill within the VCS skill group.
- Support the repository's VCS (jj and git) conflict-resolution workflows.

## Acceptance Criteria

- [ ] Invoking `/resolve-conflicts` guides or performs conflict resolution for
      the active VCS.

## Open Questions

- How interactive should resolution be (guided vs. automated)?
- jj and git conflict models differ — is parity required at launch?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- Built on the existing VCS abstraction layer.

## Technical Notes

- Relates to the VCS abstraction layer and VCS detection.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
