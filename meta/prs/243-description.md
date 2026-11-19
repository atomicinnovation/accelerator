---
type: "pr-description"
id: "243"
title: "Sync work items with Linear and ignore machine-local sync state"
date: "2026-11-19T00:39:53+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/243"
pr_number: 243
tags: []
revision: "379e5951c44983a6d6128c0b790ca1c213a4efa0"
repository: "accelerator"
last_updated: "2026-11-19T00:39:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Sync work items with Linear and ignore machine-local sync state

## Summary

This PR brings `meta/work/` back in line with Linear. It links the 37 work
items that had no Linear issue to new ones, takes Linear's newer text for 0307
and 0333, and corrects the body status line on twelve finished items. It also
applies migration 0011, so the per-machine state that a sync writes stays out
of version control.

## Changes

- **New Linear issues.** 0340–0342, 0355–0370 and 0372–0389 each gain an
  `external_id` (`PP-…`) for the issue the sync created. Nothing else in those
  files changes.
- **Pulled from Linear.** 0307 and 0333 take the refined text that was pushed
  to Linear from other workspaces. 0333's earlier local draft was discarded:
  the pulled version supersedes it.
- **Conflicts resolved as local.** 0351–0354 had local status and content
  changes that conflicted with older Linear drafts. They were resolved in
  favour of the local version and pushed, so only the sync record changes for
  them here.
- **Body status lines.** 0162, 0164–0168, 0170, 0178, 0180, 0182, 0221 and 0285
  are `done` in their frontmatter, but their `**Status**` body line still read
  Ready, In Progress or Draft. Linear shows that line in the issue
  description, so it now reads Done, and the corrected text has been pushed.
- **Sync record.** `.accelerator/state/integrations/linear/last-sync.json`
  records all of the above, so the next sync starts from this state.
- **Migration 0011.** `.accelerator/.gitignore` now ignores
  `state/retirement-recovery/`, `state/sync-recovery/`, and the `pending-push/`,
  `conflicts/` and `adopted/` directories under `state/integrations/*/`.
  `migrations-applied` records the migration.

## Context

Housekeeping, with no owning work item. It follows the status reconciliation
in #237. Every PR merged since #237 already had the right status for its work
item, so no frontmatter `status:` changes here.

## Testing

- [x] `accelerator corpus frontmatter validate` passes on all 51 changed work
      items.
- [x] `accelerator work sync --preview` on the branch tip reports 390 items in
      sync, with nothing to push or pull and no untracked issues found.
- [x] None of the paths that migration 0011 now ignores are tracked, so
      nothing needs removing from version control.

## Notes for Reviewers

- **Merge order with other workspaces.** 0307 and 0333 are also being edited
  in work that has not merged yet: 0307 in `visualisation-system` (reviewed
  and planned) and 0333 in `ticket-management`, which is built on the 0351
  branch of #234. Each version there started from the text that was pulled
  here, so whichever merges second may conflict on those two files. Keep the
  workspace's version: it carries the item's newer status and its original
  Markdown.
- **Linear's Markdown dialect.** The pulled 0307 and 0333 come back in Linear's
  style: `*` bullets, and `~` escaped as `\~` in 0307. This is the round-trip
  that 0320 (Linear push corrupts Markdown content) describes, and it is left
  as Linear returned it so the sync record matches.
- **Duplicate 0391 number.** The `build-system` workspace has raised its own
  0391, which collides with the 0391–0398 in #215. Whichever lands second will
  need a new number. That is outside this PR's scope.
