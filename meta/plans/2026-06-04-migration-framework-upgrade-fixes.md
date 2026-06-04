---
type: plan
id: "2026-06-04-migration-framework-upgrade-fixes"
title: "Migration Framework Upgrade-Failure Fixes Implementation Plan"
date: "2026-06-04T15:54:32+00:00"
author: "Toby Clemson"
producer: create-plan
status: draft
work_item_id: ""
parent: ""
reviewer: ""
tags: [migrate, run-migrations, bash-compatibility, merge-move, shell-lint]
revision: "03b648e48e3ccf390c71cc1e622e350d4c5fc450"
repository: "accelerator"
last_updated: "2026-06-04T15:54:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Migration Framework Upgrade-Failure Fixes Implementation Plan

## Overview

A consumer on system bash 3.2 upgraded the plugin and hit a cascade of
migration failures (0003 abort on a re-created `tmp/`, a mid-batch dirty-tree
re-check in 0004, and `declare -A` crashing 0006 under bash 3.2). The research
at `meta/research/codebase/2026-06-04-migration-upgrade-failures.md` resolved
every design question. This plan executes the four actionable fixes plus a
regression harness, test-first, as independent phases:

1. Wire the migrate test suites into the build/CI as a regression net.
2. Replace the abort-on-conflict relocation policy with a shared **merge-move**.
3. Remove migration 0004's internal dirty-tree / no-VCS pre-flight.
4. Port the bash-4 `declare -A` usages (0006, `config-dump.sh`) to bash 3.2.
5. Add shell format + lint tooling (shfmt + ShellCheck + a bashisms grep-lint),
   split into `check`/`fix` task families wired to CI.

Findings 3 (orchestrator pre-flight) and 5 (0002 perpetual no-op) are
deliberate **no-changes** per the research and are out of scope.

## Current State Analysis

- **Relocation moves abort instead of merging.** `0003:_move_if_pending`
  (`skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64`)
  fatally `exit 1`s when source and destination both exist (`0003:44-56`),
  invoked for six path pairs (`0003:197-205`). `0004` has the same
  anti-idempotent policy in a different shape: `_check_collisions`
  (`0004:259-274`) aborts the whole migration if any planned destination exists,
  and its `_move_if_pending` (`0004:276-283`) is a bare file-level `mv`. **0001**
  carries the *same* policy at its origin: two collision checks
  (`0001-rename-tickets-to-work.sh:71-87`) abort when both the legacy and the new
  directory exist (default paths only), then Step 4 renames via bare `mv`
  (`0001:89-107`). Many users have not yet run 0001, so it must merge too. Note
  0001 does **not** currently source `atomic-common.sh` (only `config-common.sh`
  at `0001:7`).
- **0004 re-polices the tree mid-batch.** `_preflight_scan_corpus_clean`
  (`0004:171-207`, called at `0004:207`) hard-fails on no-VCS
  (`0004:181-187`, gated by `ACCELERATOR_MIGRATE_FORCE_NO_VCS`) and `exit 1`s if
  the scan corpus is dirty (`0004:201-205`) — which trips on the legitimate
  changes earlier migrations in the same batch made. **`build_scan_corpus`
  (`0004:155-169`) is shared with Step 3's inbound-link scan (`0004:498,556-560`)
  and must be retained.**
- **Bash-4 associative arrays.** `0006:315` (`declare -A WALKED`) and `0006:361`
  (`declare -A TEMPLATE_PATHS`) crash under bash 3.2. A latent third instance
  lives at `scripts/config-dump.sh:107` (`declare -A DEFAULTS`).
- **No bash-3.2 enforcement and the migrate suites don't run in CI.**
  `run_shell_suites` (`tasks/test/helpers.py:13-34`) only scans
  `scripts`, `skills/visualisation/visualise`, `skills/decisions`, `hooks`,
  `skills/github` (`tasks/test/integration.py`) — never `skills/config/migrate`.
  Repo-wide grep confirms no `test-migrate` reference in `tasks/`, `mise.toml`,
  or `.github/`. CI runs `mise run test` on `ubuntu-latest` (bash 5.x), so
  `declare -A` passed silently.

### Key Discoveries

- **Test harness shape.** `test-migrate.sh` is a flat top-to-bottom script
  (no runner framework, no `main`); it sources shared asserts from
  `scripts/test-helpers.sh` (`assert_eq:20-31`, `assert_contains:33-44`,
  `assert_stderr_contains:256-270`, `test_summary:347-357`) and ends with a bare
  `test_summary` (`test-migrate.sh:2021`). Per-test fixtures are `mktemp -d`
  under one `TMPDIR_BASE` with an EXIT trap.
- **Fixture VCS patterns.** `setup_old_repo` (`test-migrate.sh:46-58`) = real
  no-VCS dir; 0002/0004 fixtures use a bare `mkdir .git` or no VCS dir at all;
  dirty-tree tests use a real `git init` + commit + append (`test-migrate.sh:306-310`).
- **Exact tests to update:** 0003 conflict abort (`test-migrate.sh:912-923`),
  0004 collision abort (`test-migrate.sh:1077-1087` and `632-639`), 0004 no-VCS
  refusal (`test-migrate.sh:1103-1108`), `run_0004` helper (`test-migrate.sh:1004-1010`),
  0006 alias dedup (`test-migrate.sh:1973-1982`, `1987-1999`).
- **Task wiring.** New task families register in `tasks/__init__.py`
  (import tuple at `:3-14`, `ns.add_collection(Collection.from_module(...))` at
  `:30-39`) and `mise.toml`. Shell suites run via `run_shell_suites(context, subtree)`
  (`tasks/test/helpers.py`), which requires the **exec bit** and excludes
  `test-helpers.sh`. `shellcheck = "0.11.0"` is pinned (`mise.toml:10`); `shfmt`
  is mise-installable (`aqua:mvdan/sh`) and not yet present.
- **bash-3.2 idiom to follow:** parallel indexed arrays + a linear-search
  out-param helper, as in `scripts/interactive-harness.sh:42-67`; lighter
  `case " $str " in *" $x "*)` set-membership as in `scripts/config-summary.sh:49-64`.
  `config-dump.sh`'s `DEFAULTS`→`REVIEW_DEFAULTS` port is fully self-contained
  (the sourced `PATH_KEYS`/`WORK_KEYS`/… arrays in `config-defaults.sh` are
  already indexed; `config-defaults.sh` is **not** touched).
- **Exec-bit gap:** `test-migrate.sh` is `-rwxr-xr-x`;
  `test-migrate-interactive.sh` and `test-migrate-snapshot.sh` are `-rw-r--r--`.
- **Snapshot suite caveat:** `test-migrate-snapshot.sh` byte-compares against
  checked-in fixtures and regenerates with `ACCELERATOR_MIGRATE_SNAPSHOT_REGEN=1`;
  removing 0004's pre-flight changes its stderr, so any 0004 snapshot must be
  regenerated. Snapshots may be platform-sensitive (generated on macOS).

## Desired End State

- A re-run after a partial/aborted migration **converges** instead of
  deadlocking: directory relocations in 0003 and 0004 merge source into an
  existing target (source wins on leaf collisions), then remove the empty source.
- A clean-start `/accelerator:migrate` completes in a **single** invocation:
  0004 no longer re-checks the tree mid-batch and no longer hard-fails without
  VCS. The orchestrator's single per-invocation clean-tree gate is unchanged.
- 0006 and `config-dump.sh` run under bash 3.2 with byte-identical behaviour.
- `mise run check` (shfmt + ShellCheck + bashisms, read-only) passes locally and
  in a dedicated CI job; `mise run default` auto-formats locally via the `fix`
  variants; `mise run test` now also exercises the migrate suites.

Verify: `mise run test` and `mise run check` both green; a manual upgrade replay
on a bash-3.2 host (or `bash3.2`-equivalent) completes in one pass.

## What We're NOT Doing

- **No orchestrator change** (`run-migrations.sh:67-141`). The single
  per-invocation clean-tree gate and `ACCELERATOR_MIGRATE_FORCE` are retained
  verbatim (finding 3).
- **No 0002 change.** The `no_op_pending` perpetual-pending behaviour is
  intended and pinned by `test-migrate.sh:522-535` (finding 5).
- **No `ACCELERATOR_MIGRATE_FORCE_NO_VCS` retention** — the env var and its
  plumbing are deleted (we assume users are on VCS).
- **No new abort/preview/confirm UX.** VCS revert remains the recovery model.
- Option B from the research (running suites under a real bash 3.2 in CI) is
  **not** adopted; the bashisms lint is the bash-3.2 guard.

## Implementation Approach

Test-driven and phase-independent. Phase 1 lands the regression harness first so
Phases 2–4 are protected as they change migrations and their tests together.
Phases 1–4 are mutually independent at the source level (different files /
different regions of 0004).

Phase 5 (format + lint) sequences **last** — it has more coupling to the earlier
phases than a single constraint:

- Its bashisms gate fails until Phase 4's `declare -A` port lands (the
  originally-noted constraint).
- Its repo-wide `format:scripts:fix` reformat rewrites the very files Phases 2–4
  edit (`0001`/`0003`/`0004`/`0006`, the new `fs-common.sh`, `config-dump.sh`),
  so landing the reformat before those phases would guarantee merge churn. Either
  Phase 5's reformat lands after 2–4, or all earlier-phase shell is authored in
  shfmt-canonical 2-space style up front.
- `merge_move` (Phase 2) and any new shell (`fs-common.sh`,
  `lint-bashisms.sh`) must themselves pass the Phase 5 format/lint checks
  regardless of merge order — see Phase 2's success criteria.

Each fix phase follows red→green: write/adjust the failing test against the new
contract first, watch it fail, then implement until green.

---

## Phase 1: Wire migrate test suites into the build and CI

### Overview

Make `test-migrate.sh`, `test-migrate-interactive.sh`, and
`test-migrate-snapshot.sh` run under `mise run test` (hence CI), giving the later
phases a regression net. No production code changes.

### Changes Required

#### 1. Make the two non-executable suites discoverable

**Files**: `skills/config/migrate/scripts/test-migrate-interactive.sh`,
`skills/config/migrate/scripts/test-migrate-snapshot.sh`
**Changes**: add the exec bit (`chmod +x`) so `run_shell_suites`' `os.access(..., X_OK)`
filter discovers them. `test-migrate.sh` already has it.

#### 2. New invoke task for the migrate subtree

**File**: `tasks/test/integration.py`
**Changes**: add a `migrate` task mirroring `config`/`github`.

```python
@task
def migrate(context: Context):
    """Integration tests for the meta-directory migration framework."""
    run_shell_suites(context, "skills/config/migrate")
```

#### 3. Expose it via mise and add to the integration aggregate

**File**: `mise.toml`
**Changes**: add a `test:integration:migrate` block and append it to
`test:integration`'s `depends` list (`mise.toml:129-139`).

```toml
[tasks."test:integration:migrate"]
description = "Run meta-directory migration framework tests (shell harnesses)"
run = "invoke test.integration.migrate"
```

### Snapshot-suite portability gate

`test-migrate-snapshot.sh` byte-compares checked-in fixtures and may have been
generated on macOS. Run it on Linux during this phase. If it fails for
platform reasons (not a real regression), **do not** add its exec bit — leave it
manual-only and wire just `test-migrate.sh` + `test-migrate-interactive.sh` —
and record the exclusion in this plan's Migration Notes. Do not paper over a
platform-flaky snapshot by globbing it into CI.

### Success Criteria

#### Automated Verification:

- [x] `invoke test.integration.migrate` discovers and runs all wired suites
- [x] Discovery is asserted non-zero: the task fails loudly (not silently skips)
      if the expected suite count isn't found — guards against an exec-bit
      dropped on an exec-bit-lossy filesystem leaving the regression net unrun
- [x] Migrate suites pass: `mise run test:integration:migrate`
- [x] Full integration aggregate still green: `mise run test:integration`
- [ ] Whole suite green: `mise run test` (deferred to a consolidated run; Phase 1
      changes don't touch unit/e2e)

#### Manual Verification:

- [ ] On `ubuntu-latest` (or a Linux runner), the snapshot suite passes; if not,
      it is intentionally excluded and the reason is recorded
- [ ] CI's `test` job log shows the migrate suites executing

> **Implementation note (Phase 1).** All three suites are wired (exec bit added
> to `-interactive` and `-snapshot`). The snapshot suite's only *active* case is
> `baseline-no-pending`; cases 0002–0006 auto-skip (no `seed.sh` fixtures). The
> baseline snapshot is platform-independent — `stdout` is the literal
> `No pending migrations.`, `stderr` is empty, exit code `0`, and `files.sha256`
> is a content hash of `migrations-applied` — so wiring it on Linux CI is safe;
> no platform exclusion was needed. The discovery-count guard lives in
> `tasks/test/integration.py` (`_EXPECTED_MIGRATE_SUITES = 3`) backed by
> `run_shell_suites` now returning the discovered list.
>
> **Knock-on for Phase 3 §3:** there is **no 0004 snapshot** to regenerate (0004
> has no `seed.sh`), so that step is a no-op.

---

## Phase 2: Shared merge-move helper (replace abort-on-conflict)

### Overview

Add a reusable `merge_move` to a new `scripts/fs-common.sh` and route 0001's,
0003's, and 0004's directory relocations through it. Both-present states
**merge** instead of aborting; on a same-named leaf-file collision the **source
overwrites the target**.

### Changes Required

#### 1. Failing tests first (red)

**File**: `skills/config/migrate/scripts/test-migrate.sh`

> **Anchor on banners, not line numbers.** `test-migrate.sh` is a flat
> 2000+-line script with no named tests, so every line reference below drifts
> the moment an earlier test is rewritten (and the `183-184` reference is
> already the Test 8 banner, not its assertions). Before each edit, locate the
> target by its unique `echo "Test: …"` banner string rather than the cited
> line range, and execute Phases 2→3→4 in order so later phases re-grep against
> the already-shifted file.

- **Rewrite the 0001 collision test (`183-184`, Test 8).** Pre-create
  `meta/work/` (and, for the review pair, `meta/reviews/work/`) alongside the
  legacy `meta/tickets/` with an overlapping filename of differing content; run
  the driver/0001 and assert: exit 0; `meta/work/` holds the merged set with the
  **source** content winning the overlap; `meta/tickets/` removed. (Replaces the
  abort + "cannot proceed" assertions.) Cover both the `tickets→work` and
  `reviews/tickets→reviews/work` pairs.
- **Rewrite the 0003 conflict test (`912-923`).** Re-create
  `.accelerator/config.md` with differing content, run the driver, then assert:
  exit 0; `.accelerator/config.md` now holds the **source** (`.claude/accelerator.md`)
  content; `.claude/accelerator.md` is gone. (Replaces the abort + both-preserved
  assertions.)
- **Add a directory-merge test for 0003.** Pre-create `.accelerator/tmp/` with a
  file `keep.md` while `meta/tmp/` holds `new.md` and a same-named `keep.md` with
  differing content. Assert: exit 0; `.accelerator/tmp/new.md` and
  `.accelerator/tmp/keep.md` (source content) both present; `meta/tmp/` removed.
- **Rewrite the 0004 collision tests (`1077-1087` and `632-639`).** Pre-create
  the destination file, run 0004, then assert: exit 0; destination holds the
  **moved source** content; source removed.

Run `bash skills/config/migrate/scripts/test-migrate.sh` and confirm the new
assertions fail against current code.

#### 1b. Dedicated `merge_move` unit harness (new)

**File**: `scripts/test-merge-move.sh` (new; exec bit set, wired into the
`config` shell suites via `run_shell_suites`)
**Changes**: source `scripts/fs-common.sh` directly and drive every branch of
`merge_move` in isolation — coverage the migration integration tests cannot give
(they exercise only each migration's fixed path-pairs). Cases:

- dest-absent plain move; missing-source no-op.
- file-onto-file leaf collision → source content wins (byte-compare).
- **type mismatch both directions** — source *file* over destination *directory*,
  and source *directory* over destination *file* — asserting source-wins-wholesale
  and that the displaced destination subtree is gone (this is the
  `rm -rf -- "$dst"` branch the migration tests never reach).
- nested same-named-subdir merge (recursion); leaf collision inside a merged dir.
- filenames with spaces; dotfile-only source (e.g. only a `.keep`) merges and the
  source dir is removed.
- empty-source directory → source removed, no error.
- **partial/interrupted convergence** — pre-populate `dst` so exactly one of
  several entries is already present; assert a second `merge_move` run converges
  and the source is emptied.
- **unsafe-destination refusal** — assert `merge_move` returns non-zero (and
  deletes nothing) for an empty `$dst`, `"/"`, a trailing-slash `$dst`, and a
  `..`-escaping `$dst`, pinning the path-safety bug-guard.

Mutation check: flipping `rm -rf` → `rm -f`, or the type-mismatch operator, must
fail at least one assertion.

#### 2. Add `merge_move` to a new relocation-focused helper

**File**: `scripts/fs-common.sh` (new)
**Changes**: create a relocation/merge helper module and add a bash-3.2-compatible
recursive merge helper. It lives in its own module (not `atomic-common.sh`)
because `merge_move` is explicitly **non-atomic** — the opposite of that file's
atomic-write contract — and because 0001 should not have to pull in
`atomic-common.sh`'s transitive JSONL/lock machinery just to relocate
directories. 0001/0003/0004 source `fs-common.sh` for it (see routing sections
below).

```bash
# merge_move <src> <dst>
#   Move <src> onto <dst>, merging directories recursively. When the
#   destination is absent it is a plain move. When both are directories each
#   entry of <src> is merged into <dst>; same-named leaf files are overwritten
#   by the source (source-wins). The now-empty source directory is removed.
#   Missing <src> is a no-op.
#   NON-ATOMIC: a per-entry mv/rm sequence, so a mid-merge failure leaves a
#   partially-merged tree — a re-run converges (idempotent), VCS is the recovery
#   net. Same filesystem assumed (cross-fs mv is itself non-atomic on POSIX).
#   bash 3.2 compatible (no globstar/nullglob deps).
merge_move() {
  local src="$1" dst="$2"
  [ -e "$src" ] || return 0
  # Cheap bug-guard: refuse a destination that could escalate the rm/mv below
  # into a wide delete — empty, root, a trailing-slash empty leaf (e.g. an empty
  # rel component yielding "$PROJECT_ROOT/"), or a '..' escape. The current
  # callers all pass "$PROJECT_ROOT/<fixed-rel>", so this only fires on a future
  # caller bug. (Recovery is VCS; this guards a code bug, not a no-VCS user.)
  case "$dst" in
    "" | "/" | */)        echo "merge_move: refusing unsafe destination '$dst' for '$src'" >&2; return 1 ;;
    *"/../"* | */..)      echo "merge_move: refusing path-escaping destination '$dst'" >&2; return 1 ;;
  esac

  if [ ! -e "$dst" ]; then
    mkdir -p "$(dirname "$dst")"
    mv "$src" "$dst"
    return 0
  fi

  # Type mismatch or leaf collision: source wins wholesale. The non-empty
  # guard above ensures rm never targets an unintended path; `--` stops a
  # leading-dash path being read as an option.
  if [ ! -d "$src" ] || [ ! -d "$dst" ]; then
    rm -rf -- "$dst"
    mkdir -p "$(dirname "$dst")"
    mv "$src" "$dst"
    return 0
  fi

  # Both directories — merge each entry of src into dst. The three globs match
  # regular files, then dotfiles excluding '.' and '..' ('.[!.]*' and '..?*'),
  # so no basename '.'/'..' filter is needed (the old `basename "$src/.."`
  # never equalled '..' anyway). '[ -e ]' guards each unmatched literal glob.
  local entry name
  for entry in "$src"/* "$src"/.[!.]* "$src"/..?*; do
    [ -e "$entry" ] || continue
    name="${entry##*/}"
    merge_move "$entry" "$dst/$name"
  done
  # Source should now be empty; a non-empty source signals a non-converging
  # merge — surface it rather than swallowing the rmdir failure with `|| true`.
  if ! rmdir "$src" 2>/dev/null; then
    echo "merge_move: source '$src' not empty after merge — left in place" >&2
  fi
}
```

#### 3. Route 0003 through `merge_move`

**File**: `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh`
**Changes**: add a `source "$PLUGIN_ROOT/scripts/fs-common.sh"` alongside the
existing `atomic-common.sh` source; delete the both-present conflict branch
(`0003:44-56`); make `_move_if_pending` a thin wrapper that resolves rel→abs,
calls `merge_move`, and records `MOVED_THIS_RUN`. The six call sites
(`0003:197-205`) are unchanged.

```bash
_move_if_pending() {
  local rel_src="$1" rel_dst="$2"
  local src="$PROJECT_ROOT/$rel_src" dst="$PROJECT_ROOT/$rel_dst"
  [ -e "$src" ] || return 0
  merge_move "$src" "$dst"
  MOVED_THIS_RUN+=("$rel_src → $rel_dst")
}
```

#### 4. Route 0004 through `merge_move`, delete `_check_collisions`

**File**: `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh`
**Changes**: add a `source "$PLUGIN_ROOT/scripts/fs-common.sh"` alongside the
existing `atomic-common.sh` source; delete `_check_collisions` (`259-274`) and
its call (`274`); replace the file-level `_move_if_pending` body (`276-283`) so
the planned-moves loop (`285-288`) routes each `src→dst` through `merge_move`.

```bash
_move_if_pending() {
  local rel_src="$1" rel_dst="$2"
  local src="$PROJECT_ROOT/$rel_src" dst="$PROJECT_ROOT/$rel_dst"
  [ -e "$src" ] || return 0          # also gates the move-record below, so kept
  merge_move "$src" "$dst"
  echo "0004: moved $rel_src → $rel_dst"
}
```

> Keep 0003's and 0004's wrapper parameter names identical (`rel_src`/`rel_dst`)
> so the two thin shims read the same. The `[ -e "$src" ] || return 0` guard
> duplicates `merge_move`'s own missing-source no-op, but it is **retained on
> purpose**: it also gates the per-move bookkeeping (`MOVED_THIS_RUN` /
> `echo "0004: moved …"`) so a no-op source is not falsely logged as moved.

> Note: `_rename_user_template_file_if_present` (`0004:419-429`) keeps its own
> `log_die` on a pre-existing destination — it renames a single user template
> and is **not** a relocation move; left unchanged (out of the merge contract).

#### 5. Route 0001 through `merge_move`, delete its collision checks

**File**: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
**Changes**: source the relocation helper (0001 does not yet) — add
`source "$PLUGIN_ROOT/scripts/fs-common.sh"` after `0001:7`. (Sourcing the
lean `fs-common.sh` rather than `atomic-common.sh` avoids pulling 0001 into the
JSONL/lock machinery it does not use.) Delete both
Step 3 collision checks (`0001:71-87`). Replace the Step 4 bare `mv` renames
(`0001:91-107`) with `merge_move`, keeping the default-path gating
(`tickets_is_default` / `review_tickets_is_default`) and the frontmatter
rewrites in Step 2 unchanged.

```bash
if [ "$tickets_is_default" -eq 1 ] && [ -d "$tickets_dir" ]; then
  merge_move "$tickets_dir" "$work_dir"
fi

if [ "$review_tickets_is_default" -eq 1 ] && [ -d "$review_tickets_dir" ]; then
  merge_move "$review_tickets_dir" "$review_work_dir"
fi
```

> This also removes the `mv`-fails-on-regular-file "retry-safe" behaviour
> documented at `0001:93-96`: under the merge contract a regular-file
> destination is overwritten by the source directory (source-wins), consistent
> with 0003/0004.
>
> Note on frontmatter scope: 0001's Step 2 `ticket_id:`→`work_item_id:` rewrite
> walks `$tickets_dir` *before* the relocation, so in the newly-tolerated
> both-present case, files that already lived in `meta/work/` are not visited by
> Step 2 (and `merge_move` does not rewrite content). This is acceptable —
> 0006 later canonicalises `work_item_id` corpus-wide — but the merge red test
> should confirm the destination-resident files are left for 0006 rather than
> silently expected to be canonical, so the partial-postcondition is understood.
>
> Run at least one 0001 merge test **through the orchestrator** (not just direct
> invocation) so the real dispatch-context `source fs-common.sh` path —
> `PLUGIN_ROOT` resolution under PATH `bash` — is exercised, not just the merge
> outcome.

#### 6. CHANGELOG entry for the behaviour change

**File**: `CHANGELOG.md`
**Changes**: add an entry under `## [Unreleased]` (currently empty) recording
the consumer-facing behaviour change — relocations in 0001/0003/0004 now **merge**
into an existing target and **overwrite same-named leaf files source-wins**, where
previously a both-present collision aborted the migration. Place it under
`### Changed` (or `### Breaking` if the team treats a migration-behaviour change
as breaking), matching the existing migration-framework entries' style. The
plan's Migration Notes document this for reviewers; this surfaces it through the
project's normal release-notes channel for consumers.

### Success Criteria

#### Automated Verification:

- [x] Dedicated `merge_move` unit harness passes, covering every branch incl.
      both type-mismatch directions and partial-merge convergence:
      `bash scripts/test-merge-move.sh` (36 assertions; wired via the `config`
      suite). Mutation-checked: `rm -rf`→`rm -f` and the type-mismatch `||`→`&&`
      each fail ≥1 assertion.
- [x] New/rewritten 0001 + 0003 + 0004 merge tests pass: `mise run test:integration:migrate`
- [x] No collision-abort strings remain in the relocation migrations:
      `! grep -rn 'Manually merge or remove\|destination collision\|conflict — both' …` (clean)
- [x] Idempotency holds — re-running a completed migration is a byte-identical
      no-op (existing `tree_hash` idempotency tests stay green)
- [ ] `merge_move` passes the Phase 5 format/lint checks once landed
      (`mise run format:scripts:check` + `mise run lint:scripts:check`) — deferred to Phase 5
- [ ] Full suite green: `mise run test` — deferred to a consolidated run

> **Deviations (Phase 2).** (1) The plan's 0004 collision-test line reference
> `632-639` was stale — at the current revision that region is the unrelated
> **0002** target-file collision test (`setup_0002_repo`), and 0002 is out of
> scope (not routed through merge_move), so it was left untouched. The only
> 0004 relocation-collision test is "collision at destination …", which was
> rewritten. (2) Test 4 "Failed migration aborts without updating state file"
> previously induced its failure via 0001's now-removed abort (pre-creating
> `meta/work` as a regular file); it was re-targeted to a failing **stub
> migration** so it still exercises the orchestrator's failure→state contract
> without depending on the removed abort.

#### Manual Verification:

- [ ] Reproduce the original report: a stale `meta/tmp/` plus a fresh
      `.accelerator/tmp/` (e.g. a `pr-body-*.md`) now merges in one pass with no
      abort, and the VCS diff shows the expected overwrite-by-source on any
      genuine leaf collision

---

## Phase 3: Remove migration 0004's internal dirty-tree / no-VCS pre-flight

### Overview

Delete `_preflight_scan_corpus_clean` and the `ACCELERATOR_MIGRATE_FORCE_NO_VCS`
plumbing. The orchestrator stays the single clean-tree gate. **Keep
`build_scan_corpus`** — Step 3 depends on it.

### Changes Required

#### 1. Adjust tests first (red)

**File**: `skills/config/migrate/scripts/test-migrate.sh`

- **Delete the no-VCS refusal test (`1103-1108`).** Its `assert_neq` +
  `"no VCS detected"` no longer holds.
- **Add a no-VCS success test.** `setup_0004_repo default-layout` (no `.git`/`.jj`),
  run 0004 **without** any force env var, assert exit 0 and that the research
  files moved into `…/codebase/`.
- **Simplify `run_0004` (`1004-1010`)** to drop `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1`.
- **Add a mid-batch sibling-dirty test.** Through the orchestrator with a real
  committed git repo, apply the batch so 0001/0003 dirty the tree before 0004;
  assert 0004 no longer aborts with `"scan corpus has uncommitted changes"`.
  Pair this negative assertion with a **positive postcondition** — exit 0 **and**
  the research files actually relocated into `…/codebase/` after the
  sibling-dirtied batch — so the test proves convergence, not merely the absence
  of one error message (a bare absence-of-string check would also pass if 0004
  failed downstream or silently no-op'd).
- **Add a VCS-present clean-tree 0004 test.** Deleting the no-VCS refusal test
  would otherwise leave *zero* coverage of 0004 against a real (committed,
  clean) VCS working copy — yet `build_scan_corpus` is retained and still
  inspects VCS state for Step 3's inbound-link scan. Add a 0004-direct test in a
  committed-clean `git init` fixture asserting the research files move into
  `…/codebase/` **and** the Step 3 inbound-link rewrite still succeeds, so the
  retained VCS-touching code path keeps a regression test.

#### 2. Remove the pre-flight from 0004

**File**: `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh`
**Changes**: delete `_preflight_scan_corpus_clean` (`171-207`) and its call
(`207`). **Retain** `build_scan_corpus` (`155-169`), but re-home/relabel it: its
"Step 0c: scan-corpus dirty-tree pre-flight" section header no longer describes
what remains, so move it adjacent to its sole surviving consumer (Step 3's
inbound-link scan) and retitle it for that purpose (corpus enumeration), so a
future maintainer isn't misled about why it exists. Remove the only two
`ACCELERATOR_MIGRATE_FORCE_NO_VCS` references (inside the deleted block). The
jj op-id breadcrumb (`16-23`) and the no-legacy-dirs short-circuit (`148-152`)
remain.

#### 3. Regenerate 0004 snapshots if present

**File**: `skills/config/migrate/scripts/test-migrate-snapshot.sh` fixtures
**Changes**: if 0004 is snapshotted, its stderr changed (pre-flight lines gone);
run with `ACCELERATOR_MIGRATE_SNAPSHOT_REGEN=1` and commit the regenerated
fixtures. Wholesale regen overwrites the `files.sha256` artefact manifest and
the exit code too, not just stderr — so to avoid baptising a real regression as
the new baseline, review the diff **by dimension**: the `files.sha256` manifest
and exit code for the 0004 fixture must be **unchanged** (only stderr should
differ here), and enumerate the exact pre-flight stderr lines expected to
disappear. Regenerate only **after** Phase 2's merge tests are independently
green, so any filesystem change from the `merge_move` reroute is already pinned
by assertions rather than silently absorbed into the snapshot.

### Success Criteria

#### Automated Verification:

- [x] No-VCS 0004 run succeeds without any force env var: `mise run test:integration:migrate`
- [x] Mid-batch sibling-dirty no longer aborts 0004 (regression test through the
      orchestrator asserts exit 0 **and** the research file relocated)
- [x] No `ACCELERATOR_MIGRATE_FORCE_NO_VCS` references remain (incl. tests/CI):
      `! grep -rn ACCELERATOR_MIGRATE_FORCE_NO_VCS skills/ scripts/ tasks/` (clean)
- [x] `build_scan_corpus` retained (re-homed next to Step 3, retitled for corpus
      enumeration) and Step 3 inbound-link rewrite test passes (VCS-present test)
- [ ] Full suite green: `mise run test` — deferred to a consolidated run

> **Note (Phase 3 §3).** No 0004 snapshot exists (`test-migrate-snapshot.sh`
> auto-skips 0002–0006; only `baseline-no-pending` is live), so snapshot
> regeneration is a no-op. `build_scan_corpus` carries no VCS inspection itself
> (the VCS checks lived entirely in the deleted `_preflight_scan_corpus_clean`);
> the VCS-present test exercises the retained corpus-walk + Step 3 rewrite path.

#### Manual Verification:

- [ ] A clean-start `/accelerator:migrate` over a corpus needing 0001+0003+0004
      completes in **one** invocation with no mid-batch dirty-tree complaint

---

## Phase 4: Port bash-4 associative arrays to bash 3.2

### Overview

Replace `declare -A` in `0006` (two sites) and `config-dump.sh` (one site) with
parallel indexed arrays + linear search, preserving exact behaviour, warning
strings, and iteration order.

### Changes Required

#### 1. Behaviour-pinning tests (safety net)

The existing tests already pin the observable behaviour and must stay green:
0006 `paths-alias-research` (`test-migrate.sh:1973-1982`: one rewrite,
`"aliases paths."`, `"skipping duplicate walk"`), 0006 `template-alias`
(`test-migrate.sh:1987-1999`: one rewrite, `"resolve to the same file"` on
**stderr**), and `config-dump.sh` output assertions
(`scripts/test-config.sh:2533-2550`, the `review.max_inline_comments` default/source
row). The durable bash-3.2 guard arrives in Phase 5 (bashisms lint).

This is a refactor-under-green, but the parallel-array port is exactly the kind
of change prone to a silent reordering or index-misalignment regression that the
*current* assertions would not catch (they pin "one rewrite" but not *which key*
is recorded as the alias owner, and they pin only one of eight `config-dump`
review rows). So the following assertions are **mandatory**, not conditional:

- **0006 owner key**: assert the alias `log_warn` names the correct
  previously-recorded owner key (e.g. `aliases paths.plans`), not just that the
  `"aliases paths."` substring appears — pinning that `_walked_owner`/the
  template lookup return the right recorded entry, not merely *an* entry.
- **`config-dump.sh` array alignment**: assert the emitted default for the
  **first and last** `REVIEW_KEYS` entries (not only `max_inline_comments`), so
  a one-position slip between `REVIEW_DEFAULTS` and `REVIEW_KEYS` is caught at
  both ends of the array rather than silently shipping wrong defaults for the
  other seven keys.

#### 2. Port `0006` `WALKED` and `TEMPLATE_PATHS`

**File**: `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh`
**Changes**: replace `declare -A WALKED` (`315`) with parallel
`WALKED_CANONS[]` / `WALKED_KEYS[]` + a linear-search owner lookup; replace
`declare -A TEMPLATE_PATHS` (`361`) with `TEMPLATE_PATHS[]` / `TEMPLATE_NAMES[]`
+ lookup. Preserve once-only walk/rewrite, the exact `log_warn` strings
(including the previously-recorded key/name), the WALKED-only stdout
`"skipping duplicate walk for paths.$key"`, and iteration order
(`plans research_codebase research_issues`; `plan codebase-research rca`).

```bash
WALKED_CANONS=(); WALKED_KEYS=()
_walked_owner() {  # echoes the recorded key for a canon, empty if unseen
  local needle="$1" i
  for ((i = 0; i < ${#WALKED_CANONS[@]}; i++)); do
    [ "${WALKED_CANONS[$i]}" = "$needle" ] && { printf '%s' "${WALKED_KEYS[$i]}"; return 0; }
  done
}
for key in plans research_codebase research_issues; do
  raw_rel="$(cd "$PROJECT_ROOT" && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" "$key" 2>/dev/null || true)"
  [ -z "$raw_rel" ] && continue
  canon="$(canonicalise_rel "$raw_rel")"
  owner="$(_walked_owner "$canon")"
  if [ -n "$owner" ]; then
    log_warn "0006: paths.$key aliases paths.$owner ($raw_rel -> $canon) — skipping duplicate walk"
    echo "0006: skipping duplicate walk for paths.$key"
    continue
  fi
  WALKED_CANONS+=("$canon"); WALKED_KEYS+=("$key")
  walk_corpus "$key"
done
```

Apply the analogous parallel-array treatment to the `TEMPLATE_PATHS` loop
(`361-373`), preserving the `"resolve to the same file"` stderr warning.

#### 3. Port `config-dump.sh` `DEFAULTS`

**File**: `scripts/config-dump.sh`
**Changes**: replace `declare -A DEFAULTS` (`107-118`) with an ordered
`REVIEW_DEFAULTS=(...)` array parallel to `REVIEW_KEYS` (`121-131`), and rewrite
the review loop (`159-164`) to index-based, mirroring the existing
`AGENT_KEYS`/`AGENT_DEFAULTS` loop (`166-172`). Self-contained — no change to
`config-defaults.sh` (the `test-config.sh:2513-2520` single-definition guard does
not cover these inline arrays).

```bash
REVIEW_DEFAULTS=(
  "10" "4" "8" "3"
  "[architecture, code-quality, test-coverage, correctness]"
  "[]" "critical" "critical" "3"
)
for i in "${!REVIEW_KEYS[@]}"; do
  key="${REVIEW_KEYS[$i]}"; default="${REVIEW_DEFAULTS[$i]}"
  value=$("$READ_VALUE" "$key" "$default")
  source=$(get_source "$key")
  echo "| \`$key\` | \`$value\` | $source |"
done
```

### Success Criteria

#### Automated Verification:

- [x] No `declare -A` / `local -A` / `typeset -A` in scope:
      `! grep -rnE '(declare|local|typeset) -A' skills/config/migrate/migrations/0006-*.sh scripts/config-dump.sh` (clean)
- [x] 0006 alias dedup tests pass, incl. the strengthened owner-key assertion
      (`aliases paths.research_codebase`): `bash test-migrate.sh` → 472 pass
- [x] `config-dump.sh` output tests pass, incl. the new first+last
      `REVIEW_DEFAULTS`↔`REVIEW_KEYS` alignment assertion: `bash test-config.sh` → 550 pass
- [ ] Full suite green: `mise run test` — deferred to a consolidated run

#### Manual Verification:

- [x] Ran `0006` (parse) and `config-dump.sh` (full) under macOS system
      `/bin/bash` 3.2.57 — no `declare: -A: invalid option`; config-dump emits
      the correct default rows (`max_inline_comments`→10, `plan_revise_major_count`→3)

> **Deviation (Phase 4).** The plan's `_walked_owner` snippet (and the analogous
> `_template_owner`) had a latent bug: with `set -euo pipefail`, the trailing
> `[ … ] && { … }` made the function return the failing test's status (1) on a
> no-match iteration, so `owner="$(_walked_owner …)"` aborted 0006 mid-run. Fixed
> by using `if … fi` and an explicit terminal `return 0` in both helpers.

---

## Phase 5: Shell format + lint tooling wired to CI

### Overview

Add two task families — **`format`** (shfmt) and **`lint`** (ShellCheck + the
bashisms grep-lint) — enforcing shell hygiene and the bash-3.2 floor repo-wide,
in a dedicated CI job. Each family splits into a **`check`** variant (read-only,
fails on violations) and, where a tool can auto-fix, a **`fix`** variant, and
carries a **`scripts`** component slot so `format`/`lint` for other
languages/components can slot in later (`format:frontend:*`, `lint:python:*`, …)
without rewiring callers. Depends on Phase 4 (the bashisms gate fails on
un-ported `declare -A`).

**Where `fix` applies:**

- **shfmt** has a meaningful auto-fix → both `format:scripts:check`
  (`shfmt -d`) and `format:scripts:fix` (`shfmt -l -w`).
- **ShellCheck and bashisms have no safe auto-fix** — ShellCheck findings are
  resolved manually (§7) and the bashisms denylist has nothing to rewrite — so
  the `lint` family is **check-only**. No `lint:scripts:fix` is added; a no-op
  alias for symmetry is intentionally omitted. (If a future linter gains a real
  fixer, it slots in as `lint:<component>:<tool>:fix`.)

**Check/fix maps to CI vs local:**

- **CI** runs the **check** variants (read-only; fail on drift/violation) via a
  single `mise run check`.
- The **local `default`** mise task runs the **fix** variants where they exist
  (auto-format) plus the lint checks.
- Running the CI checks locally stays possible — `mise run check` (or any
  individual `:check` task) works unchanged on a dev machine.

### Task taxonomy

```
format:scripts:check            shfmt -d <scripts>          read-only, fails on drift
format:scripts:fix              shfmt -l -w <scripts>       formats in place, lists changed
format:check                    → depends [format:scripts:check]    component umbrella
format:fix                      → depends [format:scripts:fix]

lint:scripts:shellcheck:check   shellcheck -x --severity=warning <scripts>
lint:scripts:bashisms:check     scripts/lint-bashisms.sh <scripts>
lint:scripts:check              → depends [lint:scripts:shellcheck:check,
                                            lint:scripts:bashisms:check]
lint:check                      → depends [lint:scripts:check]       component umbrella

check                           → depends [format:check, lint:check]  ← what CI runs
```

The component umbrellas (`format:check`/`format:fix`/`lint:check`) currently
delegate to the sole `scripts` component; they exist so a future
`format:frontend:check` is added by extending one `depends` list rather than by
rewiring CI or the default task. `check` is the single read-only entry point CI
invokes and a developer can mirror locally.

### Changes Required

#### 1. Pin shfmt; keep shellcheck pinned

**File**: `mise.toml`
**Changes**: add shfmt under `[tools]` pinned to an **exact** version (resolve
the current `mvdan/sh` release at implementation time; do not leave a floating
pin). The pinned version **must be the same one used to generate the repo-wide
`mise run format:scripts:fix` reformat commit** (§7) — shfmt output is
version-sensitive, so a mismatch makes the `format:scripts:check` drift check
fail in CI against a correctly-formatted tree. Prefer mise's registry short name
(`shfmt = "<exact>"`) over an explicit `aqua:mvdan/sh` reference if the registry
resolves it, to avoid coupling the format task to the aqua backend being
installed/enabled on every host.

#### 2. Drive shfmt via `.editorconfig` (single source of truth)

**File**: `.editorconfig`
**Changes**: add a `[*.sh]` block matching the existing 2-space style, including
the shfmt-specific `switch_case_indent` key (the `.editorconfig` equivalent of
`-ci`).

```ini
[*.sh]
indent_style = space
indent_size = 2
switch_case_indent = true
```

> **shfmt reads `.editorconfig` only when no *formatting* flags are passed.**
> Per the shfmt man page, *"If any parser or printer flags are given to the
> tool, no EditorConfig formatting options will be used"* — it is all-or-nothing.
> So the format task (§4) must invoke shfmt with **no formatting flags** (`-i`,
> `-ci`, `-bn`, `-sr`, …); only operation flags (`-d`, `-l`, `-w`) are allowed.
> This makes `.editorconfig` the single source of truth shared by editors and
> CI, avoiding the local-vs-CI drift that passing `-i 2 -ci` on the CLI would
> reintroduce (the CLI flags would silently override the file). Add
> `switch_case_indent = true` here rather than `-ci` on the command line.

#### 3. The bashisms grep-lint (committed script)

**File**: `scripts/lint-bashisms.sh` (new)
**Changes**: a self-contained denylist scanner over tracked `.sh` files,
excluding fixtures/stubs/workspaces and `test-helpers.sh`. Denylist:
`declare -A` / `local -A` / `typeset -A`, `mapfile`, `readarray`,
`${x^^}`/`${x^}`/`${x,,}`/`${x,}`, `&>>`, `|&`, negative array subscripts.
Exit non-zero with file:line on any hit. Exclusion globs: `**/test-fixtures/**`,
`workspaces/**`, `**/test-helpers.sh`.

To keep this gate trustworthy long-term (it is the *only* durable bash-3.2
guard), specify its matching precisely rather than as a naive `grep`:

- **Strip comments before scanning** — drop everything from an unquoted `#` to
  end-of-line — so a comment that names a forbidden construct (e.g. a note
  explaining *why* `declare -A` is avoided) does not trip the lint.
- **Inline opt-out**: honour a trailing `# lint-bashisms: ignore` marker on a
  line so a deliberate, justified exception is expressible without contorting
  the source. (Heredoc bodies remain a known minor false-positive surface;
  document it rather than over-engineer a shell tokeniser.)
- **Document the denylist is known-incomplete**: a header comment must state it
  catches the *enumerated* bash-4 constructs only — it cannot prove bash-3.2
  compatibility (a future bash-4-only feature outside the list, e.g. `${x@Q}`,
  would pass). The manual bash-3.2 replay (Testing Strategy) remains the
  behavioural backstop; this lint is the regression gate for the *known* set.

Cover the comment-stripping and opt-out behaviours in `tests/tasks/test_lint.py`.

#### 4. The `format` and `lint` task families + shared discovery

**Files**: `tasks/shared/sources.py` (new shared helper package), `tasks/format/`
(new package: `__init__.py`, `scripts.py`), `tasks/lint/` (new package:
`__init__.py`, `scripts.py`).

- **`tasks/shared/sources.py`** — a `shell_sources()` helper returning tracked
  `.sh` files with the exclusion set applied (`**/test-fixtures/**`,
  `workspaces/**`, `**/test-helpers.sh`). Shared by both families so format and
  lint scan an **identical** file set. It lives under `tasks/shared/` (a helpers
  package containing no invoke tasks — not registered as a collection) to keep
  helper modules separate from task modules. (Plain name, no leading underscore.)
- **`tasks/format/scripts.py`** — `check` (`shfmt -d` over `shell_sources()`)
  and `fix` (`shfmt -l -w`), both **flag-free re: formatting** so `.editorconfig`
  (§2) governs style.
- **`tasks/lint/scripts.py`** — `shellcheck` (`-x --severity=warning`) and
  `bashisms` (invoke `scripts/lint-bashisms.sh`), both check-only.

Each task mirrors the accumulate-then-`raise Exit(code=1)` pattern from
`tasks/test/unit.py:42-49` (the `failures = []` / `if failures: raise Exit(...,
code=1)` block — *not* `tasks/build.py:34-50`, which holds Mach-O/ELF magic-byte
constants).

#### 5. Register the families + define mise tasks

**File**: `tasks/__init__.py`
**Changes**: add `format` and `lint` to the import tuple (`:3-14`) and register
each as a collection with a `scripts` sub-collection, mirroring how `test` adds
`integration`/`unit`. Note `format` shadows a Python builtin — alias the import
(e.g. `from tasks import format as format_`) to avoid confusion.

```python
ns_format = Collection("format")
ns_format.add_collection(Collection.from_module(format_.scripts))  # format.scripts.check / .fix
ns.add_collection(ns_format)

ns_lint = Collection("lint")
ns_lint.add_collection(Collection.from_module(lint.scripts))       # lint.scripts.shellcheck / .bashisms
ns.add_collection(ns_lint)
```

**File**: `mise.toml`
**Changes**: define the public taxonomy. Leaf tasks delegate to invoke;
aggregates compose via `depends`, so the mise `:`-name hierarchy is independent
of invoke's collection nesting (the per-tool mise names carry a `:check` suffix
for forward-consistency with `format`'s check/fix pair, while the invoke leaf is
named for the tool since lint has no `fix`).

```toml
[tasks."format:scripts:check"]
run = "invoke format.scripts.check"   # shfmt -d (no formatting flags)
[tasks."format:scripts:fix"]
run = "invoke format.scripts.fix"     # shfmt -l -w
[tasks."format:check"]
depends = ["format:scripts:check"]
[tasks."format:fix"]
depends = ["format:scripts:fix"]

[tasks."lint:scripts:shellcheck:check"]
run = "invoke lint.scripts.shellcheck"
[tasks."lint:scripts:bashisms:check"]
run = "invoke lint.scripts.bashisms"
[tasks."lint:scripts:check"]
depends = ["lint:scripts:shellcheck:check", "lint:scripts:bashisms:check"]
[tasks."lint:check"]
depends = ["lint:scripts:check"]

[tasks.check]
depends = ["format:check", "lint:check"]   # read-only; what CI runs
```

**File**: `mise.toml` — wire the local default
**Changes**: append `format:fix` and `lint:check` to the existing `default`
task's `depends` (currently `build:frontend`, `build:server:dev`, `test`), so a
local `mise run default` auto-formats and lint-checks. CI instead runs the
read-only `mise run check`. The default's `format:fix` mutates the working tree
by design (local convenience); CI never mutates.

#### 6. Pytest coverage

**Files**: `tests/tasks/test_format.py` (new), `tests/tasks/test_lint.py` (new),
and a discovery test for `tasks/shared/sources.py` (in either file or a
`tests/tasks/shared/test_sources.py`).
**Changes**: `MagicMock(spec=Context)` per `tests/tasks/test_github.py:27-31`;
assert each task issues the expected command — `format:scripts:check` runs
`shfmt -d` with **no formatting flags**, `format:scripts:fix` runs `shfmt -l -w`,
and the shellcheck/bashisms checks issue their expected commands. Asserting an
exclusion *glob string* is absent from the constructed command is a weak proxy —
it does not prove fixtures are excluded. So **additionally** add a
discovery-level test that runs the shared `tasks/shared/sources.py shell_sources()`
against a tmp tree containing a fixture (`**/test-fixtures/`), a `workspaces/`
entry, a `test-helpers.sh`, and a normal script, asserting the resolved list
**excludes** the first three and **includes** the normal one — testing the
exclusion behaviour (load-bearing per the research) rather than the command
string. Also cover the bashisms comment-stripping and `# lint-bashisms: ignore`
opt-out (§3). Append the new test files to the `test:unit:tasks` pytest list
(`mise.toml:88`) — that list is explicit, not glob-discovered.

#### 7. One-time cleanup (each an isolated commit)

- **shfmt reformat:** run `mise run format:scripts:fix` repo-wide (the shared
  discovery already excludes fixtures/workspaces) as its **own dedicated
  commit**, separate from the tooling-wiring commit, so the mechanical diff
  reviews and reverts cleanly.
- **ShellCheck backlog:** **fix all** surfaced warnings now (no blanket
  severity-floor escape); resolve genuine issues and add narrowly-scoped
  `# shellcheck disable=<code>` only where a finding is a deliberate false
  positive, each with a one-line reason.
- The bash-3.2 port (0006 + `config-dump.sh`) is already done in Phase 4, so the
  bashisms gate passes on arrival.

#### 8. CI: dedicated parallel checks job

**File**: `.github/workflows/main.yml`
**Changes**: add a `checks` job (parallel to `test`, `runs-on: ubuntu-latest`,
mise install) running the read-only aggregate `mise run check` (= `format:check`
+ `lint:check`). It must run the **check** variants, never `format:fix`, so CI
never mutates the tree — drift fails the build.

```yaml
  checks:
    name: Run shell format & lint checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: jdx/mise-action@v4
        with: { install: true, cache: true, experimental: true }
      - run: mise run check
```

### Success Criteria

#### Automated Verification:

- [ ] `mise run lint:scripts:bashisms:check` passes (zero `declare -A` in scope)
- [ ] `mise run lint:scripts:shellcheck:check` passes at `--severity=warning` with `-x`
- [ ] `mise run format:scripts:check` reports no drift; `format:scripts:fix` is a
      no-op on an already-formatted tree (idempotent)
- [ ] Aggregates pass: `mise run lint:check`, `mise run format:check`, `mise run check`
- [ ] Format/lint task unit tests pass: `mise run test:unit:tasks`
- [ ] `mise run test` unaffected by the reformat

#### Manual Verification:

- [ ] CI shows a separate `checks` job, green and parallel to `test`, running `mise run check`
- [ ] The shfmt reformat is an isolated commit; the diff is purely whitespace/format
- [ ] `mise run default` auto-formats locally via `format:fix`; re-running leaves the tree clean
- [ ] Introducing a `declare -A` anywhere in scope fails `mise run check` (and `mise run lint:scripts:bashisms:check`) locally

---

## Testing Strategy

### Unit Tests

- `merge_move` (dedicated harness `scripts/test-merge-move.sh`, Phase 2 §1b):
  dest-absent move; file-onto-file overwrite (source wins); nested dir merge
  with same-named subdir; leaf collision in a merged dir; **type mismatch both
  directions** (file→dir, dir→file); missing source no-op; empty-source dir;
  partial/interrupted convergence; filenames with spaces and dotfiles.
- `tasks/format/` + `tasks/lint/`: each task issues the expected command
  (shfmt flag-free; shellcheck/bashisms check-only); `tasks/shared/sources.py`
  `shell_sources()` applies the exclusion set.
- `config-dump.sh`: `REVIEW_DEFAULTS` parity with `REVIEW_KEYS` ordering/values.

### Integration Tests

- Migrate suites (`test-migrate.sh`, `-interactive`, `-snapshot`) via
  `mise run test:integration:migrate`, plus `test:integration:config` for
  `config-dump.sh`.
- End-to-end upgrade replay through the orchestrator: clean repo needing
  0001+0003+0004 completes in one invocation; re-run after a forced partial
  converges.

### Manual Testing Steps

1. On a stock-macOS bash 3.2 host, replay the consumer scenario: stale
   `meta/tmp/` + fresh `.accelerator/tmp/`, then `/accelerator:migrate` — expect
   a single clean pass, no `declare -A` error, no mid-batch dirty abort.
2. Inspect the VCS diff for any leaf collision and confirm source-wins.
3. Add a stray `declare -A` and confirm `mise run check` fails.

## Performance Considerations

`merge_move` recursion is bounded by relocation tree depth (a handful of small
dirs) — negligible. Repo-wide lint adds a short parallel CI job. No runtime hot
paths affected.

## Migration Notes

- The merge contract changes observable behaviour on a genuine leaf collision
  (previously: abort; now: source overwrites target). VCS revert is the safety
  net; the pre-run banner already states migrations rewrite files.
- If `test-migrate-snapshot.sh` proves platform-sensitive on Linux, it stays
  manual-only (exec bit not added) and the exclusion is recorded here in
  Phase 1.
- Deleting `ACCELERATOR_MIGRATE_FORCE_NO_VCS` is a silent no-op for anyone who
  set it (unknown env vars are ignored); no user-facing docs reference it.
- **Accepted tradeoff (no-VCS users).** Review flagged that removing 0004's
  no-VCS hard-fail — the only no-VCS guard, since the orchestrator's clean-tree
  gate passes silently without a VCS — while making relocations destructively
  overwrite leaves a non-VCS user with no recovery net. This is **deliberately
  accepted**: we assume users are on VCS (consistent with the research's
  resolved decision and the project-wide "VCS revert is the recovery path"
  stance). `merge_move` still carries a cheap non-empty-`$dst` / `rm -rf --`
  bug-guard so a code defect can't escalate to a project-root wipe, but no
  no-VCS-specific protection is added.
- **bash-3.2 floor is guarded by the bashisms lint (known construct set) plus a
  manual 3.2 replay, not an automated 3.2 CI run.** Running the suites under a
  real bash 3.2 in CI (research Option B) was not adopted, so a future bash-4
  construct outside the denylist could still reach a 3.2 consumer; the manual
  replay (Testing Strategy step 1) is the behavioural backstop. Recorded as a
  known residual risk.

## References

- Research: `meta/research/codebase/2026-06-04-migration-upgrade-failures.md`
- ADR-0023 (migration framework; single clean-tree gate, per-script idempotency):
  `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- ADR-0016 (bash-3.2 floor): `meta/decisions/ADR-0016-userspace-configuration-model.md`
- Merge-move helper home: `scripts/fs-common.sh` (new; relocation-focused,
  deliberately separate from the atomicity-contracted `scripts/atomic-common.sh`)
- bash-3.2 idiom: `scripts/interactive-harness.sh:42-67`, `scripts/config-summary.sh:49-64`
- Task wiring: `tasks/__init__.py:3-39`, `tasks/test/integration.py`,
  `tasks/test/helpers.py:13-34`, `mise.toml`
- CI: `.github/workflows/main.yml:14-31`
