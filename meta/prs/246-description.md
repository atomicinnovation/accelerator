---
type: "pr-description"
id: "246"
title: "[0333] Always write tags on new work items and backfill missing ones"
date: "2026-11-19T10:33:18+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0333"
parent: "work-item:PP-886"
relates_to: ["work-item:PP-885"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/246"
pr_number: 246
tags: ["work", "work-cli", "migrate", "frontmatter"]
revision: "a1d449c67a14d44fc8db6d7b0f229d4306417eaf"
repository: "accelerator"
last_updated: "2026-11-19T10:33:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0333] Always write tags on new work items and backfill missing ones

## Summary

`tags` is a required base field, but `work create`, `work create-batch` and
the sync importer left it out of any work item that had no tags. Every such
item failed `accelerator corpus frontmatter validate` with
`MISSING-BASE-FIELD`. This PR makes the shared writer always emit `tags`
(as `[]` when empty) and adds migration `0012-backfill-work-item-tags` to
repair items already written without it.

> **Stacked on #234.** The base is `0351-preview-pending-push-refusals`, so
> this diff shows only the 0333 work. Retarget it to `main` once #234 merges.

## Changes

### Writers always emit `tags`

- `compose_frontmatter` (`cli/work/src/create.rs`) now pushes `tags`
  unconditionally, on the line directly before `last_updated`. Every
  writer reaches it through `render_frontmatter`, so this one-line change
  fixes `/create-work-item`, `/extract-work-items` and `/sync-work-items`
  together. The omit-when-empty fields (`parent`, `blocks`, `blocked_by`,
  `derived_from`, `relates_to`, `source`, `external_id`) are unchanged.
- A new unit test reads the required set from the `corpus` schema
  (`BASE_FIELDS` plus the work-item row's `required_extras`). Any required
  field added there later will fail at the writer, not in the corpus.
- Each of the three writer paths (`work create`, `create-batch`, a sync
  pull) has a test that pins `tags: []` before `last_updated`, the absence
  of empty linkage keys, and a clean `validate_text`.

### Migration 0012 backfills existing items

- `cli/migrate/src/migrations/m0012.rs` inserts exactly one `tags: []`
  line into every `type: "work-item"` file under `paths.work` (including
  `drafts/`) that lacks the key. The line goes before `last_updated`, or
  at the end of the frontmatter when there is none. The insertion works on
  byte offsets, so CRLF endings and a missing final newline survive and
  every other byte is unchanged.
- Key presence and document type come from the live
  `corpus::frontmatter_validation` scanner, so the migration repairs
  exactly what `MISSING-BASE-FIELD` reports.
- That scanner reads only bare column-0 keys, so each repair is also
  checked against a YAML parse. A file is left byte-identical, and named on
  stderr with the fix it needs, when it:
  - is not UTF-8;
  - has frontmatter that doesn't parse;
  - already has a `tags` key spelled in a form the scanner misses (e.g.
    `"tags":`);
  - would have another field's value changed by the insertion.
- It realigns the `/sync-work-items` baselines of the items it repairs, so
  synced items stay `Synced` instead of reading as `LocallyModified` on the
  next sync.
- If a write or the realignment fails, it restores every item already
  repaired. An item that changed after its repair is never overwritten; the
  error names it and says to remove the one added line. The error then
  walks through the recovery steps in order.
- The migration port gains `read_text`, a strict UTF-8 read returning
  `TextFile::Utf8 | NotUtf8`. The existing `read` decodes lossily, so
  writing its result back would turn invalid bytes into U+FFFD. The
  default implementation errors rather than returning `Ok(None)`, so a
  context that hasn't implemented it fails loudly instead of looking like
  a missing file.
- 0012 is registered after 0011, and every test that hard-codes the full
  ledger is updated. The realignment docs in `ports.rs`,
  `sync_baselines.rs` and `realign.rs` now describe any migration that
  rewrites work items, not just 0008's re-render.

### Docs and meta

- The `/create-work-item` and `/extract-work-items` SKILL.md passages, and
  both Quality Guidelines bullets, now say `tags` is always written and
  list the keys that really are omitted when empty. The evals that pass
  through that text were re-run and their benchmarks re-recorded.
- CHANGELOG entries are under `Fixed` (the writer) and `Migrations`
  (0012).
- The work item, codebase research, plan, plan review and validation
  report for 0333 are included. 0333 is marked `done`.

## Migration notes

- **Run `/accelerator:migrate`** to apply 0012. It inserts one line per
  item lacking `tags` and changes nothing else.
- **Upgrade everyone who writes work items before running it.** An
  unfixed writer can keep producing tag-less items after 0012 has run. To
  repair those stragglers, run
  `accelerator migrate --unapply 0012-backfill-work-item-tags` and then
  `/accelerator:migrate`.
- No `schema_version` bump: adding `tags` brings items up to the existing
  v1 shape.

## Context

- Work item: `meta/work/PP-886-work-item-writers-omit-the-required-tags-field.md`
  (Linear PP-886)
- Plan: `meta/plans/2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field.md`
- Validation:
  `meta/validations/2026-11-18-PP-886-work-item-writers-omit-the-required-tags-field-validation.md`
- Decisions: ADR-0033 (unified base frontmatter), ADR-0040
  (omit-when-empty emission)

## Testing

- [x] `mise run test:unit:cli`: 7,808 run, 7,808 passed.
- [x] `accelerator-migrate` suite: 204 passed, including the 14
      `migration_0012` end-to-end tests (CRLF through the real YAML guard,
      non-UTF-8, quoted-`tags` and unparseable items left alone, baseline
      realignment, a sealed directory forcing a failed write that leaves
      the tree as it found it, idempotence and byte stability on
      re-application).
- [x] Every other `mise run` task passed, including `check`, `pup:check`,
      `public-api:check`, `deny:check`, `docs:build`, the Python suites
      and the visualiser suites. The first `test:unit:cli` run inside it
      hit a flaky, unrelated `design-adapters` test and then a full disk.
      It was re-run on its own and passed, so `mise run` has not been
      re-run end to end in one go since.
- [x] Manual: in a scratch repository, a tag-less item written by the
      pre-fix `work create` gained only `tags: []` and then validated; a
      second migrate run reported no pending migrations.
- [x] Manual: this repository's 390 work items all already carry `tags`.
      On a scratch copy, 0012 changed none of them.
- [x] Manual: PP-23 and PP-24 were pulled from Linear with
      `--pull-only --target`, so nothing was sent to Linear, and both
      lacked `tags`. After the migration both stayed `synced` under
      `--preview`. With the pre-migration `last-sync.json` put back, both
      read `locally-modified`.

## Notes for Reviewers

- **Where to focus:** `m0012.rs`'s `yaml_obstacle` / `Repair::restore`
  pair. The guard turns any disagreement between the line scanner and the
  YAML parser into "leave the file alone", and `restore` only ever writes
  back from a state it knows (repaired → original; already original → no
  op; anything else → name it).
- **Recording before writing:** a repair is recorded before its write, on
  purpose. A write can land its rename and still return an error (for
  example when the manifest append fails), and that file must still be
  restored.
- **Known limitations, deliberately out of scope:**
  - Tracker labels are still not mapped to `tags`; the importer writes
    `tags: []`.
  - Items whose `tags` sits out of template order are not reordered.
  - An item repaired while its promotion is stalled under `pending-push/`
    will push unchanged content once on its next sync. That is a
    redundant push; no data is lost.
- `.accelerator/state/integrations/linear/last-sync.json` changes only
  0333's own baseline entry, from syncing the work item while it was
  refined.
