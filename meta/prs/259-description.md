---
type: "pr-description"
id: "259"
title: "Bring work item statuses in line with recent PRs"
date: "2026-11-23T16:14:20+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:PP-701", "work-item:PP-891", "work-item:PP-926", "work-item:PP-990"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/259"
pr_number: 259
tags: ["work-items", "housekeeping"]
revision: "366dcf81b12515a4b8eb8868a148a96b9cef2196"
repository: "accelerator"
last_updated: "2026-11-23T16:14:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bring work item statuses in line with recent PRs

## Summary

Four work items on `main` no longer matched the pull requests behind them. Two
have merged but still read as open, one has an open PR but still reads as
draft, and one epic still reads as draft though its first child is done. This
PR sets each one to match.

## Changes

| Work item | From | To | Evidence |
|---|---|---|---|
| PP-926 — Sync preview hides pending-push refusals | `in-progress` | `done` | #234 merged 23 Nov |
| PP-990 — Shortcut integration | `draft` | `done` | #215 merged 19 Nov; its follow-ups PP-983 to PP-989 are raised |
| PP-701 — Work management additional integrations | `draft` | `in-progress` | PP-990, its first child, is done |
| PP-891 — Tier DOI links by venue | `draft` | `in-progress` | #258 open |

Each change touches only `status:`, `last_updated:`, `last_updated_by:` and
the `**Status**:` line under the heading, as in #237.

## Context

- Follows #237, #179, #165 and #150, which last reconciled work item statuses
  with merged and in-flight work.
- The check covered every work item referenced by PRs #150 to #258. Apart from
  the four here, every merged PR's item is already `done`, and the
  planning-only PRs leave their items at `draft` or `ready` as they should.

## Testing

- [x] `accelerator corpus frontmatter validate` exits 0 across the whole
      corpus.
- [ ] `mise run check` was not run. The PR changes only `meta/` Markdown,
      which none of the check components cover.

## Notes for Reviewers

- **PP-891 will conflict with #258.** #258 changes the same `status:` line
  from `draft` to `ready`. Whichever merges second has to resolve the
  conflict, and `done` is the right value once #258 merges, since `ready`
  would leave the item looking unstarted.
- **PP-886 (#246) and PP-893 (#250) are left alone.** Both PRs already set
  their item to `done`, so changing `main` to `in-progress` here would only
  add a status conflict to each.
- **PP-990 merged with 16 manual checks still open**, and its validation
  result is `partial`. Marking it `done` matches how PP-927 was handled, but
  PP-927's unticked checks got their own follow-up, PP-980, and none of
  PP-990's follow-ups covers its manual checks yet.
- The statuses aren't synced to Linear yet. Run `/sync-work-items` after
  merge.
