---
type: work-item
id: "0128"
title: "Select jj Workspace / Git Worktree in Visualiser"
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
external_id: PP-149
---

# 0128: Select jj Workspace / Git Worktree in Visualiser

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Allow the user to select which jj workspace or git worktree the visualiser is
viewing, so a repo with multiple workspaces/worktrees can be navigated without
restarting against a different directory.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The repo uses jj workspaces (and
supports git worktrees); the visualiser currently binds to a single checkout.

## Requirements

- Detect available jj workspaces / git worktrees for the current repository.
- Provide a UI affordance to switch the active workspace/worktree the
  visualiser renders from.

## Acceptance Criteria

- [ ] Given a repo with multiple jj workspaces / git worktrees, when the user
      selects a different one, then the visualiser renders that checkout's
      meta directory.

## Open Questions

- Should switching change server-side state, or open the selected workspace in a
  new view?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- Workspace/worktree enumeration can build on existing VCS detection logic.

## Technical Notes

- Relates to the workspace/worktree boundary detection already present in the
  hooks/VCS layer.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
