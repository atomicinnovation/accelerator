---
type: "work-item"
id: "0238"
title: "Single Authority For Lifecycle States"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["config", "templates", "lifecycle"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-768"
---

# 0238: Single Authority For Lifecycle States

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Make either configuration or templates the single authority for the valid
lifecycle states of meta artefacts, so status values are defined in one
place rather than duplicated.

## Context

Captured in the further-ideas backlog. Valid statuses for meta artefacts
are implied by both templates and configuration; the two can drift.

## Requirements

- Choose one authoritative source (config or templates) for valid lifecycle
  states.
- Have all state validation read from that single source.

## Acceptance Criteria

- [ ] Valid lifecycle states are declared in exactly one place.
- [ ] Producers and validators read valid states from that source.

## Open Questions

- Which is authoritative — configuration or templates?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
