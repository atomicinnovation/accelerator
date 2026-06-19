---
type: work-item
id: "0118"
title: "Reconcile 0007 Backfill Sentinel With Its Validator"
date: "2026-06-19T23:13:17+00:00"
author: Toby Clemson
producer: refine-work-item
status: ready
kind: task
priority: high
parent: "work-item:0115"
relates_to: ["work-item:0114"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-19T23:47:28+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0118: Reconcile 0007 Backfill Sentinel With Its Validator

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Stop migration 0007 from hard-failing on the exact state its own backfill chose
to tolerate. When a required type-extra has no derivable default, the backfill
must write the accepted sentinel token `unknown` — the same placeholder 0007
already writes for the `verdict` and `lenses` extras — instead of leaving the
extra absent, so `self_validate_structural` passes. This implements research fix
option C (reconcile the backfill with its validator) as a child of 0115, scoped
narrowly to the no-derivable-default path.

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

- On the no-derivable-default path, have the backfill write the sentinel token
  `unknown` for the required type-extra rather than leaving it absent. `unknown`
  is chosen for consistency with the sentinel 0007 already emits for the
  `verdict` and `lenses` extras (`0007:219-220`).
- The sentinel must satisfy the validator on both axes: it must not be a
  `MISSING-EXTRA` (the extra is present) and must not be an `EMPTY-PLACEHOLDER`
  (the validator rejects only literal `""` / `[]`, so the non-empty `unknown`
  token passes).
- Scope strictly to the no-derivable-default path; the broader 0007 backfill
  completeness work is owned by 0114.

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

**Size**: S — write a sentinel on the no-derivable-default path in the 0007
backfill, cross-checked against the corpus validator's two gates; narrowly
scoped to one branch.

- `extra_default()` derivation, including the `pr_number` case and the
  date-prefixed-stem no-fallback branch — `0007:193-223`. Note this same
  function already emits the `unknown` sentinel for the `verdict` and `lenses`
  extras (`0007:219-220`) — reuse that token on the no-derivable-default path.
- The tolerant "left absent" backfill branch —
  `0007:507-510` (`log_warn 0007-DIVERGE[missing-extra-no-default] … left
  absent`).
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
