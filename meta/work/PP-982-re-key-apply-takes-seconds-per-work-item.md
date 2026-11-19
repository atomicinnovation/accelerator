---
type: "work-item"
id: "PP-982"
external_id: "PP-982"
aliases: ["draft-g916bq"]
title: "Re-Key Apply Takes Seconds per Work Item"
date: "2026-11-19T08:38:37+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:PP-167"
relates_to: ["work-item:PP-879", "work-item:PP-981"]
tags: ["rekey", "performance", "work-items"]
last_updated: "2026-11-19T08:38:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-982: Re-Key Apply Takes Seconds per Work Item

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator work rekey` took about 40 minutes to apply a re-key of 390 work
items, roughly six seconds per item, which leaves a developer waiting on a
command that looks hung and holds the identity lock for the whole time.

## Context

Re-keying this repository's work items from `0001`–`0390` onto Linear
identifiers under `work.id_pattern: "{tracker}"` ran from 00:44 to 01:24. The
corpus held about 1,500 files under `meta/`, and the apply changed 1,567 of
them. The rate was uneven: about 9 items a minute for the first 10 minutes,
about 5 a minute around the 15-minute mark, then about 11 a minute to the end.

While the apply runs, every other identity command (`work create`,
`work id next`, `work sync`) refuses with `E_IDENTITY_RUN_BUSY`.

## Requirements

Reproduction:

1. Take a corpus of about 390 work items under a numeric pattern, with about
   1,500 files under `meta/`.
2. Set `work.id_pattern: "{tracker}"` and run `accelerator work rekey --preview`.
3. Run `accelerator work rekey`.

Expected: the apply finishes in a time dominated by the files it writes,
growing roughly in proportion to the number of items and the size of the
corpus.

Actual: the apply takes about 40 minutes, about six seconds per item, with a
rate that varies by a factor of two over the run.

## Acceptance Criteria

- [ ] Given a benchmark corpus shaped like this repository's before its
      re-key (about 390 work items and 1,500 files under `meta/`), when
      `accelerator work rekey` applies a re-key of every item, then it
      finishes in under 2 minutes on a developer laptop.
- [ ] Given benchmark corpora of N and 2N work items with proportionally many
      other documents, when each is re-keyed, then the 2N apply takes no more
      than 2.5 times as long as the N apply.
- [ ] Given an apply interrupted after any entry, when `accelerator work rekey`
      runs again it finishes the run, and when `accelerator work rekey
      --discard` runs instead it restores the corpus, as before this change.

## Open Questions

- Where does the time go: a scan of the whole corpus per entry, the progress
  record written before each entry, a working-copy probe, or something else?
  This needs profiling before a fix is chosen.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- The cost is in the apply, not the preview. The preview of the same corpus
  finished in under two minutes.

## Technical Notes

- Each entry is applied as a `Retirement` (`cli/work/src/rekey/entry.rs`
  into `cli/work/src/retirement.rs`). If each retirement scans the whole
  corpus for references, the apply costs items × files, which would explain
  the time.
- Widening the re-key to rename documents named after work items (see
  References) adds more work per entry, so this is worth fixing first or
  alongside.

## Drafting Notes

- The 2-minute target is a judgement: a few hundred items should re-key in
  about the time it takes to read and write every affected file a few times.
  Adjust it once profiling shows what is achievable.
- Priority is medium: a re-key is rare, but the identity lock blocks all
  other work item commands while it runs.

## References

- Related: PP-879 (Work Item Re-Key Command), PP-981 (Re-Key the Documents and References That Carry a Work Item's ID)
