---
type: work-item
id: "0118"
title: "Reconcile 0007 Backfill Sentinel With Its Validator"
date: "2026-06-19T23:13:17+00:00"
author: Toby Clemson
producer: refine-work-item
status: done
kind: task
priority: high
parent: "work-item:0115"
relates_to: ["work-item:0114"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T20:13:14+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-140
---

# 0118: Reconcile 0007 Backfill Sentinel With Its Validator

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Stop migration 0007 from hard-failing on the exact state its own backfill chose
to tolerate. When a required type-extra has no derivable default, the backfill
must write the accepted sentinel token `unknown` — the same placeholder 0007
already writes for the `verdict` and `lenses` extras — instead of leaving the
extra absent, so `self_validate_structural` passes. It must also emit a counted
`0007-DIVERGE[backfill-sentinel]` breadcrumb naming each stamped file so the
sentinel write (a silent, sticky degradation) leaves an audit trail. This
implements research fix option C (reconcile the backfill with its validator) as a
child of 0115.

**Scope note (revised after planning + two plan-review passes).** The fix covers
**every** required extra that reaches the no-derivable-default branch — not only
`pr_number`. Plan review established that `pr_number` is merely the live,
test-exercised trigger: `result` (plan-validation), `current_inventory`/
`target_inventory` (design-gap), the `design-inventory` `source…
screenshots_incomplete` bundle, and `review_pass` (reviews) all reach it and would
otherwise abort 0007 identically. The fix is a **hybrid** to avoid type coercion:

- **Numeric/boolean** extras (`review_pass`, `sequence`,
  `screenshots_incomplete`) get type-correct bare defaults (`1`/`1`/`true`) in
  `extra_default` — stamping the string `unknown` on them would change their YAML
  type, which the visualiser's typed-frontmatter parser propagates.
- **String/enum** extras with no derivation (`pr_number`, `result`, `priority`,
  `current_inventory`, `target_inventory`, `source`, `source_kind`,
  `source_location`, `crawler`) get the `unknown` sentinel + the
  `backfill-sentinel` breadcrumb at the loop branch — type-safe (string→string).

This **changes the implementation seam** from the originally-drafted
`pr_number`-only edit to the shared loop branch + three `extra_default` typed
cases + a one-condition awk emission change, which pulls a small slice of 0114's
derivation-completeness remit in (see Dependencies).

## Context

Child of 0115 — Make Interactive Migrations Satisfiable Under Agent Invocation.

0007's required-extras backfill tolerantly *leaves an extra absent* when
`extra_default` returns empty (e.g. a `pr_number` whose filename carries an
external tracker key rather than a numeric PR id, so no default is derivable).
But `self_validate_structural` then runs the corpus validator inside a
`set -euo pipefail` block, and the validator treats an absent required extra as
a `MISSING-EXTRA` violation — aborting the whole migration before the
interactive stage is even reached. The backfill and the validator disagree about
the same state.

## Requirements

- On the no-derivable-default path, have the backfill populate the required
  type-extra rather than leaving it absent: a **type-correct default** for
  numeric/boolean extras (`review_pass`/`sequence`→`1`,
  `screenshots_incomplete`→`true`, emitted bare), and the **`unknown` sentinel**
  for string/enum extras (consistent with the sentinel 0007 already emits for
  `verdict`/`lenses`, `0007:219-220`). No field may be type-coerced (e.g. a
  numeric extra must not become the string `"unknown"`).
- The sentinel must satisfy the validator on both axes: it must not be a
  `MISSING-EXTRA` (the extra is present) and must not be an `EMPTY-PLACEHOLDER`
  (the validator rejects only literal `""` / `[]`, so the non-empty `unknown`
  token passes).
- Emit a counted `0007-DIVERGE[backfill-sentinel]` breadcrumb naming each file
  stamped with the sentinel, so the (silent, sticky) degradation is auditable and
  manually reconcilable — replacing the audit signal previously carried by the
  hard abort / the `missing-extra-no-default` breadcrumb.
- Scope to the no-derivable-default path. The fix lives at the loop branch, so it
  applies to every required extra reaching that branch (not only `pr_number`);
  the broader 0007 backfill *derivation* completeness (teaching `extra_default`
  to derive real values for more extras) remains 0114's concern, and the
  standalone prevention cross-check/lint remains 0120's.

## Acceptance Criteria

- [ ] Given a corpus file whose required type-extra cannot be auto-derived from
      its filename (e.g. a `pr_number` on a `<TRACKER>-NNNN-description.md`
      file), when 0007's backfill runs, then it writes `pr_number: unknown` and
      0007's `self_validate_structural` passes rather than hard-failing on the
      absent extra.
- [ ] Given the `unknown` sentinel token, when the corpus validator checks it,
      then it is accepted as neither `MISSING-EXTRA` nor `EMPTY-PLACEHOLDER`.
- [ ] Given a corpus containing a required-extra-bearing file with no derivable
      default, when 0007 runs in full, then it completes its mechanical and
      `self_validate_structural` stages without aborting with
      `FAIL: … MISSING-EXTRA` before the interactive stage is reached.
- [ ] Given a PR-description file whose filename carries a numeric stem (e.g.
      `0042-...` or a `pr`/`PR` segment), when the backfill runs, then the
      derived `pr_number` is written unchanged and no `unknown` sentinel is
      substituted (no regression on the derivable-default path).
- [ ] Given a non-`pr_number` string/enum required extra with no derivation (e.g.
      a plan-validation missing `result`), when 0007's backfill runs, then it is
      written as the `unknown` sentinel and the run completes — confirming the fix
      is generic to the loop branch, not specific to `pr_number`.
- [ ] Given a numeric/boolean required extra with no value (e.g. a
      design-inventory missing `sequence`/`screenshots_incomplete`), when the
      backfill runs, then it is written as a type-correct bare scalar
      (`sequence: 1`, `screenshots_incomplete: true`) — NOT the string `unknown`
      — so its YAML type is preserved (no type coercion).
- [ ] Given any file stamped with the sentinel, when 0007 runs, then a counted
      `0007-DIVERGE[backfill-sentinel]` breadcrumb naming that file is emitted,
      and the removed `missing-extra-no-default` breadcrumb no longer fires for a
      backfilled extra.

## Open Questions

- None.

## Dependencies

- Blocked by: none.
- Blocks: 0120 (the prevention cross-check asserts the backfill-vs-validator
  invariant this task establishes, so it must land after this).
- Relates to: 0115 (parent), 0114 (0007 backfill completeness; this is scoped
  around it).
- Concurrent-edit coupling: this task and 0114 both mutate the same 0007
  required-extras backfill (`0007:502-512`). 0114 is complete and did not touch
  the no-derivable-default branch, so there is no live conflict today; but if
  0114 is ever reopened, reconcile its changes with this `unknown` sentinel
  write on that branch.
- Downstream consumers: the sentinel `unknown` is a persisted corpus value. Any
  later reader of these extras (0007's own interactive linkage stage, or a
  future migration) sees `unknown` rather than a real value. This matches the
  existing `verdict`/`lenses` sentinel contract, so no new consumer handling is
  introduced.

## Assumptions

- The single sentinel token `unknown` is acceptable across all
  no-derivable-default required extras — it reuses the token 0007 already emits
  for `verdict`/`lenses` (`0007:219-220`) and clears both validator gates. A
  per-extra sentinel is explicitly out of scope; if one is ever needed it is a
  separate follow-up, not an expansion of this task.
- This fix remains required even though 0114 (broader 0007 backfill
  completeness) is complete — 0114 did not address the no-derivable-default
  path, and the research shows the contradiction live in the current tree.

## Technical Notes

**Size**: S — write a sentinel + breadcrumb on the no-derivable-default branch in
the 0007 backfill loop, cross-checked against the corpus validator's two gates;
scoped to one branch (which covers all required extras reaching it).

- Three coordinated edits across two files:
  - **`extra_default()` typed defaults** (alongside `review_number) printf '1'`,
    `0007:218`): add `review_pass)`/`sequence)`→`1`, `screenshots_incomplete)`→
    `true`. These never reach the loop sentinel.
  - **awk bare emission** (`0007-frontmatter-rewrite.awk:222`): extend the
    bare-print branch (currently `pr_number`/`review_number`) to include the three
    typed extras so they emit as bare YAML number/boolean, not quoted strings (the
    generic `:223` path normalises and would quote them).
  - **Loop sentinel** (`0007:507-510`): replace
    `log_warn 0007-DIVERGE[missing-extra-no-default]` + `continue` with
    `dv='unknown'` + a `0007-DIVERGE[backfill-sentinel]` breadcrumb, then fall
    through to the pack step. Reached only by string/enum extras after the typed
    cases above.
- `extra_default()`'s catch-all `*)` and `pr_number` case still return empty (the
  loop is the authority for the string/enum sentinel); `lenses` — the only list
  extra — has its own sentinel and never reaches the branch, so no awk
  list-emission branch is needed.
- `self_validate_structural` invocation inside the `set -euo pipefail` block —
  `0007:555-566`, called at `:771`, redirected `} >&2` at `:786`.
- Validator hard-fail: `MISSING-EXTRA` at
  `scripts/validate-corpus-frontmatter.sh:345`; `EMPTY-PLACEHOLDER` (rejects
  literal `""` / `[]` only) at `:348-359` — the sentinel must clear both.

## Drafting Notes

- Implements research fix option C, decomposed under 0115. Low-effort but
  cross-script (0007 driver + corpus validator) reconciliation.
- The sentinel token `unknown` deliberately supersedes the illustrative
  `pending` named in the source research (option C: "e.g. `pending`"). `unknown`
  is chosen for parity with the sentinel 0007 already emits for the
  `verdict`/`lenses` extras (`0007:219-220`); `pending` was only an example.

## References

- Source: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Related: 0115, 0114
