---
type: "pr-description"
id: "80"
title: "[0211] Retire the cross-cluster bash residue and finalise the 0211 records"
date: "2026-08-23T23:13:11+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to: ["work-item:0171", "work-item:0210", "work-item:0212", "work-item:0174"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/80"
pr_number: 80
tags: ["jira", "linear", "integrations", "cli", "cutover", "residue"]
revision: "f343df772dffc338835856b19c825b50ab364c2e"
repository: "accelerator"
last_updated: "2026-08-24T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0211] Retire the cross-cluster bash residue and finalise the 0211 records

## Summary

Retires the last bash that only dies once both integration clusters are gone, lands the whole-repository assertions, and finalises the 0211 record set — completing work item 0211. This is **stacked on the Jira binary/cutover PR (#78)**, which is itself stacked on the Linear PR (#77); review those first. This is the child's merge boundary: `mise run` exits 0 end to end here.

## Changes

- **`scripts/work-common.sh` retired** — orphaned once both clusters went (its only consumers were inside them); removed with its two guard-list entries (`SHELL_LIBRARIES` 14 → 13).
- **`test:integration:work` husk retired** — it discovered zero suites and passed green over nothing; removed with its task, `mise` leaf, roll-up member and launcher-dependent partition mirror.
- **`cli/tracker-support` dead bash helpers removed** — `run_bash` and its sole caller `repo_root`, unused since the mapper differential was retired; the stale module doc (which still named the deleted `mapper_differential.rs`) is rewritten.
- **Whole-repository assertions** — `grep "Bash(jq\|Bash(curl" skills/` returns nothing; the shared-asset sweep's residual set is empty under its declared exclusions; no Python remains in the `cli/` test lane. The results are recorded in 0171's decision register.
- **Records finalised** — the `0211-reconciliation.md` jira flow-coverage section; all twenty-one plan decisions mirrored into 0171 (with the surface-widening amendment) and the six 0211-owned `pending`/`open` decision entries closed; 0171's stale attribution of the whole-repository `jq`/`curl` equality to 0212 corrected; 0211's own criteria corrected on seven points (dispatch band `70`–`74`, deletion 263 files / 21,422 lines, 21/6 dispatch modes, `public_api` classification, the Jira-only TTY policy, the additive search op, and the read-compatible init cache).
- **Two stale strings swept** — the `cli/{jira,linear}-client/tests/evidence/README.md` "not committed yet" lines beside committed data, and `create-work-item`'s reference to the 0212-deleted `config-read-work.sh`.
- **Plan marked complete** — Phase 4/5 criteria checked and the plan status set to `done` after the green merge-boundary run.

## Context

Completes `work-item:0211` per `meta/plans/2026-08-19-0211-integration-binaries-and-bash-cluster-retirement.md` (Phase 5). Parent epic `work-item:0171`; unblocks `work-item:0174` (the retired `integrations` floor and the cluster `SHELL_LIBRARIES` entries).

## Testing

- [x] Child merge boundary green: the full `mise run` exits 0 end to end with zero failures (the load-flake seen in an earlier run did not recur).
- [x] `build-system:check`, `cli:check`, `dispatch-coherence`, `integration-skills` all green after the residue removals.
- [x] Whole-repository assertions verified empty: the `jq`/`curl` grant survivor set, the shared-asset sweep residual, and Python in the `cli/` test lane.
- [x] `cli/tracker-support`'s `mapper_differential_self_test.rs` still compiles and passes without the removed helpers.

## Notes for Reviewers

- **Stacked PR** — base is `0211-jira-integration-binary-and-cutover` (#78); this PR's diff is only the residue removal and the record finalisation. Review #77 (Linear) → #78 (Jira binary/cutover) → this.
- **Records-only, mostly** — beyond the three small bash-residue deletions and their guard edits, this PR is documentation: the inventories, the 0171 decision mirror, and the 0211 criteria corrections. The one code-adjacent risk is the `tracker-support` helper removal, covered by the surviving self-test.
- **Merge order** — merge #77, then #78, then this. When #77 merges, GitHub should auto-retarget #78 to `main`, and this PR to #78's post-merge base; retarget by hand if it does not.
