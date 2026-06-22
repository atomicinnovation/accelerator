---
type: work-item
id: "0135"
title: "Comments Against Artefacts"
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

# 0135: Comments Against Artefacts

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Allow comments to be made against artefacts, so users can annotate and discuss
artefacts in place.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The source framed this as an
investigation ("Investigate how to allow comments against artefacts"); recorded
as a story per the user's instruction.

## Requirements

- Allow users to add comments associated with an artefact.
- Persist and display comments alongside the artefact.

## Acceptance Criteria

- [ ] Given an artefact, when a user adds a comment, then the comment is
      persisted and shown against that artefact.

## Open Questions

- Where do comments live (in the artefact file, a sidecar, or a store)?
- Are comments anchored to specific locations within an artefact or to the
  artefact as a whole?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: track artefact changes over time; retain edit deltas for artefacts.

## Assumptions

- Comments are a visualiser-facing collaboration feature.

## Technical Notes

- None identified yet.

## Drafting Notes

- Recorded as a story per the user's instruction despite the "investigate"
  phrasing; persistence model is left as an open question.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
