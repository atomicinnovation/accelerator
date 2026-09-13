---
type: "pr-description"
id: "121"
title: "Mark work item 0228 as done"
date: "2026-09-13T07:52:54+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0228"
parent: "work-item:0228"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/121"
pr_number: 121
tags: []
revision: "3ce97f85824947d05837ae72a0cb3b3753f1400c"
repository: "accelerator"
last_updated: "2026-09-13T07:52:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Mark work item 0228 as done

## Summary

Transitions work item 0228 (Layered Configuration Key Model) from `ready` to
`done`. The substantive work — renaming `work.default_project_code` to
`work.key`, separating the integration-owned scope key from the local ID
prefix, and the tracker-aware deprecation alias — shipped and merged in PR
#116. This flips the work item's lifecycle status to match.

## Changes

- Set `status: "done"` in the work item frontmatter, replacing `ready`.
- Update the body `**Status**` line to `Done` to match the frontmatter.

## Context

Closes the lifecycle of work item 0228 (`meta/work/0228-layered-configuration-key-model.md`),
implemented by PR #116.

## Testing

- [x] No automated verification applies — the change edits a `meta/work/`
      Markdown work item only, outside all four toolchains, so no `mise`
      check or test covers it.

## Notes for Reviewers

Status-only transition; no code, schema, or behaviour changes. The
implementation this closes landed in PR #116.
