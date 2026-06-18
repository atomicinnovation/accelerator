---
type: plan
id: "2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation"
title: "Fix Migration 0007 Incomplete Mechanical Normalisation Implementation Plan"
date: "2026-06-17T22:43:39+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0114"
parent: "work-item:0114"
derived_from: ["codebase-research:2026-06-17-0114-migration-0007-incomplete-mechanical-normalisation"]
tags: [migrate, frontmatter, validator, unified-schema, "0007", awk]
revision: "24cd0f82f087dfaa908d5d8bd59d5e9a3590c3d9"
repository: "accelerator"
last_updated: "2026-06-18T00:08:10+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Fix Migration 0007 Incomplete Mechanical Normalisation Implementation Plan

## Overview

Migration `0007-unify-meta-corpus-frontmatter` cannot pass its own structural
validation gate (`validate-corpus-frontmatter.sh`) on real-world corpora: its
mechanical normalisation passes (fence-less backfill + the deterministic awk
rewrite) are incomplete relative to the rule set its validator enforces. On any
corpus containing the unhandled shapes, the migration mutates files, fails the
gate under `set -euo pipefail`, exits before `harness_run`, is never recorded as
applied, and repeats identically on every re-run — permanently blocking the 1.23
upgrade.

This plan closes the six gaps so the in-scope corpus is **validator-clean by the
time `self_validate_structural` runs**, leaving the gate ordering and the
`set -euo pipefail` abort untouched (both are correct per the RCA). The fix is
bounded to the migrate subsystem + the validator; no cross-component reach.

## Current State Analysis

The defect is upstream of the gate, in the mechanical passes. Six distinct gaps:

1. **`meta/prs/` PR descriptions are untypeable.** `infer_type_from_path` (the
   migration `0007-…sh:82-101` and the validator
   `validate-corpus-frontmatter.sh:79-98`, **byte-identical** copies) has a
   `*/reviews/prs/* → pr-review` arm but **no** bare `*/prs/* → pr-description`
   arm; the awk's third encoding `path_to_typed`
   (`0007-frontmatter-rewrite.awk:71-90`) likewise omits it. Empty-`type:` files
   under `meta/prs/` therefore stay typeless → `INVALID-TYPE`. (Largest single
   violation bucket in the RCA.)
2. **Forbidden own-id keys are never dropped.** The schema TSV declares
   `forbidden_own_id_key` (col 6: `pr_title` for pr-description; `pr_title
   review_pass` for pr-review), but **nothing in the migration reads col 6** —
   `own_id_key_for_type` (`0007-…sh:57-63`) hard-codes work-item/adr only. So
   `pr_title`/`review_pass` pass through → `FORBIDDEN-OWN-ID`.
3. **Obsolete legacy keys `ticket`/`ticket_id` are never dropped.** The validator
   forbids them cross-cutting on every type (`OBSOLETE_LEGACY_KEYS`,
   `validate-corpus-frontmatter.sh:47,330-338`); the awk has no drop arm →
   `OBSOLETE-LEGACY-KEY`.
4. **Required type-extras are not backfilled on fenced files.** The closing-fence
   emission block (`0007-frontmatter-rewrite.awk:185-225`) backfills base fields
   and work-item `priority`, but not `topic`/`pr_number`/`review_number`/
   `verdict`/`lenses`. `extras_for_type` (`0007-…sh:52`) is dead code **and** has
   a `cut -f5`→`-f4` off-by-one (col 5 is `status_vocab`). The fence-less
   `topic` backfill (`backfill_file:306-310`) only seeds `note`, not the other
   two `topic`-bearing types → `MISSING-EXTRA`.
5. **Non-canonical linkage shapes pass through.** `normalize_paths`/
   `normalize_bare` (`0007-frontmatter-rewrite.awk:95-143`) only canonicalise
   path-shape and bare-number tokens; a `target: "PR #416"` value survives →
   `BAD-LINKAGE-SHAPE`.
6. **`meta/docs/` is swept in.** `out_of_scope` (`0007-…sh:103-108`,
   `validate-corpus-frontmatter.sh:101-106`) excludes only specs/talks/global.
   Freeform `meta/docs/` files (no schema type, plugin-unowned) are processed and
   judged → `INVALID-TYPE`.

### Key Discoveries

- **Type-by-path is encoded three times.** Two byte-identical bash copies plus
  the awk `path_to_typed`. A shared-sourcing channel already exists: the
  migration sources helpers from `$PLUGIN_ROOT/scripts/` (`0007-…sh:9-12`); the
  validator sources `frontmatter-emission-rules.sh` via an env-overridable path
  (`validate-corpus-frontmatter.sh:24-28`) as its declared single source. A new
  `scripts/`-level helper can be sourced by both; the awk takes the file's type
  via the existing `-v type` channel (computed shell-side).
- **`out_of_scope` is duplicated the same way** (`0007-…sh:103-108` ≈
  `validate-corpus-frontmatter.sh:101-106`) — same drift class, single-sourced
  in the same helper.
- **Schema TSV column order** (1-based): `1 template`, `2 type`,
  `3 code_state_anchored`, `4 extras`, `5 status_vocab`,
  `6 forbidden_own_id_key`, `7 typed_linkage_keys`. `-` = empty cell.
- **Required extras = `extras` (col 4) − `FM_OPTIONAL_EXTRAS`**
  (`frontmatter-emission-rules.sh:74` = `external_id reviewer pr_url merge_commit
  decision_makers work_item_id`). The migration does **not** currently source
  that file; the validator does. pr-review required extras are
  `verdict lenses review_number pr_number` (`reviewer` optional).
- **The migration's `SCHEMA_TSV` is hard-coded** (`0007-…sh:26`), not
  env-overridable — unlike the validator's (`:28`). Making it overridable is what
  lets a test prove the forbidden-key drop is schema-driven.
- **`pr:N` validates.** `FM_TYPED_REF_RE` (`frontmatter-emission-rules.sh:88`)
  tolerates the `pr` prefix and `pr:*` refs skip the `DANGLING-REF` index lookup
  (`validate-corpus-frontmatter.sh:389-390`). The id grammar `[A-Za-z0-9.-]+`
  forbids `/` and `#`.
- **0001 is complementary, not conflicting.** `0001-rename-tickets-to-work.sh`
  renames `ticket_id:`→`work_item_id:` *inside* `meta/tickets/` (`:56-70`) then
  `merge_move`s the whole dir to `meta/work/` (`:79-81`). After 0001 there is no
  `meta/tickets/` and no `ticket_id:` in those files, so 0007's unconditional
  drop only ever catches stray legacy keys elsewhere.
- **Tests use no on-disk fixtures.** `test-migrate-0007.sh` builds throwaway
  corpora under `mktemp -d`, `git_init`s, runs via the real driver (`run_0007`,
  `:36-41`), then gates with the inline validator-clean idiom (`:142-152`). The
  path-shape block (`:230-310`) is the model for new fixtures. The harness has no
  `assert_validates` helper; validator-clean is the inline idiom.

## Desired End State

After 0007's mechanical passes run on a corpus carrying **every** reproduction
shape, `validate-corpus-frontmatter.sh` exits 0 and `harness_run` is reached. The
six gaps are closed; the two byte-identical bash copies of `infer_type_from_path`
(+ `out_of_scope`) collapse to one shared source (the awk's `path_to_typed`
remains a documented, cross-referenced third encoding with a distinct input —
referenced-path vs current-file — kept aligned by a fixture); and a regression
guard asserts `run_backfill + run_rewrite` leaves zero validator violations on the
full fixture corpus.

**Verification:** `bash skills/config/migrate/scripts/test-migrate-0007.sh`
passes (with the new per-shape fixtures and the capstone guard), and
`mise run check` is green across all four components.

## What We're NOT Doing

- **Not reordering the gate** or touching `set -euo pipefail`. The
  structural-gate-aborts-before-harness behaviour is correct (RCA Hypothesis 3).
- **Not backfilling `review_pass`** on plan-review/work-item-review. It is a
  required extra on those two types (a forbidden key only on pr-review) with no
  derivable default; a legacy plan-review/work-item-review missing it remains a
  documented, out-of-scope `MISSING-EXTRA` (not a reproduction shape).
- **Not adding new interactive prompts.** All six gaps are deterministic and
  judgment-free given the decisions below.
- **Not touching producer-side emission (0103) or validator blind-spots (0105)** —
  separate, already-`done` efforts.
- **Not path-scoping the `ticket_id` drop away from `meta/tickets/`.** The drop
  is unconditional; complementarity with 0001 is proven by an integration test
  (see Phase 3).

## Implementation Approach

Schema-driven wherever the contract already encodes the fact (forbidden keys via
TSV col 6; required extras via col 4 − `FM_OPTIONAL_EXTRAS`), single-sourced
wherever the fact is currently duplicated (path → type, out-of-scope). Each phase
is an independently mergeable unit: it adds one transform plus its own narrow,
self-complete fixtures and leaves the whole suite (and `mise run check`) green.
Phases 1–5 each ship green in isolation; Phase 6 is the capstone that can only go
green once 1–5 have landed.

**TDD discipline:** for each phase, add the failing fixture(s) first, confirm they
fail for the expected reason (the targeted violation code), then implement until
green. **bash 3.2 floor** applies to all new shell (no associative arrays, no
`${var,,}`, etc.); the bashisms linter guards it. Run `mise run fix && mise run
check` before considering any phase done.

**Assertion discipline (applies to every phase's Success Criteria).** Three rules
make the fail-first intent verifiable and the new transforms regression-proof —
they are why several ACs below carry both a *content* and a *clean* check:

1. **Assert the transform, not just cleanliness.** Validator-clean (exit 0) is
   necessary but not sufficient — a fixture can pass for the wrong reason (the
   exact failure mode the RCA blames: the dogfood corpus never exercised these
   shapes). Every phase therefore adds a *content-level* assertion on the
   rewritten file (e.g. `fm_line … type` equals `type: pr-description`; the dropped
   key is absent) **in addition to** the validator-clean gate. Where the harness
   lacks a helper, add two thin ones to the suite (or `test-helpers.sh`) and route
   all new blocks through them: `assert_validates <dir|files…>` (wraps the inline
   validator-clean idiom, currently copy-pasted three times) and
   `assert_violation <code> <files…>` (asserts the pre-fix corpus reports the
   targeted violation code, encoding the red step). **`assert_violation` runs the
   standalone validator directly over the freshly-seeded (pre-migration) fixture
   files — NOT via `run_0007`**, which under `set -euo pipefail` mutates files then
   aborts at `self_validate_structural`, leaving a half-migrated tree whose
   surfaced code may differ. Decoupling the red step from the migration's abort
   behaviour makes it assert exactly the shape under test.
2. **Re-assert idempotency on the new shapes.** The existing `=== Idempotency ===`
   block carries none of the reproduction shapes, so re-running it proves nothing
   about the new arms (fold, sentinel backfill, `pr:N` coercion). Every phase adds
   an AC that re-runs the migration on its just-migrated fixture and asserts an
   empty `meta/` working-tree diff (the second pass must be a no-op).
3. **Assert every diagnostic breadcrumb.** Each new `0007-DIVERGE[...]` line a
   transform can emit (discarded `pr_title`, dropped `ticket`/`ticket_id`,
   backfilled sentinel extra) is load-bearing — it is the only signal of a
   fabricated or destroyed value. `run_0007` captures runner stderr into the run
   output (`2>&1`), so each breadcrumb is assertable; the relevant phase adds an
   AC asserting it fires for the seeded shape (these are NOT left as manual-only
   checks).

### Resolved decisions (folded in; no open questions)

- **verdict/lenses** → backfilled with non-empty **sentinels**
  (`verdict: "unknown"`, `lenses: ["unknown"]`) plus a
  `0007-DIVERGE[backfilled-extra]` breadcrumb that is **asserted by an automated
  AC** (Phase 4), not left to manual verification. The neutral `"unknown"` value
  is retained deliberately (not a louder self-identifying string): the migration
  log + asserted breadcrumb are the audit trail, and re-runs will not re-warn once
  the sentinel is committed — a known, accepted limitation recorded here. Empty
  values are impossible (`EMPTY-PLACEHOLDER` forbids `""`/`[]`). Keyed on
  extra-*name*, so the same default-providers cover plan-review/work-item-review
  where those extras are also required.
- **`topic` backfill** → widened to `note`, `codebase-research`, `issue-research`
  in **both** the fence-less (`backfill_file`) and fenced (awk) passes. The default
  is derived from the file's **actual title** — the existing `title:` value
  (`fm_inner "$(fm_get title "$f")"`) when present, else the H1/stem fallback —
  **not** from `title_default`, which `rewrite_file` only populates when
  `has_title==0` and would therefore be empty for the common titled-but-topic-less
  research file (leaving it `MISSING-EXTRA`). The reused title is quote-stripped
  (`tr -d '"'`, parity with the `title_default` path) before emission, so a
  quote-bearing title cannot produce malformed YAML.
- **`pr-description` id** → full filename stem (`240-description`), consistent
  with every non-work-item/adr type. `pr_number` is the **PR-anchored** number
  (the digits adjacent to a `pr-`/`pr` token, else the first leading number),
  never a date component. The leading-number fallback is **guarded against a
  date prefix**: a `^[0-9]{4}-[0-9]{2}-[0-9]{2}` prefix is stripped before the
  fallback runs, so a date-prefixed, pr-token-less stem like `2026-06-17-summary`
  yields **no** `pr_number` (and the `0007-DIVERGE[backfilled-extra]` breadcrumb)
  rather than fabricating the year; `2026-06-17-pr-416-review` still yields `416`
  via the pr-token branch. When no unambiguous number exists, no `pr_number` is
  emitted and the breadcrumb surfaces it rather than fabricating a value.
- **`ticket`/`ticket_id` drop** → unconditional; complementarity with 0001
  proven by integration test. A **non-empty** drop emits a
  `0007-DIVERGE[dropped-legacy-key]` breadcrumb (key + value) so a hand-added
  external-tracker reference is auditable, not silently destroyed.
- **`pr_title` fold** → folds into `title:` only when the file has no `title:`
  **and** the `pr_title` value is non-empty; an empty `pr_title` simply drops
  (the stem-derived `title_default` then supplies a non-empty title). A non-empty
  `pr_title` dropped because a differing `title:` already exists emits a
  `0007-DIVERGE[discarded-key]` breadcrumb.
- **Backfill threading** → required-extra backfills are passed to the awk through
  a **single packed `-v backfill_extras` channel** parsed once by a generic emit
  loop keyed on extra name — not one `-v bf_<extra>` flag per extra. Records are
  `name=value`, **separated by a control byte that cannot appear in a single-line
  YAML scalar** (an ASCII **Unit Separator**, `RS = "\037"`, safe under the
  migration's `LC_ALL=C`), **not** a printable `;` — a `topic` value is an
  arbitrary user H1 and could legitimately contain `;`, which would silently
  truncate a `;`-separated record. The value half may contain `=` (the parser
  splits on the first `=` only) and any printable text. The builder additionally
  strips any stray control byte from each value as defence-in-depth. This keeps
  `rewrite_file` from accreting a new `-v` flag (and the shell-local +
  awk-param + emit-arm triplication) for every future schema extra.
- **Schema column reads** → the new `cut -f4` (extras) / `cut -f6`
  (`forbidden_own_id_key`) reads are pinned by a one-time header guard
  (`fm_assert_schema_columns`) living in `frontmatter-emission-rules.sh` (the file
  that already owns cross-cutting schema rules and is sourced by both surfaces) and
  invoked by **both** the migration and the validator's own positional TSV load
  (`validate-corpus-frontmatter.sh:56`) — so a future column **reorder** fails
  loudly for *both* positional readers. It is a **prefix** check (tolerates a
  forward-compatible trailing column extension, matching the validator's
  surplus-tolerant `read`), built with `$'\t'` (ANSI-C quoting), never a raw
  embedded tab.

---

## Phase 1: Single-source path classification + `meta/prs/` + `meta/docs/`

### Overview

Collapse the triplicated path-classification logic to one source and close gaps
1 and 6. After this phase, `meta/prs/` files type as `pr-description` everywhere
and `meta/docs/` files are skipped by both migration and validator.

### Changes Required

#### 1. New shared helper

**File**: `scripts/doc-type-inference.sh` (new; kebab-case per repo convention)
**Changes**: Holds the two pure path-classification functions, byte-for-byte the
current logic plus the new arms. No top-level side effects (safe to source under
`set -euo pipefail`, bash 3.2).

```bash
#!/usr/bin/env bash
# Path-based doc-type classification, single-sourced by the 0007 migration and
# the corpus validator (previously byte-identical duplicated copies).

# Location → doc-type (exhaustive; reviews discriminated by subdirectory, which
# MUST precede the generic */work/* and */plans/* and the bare */prs/* arms).
infer_type_from_path() {
  case "$1" in
    */reviews/plans/*) echo plan-review ;;
    */reviews/work/*) echo work-item-review ;;
    */reviews/prs/*) echo pr-review ;;
    */prs/*) echo pr-description ;;        # NEW — after reviews/prs so it can't shadow it
    */work/*) echo work-item ;;
    */plans/*) echo plan ;;
    */decisions/*) echo adr ;;
    */research/codebase/*) echo codebase-research ;;
    */research/issues/*) echo issue-research ;;
    */research/design-gaps/*) echo design-gap ;;
    */research/design-inventories/*) echo design-inventory ;;
    */validations/*) echo plan-validation ;;
    */notes/*) echo note ;;
    *) echo "" ;;
  esac
}

# Out of scope (skip entirely): specs/talks/global (freeform) and meta/docs/ (NEW
# — freeform docs the plugin does not own; no schema type). The docs/ arm is
# anchored to */meta/docs/* (NOT a bare */docs/*) so it excludes only the
# top-level corpus docs tree and cannot over-match a nested `…/docs/…` segment
# elsewhere in the corpus.
out_of_scope() {
  case "$1" in
    */specs/* | */talks/* | */global/* | */meta/docs/*) return 0 ;;
    *) return 1 ;;
  esac
}
```

**Validator-wide scope note.** `out_of_scope` lives in the *shared* helper, so
this widening changes behaviour for **every** `validate-corpus-frontmatter.sh`
consumer (the standalone whole-corpus invocation and the migration's file-list
mode), not just the migration. After this change no `meta/docs/` file is validated
by anyone. This is intended — `meta/docs/` is freeform, plugin-unowned, and
carries no schema type — but it is a deliberate contract change recorded here and
in the validator's top-of-file comment (see Change 3). A Phase 1 AC exercises the
validator's **standalone whole-corpus mode** over a corpus containing `meta/docs/`
(not only the migration-routed file-list path) so both surfaces are covered.

#### 2. Migration sources the helper

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: Add `source "$PLUGIN_ROOT/scripts/doc-type-inference.sh"` to the
source block (`:9-12`); **delete** the local `infer_type_from_path` (`:82-101`)
and `out_of_scope` (`:103-108`) definitions.

While editing the orchestration block, **append the VCS-revert recovery hint to
the two abort `log_warn` lines** (`:631` pre-pass refuse, `:637`
REFUSE/MALFORMED) — e.g. `… — revert meta/ via your VCS to recover, then re-run`.
The migration mutates file-by-file with no batch transaction, so this puts the
recovery path in the operator's face at the abort moment rather than only in this
plan's prose (closes the safety signpost as a concrete code change, not a note).

#### 3. Validator sources the helper

**File**: `scripts/validate-corpus-frontmatter.sh`
**Changes**: Add an env-overridable source line alongside the existing emission
rules source (`:24-28`):

```bash
DOC_TYPE_INFERENCE="${DOC_TYPE_INFERENCE:-$SCRIPT_DIR/doc-type-inference.sh}"
# shellcheck source=doc-type-inference.sh
source "$DOC_TYPE_INFERENCE"
```

Delete the local `infer_type_from_path` (`:79-98`) and `out_of_scope`
(`:101-106`) definitions. Update the validator's top-of-file comment (and the
`out_of_scope` skip note, which currently reads "specs/talks/global") to name
`doc-type-inference.sh` as the single source for path classification and to record
the widened skip set (now including `meta/docs/`), so the header does not drift
out of date — the same documentation-drift class this plan is closing for the
path-classification logic itself. The env override `DOC_TYPE_INFERENCE` is a
**test-only** seam (it mirrors the existing `FM_EMISSION_RULES` override and is not
a production configuration knob); note this alongside the source line. Confirm the
new helper lints clean under the root `.shellcheckrc` (`enable=all`): it defines
only functions, so `SC2034` should not fire and no top-of-file disable block is
expected; add one in the `frontmatter-emission-rules.sh` style only if a finding
appears.

Also invoke the shared schema-column guard once after the validator resolves its
`SCHEMA_TSV` (it already sources `frontmatter-emission-rules.sh`, which now defines
`fm_assert_schema_columns` — see Phase 2): `fm_assert_schema_columns "$SCHEMA_TSV"
|| exit 1`, so a column-reorder aborts the validator loudly rather than silently
skewing its positional `IFS=$'\t' read`.

#### 4. Awk third encoding

**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: Add the `meta/prs/` arm to `path_to_typed` (`:71-90`). The
`else id = base` default (`:88`) already yields the full-stem id automatically.

```awk
  else if (p ~ /^meta\/reviews\/prs\//) type = "pr-review"
  else if (p ~ /^meta\/prs\//) type = "pr-description"   # NEW — after reviews/prs
```

(Placement after the `reviews/prs` arm matters: `meta/reviews/prs/...` also
matches `^meta\/prs\/`? No — it begins `meta/reviews/`, so the regex `^meta\/prs\/`
does not match it; ordering is nonetheless kept defensive and readable.)

**Why the awk keeps its own copy (residual, documented drift).** `path_to_typed`
is *not* the same function as the shared `infer_type_from_path` and cannot be
collapsed into it: it classifies the **referenced** meta-paths inside a linkage
value (`normalize_paths` → `path_to_typed`), not the current file, so it cannot
consume the file-level `-v type` channel and must run inside the awk runtime. The
single-sourcing therefore reduces the path→type duplication from **three copies to
two** (the two bash copies collapse; the awk's stays), not to one. This is an
accepted, explicitly-documented residual: add a cross-reference comment in both
`doc-type-inference.sh` and the awk noting they encode the same directory→type
fact for different inputs (current-file vs referenced-path) and **must** be kept
aligned, plus a fixture (Phase 1 ACs) asserting a `meta/prs/` path appearing as a
linkage *target* resolves to `pr-description:<stem>` — so the awk arm and the
shared helper are observably in step.

### Success Criteria

#### Automated Verification

- [x] **Red step**: before the fix, the empty-`type:` `meta/prs/240-description.md`
      fixture reports `INVALID-TYPE` (`assert_violation INVALID-TYPE`); after, it
      types as `pr-description` (`fm_line … type` = `type: pr-description`) and
      validates clean (`assert_validates`). **The fixture is authored
      otherwise-complete** — `pr-description` is `code_state_anchored` and requires
      `pr_number` (TSV col 4, not optional), neither of which Phase 1 backfills, so
      the fixture carries `pr_number` (and lets the anchored `revision`/`repository`
      backfill supply provenance). This keeps Phase 1 green in isolation — typing
      is the only variable under test; `pr_number` backfill is exercised separately
      in Phase 4. (NB: the awk now drops an EMPTY `type:` value so the closing-fence
      backfill emits the inferred type — without this the empty value left a
      duplicate `type:` line; see the awk `type` handler.)
- [x] `meta/docs/logging-guide.md` fixture is byte-unchanged after 0007 and
      produces no `INVALID-TYPE` violation.
- [x] **Validator standalone-mode AC**: `validate-corpus-frontmatter.sh` invoked
      in whole-corpus mode over a corpus containing `meta/docs/` exits 0 (isolates
      the validator-side `out_of_scope` skip from the migration's file-list mode).
- [x] **Linkage-target alignment (table-driven, full id)**: a fixture iterating
      one representative path per shared directory arm (`meta/prs/`, `meta/work/`,
      `meta/plans/`, `meta/decisions/`, `meta/reviews/prs/`,
      `meta/research/codebase/`, …) as a linkage value asserts each resolves to the
      **full expected `doc-type:id`** — including the id-derivation halves that
      differ by type (work-item bare number, ADR `ADR-NNNN`, pr-description full
      stem). This pins the awk `path_to_typed` *id* logic (which the shared
      `infer_type_from_path` does not encode) against drift, not only the
      directory→type mapping. (Implemented as a `path_to_typed` `BEGIN{}` probe.)
- [x] **`pr-description` enters the namespace cleanly**: a `meta/prs/` fixture
      carrying a body-section linkage reference is processed without error and the
      `precondition_prepass` duplicate post-rewrite-id check behaves sanely across
      two `pr-description` stems (the broadened identity namespace is exercised,
      not just the typing). (Two distinct `meta/prs/` stems + a path-shape
      `relates_to` ref between them, normalised to `pr-description:<stem>`.)
- [x] Both surfaces use the helper: migration + validator no longer define the
      functions locally (grep guard, or a small unit asserting both source it).
- [x] **Idempotency**: a second `run_0007` over the just-migrated Phase 1 fixtures
      leaves an empty `meta/` diff.
- [x] Existing suite still green: `bash skills/config/migrate/scripts/test-migrate-0007.sh`
- [x] Shell checks pass: `mise run scripts:check` (shfmt + ShellCheck + bashisms)
- [ ] `mise run check` (deferred to the end-of-implementation full run; Phase 1
      touches only the `scripts` component, which is green)

#### Manual Verification

- [ ] A real downstream corpus with `meta/prs/` + `meta/docs/` no longer reports
      the `INVALID-TYPE` bucket for those directories.

---

## Phase 2: Schema-driven forbidden own-id key drop + `pr_title`→`title` fold

### Overview

Close gap 2: drop the schema's `forbidden_own_id_key`s, driven by TSV col 6 (not
hard-coded). Fold a dropped `pr_title` into `title:` when the file has no
`title:`, otherwise discard.

### Changes Required

#### 1. Make the migration's schema path overridable + read col 6

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**:

```bash
SCHEMA_TSV="${SCHEMA_TSV:-$PLUGIN_ROOT/scripts/templates-schema.tsv}"   # was hard-coded (:26)
```

Also add `source "$PLUGIN_ROOT/scripts/frontmatter-emission-rules.sh"` to the
source block (`:9-12`) — needed here for `fm_assert_schema_columns` (below) and
reused in Phase 4 for `FM_OPTIONAL_EXTRAS`.

```bash
# Space-joined forbidden own-id keys for a type (TSV col 6); "-" → empty.
forbidden_keys_for_type() {
  local v
  v="$(schema_row "$1" | cut -f6)"
  [ "$v" = "-" ] && v=""
  printf '%s' "$v"
}
```

Add a one-time **header-name assertion** so the positional `cut -f4`/`cut -f6`
reads here and in Phase 4 — **and the validator's own positional TSV load**
(`validate-corpus-frontmatter.sh:56`) — fail loudly if the schema is ever
**reordered**, rather than silently reading the wrong column (the exact class of
the `extras_for_type` off-by-one this plan fixes). It lives in
`frontmatter-emission-rules.sh` — the file that already self-declares as the
single source for cross-cutting schema rules and references the schema's
`forbidden_own_id_key`/`extras` columns, and is already sourced by the validator
(and by the migration, see below) — **not** in `doc-type-inference.sh`, which
stays cohesively scoped to path classification. It is invoked once at startup by
**both** the migration (in the pre-harness `{ … }` block, alongside the other gate
checks: `fm_assert_schema_columns "$SCHEMA_TSV" || exit 1`) and the validator
(after it resolves its `SCHEMA_TSV`).

It is a **prefix** check, not exact-match: it pins the canonical 7-column order a
reorder would break, but tolerates a forward-compatible *trailing* column
extension — mirroring the validator's surplus-tolerant `IFS=$'\t' read`, so the
guard does not newly reject a column-extended `SCHEMA_TSV` that the validator's
read already accepts. The expected header is built with `$'\t'` (ANSI-C quoting),
never a raw embedded tab, and a trailing `\r` is tolerated (CRLF robustness):

```bash
# Pin the column ORDER the cut -fN / positional read consumers depend on; a
# reorder then fails here, in one place, for every positional reader instead of
# silently skewing extras/forbidden reads. Prefix-match (exact OR canonical
# columns followed by a tab + extra columns) so a forward-compatible trailing
# extension is tolerated, matching the validator's surplus-tolerant read. Takes
# the schema path as $1 so both the migration ($SCHEMA_TSV) and validator call it.
fm_assert_schema_columns() {
  local hdr expected
  hdr="$(head -1 "$1")"
  hdr="${hdr%$'\r'}"   # tolerate a CRLF-authored TSV
  # Assembled across two appends to stay within the 80-col floor (a single
  # $'...' literal would be ~110 cols and shfmt cannot wrap it).
  expected=$'template\ttype\tcode_state_anchored\textras\t'
  expected+=$'status_vocab\tforbidden_own_id_key\ttyped_linkage_keys'
  case "$hdr" in
    "$expected" | "$expected"$'\t'*) return 0 ;;
    *)
      printf '%s\n' "schema column order changed in $1; update positional readers" >&2
      return 1
      ;;
  esac
}
```

Because `fm_assert_schema_columns` lives in `frontmatter-emission-rules.sh`, the
migration must source that file **by Phase 2** (Phase 4 then reuses it for
`FM_OPTIONAL_EXTRAS` — see Phase 4, which notes it is already sourced). The Phase 2
AC asserts the migration **aborts non-zero with zero file mutations** on a
column-**reordered** `SCHEMA_TSV` (halt semantics pinned, not just firing), and a
separate AC asserts a column-**extended** `SCHEMA_TSV` is **accepted** (proves the
prefix tolerance, so existing column-extending downstream validator consumers are
not broken).

In `rewrite_file`, compute and thread it (alongside the existing `-v` list at
`:415-427`):

```bash
forbidden="$(forbidden_keys_for_type "$type")"
# ... awk call gains:
#   -v forbidden="$forbidden"
# has_title is already computed (:370).
```

#### 2. Awk: membership helper + drop arm + fold

**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: Add `is_forbidden()` (model on `in_vocab`, `:145-149`, but
space-split since col 6 is space-separated); add `emitted_title` to `BEGIN`
state (`:167-173`); add the drop/fold arm at a **pinned position** — immediately
after the `id`/own-id handlers (`:247-254`) and **before** the linkage and
omit-when-empty arms (`:288-301`) and the catch-all `print $0` (`:303`). The
position matters: the fold logic must run before the omit-when-empty handler
(`:301`) could otherwise drop the value, and the empty-value guard below ensures an
empty forbidden key drops cleanly rather than folding. Guard the closing-fence
title backfill (`:194`) with `!emitted_title`.

```awk
function is_forbidden(k,   n, a, i) {
  n = split(forbidden, a, " ")
  for (i = 1; i <= n; i++) if (trim(a[i]) == k) return 1
  return 0
}
```

```awk
  # Forbidden own-id keys (schema TSV col 6): drop. pr_title additionally folds
  # into title: ONLY when the file has no title: AND the value is non-empty;
  # otherwise it is discarded — and a non-empty discard (a real pr_title lost
  # because a differing title: already exists) is surfaced as a breadcrumb so it
  # is auditable rather than silently destroyed.
  if (is_forbidden(key)) {
    if (key == "pr_title" && !has_title && !emitted_title && !is_empty_val(val)) {
      print "title: " fm_normalise_value(val); emitted_title = 1
    } else if (key == "pr_title" && !is_empty_val(val)) {
      print "0007-DIVERGE[discarded-key]: " file " — pr_title discarded (title present): " val > "/dev/stderr"
    }
    next
  }
```

(An empty forbidden key — `pr_title: ""` / `review_pass: ""` — neither folds nor
DIVERGEs: it simply drops, and the stem-derived `title_default` then supplies a
non-empty `title:`. This closes the `title: ""` EMPTY-PLACEHOLDER hole that an
unguarded fold would open.)

Closing-fence guard:

```awk
  if (!has_title && !emitted_title && title_default != "") print "title: \"" title_default "\""
```

### Success Criteria

#### Automated Verification

- [x] **Red step**: the pr-review fixture with `pr_title:` + `review_pass:`
      reports `FORBIDDEN-OWN-ID` before the fix (`assert_violation`).
- [x] pr-review fixture with `pr_title:` + `review_pass:` → both keys removed
      (asserted absent via `fm_line`), validates clean (fixture authored with
      `verdict`/`lenses`/`review_number`/`pr_number` present so it is otherwise
      complete).
- [x] `pr_title` + no `title:` (non-empty) → value promoted to `title:`,
      `pr_title` removed.
- [x] `pr_title` + existing differing `title:` → `pr_title` dropped, `title:`
      unchanged, and `0007-DIVERGE[discarded-key]` asserted present. (Asserted via
      a DIRECT migration run — the runner sandboxes the migration's stderr to a
      per-migration log it DELETES on success, so DIVERGE breadcrumbs are NOT in
      the driver's `RUN_OUT`; a `run_0007_direct` helper captures the migration's
      own stderr. This corrects the plan's "captures runner stderr" assumption.)
- [x] `pr_title` **equal to** an existing `title:` → `pr_title` dropped, `title:`
      unchanged (pins the equal-value boundary; the breadcrumb fires as for any
      title-present discard — a benign no-information-loss case).
- [x] **Empty-fold guard**: a `pr_title: ""` (no `title:`) → no `title: ""`
      emitted; the stem-derived `title_default` supplies the title; validates
      clean (no `EMPTY-PLACEHOLDER`).
- [x] **Closing-fence interaction**: a title-less file yields **exactly one**
      `title:` line in both the `pr_title`-present (folded) and `pr_title`-absent
      (defaulted) variants (guards the `emitted_title` coupling).
- [x] **Schema-driven proof**: a fixture using a custom `SCHEMA_TSV` (via the new
      env override) declaring a *novel* forbidden key (e.g. `bogus_key`) on some
      type, with a corpus file carrying it → dropped (asserted absent). A
      hard-coded implementation would fail this.
- [x] **Header assertion (halt, not just warn)**: a fixture pointing `SCHEMA_TSV`
      at a column-**reordered** TSV makes the migration **exit non-zero with zero
      file mutations** (empty `meta/` diff) — pins the guard's abort semantics, not
      only that the warning text appears.
- [x] **Header assertion (extension tolerated)**: a `SCHEMA_TSV` with an extra
      **trailing** column (canonical 7 unchanged) is **accepted** by both the
      migration and the validator — proves the prefix-match does not break a
      forward-compatible column extension.
- [x] **Idempotency**: second `run_0007` over the migrated Phase 2 fixtures leaves
      an empty `meta/` diff.
- [x] `bash skills/config/migrate/scripts/test-migrate-0007.sh` green; `mise run check`
      (full `mise run check` deferred to end; `scripts:check` green)

#### Manual Verification

- [ ] On a real corpus, pr-review/pr-description files no longer report
      `FORBIDDEN-OWN-ID`, and folded titles read sensibly.

---

## Phase 3: Unconditional `ticket`/`ticket_id` drop

### Overview

Close gap 3: drop `ticket`/`ticket_id` on any type (the validator forbids them
cross-cutting). Prove complementarity with 0001 via an integration test rather
than special-casing `meta/tickets/`.

### Changes Required

#### 1. Awk drop arm

**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: Add an unconditional drop (model on the `branch:` drop, `:241`),
placed with the other early key handlers. A **non-empty** value is surfaced as a
breadcrumb before dropping, so a hand-added external-tracker reference (e.g.
`ticket: "PROJ-1234"` on a note) is auditable in the migration log rather than
silently destroyed; an empty key drops quietly:

```awk
  # Obsolete legacy keys (cross-cutting, any type) — migrated out by 0001 (in
  # meta/tickets/) and dropped everywhere else here. A non-empty value is logged
  # so a real external-tracker reference is recoverable via the breadcrumb + VCS.
  if (key == "ticket" || key == "ticket_id") {
    if (!is_empty_val(val))
      print "0007-DIVERGE[dropped-legacy-key]: " file " — dropped " key ": " val > "/dev/stderr"
    next
  }
```

### Success Criteria

#### Automated Verification

- [x] **Red step**: a `note` carrying `ticket: "PROJ-1234"` reports
      `OBSOLETE-LEGACY-KEY` before the fix (`assert_violation`).
- [x] A `note` with `ticket: "PROJ-1234"` → `ticket` removed (asserted absent),
      validates clean, and `0007-DIVERGE[dropped-legacy-key]` asserted present
      (via a direct run — see Phase 2 note on the runner deleting migration stderr).
- [x] A non-note type carrying `ticket_id:` → `ticket_id` removed regardless of
      value.
- [x] **Integration (0001 → 0007, same session)**: a `meta/tickets/NNNN-foo.md`
      with `ticket_id:` run through a migrations dir containing **both** 0001 and
      0007 → lands as `meta/work/NNNN-foo.md` with `id: "NNNN"` (0001 renamed
      `ticket_id`→`work_item_id`, dir moved to `meta/work/`, 0007 folded
      `work_item_id`→`id`), with no `ticket_id` surviving and no data clobbered.
- [x] **Integration (0001 pre-applied, cross-session)**: the more common
      downstream path — 0001 already recorded applied, only the new 0007 runs over
      an already-renamed `meta/work/NNNN-foo.md` → the unconditional
      `ticket`/`ticket_id` drop is a no-op on the 0001-output shape and the run is
      idempotent.
- [x] **Combined idempotency**: a second pass of the two-migration sequence over
      the already-migrated tickets corpus leaves an empty working-tree diff (the
      second pass re-runs both migrations DIRECTLY, since the runner ledger skips
      already-applied migrations).
- [x] `bash skills/config/migrate/scripts/test-migrate-0007.sh` green; `mise run check`
      (full `mise run check` deferred to end; `scripts:check` green)

#### Manual Verification

- [ ] On a real corpus, no `OBSOLETE-LEGACY-KEY` violations remain.

---

## Phase 4: Required-extras backfill (`topic`/`pr_number`/`review_number`/`verdict`/`lenses`)

### Overview

Close gap 4: backfill required type-extras on fenced files (each only when
absent; never overwrite), widen the fence-less `topic` backfill to all three
`topic`-bearing types, and wire in `extras_for_type` (fixing its off-by-one).

### Changes Required

#### 1. Fix the off-by-one and source the optional list

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**:

```bash
extras_for_type() {              # was cut -f5 (status_vocab); extras is col 4
  local v
  v="$(schema_row "$1" | cut -f4)"
  [ "$v" = "-" ] && v=""
  printf '%s' "$v"
}
```

While in this helper cluster, **remove the dead, column-confused
`status_vocab_for_type()` (`:53`)** and its stale `# 5th col` comment, leaving the
single live `status_vocab_of()` (`:55`) alongside the now-correct
`extras_for_type`/`forbidden_keys_for_type`. Leaving a second status-vocab reader
beside the just-fixed `extras_for_type` would perpetuate exactly the
which-one-is-real confusion that produced the original off-by-one.

Reuse `FM_OPTIONAL_EXTRAS` from `frontmatter-emission-rules.sh` (do not duplicate
the list). That file is **already sourced in Phase 2** (for
`fm_assert_schema_columns`), so no new source line is needed here — just consume
the constant.

**Frozen-migration coupling note.** `frontmatter-emission-rules.sh` documents
itself as the *evolving* single source for cross-cutting rules, while a shipped
migration must reproduce its historical output forever. The dependency is
acceptable because the **specific** constant 0007 consumes — `FM_OPTIONAL_EXTRAS`
for the doc-types 0007 touches (`note`, `codebase-research`, `issue-research`,
`pr-review`, `plan-review`, `work-item-review`, `pr-description`) — is
contractually stable: those extras (`topic`/`verdict`/`lenses`/`review_number`/
`pr_number`) are *required*, not optional, so a future edit to the optional set
cannot retroactively change what 0007 backfills for them. Record this assumption
in a comment at the source line so a future editor of `FM_OPTIONAL_EXTRAS` knows
0007 depends on it — **and enforce it** rather than leaving it comment-only: add a
regression assertion (a Phase 4 AC) that the required-extra set 0007 derives for
each touched type (`extras_for_type <type>` − `FM_OPTIONAL_EXTRAS`) still contains
`topic`/`verdict`/`lenses`/`review_number`/`pr_number` as expected. If a future
schema edit ever moves one of these into the optional carve-out, that test fails
loudly instead of 0007 silently changing its historical output.

Add a helper that yields the *required-and-absent* extras' defaults for a file,
and a default-provider. Defaults keyed on extra **name** (so plan-review/
work-item-review verdict/lenses are covered too):

```bash
# Echo the default value for a required extra, or empty if none derivable.
# CRITICAL: every command substitution here must succeed under `set -euo
# pipefail` — an unguarded pipe whose grep finds no match exits non-zero and
# aborts the whole migration mid-rewrite (the exact permanent-stall this plan
# fixes). Hence the `|| true` guards.
extra_default() {            # $1=extra-name $2=file $3=stem $4=title
  local n
  case "$1" in
    topic)
      # ← title, with embedded quotes stripped (parity with title_default's
      # `tr -d '"'`; an unescaped " in a double-quoted scalar is invalid YAML).
      printf '%s' "$4" | tr -d '"'
      ;;
    pr_number)
      # PR-anchored number: digits of a genuine pr-/PR- *segment* — the `pr` token
      # must be at start-of-stem or preceded by a hyphen, so a `pr` embedded in a
      # word (expr-3, improve-2) does NOT match. Plain (case-sensitive) grep with
      # an explicit [Pp][Rr] class — no `-i` (stems are lowercase by convention).
      n="$(printf '%s' "$3" | grep -oE '(^|-)[Pp][Rr]-?[0-9]+' | grep -oE '[0-9]+' | head -1 || true)"
      # Leading-number fallback ONLY for a stem that is NOT date-prefixed (e.g.
      # 240-description → 240). A date-prefixed, pr-token-less stem
      # (2026-06-17-summary, 2026-06-17-0114-foo) has no derivable PR number → stays
      # empty, so the builder breadcrumbs it rather than fabricating a date/id part.
      if [ -z "$n" ]; then
        case "$3" in
          [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*) : ;;   # date-prefixed → no fallback
          *) n="$(printf '%s' "$3" | grep -oE '^[0-9]+' | head -1 || true)" ;;
        esac
      fi
      printf '%s' "$n"
      ;;
    review_number) printf '1' ;;
    verdict)       printf 'unknown' ;;                       # sentinel
    lenses)        printf 'unknown' ;;                       # sentinel (emitted as a list)
    *)             printf '' ;;                              # no derivable default → not backfilled
  esac
}
```

In `rewrite_file`, build a **single packed `backfill_extras` channel** rather than
one `-v bf_<extra>` flag per extra. For each required extra (`extras_for_type` −
`FM_OPTIONAL_EXTRAS`) that is **absent-or-empty** on the file, append a
`name=value` record (where `value = extra_default …`, skipping records whose
default is empty). Two correctness points the builder must get right:

- **Presence check uses an empty-value helper, not bare `fm_get`.** A
  present-but-empty placeholder (`lenses: []`, `verdict: ""`) returns a non-empty
  *string* from `fm_get`, yet the awk's omit-when-empty arm (`:301`) will *drop*
  it — so a bare `[ -n "$(fm_get …)" ]` check would skip backfill and leave the
  file `MISSING-EXTRA`. Use a shared `fm_is_empty_val` helper (below) that mirrors
  the awk's `is_empty_val` and the validator's EMPTY-PLACEHOLDER rule, rather than
  open-coding the `""`/`[]` set a third time.
- **`topic` derives from the file's actual title, not `title_default`.**
  `title_default` is empty whenever the file already has a `title:`
  (`:387-390`), which is the common research-file shape — so pass the resolved
  current title (existing `title:` value, else the computed `title_default`/H1).
- **A required extra with no derivable default is breadcrumbed, not silently
  skipped.** `pr_number` for a numberless pr file (see "What We're NOT Doing")
  routes here; emitting a `0007-DIVERGE[missing-extra-no-default]` makes the
  residual `MISSING-EXTRA` auditable rather than silent.

Add the empty-value helper near `fm_get`/`fm_inner` (single shell home for the
placeholder definition; the awk and validator keep their cross-runtime siblings):

```bash
# Empty-placeholder test, mirroring the awk is_empty_val and the validator's
# EMPTY-PLACEHOLDER rule. Keep the three in lockstep.
fm_is_empty_val() { case "$1" in '' | '""' | '[]') return 0 ;; *) return 1 ;; esac; }
```

Records are separated by an ASCII **Unit Separator** (`$'\x1F'` — the spelling the
codebase's existing US-channel sites use; the same byte the awk splits on as octal
`"\037"`), which cannot occur in a single-line YAML scalar (safe under `LC_ALL=C`);
the parser splits on the first `=` per record so `=` in a value is safe.

```bash
US=$'\x1F'                                  # record separator (0x1F; awk splits "\037")
# Resolved current title: prefer the file's own title:, else the H1/stem default.
cur_title="$(fm_inner "$(fm_get title "$f")")"
[ -n "$cur_title" ] || cur_title="$title_default"
backfill_extras=""
for ex in $(extras_for_type "$type"); do
  case " $FM_OPTIONAL_EXTRAS " in *" $ex "*) continue ;; esac   # optional → skip
  fm_is_empty_val "$(fm_get "$ex" "$f")" || continue           # present & non-empty → skip
  dv="$(extra_default "$ex" "$f" "$stem" "$cur_title")"
  dv="${dv//$US/}"                                              # defence-in-depth: strip stray US
  if [ -z "$dv" ]; then
    log_warn "0007-DIVERGE[missing-extra-no-default]: $f — required extra '$ex' has no derivable default; left absent" >&2
    continue
  fi
  backfill_extras="${backfill_extras:+$backfill_extras$US}${ex}=${dv}"
done
# threaded into the awk call (alongside :415-427):
#   -v backfill_extras="$backfill_extras"
```

(`${dv//$US/}` is bash 3.2-safe — pattern substitution, not case-modification — and
is a parser-safety guard expected to be a no-op; a 0x1F byte in a title is
impossible under `LC_ALL=C`. One packed `-v` instead of five means a future
required extra needs no new awk parameter — only a new `extra_default` arm and, if
its emission differs from the generic scalar/list rule, a branch in the generic
emit loop. `topic`'s spaces are fine: the record separator is the US control byte,
not whitespace.)

#### 2. Awk closing-fence backfill

**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: Extract the parse/emit into a **callable awk function**
`emit_backfill_extras(packed)` (so it is unit-probeable from a `BEGIN{}` block,
exactly like the existing `fm_normalise_value` parity probe — the parse logic must
not be inlined in the pattern-action block, or the Phase 4 parser-probe AC cannot
invoke it). Call it from the closing-fence block (`:185-225`), before `in_fm = 0`
(`:223`). The emission is data-driven: `lenses` is the only list-cardinality
backfilled extra (single-element flow list, spelled to match the `tags: []`
precedent); `verdict`/`lenses` additionally print a sentinel breadcrumb. Scalars
are emitted via `fm_normalise_value` (parity with the `pr_title` fold arm), so a
backslash/indicator-leading title is quoted/escaped consistently rather than
hand-wrapped. Records split on the US byte as octal `"\037"`, matching the shell
builder's `$'\x1F'` separator (same byte, two encodings):

```awk
function diverge_backfill(k) {
  print "0007-DIVERGE[backfilled-extra]: " file " — " k " backfilled with sentinel; review manually" > "/dev/stderr"
}

# Parse "name=value\037name=value…" (see rewrite_file). Pure/callable so a
# BEGIN{} probe can exercise the empty/single-record/=-in-value cases.
function emit_backfill_extras(packed,   nbf, bfa, bi, eq, bk, bv) {
  if (packed == "") return
  nbf = split(packed, bfa, "\037")            # octal US == shell $'\x1F' (same byte)
  for (bi = 1; bi <= nbf; bi++) {
    eq = index(bfa[bi], "=")
    if (eq == 0) continue                       # malformed record → skip
    bk = substr(bfa[bi], 1, eq - 1)
    bv = substr(bfa[bi], eq + 1)
    if (bk == "lenses") { print "lenses: [\"" bv "\"]"; diverge_backfill(bk) }
    else if (bk == "verdict") { print "verdict: " fm_normalise_value(bv); diverge_backfill(bk) }
    else if (bk == "pr_number" || bk == "review_number") print bk ": " bv
    else print bk ": " fm_normalise_value(bv)   # topic and any future scalar extra
  }
}
```

```awk
  emit_backfill_extras(backfill_extras)         # in the closing-fence block
```

Note the list-vs-scalar **cardinality is hard-coded** in this function (`lenses`
is the sole list extra today) — the TSV does not carry a cardinality column, so
`assert_schema_columns` cannot guard it. Add a comment flagging that a future
list-valued required extra needs a branch here, and the Phase 4 sentinel-`lenses`
AC (`lenses: ["unknown"]`) plus the populated-`lenses` AC together exercise the
flow-list emission path.

#### 3. Widen fence-less `topic` backfill

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: In `backfill_file` (`:306-310`), seed `topic` for the three
`topic`-bearing types, not just `note` (the `note` branch keeps its
`producer`/`status` emission):

```bash
    if [ "$type" = "note" ]; then
      printf 'producer: create-note\n'
      printf 'status: captured\n'
    fi
    case "$type" in
      note | codebase-research | issue-research)
        # Strip embedded quotes from the title (an unescaped " inside a
        # double-quoted scalar is invalid YAML; parity with the fenced
        # extra_default topic path).
        printf 'topic: "%s"\n' "$(printf '%s' "$title" | tr -d '"')" ;;
    esac
```

### Success Criteria

#### Automated Verification

- [ ] **Red step**: a fenced `note` missing `topic` reports `MISSING-EXTRA`
      before the fix (`assert_violation`).
- [ ] Fenced `note` missing `topic` → `topic` backfilled from title, validates.
- [ ] **Titled-but-topic-less research** (the common shape): a fenced
      `codebase-research` / `issue-research` that **has** a `title:` but lacks
      `topic` → `topic` backfilled from the *existing title* (proves the default
      derives from the actual title, not the empty `title_default`), validates.
- [ ] **Empty-placeholder extras**: a review carrying `lenses: []` / `verdict: ""`
      → treated as absent, backfilled (not dropped-then-MISSING-EXTRA), validates
      clean (guards the `is_empty_val`-semantics presence check).
- [ ] Fence-less `codebase-research` file → backfilled with `topic` (proves the
      fence-less widen).
- [ ] **Quote-bearing title** → emitted `topic`/`title` carry no stray `"`
      (validates; no malformed-YAML), in both the fenced and fence-less paths.
- [ ] **Semicolon-bearing title**: a fixture whose H1 contains `;` (e.g.
      `# Add caching; drop the old path`) → the full title survives as a single
      valid YAML scalar in the backfilled `topic` (proves the US-separator packed
      channel does not truncate on `;`).
- [ ] Fenced `pr-review` from `pr-430-review.md` lacking `pr_number`/
      `review_number`/`verdict`/`lenses` → `pr_number: 430`, `review_number: 1`,
      `verdict: "unknown"`, `lenses: ["unknown"]`; validates clean.
- [ ] **PR-anchored number**: a date-prefixed stem `2026-06-17-pr-416-review`
      backfills `pr_number: 416` (NOT `2026`); a **date-prefixed, pr-token-less**
      stem `2026-06-17-summary` (and `2026-06-17-0114-foo`) emits **no** `pr_number`
      (NOT `2026`/`0114`) and a `0007-DIVERGE[missing-extra-no-default]` breadcrumb;
      a word containing `pr` (`expr-3`) does **not** yield `pr_number: 3`.
- [ ] **Digit-less stem does not abort**: a pr-review fixture whose stem has no
      digits runs to completion under `set -euo pipefail` (guards the critical
      unguarded-grep regression) and reaches the validator gate.
- [ ] **Breadcrumb asserted**: the sentinel `verdict`/`lenses` backfill emits
      `0007-DIVERGE[backfilled-extra]` in the run output (asserted, not manual).
- [ ] **Packed-channel parser edge cases** (awk-level probe calling the extracted
      `emit_backfill_extras` from a `BEGIN{}` block, in the style of the existing
      frag.awk parity probe): an **empty** channel emits no stray line; a
      **single-record** channel (no separator) emits exactly that record; a value
      containing `=` (split on first `=` only) and a value containing a **space**
      round-trip intact. **Run the probe under the actual system awk on both macOS
      and Linux CI** (awk is unpinned — BSD vs gawk/mawk) to prove the raw-US-byte
      `-v` + `split("\037")` round-trips on both.
- [ ] Existing values never overwritten: a file already carrying `topic`/
      `verdict`/etc. is left unchanged.
- [ ] **Populated multi-element list not clobbered**: a review carrying
      `lenses: ["security", "performance"]` and a real `verdict:` is left
      byte-unchanged and emits **no** `0007-DIVERGE[backfilled-extra]` (guards
      against the sentinel overwriting real reviewer data).
- [ ] **Micro-assertions on the pure helpers** (no full corpus run):
      `extra_default pr_number "" 2026-06-17-pr-416-review "" → 416`,
      `extra_default pr_number "" 2026-06-17-summary "" → ""` (date-prefixed, no
      fallback), `extra_default pr_number "" 2026-06-17-0114-foo "" → ""`,
      `extra_default pr_number "" 240-description "" → 240`,
      `extra_default pr_number "" expr-3-foo "" → ""` (word-boundary anchor),
      `forbidden_keys_for_type pr-review → "pr_title review_pass"`,
      `extras_for_type pr-review` contains `pr_number`;
      `fm_is_empty_val '[]'`/`fm_is_empty_val '""'` true, `fm_is_empty_val x` false.
- [ ] **Required-extra contract guard**: for each touched type, the derived
      required set (`extras_for_type <type>` − `FM_OPTIONAL_EXTRAS`) still contains
      `topic`/`verdict`/`lenses`/`review_number`/`pr_number` as applicable — fails
      loudly if a future schema edit moves one into the optional carve-out
      (enforces the frozen-migration coupling assumption).
- [ ] **Idempotency**: second `run_0007` over the migrated Phase 4 fixtures leaves
      an empty `meta/` diff (the absence-gated backfill must not re-fire).
- [ ] `bash skills/config/migrate/scripts/test-migrate-0007.sh` green; `mise run check`

#### Manual Verification

- [ ] Spot-check that sentinel-backfilled reviews read sensibly and that the
      `0007-DIVERGE[backfilled-extra]` log lines name the right files for human
      follow-up (the automated AC above guarantees the breadcrumb fires; this is a
      legibility spot-check only).

---

## Phase 5: Non-canonical linkage shape coercion (`PR #N` → `pr:N`)

### Overview

Close gap 5: coerce non-canonical PR-reference linkage tokens — `"PR #N"`,
`"PR#N"`, `"pr #N"`, `"PR-N"`/`"pr-N"`, and bare `"#N"` — to `"pr:N"`.

### Changes Required

#### 1. Awk `normalize_pr_ref` sibling + chain

**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: Add a sibling of `normalize_paths`/`normalize_bare` matching the
double-quoted PR-reference token shapes; chain it **innermost** (so `#` is
consumed before the bare-number loop) at the linkage chain site (`:291`). The
pattern tolerates the real spelling variants — `"PR #416"`, `"PR#416"`,
`"pr #416"`, `"PR-416"`/`"pr-416"`, and bare `"#416"` — via the
`[Pp][Rr]` class and an optional space/hyphen/`#` (POSIX awk has no inline
case-insensitive flag, hence the explicit class):

```awk
function normalize_pr_ref(val,   out, rest, pre, tok, num) {
  out = ""; rest = val
  while (match(rest, /"([Pp][Rr][ -]?#?|#)[0-9]+"/)) {
    pre = substr(rest, 1, RSTART - 1)
    tok = substr(rest, RSTART, RLENGTH)
    num = tok; gsub(/[^0-9]/, "", num)
    out = out pre "\"pr:" num "\""
    rest = substr(rest, RSTART + RLENGTH)
  }
  return out rest
}
```

```awk
  newval = normalize_bare(normalize_pr_ref(normalize_paths(val)), type, key)
```

(Idempotent / no re-grab: a rewritten `"pr:416"` has `:` immediately after `pr`,
so the `[ -]?#?[0-9]+` tail fails at `:` — the pattern does not re-match it — and
`normalize_bare` matches only fully-bare `"[0-9]+"`, so it leaves `"pr:416"`
alone. An already-typed `"plan:2026-…"` is likewise untouched, since `pl` fails
`[Pp][Rr]`.)

### Success Criteria

#### Automated Verification

- [ ] **Red step**: `target: "PR #416"` reports `BAD-LINKAGE-SHAPE` before the
      fix (`assert_violation`).
- [ ] `target: "PR #416"` → `target: "pr:416"`, validates clean (`pr:` tolerated
      by `FM_TYPED_REF_RE` and skipped from the dangling-ref lookup).
- [ ] `"#416"` form → `"pr:416"`.
- [ ] **Spelling variants**: `"PR#416"`, `"pr #416"`, `"PR-416"`, `"pr-416"` each
      → `"pr:416"` (proves the broadened `[Pp][Rr][ -]?#?` pattern).
- [ ] **List value, multiple tokens**: `relates_to: ["PR #416", "#417"]` →
      `["pr:416", "pr:417"]` (proves the `normalize_pr_ref` loop handles multiple
      tokens in one value).
- [ ] **Mixed list**: a value mixing `"PR #416"` with an already-typed
      `"plan:2026-…"` ref **and** a `"pr-review:2026-06-17-pr-430-review"` ref
      (whose stem embeds a `pr-430` token) → only the `"PR #416"` token is
      rewritten; both typed refs are byte-unchanged (confirms `normalize_pr_ref`
      requires both delimiting quotes and does not corrupt an embedded `pr-NNN`).
- [ ] **Idempotency / no re-grab**: a corpus already carrying `target: "pr:416"`
      is **byte-unchanged** after a run (proves `normalize_bare`/`normalize_paths`
      do not re-grab the rewritten token), and a second `run_0007` over the
      migrated Phase 5 fixtures leaves an empty `meta/` diff.
- [ ] `bash skills/config/migrate/scripts/test-migrate-0007.sh` green; `mise run check`

#### Manual Verification

- [ ] On a real corpus, no `BAD-LINKAGE-SHAPE` violations from `PR #N` values.

---

## Phase 6: Integration guard — combined fixture corpus, validator-clean by construction

### Overview

Capstone. Assemble a single fixture corpus carrying **every** reproduction shape
and assert the mechanical passes leave it validator-clean and `harness_run` is
reached. This is the AC that binds the fix to a corpus the dogfood corpus could
not exercise.

### Changes Required

#### 1. Combined fixture corpus + guard

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`
**Changes**: Add a final `=== … ===` block seeding one repo containing:

- an empty-`type:` `meta/prs/240-description.md`;
- a `ticket: "PROJ-1234"` note with no `topic:`;
- a non-note type carrying `ticket_id:`;
- a `pr_title:` + `review_pass:` pr-review **with** and **without** a pre-existing
  `title:`;
- a fenced `note` missing `topic`; a fenced **titled** `codebase-research` missing
  `topic` (exercises the actual-title-not-`title_default` derivation);
- a review carrying empty placeholders `lenses: []` / `verdict: ""` (exercises the
  `is_empty_val` presence check);
- a fixture whose H1 carries a `;` (exercises the US-separator packed channel);
- a fenced `pr-430-review.md` pr-review lacking `pr_number`/`review_number`/
  `verdict`/`lenses`; a date-prefixed, pr-token-less `meta/prs/` file (exercises
  the pr_number date-strip → no fabricated year);
- a `target: "PR #416"` linkage plus a spelling variant (`"PR-417"`);
- a `meta/docs/logging-guide.md` freeform doc;

then assert: (a) `run_0007` exits 0 and is recorded applied; (b) `assert_validates`
over `$REPO/meta` passes; (c) `harness_run` is reached (`READY` frame /
completion); (d) **combined idempotency** — a second `run_0007` over the migrated
combined corpus leaves an empty `meta/` diff. Add the **regression guard** invoking
the migration through `run_backfill`+`run_rewrite` only (pre-harness) and asserting
zero validator violations on this corpus.

Use the shared `assert_validates` / `assert_violation` helpers introduced under
the assertion discipline (Implementation Approach) rather than re-pasting the
~9-line inline validator-clean idiom; retrofit the three existing inline
occurrences (`test-migrate-0007.sh:142,301,379`) to the helper in this phase so
the suite has one gate implementation.

The 0001→0007 integration fixtures (Phase 3 — both the same-session and the
0001-pre-applied cross-session cases) live in their own block since they need a
two-migration dir.

### Success Criteria

#### Automated Verification

- [ ] Combined-corpus block: `run_0007` exits 0, ledger records 0007, validator
      exits 0 over the full corpus.
- [ ] **Prepass coexistence**: the assembled multi-type corpus (work-item +
      pr-review + pr-description + research + note shapes together) passes
      `precondition_prepass` and reaches `run_rewrite` — so every transform AC tests
      its intended code path, not a prepass bail-out.
- [ ] **Broadened-namespace collision behaviour** (pins the new `pr-description`
      identity surface, not just "doesn't bail"): two `meta/prs/` files yielding the
      **same** post-rewrite id correctly **REFUSE**; a `pr-description` and a
      different-type artifact sharing a stem (distinct typed refs) correctly do
      **not** collide.
- [ ] Regression guard: `run_backfill + run_rewrite` → zero validator violations
      on the combined corpus.
- [ ] Full suite green: `bash skills/config/migrate/scripts/test-migrate-0007.sh`
- [ ] `mise run` (bare default task) exits 0 end-to-end.

#### Manual Verification

- [ ] Re-running `/accelerator:migrate` against the originally-failing downstream
      corpus now completes 0007 (reaches the interactive harness; records as
      applied) — the real-world unblock.

---

## Testing Strategy

### Unit / fixture Tests (`test-migrate-0007.sh`)

Each phase adds a labelled `=== … ===` block modelled on the existing path-shape
block (`:230-310`): seed `meta/**/*.md` via heredocs, `git_init`, `run_0007`,
assert on `fm_line` outputs, gate with the shared `assert_validates` helper (see
the assertion discipline in Implementation Approach). Author each fixture
**otherwise-complete** so the phase is independently green (e.g. a Phase 2
pr-review carries `verdict`/`lenses`/`review_number`/`pr_number`). Each phase pairs
its content assertions with (a) an `assert_violation` red-step on the pre-fix
corpus, (b) an idempotency re-run leaving an empty `meta/` diff, and (c) where the
transform fabricates or destroys a value, a breadcrumb assertion.

Shared helpers added to the suite (or `test-helpers.sh`): `assert_validates
<dir|files…>` (wraps the validator-clean idiom, replacing the three existing
inline copies) and `assert_violation <code> <files…>` (asserts a targeted
violation is present pre-fix). Where cheap, also unit-assert the pure shell
helpers directly (`extra_default`, `forbidden_keys_for_type`, `extras_for_type`)
so the schema-reading/default-deriving logic is pinpointed without a full corpus
run.

Key edge cases:

- Forbidden-key drop is schema-driven (custom `SCHEMA_TSV` with a novel key) +
  the header-name assertion fires on a reordered schema.
- `pr_title` fold three ways (no title / existing title / empty `pr_title`), with
  the discarded-key breadcrumb asserted and exactly one `title:` line emitted.
- `pr_number` is PR-anchored (date-prefixed stem → PR number, not the year); a
  digit-less stem does not abort under `set -euo pipefail`.
- Existing extra values never overwritten; quote-bearing titles produce valid YAML.
- `ticket`/`ticket_id` complementarity (0001→0007 same-session and 0001-pre-applied
  cross-session, no clobber) + dropped-legacy-key breadcrumb on a non-empty value.
- `pr:N` coercion over list/multi-token values; already-canonical `pr:N` unchanged.
- Sentinel backfill emits the `0007-DIVERGE[backfilled-extra]` breadcrumb (asserted).
- Every phase's migrated fixtures are idempotent on a second pass.

### Integration Tests

- 0001 → 0007 two-migration sequence on a `meta/tickets/` file, **same session**
  (Phase 3).
- 0001-pre-applied, 0007-runs-alone **cross-session** path over an already-renamed
  `meta/work/` corpus (Phase 3) — the common downstream ordering.
- Combined fixture corpus end-to-end + validator-clean-by-construction guard +
  combined idempotency (Phase 6).

### Manual Testing Steps

1. Run `/accelerator:migrate` on the originally-failing downstream corpus;
   confirm 0007 completes and is recorded applied.
2. Spot-check sentinel-backfilled reviews and folded titles read sensibly.
3. Confirm `meta/docs/` files are byte-unchanged.
4. **Recovery rehearsal**: if 0007 aborts mid-run (e.g. a deliberately broken
   fixture), confirm the working tree is recoverable with a single VCS revert of
   `meta/` — the migration mutates file-by-file with no batch transaction, so the
   dirty-tree guard + VCS revert is the documented recovery path.

## Performance Considerations

Negligible. The new work is per-file awk arms + a few `cut`/`grep` calls in the
shell driver; the corpus is small and the whole-corpus validation already runs
inside the migration's post-DONE watchdog. Sourcing one extra helper
(`doc-type-inference.sh`, `frontmatter-emission-rules.sh` in the migration) is
constant-cost.

## Migration Notes

This *is* a migration fix. It changes only the mechanical passes, so a corpus
that already migrated cleanly is unaffected. **Idempotency is asserted per phase
on the new reproduction shapes** (not merely re-confirmed via the existing
`=== Idempotency ===` block, which carries none of them) — each phase re-runs its
migrated fixtures and requires an empty `meta/` diff.

Sentinel `verdict`/`lenses` values are intentionally visible (`"unknown"`) and
logged via an asserted `0007-DIVERGE[backfilled-extra]` breadcrumb for human
follow-up; the neutral value (rather than a louder self-identifying string) plus
the breadcrumb is the accepted audit trail (a committed sentinel will not re-warn
on subsequent runs — a known limitation).

**Shared-helper / validator scope change.** Single-sourcing path classification
into `doc-type-inference.sh` and widening `out_of_scope` to skip `meta/docs/`
changes behaviour for **every** `validate-corpus-frontmatter.sh` consumer, not
just the migration: no `meta/docs/` file is validated by anyone after this change.
This is intended (freeform, plugin-unowned, no schema type) and is documented in
the validator's top-of-file comment.

**Recovery.** The migration mutates the corpus file-by-file (`atomic_write` per
file) with no batch transaction, so a mid-run abort leaves a partially-migrated
tree. Recovery is a single VCS revert of `meta/`, gated by the dirty-tree
precondition; this is signposted in the Manual Testing recovery rehearsal and
should be surfaced in the migration's abort log line.

## References

- Original work item: `meta/work/0114-fix-migration-0007-incomplete-mechanical-normalisation.md`
- Codebase research: `meta/research/codebase/2026-06-17-0114-migration-0007-incomplete-mechanical-normalisation.md`
- RCA: `meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md`
- Migration: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
- Awk rewrite: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
- Validator: `scripts/validate-corpus-frontmatter.sh:79-106` (path classification), `:322-344` (forbidden/legacy/extra checks)
- Schema: `scripts/templates-schema.tsv`; emission rules: `scripts/frontmatter-emission-rules.sh:74,88`
- 0001 complementarity: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:56-81`
- Test harness: `skills/config/migrate/scripts/test-migrate-0007.sh:230-310`
