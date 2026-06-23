---
type: work-item
id: "0149"
title: "Normalise Statuses Across Artefacts"
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
external_id: PP-170
---

# 0149: Normalise Statuses Across Artefacts

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Normalise statuses and status formats across artefact types, so status
vocabularies and representations are consistent.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Different artefact types (work
items, reviews, plans, ADRs) use varying status vocabularies and formats.

## Requirements

- Define normalised status vocabularies and formats across artefact types.
- Reconcile existing artefacts / templates / skills with the normalised scheme.

## Acceptance Criteria

- [ ] Artefact statuses follow a consistent, documented vocabulary and format
      across artefact types.

## Open Questions

- Should all artefact types share one status vocabulary, or per-type vocabularies
  with a consistent format?
- Is a migration of existing artefacts required?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: prompt skills to update artefact statuses on completion.

## Assumptions

- Normalisation spans frontmatter status fields and their rendered forms.

## Technical Notes

- May require a corpus migration and template updates.

## Drafting Notes

- None beyond faithful restatement of the source line; whether vocabularies are
  unified or per-type is left open.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
