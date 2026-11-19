---
type: "work-item"
id: "PP-977"
title: "Bring 0352's CHANGELOG Entry and Plan Into Line With What Shipped"
date: "2026-11-16T11:25:19+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:PP-963"
relates_to: ["work-item:PP-927"]
tags: ["sync", "changelog", "documentation"]
last_updated: "2026-11-16T11:25:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-977"
aliases: ["0386"]
---
# PP-977: Bring 0352's CHANGELOG Entry and Plan Into Line With What Shipped

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

0352's CHANGELOG entry leaves out that a preview lost the duplicate row too,
and sits under `Fixed` on a claim nobody verified. Its plan still describes
advice and wording that did not ship. Correct both.

## Context

Raised in PR #188's review as `R2-01` and two observations on the chapter
"What the skill is told", all marked to be fixed. This epic owns them
instead of the PR.

The entry says an item `work sync` "creates or adopts in the tracker" no
longer prints a second `noop\tunsynced` row. A `--preview` run creates
nothing and loses the row as well.

The entry is under `Fixed` because the PR description says the duplicate row
shipped only in 1.24.0 pre-releases. The release's other changes to
`work sync` output sit under `Breaking`. Whether any stable release printed
the duplicate was not checked.

The plan
(`meta/plans/2026-11-11-PP-927-sync-reports-every-unsynced-item-twice.md`) is
stale in two places. Its Migration Notes still tell scripts to key on the
`unsynced` state column, advice dropped because other rows carry that state.
Its Phase 2 gives the Membership rule and its test in a form that differs
from what shipped.

## Requirements

- Establish whether any stable release printed the duplicate row.
- If one did, move the entry under `Breaking`; if none did, leave it under
  `Fixed`.
- The entry says the row is gone from `--preview` output as well.
- The plan's Migration Notes give the advice the CHANGELOG gives: read the
  item's `create-from-local`, `adopt` or `failed` row.
- The plan's Phase 2 shows the rule and the test as shipped.

## Acceptance Criteria

- [ ] The work item or PR records which releases printed the duplicate row
      and how that was established.
- [ ] The CHANGELOG entry sits under the heading that finding calls for.
- [ ] The entry names `--preview`.
- [ ] The plan no longer advises keying on the `unsynced` state column.
- [ ] The plan's Phase 2 rule reads "a `failed` row, `order-refused`" and
      its test unpacks a single row.

## Open Questions

- If the entry's release has already been cut when this lands, is the
  correction made in place or as a new entry?

## Dependencies

- Blocked by: 0352 (PR #188) merging.
- Blocks: none.

## Assumptions

- A plan is a record that may be corrected after its work ships, as was done
  for 0281.

## Technical Notes

- The duplicate came from `unsynced_creates` cloning plan entries; the first
  release containing that clone bounds the search.

## Drafting Notes

- `R2-02`, on the entry's watermark sentence, was rejected in review and is
  not part of this.

## References

- Source: PR #188's review notes and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0372
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
