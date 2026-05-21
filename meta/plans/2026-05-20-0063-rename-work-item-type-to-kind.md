---
date: "2026-05-20T22:45:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0063-rename-work-item-type-to-kind.md"
status: reviewed
---

# Rename work-item `type:` field to `kind:` — Implementation Plan

## Overview

Rename the work-item frontmatter field `type:` (semantic kind —
`story | epic | task | bug | spike`) to `kind:` across every producer
(template, helper scripts, work-skill and review-lens SKILL.md files,
visualiser frontend), every consumer (eval fixtures, graders), and every
existing work-item in this repo. Ship a new migration (0005) that
performs the corpus rewrite both here and in any downstream user repo,
and apply it to this repo's 72 work-items as part of the same atomic
delivery.

The rename frees `type:` for the unified artifact-type discriminator
introduced by ADR-0033, unblocks story 0070 (broader unified-schema
migration) and story 0065 (template updates).

## Current State Analysis

- **Template** (`templates/work-item.md:6,15`) declares `type: story #
  story | epic | task | bug | spike` in frontmatter and `**Type**:
  Story | Epic | Task | Bug | Spike` in the body.
- **Helper scripts** under `skills/work/scripts/` are largely
  field-name-agnostic, except `work-item-template-field-hints.sh:25`
  which has a hardcoded `case "$field" in type)` arm returning the
  five kind values. Tests in `test-work-item-scripts.sh` exercise this
  via `FIELD_HINTS type` with multiple `type: story|epic` fixture lines.
- **Work-skill SKILL.md** files (seven: `create-work-item`,
  `refine-work-item`, `list-work-items`, `update-work-item`,
  `review-work-item`, `extract-work-items`, `stress-test-work-item`)
  reference `type:` in prose, examples, table headers, and templates.
  The load-bearing reference is `update-work-item/SKILL.md:191` (field
  ↔ body-label sync table).
- **Review-lens SKILL.md** files: `completeness-lens` references the
  literal `type` field at four sites; `scope-lens`, `dependency-lens`,
  `testability-lens` branch on kind in prose only (no literal field
  reference).
- **Visualiser frontend**: `WorkItemCard.tsx:27` reads
  `entry.frontmatter['type']` as the only production literal-key
  access. Co-located `WorkItemCard.test.tsx` and incidental
  `KanbanBoard.test.tsx` fixtures use `type: ...`. The Rust backend
  treats frontmatter as `serde_json::Value` and needs no change.
- **Eval fixtures**: 137 fixture `.md` files (106 work-skill + 31 lens)
  carry `^type: <kind>`. **104** also carry `^\*\*Type\*\*:` body labels
  (every modern fixture; `0042-existing-*`, `0099-mismatched-id`, every
  `scenario-*/work-item.md`, every stress-test fixture, etc.). The
  heaviest single surface is
  `skills/work/refine-work-item/evals/files/` (89 files including
  scenario-11b/c stub fleets).
- **Eval graders**: three `Frontmatter: type` strings in `scope-lens`
  evals/benchmark JSON (line 19 of `evals.json`, lines 62 and 92 of
  `benchmark.json`). **Nine additional grader strings** in
  `stress-test-work-item/evals/{evals,benchmark}.json` and
  `create-work-item/evals/benchmark.json` reference `**Type**:` body
  labels literally (asserting the skill preserves the body-label form).
  These must also be updated. The remaining `"type":` JSON occurrences
  are grader-infrastructure false positives.
- **Migration framework** (ADR-0023): driver at
  `skills/config/migrate/scripts/run-migrations.sh` owns dirty-tree
  guard, preview banner, state-file append, skip handling. Migrations
  are bash files at
  `skills/config/migrate/migrations/NNNN-<slug>.sh`. The canonical
  frontmatter-key-rename pattern is `0001-rename-tickets-to-work.sh:53-69`.
- **Test harness**: `skills/config/migrate/scripts/test-migrate.sh`
  drives migration tests with fixtures at
  `skills/config/migrate/scripts/test-fixtures/NNNN/`. Uses
  `assert_eq`, `assert_contains`, `assert_file_exists` helpers from
  `scripts/test-helpers.sh`. Test runs are gated via
  `ACCELERATOR_MIGRATIONS_DIR` overrides so each test only sees a
  curated subset of migrations.
- **`meta/work/` corpus**: 72 work-items. 43 carry kind values
  (`story|epic|task|bug|spike`) with matching `**Type**:` body labels;
  29 carry legacy `type: adr-creation-task` (no body label). All bare
  unquoted YAML — sed-based rewrites are safe.

## Desired End State

After this plan is complete:

1. `migrations/0005-rename-work-item-type-to-kind.sh` exists, is
   idempotent, honours `paths.work` overrides, rewrites both
   `^type:` → `^kind:` (frontmatter) and `^\*\*Type\*\*:` →
   `^\*\*Kind\*\*:` (body label, unconditionally on the regex).
2. Migration 0005 has been applied to this repo. All 72 files under
   `meta/work/*.md` carry `^kind:`; no file carries `^type:` or
   `^\*\*Type\*\*:`.
3. Template, helper scripts, all eleven SKILL.md files, and the
   visualiser frontend reference `kind:` exclusively for the work-item
   semantic kind.
4. All 137 eval fixture `.md` files carry `^kind:`. The three scope-lens
   grader strings reference `Frontmatter: kind`.
5. The AC verification greps (story 0063, AC2 / AC4 / AC5 / AC6 / AC8 /
   AC9) all return zero hits modulo the documented false-positive
   exclusions.
6. Eval-suite pass rate is unchanged vs. the pre-rename baseline
   captured at the start of Phase 5.

### Key Discoveries

- Outer `grep -q '^old_key:'` guard + dual-presence sub-check is the
  established migration idempotence pattern
  (`migrations/0001-rename-tickets-to-work.sh:53-69`).
- Migration framework exports `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`,
  `ACCELERATOR_MIGRATION_MODE=1` to each migration. Migration MUST NOT
  touch the state file — driver appends to
  `.accelerator/state/migrations-applied` on success
  (`run-migrations.sh:204-205`).
- `bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work` honours
  `paths.work` overrides; returns path relative to project root.
- `atomic_write` from `scripts/atomic-common.sh` is the preferred
  rewrite primitive (migrations 0002-0004 use it; 0005 should too).
- Frontmatter typing in the visualiser is open
  (`Record<string, unknown>` in TS, `serde_json::Value` in Rust) —
  zero type-system changes needed.
- Test harness convention: per-migration fixtures live at
  `scripts/test-fixtures/NNNN/<scenario>/`, copied into `mktemp -d`
  test repos by helper functions in `test-migrate.sh`. Tests use
  `ACCELERATOR_MIGRATIONS_DIR` to gate which migrations the driver
  sees.

## What We're NOT Doing

- **Not** touching `agents/*.md`. Research confirmed every `type`
  reference there is unrelated (TypeScript types, HTML input types,
  `subagent_type` parameters, browser command `type`).
- **Not** touching the visualiser Rust backend
  (`skills/visualisation/visualise/server/`). The `"type"` literals in
  `sse_hub.rs` and `api/docs.rs` are unrelated to frontmatter.
- **Not** touching `type: work-item-review` references in
  `skills/work/review-work-item/SKILL.md:354` or in review-artifact
  frontmatter — this is the review-artifact's own type discriminator,
  a distinct schema.
- **Not** changing the semantic-kind vocabulary itself (`story | epic |
  task | bug | spike`) — only the field name.
- **Not** rewriting unrelated `type:` infrastructure JSON in graders
  (`"type": "llm"`, `"type": "contains"`, etc.) — those are
  grader-infrastructure types, not work-item kind.
- **Not** authoring or modifying migration 0070 — that story drops the
  rewrite step but is owned by its own work item.

## Implementation Approach

Strict sequencing inside a single atomic delivery (story 0063,
Sequencing sub-section):

1. Author migration 0005 first so the dogfood corpus can be migrated
   before any producer reads `kind:` from a file still on disk as
   `type:`.
2. Apply the migration in this repo, so `meta/work/*.md` carries
   `kind:` everywhere.
3. Land all producer-code changes (template, helpers, SKILL.md,
   visualiser frontend), eval-fixture rewrites, and grader updates in
   the same commit/PR as the migration so no consumer reads a `type:`
   frontmatter line from a producer that has already moved.

Within that ordering, **TDD applies to every phase with executable
tests**: migration 0005 (Phase 1), helper script (Phase 3), visualiser
frontend (Phase 4). For phases that rewrite content without
executable tests (template, SKILL.md prose, eval fixtures, graders —
Phases 5 and 6), the story's acceptance-criteria greps serve as the
oracle: run the greps first to confirm they hit, perform the rewrites,
re-run to confirm zero hits.

**Phase independence after Phase 2**: Phases 3, 4, 5, and 6 are
independent of each other and may be performed in any order (or in
parallel) once the migration has been applied. Phase 7 depends on all
prior phases.

---

## Phase 1: Migration 0005 — author with TDD

### Overview

Write test cases against the migration framework's harness first, then
author migration 0005 to satisfy them. The test cases exercise every
branch the migration must handle: clean rename, body-label rewrite,
idempotence (re-run no-op), partial prior run (both keys present),
`paths.work` override, files with no body label (legacy
`adr-creation-task` shape), and files with only a body label and no
frontmatter rewrite needed.

### Changes Required

#### 1. Test fixtures

**Directory**: `skills/config/migrate/scripts/test-fixtures/0005/`

Create ten fixture scenarios:

- `default-layout/` — repo root with one work-item under `meta/work/`
  carrying `type: story` frontmatter and a `**Type**: Story` body label.
- `legacy-adr-task/` — work-item carrying `type: adr-creation-task`
  with no body label (mirrors the 29 legacy items in `meta/work/`).
- `partial-prior-run/` — work-item carrying both `type: story` and
  `kind: story` (matching values; simulates an interrupted earlier run).
- `partial-prior-run-divergent/` — work-item carrying `type: bug` and
  `kind: story` (divergent values). Policy: `kind:` wins silently;
  `type:` is dropped. Asserts no error, asserts final state has only
  `kind: story`.
- `partial-prior-run-body-label/` — work-item carrying both
  `**Type**: Story` and `**Kind**: Story` body labels (interrupted body
  rewrite, matching values). Asserts stale `**Type**:` line is removed;
  `**Kind**: Story` remains.
- `partial-prior-run-body-label-divergent/` — work-item carrying both
  `**Type**: Bug` and `**Kind**: Story` body labels (divergent values).
  Asserts final state has only `**Kind**: Story`; `**Type**: Bug` is
  dropped per the same "kind wins" policy as the frontmatter divergent
  case.
- `paths-override/` — repo root with `.accelerator/config.md` setting
  `paths.work: docs/work` and a work-item containing `type: story` at
  `docs/work/0001-foo.md`. Default `meta/work/` is absent.
- `paths-override-missing/` — `.accelerator/config.md` sets
  `paths.work: docs/wrok` (typo); the directory does not exist. Asserts
  migration emits a stderr warning and exits 0 (so the driver records
  the migration as applied and downstream phases proceed). The warning
  string is verifiable via `2>&1 | grep`.
- `body-label-only/` — work-item carrying `kind: story` frontmatter but
  a stale `**Type**: Story` body label (asserts the body rewrite runs
  even when frontmatter is already migrated).
- `empty-work-dir/` — `.accelerator/config.md` sets `paths.work: docs/work`
  with `docs/work/` existing as an empty directory (no `.md` files).
  Asserts migration exits 0 with no-op.

Each fixture directory contains the modern config-file layout
(`.accelerator/config.md`) where a config is needed, plus the
work-item `.md` file(s). Only the `paths-override*` and `empty-work-dir`
fixtures need a config; the others rely on the default `meta/work`
path.

#### 2. Test harness extensions

**File**: `skills/config/migrate/scripts/test-migrate.sh`
**Changes**: Append a new `=== 0005 ===` test block. Add a setup
helper analogous to `setup_old_repo`, plus an `ONLY_0005_DIR` migrations
dir that contains only 0005 (so 0001-0004 don't run during these tests
and contaminate the assertions).

```bash
# A migrations directory containing only 0005, used by tests that
# focus on 0005 behaviour without earlier migrations running.
ONLY_0005_DIR="$TMPDIR_BASE/only-0005-migrations"
mkdir -p "$ONLY_0005_DIR"
cp "$MIGRATIONS_DIR/0005-rename-work-item-type-to-kind.sh" "$ONLY_0005_DIR/"

# setup_0005_repo: copies a fixture from test-fixtures/0005/ into mktemp.
# Fixtures contain no `.jj` or `.git` dir; tests pass
# ACCELERATOR_MIGRATE_FORCE_NO_VCS=1 to make the VCS-absence path explicit
# (mirroring the 0004 pattern at test-migrate.sh:1008).
setup_0005_repo() {
  local scenario="$1"
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-0005-XXXXXX")
  cp -R "$SCRIPT_DIR/test-fixtures/0005/$scenario/." "$repo_dir/"
  echo "$repo_dir"
}
```

Add tests, each invoking the driver with
`ACCELERATOR_MIGRATIONS_DIR=$ONLY_0005_DIR ACCELERATOR_MIGRATE_FORCE_NO_VCS=1`:

- **default-layout** — work-item ends up with `^kind: story` and
  `^\*\*Kind\*\*: Story`; no `^type:` or `^\*\*Type\*\*:` lines remain;
  driver state file lists migration 0005 as applied.
- **legacy-adr-task** — frontmatter renamed to
  `kind: adr-creation-task`; file has no body label before or after.
- **partial-prior-run** — only one `kind:` line remains; stale `type:`
  removed. Negative stderr assertion: `assert_not_contains "$stderr"
  'divergent type/kind'` (locks in the silent-on-match contract).
- **partial-prior-run-divergent** — concrete assertions:
  `assert_contains "$work_item" '^kind: story$'`,
  `assert_not_contains "$work_item" '^kind: bug'`,
  `assert_not_contains "$work_item" '^type:'`. Also assert the stderr
  warning fires: `assert_contains "$stderr" 'divergent type/kind'`.
- **partial-prior-run-body-label** — `assert_contains "$work_item"
  '^\*\*Kind\*\*: Story$'`, `assert_not_contains "$work_item"
  '^\*\*Type\*\*:'`. Frontmatter unchanged: `assert_contains
  "$work_item" '^kind: story$'`. Negative stderr assertion:
  `assert_not_contains "$stderr" 'divergent \*\*Type\*\*/\*\*Kind\*\*'`.
- **partial-prior-run-body-label-divergent** — `assert_contains
  "$work_item" '^\*\*Kind\*\*: Story$'`, `assert_not_contains
  "$work_item" '^\*\*Kind\*\*: Bug'`, `assert_not_contains "$work_item"
  '^\*\*Type\*\*:'`. Stderr assertion: `assert_contains "$stderr"
  'divergent \*\*Type\*\*/\*\*Kind\*\*'`.
- **paths-override** — `docs/work/0001-foo.md` renamed; `meta/work/`
  is left absent (not created).
- **paths-override-missing** — migration exits 0; stderr contains a
  warning naming the missing `paths.work` directory
  (`assert_contains "$stderr" 'work directory does not exist'`); no
  files modified; driver records 0005 as applied.
- **body-label-only** — body label rewritten:
  `assert_contains "$work_item" '^\*\*Kind\*\*: Story$'`,
  `assert_not_contains "$work_item" '^\*\*Type\*\*:'`. Frontmatter
  `kind:` unchanged: `assert_contains "$work_item" '^kind: story$'`.
- **empty-work-dir** — migration exits 0; no files created or modified
  (`find docs/work -type f | wc -l` equals 0 after the run); stdout
  contains `0005: rewrote 0 file(s) under docs/work`.
- **idempotent** — run twice; second run is no-op (exit 0; `tree_hash`
  of the repo's `meta/` subtree is byte-identical between runs, per the
  precedent established for migration 0002 in `test-migrate.sh:611-617`).

#### 3. Migration 0005

**File**: `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh` (new)
**Changes**: Author the migration to satisfy the tests above.

```bash
#!/usr/bin/env bash
# DESCRIPTION: Rename work-item type field to kind in frontmatter and body labels.
#
# Per-file logic — two independent passes, each mirroring 0001's
# dual-presence pattern (canonical reference: 0001-rename-tickets-to-work.sh:53-69):
#
#   1. Frontmatter:
#      - If both `type:` and `kind:` lines are present (partial prior
#        run), drop the stale `type:` line. Divergent values between
#        the two are NOT reconciled — `kind:` wins silently. A stderr
#        line names each file where divergence was detected.
#      - If only `type:` is present, rename in place.
#
#   2. Body label:
#      - Same dual-presence pattern: if both `**Type**:` and
#        `**Kind**:` lines exist, drop the stale `**Type**:`.
#      - Otherwise rename `**Type**:` → `**Kind**:` in place.
#      - The body-label rewrite is unconditional on the regex —
#        quoted/example `**Type**:` lines in body prose are also
#        rewritten. This matches the renamed template and avoids
#        per-value filtering.
#
# Each pass that fires issues one atomic_write per file. For files that
# need both passes, the file may briefly exist with new-shape frontmatter
# but old-shape body label between writes. The dual-presence guards in
# both passes make the migration idempotent against interruption — a
# subsequent run completes the transition cleanly.

set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"

source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

work_dir_rel="$(bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work)"

# Guard against config-resolver returning empty — would otherwise expand
# the find scope to the entire repo.
if [ -z "$work_dir_rel" ]; then
  log_die "0005: config-read-path.sh returned empty for 'work'"
fi

# Guard against pathological values that would expand find scope outside
# the intended work directory (`.` = project root, `/` or `/...` = filesystem
# root or absolute path, `..` or `..../..` = escape via parent traversal).
case "$work_dir_rel" in
  .|..|/|/*|*/..|../*|*/../*)
    log_die "0005: refusing dangerous paths.work value: $work_dir_rel"
    ;;
esac

work_dir="$PROJECT_ROOT/$work_dir_rel"

# Warn if the configured work directory does not exist (whether or not
# `paths.work` was set explicitly). Exit 0 so the driver records the
# migration as applied — there is nothing to rewrite, and a missing
# directory may be legitimate on a fresh checkout.
if [ ! -d "$work_dir" ]; then
  log_warn "0005: work directory does not exist: $work_dir_rel"
  exit 0
fi

rewrote=0
while IFS= read -r -d '' file; do
  touched=0

  # Pass 1: frontmatter key
  if grep -q '^type:' "$file" 2>/dev/null; then
    if grep -q '^kind:' "$file" 2>/dev/null; then
      # Partial prior run — drop stale `type:` line.
      # On divergent values (`type: bug` + `kind: story`), `kind:` wins;
      # emit a stderr line so the user can spot the divergence.
      old_type=$(grep -m 1 '^type:' "$file" | sed 's/^type:[[:space:]]*//')
      old_kind=$(grep -m 1 '^kind:' "$file" | sed 's/^kind:[[:space:]]*//')
      if [ "$old_type" != "$old_kind" ]; then
        log_warn "0005: divergent type/kind in $file — kept kind=$old_kind, dropped type=$old_type"
      fi
      grep -v '^type:' "$file" | atomic_write "$file"
    else
      sed 's/^type:/kind:/' "$file" | atomic_write "$file"
    fi
    touched=1
  fi

  # Pass 2: body label
  if grep -q '^\*\*Type\*\*:' "$file" 2>/dev/null; then
    if grep -q '^\*\*Kind\*\*:' "$file" 2>/dev/null; then
      # Partial prior run — drop stale `**Type**:` line.
      # On divergent values, `**Kind**:` wins; emit a stderr line.
      old_type_body=$(grep -m 1 '^\*\*Type\*\*:' "$file" | sed 's/^\*\*Type\*\*:[[:space:]]*//')
      old_kind_body=$(grep -m 1 '^\*\*Kind\*\*:' "$file" | sed 's/^\*\*Kind\*\*:[[:space:]]*//')
      if [ "$old_type_body" != "$old_kind_body" ]; then
        log_warn "0005: divergent **Type**/**Kind** body label in $file — kept Kind=$old_kind_body, dropped Type=$old_type_body"
      fi
      grep -v '^\*\*Type\*\*:' "$file" | atomic_write "$file"
    else
      sed 's/^\*\*Type\*\*:/**Kind**:/' "$file" | atomic_write "$file"
    fi
    touched=1
  fi

  rewrote=$((rewrote + touched))
done < <(find "$work_dir" -name '*.md' -print0)

echo "0005: rewrote $rewrote file(s) under $work_dir_rel"
```

### Success Criteria

#### Automated Verification

- [x] All new 0005 tests pass:
  `bash skills/config/migrate/scripts/test-migrate.sh` exits 0 and
  prints each new test as `PASS`.
- [x] Migration file is executable and bash-syntax-clean:
  `bash -n skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`.
- [x] Driver discovers 0005 in preview banner:
  `cd /tmp && CLAUDE_PLUGIN_ROOT=<repo> bash skills/config/migrate/scripts/run-migrations.sh --dry-run` (or equivalent) lists `0005` in the pending preview.

#### Manual Verification

- [x] Reading the migration against
  `0001-rename-tickets-to-work.sh:53-69` confirms the same dual-presence
  shape is applied independently in two sequential passes — one for the
  frontmatter key, one for the body label, each gated by its own
  `grep -q` outer guard and `grep -q` dual-presence sub-check.
- [x] Reading the tests confirms each branch is exercised at least
  once: clean rename, legacy-no-body-label, partial prior run (matching
  values), partial prior run (divergent values → `kind:` wins +
  stderr warning), partial prior run on body label (matching values),
  partial prior run on body label (divergent values), paths override,
  paths override pointing at missing dir (warning emitted), empty work
  dir, body-label-only, idempotent re-run (`tree_hash` byte-identical).

---

## Phase 2: Apply migration 0005 to this repo's `meta/work/`

### Overview

Run `/accelerator:migrate` against this repo so all 72 work-items
under `meta/work/` carry `kind:` frontmatter (and the 43 with body
labels also carry `**Kind**:`). This step is mechanical but must occur
before any producer code is rewritten — otherwise existing work-items
become unreadable.

### Changes Required

#### 1. Pre-flight: capture baselines

- Commit Phase 1 (migration script + tests + fixtures) as its own jj
  revision before running `/accelerator:migrate`. The driver's
  dirty-tree guard only scopes to `^(meta/|\.claude/accelerator|\.accelerator/)`
  so Phase 1 staged changes under `skills/config/migrate/` would not
  actually block the migration — but committing first means any
  subsequent `jj op restore` rolls back only the migration's data
  effects, not the migration script itself.
- Confirm `jj status` is clean.
- Capture the pre-rename eval-suite baseline to a gitignored scratch
  artefact at `meta/scratch/0063-eval-baseline.txt`. The artefact is
  the input to Phase 7's AC10 comparison and must be preserved (do not
  delete until Phase 7 is complete). Concretely:

  ```bash
  mkdir -p meta/scratch
  {
    echo "# Pre-rename eval-suite baseline — captured $(date -u +%FT%TZ)"
    echo "# Plan: meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md"
    echo "# jj revision: $(jj log -r @ --no-graph -T 'change_id.shortest()' 2>/dev/null)"
    echo
    for skill_dir in skills/work/*/evals skills/review/lenses/*/evals; do
      [ -d "$skill_dir" ] || continue
      runner="$skill_dir/run.sh"
      [ -x "$runner" ] || continue
      echo "=== $skill_dir ==="
      bash "$runner" --summary 2>&1 | tail -5
      echo
    done
  } > meta/scratch/0063-eval-baseline.txt
  ```

  If a skill's eval runner has a different invocation, document the
  per-skill command inline in this file before running. The Phase 7
  AC10 comparison re-runs the same script and diffs the two artefacts
  (see Phase 7 "Eval-suite baseline comparison").

  **Tolerance**: LLM-judge graders are non-deterministic. AC10 passes
  if (a) no fixture flips from PASS → FAIL, and (b) the aggregate
  pass-rate delta per suite is within ±3%. Any new failure outside
  this band is enumerated and attributed in the PR description.

#### 2. Run the migration

```bash
/accelerator:migrate
```

#### 3. Verification of the corpus state

```bash
# Zero `^type:` frontmatter lines remain
rg -n '^type:' meta/work/

# Zero `^**Type**:` body labels remain
rg -n '^\*\*Type\*\*:' meta/work/

# Every file has a `^kind:` line
fd -e md . meta/work/ -x sh -c '
  if ! grep -q "^kind:" "$1"; then echo "MISSING kind: $1"; fi' sh {}

# Every previously-labelled file has a `**Kind**:` body label
# (Expect 43 of the 72 files — the 29 legacy adr-creation-task files
# have no body label by design)
rg -lc '^\*\*Kind\*\*:' meta/work/ | wc -l

# Migration is recorded as applied
grep -F '0005-rename-work-item-type-to-kind' \
  .accelerator/state/migrations-applied
```

#### 4. Re-run idempotency check

```bash
/accelerator:migrate
# Expect: exit 0, "No pending migrations" (or driver reports 0005
# as already applied), and `jj status` shows no further changes under
# meta/work/.
jj status | grep 'meta/work/' && echo "FAIL: post-rerun changes" || echo "PASS: idempotent"
```

### Success Criteria

#### Automated Verification

- [x] `rg -n '^type:' meta/work/` returns zero hits.
- [x] `rg -n '^\*\*Type\*\*:' meta/work/` returns zero hits.
- [x] All 72 `.md` files under `meta/work/` contain a `^kind:` line.
- [x] `.accelerator/state/migrations-applied` contains
  `0005-rename-work-item-type-to-kind`.
- [x] Re-running `/accelerator:migrate` produces exit 0 and zero
  further changes under `meta/work/`.

#### Manual Verification

- [x] Spot-check three representative files:
  `meta/work/0063-rename-work-item-type-to-kind.md`,
  `meta/work/0001-three-layer-review-system-architecture.md` (legacy
  adr-creation-task shape), `meta/work/0072-playwright-daemon-cjs-import-bug.md`.
  All three have `^kind:` instead of `^type:`; the modern shapes also
  have `^\*\*Kind\*\*:` instead of `^\*\*Type\*\*:`; the legacy shape
  has no body label.
- [x] `jj diff meta/work/` review shows only the expected line-level
  rewrites — no other content changes.

---

## Phase 3: Helper script — TDD

### Overview

Update `work-item-template-field-hints.sh` so the hardcoded fallback
arm responds to `kind` instead of `type`. Update the test suite first
to reflect the new contract, see it fail, then update production.

### Changes Required

#### 1. Update tests first

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes**: Only edit fixture lines and helper invocations that
exercise the work-item *kind* semantic. Two groups:

**(a) Load-bearing edits — rename to `kind`:**

- `FIELD_HINTS type` → `FIELD_HINTS kind` at lines 1006-1008, 1057,
  1088-1089 (and any tripwire block that asserts the helper returns
  the five kind values).
- `type: story|epic|task|bug|spike` fixture lines that exist purely
  to feed `FIELD_HINTS` / kind-semantic tests at lines 685, 695, 699,
  703, 767, 1081.

**(b) Explicitly exempt — leave verbatim:**

- Lines 805, 809, 812, 818, 821 (`sub.type: foo` / `READ_FIELD
  "sub.type"` literal-dot regression test for
  `work-item-read-field.sh`). The point of these tests is regex
  literal-dot handling, not kind semantics. Renaming would erode the
  test rationale without strengthening coverage.
- Line 690 (`sub.type: foo` fixture for test 16a/b literal-dot
  matching) — same rationale.
- Line 1029 (`type: feature` inside the "user-overridden template with
  custom values" test). This test exercises that field-hints accepts
  arbitrary user vocabularies; rewriting the fixture to `kind: feature`
  would still test the same path but rewriting the field name to a
  non-plugin convention (e.g. `category: feature`) preserves the
  "arbitrary field name" coverage more honestly. If the test stays
  with `type:`, add a comment explaining why this `type:` is left as
  legacy fixture text.

Verify with grep after editing:

```bash
# Load-bearing residue check — should be zero:
rg -n 'FIELD_HINTS\s+type\b' \
  skills/work/scripts/test-work-item-scripts.sh
# Kind-vocabulary fixture residue — should be zero outside the carve-outs:
rg -n '^\s*type:\s*(story|epic|task|bug|spike)\b' \
  skills/work/scripts/test-work-item-scripts.sh
```

#### 2. Run tests; expect failure

```bash
bash skills/work/scripts/test-work-item-scripts.sh
# Expect: the field-hints assertions fail because production still
# returns kind values for `type` rather than `kind`.
```

#### 3. Update production

**File**: `skills/work/scripts/work-item-template-field-hints.sh`
**Changes**:
- Line 7 docstring: replace `type/status/priority` reference with
  `kind/status/priority`.
- Line 25 case arm: `type)` → `kind)`.

#### 4. Run tests; expect pass

```bash
bash skills/work/scripts/test-work-item-scripts.sh
# Expect: all tests pass.
```

### Success Criteria

#### Automated Verification

- [ ] `bash skills/work/scripts/test-work-item-scripts.sh` exits 0.
- [ ] `rg -n 'type\)' skills/work/scripts/work-item-template-field-hints.sh`
  returns zero hits (the `kind)` arm replaces it).
- [ ] `bash skills/work/scripts/work-item-template-field-hints.sh kind`
  prints exactly five lines: `story`, `epic`, `task`, `bug`, `spike`.
- [ ] `bash skills/work/scripts/work-item-template-field-hints.sh type`
  prints nothing (falls through to the wildcard case).

#### Manual Verification

- [ ] The dynamic-hint path (when the template's trailing comment is
  parseable) still works: spot-check by running the helper against the
  Phase 5 updated template.

---

## Phase 4: Visualiser frontend — TDD

### Overview

Update `WorkItemCard.tsx` to read `frontmatter['kind']` instead of
`frontmatter['type']`. Update its co-located tests first, see them
fail, then update production. Also update incidental fixture data in
`KanbanBoard.test.tsx`.

### Changes Required

#### 1. Update tests first

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.test.tsx`
**Changes**: Touch sites identified by research: lines 27, 42, 54, 65
(test title `frontmatter.type is missing` → `frontmatter.kind is
missing`), 70, 83, 96, 108. Replace `type:` keys in fixture frontmatter
objects with `kind:`. Leave line 33 alone (asserts on the rendered
string `'adr-creation-task'` — content not key).

While editing the missing-kind test, strengthen the assertion from
`expect(screen.queryByText(/undefined/)).toBeNull()` (which only
catches accidental `undefined` rendering) to also assert that the
kind chip element is not in the DOM:

```ts
expect(container.querySelector(`.${styles.cardKind}`)).toBeNull();
```

This catches the mutation class "kindLabel defaults to empty string"
that the current `queryByText(/undefined/)` cannot.

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx`
**Changes**: Lines 149, 153, 157, 170, 181 — replace fixture `type:`
keys with `kind:`. No assertions reference these.

Verify with grep:

```bash
rg -n "frontmatter:\s*\{[^}]*\btype\s*:" \
  skills/visualisation/visualise/frontend/src/routes/kanban/
# Expect zero hits after this edit.
```

#### 2. Run tests; expect failure

```bash
cd skills/visualisation/visualise/frontend && pnpm test \
  src/routes/kanban/WorkItemCard.test.tsx
# Expect: assertions on the rendered type/kind label fail because
# production still reads `frontmatter['type']`.
```

#### 3. Update production

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx`
**Changes**:
- Line 27: `entry.frontmatter['type']` → `entry.frontmatter['kind']`.
- Line 27: `const fmType` → `const fmKind`.
- Line 28: `const typeLabel = ...fmType...` →
  `const kindLabel = ...fmKind...`.
- Line 55: `{typeLabel !== null && ... {typeLabel} ...}` →
  `{kindLabel !== null && ... {kindLabel} ...}`.
- Line 55: `className={styles.cardType}` → `className={styles.cardKind}`.

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.module.css`
**Changes**:
- Rename the `.cardType` CSS class to `.cardKind`. Single key in the
  module; no other references in the component tree (verified by
  `rg 'cardType' skills/visualisation/`).

**DO NOT touch** line 39 — `params={{ type: 'work-items', fileSlug }}`
is the TanStack Router path-param name for `/library/$type/$fileSlug`,
unrelated to the frontmatter field.

#### 4. Run tests; expect pass

```bash
cd skills/visualisation/visualise/frontend && pnpm test \
  src/routes/kanban/WorkItemCard.test.tsx \
  src/routes/kanban/KanbanBoard.test.tsx
# Expect: all tests pass.
```

#### 5. Full frontend test pass

```bash
cd skills/visualisation/visualise/frontend && pnpm test
# Expect: no regressions in other tests.
```

### Success Criteria

#### Automated Verification

- [ ] `pnpm test` in
  `skills/visualisation/visualise/frontend/` exits 0.
- [ ] `pnpm typecheck` (or `tsc --noEmit`) in the same dir exits 0.
- [ ] `rg -n "frontmatter\[['\"]type['\"]\]|frontmatter\.type"
  skills/visualisation/visualise/frontend/src/routes/kanban/`
  returns zero hits (story 0063 AC2 second grep).
- [ ] `rg -n 'cardType' skills/visualisation/` returns zero hits
  (CSS class rename complete).

#### Manual Verification

- [ ] Run the visualiser dev server, navigate to the Kanban board, and
  confirm every work-item card displays its kind label (story / epic /
  task / bug / spike / adr-creation-task) as before.
- [ ] Confirm no visual regression in card layout — same chip
  position, same typography, same spacing.

---

## Phase 5: Template + SKILL.md content updates

### Overview

Rewrite the template and all eleven affected SKILL.md files. This phase
has no executable tests; the story's acceptance-criteria greps (AC2,
AC3, AC4) serve as the oracle. Run them before editing to confirm they
hit, perform the edits, re-run to confirm zero hits.

### Pre-edit baseline greps

```bash
# AC2 first grep — kind-bearing `type:` lines anywhere
rg -n '(?:^|[^.\w])type:\s*(story|epic|task|bug|spike)\b' \
  | rg -v 'work-item-review|subagent_type'
# Expect: many hits (template, SKILL.md prose, fixtures, etc.).
# This is the baseline.
```

### Changes Required

#### 1. Template

**File**: `templates/work-item.md`
**Changes**:
- Line 6: `type: story                                  # story | epic | task | bug | spike`
  → `kind: story                                  # story | epic | task | bug | spike`
  (preserve trailing comment — it's the dynamic-hint source consumed
  by `work-item-template-field-hints.sh:62-67`).
- Line 15: `**Type**: Story | Epic | Task | Bug | Spike` →
  `**Kind**: Story | Epic | Task | Bug | Spike`.

#### 2. Work-skill SKILL.md files (seven)

Per research, exact touch sites by file:

- **`skills/work/create-work-item/SKILL.md`** — lines 57, 115, 238,
  254-256, 275, 322, 344, 521-523, 527, 562.
- **`skills/work/refine-work-item/SKILL.md`** — lines 31, 83, 130,
  189-191 (child kind derivation rule), 363-366 (canonical tree fence
  template), 370, 402.
- **`skills/work/list-work-items/SKILL.md`** — lines 5, 48, 52
  (invokes `work-item-template-field-hints.sh type` — change to
  `kind`), 58, 74, 80, 87, 184, 195, 225 (table header `| ID | Title |
  Type | ...` → `| ID | Title | Kind | ...`), 246-249, 272, 298, 302.
- **`skills/work/update-work-item/SKILL.md`** — load-bearing rule at
  line 191 (field ↔ body-label sync table: `` `type` ↔ `**Type**: ` ``
  → `` `kind` ↔ `**Kind**: ` ``); line 273 (enumerated label list);
  lines 279-280 (legacy-item examples).
- **`skills/work/review-work-item/SKILL.md`** — lines 77 (reword
  "type-appropriate content" → "kind-appropriate content"), 91-92, 173.
  **DO NOT touch line 354** — `type: work-item-review` is the
  review-artifact's own type, not the work-item kind.
- **`skills/work/extract-work-items/SKILL.md`** — lines 34, 70-71,
  181-182, 188, 191, 210, 257, 309, 502-504, 507. **DO NOT touch**
  lines 224 and 403 (generic English usage of "type", unrelated).
- **`skills/work/stress-test-work-item/SKILL.md`** — lines 166-167
  (frontmatter immutability rule listing `type` and `**Type**:` body
  label).

#### 3. Review-lens SKILL.md files (four)

- **`skills/review/lenses/completeness-lens/SKILL.md`** — literal
  `type` field references at lines 28, 36, 54, 96; section headings
  at lines 26, 76, 94, 96; kind-conditional rules at lines 29-35,
  78-92, 107-108, 133-136 (where field name appears).
- **`skills/review/lenses/scope-lens/SKILL.md`** — kind-conditional
  prose only; no literal field reference. Read lines 34, 43-66, 87-96,
  113-118, 131 to confirm; only edit if a prose reference to the
  field name slips through.
- **`skills/review/lenses/dependency-lens/SKILL.md`** — same as
  scope-lens; read lines 55-56, 89-102, 109-111.
- **`skills/review/lenses/testability-lens/SKILL.md`** — same; read
  lines 31-40, 69-78, 103-105.

### Verification grep (post-edit)

```bash
# AC2 first grep
rg -n '(?:^|[^.\w])type:\s*(story|epic|task|bug|spike)\b' \
  | rg -v 'work-item-review|subagent_type'
# Expect: zero hits outside `skills/work` and `skills/review/lenses`
# eval fixtures (those are addressed in Phase 6) and outside `meta/work/`
# (addressed in Phase 2).
```

### Success Criteria

#### Automated Verification

- [ ] AC2 first grep returns zero hits in the producer surface:
  `rg -n '(?:^|[^.\w])type:\s*(story|epic|task|bug|spike)\b'
  templates/ skills/work/*/SKILL.md
  skills/review/lenses/{completeness,scope,dependency,testability}-lens/SKILL.md
  | rg -v 'work-item-review|subagent_type'` — zero hits.
- [ ] Template body label updated:
  `rg -n '^\*\*Kind\*\*:' templates/work-item.md` returns exactly
  one hit (line 15); `rg -n '^\*\*Type\*\*:' templates/work-item.md`
  returns zero hits.
- [ ] `list-work-items/SKILL.md` invokes the helper with the new
  argument: `grep -n 'work-item-template-field-hints.sh kind'
  skills/work/list-work-items/SKILL.md` returns at least one hit;
  `grep -n 'work-item-template-field-hints.sh type'` returns zero
  hits.
- [ ] `update-work-item/SKILL.md:191` field ↔ body-label table maps
  `` `kind` `` to `` `**Kind**: ` ``: verified by
  `rg -n '\`kind\`\s*↔\s*\`\*\*Kind\*\*:' skills/work/update-work-item/SKILL.md`.

#### Manual Verification

- [ ] Read each of the eleven SKILL.md files and confirm every
  remaining `type:` mention refers to either the artifact-type
  discriminator (`type: work-item-review`) or a TanStack/HTML/agent
  `type` — never the work-item semantic kind.
- [ ] Confirm cross-references between SKILL.md files (e.g.
  `update-work-item` → `create-work-item`) still resolve and use
  consistent vocabulary.

---

## Phase 6: Eval fixtures + grader strings

### Overview

Mechanically rewrite 137 fixture work-item `.md` files and three
scope-lens grader strings. No executable test gates this phase; the AC
greps (AC6) serve as the oracle.

### Changes Required

#### 1. Mass rewrite of fixture work-items

**Scope**: 137 `.md` files across:

- `skills/work/create-work-item/evals/files/` (7)
- `skills/work/refine-work-item/evals/files/` (89)
- `skills/work/review-work-item/evals/files/` (1)
- `skills/work/stress-test-work-item/evals/files/` (6)
- `skills/review/lenses/clarity-lens/evals/files/` (5)
- `skills/review/lenses/completeness-lens/evals/files/` (5)
- `skills/review/lenses/dependency-lens/evals/files/` (8)
- `skills/review/lenses/scope-lens/evals/files/` (9)
- `skills/review/lenses/testability-lens/evals/files/` (5)

**Rewrite mechanism** — uses the same two-pass per-file logic as
migration 0005, routed through `atomic_write` for atomicity and
EXIT-trap cleanup. 104 of the 137 fixture files carry `^\*\*Type\*\*:`
body labels (verified: `rg -c '^\*\*Type\*\*:' skills/**/evals/files/`),
so the body-rewrite pass is required.

Run from the repo root:

```bash
set -euo pipefail
source scripts/atomic-common.sh

rewrote=0
while IFS= read -r -d '' file; do
  touched=0

  if grep -q '^type:' "$file" 2>/dev/null; then
    if grep -q '^kind:' "$file" 2>/dev/null; then
      grep -v '^type:' "$file" | atomic_write "$file"
    else
      sed 's/^type:/kind:/' "$file" | atomic_write "$file"
    fi
    touched=1
  fi

  if grep -q '^\*\*Type\*\*:' "$file" 2>/dev/null; then
    if grep -q '^\*\*Kind\*\*:' "$file" 2>/dev/null; then
      grep -v '^\*\*Type\*\*:' "$file" | atomic_write "$file"
    else
      sed 's/^\*\*Type\*\*:/**Kind**:/' "$file" | atomic_write "$file"
    fi
    touched=1
  fi

  rewrote=$((rewrote + touched))
done < <(find skills/work skills/review/lenses \
  -path '*/evals/files/*.md' -print0)

echo "Phase 6: rewrote $rewrote fixture file(s)"
```

Recommended: run the loop against
`skills/work/create-work-item/evals/files/` first (7 files), inspect
`jj diff` to confirm the rewrite shape, then re-run against the full
surface. If the shape is wrong, `jj op restore` reverts cleanly.

#### 2. Scope-lens grader strings

Three exact-string edits (research §6 Graders):

- **File**: `skills/review/lenses/scope-lens/evals/evals.json`
  - Line 19: `"located in Summary or Frontmatter: type"`
    → `"located in Summary or Frontmatter: kind"`.

- **File**: `skills/review/lenses/scope-lens/evals/benchmark.json`
  - Line 62: replace both `"Finding is located in Summary or
    Frontmatter: type"` and `"Location: 'Frontmatter: type'."` with
    the `kind` variants.
  - Line 92: replace `"Finding is in Frontmatter: type, Requirements,
    or Stories"` and `"Major finding in Frontmatter: type"` with the
    `kind` variants.

#### 3. stress-test-work-item and create-work-item grader strings

These graders assert that work-skills preserve or modify the body
`**Type**:` label literally. After Phase 2 (corpus) and Phase 6 §1
(fixtures) rewrite body labels to `**Kind**:`, these grader strings
must follow or they will assert against evidence that no longer
exists. Verified via
`rg -n '\*\*Type\*\*' skills/work/*/evals/{evals,benchmark}.json`
(9 hits across 3 files):

- **File**: `skills/work/stress-test-work-item/evals/evals.json`
  - Line 159 (`expected_output` string) and line 167 (grader
    description): replace every `**Type**` occurrence with `**Kind**`.
- **File**: `skills/work/stress-test-work-item/evals/benchmark.json`
  - Lines 183, 185, 297, 299: replace every `**Type**` occurrence
    with `**Kind**`. Line 299 in particular references
    `'**Type**: Story'` literally — update to `'**Kind**: Story'`.
- **File**: `skills/work/create-work-item/evals/benchmark.json`
  - Line 552: replace `**Type**` with `**Kind**` (occurs once in the
    evidence string `'**Title**: keep existing 'story', **Type**: ...'`).

Do **not** touch any other `"type":` JSON occurrence in any grader
file — those are grader-infrastructure types (`"type": "llm"`,
`"type": "contains"`, etc.) per research §6.

### Success Criteria

#### Automated Verification

- [ ] AC6 first grep:
  `rg -n '^type:\s*(story|epic|task|bug|spike)\b' skills/work
  skills/review/lenses` returns zero hits.
- [ ] AC6 second grep:
  `rg -n '"type"\s*:\s*"(story|epic|task|bug|spike)"' -g '*evals.json'
  -g '*benchmark.json'` returns zero hits.
- [ ] Body-label grep (fixture corpus):
  `rg -n '^\*\*Type\*\*:' skills/work skills/review/lenses` returns
  zero hits.
- [ ] Body-label grep (grader JSON):
  `rg -n '\*\*Type\*\*' skills/work/*/evals/*.json` returns zero
  hits.
- [ ] `rg -n 'Frontmatter: type' skills/review/lenses/scope-lens/evals/`
  returns zero hits. Broaden as a defence-in-depth check:
  `rg -n 'Frontmatter: type' skills/` returns zero hits.
- [ ] `rg -lc '^kind:' skills/work/*/evals/files/
  skills/review/lenses/*/evals/files/ | wc -l` returns 137.
- [ ] Eval JSON files remain valid JSON:
  `find skills -name 'evals.json' -o -name 'benchmark.json' \
    | xargs -I{} python3 -c 'import json,sys; json.load(open(sys.argv[1]))' {}`
  exits 0 for each.

#### Manual Verification

- [ ] Spot-check three fixture files across different surfaces (a
  `create-work-item` fixture, a `refine-work-item` scenario-11b stub,
  a `scope-lens` fixture) to confirm only the field name changed —
  no incidental edits.
- [ ] Read the three edited scope-lens grader strings in context to
  confirm the surrounding prose still parses and asserts on the
  intended evidence location.

---

## Phase 7: Final verification

### Overview

Run the full set of story-0063 acceptance-criteria greps across the
whole repo. Run the eval suites and compare against the baseline
captured in Phase 2. Re-run the migration to verify end-to-end
idempotency.

### Verification commands

```bash
# AC2 first grep — kind-bearing `type:` lines anywhere
rg -n '(?:^|[^.\w])type:\s*(story|epic|task|bug|spike)\b' \
  | rg -v 'work-item-review|subagent_type'
# Expect: zero hits.

# AC2 second grep — visualiser kanban literal-key access
rg -n "frontmatter\[['\"]type['\"]\]|frontmatter\.type" \
  skills/visualisation/visualise/frontend/src/routes/kanban/
# Expect: zero hits.

# AC6 first grep — fixture corpus
rg -n '^type:\s*(story|epic|task|bug|spike)\b' \
  skills/work skills/review/lenses
# Expect: zero hits.

# AC6 second grep — graders
rg -n '"type"\s*:\s*"(story|epic|task|bug|spike)"' \
  -g '*evals.json' -g '*benchmark.json'
# Expect: zero hits.

# AC8 corpus
rg -n '^type:' meta/work/
rg -n '^\*\*Type\*\*:' meta/work/
# Expect: both return zero hits.

# Whole-repo body-label sweep (defence-in-depth against Phase 5/6
# misses in producer SKILL.md examples or fixture corpus)
rg -n '^\*\*Type\*\*:' templates/ skills/ meta/work/
# Expect: zero hits.

# Grader-JSON body-label sweep
rg -n '\*\*Type\*\*' skills/work/*/evals/*.json \
  skills/review/lenses/*/evals/*.json
# Expect: zero hits.

# Legacy-kind sanity check (kind value outside the canonical five)
rg -n '^type:\s*adr-creation-task' .
# Expect: zero hits.

# Migration idempotency
/accelerator:migrate
jj status | grep 'meta/work/' \
  && echo "FAIL: post-rerun changes" \
  || echo "PASS: idempotent"
```

### Eval-suite baseline comparison (AC10)

Re-run the exact capture script from Phase 2 §1 and write the result
to `meta/scratch/0063-eval-post.txt`. Diff against the baseline:

```bash
# Re-run the same capture as Phase 2 §1, redirecting to a new file
# (see Phase 2 §1 for the full script body).
<re-run capture script> > meta/scratch/0063-eval-post.txt
diff -u meta/scratch/0063-eval-baseline.txt meta/scratch/0063-eval-post.txt
```

**Tolerance** (per Phase 2 §1):

- AC10 passes if no fixture flips from PASS → FAIL and per-suite
  aggregate pass rate is within ±3% of baseline.
- Any new failure outside this band must be enumerated and attributed
  in the PR description ("rename-caused" → fix in this PR;
  "pre-existing flakiness" → document and cross-link the prior known
  flake).

Both `meta/scratch/0063-eval-baseline.txt` and
`meta/scratch/0063-eval-post.txt` are gitignored scratch artefacts;
attach them to the PR description if any per-fixture variance needs
context.

### Frontend smoke test

```bash
cd skills/visualisation/visualise/frontend && pnpm test
cd skills/visualisation/visualise/frontend && pnpm typecheck
```

### Migration framework regression

```bash
bash skills/config/migrate/scripts/test-migrate.sh
# Expect: all tests (including the new 0005 ones) pass.
```

### Success Criteria

#### Automated Verification

- [ ] All AC2 / AC6 / AC8 greps above return zero hits.
- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` exits 0.
- [ ] `bash skills/work/scripts/test-work-item-scripts.sh` exits 0.
- [ ] `pnpm test` and `pnpm typecheck` in
  `skills/visualisation/visualise/frontend/` exit 0.
- [ ] Re-running `/accelerator:migrate` produces exit 0 and no
  further changes (`jj status` shows nothing new under `meta/work/`).
- [ ] All fixture `.md` files under
  `skills/work/*/evals/files/` and
  `skills/review/lenses/*/evals/files/` contain a `^kind:` line; none
  contains a `^type:` line.

#### Manual Verification

- [ ] Eval-suite pass rates match the Phase 2 baseline, or any new
  failure is enumerated and attributed in the PR description.
- [ ] Visualiser dev server renders the Kanban board correctly with
  every card showing its kind label.
- [ ] Reading the diff for `meta/work/` confirms only line-level field
  and body-label rewrites — no other edits.
- [ ] Reading the diff for SKILL.md files confirms no stale references
  to `type:` for the semantic kind.

---

## Testing Strategy

### Unit-level tests (driven by TDD in Phases 1, 3, 4)

- **Migration 0005**: five branch tests in `test-migrate.sh` (default
  layout, legacy adr-task, partial prior run, paths override, body
  label only) plus an idempotency test.
- **Helper script**: `test-work-item-scripts.sh` exercises
  `FIELD_HINTS kind` (returns the five values), `FIELD_HINTS type`
  (returns nothing, falls through wildcard), and dynamic-hint flow
  against the renamed template.
- **Visualiser**: `WorkItemCard.test.tsx` exercises both the
  "kind is present" and "kind is absent" rendering branches.

### Integration tests

- **End-to-end migration**: Phase 2 runs `/accelerator:migrate` against
  this repo (72 files), exercising the whole driver+migration stack.
- **Eval suites**: Phase 7 baseline-comparison run.

### Manual testing steps

1. Open the Kanban board in the visualiser dev server. Verify each
   card shows its kind label (story / epic / task / bug / spike /
   adr-creation-task).
2. Open `meta/work/0063-rename-work-item-type-to-kind.md` in the
   visualiser's library view; confirm the rendered frontmatter shows
   `kind: story`.
3. Create a new work-item via `/accelerator:create-work-item`; confirm
   the wizard prompts for `kind` (not `type`) and writes a file
   matching the new template.
4. Update an existing work-item's kind via
   `/accelerator:update-work-item`; confirm both the frontmatter
   `kind:` line and the body `**Kind**:` label are kept in sync per
   the rule at `update-work-item/SKILL.md:191`.

## Performance Considerations

None. The migration is a 137-file (eval) + 72-file (corpus) sed pass —
sub-second on any modern disk. The visualiser change is a single
property-key rename. No hot-path implications.

## Migration Notes

This plan **is** the migration. The downstream story 0070 must drop
its own `type:` → `kind:` rewrite step. As part of this PR, update
`meta/work/0070-ship-meta-corpus-unified-schema-migration.md` to
remove the line `- Renames work-item type: → kind:.` from its
Requirements section and the corresponding `kind:` mention from its
Acceptance Criteria. Migration 0005 owns this rewrite once.

When migration 0005 ships to user repos, it will be applied
automatically by the next `/accelerator:migrate` run; the dirty-tree
guard and idempotency contract make this safe. User repos with
`paths.work` overrides will have their custom path honoured — the
Phase 1 `paths-override` test covers this branch.

### Breaking change for downstream user repos

This rename is a **breaking schema change** to work-item frontmatter.
After upgrading the plugin and before running `/accelerator:migrate`,
user repos will experience a window where:

- The visualiser kanban card shows no kind chip (frontmatter `kind:`
  is undefined on legacy work-items).
- LLM-driven flows that read `kind:` from work-items will not find
  the value until the migration runs.
- The dirty-tree guard refuses to run the migration if `meta/` has
  uncommitted changes — users must commit or set
  `ACCELERATOR_MIGRATE_FORCE=1` to proceed.
- Users running the visualiser dev server should restart it after
  `/accelerator:migrate` completes. The pre-migration JS bundle still
  reads `frontmatter['type']` and will silently render no kind chip
  even after the underlying data is correct.

User-side scripts that pass `type` as a field name to
`work-item-read-field.sh` (or that grep `^type:` in work-item
markdown) will break after migration runs. There is no transitional
alias.

This story's PR description and the next plugin release's CHANGELOG
must call out:
1. The breaking rename of the work-item `type:` field to `kind:`.
2. The need to run `/accelerator:migrate` immediately after upgrade.
3. The body-label rewrite is **unconditional on the regex** — any
   `^**Type**:` line in a work-item body (including quoted/example
   lines) will be rewritten. Users with legitimate non-kind
   `**Type**:` mentions in work-item prose should review the
   migration diff and revert specific lines via `jj` if needed.

## References

- Source: `meta/work/0063-rename-work-item-type-to-kind.md`
- Research: `meta/research/codebase/2026-05-20-0063-rename-work-item-type-to-kind.md`
- Approved review: `meta/reviews/work/0063-rename-work-item-type-to-kind-review-1.md` (verdict APPROVE on pass 4)
- Migration framework: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Canonical migration model: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:53-69`
- Body-rewrite precedent: `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh`
- Atomic rewrite helper: `scripts/atomic-common.sh` (`atomic_write`)
- Config-path resolver: `scripts/config-read-path.sh`
- Test harness: `skills/config/migrate/scripts/test-migrate.sh`
- Test helpers: `scripts/test-helpers.sh`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Unified-schema ADR (motivator): `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Blocked stories: `meta/work/0065-update-artifact-templates-to-unified-schema.md`, `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
