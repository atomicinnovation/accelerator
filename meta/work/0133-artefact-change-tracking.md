---
type: work-item
id: "0133"
title: "Track Artefact Changes Over Time"
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

# 0133: Track Artefact Changes Over Time

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Track changes to artefacts so the visualiser can highlight change as it happens
and let users scan through an artefact's history.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The source framed this as a spike ("
Spike how to track changes to artefacts ..."); recorded here as a story per the
user's instruction, so feasibility questions are carried as open questions.

## Requirements

- Capture or derive a history of changes to each artefact.
- Surface changes in the visualiser: highlight live/recent change and allow
  scanning through artefact history.

## Acceptance Criteria

- [ ] Given an artefact that has changed, when viewed in the visualiser, then
      recent changes are highlighted and prior versions can be browsed.

## Open Questions

- Is history sourced from VCS (jj/git) history or tracked independently?
- What granularity — per-save deltas, per-commit, or both? (See related edit
  deltas work item.)

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: edit deltas for artefacts; comments against artefacts.

## Assumptions

- "Change as it is happening" implies near-real-time reflection of edits in the
  visualiser.

## Technical Notes

- May build on VCS history rather than a bespoke change store.

## Drafting Notes

- Recorded as a story (not a spike) per the user's instruction, despite the
  source phrasing; the underlying feasibility questions are preserved under Open
  Questions so refinement can decide whether a spike is still warranted first.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
