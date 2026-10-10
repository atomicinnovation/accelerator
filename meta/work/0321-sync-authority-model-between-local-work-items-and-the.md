---
type: "work-item"
id: "0321"
title: "Sync Authority Model Between Local Work Items and the Tracker"
date: "2026-10-10T16:36:05+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0296", "work-item:0213", "work-item:0230", "work-item:0290"]
tags: ["sync", "adr", "authority"]
last_updated: "2026-10-10T16:36:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0321: Sync Authority Model Between Local Work Items and the Tracker

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Decide which side, local work items or the remote tracker, is authoritative
for each concern sync touches, and record it in an ADR. Today the answer is
implied piecemeal and inconsistently across ADR-0044, 0051, 0213, 0230, 0290,
0296 and the sync skill.

## Context

No ADR names an authority model. ADR-0044 calls the local `id` authoritative
for the filename; 0230 makes the tracker the source of truth for ids under
`{tracker}` without superseding 0044. 0290 says template vocabularies are
canonical and trackers map onto them, while its field conflicts use last-
writer-wins. 0051 made remote the conflict default; 0213 removed the Enter
default; `skills/work/sync-work-items/SKILL.md` still labels the remote side
"recommended, newer". 0296 adopted "neither side has priority for content"
and "the local side owns representation only while the bodies are
equivalent".

## Requirements

- One ADR states, per concern, which side is authoritative or that sync is
  symmetric, with a rationale: body representation, body content, identity
  (`id` versus `external_id`, under the default pattern and `{tracker}`),
  field values (status, kind, priority), and conflict defaults.
- The ADR resolves ADR-0044 against 0230: supersede 0044's `id` clause, or
  explain why not.
- The decision's follow-on changes are listed, and any not already tracked are
  raised as work items.

## Acceptance Criteria

- [ ] Given the ADR is accepted, then each of the five concerns names a side or
      "symmetric", with a rationale.
- [ ] Given ADR-0044, when the new ADR is accepted, then 0044 is marked
      superseded or the ADR explains why it is not.
- [ ] Given 0296's representation rule, then the ADR either ratifies it or
      names the change 0296 needs.
- [ ] Given the decision, then each divergent artefact found (sync skill text,
      0213, 0290) has a recorded follow-up or a note that none is needed.

## Open Questions

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- 0296 does not wait for this ADR; its representation rule stands unless the
  ADR overturns it.

## Technical Notes

- Evidence: `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md`,
  `meta/work/0230-tracker-owned-work-item-id-generation.md`,
  `meta/work/0290-bidirectional-field-mapping.md`,
  `meta/work/0051-sync-work-items-skill.md`,
  `meta/work/0213-conversational-conflict-resolution-flow.md`,
  `skills/work/sync-work-items/SKILL.md`.
- Consumers anchoring on local markdown structure: `refine-work-item`'s
  literal section anchors, the review lenses, and `section_diff.rs`.

## Drafting Notes

- 0290's "template vocabularies are canonical" is treated as evidence for the
  ADR, not as a decision already taken.

## References

- Parent: 0146 — Work Item Synchronisation Enhancements
- Related: 0296, 0213, 0230, 0290; ADR-0044
