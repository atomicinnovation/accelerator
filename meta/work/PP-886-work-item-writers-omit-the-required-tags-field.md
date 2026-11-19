---
type: "work-item"
id: "PP-886"
title: "Work Item Writers Omit the Required Tags Field"
date: "2026-10-13T13:38:37+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:PP-167"
relates_to: ["work-item:PP-885"]
external_id: "PP-886"
aliases: ["0333"]
tags: []
last_updated: "2026-10-13T13:38:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# PP-886: Work Item Writers Omit the Required Tags Field

**Kind**: Bug
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator work create`, `accelerator work create-batch`, and the
sync importer leave the `tags` key out of a new work item's frontmatter
whenever there are no tags, but `tags` is a required base field. Every
such item fails `accelerator corpus frontmatter validate` with
`MISSING-BASE-FIELD`. `create-work-item`, `extract-work-items`, and
`sync-work-items` each run that validation on the items they write, so
each item missing the `tags` key needs a hand fix, and repositories
that have already used these paths hold items missing the key that
fail validation today.

## Context

`render_frontmatter` in `cli/work-cli/src/create.rs` builds its field
list by calling `compose_frontmatter` in `cli/work/src/create.rs`, so
the defect lives in `compose_frontmatter` and every caller of
`render_frontmatter` inherits it. `compose_frontmatter` emits `tags`
through `push_sequence_if_non_empty`, treating it like the
omit-when-empty fields. The corpus schema disagrees: `BASE_FIELDS` in
`cli/corpus/src/frontmatter_validation/schema.rs` requires `tags`, and
the work-item template places `tags` outside its omit-when-empty block.
The writer is the side in error.

The **omit-when-empty fields** are exactly `parent`, `blocks`,
`blocked_by`, `derived_from`, `relates_to`, `source`, and
`external_id`: each is written only when it has a value.

Three paths call `render_frontmatter` and so share the defect:

* `accelerator work create`, used by `create-work-item`.
* `accelerator work create-batch --manifest <file>`, used by
  `extract-work-items`.
* The sync importer in `cli/work-cli/src/sync_author.rs`, used by
  `sync-work-items`. It always passes no tags, so it writes every
  pulled issue without the key.

Because the plugin writes into each consuming repository's own work
directory, items missing the `tags` key already exist wherever these
paths have run, most of all where `sync-work-items` has imported
issues.

The defect surfaced when 0332 was created through `create-work-item`:
the item was written without `tags`, its final validation step failed,
and the key was added by hand.

## Requirements

Reproduction:

1. Run `accelerator work create "Example" story medium` with no
   `--tag`.
2. Run `accelerator corpus frontmatter validate --file <written path>`.

Expected: the item carries `tags: []` and validation exits 0.

Actual: the item has no `tags` key and validation exits 1 with
`MISSING-BASE-FIELD — required base field 'tags' absent`.

Fix:

* Every work item written through `render_frontmatter` carries `tags`,
  as `[]` when there are none, on the line immediately before
  `last_updated` and after any omit-when-empty fields that are present.
* The omit-when-empty fields keep their behaviour on all three paths:
  written when they have a value, left out when they do not.
* The `create-work-item` SKILL.md no longer claims `work create` omits
  the `tags` key when `--tag` is absent, and instead states that `tags`
  is always written and names the omit-when-empty fields as the only
  keys left out when empty.
* A migration, applied through `/accelerator:migrate`, repairs every
  work item in the work directory whose frontmatter has no `tags` key:
  * it inserts exactly one line, `tags: []`, immediately before the
    `last_updated` line, or as the last frontmatter line when there is
    no `last_updated`;
  * the rest of the repaired file is byte-for-byte unchanged;
  * it decides whether `tags` is present by reading the frontmatter
    only, so a `tags` key in any YAML form (flow or block list) counts
    as present and a `tags:` line in the body is ignored;
  * every file that already has a `tags` key is byte-for-byte
    unchanged.

## Acceptance Criteria

- [ ] Given `work create` with no `--tag` and no linkage flags, when
      the item is written, then its frontmatter contains `tags: []` on
      the line immediately before `last_updated`, none of the
      omit-when-empty fields is present, and
      `accelerator corpus frontmatter validate` exits 0.
- [ ] Given `work create` with `--tag api` and
      `--relates-to work-item:PP-23`, when the item is written, then its
      frontmatter contains `relates_to: ["work-item:PP-23"]` followed
      later by `tags: ["api"]` on the line immediately before
      `last_updated`.
- [ ] Given a `create-batch` manifest entry with no tags and no linkage
      fields, when it is written, then its frontmatter contains
      `tags: []` on the line immediately before `last_updated`, none of
      the omit-when-empty fields is present, and
      `accelerator corpus frontmatter validate` exits 0.
- [ ] Given a `create-batch` manifest entry with tags `["api"]`, when
      it is written, then its frontmatter contains `tags: ["api"]` on
      the line immediately before `last_updated`.
- [ ] Given a remote issue carrying one or more tracker labels and no
      parent or linked issues, when sync imports it, then the local
      item's frontmatter contains `external_id` holding the issue's
      key, followed by `tags: []` on the line immediately before
      `last_updated`, no other omit-when-empty field is present, and
      `accelerator corpus frontmatter validate` exits 0.
- [ ] Given the `create-work-item` SKILL.md, when its description of
      the `accelerator work create` call is read, then it states that
      `tags` is always written (as `[]` when no `--tag` is passed) and
      names `parent`, `blocks`, `blocked_by`, `derived_from`,
      `relates_to`, `source`, and `external_id` as the only keys left
      out when empty.
- [ ] Given a work directory holding an item with no `tags` key, when
      the migration is applied, then that item's new content equals its
      original content with exactly one line, `tags: []`, inserted
      immediately before the `last_updated` line, and
      `accelerator corpus frontmatter validate` exits 0 on it.
- [ ] Given an item with no `tags` key and no `last_updated` key, when
      the migration is applied, then `tags: []` is inserted as the last
      frontmatter line and the rest of the file is byte-for-byte
      unchanged.
- [ ] Given items with `tags: ["api"]`, with a block-style `tags` list,
      and with no `tags` key in the frontmatter but a `tags:` line in
      the body, when the migration is applied, then the first two are
      byte-for-byte unchanged and the third gains `tags: []` in its
      frontmatter with its body unchanged.
- [ ] Given a work directory the migration has already been applied
      to, when `/accelerator:migrate` runs again, then the migration is
      not listed as pending and no file changes.

## Dependencies

* Blocked by: none
* Blocks: none
* Ordering: the migration runs after `m0007`, which gives every legacy
  item its frontmatter, and takes the next free migration number at
  the time it lands; any other in-flight item adding a migration must
  agree the numbering.
* Informs: a future work item mapping tracker labels to tags (not yet
  raised), which would supersede the sync criterion's `tags: []`
  expectation.

## Assumptions

* Imported issues should start with `tags: []` rather than carrying the
  tracker's labels; mapping labels to tags is a separate concern for
  which no work item has been raised yet.
* The migration covers work items only; other artefact types are
  written from templates that already carry `tags`.

## Technical Notes

* The omission is at `cli/work/src/create.rs:154` in
  `compose_frontmatter`. The function's doc comment
  (`cli/work/src/create.rs:77-78`) lists `tags` among its
  "non-empty-only" fields — its name for the omit-when-empty fields —
  and needs correcting alongside.
* `cli/work-cli/tests/cli_create.rs:123`
  (`typed_linkage_and_tags_are_populated_and_omitted_when_empty`)
  asserts only the populated `tags` case; the empty case is untested.
* Migrations live in `cli/migrate/src/migrations/`; the latest is
  `m0011.rs`. `m0007/backfill.rs` already writes `tags: []` when it
  synthesises frontmatter and is a starting point for the insertion.
* This repository's own `meta/work/` items all carry `tags`, so here
  the migration is expected to change nothing.

## Drafting Notes

* Classed as a bug rather than a schema change because both the schema
  and the template treat `tags` as always present; only the writer
  diverges.
* Priority set high because the defect fires on the common path of
  `create-work-item`, `extract-work-items`, and `sync-work-items`, and
  each one's own validation step fails on the item it just wrote.
* The backfill migration is kept in this item rather than split out
  because both halves serve one goal — every work item carries `tags`
  and validates — and together they remain a small change.
* An item with no `last_updated` gets `tags: []` as its last
  frontmatter line rather than being skipped, so the migration leaves
  no item failing validation for want of `tags`.

## References

* Related: 0332 (Push PR Descriptions to GitHub Unwrapped), the item
  that surfaced this
* Review: `meta/reviews/work/0333-work-item-writers-omit-the-required-tags-field-review-1.md`
