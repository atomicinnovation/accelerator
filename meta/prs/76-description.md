---
type: pr-description
id: "76"
title: "Record Linear issue keys and raise the untracked-discovery gate bug"
date: "2026-08-22T22:46:33+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
relates_to: ["work-item:0220"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/76"
pr_number: 76
tags: [sync, linear, work-items]
revision: "8b37e376ef2131de375f78c1a09ed29a0f6d1363"
repository: "accelerator"
last_updated: "2026-08-22T22:46:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Record Linear issue keys and raise the untracked-discovery gate bug

## Summary

Records the Linear issue keys for 37 work items that a bidirectional `accelerator work sync` created as remote issues, and raises work item 0220 for a bug found while investigating why that same run reported zero untracked pulls.

## Changes

- 37 work items (0182–0213, 0215–0219) gain their `external_id` (`PP-712` … `PP-748`) after the sync created each as a Linear issue, and `.accelerator/state/integrations/linear/last-sync.json` advances to match.
- Each write-back carries only the added `external_id` line. The sync engine re-serialised the whole frontmatter as it wrote — stripping quotes from `title`, `date`, `parent`, `derived_from`, `relates_to`, and `last_updated`, refolding three long titles into `>-` block scalars, and collapsing one `last_updated_note` and five `relates_to` arrays onto single lines. That churn was reverted before committing, taking the diff from 248 insertions / 226 deletions down to 38 / 1.
- New work item 0220 records that untracked-remote discovery never runs when the active tracker is Linear: the gate at `cli/work-adapters/src/sync/run.rs:707` requires a configured project scope, but the Linear client already bounds its own search to the credentialed team at `cli/linear-client/src/client.rs:441`. The gate short-circuits before the client is consulted, so a sync reports zero untracked pulls without having searched — indistinguishable from a search that ran and found nothing.

## Context

Work item 0220 is parented to epic 0171 (Jira and Linear Integrations) and relates to 0194 (Tracker Crate and Remote Sync Engine) and 0204 (RemoteTracker Port), both of which built the machinery involved. The item deliberately leaves the fix mechanism open; two candidates are recorded in its Technical Notes.

## Testing

- [x] `accelerator corpus frontmatter validate` exits 0 across the corpus. It first flagged three `BAD-LINKAGE-SHAPE` violations in 0220 — unquoted `parent` and `relates_to` refs, as written by `accelerator work create` — which are fixed in this branch.
- [x] `accelerator work sync --preview` parses every work item and resolves all 37 as synced against their Linear issues.
- [x] Each of the 37 write-backs verified to differ from its pre-sync content by exactly one added `external_id` line.
- [ ] `mise run check` not run. The branch changes only Markdown work items and one JSON state file; no code is touched.

## Notes for Reviewers

The 37 items now read as locally-modified against the sync baseline, because the baseline hashes the engine's re-serialised content rather than the restored files. A sync therefore refuses at 37 pushes until one run with `--max-pushes 37` realigns it; those pushes are semantic no-ops against issues Linear already holds.

`accelerator work create` wrote 0220's linkage refs in a shape the corpus validator rejects. That is a second instance, in a different code path, of the same serialiser-versus-validator mismatch that motivated reverting the sync engine's churn above. No work item captures it yet.

Neither problem would have been caught by a build: `corpus frontmatter validate` is not wired into `mise run check` or any CI workflow.
