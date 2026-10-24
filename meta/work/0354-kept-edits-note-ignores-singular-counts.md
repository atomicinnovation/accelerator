---
type: "work-item"
id: "0354"
title: "Kept-Edits Note Ignores Singular Counts"
date: "2026-10-24T11:40:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "low"
parent: "work-item:0146"
relates_to: ["work-item:0300"]
tags: ["sync", "kept-edits", "wording"]
last_updated: "2026-10-24T11:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0354: Kept-Edits Note Ignores Singular Counts

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

With one earlier kept copy, `accelerator work sync` prints `note: -:
kept-edits-earlier: 1 earlier copies still kept …`. The noun does not agree
with the count.

## Context

Found on 2026-10-24 during 0300's manual checks against the `TES` sandbox
team, after one resolve-to-remote had kept a copy of item 0002's edits.
`earlier_copies_note` (`cli/work-cli/src/sync.rs:787-803`) formats `{} earlier
copies` for every count.

## Requirements

- One copy reads `1 earlier copy still kept`; any other count reads `<N>
  earlier copies still kept`.
- The `kept-edits-earlier` keyword and the rest of the line are unchanged.

## Acceptance Criteria

- [ ] Given one earlier kept copy, when a run reports it, then the note reads
      `1 earlier copy still kept`.
- [ ] Given two earlier kept copies, when a run reports them, then the note
      reads `2 earlier copies still kept`.

## Open Questions

- None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- No skill or script parses the count's noun; `/sync-work-items` reads
  kept copies from the `#\tkept-edits` stdout records.

## Technical Notes

- The two-copy case is already asserted at `cli/work-cli/src/sync.rs:3772`.

## References

- Found during: `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
  (Phase 7 manual verification)
