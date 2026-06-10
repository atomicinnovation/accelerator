---
type: work-item
id: "0105"
title: "Close the Corpus Validator Provenance and Linkage Blind Spots"
date: "2026-06-10T13:37:48+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
parent: "work-item:0057"
relates_to: ["work-item:0103"]
tags: [frontmatter, schema, validator, provenance, linkage]
last_updated: "2026-06-10T13:37:48+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0105: Close the Corpus Validator Provenance and Linkage Blind Spots

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Fold two known blind spots into `scripts/validate-corpus-frontmatter.sh` so the
single oracle enforces them directly, allowing the bespoke checks in
`scripts/test-skill-frontmatter-conformance.sh` (and the by-inspection coverage
in the 0103 audit table) to collapse back to the validator:

1. **Provenance over-emission** — a *non-anchored* type that wrongly emits
   `revision`/`repository` is currently **not** flagged (the "iff" is enforced
   only in the anchored ⇒ present direction).
2. **Bare/unquoted typed-linkage values** — the shape check extracts only
   *quoted* tokens, so a bare `parent: 0042` (or path-shaped value) yields no
   tokens and escapes `BAD-LINKAGE-SHAPE` entirely.

## Context

This is a **schema-source / oracle** follow-on surfaced by work item 0103. The
0103 audit established that "passes the validator" is necessary but not
sufficient for full conformance: the validator is a *partial* oracle on these
two axes. To stay green, 0103's conformance guard covers them with its own
`assert_no_provenance_over_emission` and `assert_linkage_shape` helpers (which
bypass the validator and carry a comment naming **this** work item), and the
audit table records them by inspection. That is a deliberate temporary
three-authority state (validator-doesn't-enforce, guard-does, table-records).

Closing the blind spots in the validator itself returns the contract to a
single authority: the guard's two bespoke helpers can then be deleted (or
reduced to a thin liveness check that the validator now rejects the bad case),
and the audit table's by-inspection axes become validator-checkable.

### Blind-spot detail (verified at source)

- **Provenance "iff" one-directional** — `validate-corpus-frontmatter.sh:314-324`:
  `anchored=yes ⇒ revision+repository present`, but no rule rejects
  `revision`/`repository` on a type whose `code_state_anchored` is not `yes`.
- **Quoted-tokens-only linkage shape** — `validate-corpus-frontmatter.sh:355-376`:
  the `while [[ "$rest" =~ \"([^\"]*)\" ]]` loop only sees `"…"` tokens, so a
  bare scalar produces zero tokens and is never shape-checked.

## Requirements

- For any type whose TSV `code_state_anchored` is not `yes`, reject the presence
  of any `FM_PROVENANCE_FIELDS` member (new diagnostic, e.g.
  `FORBIDDEN-PROVENANCE-NONANCHORED`).
- For each typed-linkage key, reject a present-but-non-empty value that is not a
  quoted `"doc-type:id"` reference (i.e. catch bare/unquoted and path-shaped
  values), not only the quoted-token case.
- Add failure-mode fixtures to `scripts/test-validate-corpus-frontmatter.sh` for
  both new rules, mirroring the existing `assert_rejects` pattern.
- Reduce or remove the corresponding bespoke helpers in
  `scripts/test-skill-frontmatter-conformance.sh` so the contract has one
  authority again; update the comments that name this work item.
- Keep all per-type facts data-driven from `templates-schema.tsv` /
  `frontmatter-emission-rules.sh` (no re-encoded contract).
- `mise run test:integration:config` and `mise run test:unit:templates` stay
  green.

## Acceptance Criteria

- [ ] A non-anchored type carrying `revision`/`repository` is rejected by
      `validate-corpus-frontmatter.sh`.
- [ ] A bare/unquoted (and path-shaped) typed-linkage value is rejected by
      `validate-corpus-frontmatter.sh`.
- [ ] `test-validate-corpus-frontmatter.sh` carries failure-mode fixtures for
      both new rules.
- [ ] The bespoke blind-spot helpers in `test-skill-frontmatter-conformance.sh`
      are removed or reduced to liveness checks against the now-enforcing
      validator; the work-item-naming comments are updated.
- [ ] `mise run test:integration:config` and `mise run test:unit:templates`
      stay green.

## Dependencies

- Blocked by: work item 0103 (which introduces the bespoke guard helpers that
  this work item collapses back into the validator).
- Relates to: 0057 (parent epic), 0070 (the migration that shipped the
  validator), ADR-0033 / ADR-0034 / ADR-0040 (the schema this oracle enforces).

## References

- Source: work item 0103 Discovery Pass Record (blind-spot consolidation
  follow-on)
- `scripts/validate-corpus-frontmatter.sh:314-324,355-376` (the two blind spots)
- `scripts/frontmatter-emission-rules.sh` (`FM_PROVENANCE_FIELDS`,
  `FM_TYPED_REF_RE`)
- `scripts/test-skill-frontmatter-conformance.sh` (the bespoke helpers to
  collapse)
