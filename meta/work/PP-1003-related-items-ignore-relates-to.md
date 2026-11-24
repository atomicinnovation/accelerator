---
type: "work-item"
id: "PP-1003"
external_id: "PP-1003"
aliases: ["draft-e8c17r"]
title: "Related Items Ignore `relates_to`"
date: "2026-11-24T08:08:30+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:PP-22"
relates_to: ["work-item:PP-994"]
tags: ["visualiser", "linkage"]
last_updated: "2026-11-24T08:08:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1003: Related Items Ignore `relates_to`

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

The visualiser's related items show none of the links a document declares
in `relates_to`, and none of a `target` that names anything but a work
item. 839 documents in `meta/` carry a non-empty `relates_to` and 368 a
`target` naming another doc type, so most declared links never appear.

## Context

The related response builds its declared items from `read_ref_keys`
(`cli/visualiser/server/src/frontmatter.rs:178-228`). It reads
`work_item_id`, a `target` of the `work-item:` form, `parent`, and
`related`, and resolves them to work items only. `relates_to`, the field
the unified frontmatter schema uses for related artefacts, is never read.

`corpus::linkage` (`cli/corpus/src/linkage.rs:72-80`) already declares
which `relates_to` pairs are allowed between doc types, such as a plan to
a PR description or a work item to topic research.

This was found while settling PP-994, the set model, on 2026-11-24.
PP-994 keeps to the fields read today and leaves this to its own item.

## Requirements

Reproduction, read from the code and not yet run:

1. Have a plan whose `relates_to` names a PR description.
2. Open the plan in the visualiser and look at its related items.

Expected: the PR description shows as an outbound related item, and the
plan as an inbound one on the PR description's page.

Actual: neither page shows the other.

The fix:

- Read `relates_to`, and `target` of any typed-linkage form, through
  `corpus::linkage`'s allowed pairs, for every doc type.
- Resolve each to its document whatever its doc type, not only to work
  items.
- Show each as an outbound related item on the declaring document and as
  an inbound one on the document it names.
- Keep the fields read today working as they do.

## Acceptance Criteria

- [ ] Given a plan whose `relates_to` names a PR description, then the
      plan's related response lists the PR description as outbound, and
      the PR description's lists the plan as inbound.
- [ ] Given a design gap whose `relates_to` names a design inventory, and
      a work item whose `relates_to` names a topic-research set, then each
      pair shows outbound and inbound in the same way.
- [ ] Given a document whose `target` names a document of a type other
      than work item, then that document shows as outbound and the
      declaring document as inbound.
- [ ] Given a `relates_to` value naming a document that does not exist, or
      a pair `corpus::linkage` does not allow, then the related response
      omits it and still returns the document's other related items.
- [ ] Given documents linked today through `work_item_id`, a work-item
      `target`, `parent`, or `related`, then their related items are
      unchanged.
- [ ] Given a document both declared and inferred as related, then it is
      shown once, as declared, as it is today.

## Open Questions

- Does `corpus::linkage` cover every `relates_to` pair found in `meta/`
  today, or do some documents use pairs it does not allow?

## Dependencies

- Blocked by: none.
- Related: PP-994, the set model, which attributes a member document's
  declared links to the member and relies on whatever fields this item
  makes the related response read.

## Assumptions

- `corpus::linkage`'s allowed pairs are the source of which links are
  valid, and the visualiser adds none of its own.

## Technical Notes

**Size**: M — new linkage reading in the related response, an inbound map
across doc types, and tests per allowed pair.

- `read_ref_keys`: `cli/visualiser/server/src/frontmatter.rs:178-228`.
- The related endpoint: `cli/visualiser/server/src/api/related.rs`.
- Allowed pairs: `cli/corpus/src/linkage.rs:72-80`.

## Drafting Notes

- Raised on 2026-11-24 while settling PP-994's open decisions, at the
  author's direction: `relates_to` resolution changes related items for
  every doc type, so it is its own bug rather than part of the set model.
- The counts of 839 and 368 documents were taken from this repository's
  `meta/` on 2026-11-24.
