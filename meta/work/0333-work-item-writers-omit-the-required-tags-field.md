---
type: "work-item"
id: "0333"
title: "Work Item Writers Omit the Required Tags Field"
date: "2026-10-13T13:38:37+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
relates_to: ["work-item:0332"]
external_id: "PP-886"
tags: []
last_updated: "2026-10-13T13:38:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0333: Work Item Writers Omit the Required Tags Field

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator work create`, its batch mode, and the sync importer omit
`tags` from a new work item's frontmatter whenever there are no tags,
but `tags` is a required base field. Every such item fails
`accelerator corpus frontmatter validate` with `MISSING-BASE-FIELD`,
which `create-work-item` runs as its final step, so each untagged item
needs a hand fix.

## Context

`compose_frontmatter` in `cli/work/src/create.rs` emits `tags` through
`push_sequence_if_non_empty`, grouping it with the omit-when-empty
typed-linkage lists. The corpus schema disagrees: `BASE_FIELDS` in
`cli/corpus/src/frontmatter_validation/schema.rs` requires `tags`, and
the work-item template places `tags` outside its omit-when-empty
linkage block. The writer is the side in error.

Three paths share `render_frontmatter` and so share the defect:
`work create` (used by `create-work-item`), batch create (used by
`extract-work-items`), and the sync importer in
`cli/work-cli/src/sync_author.rs`, which always passes no tags and so
writes every pulled issue without the field.

## Requirements

Reproduction:

1. Run `accelerator work create "Example" story medium` with no
   `--tag`.
2. Run `accelerator corpus frontmatter validate --file <written path>`.

Expected: the item carries `tags: []` and validation exits 0.

Actual: the item has no `tags` key and validation exits 1 with
`MISSING-BASE-FIELD — required base field 'tags' absent`.

Fix:

- Every work item written through `render_frontmatter` carries `tags`,
  as `[]` when there are none, in template order.
- `parent`, `blocks`, `blocked_by`, `derived_from`, `relates_to`,
  `source`, and `external_id` keep their omit-when-empty behaviour.
- The `create-work-item` SKILL.md no longer claims `work create` omits
  the key when `--tag` is absent.

## Acceptance Criteria

- [ ] Given `work create` with no `--tag`, when the item is written,
      then its frontmatter contains `tags: []` and
      `accelerator corpus frontmatter validate` exits 0.
- [ ] Given `work create` with `--tag api`, when the item is written,
      then its frontmatter contains `tags: ["api"]` as today.
- [ ] Given a batch manifest entry with no tags, when batch create
      writes it, then its frontmatter contains `tags: []` and
      validates.
- [ ] Given a remote issue imported by sync, when the local item is
      written, then its frontmatter contains `tags: []` and validates.
- [ ] Given `work create` with no linkage flags, when the item is
      written, then `blocked_by`, `derived_from`, `relates_to`,
      `parent`, `source`, and `external_id` remain absent.

## Open Questions

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- Imported issues should start with `tags: []` rather than carrying the
  tracker's labels; mapping labels to tags is a separate concern.

## Technical Notes

- The omission is at `cli/work/src/create.rs` in
  `compose_frontmatter`; its doc comment lists `tags` among the
  non-empty-only fields and needs correcting alongside.
- `cli/work-cli/tests/cli_create.rs`
  (`typed_linkage_and_tags_are_populated_and_omitted_when_empty`)
  asserts only the populated `tags` case; the empty case is untested.
- All existing items in `meta/work/` carry `tags`, so no backfill or
  migration is needed.

## Drafting Notes

- Classed as a bug rather than a schema change because both the schema
  and the template treat `tags` as always present; only the writer
  diverges.
- Priority set high because the defect fires on the common path of
  three skills and is caught only by their own validation step.

## References

- Related: 0332 (Push PR Descriptions to GitHub Unwrapped), the item
  that surfaced this
