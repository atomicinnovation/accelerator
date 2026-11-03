---
type: "pr-description"
id: "179"
title: "Mark 0300 and 0306 done and 0281 in progress"
date: "2026-11-03T21:26:13+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0281", "work-item:0300", "work-item:0306"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/179"
pr_number: 179
tags: []
revision: "183a3089902f89124e6aae30eec4648499e4b5fb"
repository: "accelerator"
last_updated: "2026-11-03T21:26:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Mark 0300 and 0306 done and 0281 in progress

## Summary

Brings three work items' statuses into line with the state of their pull
requests. The two whose implementing PRs have merged move to `done`; 0281,
whose implementing PR is open, moves into `in-progress`.

## Changes

| Work item | From | To | Evidence |
|---|---|---|---|
| 0281 — corpus consumption: ask and report | `ready` | `in-progress` | #176 open |
| 0300 — sync pulls of tracked work items fail silently | `in-progress` | `done` | #169 merged |
| 0306 — deterministic rule for citing a source once | `ready` | `done` | #173 merged |

Each file changes in two places only: the `status:` frontmatter field and the
matching `**Status**:` line under the heading.

## Context

Follows the same pattern as #165 and #150, which last reconciled work item
statuses with merged and active work.

## Testing

- [x] Confirmed #169 and #173 have merged and #176 is open
- [x] Confirmed the diff touches only the `status:` field and the body status
  line in each of the three files

## Notes for Reviewers

No code changes. #180, which adds the `explore-pr` skill, is stacked on this
branch.
