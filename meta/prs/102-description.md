---
type: "pr-description"
id: "102"
title: "Record Linear sync baseline after pushing 40 work items"
date: "2026-09-06T22:20:00+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/102"
pr_number: 102
tags: []
revision: "4ff2146d52108205c3bbdd2cd5d35bd4dee28ac5"
repository: "accelerator"
last_updated: "2026-09-06T22:20:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Record Linear sync baseline after pushing 40 work items

## Summary

Advances the Linear sync baseline in
`.accelerator/state/integrations/linear/last-sync.json` to reflect a push-only
sync that updated 40 locally-modified work items in Linear. The baseline is the
sync engine's record of the last-reconciled remote and local hashes per item;
committing it prevents the next sync from re-detecting those 40 items as
divergent.

## Changes

- Rewrites the single-line `last-sync.json` state file: the sync timestamp
  advances and 44 per-item baseline entries change.
- 40 entries are the pushed items, whose `local_hash` now matches what was
  sent to Linear: `0182 0184 0198 0201 0206 0208 0221–0227 0240 0241
  0245–0254 0256 0258–0260 0263–0273`.
- 4 entries are conflicted items (`0136 0158 0169 0203`) whose remote baseline
  (`remote_updated_at`, `remote_hash`) was refreshed by the sync's bulk remote
  read, though they were not pushed.
- No item entries added or removed; all 274 tracked items remain.

## Context

State-file follow-up to the push-only run of `/accelerator:sync-work-items
--push-only`. No work item owns this change — it is the durable baseline the
sync engine writes after reconciliation.

## Testing

- [x] Frontmatter of all 40 pushed work items validated clean during the sync
  (`accelerator corpus frontmatter validate`).
- [x] Change is confined to the machine-generated state file; no source, build,
  or test surface is touched, so the `mise run check` lanes are unaffected.

## Notes for Reviewers

- The diff renders as one enormous line because `last-sync.json` is stored
  single-line; the substantive change is the timestamp plus 44 hash entries, not
  the whole file.
- Two conflicts (`0136 0158 0169 0203`) and six remotely-modified items
  (`0183 0197 0204 0215 0216 0217`) still diverge from Linear and need a
  bidirectional sync to reconcile — out of scope here.
