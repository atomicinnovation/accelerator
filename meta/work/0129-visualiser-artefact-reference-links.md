---
type: work-item
id: "0129"
title: "Render Artefact References as Links in Visualiser"
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
external_id: PP-150
---

# 0129: Render Artefact References as Links in Visualiser

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Convert all artefact references — in both frontmatter and markdown body — into
navigable links before the visualiser renders them, so cross-references between
artefacts become clickable.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Artefacts use typed-linkage
references (e.g. `work-item:NNNN`, `plan:NNNN`) and inline references that
currently render as plain text.

## Requirements

- Resolve artefact references found in frontmatter (typed-linkage slots) into
  links to the referenced artefact.
- Resolve artefact references appearing in markdown body content into links.

## Acceptance Criteria

- [ ] Given an artefact that references another artefact in frontmatter or body,
      when rendered in the visualiser, then the reference appears as a working
      link to the target artefact.

## Open Questions

- How should unresolved / dangling references be rendered (greyed out, error
  styling)?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- Reference formats follow the established typed-linkage vocabulary.

## Technical Notes

- Link resolution likely happens during the render/transform stage in the
  server or frontend markdown pipeline.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
