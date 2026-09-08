---
type: "work-item"
id: "0281"
title: "Corpus Consumption: Ask and Report"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocked_by: ["work-item:0277", "work-item:0279"]
tags: ["research", "skills", "consumption"]
last_updated: "2026-09-08T11:42:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-865"
---

# 0281: Corpus Consumption: Ask and Report

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Read the accreted corpus. `ask <handle> <question>` answers a question in
conversation from the findings and synthesis, writing nothing; `report <handle>
<question>` writes a formal `reports/<slug>.md` synthesising an answer. Both are
read-only over the findings and synthesis, and both may propose a new focus area
to a subsequent `outline` when coverage is thin, applied only on explicit user
confirmation.

## Context

This closes the build → consult → discover-a-gap → refine loop the reference
systems stop short of. The read-only core depends only on 0277 (findings +
synthesis to read); the gap-proposal is inert until 0279's outline-appending and
gap-detection `conduct` exist to act on it, so this item records `blocked_by`
0279 at the user's direction.

## Requirements

- `ask <handle> <question>`: answers from the findings and synthesis in
  conversation, writes nothing to the set.
- `report <handle> <question>`: writes `reports/<slug>.md` (`research_kind:
  report`) synthesised from the corpus, read-only over the findings and
  synthesis; derives its Sources from the findings' tiers.
- Both may propose a new focus area for a subsequent `outline` when corpus
  coverage is thin — applied only on explicit user confirmation, never a silent
  mutation, and the only way either verb changes the set.
- Registers the `report` `research_kind` value into the existing umbrella doc
  type (no new library entry) and the `reports/` layout + rendering, plus a
  report template resolving through the 3-tier override.
- Reports render through the shared `LibraryDocView` until the Slice 6 page
  (0284).

## Acceptance Criteria

- [ ] `ask <handle> <question>` answers from the findings and synthesis in
      conversation and writes nothing to the set.
- [ ] `report <handle> <question>` writes `reports/<slug>.md` (`research_kind:
      report`) synthesised from the corpus, read-only over the findings and
      synthesis.
- [ ] `ask` and `report` may propose a new focus area for a subsequent `outline`
      when corpus coverage is thin, applied only on explicit user confirmation
      and never as a silent mutation; this is the only way either verb changes
      the set.

## Open Questions

- None specific to this slice.

## Dependencies

- Blocked by: 0277 (findings + synthesis to read), 0279 (gap-proposal
  activation — the read-only verbs depend only on 0277, but the proposed focus
  area is inert without 0279's re-invocation machinery).

## Assumptions

- Reports are many and on-demand, so `report` stays its own verb rather than
  being folded into the singleton `synthesise`.

## Technical Notes

- `ask` and `report` take the set handle plus a question argument; surface this
  in each subcommand's `argument-hint`.
- `synthesise` separately records what is thin as Open Threads in `synthesis.md`,
  which a later `outline` can draw on.

## Drafting Notes

- The read-only core depends only on 0277; the gap-proposal depends on 0279, so
  this item records `blocked_by` 0279 at the user's direction rather than
  documenting the proposal as deferred.
- The `report` template ownership is placed here; the epic leaves it implicit.
- Extracted from source documents without interactive enrichment. Acceptance
  criteria, dependencies, and kind may need refinement before promoting from
  `draft` to `ready`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 4)
- Parent epic: 0121
