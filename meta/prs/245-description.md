---
type: "pr-description"
id: "245"
title: "Re-key work items onto their Linear identifiers"
date: "2026-11-19T09:24:38+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:PP-879", "work-item:PP-981", "work-item:PP-982", "work-item:PP-933"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/245"
pr_number: 245
tags: []
revision: "ef9476e239e07c6a287db3eb3215ee171ce7cb0d"
repository: "accelerator"
last_updated: "2026-11-19T09:24:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Re-key work items onto their Linear identifiers

## Summary

Work items now take their identity from Linear: `work.id_pattern` becomes
`{tracker}`, and the 390 local work items move from `0001`–`0390` onto their
`PP-` keys. The new keys are carried through the rest of the corpus and pushed
to Linear, and the code that pointed at the old file names follows them,
including a validator check the rename would otherwise have silently switched
off.

## Changes

- **Configuration**: `work.id_pattern: "{tracker}"` in `.accelerator/config.md`.
- **Re-key**: `accelerator work rekey` renamed each work item to
  `PP-<n>-<slug>.md`, added its old number to `aliases`, and rewrote
  `work-item:` references across `meta/`.
- **Documents named after a work item** (by hand): 808 plans, codebase
  research, plan and work-item reviews, validations, inventories, measurements,
  migrations and one design gap are renamed with the key in the ID position,
  e.g. `plans/2026-11-11-0352-…` → `plans/2026-11-11-PP-927-…`. 4,000
  references in 1,059 files follow: frontmatter `id`, `<type>:<id>` typed
  references and paths.
- **References the re-key missed**: 266 `work_item_id` values (each keeping
  its shape), 9 inventory `parent` references and one prose `work-item:`
  reference.
- **PP-977 and PP-980**: their slugs, titles and bodies named other items by
  old number (`Run 0352's …`); they now use the keys.
- **Validator regression**: `SCHEMA_REFERENCE_WORK_ITEMS` in
  `cli/corpus/src/frontmatter_validation/pipeline.rs` named the three work
  items whose Schema Reference tables the templates are cross-checked against.
  The check skips work items it cannot find, so the rename turned it off
  without an error. It now names `PP-87`–`PP-89`, and a new test fails if any
  named work item is missing from the repository.
- **Stale pointers outside `meta/`**: comments and docs naming a real work item
  or plan by its old path (`Glyph.tsx`, `tokens.ts`,
  `code-syntax.global.css`, `visual-diff-ciede2000.ts`, the frontend README,
  `masks.toml`, the migrate skill).
- **Follow-up work items**: PP-981 (teach `work rekey` the cases fixed by hand
  here), PP-982 (the apply took about 40 minutes for 390 items), and PP-933
  widened to cover old numbers in titles and slugs.
- **Linear sync state**: `last-sync.json` after pushing the re-keyed items.

## Context

PR #243 connected the work items to Linear. This PR makes Linear the owner of
their IDs. The gaps the re-key left are recorded in work-item:PP-981 so a
future re-key handles them itself.

## Testing

- [x] `accelerator corpus frontmatter validate` over the whole corpus: no
      violations
- [x] No renamed document's old stem remains anywhere in `meta/`, and no
      rename collided with an existing file
- [x] The new `every_schema_reference_work_item_is_in_the_repository` test
      failed against the old paths and passes against the new ones; the
      real-repository template guard (`template_shape_tree`) passes with the
      cross-check running again
- [x] `mise run fix` (no changes) and `mise run check`
- [x] `mise run test:unit:cli`: 7,728 passed; coverage 94.41% lines, 93.90%
      functions, 93.15% regions
- [x] `mise run test:integration:conformance`: 30 passed
- [x] Linear sync previewed, then applied: 362 issues pushed, 29 baselines
      recorded, no pulls, conflicts or failures
- [ ] The full `mise run` default task (including `docs:check`) was not run;
      no `docs-site/` file changed

## Notes for Reviewers

- The diff is about 2,600 files, almost all renames and mechanical rewrites.
  Review commit by commit; most renamed files also changed content, so they
  show as an add and a delete rather than a rename.
- Linear is already up to date: the 362 pushes happened before this PR was
  opened, so merging sends nothing to the tracker.
- Old IDs still resolve through each work item's `aliases`.
- Left alone on purpose: old numbers inside code spans and fenced blocks,
  synthetic test fixtures (`work-item:0900`, the example paths in
  `clusters.rs` and `indexer.rs`), `work-item:2026` describing a year
  mis-parse, the format examples in the review-plan and validate-plan skills,
  the docs-site case study (it quotes documents as they were), and bare old
  numbers in titles and prose, which are work-item:PP-933's.
