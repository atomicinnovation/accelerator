---
type: work-item
id: "0105"
title: "Close the Corpus Validator Provenance and Linkage Blind Spots"
date: "2026-06-10T13:37:48+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: task
priority: medium
parent: "work-item:0057"
blocked_by: ["work-item:0103"]
relates_to: ["work-item:0104", "work-item:0070", "adr:ADR-0033", "adr:ADR-0034", "adr:ADR-0040"]
tags: [frontmatter, schema, validator, provenance, linkage]
last_updated: "2026-06-15T20:21:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0105: Close the Corpus Validator Provenance and Linkage Blind Spots

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Fold two known blind spots into `scripts/validate-corpus-frontmatter.sh` so the
single oracle enforces them directly, allowing the bespoke checks in
`scripts/test-skill-frontmatter-conformance.sh` (and the by-inspection coverage
in the 0103 audit table) to collapse back to the validator:

1. **Provenance over-emission** — a *non-anchored* type (one whose TSV
   `code_state_anchored` is not `yes`) that wrongly emits `revision`/`repository`
   is currently **not** flagged: the "iff" is enforced only in the forward
   direction (anchored ⇒ provenance present), not the reverse (provenance
   present ⇒ anchored).
2. **Bare/unquoted typed-linkage values** — the shape check extracts only
   *quoted* tokens, so a bare `parent: 0042` (or path-shaped value) yields no
   tokens and escapes `BAD-LINKAGE-SHAPE` entirely.

## Context

This is a **schema-source / oracle** follow-on surfaced by work item 0103. The
0103 audit established that "passes the validator" is necessary but not
sufficient for full conformance: the validator is a *partial* oracle on these
two axes. To stay green, 0103's conformance guard covers them with its own
`check_no_provenance_over_emission` and `check_linkage_quoted` helpers (which
bypass the validator and carry a comment naming **this** work item), and the
audit table records them by inspection. That is a deliberate temporary
three-authority state (validator-doesn't-enforce, guard-does, table-records).

Closing the blind spots in the validator itself returns the contract to a
single authority: the guard's two bespoke helpers can then be deleted (or
reduced to a thin liveness check that the validator now rejects the bad case),
and the audit table's by-inspection axes become validator-checkable.

### Blind-spot detail (verified at source)

- **Provenance "iff" one-directional** — `validate-corpus-frontmatter.sh:296-301`:
  `anchored=yes ⇒ revision+repository present`, but no rule rejects
  `revision`/`repository` on a type whose `code_state_anchored` is not `yes`.
- **Quoted-tokens-only linkage shape** — `validate-corpus-frontmatter.sh:334-357`:
  the `while [[ "$rest" =~ \"([^\"]*)\" ]]` loop only sees `"…"` tokens, so a
  bare scalar produces zero tokens and is never shape-checked.

## Requirements

- For any type whose TSV `code_state_anchored` is not `yes`, reject the presence
  of any `FM_PROVENANCE_FIELDS` member (new diagnostic, e.g.
  `PROVENANCE-ON-NONANCHORED`).
- For each typed-linkage key, reject a present-but-non-empty value that is not a
  quoted `"doc-type:id"` reference (i.e. catch bare/unquoted and path-shaped
  values), not only the quoted-token case. The reject set includes a bare scalar
  (`parent: 0042`), a path-shaped value (`parent: docs/x.md`), a
  bracketed-but-unquoted element (`parent: [plan:0042]`), and a list with any
  one bad element among well-formed ones; the accept set is a quoted
  `"doc-type:id"` reference (matching `FM_TYPED_REF_RE` — the regex in
  `frontmatter-emission-rules.sh` for a quoted `doc-type:id` scalar) and an
  omitted/empty value.
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
      `validate-corpus-frontmatter.sh` with the `PROVENANCE-ON-NONANCHORED`
      diagnostic.
- [ ] For a typed-linkage key, `validate-corpus-frontmatter.sh` rejects each of
      these present-but-non-empty value forms: a bare scalar (`parent: 0042`), a
      path-shaped value (`parent: docs/x.md`), and a bracketed-but-unquoted list
      element (`parent: [plan:0042]`); and a list in which any one element is
      bare/path-shaped while others are well-formed. It continues to accept a
      quoted `"doc-type:id"` reference (`parent: "work-item:0042"`) and an
      omitted/empty value. ("Quoted `"doc-type:id"` reference" means a token
      matching `FM_TYPED_REF_RE` — the regex in `frontmatter-emission-rules.sh`
      for a quoted `doc-type:id` scalar.)
- [ ] `test-validate-corpus-frontmatter.sh` carries failure-mode fixtures for
      both new rules, and each fixture asserts rejection with the specific
      diagnostic for its rule (`PROVENANCE-ON-NONANCHORED` for the
      provenance rule; `BAD-LINKAGE-SHAPE` — reused — for the linkage rule) —
      not merely a non-zero exit. The new linkage fixtures use genuinely
      *unquoted* values (`parent: 0030`, `parent: meta/work/0030-foo.md`) and a
      mixed list with one unquoted element, distinct from the existing
      *quoted*-malformed fixtures (`parent: "0030"` etc.) that the loop already
      catches.
- [ ] The bespoke blind-spot helpers in `test-skill-frontmatter-conformance.sh`
      are either deleted outright, or reduced to a liveness check that asserts
      only that `validate-corpus-frontmatter.sh` now rejects the previously
      uncaught bad fixture (non-anchored-with-provenance / bare-linkage) and no
      longer re-derives the rule independently; the work-item-naming comments
      are updated.
- [ ] `mise run test:integration:config` and `mise run test:unit:templates`
      stay green. (Regression gate, not a sufficiency criterion — the
      blind-spot closure itself is verified by the criteria above. The new
      fixtures from the third criterion are run under `test:integration:config`,
      tying the suite's greenness to the new rules actually executing.)

## Open Questions

- Should the bespoke helpers in `test-skill-frontmatter-conformance.sh` be
  **deleted outright** or **reduced to thin liveness checks** that assert the
  validator now rejects the bad case? The Requirements say "removed or reduced"
  — a genuine choice deferred to implementation.
- ~~Should the bare/path-shaped linkage rejection reuse the existing
  `BAD-LINKAGE-SHAPE` diagnostic, or get a distinct code (e.g.
  `BARE-LINKAGE-VALUE`) for clearer triage?~~ **Resolved: reuse
  `BAD-LINKAGE-SHAPE`.** The unquoted/path-shaped case is the same violation
  class as the already-caught quoted-wrong-shape case ("linkage value is not a
  well-formed typed `doc-type:id` reference"); the quoted-vs-unquoted difference
  is mechanical, not semantic. The validator's convention is one code per
  violation class with specifics in the message (cf. `FORBIDDEN-PROVENANCE`
  bundling `git_commit`/`branch`; `MISSING-PROVENANCE` bundling
  `revision`/`repository`), so a distinct `BARE-LINKAGE-VALUE` would be the only
  code that splits one class on a syntactic axis. Triage clarity is delivered by
  the message text (e.g. `parent: unquoted value '0030' is not a quoted
  "doc-type:id" reference`). Reuse also avoids disturbing the direct `grep -qF
  "BAD-LINKAGE-SHAPE"` consumer at `test-validate-corpus-frontmatter.sh:163`
  (the single-source tamper guard) — the only literal grep of this diagnostic.

## Dependencies

- Blocked by: work item 0103 (which introduces the bespoke guard helpers that
  this work item collapses back into the validator).
- Relates to: 0057 (parent epic), 0070 (the migration that shipped the
  validator), 0104 (the sibling schema-vocab follow-on under 0057, fixing the
  `adr` `status_vocab` in `templates-schema.tsv` / `frontmatter-emission-rules.sh`
  — it touches the same contract files this work item edits, so the two should
  be merge-ordered rather than landed blind in parallel), ADR-0033 / ADR-0034 /
  ADR-0040 (the schema this oracle enforces).

## Assumptions

- 0103 lands first (this work item is `blocked_by` 0103). The two **validator
  rules** themselves are independent of 0103 and could ship early; only the
  **helper-collapse** requirement depends on 0103 having introduced those
  helpers. If 0103 slips, the validator changes can proceed and the collapse
  step waits.
- `FM_PROVENANCE_FIELDS` (currently `revision repository`) is the complete set
  to forbid on non-anchored types. The new rule stays data-driven from that
  array, so any future addition to the provenance bundle is picked up
  automatically without re-encoding the contract.

## Technical Notes

- **Provenance rule** — add a complementary loop mirroring the existing
  `anchored = "yes"` block (`validate-corpus-frontmatter.sh:296-301`): when
  `anchored != yes`, reject the presence of any `FM_PROVENANCE_FIELDS` member
  with a new diagnostic (`PROVENANCE-ON-NONANCHORED`). It sits naturally
  beside the existing legacy-field forbid loop at `:302-305`
  (`FM_FORBIDDEN_PROVENANCE_FIELDS`). The inline comment at `:295`
  ("Provenance bundle iff code_state_anchored=yes") currently overstates what is
  enforced — update it once the reverse direction is real.
- **Linkage rule** — the bare/path-shaped case escapes because the extraction
  loop (`:334-357`) only matches `"…"` tokens via
  `while [[ "$rest" =~ \"([^\"]*)\" ]]`. Pre-split the value on commas/brackets
  and assert each non-empty element is a quoted token matching `FM_TYPED_REF_RE`;
  a bare (unquoted) element then fails the quoting assertion before the
  type-shape check, rather than producing zero tokens and passing silently.
  Emit the same `BAD-LINKAGE-SHAPE` diagnostic (resolved in Open Questions —
  same violation class), with message text distinguishing the unquoted sub-case
  for triage. Note the existing fixtures named "bare-number"/"path-shape"
  (`test-validate-corpus-frontmatter.sh:73-77`) are actually *quoted*
  (`parent: "0030"`, `parent: "meta/work/0030-foo.md"`) and already caught; the
  new fixtures must exercise genuinely *unquoted* values and a mixed list with
  one unquoted element, which are the real blind spot.

## Drafting Notes

- Read "single oracle / authority" as: the validator becomes the sole enforcer;
  the guard helpers are reduced-to-liveness or removed; and the audit table's
  by-inspection axes become validator-checkable. If the intent was instead to
  keep belt-and-braces coverage in the guard, the helper-collapse requirement
  should be softened.
- Both blind-spot line references were re-verified against the current
  `validate-corpus-frontmatter.sh` at revision d8d49046 (provenance block at
  `:295-305` — forward rule `:296-301`, legacy forbid `:302-305`; linkage loop
  at `:334-357`).
- Moved `0103` from `relates_to` to `blocked_by` in frontmatter to match the
  body's "Blocked by: work item 0103" statement; the prior frontmatter left the
  blocking edge unrepresented.
- Provenance diagnostic renamed from `FORBIDDEN-PROVENANCE-NONANCHORED` to
  `PROVENANCE-ON-NONANCHORED` during plan review (2026-06-15): the original was a
  substring-prefix of the existing `FORBIDDEN-PROVENANCE` code, which would let a
  `grep -qF` assertion on the legacy code be vacuously satisfied by the new
  rule's output. The new name carries no such collision.

## References

- Source: work item 0103 Discovery Pass Record (blind-spot consolidation
  follow-on)
- `meta/research/codebase/2026-06-15-0105-corpus-validator-provenance-linkage-blind-spots.md`
  (verified current-state research; the line references above were corrected
  against it)
- `scripts/validate-corpus-frontmatter.sh:295-305,334-357` (the two blind spots)
- `scripts/frontmatter-emission-rules.sh` (`FM_PROVENANCE_FIELDS`,
  `FM_TYPED_REF_RE`)
- `scripts/test-skill-frontmatter-conformance.sh` (the bespoke helpers to
  collapse)
