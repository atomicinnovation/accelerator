---
type: plan
id: "2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator"
title: "Reconcile 0007 Backfill Sentinel With Its Validator Implementation Plan"
date: "2026-06-20T18:22:11+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0118"
parent: "work-item:0118"
derived_from: ["codebase-research:2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator"]
tags: [migrate, migration-0007, corpus-validator, backfill, sentinel]
revision: "71735c71d7dab85e8ce4f743c3edac2693d2c563"
repository: "visualisation-system"
last_updated: "2026-06-20T21:41:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Reconcile 0007 Backfill Sentinel With Its Validator Implementation Plan

## Overview

Migration 0007's required-extras backfill leaves a required type-extra **absent**
when no default is derivable (e.g. the `pr_number` of a date-/tracker-prefixed
filename). One step later, inside the same `set -euo pipefail` block, it runs the
corpus validator, which treats that absent extra as a hard `MISSING-EXTRA`
violation and aborts the entire migration before the interactive stage is
reached. The backfill and the validator disagree about the same state.

This plan makes the backfill populate **every** required extra that has no
derivable default, using a **hybrid** strategy so no field is type-coerced:

- **Numeric/boolean extras** (`review_pass`, `sequence`,
  `screenshots_incomplete`) get a **type-correct default** in `extra_default()`
  (`1`, `1`, `true`), emitted as bare YAML numbers/booleans — never a string.
- **String/enum extras with no derivation** (`pr_number`, `result`, `priority`,
  `current_inventory`, `target_inventory`, `source`, `source_kind`,
  `source_location`, `crawler`) get the sentinel token `unknown` written by the
  backfill loop — generalising the exact token 0007 already emits for `verdict`
  and `lenses` — plus a counted `0007-DIVERGE[backfill-sentinel]` breadcrumb
  naming each stamped file. For these string-typed fields `unknown` is a string,
  so there is no type change.

Either way the validator's two gates pass, the migration runs to completion, and
operators retain an audit trail of every degraded value. It implements research
fix **option C**, decomposed under epic 0115.

**Scope note (changed during plan review).** The work item and the original draft
scoped this strictly to the `pr_number`-in-`extra_default` path (research "option
A"). Plan review established that `pr_number` is *not* the only required extra
that reaches the no-derivable-default branch — `result`, `current_inventory`,
`target_inventory`, the `design-inventory` `source…screenshots_incomplete`
bundle, and `review_pass` all reach it identically — so a `pr_number`-only fix
leaves the same contradiction live for those types. The fix was therefore
**deliberately widened** to the loop's generic no-derivable-default branch,
resolving the contradiction for every required extra at once. This overlaps the
territory nominally owned by 0114 (backfill completeness) and 0120 (prevention
cross-check); see Migration Notes and the work-item reconciliation note. The
live, test-exercised trigger remains `pr_number`. A second plan-review pass then
refined the blanket sentinel into the hybrid above, because stamping the string
`unknown` onto the numeric/boolean extras would have been a type change the
visualiser's typed-frontmatter parser propagates; adding their type-correct
defaults pulls a small slice of 0114's "backfill derivation completeness" remit
into this task (recorded in the work-item reconciliation note).

## Current State Analysis

The contradiction is live and confirmed in the current tree
(`scripts/validate-corpus-frontmatter.sh` revision `71735c71`):

- **`extra_default()` returns empty for an underivable `pr_number`.** The
  `pr_number` case (`0007:201-216`) is two-stage against the stem: a PR-anchored
  segment match, then a leading-number fallback that is *deliberately suppressed*
  for date-prefixed stems (`0007:212`). A `<TRACKER>-NNNN-description.md` or
  `YYYY-MM-DD-...` filename matches neither stage → `printf '%s' ""`.
- **`extra_default()` returns empty for several other required extras too.** Its
  catch-all `*) printf ''` (`0007:221`) fires for every required extra it has no
  case for. Per `templates-schema.tsv` col 4, minus the optional carve-out
  `FM_OPTIONAL_EXTRAS` (`external_id reviewer pr_url merge_commit decision_makers
  work_item_id`), the required extras with **no** `extra_default` derivation and
  **no** sentinel are: `result` (plan-validation), `current_inventory` /
  `target_inventory` (design-gap), `source` / `source_kind` / `source_location` /
  `crawler` / `sequence` / `screenshots_incomplete` (design-inventory),
  `review_pass` (plan-/work-item-review), and `priority` (work-item; `kind` is
  separately guarded by a `0007-REFUSE` precondition, so only `priority` is
  exposed there). All are scalars (verified against the templates), but of two
  flavours: **string/enum** (`pr_number` emitted bare; `result`, `priority`,
  `current_inventory`, `target_inventory`, `source`, `source_kind`,
  `source_location`, `crawler`) and **numeric/boolean** (`review_pass: 1`,
  `sequence: 1`, `screenshots_incomplete: false` in the templates). A blanket
  string sentinel is safe for the former but would type-coerce the latter, which
  is why the fix is the hybrid in the Overview.
- **The backfill loop then leaves the extra absent.** When `extra_default`
  returns empty, the tolerant branch (`0007:507-509`) logs
  `0007-DIVERGE[missing-extra-no-default]` and `continue`s — no record is added,
  the extra is omitted from the file.
- **`self_validate_structural` aborts on that same state.** Called unguarded at
  `0007:771` inside the `{ ... } >&2` block (`0007:747-786`), under `set -e`, a
  non-zero validator exit propagates straight out and aborts the migration —
  *before* the interactive harness (`0007:788`) runs. `MISSING-EXTRA`
  (`validate-corpus-frontmatter.sh:345`) is a pure key-presence check.

The sentinel mechanism already exists and the validator already accepts it:

- `extra_default()` already `printf 'unknown'`s for `verdict`/`lenses`
  (`0007:219-220`). Reusing it for the no-derivable-default branch is exact
  parity, not a new convention.
- `MISSING-EXTRA` is key-presence only (via `bk_present`,
  `validate-corpus-frontmatter.sh:186-192,343-346`) → a present `name: unknown`
  is not missing. `EMPTY-PLACEHOLDER` rejects *only* the literal `""` and `[]`
  (`:354-356`) → the non-empty token `unknown` falls through. There is **no
  third check** keyed to any extra — no numeric/format validation exists — so the
  bareword `unknown` is accepted on every axis, for every extra. **AC #2 is
  therefore already true of the validator; no validator production change is
  needed.**
- The awk emitter parses the packed backfill channel with a **generic** emit loop
  (`0007-frontmatter-rewrite.awk:203-225`). It hard-codes only one list extra
  (`lenses`, `:220`); every other extra — `pr_number`/`review_number` bare
  (`:222`), and the `else` scalar path for `topic` and any future scalar extra
  (`:223`, which normalises via `fm_normalise_value` and therefore **quotes** the
  value) — emits a scalar with no awk change. So `pr_number: unknown` (bare) and
  `result: "unknown"` / `current_inventory: "unknown"` (quoted) all emit as YAML
  strings the validator accepts.

### Key Discoveries:

- **A fixture already exercises the `pr_number` path — and is green while masking
  the bug.** `test-migrate-0007.sh:1222-1250` (the `no-pr-number-review` fixture,
  "P4BC") runs a numberless pr-review via `run_0007_direct`, asserts the
  `missing-extra-no-default` breadcrumb *fires* (`:1243`), asserts `review_number`
  was backfilled (`:1247`), and asserts the file has **no** `pr_number:` line at
  all (`:1249-1250`) — but **never asserts the exit code**. The migration *does*
  abort at `self_validate_structural` on this fixture today; because the mutation
  and breadcrumbs happen *before* the abort, the suite stays green. This fix must
  **rewrite that test**: the `missing-extra-no-default` breadcrumb is replaced by
  `backfill-sentinel`; `pr_number: unknown` is now written (so the `:1249-1250`
  "no fabricated pr_number" assertion must be removed/inverted); the run no longer
  aborts; and the stale block comment at `:1218-1220` must be corrected.
- **No reachable required extra is list-valued, so no awk list branch is
  needed.** `lenses` — the only list extra — already has its own
  derivation/sentinel and never reaches the empty branch; every other reachable
  required extra is a scalar. The string/enum scalars route through the generic
  `awk:223` path and emit correctly as (quoted) strings. The numeric/boolean
  scalars (`sequence`, `review_pass`, `screenshots_incomplete`) must emit *bare*
  to preserve their type, so they are given typed defaults in `extra_default` and
  added to the awk bare-print branch (`awk:222`) — a one-condition extension, not
  a new list branch.
- **The in-place fixture-rewrite idiom is `sed -i.bak`** in the validator suite
  (`test-validate-corpus-frontmatter.sh:57,61`), and `emit_valid` emits required
  (non-optional) extras as `name: "x"` — so a `pr-description` fixture is
  generated with `pr_number: "x"`, which we rewrite to the bare sentinel. The
  `.bak` suffix is required for BSD/GNU `sed -i` cross-compatibility and must be
  kept.

## Desired End State

Migration 0007 populates every required, non-optional extra rather than leaving
it absent: numeric/boolean extras get type-correct bare defaults from
`extra_default` (`sequence: 1`, `review_pass: 1`, `screenshots_incomplete:
true`), and string/enum extras with no derivation get the `unknown` sentinel
(bare for `pr_number`, quoted for the rest) written by the backfill loop, which
also emits a counted `0007-DIVERGE[backfill-sentinel]` breadcrumb naming each
stamped file. Run over a corpus containing such files (e.g. a pr-review whose
`pr_number` cannot be derived, a plan-validation missing `result`, or a
design-inventory missing its whole extras bundle), 0007 populates them and
completes its mechanical + `self_validate_structural` stages without aborting. The
corpus validator accepts every emitted value on both gates. The derivable/present
path is untouched (no default substituted where a real value exists or is
derivable), and no field is type-coerced. All four migrate test suites and the
validator suite pass; `mise run` is green.

Verification: the automated success criteria of both phases below, plus the
broader migrate suites and `mise run`.

## What We're NOT Doing

- **Not** changing the corpus validator. AC #2 is already satisfied by the
  current validator; Phase 1 adds only guard tests that lock the contract.
- **Not** introducing a sentinel for the numeric/boolean extras — those get
  type-correct defaults (`1`/`1`/`true`), not the string `unknown`. The single
  `unknown` token is used only for the string/enum extras with no derivation,
  where it is type-safe.
- **Not** changing the awk emitter's quoting *for string extras*.
  `pr_number`/`review_number` emit bare (`pr_number: unknown`, `awk:222`); every
  other string/enum extra is normalised and emitted quoted (`result: "unknown"`,
  `awk:223`). Both parse as YAML strings and the validator accepts both — see the
  emission note in Phase 2 §1(b). (The awk *bare* branch is extended for the
  numeric/boolean extras — see Phase 2 §2b — but the string-quoting path is
  unchanged.)
- **Not** adding awk list-emission branches — no required extra reaching the
  branch is list-valued; `lenses` (the only list extra) has its own sentinel and
  never reaches it. (A *future* list-valued required extra with no derivation
  would need a list branch; out of scope, and the HYBRID test would surface it.)
- **Not** addressing the broader 0007 backfill *derivation* completeness beyond
  the three numeric/boolean typed defaults this fix adds (teaching `extra_default`
  to derive *real* values for more extras remains 0114's concern), nor building
  the standalone prevention cross-check / lint (0120). This plan resolves the
  **backfill-vs-validator contradiction** for every required extra and adds
  type-correct defaults only where a blanket sentinel would type-coerce; it does
  not add 0120's separate guard. The scope overlap with 0114/0120 is intentional
  and recorded in the work-item reconciliation note (Migration Notes).

## Implementation Approach

Two independently-mergeable phases, TDD where a true red step exists:

1. **Phase 1 — validator guard tests (AC #2).** Test-only, no production change.
   Locks in that `name: unknown` is accepted by the validator (neither
   `MISSING-EXTRA` nor `EMPTY-PLACEHOLDER`) for both a `pr_number` and a
   non-`pr_number` extra (`result`), proving the contract is extra-agnostic. This
   is a characterization/guard test — it passes immediately against the current
   validator — and it is the contract both the Phase 2 fix and the future 0120
   cross-check depend on. Mergeable alone.

2. **Phase 2 — typed defaults + sentinel + breadcrumb (AC #1, #3, #4).** Add
   failing 0007 tests (numberless pr-review → `pr_number: unknown`; `result`-less
   plan-validation → `result: "unknown"`; design-inventory → typed `sequence: 1`/
   `screenshots_incomplete: true` plus quoted `source: "unknown"`; full run
   completes without abort), then the three coordinated production edits (typed
   defaults in `extra_default`; bare emission for them in the awk emitter; the
   `unknown` sentinel + `backfill-sentinel` breadcrumb in the loop's
   no-derivable-default branch), then reconcile the existing P4BC fixture and add
   the derivable-path no-regression assertion. Mergeable alone (does not depend on
   Phase 1's test existing).

---

## Phase 1: Validator guard tests for the `unknown` sentinel

### Overview

Add guard tests to the corpus-validator suite proving `name: unknown` is accepted
on both gates, for a `pr_number` extra and a non-`pr_number` extra (`result`). No
production code changes — this characterises and locks the already-true,
extra-agnostic contract (AC #2) so a future validator tightening (e.g. adding
numeric validation, or widening `EMPTY-PLACEHOLDER`) cannot silently break the
sentinel the Phase 2 fix relies on.

### Changes Required:

#### 1. Validator test suite

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: In the failure-mode/accept-side fixture section (alongside the
existing `EMPTY-PLACEHOLDER` fixture at `:136-137`), add two fixtures carrying the
bare sentinel and assert each is accepted and trips neither gate. `pr-description`
is anchored with required extras `pr_url pr_number merge_commit` and status vocab
`complete` (per `templates-schema.tsv` row 5); `pr_url`/`merge_commit` are
optional (skipped by `emit_valid`), so only `pr_number: "x"` is emitted and
rewritten. `plan-validation` is anchored with required extra `result` and status
`complete` (row 4), so only `result: "x"` is emitted and rewritten.

```bash
# name: unknown sentinel (the no-derivable-default backfill writes this; 0118) is
# a present, non-empty value: neither MISSING-EXTRA nor EMPTY-PLACEHOLDER. The
# validator's gates are extra-agnostic, so this holds for pr_number and result
# alike. Guards the contract migration 0007's backfill relies on.
emit_valid pr-description yes "pr_url pr_number merge_commit" "complete" \
  "$TMP/ok-pr-unknown.md"
sed -i.bak 's/^pr_number: "x"$/pr_number: unknown/' "$TMP/ok-pr-unknown.md"
assert_accepts "pr_number: unknown sentinel accepted" "$TMP/ok-pr-unknown.md"
assert_absent "pr_number: unknown is not MISSING-EXTRA" \
  "MISSING-EXTRA" "$TMP/ok-pr-unknown.md"
assert_absent "pr_number: unknown is not EMPTY-PLACEHOLDER" \
  "EMPTY-PLACEHOLDER" "$TMP/ok-pr-unknown.md"

# result is routed through the awk normaliser, so the migration writes it QUOTED
# (result: "unknown"); the guard mirrors that emitted form. (The validator
# accepts bare or quoted alike; pr_number above is the bare case.)
emit_valid plan-validation no "result" "complete" "$TMP/ok-result-unknown.md"
sed -i.bak 's/^result: "x"$/result: "unknown"/' "$TMP/ok-result-unknown.md"
assert_accepts "result: \"unknown\" sentinel accepted (extra-agnostic)" \
  "$TMP/ok-result-unknown.md"
assert_absent "result: \"unknown\" is not MISSING-EXTRA" \
  "MISSING-EXTRA" "$TMP/ok-result-unknown.md"
assert_absent "result: \"unknown\" is not EMPTY-PLACEHOLDER" \
  "EMPTY-PLACEHOLDER" "$TMP/ok-result-unknown.md"
```

(`assert_absent` is the existing rc-agnostic "this code is NOT emitted" helper,
`frontmatter-fixtures.sh:90`; `assert_accepts` asserts a clean rc==0, `:108`.)

### Success Criteria:

#### Automated Verification:

- [x] Validator suite passes, including the six new assertions:
      `bash scripts/test-validate-corpus-frontmatter.sh`
- [x] Shell component checks pass (shfmt + ShellCheck + bashisms over the changed
      test): `mise run scripts:check`

#### Manual Verification:

- [x] The fixtures carry `pr_number: unknown` (bare) and `result: "unknown"`
      (quoted) after the `sed` rewrite — mirroring the migration's per-extra
      emission — and the rewrites target `pr_number: "x"` / `result: "x"` as
      emitted by `emit_valid`.

---

## Phase 2: Typed defaults + `unknown` sentinel for underivable required extras

### Overview

Populate every required extra rather than leaving it absent: give the
numeric/boolean extras type-correct defaults in `extra_default` (emitted bare via
the awk emitter), and make the backfill loop's no-derivable-default branch
(`0007:507-510`) write the `unknown` sentinel + a counted
`0007-DIVERGE[backfill-sentinel]` breadcrumb for the remaining string/enum extras
instead of logging `missing-extra-no-default` and leaving the extra absent. The
live trigger is `pr_number` (string sentinel); `result`/`current_inventory`/etc.
share the sentinel branch, while `sequence`/`review_pass`/`screenshots_incomplete`
take the typed-default path. Drive it with failing 0007 tests first, then
reconcile the existing P4BC fixture (which currently asserts the now-gone
`missing-extra-no-default` breadcrumb and the now-false "no `pr_number:`" line) and
add the derivable-path no-regression assertion.

### Changes Required:

#### 1. Failing tests first (red)

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`

(a) **End-to-end via the runner, `pr_number` (AC #3).** Add a numberless pr-review
to the Phase 4 corpus (`P4`, generated before `run_0007 "$P4"` at `:1169-1175`) so
the existing `assert_eq "Phase 4 corpus exits 0" "0" "$RUN_RC"` (`:1176`) and
`assert_validates "Phase 4 corpus validates clean"` (`:1210`) become the AC #3
gate. Pre-fix, `self_validate_structural` aborts under the runner → `RUN_RC != 0`
(red); post-fix it exits 0 and validates.

```bash
# NODEFAULT: fenced pr-review whose stem carries no derivable PR number (no pr-
# token, date-prefixed stem) — exercises the no-derivable-default backfill
# end-to-end through the runner's self_validate_structural gate (0118).
cat >"$P4/meta/reviews/prs/2026-06-20-dateonly-pr-review.md" <<'EOF'
---
type: pr-review
id: "2026-06-20-dateonly-pr-review"
title: "Date Only PR Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Date Only PR Review
EOF
```

(b) **End-to-end via the runner, widening proof on a non-`pr_number` extra
(AC #3).** Add a `result`-less plan-validation to the same `P4` corpus so the
runner gate also exercises the widened branch on a different document type and
extra. Pre-fix this aborts too (the `result` extra is left absent →
`MISSING-EXTRA`); post-fix it is stamped `result: "unknown"` (quoted string) and
validates.

```bash
# WIDENING: fenced plan-validation missing its required `result` extra —
# exercises the no-derivable-default backfill for a non-pr_number, non-derivable
# extra, proving the loop-level fix covers every required extra (0118).
mkdir -p "$P4/meta/validations" # P4 setup only creates meta/reviews/prs etc.
cat >"$P4/meta/validations/2026-06-20-widening-validation.md" <<'EOF'
---
type: plan-validation
id: "2026-06-20-widening-validation"
title: "Widening Validation"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Widening Validation
EOF
```

Add, after the existing Phase 4 backfill assertions:

```bash
NODEF="$P4/meta/reviews/prs/2026-06-20-dateonly-pr-review.md"
assert_contains "Phase 4 NODEFAULT: underivable pr_number -> unknown sentinel" \
  "$(fm_line "$NODEF" pr_number)" 'pr_number: unknown'
WIDEN="$P4/meta/validations/2026-06-20-widening-validation.md"
assert_contains "Phase 4 WIDENING: underivable result -> unknown sentinel" \
  "$(fm_line "$WIDEN" result)" 'result: "unknown"'
```

**Emission note (quoted vs bare).** The awk emitter renders the sentinel
*quoted* for every scalar extra routed through its generic `else` path
(`0007-frontmatter-rewrite.awk:223`, which calls `fm_normalise_value`, wrapping
bare values in `"`): `result`, `current_inventory`, `target_inventory`, the
`design-inventory` bundle, `review_pass`, `priority` → `name: "unknown"`. Only
`pr_number` and `review_number` (the dedicated bare-print branch, `awk:222`) emit
unquoted: `pr_number: unknown`. The existing `verdict` sentinel test already
asserts the quoted form (`test-migrate-0007.sh:1183`, `verdict: "unknown"`). The
validator accepts both forms (neither gate inspects quoting), so this is a test-
assertion/expected-value detail, not a correctness issue in the migration —
assert the bare form for `pr_number`/`review_number` and the quoted form for
every other extra.

(c) **Direct-run reconciliation (AC #1).** The numberless `no-pr-number-review`
fixture (P4BC, `:1222-1250`) must now show `pr_number: unknown`, the
`backfill-sentinel` breadcrumb, and a completed run. Make these edits:

- **Replace** the `missing-extra-no-default` breadcrumb assertion (`:1243-1244`)
  with a `backfill-sentinel` assertion.
- **Remove** the now-false "no fabricated pr_number" assertion (`:1249-1250`) — a
  `pr_number: unknown` line is now written, so `assert_not_contains "pr_number:"`
  would fail.
- **Rewrite** the stale block comment (`:1218-1220`), which documents the old
  `missing-extra-no-default`/leave-absent behaviour.
- **Add** a no-abort + sentinel-written assertion.
- **Keep** the existing `backfilled-extra` breadcrumb assertion (`:1241-1242`,
  emitted by the awk emitter for the still-sentinel `verdict`/`lenses`) and the
  `review_number` backfill assertion (`:1247-1248`) — but **reword its label**,
  which currently reads "no mid-rewrite abort": post-fix the run always completes
  cleanly via the sentinel, so the old partial-mutation-before-abort framing no
  longer applies.

```bash
# Post-fix: pr_number gets the unknown sentinel via the loop's
# no-derivable-default branch, so NO extra is left absent and the migration
# completes without aborting at self_validate_structural. The loop emits a
# backfill-sentinel breadcrumb (not the removed missing-extra-no-default) (0118).
assert_eq "Phase 4 numberless review: direct run completes (no abort)" \
  "0" "$DIRECT_RC"
assert_contains "Phase 4 backfill-sentinel breadcrumb fired (pr_number)" \
  "$DIRECT_ERR" "0007-DIVERGE[backfill-sentinel]"
# Verify the per-file AND per-extra audit contract, not just the family tag (the
# Migration Notes recovery story depends on knowing which file AND which field).
assert_contains "Phase 4 backfill-sentinel breadcrumb names the file" \
  "$DIRECT_ERR" "no-pr-number-review.md"
assert_contains "Phase 4 backfill-sentinel breadcrumb names the extra" \
  "$DIRECT_ERR" "required extra 'pr_number'"
assert_not_contains "Phase 4 no missing-extra-no-default (reconciled)" \
  "$DIRECT_ERR" "missing-extra-no-default"
assert_contains "Phase 4 numberless review: pr_number -> unknown sentinel" \
  "$(fm_line "$P4BC/meta/reviews/prs/no-pr-number-review.md" pr_number)" \
  'pr_number: unknown'
```

(d) **No-regression on the derivable path (AC #4).** The existing PR430
assertion (`:1187-1188`) already proves a pr-token stem derives `pr_number: 430`
with no substitution. Add an explicit guard that the sentinel is *not* applied
when a number is derivable:

```bash
assert_not_contains "Phase 4 PR430: derivable pr_number NOT replaced by sentinel" \
  "$(fm_line "$PR430F" pr_number)" 'unknown'
```

(e) **Typed defaults, not string sentinel (hybrid proof).** Add a design-inventory
to the `P4` corpus missing all six of its required extras. The numeric/boolean
ones must receive type-correct *bare* defaults (not `"unknown"`), and the
string/enum ones must receive the quoted `unknown` sentinel — proving the hybrid
on one realistic document.

```bash
# HYBRID: design-inventory missing every required extra — sequence/
# screenshots_incomplete get typed bare defaults; the string/enum bundle
# (source/source_kind/source_location/crawler) gets the quoted unknown sentinel.
mkdir -p "$P4/meta/research/design-inventories"
cat >"$P4/meta/research/design-inventories/2026-06-20-hybrid-inventory.md" <<'EOF'
---
type: design-inventory
id: "2026-06-20-hybrid-inventory"
title: "Hybrid Inventory"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: draft
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Hybrid Inventory
EOF
```

```bash
INV="$P4/meta/research/design-inventories/2026-06-20-hybrid-inventory.md"
# Numeric/boolean typed defaults: assert the EXACT bare line (assert_eq, not a
# substring) so a quoted "1"/"true" type regression is caught — `sequence: 1`
# must not pass for `sequence: 10` either.
assert_eq "Phase 4 HYBRID: sequence -> bare typed default (not quoted)" \
  'sequence: 1' "$(fm_line "$INV" sequence)"
assert_eq "Phase 4 HYBRID: screenshots_incomplete -> bare bool (not quoted)" \
  'screenshots_incomplete: true' "$(fm_line "$INV" screenshots_incomplete)"
# String/enum bundle: quoted unknown sentinel (the whole bundle routes the same
# way, so pin all four to catch an extras_for_type/optional-carve-out regression).
assert_contains "Phase 4 HYBRID: source -> quoted unknown sentinel (string)" \
  "$(fm_line "$INV" source)" 'source: "unknown"'
assert_contains "Phase 4 HYBRID: source_kind -> quoted unknown sentinel" \
  "$(fm_line "$INV" source_kind)" 'source_kind: "unknown"'
assert_contains "Phase 4 HYBRID: source_location -> quoted unknown sentinel" \
  "$(fm_line "$INV" source_location)" 'source_location: "unknown"'
assert_contains "Phase 4 HYBRID: crawler -> quoted unknown sentinel" \
  "$(fm_line "$INV" crawler)" 'crawler: "unknown"'
```

(f) **Third typed default — `review_pass` (AC #3, hybrid completeness).** The
design-inventory exercises `sequence`/`screenshots_incomplete` but not
`review_pass` (forbidden on pr-review; required only on plan-/work-item-review).
Without a fixture, a regression dropping `review_pass` from the awk bare branch
(emitting the quoted string `"1"` — the exact coercion this fix prevents) would
pass undetected. Add a plan-review missing `review_pass`:

```bash
# REVIEWPASS: plan-review missing its required review_pass — exercises the third
# numeric typed default end-to-end (must emit bare review_pass: 1, not "1") (0118).
mkdir -p "$P4/meta/reviews/plans"
cat >"$P4/meta/reviews/plans/2026-06-20-reviewpass-review.md" <<'EOF'
---
type: plan-review
id: "2026-06-20-reviewpass-review"
title: "Review Pass Review"
date: "2026-06-20T00:00:00+00:00"
author: Toby
status: complete
tags: []
last_updated: "2026-06-20T00:00:00+00:00"
last_updated_by: Toby
schema_version: 1
---
# Review Pass Review
EOF
```

```bash
RP="$P4/meta/reviews/plans/2026-06-20-reviewpass-review.md"
assert_eq "Phase 4 REVIEWPASS: review_pass -> bare typed default (not quoted)" \
  'review_pass: 1' "$(fm_line "$RP" review_pass)"
```

(This plan-review also lacks `verdict`/`lenses` → they take their existing
`extra_default` sentinels (`verdict: "unknown"`, `lenses: ["unknown"]`), so the
fixture additionally re-confirms the pre-existing review sentinels alongside the
new `review_pass` typed default.)

#### 2. The production change (green)

The fix is a **hybrid**: type-correct defaults for the three required extras whose
natural type is numeric/boolean (so they are never coerced to a string), and the
`unknown` sentinel as the loop fallback for the string/enum extras that have no
derivable value. Three coordinated edits across two files.

**(2a) Typed defaults in `extra_default()`**
**File**:
`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: Add cases (alongside the existing `review_number) printf '1'` at
`0007:218`) for the numeric/boolean required extras, giving each a type-correct
default so it never reaches the loop's string-sentinel fallback:

```bash
    review_pass) printf '1' ;;                # parity with review_number
    sequence) printf '1' ;;                   # design-inventory ordinal default
    screenshots_incomplete) printf 'true' ;;  # conservative: don't claim
      # completeness for an inventory whose flag was never set
```

**(2b) Bare (type-correct) emission for those three in the awk emitter**
**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: The generic `else` path (`awk:223`) normalises (and therefore
*quotes*) scalar values, which would emit `sequence: "1"` (a string) instead of
`sequence: 1` (a number). Extend the existing bare-print branch (`awk:222`,
currently `pr_number`/`review_number`) to include the three typed extras so they
emit unquoted YAML scalars of the correct type:

```awk
    else if (bk == "pr_number" || bk == "review_number" ||
             bk == "sequence" || bk == "review_pass" ||
             bk == "screenshots_incomplete") print bk ": " bv
```

Break *after* the trailing `||` (no backslash) — awk treats a line ending in a
binary operator as an implicit continuation. This matches the file's existing
multi-condition idiom (`is_linkage_key`, `awk:62-64`) and is the one-true-awk
(macOS BWK awk) safe form; the codebase uses **no** backslash-newline
continuations, and the single-line form would be ~150 cols (over the 80 floor).
This keeps `sequence: 1`/`review_pass: 1` as YAML numbers and
`screenshots_incomplete: true` as a YAML boolean — no type coercion. (Update the
adjacent hard-coded-cardinality comment at `awk:209-211` to note these are bare
*typed* scalars, not just `lenses`-is-the-only-list.)

**(2c) `unknown` sentinel fallback in the backfill loop**
**File**:
`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: In the no-derivable-default branch (`0007:507-510`), replace the
`missing-extra-no-default` log + `continue` with a sentinel assignment +
`backfill-sentinel` breadcrumb, then fall through to the existing pack step
(`0007:511`, `backfill_extras="…${ex}=${dv}"` — left untouched). After (2a) this
branch is reached only by the *string/enum* extras with no derivation
(`pr_number`, `result`, `priority`, `current_inventory`, `target_inventory`,
`source`, `source_kind`, `source_location`, `crawler`), for which `unknown` is a
string → no type coercion.

```bash
    if [ -z "$dv" ]; then
      # Underivable string/enum required extra → write the `unknown` sentinel
      # (parity with the verdict/lenses contract) so self_validate_structural
      # sees a present value rather than aborting on MISSING-EXTRA; breadcrumb
      # each stamped file so the sentinel write is auditable, not silent (0118).
      # Numeric/boolean extras never reach here — they get typed defaults in
      # extra_default (2a). See Migration Notes for the dual sentinel-source
      # rationale.
      dv='unknown'
      log_warn "0007-DIVERGE[backfill-sentinel]: $f — required extra '$ex' has no derivable default; stamped 'unknown'" >&2
    fi
    backfill_extras="${backfill_extras:+$backfill_extras$US}${ex}=${dv}"
```

All edits are `bash 3.2`/POSIX-awk-safe (no bashisms; single-character awk
separators preserved). The backfill loop's `fm_is_empty_val` guard (`0007:504`)
still skips files that *already* carry a real value, so the derivable/present
paths are unaffected, and a re-run skips the now-present value (idempotent). The
`extra_default` catch-all `*) printf ''` (`0007:221`) and the `pr_number` case's
empty return are intentionally left as-is — the loop remains the single authority
for the string/enum sentinel, while `extra_default` owns every typed/derivable
default, so no per-case duplication is introduced.

### Success Criteria:

#### Automated Verification:

- [x] 0007 suite passes, including the new/updated assertions:
      `bash skills/config/migrate/scripts/test-migrate-0007.sh`
- [x] No regression in the broader migrate suites:
      `bash skills/config/migrate/scripts/test-migrate.sh`,
      `bash skills/config/migrate/scripts/test-migrate-snapshot.sh`,
      `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [x] Validator suite still passes:
      `bash scripts/test-validate-corpus-frontmatter.sh`
- [x] Shell component checks pass (shfmt + ShellCheck + bashisms over the changed
      migration + test): `mise run scripts:check`
- [x] Full local CI mirror is green: `mise run`

#### Manual Verification:

- [x] String/enum sentinels are emitted as strings: `pr_number: unknown` bare
      (dedicated `awk:222` branch); `result: "unknown"` / `source: "unknown"`
      quoted (normalised `awk:223` path). Both parse as YAML strings and pass the
      validator.
- [x] Typed defaults are emitted with the correct YAML type, NOT as strings:
      `sequence: 1` / `review_pass: 1` as numbers, `screenshots_incomplete: true`
      as a boolean (all bare, via the extended `awk:222` branch) — confirm none
      is quoted (`"1"` / `"true"` would be a type regression the visualiser would
      propagate).
- [x] The `0007-DIVERGE[backfill-sentinel]` breadcrumb names each stamped file in
      the migration's stderr/DIVERGE output, providing the audit trail for
      manual reconciliation. (Only string/enum sentinels breadcrumb; typed
      defaults do not, matching `review_number`.)
- [x] Confirm no *list-valued* required extra can reach the no-derivable-default
      branch in the current schema — inspect `templates-schema.tsv` col 4 against
      the awk emitter's hard-coded `lenses` list case (`awk:209-211,220`):
      `lenses` already has a sentinel and never reaches the branch; all other
      reachable required extras are scalars (string/enum → sentinel; numeric/bool
      → typed default). (A future list-valued required extra added without a
      derivation would emit a malformed scalar `unknown`; the HYBRID-style test
      for that extra would surface it.)

---

## Testing Strategy

### Unit / suite tests:

- **Validator suite** (`scripts/test-validate-corpus-frontmatter.sh`): the
  sentinel-accepted guards (Phase 1) for `pr_number` and `result` — two
  `assert_accepts` + four `assert_absent`, proving the contract is extra-agnostic.
- **0007 suite** (`skills/config/migrate/scripts/test-migrate-0007.sh`):
  - End-to-end via runner, `pr_number`: numberless pr-review in the P4 corpus →
    `RUN_RC == 0` + `assert_validates` + `pr_number: unknown` (AC #1, #3).
  - End-to-end via runner, string sentinel: `result`-less plan-validation in P4 →
    `result: "unknown"` (quoted), proving the loop fix covers a non-`pr_number`
    string extra on a different document type (AC #3, generalised).
  - End-to-end via runner, hybrid: design-inventory missing its whole bundle →
    `sequence: 1` / `screenshots_incomplete: true` (bare typed defaults, asserted
    by exact `assert_eq`) plus `source`/`source_kind`/`source_location`/`crawler`
    → `"unknown"` (quoted sentinel), proving typed-vs-sentinel routing.
  - End-to-end via runner, third typed default: plan-review missing `review_pass`
    → bare `review_pass: 1` (exact `assert_eq`), closing the gap that the
    design-inventory fixture does not reach `review_pass`.
  - Direct run: P4BC reconciled — `DIRECT_RC == 0`, `backfill-sentinel`
    breadcrumb fired and names the file, no `missing-extra-no-default`,
    `pr_number: unknown` written, the old "no `pr_number:`" assertion removed;
    `backfilled-extra` (verdict/lenses) breadcrumb and `review_number` backfill
    retained.
  - No-regression: PR430 derives `pr_number: 430`, not `unknown` (AC #4).
  - Idempotency: the existing Phase 4 second-run empty-diff assertion (`:1215`)
    now also covers the three new sentinel-/default-bearing files.

### Key edge cases:

- Date-prefixed stem (`YYYY-MM-DD-...`): reaches the no-fallback branch →
  `pr_number: unknown` (exercised by the NODEFAULT and P4BC fixtures).
- Non-`pr_number` string/enum extra with no derivation (`result`): reaches the
  catch-all → quoted `result: "unknown"` (exercised by the WIDENING fixture).
- Numeric/boolean extra with no value (`sequence`, `screenshots_incomplete`):
  takes the typed-default path → bare `sequence: 1` / `screenshots_incomplete:
  true`, NOT the sentinel (exercised by the HYBRID design-inventory fixture).
- PR-token stem (`pr-430`, `PR-430`) and bare-leading-number stem (`240-...`):
  derive the real number → no substitution (PR430 no-regression).
- Tracker-keyed stem (`<TRACKER>-NNNN-...`, e.g. `JIRA-1234-foo`): no pr-token and
  no leading digit → empty default → `unknown`. (Covered behaviourally by the
  date-prefixed case through the same branch; a dedicated fixture is optional.)

### Manual Testing Steps:

1. `bash skills/config/migrate/scripts/test-migrate-0007.sh` — confirm the new
   assertions pass and the suite is green.
2. Inspect migrated fixtures (transiently, in a scratch run): confirm
   `pr_number: unknown` is bare and `result: "unknown"` is quoted (both strings);
   confirm `sequence: 1` / `screenshots_incomplete: true` are bare typed scalars
   (not quoted); and confirm the `backfill-sentinel` breadcrumb names the
   sentinel-stamped files.
3. `mise run` — confirm the full CI mirror is green.

## Performance Considerations

None. The change is, per underivable required extra during an already-O(files)
backfill pass, a constant-time typed-default lookup or a single conditional
assignment plus a log line.

## Migration Notes

**Type-correct defaults (no coercion).** The numeric/boolean required extras
(`review_pass`, `sequence`, `screenshots_incomplete`) get a typed default
(`1`/`1`/`true`) emitted as a bare YAML number/boolean, *not* the string
`unknown`. This deliberately avoids a type change: the visualiser parses
frontmatter into typed JSON (YAML `Number`→JSON number, `Bool`→JSON boolean), so
stamping these fields with the string `"unknown"` would have silently changed
their type for every downstream consumer. `extra_default` is the authority for
these typed defaults; the loop sentinel never touches them. `screenshots_incomplete`
defaults to `true` (conservative — an inventory whose flag was never set is treated
as *not* vouched complete) rather than the template's optimistic `false`.

Caveat on `sequence: 1`: unlike the string sentinels, the typed defaults carry no
breadcrumb (parity with `review_number`), and `sequence` is the design-gap
resolver's *primary* tiebreaker among non-superseded duplicate inventories. A
backfilled `sequence: 1` is therefore a fabricated ordinal that could tie with a
real `1`; the resolver degrades safely (it falls back to mtime then directory
date for equal-`sequence` cases) rather than failing, and only pre-existing
inventories that already lacked the field are affected. Operators reconciling a
multi-inventory design area should treat a backfilled `sequence: 1` as
unauthoritative.

**Sticky sentinel.** For the string/enum extras, `unknown` is a persisted corpus
value, and it is **sticky-by-design**: once written, the backfill's
`fm_is_empty_val` guard (`0007:504`) treats it as present-and-non-empty, so a
re-run — or any later migration — skips the file and never re-derives a real
value. For a file that genuinely belongs to a real PR (or has a real `result`,
etc.) but whose value is not derivable from its filename, the true value is
therefore *not* recoverable by re-running the migration; reconciliation is manual.
The `backfill-sentinel` breadcrumb is the audit trail for that manual step — it
names every stamped file on stderr/in the DIVERGE log. Since these are all
string-typed fields, `unknown` is a string → no type change; this matches the
existing `verdict`/`lenses` sentinel contract (those carry no per-file warning
because they have no derivation at all; the loop-caught extras get the breadcrumb
precisely because they represent a *failed* derivation worth surfacing), so no new
downstream consumer handling is introduced.

Note that some sentinel'd fields are *enums* (`priority` → high/medium/low;
`result` → pass/partial/fail; `source_kind`, `crawler`), so `unknown` is a valid
string but an out-of-vocabulary enum value. The corpus validator is enum-agnostic
(it checks only presence and literal-emptiness), so this passes the migration
gate — but a downstream consumer that switches on the enum must have a default
arm rather than crash/drop on an unrecognised token. The visualiser renders
frontmatter generically (string passthrough), so it tolerates `unknown`; the
breadcrumb flags each occurrence for manual reconciliation. Any future
enum-switching consumer should be confirmed to degrade gracefully on `unknown`.

**Gate semantics.** `self_validate_structural` previously *aborted* on a missing
required extra; after this change the same corpus state passes (the extra is
present as `unknown`). The gate is not disabled — it still fires for any state the
sentinel does not cover — but a "required value could not be derived" signal is
converted from a hard stop into a breadcrumbed sentinel. This is the intended
behaviour (parity with `verdict`/`lenses`), with the breadcrumb preserving the
audit trail the hard stop used to provide.

**Concurrent-edit coupling with 0114.** This change now edits the backfill loop's
no-derivable-default branch (`0007:507-510`) directly — the **same region** 0114
operates in (`0007:502-512`). 0114 is complete and did not touch this branch, so
there is no live conflict today; but the coupling is now direct (not, as the
original draft stated, confined to `extra_default`). If 0114 is ever reopened,
reconcile its changes with this branch's sentinel-write + breadcrumb.

**Work-item / sibling reconciliation (scope change).** The parent work item 0118
and siblings 0114/0120 were written around the narrower "option A" (fix
`pr_number` in `extra_default`). This plan (a) widens the fix to the loop branch
so the backfill-vs-validator contradiction is resolved for every required extra,
and (b) adds type-correct defaults for the three numeric/boolean extras — a small
slice of 0114's "derivation completeness" remit pulled in to avoid type coercion.
Work item 0118 has been updated to record this scope (summary, requirements, two
new acceptance criteria, technical notes). 0120's prevention-cross-check scope
should still be re-confirmed: its invariant ("the backfill never emits a state the
validator rejects") is now structurally upheld by the loop branch + typed
defaults rather than needing a separate per-extra guard; what remains for 0120 is
the forward guard against a *future* list-valued required extra (which the awk
path would emit malformed) or a future validator-tightening — the cases the loop
branch alone cannot catch.

## References

- Original work item:
  `meta/work/0118-reconcile-0007-backfill-sentinel-with-validator.md`
- Related research:
  `meta/research/codebase/2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator.md`
- Source RCA:
  `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
  (fix option C)
- Plan review (drove the widening + breadcrumb decisions):
  `meta/reviews/plans/2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator-review-1.md`
- Sentinel parity + typed-default site (`review_number) printf '1'`):
  `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:218-220`
  (add `review_pass`/`sequence`/`screenshots_incomplete` cases here — edit 2a)
- No-derivable-default branch (sentinel edit site — edit 2c):
  `…0007-unify-meta-corpus-frontmatter.sh:507-510` (falls through to pack at `:511`)
- Backfill loop + generic pack channel:
  `…0007-unify-meta-corpus-frontmatter.sh:502-512`
- Self-validation gate: `…0007-unify-meta-corpus-frontmatter.sh:771` (block
  `:747-786`)
- Generic awk emit loop — bare branch (`:222`, extend for the typed extras —
  edit 2b) and normalised/quoted scalar path (`:223`):
  `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:203-225`
- Validator gates: `scripts/validate-corpus-frontmatter.sh:343-346`
  (`MISSING-EXTRA`), `:354-356` (`EMPTY-PLACEHOLDER`)
- Schema (required extras per type): `scripts/templates-schema.tsv` (col 4);
  optional carve-out `FM_OPTIONAL_EXTRAS` in
  `scripts/frontmatter-emission-rules.sh:74`
- Existing fixture to reconcile:
  `skills/config/migrate/scripts/test-migrate-0007.sh:1218-1250`
- Related work items: 0115 (parent epic), 0114 (backfill completeness), 0120
  (prevention cross-check)
