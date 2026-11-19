---
type: "work-item"
id: "PP-981"
external_id: "PP-981"
aliases: ["draft-zevte3"]
title: "Re-Key the Documents and References That Carry a Work Item's ID"
date: "2026-11-19T08:38:17+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-167"
relates_to: ["work-item:PP-879", "work-item:PP-931", "work-item:PP-933", "work-item:PP-887"]
tags: ["work-items", "rekey", "corpus"]
last_updated: "2026-11-19T08:38:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-981: Re-Key the Documents and References That Carry a Work Item's ID

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer re-keying a corpus, I want `accelerator work rekey` to rename
the documents named after a re-keyed work item and rewrite every reference
that carries its ID, so that one re-key leaves the corpus consistent instead
of leaving hundreds of files to fix by hand.

## Context

Re-keying this repository's 390 work items from `0001`–`0390` onto Linear
identifiers renamed the work items and rewrote `work-item:` references, but
left three classes of stale identity behind, all fixed by hand afterwards:

- **Documents named after a work item kept the old number.** Plans, codebase
  research, plan reviews, work-item reviews, validations, inventories,
  measurements, migrations and design gaps carry the work item's ID in the ID
  position of their file name. 808 such files kept it, and with them their
  frontmatter `id` (which equals the file stem), every `<type>:<id>` typed
  reference to them, and every path reference: 4,000 references in 1,059
  files under `meta/`.
- **`work_item_id` was not rewritten.** 260 bare values (`"0352"`), 5 typed
  values (`"work-item:0211"`), and 1 holding a work item's stem
  (`"0165-multi-binary-…"`).
- **Inventories and migrations were outside the re-key's reach.** Nine
  inventories kept `parent: "work-item:0167"`-style references, and a
  migrations document kept a prose `work-item:0186's`.

Once fixed by hand, `accelerator corpus frontmatter validate` reported no
violations across the whole corpus.

## Requirements

- Each document type that names a document after a work item declares where
  the work item's ID sits in its file name: after the date
  (`YYYY-MM-DD-<id>-<slug>`), as the work item's stem
  (`<work-item-stem>-review-<n>`), or leading (`<id>-<slug>`). The re-key
  renames a document only where its declared ID position holds a re-keyed
  item's old ID; it never infers the position from the name.
- A renamed document's frontmatter `id`, every `<type>:<id>` typed reference
  to it, and every path reference to it across `meta/` follow the rename.
- A `work_item_id` naming a re-keyed item is rewritten in the shape it already
  has: bare ID, `work-item:` typed reference, or work item stem.
- Inventories, migrations and measurements take part in the re-key: their
  documents are renamed and their references rewritten like any other
  document type's. A document with no frontmatter (migrations) or that is
  not Markdown (measurements) is still renamed, and path references to it
  follow.
- The preview lists every document rename alongside the work item mapping it
  follows from, so the one confirmation covers everything that will move.
- Document renames take part in the apply's staleness check, its recovery of
  an interrupted run, and the discard's restore, as work item renames do.
- A document rename whose target path already exists is refused at preview,
  before anything is recorded.
- Text the re-key already leaves alone stays alone: code spans, fenced code
  blocks, and references outside `meta/`.
- The apply is no slower than it is today on the same corpus.

**Out of scope**

- Bare old numbers that name other work items in a work item's slug, title,
  H1 or body (e.g. "Run 0352's Outstanding Manual Sync Checks"). These need
  judgement, and belong to the bare-number review (work-item:PP-933).
- Making the apply faster than it is today (work-item:PP-982).
- Rewriting references outside `meta/` (as in work-item:PP-879).

## Acceptance Criteria

- [ ] Given `meta/plans/2026-11-11-0352-sync-reports-every-unsynced-item-twice.md`
      with `id` equal to its stem, and a codebase research document whose
      `derived_from` names `plan:2026-11-11-0352-sync-reports-every-unsynced-item-twice`,
      when `work rekey` maps `0352 → PP-927`, then the plan is at
      `meta/plans/2026-11-11-PP-927-sync-reports-every-unsynced-item-twice.md`,
      its `id` is the new stem, and the `derived_from` names the new stem.
- [ ] Given `meta/reviews/work/0352-<slug>-review-1.md`, when the work item
      `0352-<slug>.md` is re-keyed to `PP-927-<slug>.md`, then the review is
      at `meta/reviews/work/PP-927-<slug>-review-1.md`.
- [ ] Given `meta/decisions/ADR-0026-<slug>.md`,
      `meta/research/issues/2026-06-17-migration-0007-<slug>.md` and
      `…/screenshots/detail-plan-0042.png`, when `work rekey` re-keys items
      `0007`, `0026` and `0042`, then none of the three is renamed.
- [ ] Given `work_item_id` values `"0352"`, `"work-item:0211"` and
      `"0165-multi-binary-distribution-and-release-pipeline"`, when those items
      are re-keyed to `PP-927`, `PP-741` and `PP-186`, then they read
      `"PP-927"`, `"work-item:PP-741"` and
      `"PP-186-multi-binary-distribution-and-release-pipeline"`.
- [ ] Given `meta/inventories/0167-divergences.md` with
      `parent: "work-item:0167"`, a migrations document without frontmatter
      at `meta/migrations/0196-<slug>.md` whose body names `work-item:0186`,
      and `meta/measurements/2026-09-11-0216-<name>.txt` referenced by path
      from a plan, when those items are re-keyed, then all three files are
      renamed, the `parent` and the body reference name the new IDs, and the
      plan's path reference names the measurement's new path.
- [ ] Given `work-item:0166` inside a code span and inside a fenced code block,
      when `0166` is re-keyed, then both are unchanged.
- [ ] Given a corpus with document renames to make, when `work rekey --preview`
      runs, then every document rename is listed with the work item mapping it
      follows from.
- [ ] Given a document listed in the preview is edited before the apply, when
      `work rekey` runs, then it refuses with `E_REKEY_PREVIEW_STALE` naming
      that document.
- [ ] Given an apply interrupted after some document renames, when
      `work rekey` runs again it finishes them, and when `work rekey --discard`
      runs instead it restores every renamed document.
- [ ] Given a document rename whose target path already exists, when
      `work rekey --preview` runs, then it refuses naming both paths and
      records no plan.
- [ ] Given a corpus that passes `accelerator corpus frontmatter validate`
      before a re-key, when the re-key is applied, then it still passes, and no
      renamed document's old stem appears in `meta/` outside code spans and
      fenced code blocks.
- [ ] Given a benchmark corpus of a few hundred work items with their plans,
      research, reviews and validations, when the apply is timed before and
      after this change, then it takes no more than 10% longer.

## Open Questions

- Should inventories, migrations and measurements become registered document
  types, or should the re-key take extra roots of its own? Registering them
  also brings them under the validator and the visualiser.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- A number in a document's declared ID position always names the work item
  the document is about. It did for all 808 renamed files here.
- A `work_item_id` value always names a work item, so it is rewritten without
  asking.

## Technical Notes

- The re-key engine lives in `cli/work/src/rekey/` (`mapping.rs`,
  `run_plan.rs`, `fingerprint.rs`, `write_set.rs`).
- `DocTypeKey` in `cli/corpus/src/doc_type.rs` registers 15 document types;
  inventories, migrations and measurements are not among them, which is
  likely why gap 3 arose.
- The manual fix replaced each renamed document's old stem with its new stem
  across `meta/`, longest first. This was consistent because a plan review's
  stem extends its plan's stem and both share the same ID token.

## Drafting Notes

- "No slower than today" is read as at most 10% longer on a benchmark corpus.
  The baseline from the re-key of this repository was about 40 minutes for
  390 work items.
- Each document type declares its ID position instead of the re-key matching
  a pattern, so ADR numbers, migration numbers and example numbers in names
  are never mistaken for work item IDs.

## References

- Related: PP-879, PP-931, PP-933, PP-887
