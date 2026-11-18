---
type: "pr-description"
id: "237"
title: "Reconcile work item statuses and close 0136"
date: "2026-11-17T23:44:49+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0136", "work-item:0146", "work-item:0355", "work-item:0361", "adr:ADR-0075"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/237"
pr_number: 237
tags: ["work-items", "housekeeping"]
revision: "7332f2a3cd9427fc2db5d5d0f0c777382fdc0e7a"
repository: "accelerator"
last_updated: "2026-11-18T10:39:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Reconcile work item statuses and close 0136

## Summary

Updates work item statuses to match the open and recently merged pull requests, and fixes how several items are linked. It lands the runtime-cache epic 0355, which was written in October but never pushed, and moves 0222 under it. That leaves 0136 with no open children, so 0136 closes as done.

## Changes

**Statuses that match the pull requests**

| Work item | From | To | Evidence |
|---|---|---|---|
| 0338 — Linear update overwrites fields it was not given | `draft` | `done` | #181 merged |
| 0352 — Sync reports every unsynced item twice | `draft` | `done` | #188 merged |
| 0319 — Explicit work item ID on `work create` | `draft` | `done` | #178 merged |
| 0364 — Parallelise the serial test suites | `ready` | `in-progress` | #220, #221, #222, #225, #229, #230, #232 merged; #235, #236 open |
| 0351 — Sync preview hides pending-push refusals | `draft` | `in-progress` | #234 open |
| 0361 — Speed up the full build | `draft` | `in-progress` | 0362, 0363, 0368 done; 0364 under way |

**Linking fixes**

- 0124 is `abandoned`. 0199 removed `scripts/vcs-common.sh`, so the `find_repo_root` bug it describes no longer exists.
- 0333 now sits under 0146 and is listed in 0146's Stories, because the sync importer is one of the writers with the defect.
- 0372 no longer sits under 0146. The two now list each other in `relates_to`.

**Runtime-cache epic 0355, which was never pushed**

- Two commits from 2026-10-27 were sitting in local history with no bookmark. They are rebased here unchanged, apart from the ADR number.
- They raise epic 0355, Pinned, Seedable Runtime Cache, with children 0356–0360. They also reshape 0222 into "Seed the Runtime Cache from a Staged Release Directory", now under 0355 and blocked by 0358.
- They propose the digest-pinned resolution chain ADR and extend 0340 with the obligation to check a file before executing it. The ADR was drafted as ADR-0074, but #226 has since used that number. It is **ADR-0075** here, and every reference to it in 0340 and 0355–0357 is updated to match.

**Closing 0136**

- With 0222 under 0355, every child of 0136 has reached a final status. The epic is `done` and both acceptance criteria are ticked.
- 0334 and 0339 were parented to 0136 but missing from its Decomposition list and Children line. Both are added, and 0222 is removed.

Each status change touches only the `status:` field and the `**Status**:` line below the heading, as in #179.

## Context

- Follows the same pattern as #179, #165 and #150, which last reconciled work item statuses with merged and in-flight work.
- The commit for 0355 depends on ADR-0075, which arrives in the next commit. Merge the stack as a whole.

## Testing

- [x] `accelerator corpus frontmatter validate` passes on every one of the 20 changed documents.
- [x] The stack rebases onto current `main`. The one conflict came from 0390, which `main` added to 0146's Stories next to 0333; both entries are kept.
- [x] No reference to `ADR-0074` that means the digest-pinned chain remains. The references that remain are to the pruning ADR from #226.
- [ ] `mise run check` was not run. The PR changes only `meta/` Markdown, which none of the check components cover.

## Notes for Reviewers

- **ADR-0075 is still the October draft, with `proposed` status.** It predates the acceptance of ADR-0074 (prune only a private cache root) in #226. Read the two together before ADR-0075 is reviewed for acceptance.
- **0355–0360 have no `external_id`**, so the next `/sync-work-items` run will create them in Linear. The status changes above also still need pushing to Linear.
- **0255 (merge conflicting chunks in sync) is left on its own.** A separate sync-improvements epic would overlap almost entirely with 0146.

https://claude.ai/code/session_01HEhrwVSi51BQEL1rQTyMGm
