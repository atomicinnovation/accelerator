---
type: "work-item"
id: "0284"
title: "Topic-Research Set Detail Page"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
tags: ["research", "visualiser"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-868"
---

# 0284: Topic-Research Set Detail Page

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

A visualiser-only set-level detail page that represents a whole research set —
the brief, the outline, its findings, the synthesis, and any reports — and
navigates into each sub-document, replacing the flat shared `LibraryDocView` for
topic-research entries. It reads `manifest.md`'s `primary` pointer to choose the
default sub-document. This is the epic's designated descope candidate if it runs
long, since the shared flat view still renders each sub-document.

## Context

The page builds on the umbrella doc type + nested indexing (0278) and the
round/focus-area structure (0279). A Claude Design prototype of the visualiser
already exists but does not yet cover topic-research, so extending it is in-slice
work: prototype the set-level page there first, then implement against the
umbrella doc type and nested indexing.

## Requirements

- Serve a set's sibling documents to the detail page — a new set endpoint or an
  augmented `/api/docs/{path}` detail response (see Open Questions).
- Read `manifest.md`'s `primary` pointer to choose the default sub-document
  (brief before a synthesis exists, synthesis after).
- A topic-research library entry opens the set-level page linking to `brief.md`,
  `outline.md`, each finding, `synthesis.md`, and each report.
- Prototype the set-level page in Claude Design and review it before
  implementation begins.

## Acceptance Criteria

- [ ] The set-level page is prototyped in Claude Design and reviewed before
      implementation begins.
- [ ] A topic-research library entry opens a set-level detail page linking to
      `brief.md`, `outline.md`, each finding, `synthesis.md`, and each report,
      defaulting to the sub-document named by `manifest.md`'s `primary`.

## Open Questions

- Set-document API shape (planning-time): does the detail page get a new endpoint
  that lists a research set's sub-documents, or does the existing
  `/api/docs/{path}` detail response get augmented to include siblings? Both are
  viable; decide during this slice's planning. This is the epic's one still-open
  question.

## Dependencies

- Blocked by: 0278 (umbrella doc type + nested indexing — recorded on 0278's
  `blocks`), 0279 (round/focus-area structure the page shows — recorded on
  0279's `blocks`).

## Assumptions

- Deferral leaves the other slices fully shippable, since the shared flat
  `LibraryDocView` still renders each sub-document.

## Technical Notes

- Unlike other doc types, `topic-research` needs this bespoke set-level detail
  page rather than the shared `LibraryDocView`.

## Drafting Notes

- This is the epic-level descope candidate — the only slice whose deferral leaves
  the others fully shippable.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 6)
- Parent epic: 0121
