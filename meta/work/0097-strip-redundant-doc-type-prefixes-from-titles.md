---
id: "0097"
title: "Strip Redundant Document-Type Prefixes From Artifact Titles"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: bug
priority: medium
tags: [visualiser, artifacts, titles, conventions]
last_updated: "2026-06-02T12:11:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
type: work-item
relates_to: ["work-item:0085"]
---

# 0097: Strip Redundant Document-Type Prefixes From Artifact Titles

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Certain artifact titles are prefixed with the document type (e.g.
`"Research: …"`), which is redundant because the document type and the
work-item number are already conveyed by context and metadata. Titles
should describe the content only — not the document type or the work-item
number.

## Context

The document type is already surfaced through type metadata and per-doc-type
glyphs, so repeating it in the title string is duplicative. Embedding the
work-item number in the title is likewise redundant. This concern is about
the title *content* itself, distinct from 0085, which standardises how the
detail-page H1 is *derived* but does not address redundant prefixes within
the title.

## Requirements

- Artifact titles describe their subject matter alone; the document-type
  prefix (e.g. `"Research: "`) is removed from title content.
- Titles do not embed the work-item number.
- Document type continues to be conveyed via existing type indicators
  (metadata and glyph), not the title string.

## Acceptance Criteria

- [ ] Affected artifact titles no longer begin with a `"<DocType>: "`
  prefix.
- [ ] Titles do not embed the work-item number.
- [ ] Document type remains discernible from existing type indicators
  (metadata/glyph) after the prefix is removed.

## Open Questions

- Should this be fixed at authoring time (templates/skills stop emitting the
  prefix), at render time (visualiser strips a known prefix), or both?
- Does the fix apply retroactively to existing artifacts, or only to newly
  created ones?

## Dependencies

- Related: 0085 (humanise detail-page H1 across all doc kinds).

## Drafting Notes

- Captured as a stub without interactive enrichment. The
  authoring-time-vs-render-time decision and retroactive scope need
  resolving before promoting from `draft` to `ready`.
- Kept separate from 0085 deliberately: 0085 derives the H1, this addresses
  redundant content within the title itself.

## References

- Related: 0085
