---
type: "pr-description"
id: "165"
title: "Sync six work item statuses"
date: "2026-10-15T11:17:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0225", "work-item:0296", "work-item:0303", "work-item:0310", "work-item:0332", "work-item:0334"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/165"
pr_number: 165
tags: []
revision: "b870da7d5bf012b9dd8d6ea0cb6041dbed100fae"
repository: "accelerator"
last_updated: "2026-10-15T11:24:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Sync six work item statuses

## Summary

Brings six work items' statuses into line with what has landed on `main`.
Five items whose implementing PRs have merged move to `done`; 0296, whose
refinement PR has merged but whose implementation has not, moves into
`in-progress`.

## Changes

| Work item | From | To | Evidence |
|---|---|---|---|
| 0225 — scheduled advisory and age guards | `draft` | `done` | #157 merged |
| 0296 — sync round-trip defects | `draft` | `in-progress` | #156 (refinement) merged |
| 0303 — research concurrency exceeds subagent limit | `draft` | `done` | #158 merged |
| 0310 — recovering from an interrupted conduct run | `ready` | `done` | #163 merged |
| 0332 — push PR descriptions to GitHub unwrapped | `draft` | `done` | #164 merged |
| 0334 — tree leases on the cold path | `ready` | `done` | #162 merged |

Each file changes in two places only: the `status:` frontmatter field and the
matching `**Status**:` line under the heading.

## Context

Follows the same pattern as #150, which last reconciled work item statuses
with merged work.

## Testing

- [x] Confirmed each item moved to `done` has a merged implementing PR
- [x] Confirmed the diff touches only the `status:` field and the body status
  line in each of the six files

## Notes for Reviewers

No code changes. 0296 stays open because #156 only refined the work item
for planning; no PR implementing it has merged. Check `in-progress` matches
your view of where it stands.
