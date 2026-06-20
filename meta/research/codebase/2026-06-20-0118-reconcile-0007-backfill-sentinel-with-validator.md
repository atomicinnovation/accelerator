---
type: codebase-research
id: "2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator"
title: "Research: Reconcile 0007 Backfill Sentinel With Its Validator"
date: "2026-06-20T16:47:01+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0118"
parent: "work-item:0118"
relates_to: ["work-item:0114", "work-item:0115", "work-item:0120"]
topic: "Reconcile 0007 Backfill Sentinel With Its Validator"
tags: [research, codebase, migrate, migration-0007, corpus-validator, backfill, sentinel]
revision: "6b9e9bba0648c707e6563e416b03c4203b38d736"
repository: "visualisation-system"
last_updated: "2026-06-20T16:47:01+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Reconcile 0007 Backfill Sentinel With Its Validator

**Date**: 2026-06-20T16:47:01+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 6b9e9bba0648c707e6563e416b03c4203b38d736
**Branch**: detached (work in jj workspace `visualisation-system`)
**Repository**: visualisation-system

## Research Question

For work item 0118 — "Reconcile 0007 Backfill Sentinel With Its Validator":
where and how does migration 0007's required-extras backfill choose to leave an
extra absent, how does its own `self_validate_structural` gate then hard-fail on
that same state, and what is the narrowest correct change to make the backfill
write the sentinel token `unknown` (matching the `verdict`/`lenses` sentinel
contract) so the validator's `MISSING-EXTRA` and `EMPTY-PLACEHOLDER` gates both
pass?

## Summary

The contradiction the work item describes is live and confirmed in the current
tree. Migration 0007 tolerantly leaves a required type-extra **absent** when no
default is derivable (the `pr_number` of a date-/tracker-prefixed filename),
then — one step later, inside the same `set -euo pipefail` orchestration block —
runs the corpus validator, which treats that absent extra as a hard
`MISSING-EXTRA` violation and aborts the entire migration before the interactive
stage is reached.

The fix is sound and low-risk on every axis I verified:

- **The sentinel already exists in the same function.** `extra_default()`
  already `printf 'unknown'`s for the `verdict` and `lenses` extras
  (`0007:219-220`). Reusing `unknown` on the no-derivable-default path is exact
  parity, not a new convention.
- **The validator accepts `unknown` cleanly on both gates.** `MISSING-EXTRA` is
  a pure *key-presence* check (`validate-corpus-frontmatter.sh:345`, via
  `bk_present`) — it never inspects the value, so a present `pr_number: unknown`
  is not missing. `EMPTY-PLACEHOLDER` rejects *only* the literal `""` and `[]`
  (`:354-356`); the non-empty token `unknown` falls through. There is **no
  third check** — no numeric/format validation exists for `pr_number` anywhere
  in the validator, so the bareword string `unknown` is accepted.
- **The write path already handles `pr_number`.** The awk emitter prints
  `pr_number` as a bare unquoted scalar (`0007-frontmatter-rewrite.awk:222`), so
  `pr_number: unknown` is emitted verbatim and parses as a YAML string.

One **implementation-shape decision** surfaced that the plan should settle (see
Open Questions): the sentinel can be written either (a) in the backfill loop's
empty-default branch (`0007:507-509`), which would apply to *every* required
extra with no derivable default, or (b) inside `extra_default()`'s `pr_number`
case specifically, which is narrower and keeps the sentinel logic co-located
with the other sentinels. Option (b) is the tighter match for this task's
"scope strictly to the no-derivable-default path" constraint.

## Detailed Findings

### Migration 0007 — backfill, sentinels, and the validation gate

File:
`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
(runs under `set -euo pipefail`, line 4).

#### `extra_default()` and the existing sentinels (`0007:193-223`)

`extra_default <extra> <file> <stem> <title>` is a `case` over the extra name
that echoes a derived default or empty string. The header comment (lines
188-192) is load-bearing context: under `set -euo pipefail`, every `grep`-in-a-
pipe must be guarded with `|| true` so a no-match doesn't abort the migration
mid-rewrite.

- **`pr_number` derivation (`0007:201-217`)** is two-stage against the stem:
  - Stage 1 (line 205): match a PR-anchored segment —
    `grep -oE '(^|-)[Pp][Rr]-?[0-9]+'`. The `pr` token must be at start-of-stem
    or hyphen-preceded, so `expr-3`/`improve-2` do not match.
  - Stage 2 (lines 210-215), only if stage 1 was empty: a leading-number
    fallback `grep -oE '^[0-9]+'`, **except** the date-prefixed-stem branch
    (line 212): `[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*) : ;;` is a no-op,
    deliberately yielding no fallback. A `<TRACKER>-NNNN-description.md` or
    `YYYY-MM-DD-...` filename matches neither stage → `pr_number` is empty.
- **The existing sentinels (`0007:218-221`)** — quoted verbatim:
  ```
  218    review_number) printf '1' ;;
  219    verdict) printf 'unknown' ;; # sentinel
  220    lenses) printf 'unknown' ;;  # sentinel (emitted as a list)
  221    *) printf '' ;;              # no derivable default → not backfilled
  ```
  `verdict` and `lenses` already emit `unknown`. The catch-all `*)` (line 221)
  emits empty — any extra without a case arm gets no default.

#### The tolerant "left absent" backfill branch (`0007:502-512`)

```
502  for ex in $(extras_for_type "$type"); do
503    case " $FM_OPTIONAL_EXTRAS " in *" $ex "*) continue ;; esac
504    fm_is_empty_val "$(fm_get "$ex" "$f")" || continue
505    dv="$(extra_default "$ex" "$f" "$stem" "$cur_title")"
506    dv="${dv//$US/}"
507    if [ -z "$dv" ]; then
508      log_warn "0007-DIVERGE[missing-extra-no-default]: $f — required extra '$ex' has no derivable default; left absent" >&2
509      continue
510    fi
511    backfill_extras="${backfill_extras:+$backfill_extras$US}${ex}=${dv}"
512  done
```

When a default **is** derivable, line 511 appends a `name=value` record to the
US-separated (`$'\x1F'`) `backfill_extras` channel handed to awk. When it is
**not** (empty `$dv`), lines 507-509 take the tolerant branch: log the counted
`0007-DIVERGE[missing-extra-no-default]` diagnostic and `continue` — no record
is added, the extra is omitted from the file. This is the exact state the
validator later rejects.

The shell never edits the file directly. The packed channel is consumed by
`emit_backfill_extras()` in `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:212-225`
(called at awk line 302), which splits on `\037`, then `key: value`-prints each
record. Relevant special-cases:
- `lenses` → `lenses: ["unknown"]` (awk line 220, list-wrapped).
- `verdict` → `verdict: <normalised>` (awk line 221).
- **`pr_number`/`review_number` → bare `print bk ": " bv`** (awk line 222) — no
  quoting/normalisation. So `pr_number: unknown` would be emitted verbatim.
The result is committed only if changed: `cmp -s` then `atomic_write`
(`0007:540`; `atomic_write` defined `scripts/atomic-common.sh:16`).

#### The self-validation gate that aborts (`0007:555-566`, called `:771`, `} >&2` at `:786`)

```
554  VALIDATOR="$PLUGIN_ROOT/scripts/validate-corpus-frontmatter.sh"
555  self_validate_structural() {
...
565    (cd "$PROJECT_ROOT" && bash "$VALIDATOR" "${files[@]}") >&2
566  }
```

`self_validate_structural` is called as a bare statement at line 771, inside the
orchestration block `{ ... } >&2` (lines 747-786) that runs after
`run_backfill`/`run_rewrite` (765-766) but before `build_corpus_index` (774) and
`harness_run` (788). Because of `set -e` and because the call is unguarded (not
in a condition, `||`, or negation), a non-zero validator exit propagates
straight out and aborts the migration — *before* the interactive harness runs.
A later `self_validate_referential` (`:567-572`, called `:793`) runs after the
harness and is outside this block.

### Corpus validator — the two gates `unknown` must clear

File: `scripts/validate-corpus-frontmatter.sh`.

#### `MISSING-EXTRA` is key-presence only (`:342-346`)

```
343  for f in $extras; do
344    case " $FM_OPTIONAL_EXTRAS " in *" $f "*) continue ;; esac
345    bk_present "$f" || violation "$file" "MISSING-EXTRA" "required extra '$f' absent"
346  done
```

`bk_present` (`:186-192`) returns 0 if the key name appears in `BK_KEYS`,
**ignoring its value entirely**. So `pr_number: unknown` is *present* → no
`MISSING-EXTRA`. (`pr_number` is genuinely required: it is in the schema
`extras` column but **not** in `FM_OPTIONAL_EXTRAS`, so line 344's skip never
fires for it.)

#### `EMPTY-PLACEHOLDER` rejects only literal `""` / `[]` (`:348-359`)

```
354    case "$ev" in
355      '""' | '[]')
356        violation "$file" "EMPTY-PLACEHOLDER" "key '$ek' emitted empty (should be omitted)"
357        ;;
358    esac
```

The match is against the verbatim, whitespace-trimmed scalar text and rejects
*only* the two-character literals `""` and `[]`. `unknown` matches neither →
passes.

#### No third check applies to `pr_number`

Tracing every value-level check in `validate_file` (UNQUOTED-ID, BAD-SCHEMA-
VERSION, BAD-TIMESTAMP, BAD-STATUS, provenance checks, FORBIDDEN-OWN-ID,
OBSOLETE-LEGACY-KEY, BAD-LINKAGE-SHAPE/DANGLING-REF), **none** is keyed to
`pr_number`. There is no numeric/integer regex for it (contrast the
`schema_version`-specific `re_sv_val` at `:261`). `pr_number` lives in the
schema `extras` column, not `typed_linkage_keys`, so the linkage grammar never
touches it. `pr_number: unknown` is therefore accepted cleanly on every axis.

#### Where required extras are defined

The doc-type → required-extras map is **not** hardcoded; it is the `extras`
column (col 4) of `scripts/templates-schema.tsv`, loaded into `SCHEMA_EXTRAS`
(`:84-91`). Relevant rows: `pr-description` → `pr_url pr_number merge_commit`;
`pr-review` → `reviewer verdict lenses review_number pr_number`. The
required/optional downgrade list is `FM_OPTIONAL_EXTRAS` in
`scripts/frontmatter-emission-rules.sh:74`
(`external_id reviewer pr_url merge_commit decision_makers work_item_id`) —
note `pr_url`/`merge_commit` are optional but `pr_number` is not.

### Test coverage that the fix must extend

- `skills/config/migrate/scripts/test-migrate-0007.sh` — dedicated 0007 suite
  (the natural home for the new no-derivable-default → `unknown` assertion and
  the derivable-default no-regression assertion, AC #1 and #4).
- `scripts/test-validate-corpus-frontmatter.sh` — validator suite with existing
  `MISSING-EXTRA`/`EMPTY-PLACEHOLDER` assertions (home for AC #2: assert
  `unknown` is neither).
- `skills/config/migrate/scripts/test-migrate.sh`,
  `test-migrate-snapshot.sh`, `test-migrate-interactive.sh` — broader runner /
  snapshot / interactive suites that reference 0007 (AC #3: full-run-completes
  assertion; the interactive suite is also where sibling 0116/0117/0120 work
  lands).
- Migrations are discovered by glob (`run-migrations.sh:158-164`,
  `find ... '[0-9][0-9][0-9][0-9]-*.sh'`) — **no manifest to update**.

## Code References

- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:4` —
  `set -euo pipefail` (makes the validation gate abort the whole run).
- `…0007-unify-meta-corpus-frontmatter.sh:193-223` — `extra_default()`;
  `pr_number` derivation `:201-217`; date-prefixed no-fallback `:212`.
- `…0007-unify-meta-corpus-frontmatter.sh:219-220` — existing `verdict`/`lenses`
  `unknown` sentinels (the token to reuse).
- `…0007-unify-meta-corpus-frontmatter.sh:502-512` — required-extras backfill
  loop; tolerant "left absent" branch `:507-509`.
- `…0007-unify-meta-corpus-frontmatter.sh:555-566` — `self_validate_structural`;
  invoked `:771`; block redirect `} >&2` `:786`.
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:212-225` —
  `emit_backfill_extras`; `pr_number` bare-print `:222`.
- `scripts/validate-corpus-frontmatter.sh:342-346` — `MISSING-EXTRA` (presence
  only, via `bk_present` `:186-192`).
- `scripts/validate-corpus-frontmatter.sh:348-359` — `EMPTY-PLACEHOLDER`
  (literal `""`/`[]` only).
- `scripts/templates-schema.tsv` (col 4) — per-type required `extras`.
- `scripts/frontmatter-emission-rules.sh:74` — `FM_OPTIONAL_EXTRAS` (excludes
  `pr_number`).
- `skills/config/migrate/scripts/run-migrations.sh:158-164` — glob discovery (no
  manifest).
- `skills/config/migrate/scripts/test-migrate-0007.sh`,
  `scripts/test-validate-corpus-frontmatter.sh` — test homes.

## Architecture Insights

- **The sentinel contract is a deliberate, pre-existing pattern.** `unknown` is
  not a new invention for this task — `extra_default()` already emits it for
  `verdict`/`lenses`, the awk emitter already handles the bare-scalar case, and
  the validator already accepts it. The fix extends an established contract to
  one more branch rather than introducing a mechanism. This is why AC #2 (the
  validator accepts `unknown`) is already true of the current validator —
  no validator change is required; only the backfill must start emitting it.
- **Presence-vs-value asymmetry is the root of the contradiction.** The backfill
  reasons about *derivable value* ("no default → omit"), but the validator
  reasons about *key presence* ("absent → violation"). The sentinel resolves the
  mismatch by making "present with a placeholder" the third state both agree on
  — exactly the "no derivable default ≠ no breadcrumb" principle the source
  research names.
- **`set -e` + unguarded call = the abort mechanism.** Unlike the explicit
  `exit 1` gates elsewhere in the block, `self_validate_structural` aborts purely
  by `set -e` propagation. This is why the contradiction is fatal rather than a
  warning, and why the fix has to operate *upstream* (make the state valid)
  rather than soften the gate.
- **No numeric typing in the validator** means the YAML `pr_number` field is
  effectively untyped at validation time — a string sentinel is structurally
  indistinguishable from a number to the validator. Downstream readers (0007's
  own interactive linkage stage, future migrations) inherit the same
  `unknown`-means-no-value contract that `verdict`/`lenses` already carry.

## Historical Context

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
  — the RCA underpinning the 0115 epic. **Fix option C** (line 216) is this
  task: "when a required extra has no derivable default, write a sentinel
  placeholder (e.g. `pending`) the validator accepts, instead of leaving it
  absent." The `pending` token is **illustrative only** — the binding
  requirement is "a sentinel the validator accepts"; the work item's choice of
  `unknown` (for parity with `verdict`/`lenses`) supersedes it correctly. The
  research recommends C+D as the durable fix with B as immediate mitigation.
  Hypothesis 2 (lines 115-132) documents the backfill-vs-validator contradiction
  with the same line anchors confirmed live here.
- The research's Prevention section (lines 250-253) names the invariant for
  sibling **0120**: "Forbid hard-fail-on-tolerated-state … A lint/test that
  cross-checks 'what the backfill leaves absent' against 'what the validator
  requires' would catch this class." 0118 establishes the invariant; 0120
  asserts it.
- `meta/work/0114-fix-migration-0007-incomplete-mechanical-normalisation.md`
  (status: ready) — broader 0007 backfill completeness. It shares the same
  backfill region but, per the work item's own note, did **not** touch the
  no-derivable-default branch, so there is no live conflict today.
- `meta/work/0115-…-satisfiable-under-agent-invocation.md` — parent epic; splits
  the RCA into children 0116 (structured stall / option B), 0117 (agent-decisions
  bridge / option A), 0118 (this / option C), 0119 (resume-safe partial failure
  / option E), 0120 (prevention tests).
- `meta/reviews/work/0118-…-review-1.md` — review pass 2, final verdict APPROVE
  (the in-body REVISE header is the earlier pass that flagged an under-specified
  sentinel/end-to-end test contract, now resolved in the current work item).
- Older adjacent machinery:
  `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` (the origin of
  migration 0007) and
  `meta/work/0105-close-corpus-validator-provenance-and-linkage-blind-spots.md`
  (earlier validator work).

## Related Research

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
  (direct source / RCA).
- `meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md`
  and
  `meta/research/codebase/2026-06-17-0114-migration-0007-incomplete-mechanical-normalisation.md`
  (0114 — the sibling backfill-completeness work in the same region).

## Open Questions

1. **Where to write the sentinel — `extra_default()` or the backfill loop?**
   This is an implementation-shape decision for the plan:
   - *In `extra_default()`'s `pr_number` case* (return `unknown` instead of
     empty when no number is derivable): narrowest, co-located with the existing
     `verdict`/`lenses` sentinels, affects only `pr_number`. Best matches the
     "scope strictly to the no-derivable-default path" constraint and AC #4's
     no-regression requirement is naturally satisfied (the derivable branches
     are untouched).
   - *In the backfill loop's empty-default branch* (`0007:507-509`, write
     `unknown` instead of `continue`): one edit, but applies to *every* required
     extra that yields no default — including any future extra hitting the
     `extra_default` catch-all `*)`. Broader than this task's scope.
   The first option is recommended; the plan should confirm.
2. **Does any real corpus type require an extra other than `pr_number` on the
   no-default path today?** From `templates-schema.tsv`, the required extras are
   `pr_number` (no-default-prone) plus `verdict`/`lenses`/`review_number`/`topic`
   (all of which already have defaults/sentinels). So in practice only
   `pr_number` exercises this branch today — which is why option (b) above is
   sufficient and the AC examples all use `pr_number`. Worth a one-line
   confirmation in the plan that no other required extra can reach the empty
   branch.
3. **Should the new sentinel value be quoted?** The awk emitter prints
   `pr_number` as a bare scalar (`pr_number: unknown`). This parses as a YAML
   string and passes the validator. No quoting needed; flag only if a downstream
   reader expects a quoted form.
