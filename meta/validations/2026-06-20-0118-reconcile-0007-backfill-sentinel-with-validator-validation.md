---
type: plan-validation
id: "2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator-validation"
title: "Validation Report: Reconcile 0007 Backfill Sentinel With Its Validator"
date: "2026-06-21T00:19:23+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator"
tags: [migrate, migration-0007, corpus-validator, backfill, sentinel]
last_updated: "2026-06-21T00:19:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Reconcile 0007 Backfill Sentinel With Its Validator

### Implementation Status

✓ Phase 1: Validator guard tests for the `unknown` sentinel — Fully implemented
✓ Phase 2: Typed defaults + `unknown` sentinel for underivable required extras —
  Fully implemented

Both phases were implemented in the originating session and landed as two atomic
commits:

- `rwyuwnzz` — *Add validator guard tests for the unknown backfill sentinel*
- `zxutyomp` — *Backfill underivable required extras instead of leaving them
  absent*

Every success-criteria checkbox in the plan is checked.

### Automated Verification Results

Re-run fresh against the committed tree during validation:

✓ Validator suite: `bash scripts/test-validate-corpus-frontmatter.sh` — 58
  passed, 0 failed (includes the six new Phase 1 sentinel-accept guards)
✓ 0007 suite: `bash skills/config/migrate/scripts/test-migrate-0007.sh` — 190
  passed, 0 failed (all 17 new/updated Phase 4 assertions PASS)
✓ `bash skills/config/migrate/scripts/test-migrate.sh` — 472 passed, 0 failed
✓ `bash skills/config/migrate/scripts/test-migrate-snapshot.sh` — 5 passed, 6
  skipped, 0 failed
✓ `bash skills/config/migrate/scripts/test-migrate-interactive.sh` — 160 passed,
  0 failed
✓ Shell component checks: `mise run scripts:check` — green (verified in session;
  no shell/awk files changed since)
✓ Full local CI mirror: `mise run` — green (rc=0) on a clean re-run in session
  (see Potential Issues for the first-run flake)

The named new assertions all pass: `Phase 4 NODEFAULT`, `Phase 4 WIDENING`,
`Phase 4 HYBRID` (sequence/screenshots_incomplete bare + source/source_kind/
source_location/crawler quoted), `Phase 4 REVIEWPASS`, the five reconciled
`backfill-sentinel` / no-abort / no-`missing-extra-no-default` direct-run
assertions, and `Phase 4 PR430: derivable pr_number NOT replaced by sentinel`.

### Code Review Findings

#### Matches Plan:

- **Edit 2a** (`0007-unify-meta-corpus-frontmatter.sh:219-223`) — typed defaults
  added in `extra_default()`: `review_pass) printf '1'`, `sequence) printf '1'`,
  `screenshots_incomplete) printf 'true'`, alongside the existing
  `review_number) printf '1'`. Exactly as specified.
- **Edit 2b** (`0007-frontmatter-rewrite.awk:226-229`) — the bare-print branch is
  extended to `sequence`/`review_pass`/`screenshots_incomplete` using the
  implicit `||`-continuation idiom the plan called for; the adjacent cardinality
  comment was updated to describe bare *typed* scalars.
- **Edit 2c** (`0007-unify-meta-corpus-frontmatter.sh:514-523`) — the
  no-derivable-default branch now assigns `dv='unknown'` and emits
  `0007-DIVERGE[backfill-sentinel]: $f — required extra '$ex' has no derivable
  default; stamped 'unknown'`, then falls through to the untouched pack step. The
  old `missing-extra-no-default` log + `continue` is gone (grep confirms it no
  longer appears in the migration).
- **Phase 1 guards** (`test-validate-corpus-frontmatter.sh`) — two fixtures
  (`pr-description` → bare `pr_number: unknown`; `plan-validation` → quoted
  `result: "unknown"`), each with `assert_accepts` + two `assert_absent`
  (MISSING-EXTRA, EMPTY-PLACEHOLDER), matching the plan verbatim.
- **Phase 2 tests** — all six fixture/assertion groups (NODEFAULT, WIDENING,
  HYBRID, REVIEWPASS, PR430 no-regression, P4BC reconciliation) are present and
  exercise the runner gate end-to-end plus the direct-run breadcrumb path.

#### Deviations from Plan:

- **None of substance.** Minor wording-only divergence: the retained
  `review_number`-backfill assertion's label was reworded to "Phase 4 numberless
  review still backfilled (run completes)" (plan asked for the
  "no mid-rewrite abort" framing to be dropped — done), and the P4BC block
  comment was rewritten as instructed. These are exactly the reconciliations the
  plan prescribed.
- The screenshots_incomplete typed default is `true` (conservative), as the plan
  deliberately specified, rather than the template's optimistic `false`.

#### Potential Issues:

- **First `mise run` hit a flaky SIGTERM** on the parallel visualiser Rust
  test binary (`test:unit:visualiser`, signal 15) — every visualiser test
  reported `... ok`; no assertion failed. Re-running that component in isolation
  passed (418 + 414 tests), and a clean second full `mise run` was green (rc=0).
  Unrelated to these shell/awk-only changes; consistent with known
  parallel-resource contention on the shared Rust test binary.
- **Sticky sentinel (documented, by design).** Once `unknown` is written, the
  `fm_is_empty_val` guard treats it as present, so a re-run never re-derives a
  real value; reconciliation is manual via the `backfill-sentinel` breadcrumb.
  Captured in the plan's Migration Notes — not a defect.
- **Out-of-vocabulary enums.** For enum-typed sentinel fields (`result`,
  `priority`, `source_kind`, `crawler`), `unknown` is a valid string but not a
  vocab member. The corpus validator is enum-agnostic so the gate passes; any
  future enum-switching consumer must degrade gracefully. Flagged in Migration
  Notes; no current consumer is affected (visualiser is string-passthrough).

### Manual Testing Required:

1. Scratch-run emission (already performed during implementation, reproducible):
  - [x] `pr_number: unknown` emitted bare; `result: "unknown"` / `source:
        "unknown"` emitted quoted — all parse as YAML strings.
  - [x] `sequence: 1` / `review_pass: 1` emitted as bare numbers;
        `screenshots_incomplete: true` as a bare boolean (no quoting).
  - [x] `0007-DIVERGE[backfill-sentinel]` breadcrumb names each stamped file and
        the specific extra; typed defaults emit no breadcrumb.
2. No UI surface is involved; the visualiser renders frontmatter generically, so
   no rendering verification is required for this change.

### Recommendations:

- **Merge-ready.** No blocking issues.
- Re-confirm 0120's residual scope before planning it: the
  backfill-vs-validator invariant is now structurally upheld by the loop branch
  + typed defaults, so what remains for 0120 is the forward guard against a
  *future* list-valued required extra (which the awk path would emit malformed)
  or a future validator tightening — as the plan's Migration Notes already state.
- If 0114 is ever reopened, reconcile its changes with the now-direct edit to the
  no-derivable-default branch (`0007:514-524`), per the concurrent-edit coupling
  note.
