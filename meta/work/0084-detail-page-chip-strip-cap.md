---
work_item_id: "0084"
title: "Detail-Page Chip Strip Cap (Max Four Chips)"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, detail-page, chips]
---

# 0084: Detail-Page Chip Strip Cap (Max Four Chips)

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Rebalance the chip strip in the detail-page subtitle slot so it
displays a hard-limited set of at most four chips — status, verdict,
date, author — and moves all other frontmatter content to the
frontmatter table introduced by 0078.

## Context

The current app's chip strip in the subtitle slot includes every
frontmatter key for some doc kinds (work-item: id, type, status,
priority, tags, title, author, date) and is empty for others (notes
render with no chips, leaving H1 sitting directly over the divider). On
design-inventory pages the chips duplicate author and timestamp values
(`last_updated` mirrors `date`, `last_updated_by` mirrors `author`),
and on design-gap and research pages chips carry full file paths or
40-character git hashes that dominate the strip.

The prototype's chip row is hard-limited to status + verdict + date +
author (max four chips). Other frontmatter content lives in the
frontmatter table delivered by 0078.

## Requirements

- Enforce a maximum chip set of four: status, verdict, date, author.
- Render only the chips whose underlying frontmatter key exists and is
  non-null; the strip can render fewer than four but never more.
- Remove every other key currently surfaced in the chip strip
  (id, type, priority, tags, title, slug, paths, hashes, mirrors).
- The frontmatter table (0078) is the canonical surface for everything
  else.

## Acceptance Criteria

- [ ] On every detail-page route, the chip strip renders at most four
  chips drawn from the canonical set (status, verdict, date, author).
- [ ] No other frontmatter keys appear as chips on any doc kind.
- [ ] Notes pages render with at least the date and author chips when
  those keys are present (so H1 no longer sits directly over the
  divider).
- [ ] Duplicate keys (`last_updated`/`date`,
  `last_updated_by`/`author`) collapse to a single chip per slot.

## Open Questions

- For doc kinds with no `verdict` (most), is the slot omitted entirely
  or rendered as an empty placeholder? Probably omitted.
- Should the `date` chip preferentially show `last_updated` when both
  exist (more recent), or always `date`?

## Dependencies

- Blocked by: 0038 (Chip primitive), 0081 (verdict colouring).
- Related: 0078 (frontmatter table is the destination for everything
  else).
- Blocks: none.

## Assumptions

- Status and verdict will not both appear on the same doc kind; if
  they do, both render.

## Technical Notes

- `FrontmatterChips` is the current implementation point.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0038, 0078, 0081
