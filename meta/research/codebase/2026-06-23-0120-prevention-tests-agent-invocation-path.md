---
type: codebase-research
id: "2026-06-23-0120-prevention-tests-agent-invocation-path"
title: "Research: Prevention Tests for the Agent-Invocation Path (0120)"
date: "2026-06-23T00:15:59+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0120"
parent: "work-item:0120"
topic: "Prevention tests for the agent-invocation path: the no-input interactive stall and the 0007 backfill-vs-validator cross-check"
tags: [research, codebase, migrate, interactive-migration, agent-invocation, testing, 0007, validation]
revision: "4f4dd27e6594f14c37936a49e138202c0518364b"
repository: "accelerator"
last_updated: "2026-06-23T00:15:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Prevention Tests for the Agent-Invocation Path (0120)

**Date**: 2026-06-23 00:15 UTC
**Author**: Toby Clemson
**Git Commit**: 4f4dd27e6594f14c37936a49e138202c0518364b
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What does the codebase look like today for implementing work item 0120 —
"Prevention Tests for the Agent-Invocation Path"? The item asks for two new
tests: (1) a no-TTY / no-decisions-file interactive-migration test asserting
0116's structured stall, and (2) an end-to-end cross-check in the 0007 suite
that a tolerant `unknown` backfill is a state the validator accepts. Where do
the assertion targets live, what are their exact shapes, and — critically — how
much of each acceptance criterion is *already covered* by the tests that 0116
and 0118 shipped alongside their fixes?

## Summary

**The headline finding: AC1 is substantially already implemented, AC2 is a
genuine gap.** 0120 was authored while 0116 and 0118 were still open blockers;
both have since landed (status: done) and each shipped its *own* tests. As a
result:

- **AC1 (no-input structured stall)** is already covered by a test 0116 shipped
  — `test-migrate-interactive.sh:1195-1215`, in a section literally titled
  `=== Structured stall on no decision input (0116 Phase 2) ===`. That test
  drives the driver with `</dev/null` (no TTY, fd 0 at EOF), no decisions file,
  and asserts every clause 0120's AC1 enumerates: non-zero exit, the literal key
  `k1`, the `ACCELERATOR_MIGRATE_DECISIONS_FILE=` resume form, the migration id
  `0002-predicate` in the resume path, `run-migrations.sh` as driver, and the
  absence of `failed to obtain decision`. The work item's premise for AC1 ("the
  no-input branch … is never exercised") was **true when authored but is now
  stale.**
- **AC2 (0007 backfill↔validator cross-check)** has a real residual gap. The
  0007 suite already proves the `unknown` sentinel end-to-end (`run_0007` →
  exit 0 → `assert_validates` clean), but **only for `pr-review` fixtures with
  date-only / numberless stems** (`test-migrate-0007.sh:1172-1186`,
  `:1339-1353`). No existing fixture is the exact shape 0120's AC2 mandates: a
  **`pr-description`** file (type `pr-description`, under `meta/prs/`) whose
  filename carries an **external tracker key** (`<TRACKER>-NNNN-description.md`).
  That is precisely the file shape from the original incident, and it is not yet
  tested.

A second, load-bearing finding for AC2: the regex the work item names —
`FAIL:.*MISSING-EXTRA` — **cannot match any single validator output line**. The
validator emits per-violation lines as `<file>: MISSING-EXTRA — <msg>` (no
`FAIL:` prefix) and a separate trailing summary `FAIL: N frontmatter
violation(s)` (no violation code). A literal `grep -E 'FAIL:.*MISSING-EXTRA'`
would therefore be vacuously satisfied. The meaningful assertion is "exit 0 and
no `MISSING-EXTRA` token in output" — the implementer should not encode the
regex literally without understanding this.

Adding a test inside either suite needs **no count/floor bookkeeping**: CI has
no per-suite test-count floor — only an at-least-4 *suite-file* count for the
`skills/config/migrate` subtree (`tasks/test/integration.py:8,138-147`). A new
assertion passes CI iff it passes and the suite file stays executable.

## Detailed Findings

### Area 1 — The structured stall (AC1's assertion target)

The stall is produced by `emit_no_input_stall()` in the sourced library
`skills/config/migrate/scripts/interactive-lib.sh:313-346`, **not** by
`run-migrations.sh`'s dirty-tree pre-flight (those branches mention
`--decisions-file` only in prose and do not emit the `ACCELERATOR_MIGRATE_DECISIONS_FILE=`
assignment AC1(b) wants).

Path to the stall:
- `run_interactive_migration` PROMPT handler calls
  `read_decision_or_stall "$id" "$p_key" decision`
  (`interactive-lib.sh:843`).
- `read_decision()` chooses its input source; the bare fd-0 branch is at
  `interactive-lib.sh:270-280`, returning status **2** only when the read fails
  *and* `line` is empty — genuine EOF with no channel
  (`if ! IFS= read -r line && [ -z "$line" ]; then return 2`, `:277-279`).
  (Note: the work item cites `interactive-lib.sh:262` for the bare read; the
  current line is **277** — `:262` is now inside the decisions-file blank-skip
  loop. The file shifted since the item was written.)
- `read_decision_or_stall` (`:352-363`) maps status 2 → `emit_no_input_stall`,
  status 1 → the legacy `failed to obtain $verb for $key`.

What `emit_no_input_stall` prints (all to stderr, block-level `} >&2` at
`:345`), with `id`=migration id, `key`=pending key:
- `[$id] MIGRATION STALLED: no decision input available` (`:322`)
- `[$id]   pending decision: $key` (`:323`)
- Flush-left, copy-pasteable resume lines:
  - `bash $driver --decisions-file $decisions_path` (`:340`)
  - `ACCELERATOR_MIGRATE_DECISIONS_FILE=$decisions_path bash $driver` (`:344`)
- where `decisions_path` = `$PROJECT_ROOT/.accelerator/state/migrations-${id}-decisions.txt`
  (`:318-319`, the id appears literally) and `driver` =
  `${RUNNER_SCRIPT_DIR:-.}/run-migrations.sh` (`:320`).

The bare string `failed to obtain decision` no longer exists in runner code
(only the parameterised `failed to obtain $verb for $key` at `:359`, and
comments at `:309,:350`). The pending key is `fields[0]` of the first PROMPT
frame (`:837`), passed straight through as `$key` — so a fixture's first PROMPT
key is knowable in advance.

### Area 2 — AC1 is already covered (the overlap)

`skills/config/migrate/scripts/test-migrate-interactive.sh:1192-1215`
(section `=== Structured stall on no decision input (0116 Phase 2) ===`):

```
seed_predicate_sandbox "$SBX" "k1|f1|a1|v1|ambiguous|prose1"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR=".../0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" </dev/null 2>&1) || RC=$?
assert_neq "non-zero exit on no-input stall" "0" "$RC"          # AC1 head
assert_contains "stall marker present" "$OUTPUT" "MIGRATION STALLED"
assert_contains "names the current key" "$OUTPUT" "k1"          # AC1(a)
assert_contains "resume switch form" "$OUTPUT" "--decisions-file"
assert_contains "resume names the driver" "$OUTPUT" "run-migrations.sh"  # AC1(b)
assert_contains "resume env-var form" "$OUTPUT" "ACCELERATOR_MIGRATE_DECISIONS_FILE="  # AC1(b)
assert_contains "migration id in resume path" "$OUTPUT" "0002-predicate"  # AC1(b)
assert_not_contains "old opaque message gone" "$OUTPUT" "failed to obtain decision"  # AC1(c)
assert_not_contains "no shell errors on stall path" "$OUTPUT" "unbound variable"
```

This maps onto **every** AC1 clause: non-zero exit, (a) literal key `k1`,
(b) all three resume-command facets, (c) absence of `failed to obtain decision`.
The companion test at `:1218-1239` covers the VALIDATE_ERR re-prompt no-input
stall.

Caveats / possible residual distinctions for the implementer to weigh:
- The existing test uses `ACCELERATOR_MIGRATE_FORCE=1`. AC1/AC3 neither require
  nor forbid FORCE; AC3 only requires fd 0 closed/`/dev/null` and
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` unset (both true here). FORCE is also
  faithful to the original incident (the tree was dirty from earlier
  migrations by the time 0007 prompted).
- AC1(a) wants a key "the test fixture fixes and knows in advance (not merely
  some non-empty token)." `k1` qualifies (it is the first field of the single
  seeded `ambiguous` row, `seed_predicate_sandbox … "k1|…|ambiguous|…"`), though
  it is a short token. A dedicated 0120 test could use a more distinctive key to
  make the substring assertion unambiguous.
- The work item explicitly frames this as "the agent-invocation path." The
  existing test is functionally that path but is not labelled as such.

**Decision needed:** treat AC1 as satisfied-by-0116 (and at most harden/relabel
the existing test), or author a deliberately distinct 0120 test. See Open
Questions.

### Area 3 — The 0007 backfill `unknown` sentinel (AC2's behaviour)

`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`:
- `extra_default()` (`:193-230`); the `pr_number` arm (`:201-217`) derives the
  number from a genuine `pr`/`PR` stem segment (`(^|-)[Pp][Rr]-?[0-9]+`) or a
  leading numeric stem (excluding date-prefixed stems). A tracker-key stem like
  `ENG-1234-…` matches **neither** → returns empty.
- The required-extras backfill loop sentinel branch (`:514-523`):
  ```
  if [ -z "$dv" ]; then
    dv='unknown'
    log_warn "0007-DIVERGE[backfill-sentinel]: $f — required extra '$ex' has no derivable default; stamped 'unknown'" >&2
  fi
  ```
  So an underivable required extra is stamped `unknown` (0118's reconciliation),
  not left absent. The old `0007-DIVERGE[missing-extra-no-default] … left absent`
  behaviour is gone.
- Test seam `ACCELERATOR_0007_NO_RUN=1` (`:755`) returns before the
  orchestration block, letting tests source the file and call helpers
  (`extra_default`, `extras_for_type`, …) in isolation.
- `self_validate_structural` (`:567-579`, called at `:784`) runs the corpus
  validator over in-scope files; a non-zero exit fails the migration. With the
  sentinel present, the validator passes (see Area 4).

The schema that makes `pr_number` required lives in
`scripts/templates-schema.tsv`: `pr-description` (line 5, extras
`pr_url pr_number merge_commit`) and `pr-review` (line 13). `pr_number` is **not**
in `FM_OPTIONAL_EXTRAS` (`scripts/frontmatter-emission-rules.sh:74`), so it is a
genuinely required extra for both. Type is path-inferred:
`pr-description` ← files directly under `meta/prs/`;
`pr-review` ← `meta/reviews/prs/` (longest-dir-wins), per
`scripts/config-defaults.sh` and `scripts/doc-type-inference.sh`. **The AC2
fixture must live directly under `meta/prs/`, not `meta/reviews/prs/`.**

### Area 4 — The validator accepts `unknown` (AC2's "accepted state")

`scripts/validate-corpus-frontmatter.sh`:
- MISSING-EXTRA (`:342-346`): fires iff a required extra key is *absent*
  (`bk_present "$f"` false). A present `unknown` passes.
- EMPTY-PLACEHOLDER (`:348-359`): walks every key; the predicate is
  `case "$ev" in '""' | '[]')` (`:354-355`) — it rejects **only** the literal
  two-char tokens `""` and `[]`. The 7-char token `unknown` matches neither, so
  it is accepted.
- Validator-own tests already prove this:
  `scripts/test-validate-corpus-frontmatter.sh:145-149`
  (`pr_number: unknown sentinel accepted`, `… is not MISSING-EXTRA`).

**Output-format gotcha for AC2's regex.** `violation()` (`:63-66`) prints
`printf '%s: %s — %s\n'` → e.g.
`meta/prs/ENG-1234-description.md: MISSING-EXTRA — required extra 'pr_number' absent`
(note the em-dash `—`, U+2014). The only line starting with `FAIL:` is the
summary `FAIL: N frontmatter violation(s)` (`:433-436`), which carries no
violation code. Therefore `FAIL:.*MISSING-EXTRA` matches **nothing on a single
line**. The robust assertion is: migration exits 0 *and* output contains no
`MISSING-EXTRA` token (equivalently, `assert_validates` over `$P/meta` passes).
Standalone CLI: `validate-corpus-frontmatter.sh <file>…` (file-list mode, exit 1
on violations, exit 0 clean, exit 2 on usage error).

### Area 5 — Existing 0007 coverage and the AC2 gap

`skills/config/migrate/scripts/test-migrate-0007.sh` already exercises the
sentinel end-to-end, but with the wrong fixture *shape* for AC2:
- NODEFAULT (`:1169-1186`): `meta/reviews/prs/2026-06-20-dateonly-pr-review.md`
  — a **`pr-review`**, **date-only** stem. `run_0007` full run → `assert_eq
  "Phase 4 corpus exits 0"` (`:1254`) → `assert_contains … 'pr_number: unknown'`
  (`:1289-1290`) → `assert_validates "Phase 4 corpus validates clean"` (`:1323`).
- Direct-run breadcrumb (`:1339-1377`): `meta/reviews/prs/no-pr-number-review.md`
  — a numberless **`pr-review`** via `run_0007_direct`, asserting the
  `0007-DIVERGE[backfill-sentinel]` breadcrumb names the file and the
  `pr_number` extra, exit 0, and the absence of `missing-extra-no-default`.
- Derivable no-regression (`:1264-1298`): a `pr-token` stem still derives the
  real number and is **not** sentinel-replaced.

What's **missing** for AC2: a fixture of the *incident* shape — a
**`pr-description`** (under `meta/prs/`) named with an **external tracker key**
(`<TRACKER>-NNNN-description.md`, e.g. `ENG-1234-description.md`) so `pr_number`
is underivable, run through `run_0007` end-to-end, asserting exit 0, no
`MISSING-EXTRA`, and `pr_number: unknown` (note: `pr_number` stamps the **bare**
`unknown`, per the existing assertion `'pr_number: unknown'` at `:1290`, not the
quoted `"unknown"` used for string/enum extras like `verdict`/`source`).

### Area 6 — Test harness, fixtures, and CI floor mechanics

- **Harness**: both suites source `scripts/test-helpers.sh` (counters `PASS`/
  `FAIL`/`SKIP` at `:16-18`; `test_summary` at `:371-381` returns 1 iff
  `FAIL>0`). Tests are linear assert calls — no registration, no per-test
  manifest.
- **Assertion helpers**: `assert_eq`/`assert_neq` (exit codes via captured
  `$RC`), `assert_contains`/`assert_not_contains` (`grep -qF` substring).
  The 0007 suite adds local `assert_validates` (`:61-74`, validator exit 0) and
  `assert_violation` (`:81-94`, non-zero + `grep -qF` of a code).
- **Capture idiom**: `OUTPUT=$(… bash "$DRIVER" 2>&1) || RC=$?`.
- **Fixing a known PROMPT key**: `0002-predicate` fixture reads
  `key|path|anchor|proposed|band|prose` rows from `$PROJECT_ROOT/.fixture/
  transformations`; a row with band `ambiguous` routes to PROMPT.
  `seed_predicate_sandbox "$SBX" "k1|…|ambiguous|…"` pins one PROMPT for key
  `k1` (suite `:360-369`).
- **No-input idiom**: `</dev/null` (the established pattern at `:1203`); there is
  no `0<&-` fd-close anywhere in the suite.
- **0007 full-run helper** `run_0007()` (`:37-42`): `ACCELERATOR_MIGRATIONS_DIR`
  → a temp dir holding only 0007, `ACCELERATOR_MIGRATE_FORCE=1`,
  `bash "$DRIVER" … </dev/null`. Fixtures are heredoc'd into per-test `mktemp -d`
  repos and `git_init`'d.
- **CI floor**: `tasks/test/integration.py` enforces only a *suite-file* count
  floor per subtree — `_EXPECTED_MIGRATE_SUITES = 4` (`:8`), checked in the
  `migrate` task (`:138-147`) via `run_shell_suites(context,
  "skills/config/migrate")`. Both target suites are among those 4. **There is no
  numeric per-suite test-count floor** — "the floor that suite already asserts in
  CI" (AC3) resolves to "the suite still exists/executes and exits 0," which is
  automatically satisfied by adding passing assertions.

## Code References

- `skills/config/migrate/scripts/interactive-lib.sh:313-346` — `emit_no_input_stall`; exact stall strings + resume command (AC1 target)
- `skills/config/migrate/scripts/interactive-lib.sh:270-280` — bare fd-0 read, status-2 (no-input) return; the branch AC1 must traverse (work item's `:262` is now `:277`)
- `skills/config/migrate/scripts/interactive-lib.sh:352-363` — `read_decision_or_stall`; status-2 → stall, status-1 → legacy abort
- `skills/config/migrate/scripts/test-migrate-interactive.sh:1192-1215` — **existing** no-input stall test (covers AC1)
- `skills/config/migrate/scripts/test-migrate-interactive.sh:1218-1239` — VALIDATE_ERR re-prompt no-input stall test
- `skills/config/migrate/scripts/test-migrate-interactive.sh:360-369` — `seed_predicate_sandbox`; pins the first PROMPT key
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:201-217` — `extra_default` `pr_number` arm (underivable for tracker keys)
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:514-523` — `unknown` sentinel backfill branch
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:567-579,784` — `self_validate_structural` invocation
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:755` — `ACCELERATOR_0007_NO_RUN=1` test seam
- `scripts/validate-corpus-frontmatter.sh:342-346` — MISSING-EXTRA presence check
- `scripts/validate-corpus-frontmatter.sh:348-359` — EMPTY-PLACEHOLDER (`""`/`[]` only; accepts `unknown`)
- `scripts/validate-corpus-frontmatter.sh:63-66,433-436` — violation-line vs `FAIL:` summary format (the regex gotcha)
- `scripts/templates-schema.tsv:5,13` — `pr-description`/`pr-review` require `pr_number`
- `scripts/frontmatter-emission-rules.sh:74` — `FM_OPTIONAL_EXTRAS` (excludes `pr_number`)
- `scripts/test-validate-corpus-frontmatter.sh:145-149` — validator already accepts `pr_number: unknown`
- `skills/config/migrate/scripts/test-migrate-0007.sh:1169-1186,1288-1298` — NODEFAULT date-only `pr-review` sentinel coverage (the closest existing analogue to AC2)
- `skills/config/migrate/scripts/test-migrate-0007.sh:37-42,61-94` — `run_0007`, `assert_validates`, `assert_violation`
- `tasks/test/integration.py:8,138-147` — `_EXPECTED_MIGRATE_SUITES = 4` suite-file floor (no per-test floor)

## Architecture Insights

- **The producer of the resume command is the library, not the driver.** AC1(b)'s
  `ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>` line is only ever emitted by
  `emit_no_input_stall` on the live interactive stall (read status 2). The
  dirty-tree pre-flight in `run-migrations.sh` emits a *different*, prose-only
  steer. A test asserting the env-var form must exercise the live stall, not the
  pre-flight.
- **Sentinel quoting is type-dependent.** `pr_number` (numeric-ish) stamps **bare**
  `unknown`; string/enum extras (`verdict`, `lenses`, `source`, …) stamp quoted
  `"unknown"`/`["unknown"]`. AC2's `pr_number` fixture should assert the bare form.
- **Path-based type inference is longest-dir-wins.** `meta/prs/` →
  `pr-description`; `meta/reviews/prs/` → `pr-review`. The AC2 fixture's directory
  choice is load-bearing for which extra set applies.
- **"Tolerated state" is now a present sentinel, not an absence.** 0118 turned a
  fail-closed absence into a benign present value the validator accepts — the
  cross-check guards exactly the contract "what the backfill leaves == what the
  validator accepts."
- **Prevention tests partly pre-shipped with their fixes.** Because 0116 and 0118
  each shipped tests for their own behaviour, 0120's value concentrates in (a) the
  *incident-shaped* `pr-description`/tracker-key cross-check that no prior test
  used, and (b) optionally consolidating/relabelling the no-input coverage as the
  explicit "agent-invocation path" regression. The item's AC1 premise predates
  0116 landing and is now partly stale.

## Historical Context

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
  — the RCA. Confirms Hypotheses 1 (no input channel) and 2 (0007
  self-validation contradicts its tolerant backfill); the triggering files were
  "PR-description files whose names carry an external tracker key." Its
  "Prevention" section is the direct source of 0120's two tests, including
  "the protocol-via-decisions-file tests proved the wrong thing."
- `meta/plans/2026-06-20-0116-structured-stall-on-no-decision-input.md` and
  `meta/validations/2026-06-20-0116-…-validation.md` — 0116 (done) shipped the
  stall **and** the "Phase 2" no-input tests now in `test-migrate-interactive.sh`.
- `meta/plans/2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator.md`
  and its validation — 0118 (done) shipped the `unknown` sentinel and the
  NODEFAULT/breadcrumb `pr-review` tests now in `test-migrate-0007.sh`.
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
  and `ADR-0038-interactive-validation-parameters-….md` — the interactive
  contract and validation parameters these tests guard.
- Sibling work items (`meta/work/`): 0115 (umbrella, ready), 0116/0117/0118/0119
  (all done), 0120 (this item, ready).

## Related Research

- `meta/research/codebase/2026-06-20-0116-structured-stall-on-no-decision-input.md`
- `meta/research/codebase/2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator.md`
- `meta/research/codebase/2026-06-21-0117-agent-decisions-bridge-and-invoker-contract.md`
- `meta/research/codebase/2026-06-21-0119-resume-safe-partial-migration-failure.md`

## Open Questions

1. **AC1 overlap — duplicate, harden, or relabel?** The 0116 "Phase 2" test
   (`test-migrate-interactive.sh:1195-1215`) already satisfies every AC1 clause.
   Does 0120 want a deliberately *distinct* test (e.g. a more distinctive pinned
   key, a FORCE-free clean-tree variant, an explicit "agent-invocation path"
   label/comment that traces to this incident), or is AC1 closeable by verifying
   and lightly hardening the existing test? A near-duplicate adds little
   regression value.
2. **AC2 regex literalness.** Should the cross-check encode the AC's
   `FAIL:.*MISSING-EXTRA` regex verbatim (which matches nothing and is vacuous),
   or assert the meaningful equivalent — exit 0 and absence of the `MISSING-EXTRA`
   token (e.g. via `assert_validates` + `assert_not_contains … "MISSING-EXTRA"`)?
   Recommendation: the latter, with a comment explaining why the literal regex is
   insufficient.
3. **AC2 fixture placement.** Confirm the fixture goes directly under `meta/prs/`
   (→ `pr-description`) and not `meta/reviews/prs/`, and that the tracker prefix
   chosen (e.g. `ENG-`, `PP-`) genuinely fails both `extra_default` derivation
   arms. A counter-fixture (`meta/prs/pr-42-description.md`) asserting the
   derivable path is *not* sentinel-replaced would round out the cross-check.
