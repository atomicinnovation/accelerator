---
type: work-item
id: "0104"
title: "Add rejected to the ADR Status Vocabulary in the Unified Schema"
date: "2026-06-10T13:37:48+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
parent: "work-item:0057"
relates_to: ["work-item:0103"]
tags: [frontmatter, schema, adr, status, validator]
last_updated: "2026-06-10T13:37:48+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0104: Add rejected to the ADR Status Vocabulary in the Unified Schema

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add `rejected` to the `adr` `status_vocab` in `scripts/templates-schema.tsv`
(and any surface that mirrors it) so the unified-schema corpus validator
accepts a rejected ADR. The vocabulary currently reads
`proposed | accepted | superseded | deprecated`, which omits `rejected` —
contradicting the accepted ADR-0031 lifecycle that the `review-adr` skill
implements.

## Context

This is a **schema-source** divergence surfaced by the producer-frontmatter
audit (work item 0103, Discovery Pass Record). The audit's scope explicitly
defers schema-source divergences to child work items under epic 0057 rather
than fixing them at the producer.

The divergence is unambiguous:

- **ADR-0031 (accepted)** adopts the ADR lifecycle vocabulary
  `proposed`, `accepted`, `rejected`, `superseded`, `deprecated`
  (`ADR-0031:28-29`), with `proposed → rejected` in its transition table
  (`:75`) and a `rejected_reason` metadata field written on rejection
  (`:56,67`).
- **`review-adr`** correctly implements that lifecycle: its mutability table
  documents `proposed → rejected` (`SKILL.md:85`) and its Reject action sets
  `status: rejected` and adds `rejected_reason` (`SKILL.md:192-201`).
- **`scripts/templates-schema.tsv:6`** lists the `adr` `status_vocab` as
  `proposed | accepted | superseded | deprecated` — `rejected` is **not** a
  member. A rejected ADR persisted by `review-adr` would therefore fail
  `validate-corpus-frontmatter.sh` with `BAD-STATUS`.

So the producer (`review-adr`) is right and the schema source (the TSV vocab)
is incomplete. The 0070 migration's vocab reconciliation (ADR-0042) collapsed
status synonyms and widened vocabularies only for genuinely distinct states; it
did not record dropping `rejected`, so its omission is most likely an oversight
rather than a deliberate exclusion.

## Requirements

- Add `rejected` to the `adr` `status_vocab` in `scripts/templates-schema.tsv`
  so it reads `proposed | accepted | rejected | superseded | deprecated`.
- Confirm no other surface re-encodes the `adr` vocab divergently (the vocab is
  data-driven from the TSV in both `validate-corpus-frontmatter.sh` and the
  template-shape test, so a single TSV edit should suffice — verify).
- Add a corpus-validator test fixture (in
  `scripts/test-validate-corpus-frontmatter.sh`) proving an `adr` carrying
  `status: rejected` is accepted, and that the `review-adr` status-axis
  assertion in `scripts/test-skill-frontmatter-conformance.sh` flips its
  deferred `rejected` axis from a `skip_test` to a live `assert` once this lands
  (work item 0103 leaves that axis skipped, keyed to this work item's id).
- Confirm `mise run test:integration:config` and `mise run test:unit:templates`
  stay green.

## Acceptance Criteria

- [ ] `templates-schema.tsv` `adr` row `status_vocab` includes `rejected`.
- [ ] A corpus-validator fixture asserts a `status: rejected` ADR is accepted.
- [ ] The `review-adr` conformance assertion in
      `test-skill-frontmatter-conformance.sh` exercises `rejected` as a live
      assertion (the 0103 `skip_test` is removed).
- [ ] `mise run test:integration:config` and `mise run test:unit:templates`
      stay green.

## Dependencies

- Blocked by: work item 0103 (which records this divergence and leaves the
  `review-adr` `rejected` conformance axis as a `skip_test` keyed to this id).
- Relates to: 0057 (parent epic), 0070 (the migration that shipped the vocab),
  ADR-0031 (the lifecycle this aligns the schema to), ADR-0042 (status
  reconciliation map).

## References

- Source: work item 0103 Discovery Pass Record (schema-source triage)
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- `scripts/templates-schema.tsv` (the `adr` row)
- `skills/decisions/review-adr/SKILL.md` (the producer this aligns to)
