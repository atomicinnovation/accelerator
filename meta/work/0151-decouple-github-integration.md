---
type: work-item
id: "0151"
title: "Decouple Collaboration Skills from GitHub Integration"
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

# 0151: Decouple Collaboration Skills from GitHub Integration

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move GitHub-specific interactions into an integration and decouple the
collaboration skills from GitHub, so collaboration skills can target other
providers through a pluggable integration layer.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Mirrors the integration pattern
already used for work-management trackers (Jira/Linear). Follows the
collaboration group rename.

## Requirements

- Extract GitHub-specific interactions into a dedicated integration.
- Refactor collaboration skills to depend on the integration abstraction rather
  than GitHub directly.

## Acceptance Criteria

- [ ] Collaboration skills operate through an integration abstraction, with
      GitHub-specific behaviour isolated behind it.

## Open Questions

- Which other providers should the integration abstraction anticipate?
- What is the integration contract / interface?

## Dependencies

- Blocked by: Rename GitHub skill group to collaboration.
- Blocks: None identified yet.

## Assumptions

- Follows the same integration pattern as the work-management trackers.

## Technical Notes

- Relates to the existing integrations architecture and work-management
  integration decisions.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
