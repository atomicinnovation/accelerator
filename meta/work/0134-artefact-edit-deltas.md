---
type: work-item
id: "0134"
title: "Retain Edit Deltas for Artefacts"
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

# 0134: Retain Edit Deltas for Artefacts

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Investigate and implement retention of edit deltas for artefacts, so granular
changes can be preserved and later inspected.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The source framed this as an
investigation ("Investigate how to retain edit deltas for artefacts"); recorded
as a story per the user's instruction. Closely related to artefact change
tracking.

## Requirements

- Determine how to capture and store edit deltas for artefacts.
- Retain deltas in a form that supports later inspection / reconstruction.

## Acceptance Criteria

- [ ] Edit deltas for an artefact are retained and can be retrieved to
      reconstruct or inspect prior states.

## Open Questions

- Granularity and storage format of deltas?
- Relationship to VCS history versus a separate delta store?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: track artefact changes over time; comments against artefacts.

## Assumptions

- Deltas are finer-grained than VCS commit history (e.g. per-edit).

## Technical Notes

- None identified yet.

## Drafting Notes

- Recorded as a story per the user's instruction despite the "investigate"
  phrasing; the feasibility dimension is preserved under Open Questions.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
