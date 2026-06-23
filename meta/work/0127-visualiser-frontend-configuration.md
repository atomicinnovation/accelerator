---
type: work-item
id: "0127"
title: "Visualiser Frontend Configuration"
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
external_id: PP-148
---

# 0127: Visualiser Frontend Configuration

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add configuration to the visualiser frontend so user- or project-level
preferences can influence how the visualiser renders and behaves.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The visualiser currently has limited
or no user-facing configuration surface in the frontend.

## Requirements

- Establish a configuration mechanism for the visualiser frontend.
- Determine which settings are configurable and how they are persisted /
  sourced.

## Acceptance Criteria

- [ ] The visualiser frontend exposes a configuration surface and applies the
      configured values at render time.

## Open Questions

- What specifically should be configurable (theme, default views, paths,
  feature toggles)?
- Where does configuration live — server-provided, browser-local, or sourced
  from the existing `.accelerator` config?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- "Configuration" refers to frontend-facing settings rather than build-time
  configuration.

## Technical Notes

- May intersect with the existing configuration system architecture and the
  server's role in serving config to the SPA.

## Drafting Notes

- Treated as an epic per the user's instruction; specific configurable settings
  to be enumerated during refinement.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
