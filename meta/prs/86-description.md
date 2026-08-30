---
type: "pr-description"
id: "86"
title: "Close work item 0199 as done"
date: "2026-08-30T23:05:30+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0199"
parent: "work-item:0199"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/86"
pr_number: 86
tags: []
revision: "7ee3cbccd9cc07da8e3098ff7c9cf9eccbbc1b88"
repository: "accelerator"
last_updated: "2026-08-30T23:05:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Close work item 0199 as done

## Summary

Closes work item 0199 (retire `scripts/vcs-common.sh`'s residual shell callers and `hooks/launcher-link-refresh.sh`) as done. Every decision the item scoped was settled elsewhere, so this is a bookkeeping-only change: no code moved under 0199.

## Changes

- Transition `0199`'s status from `draft` to `done` and sync the `**Status**:` body label to match.
- Tick all five acceptance criteria, each of which is satisfied by work already merged.
- Add a **Closing Note** recording how each requirement was resolved and flagging 0125 for separate reconciliation.

## Context

The `scripts/` shell surface 0199 targeted was removed wholesale by 0174: `scripts/vcs-common.sh` (with its `classify_checkout`, `find_repo_root`, and `vcs_mode`) and `scripts/test-vcs-common.sh` no longer exist. That dissolves the caller-inventory, `classify_checkout`-fate, and test-repoint requirements — there are no surviving `find_repo_root`/`vcs_mode` shell callers to inventory, and `classify_checkout` is deleted along with its file. The fourth requirement — `hooks/launcher-link-refresh.sh`'s fate — is settled the other way: it was deliberately retained as bash-3.2 shell, one of the two entries in `SURVIVING_SHELL_SOURCES` (`tasks/shared/sources.py`) and documented under "Surviving thin shell" in `tasks/README.md`.

Implements: work-item:0199.

## Testing

- [x] No code changed — the diff touches only `meta/work/0199-*.md`, so the full `mise run` gate is not exercised by this PR.

## Notes for Reviewers

Bookkeeping only; the substantive work landed under 0174 and 0169. `0199` carries `external_id: PP-729`, so the remote Jira ticket should be closed separately (via `/sync-work-items` or directly). `0125`, for which 0199 was the designated successor, had its remaining surface absorbed by the same 0174 deletion and warrants the same close-with-note treatment as a follow-up.
