---
type: work-item
id: "0104"
title: "Add rejected to the ADR Status Vocabulary in the Unified Schema"
date: "2026-06-10T13:37:48+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: task
priority: medium
parent: "work-item:0057"
relates_to: ["work-item:0103"]
tags: [frontmatter, schema, adr, status, validator]
last_updated: "2026-06-11T12:39:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-126
---

# 0104: Add rejected to the ADR Status Vocabulary in the Unified Schema

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add `rejected` to the `adr` `status_vocab` in `scripts/templates-schema.tsv`
(and the coupled comment in `templates/adr.md`) so the unified-schema corpus
validator accepts a rejected ADR. The vocabulary currently reads
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
  (column 5, the `status_vocab` field) so it reads
  `proposed | accepted | rejected | superseded | deprecated`.
- Update the coupled vocab comment in `templates/adr.md:8` to match. The
  template's `status:` line carries a re-encoded copy of the vocab as a
  trailing comment, and `test-template-frontmatter.sh:237` verbatim-checks that
  line against the TSV cell (`grep -qF -- "$status_vocab"`). The
  `test:unit:templates` suite fails until this comment is brought into sync, so
  the TSV edit alone is **not** sufficient.
- The remaining read sites are data-driven from the TSV and need no change —
  confirm this holds by running the suites:
  - `validate-corpus-frontmatter.sh` (the `BAD-STATUS` path splits the TSV
    `status_vocab` cell on `|` at `:316-330`; no hardcoded adr list).
  - The per-type valid fixtures in `test-validate-corpus-frontmatter.sh`
    (generated from each TSV row).
  - The `ADR_VOCAB` read in `test-skill-frontmatter-conformance.sh:341-342`
    (sourced from `SCHEMA_STATUS[adr]`).
- Add a corpus-validator fixture (in `scripts/test-validate-corpus-frontmatter.sh`)
  proving an `adr` carrying `status: rejected` is accepted, following the
  existing `emit_valid` + `assert_accepts` pattern.
- Flip the deferred `rejected` axis in `test-skill-frontmatter-conformance.sh`
  (`:346-350`) from a `skip_test` to a live `assert_check` + `assert_accepts`
  fixture, matching the non-`rejected` axes. The `skip_test` is already keyed to
  this work item's id (`0104`) — work item 0103 left it that way.
- Confirm `mise run test:integration:config` and `mise run test:unit:templates`
  stay green.

## Acceptance Criteria

- [ ] `templates-schema.tsv` `adr` row `status_vocab` includes `rejected`, reading
      `proposed | accepted | rejected | superseded | deprecated`.
- [ ] The `status:` vocab comment in `templates/adr.md` includes `rejected` and
      verbatim-matches the TSV `status_vocab` cell that the templates suite checks
      (`grep -qF`); kept in sync regardless of line drift.
- [ ] A corpus-validator fixture asserts a `status: rejected` ADR is accepted.
- [ ] The `review-adr` conformance suite runs `assert_check` + `assert_accepts`
      on an `adr` fixture carrying `status: rejected` and that assertion passes
      (the 0103 `skip_test` is removed, replaced by the live assertion — not a
      no-op).
- [ ] `mise run test:integration:config` and `mise run test:unit:templates`
      stay green **with the new rejected fixture and live conformance assertion
      present** — green-ness alone is insufficient if the rejected coverage is
      absent or skipped.

## Open Questions

- The visualiser already maps `rejected → red` (`status-variant.ts:7`), so a
  rejected ADR renders correctly today with no change. Out of scope for this
  work item, but unresolved: should `rejected` ADRs be *filtered out* of
  active-decision listings/visualisations, or shown with a distinct treatment?
  This work item only makes a rejected ADR schema-valid; it does not change how
  rejected ADRs surface to readers.

## Dependencies

- Blocked by: work item 0103 (which records this divergence and leaves the
  `review-adr` `rejected` conformance axis as a `skip_test` keyed to this id).
  Completing this work item closes that deferral — flipping the `skip_test`
  live discharges the axis 0103 deliberately left open.
- Sibling scope: this child's edit is isolated to the `adr` row of
  `templates-schema.tsv` and its own fixtures, so it can proceed independently of
  other 0057 children. If a sibling later touches the same TSV row or shared
  conformance fixture, sequence the two to avoid a merge collision.
- Relates to: 0057 (parent epic), 0070 (the migration that shipped the vocab),
  ADR-0031 (the lifecycle this aligns the schema to), ADR-0042 (status
  reconciliation map).

## Technical Notes

Read-site map for the `adr` status vocabulary, established by verification:

**Data-driven from the TSV — auto-propagate, no edit needed:**

- `scripts/validate-corpus-frontmatter.sh` — loads `status_vocab` into
  `SCHEMA_STATUS` at `:51`; the `BAD-STATUS` path at `:316-330` splits that cell
  on `|` and trims tokens. No hardcoded adr status list anywhere in the script.
- `scripts/test-validate-corpus-frontmatter.sh` — per-type valid fixtures are
  generated from each TSV row via the `emit_valid` helper; the status-reject
  pattern is `emit_valid` + `sed` the `status:` line + `assert_rejects BAD-STATUS`
  (see `:88-90`).
- `scripts/test-skill-frontmatter-conformance.sh:341-342` — `ADR_VOCAB` is read
  from `SCHEMA_STATUS[$ADR_IDX]` (data-driven).

**Coupled re-encodings — must be edited by hand as part of this work:**

- `templates/adr.md:8` — the template's `status:` comment re-encodes the vocab
  and is verbatim-checked by `test-template-frontmatter.sh:237`
  (`grep -qF -- "$status_vocab"`). Templates suite fails until synced.
- `scripts/test-skill-frontmatter-conformance.sh:346-350` — the `rejected`
  `skip_test` branch becomes dead once `rejected` is in the TSV; flip it to a
  live `assert_check`/`assert_accepts` (comment there says "Flips to a live
  assert_check when 0104 lands").

**Independent re-encodings that do NOT read the TSV — already carry `rejected`,
verify only, no edit expected:**

- `skills/visualisation/visualise/frontend/src/api/status-variant.ts:7` — maps
  `rejected`/`deprecated`/`superseded → red`; companion tests at
  `StatusBadge.test.tsx:50-51` and `status-variant.test.ts:29`.
- `skills/decisions/scripts/adr-read-status.sh:51` — docstring enum (already
  lists `rejected`); the script reads/echoes status, it does not validate.
- `skills/decisions/review-adr/SKILL.md` and `create-adr/SKILL.md` — lifecycle
  prose (the conformance guard parses `review-adr`'s `to status: X` prose as the
  source of the `rejected` target).

## References

- Source: work item 0103 Discovery Pass Record (schema-source triage)
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md` (status
  reconciliation map)
- `scripts/templates-schema.tsv` (the `adr` row, col 5)
- `templates/adr.md` (the coupled vocab comment, line 8)
- `scripts/validate-corpus-frontmatter.sh`, `scripts/test-validate-corpus-frontmatter.sh`,
  `scripts/test-skill-frontmatter-conformance.sh`, `scripts/test-template-frontmatter.sh`
- `skills/decisions/review-adr/SKILL.md` (the producer this aligns to)
