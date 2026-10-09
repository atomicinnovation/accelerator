---
type: "work-item"
id: "0314"
title: "Resolve Migration 0007's Unshipped ADR-0038 Interactive Affordances"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0312"
relates_to: ["work-item:0202", "work-item:0070"]
tags: ["migration-engine", "m0007", "interactive-contract", "typed-linkage"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0314: Resolve Migration 0007's Unshipped ADR-0038 Interactive Affordances

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As someone answering migration 0007's ambiguous-linkage prompts, I want the
context and edit forms ADR-0038 promised, so that I can decide correctly
without opening the source document. Five ADR-0038 decisions did not survive
the Rust port. Each one is either restored or recorded as dropped.

## Context

ADR-0038 parameterised the interactive contract for migration 0007. The Rust
port (0172) dropped several of its affordances without recording a
decision. m0007's prompt `display` repeats the mandatory elements and adds
only the section anchor (`m0007/mod.rs:245-248`).

## Requirements

Give each gap a disposition, restored or dropped:

1. Display extras (ADR-0038:94-103): the literal prose line, the section
   heading, close-scoring alternative keys, and whether the target resolves
   on disk.
2. The full v1 linkage-key menu shown on edit (ADR-0038:103).
3. Path-form edit targets (ADR-0038:110). `validate_edit` currently rejects
   them via `shape::is_well_formed`.
4. Accept-degraded, i.e. relaxing a key such as `parent` → `relates_to`
   (ADR-0038:112).
5. Validation of edits against the ADR-0033/0034 type-pair table
   (`TYPE_PAIRS`), plus validation of accepted raw targets such as
   `relates_to=0030`. Today only `finalise` catches a bad target, and only
   after it has been applied.

## Acceptance Criteria

- [ ] Given each of the five gaps, when this item closes, then the gap is
      either restored with a test exercising it or listed in the ADR-0038
      successor's "Superseded design" with a reason.
- [ ] Given a restored display extra, when an ambiguous transformation is
      prompted on a TTY, then the extra appears in the prompt.
- [ ] Given restored path-form edits, when a user enters a project-relative
      path target that resolves to a corpus document, then the edit is
      accepted and the written linkage uses that document's `doc-type:id`.
- [ ] Given restored type-pair validation, when a user enters a key and
      target type that `TYPE_PAIRS` does not admit, then the edit is
      rejected with `[interactive]` and re-prompted.

## Open Questions

- For each gap, restore or record as dropped? This is decided while
  drafting the ADR-0038 successor in 0202.

## Dependencies

- Blocked by: 0202's decision on the ADR-0038 successor's content.
- Blocks: none.

## Assumptions

- A restored path-form edit is written in `doc-type:id` form, not as a raw
  path, matching ADR-0034's preferred reference form.

## Technical Notes

- Prompt rendering: `cli/migrate-adapters/src/tty_decision_source.rs:42-73`.
  It prints `display` but not `extras`.
- Edit validation: `m0007/mod.rs:290-307`. Type pairs:
  `cli/corpus/src/linkage.rs:56-80`.

## Drafting Notes

- Priority is low: 0007 is a one-shot migration, and many repos may already
  have applied it, which limits the value of restoring these.
- The five gaps are grouped in one item because they share one decision
  point and one prompt surface.

## References

- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`
- Codebase research:
  `meta/research/codebase/2026-10-09-0202-reconcile-migration-engine-adrs-against-the-rust-port.md`
- Related: 0202, 0070
