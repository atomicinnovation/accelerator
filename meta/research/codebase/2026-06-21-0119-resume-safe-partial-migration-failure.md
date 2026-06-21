---
type: codebase-research
id: "2026-06-21-0119-resume-safe-partial-migration-failure"
title: "Research: Resume-Safe Partial Migration Failure (work item 0119)"
date: "2026-06-21T00:17:27+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0119"
parent: "work-item:0119"
relates_to: ["work-item:0115", "work-item:0116", "work-item:0118", "work-item:0069", "work-item:0120"]
topic: "Resume-Safe Partial Migration Failure (work item 0119)"
tags: [research, codebase, migrate, run-migrations, interactive-migration, manifest, tooling]
revision: "8f2814b2f569f1f6174d736e525d9d27deb77317"
repository: "build-system"
last_updated: "2026-06-21T00:17:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Resume-Safe Partial Migration Failure (work item 0119)

**Date**: 2026-06-21T00:17:27+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 8f2814b2f569f1f6174d736e525d9d27deb77317
**Branch**: (jj working copy)
**Repository**: build-system

## Research Question

What does building work item 0119 ("Resume-Safe Partial Migration Failure")
require? Specifically: how does the migration runner's apply loop and clean-tree
pre-flight work today, where would a new per-run path manifest integrate, how is
ownership/staleness best modelled given the codebase's patterns and bash-3.2
floor, how should the tests be structured, and what shared edit surfaces must be
coordinated with sibling work items (0115/0116/0118/0069/0120)?

## Summary

0119 adds two coherent halves to `run-migrations.sh`:

1. **Write a per-run path manifest** — a plain-text file (one repo-relative path
   per line) recording every path each migration mutates, appended **as paths
   are mutated** so a migration that fails mid-way still has its partial writes
   recorded. It must carry run identity so a stale manifest from a prior run is
   detectable.
2. **A manifest-driven guarded-resume branch in the clean-tree pre-flight** —
   when *every* dirty path is owned by this run's manifest, proceed into the
   apply loop without `ACCELERATOR_MIGRATE_FORCE=1`, printing a resume-affordance
   message listing the owned dirty paths. Any non-owned path, or a
   missing/empty/unreadable/stale manifest, preserves the existing refusal
   (fail-closed: owned set treated as empty).

Key confirmations from the live code:

- **The work item's premise holds.** There is *no* per-run manifest and *no*
  record of which paths any migration writes today. The only durable records are
  id-per-line ledgers (`migrations-applied`, `migrations-skipped`) and the
  interactive session log (decision values, not paths).
- **The state area is fixed, not per-run** — `$PROJECT_ROOT/.accelerator/state/`,
  derived purely from `PROJECT_ROOT`. There is no run id or start-timestamp
  identifying a run today; 0119 must introduce one.
- **Mid-loop failure does `exit 1` and leaves prior mutations on the tree** — by
  design; VCS revert is the rollback path, which is exactly why the dirty-tree
  pre-flight then blocks the re-run.
- **The line numbers in the work item have drifted** (the `--decisions-file`
  work shifted the file). The apply loop is now **`run-migrations.sh:281-324`**
  (ledger append at **:321**), and the clean-tree pre-flight is
  **`run-migrations.sh:94-168`** (FORCE guard at **:95**, generic refusal at
  **:162-166**, session-log detection at **:116-161**).
- **0116 is already done/merged**; 0119 builds against the `--decisions-file`
  switch and the `emit_no_input_stall` helper it added (which already names 0119
  in its stall text). 0116 deliberately did *not* touch the pre-flight resume
  hint, reducing the conflict surface.

The migration system lives under `skills/config/migrate/`, **not** the top-level
`scripts/` (which holds the shared libraries it sources).

## Detailed Findings

### Area 1 — The apply loop and per-migration completion point

`skills/config/migrate/scripts/run-migrations.sh`

- **Pending set**: globbed from `migrations/[0-9][0-9][0-9][0-9]-*.sh`
  (`run-migrations.sh:186-191`) minus ids already in the applied/skipped ledgers
  (`:233-245`). Migrations dir overridable via `ACCELERATOR_MIGRATIONS_DIR`
  (`:186`).
- **Apply loop**: `run-migrations.sh:281-324`. Each migration is a **standalone
  `.sh` run in its own child process**; the runner only orchestrates and records
  the ledger.
  - **Mechanical dispatch** (`:299-323`):
    `PROJECT_ROOT=… CLAUDE_PLUGIN_ROOT=… ACCELERATOR_MIGRATION_MODE=1 bash "$f"`
    — `PROJECT_ROOT` is exported (`:284`), so a migration writes under it.
  - **Interactive dispatch** (`:287-297`): delegated to
    `run_interactive_migration` (`interactive-lib.sh:358`), keyed on a
    `# INTERACTIVE: yes` header (`is_interactive_migration`,
    `interactive-lib.sh:22-34`).
  - **No-op-pending sentinel**: a migration printing
    `MIGRATION_RESULT: no_op_pending` is not recorded (`:307-310`).
- **Ledger append** (`run-migrations.sh:320-321`):
  ```sh
  mkdir -p "$(dirname "$STATE_FILE")"
  atomic_append_unique "$STATE_FILE" "$id"
  ```
  `STATE_FILE` = `$PROJECT_ROOT/.accelerator/state/migrations-applied`
  (`:25`); **plain text, one migration id per line** (not JSONL). The interactive
  path has the twin append at `interactive-lib.sh:648-651`. **This is the
  per-migration completion point** the work item references — the manifest write
  integrates here (and, for partial-failure recording, *inside* each migration's
  mutation, which today is opaque to the runner — see Open Questions).
- **Mid-loop failure**: mechanical at `:300-305` (dump captured output, `[id]
  failed`, `exit 1` at :304); interactive at `:289-292` (`exit 1` at :291). No
  rollback; prior and partial mutations stay on the tree. The interactive stall
  text already says the migration "may have already partially modified the
  working tree" and "resume-safety for partial runs is tracked separately
  (0119)" (`interactive-lib.sh:320-322`).
- **Sourced libraries**: `config-common.sh` (:7), `atomic-common.sh` (:9, which
  transitively sources `jsonl-common.sh`), `interactive-lib.sh` (:277, sourced
  *after* the pre-flight). VCS detection is **inline** in the runner — no
  `vcs-common.sh` dependency.
- **No manifest exists** — confirmed by grep across `skills/config/migrate`
  (`manifest|mutated|written_paths` hits are only test fixtures and SKILL.md
  docs).

### Area 2 — The clean-tree pre-flight and FORCE bypass

`skills/config/migrate/scripts/run-migrations.sh:94-168` — straight-line script
code, **no functions, no early returns**; control flow is nested `if`/`exit`.

- **Whole-block FORCE gate** (`:95`): `if [ -z "${ACCELERATOR_MIGRATE_FORCE:-}" ];
  then` — any non-empty value bypasses the *entire* pre-flight (VCS detection,
  dirty enumeration, both refusal paths) and jumps to "Read state files" (:170).
  Empty string does not bypass.
- **VCS detection** (`:96-101`): jj preferred (`.jj` present), git fallback
  (`.git`).
- **Dirty-path enumeration** (`:103-114`):
  - jj (`:104-108`): `jj --no-pager diff --name-only | grep -E
    '^(meta/|\.claude/accelerator|\.accelerator/)'` — includes untracked.
  - git (`:109-113`): `git status --porcelain <pathspecs> | grep -v '^??'` —
    excludes untracked. (Note the jj-vs-git untracked asymmetry.)
- **Dirty test** (`:116`): `if [ -n "$dirty" ]; then`.
- **Generic refusal** (`:162-166`, exit 1) — verbatim:
  ```
  Error: dirty working tree — uncommitted changes detected in meta/, .claude/accelerator*.md, or .accelerator/.
  Commit or discard those changes first, or set ACCELERATOR_MIGRATE_FORCE=1 to skip this check.
  ```
  The `ACCELERATOR_MIGRATE_FORCE=1` hint text appears **only here** (:165). This
  is the string AC3/AC4 must assert is *still present* on a refusal.
- **Session-log detection** (`:116-161`, nested before the generic refusal):
  extracts paths matching
  `.accelerator/state/migrations-<id>-session.jsonl` from the dirty set
  (`:120-123`), and if any are found, prints a resume/discard/status scaffold
  (header :125, resume hint :138-139, discard hints :148, status hint :158-159)
  then `exit 1` at **:160**. It does **not** resume — it steers the user to the
  structured resume/discard rather than a blind `jj abandon`.
- **Where the guarded-resume branch inserts**: the work item frames it inside the
  session-log branch, but the manifest-driven resume is **broader** — it must
  fire whenever every dirty path is owned (mechanical *or* interactive). The
  cleanest structure: after computing `$dirty` (:114) and confirming `-n
  "$dirty"` (:116), compute the owned set from the manifest; if fully owned,
  print the resume-affordance and **fall through past the pre-flight** (e.g. set
  a flag and skip the two `exit 1`s, or wrap the refusal block in an `else`).
  Because both refusals are unconditional `exit 1` with no shared epilogue, the
  proceed-instead-of-refuse logic must intercept *before* reaching them.

### Area 3 — Patterns to model the manifest on

- **Atomic append** — `scripts/atomic-common.sh:38-61` (`atomic_append_unique`):
  idempotent via `grep -Fxq -- "$line" "$target"`, then reread+rewrite via
  `atomic_write` (`:16-32`, same-dir temp + `mv`). O(n) per call — fine for a
  modest manifest. For high-volume non-unique appends, a plain `printf '%s\n'
  "$rel" >> "$manifest"` (idiom at `0003-relocate-accelerator-state.sh:243`) is
  lighter. A lock-serialised JSONL appender exists (`atomic_jsonl_append`,
  `atomic-common.sh:177-210`) but is overkill for a single-process run.
- **Run identity / timestamp** — `interactive-lib.sh:174-175`:
  `timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)` is the established run-identity
  format in the migration layer. Recommended: write the run id (or this
  timestamp) as a **header line** when the manifest is first created, and compare
  on every later open.
- **Fail-closed-on-stale precedent** — the visualiser launcher records an
  identity and refuses on mismatch: `launcher-helpers.sh:157-167`
  (`stop_server_stop` — strict `!=`, named reason, cleanup). Mirror this:
  recorded-vs-current run id mismatch → treat owned set as empty → refuse.
- **State-area path idiom** — `run-migrations.sh:11-26` (`STATE_FILE`/`SKIP_FILE`)
  and the per-migration family `migrations-${id}-*` at `interactive-lib.sh:362-369`
  (`-resume-state.tmp`, `-stderr.log`, `-session.jsonl`, `-decisions.txt`). A
  manifest naturally becomes `…/migrations-${id}-paths.txt` (per-migration) or a
  single per-run `…/migrations-run-paths.txt` — planning decision.
- **Read state file back** — `run-migrations.sh:170-176` (`while IFS= read -r
  line … <"$STATE_FILE"`).
- **Relative↔absolute path handling** — resolve recorded relative paths against
  `$PROJECT_ROOT` (`interactive-lib.sh:493-496`, `run-migrations.sh:130/:143`);
  strip a prefix to produce a repo-relative entry via parameter expansion
  (`0003-relocate-accelerator-state.sh:120`: `rel="${path#"$PROJECT_ROOT/"}"`).
- **Fail-closed-on-missing/unreadable triad** — `run-migrations.sh:76-92`
  (decisions-file validation: is-dir / not-exist / not-readable, each `exit 1`).
- **bash-3.2-safe set membership** — Idiom A: file as the set + `grep -Fxq`
  (`atomic-common.sh:46`, `accelerator-scaffold.sh:37`). Idiom B: load into a
  parallel array + linear-scan dedup loop with the `"${arr[@]+"${arr[@]}"}"`
  unbound-safe expansion (`0003-relocate-accelerator-state.sh:196-218`,
  `run-migrations.sh:236-242`). **No associative arrays** (bash 3.2 floor).

### Area 4 — Tests

Suite to extend: `skills/config/migrate/scripts/test-migrate.sh` (≈2169 lines,
flat sequential `Test:` blocks, ~180 `assert_*` calls), sourcing
`scripts/test-helpers.sh`. Run directly with `bash …/test-migrate.sh` or via
`mise run test:integration:migrate`.

- **Harness**: `mktemp -d` temp repos; stub corpora passed via
  `ACCELERATOR_MIGRATIONS_DIR` (`:25-45`); `setup_old_repo` builds a **VCS-less**
  repo so the clean-tree check is skipped (`:63-75`); real git init for
  dirty-tree tests (`:357-362`), or `mkdir .git` to merely make the runner detect
  git (`:140`).
- **Existing dirty-tree + FORCE case — Test 14** (`:350-376`): asserts non-zero
  exit on dirty tree, `assert_contains "$OUTPUT" "dirty"`, no ledger entry on
  abort, then FORCE → exit 0 + applied. This is the direct model for AC2/AC3.
- **Existing failing stub — Test 4** (`:138-154`): heredoc stub that `exit 1`s
  **without mutating paths**. For AC1 (manifest after partial failure) write a
  new stub that **writes a known set of paths under `$PROJECT_ROOT` then `exit
  1`**, following this heredoc-to-`$FAIL_DIR` idiom.
- **Assertions** (`scripts/test-helpers.sh`): exit codes via `RC=0; OUT=$(…) ||
  RC=$?`; stderr via `assert_stderr_contains`/`assert_stderr_not_contains`
  (`:257`/`:273`) or `2>&1`+`assert_contains`; "exactly these lines" via per-line
  `grep -cFx '<path>' == 1` plus a `wc -l` total (model:
  `:1004-1012`, `:791-796`).
- **Count floor**: `tasks/test/integration.py:8` `_EXPECTED_MIGRATE_SUITES = 4`
  is an **at-least floor on suite files** (the four `test-migrate*.sh`), *not* a
  per-test-case count. Adding cases to `test-migrate.sh` requires no bump; only a
  brand-new suite file would.
- **0119's four ACs → four tests, all in `test-migrate.sh`**: (a) manifest
  correctness after partial failure; (b) guarded resume on fully-owned dirty
  tree (exit 0, resume message on stderr); (c) refusal on mixed/non-owned tree
  (non-zero, no resume message, FORCE hint present); (d) fail-closed on
  missing/empty/unreadable/stale manifest (same observable refusal as c).
- Also extend `scripts/test-atomic-common.sh:47-67` if the manifest reuses/extends
  `atomic_append_unique` (a prior plan review flagged clobbering that net).

### Area 5 — Related work items, shared edit surfaces, sequencing

**Source research**: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
— Fix E = "detect that the dirty paths are this run's own prior-migration output
and offer a guarded resume, rather than forcing `ACCELERATOR_MIGRATE_FORCE=1`."
Verdict: **valuable hygiene but treats a symptom; secondary to C/D** (which
remove the root cause of partial-run failures). The research's open question
"is detecting 'this run's own output' reliable enough, or adopt a real
staging/transaction boundary?" is resolved by 0119 in favour of path-ownership
detection (transaction boundary explicitly out of scope).

**Parent epic 0115** (`meta/work/0115-…md`): five children (0116 B, 0117 A, 0118
C, 0119 E, 0120 tests). 0115 **adopted A (decisions bridge) and rejected D (the
0007-split)** — so the "ordered after D" half of 0119's sequencing is
**unenforceable** (D has no work item). E can be built independently; doing so
ahead of C/D risks guarding a symptom C/D would remove.

**Shared edit surfaces / coordination**:

| Task | Region | Concern |
|------|--------|---------|
| **0119** | `run-migrations.sh` apply loop `:281-324` (next to ledger append `:321`) + per-migration mutation | Manifest append |
| **0119** | `run-migrations.sh` pre-flight `:94-168` (FORCE gate `:95`) | Guarded-resume branch |
| **0116** (done) | `interactive-lib.sh` stall logic; **its own** resume line | Did *not* touch pre-flight hint — low conflict |
| **0118** (ready) | `0007` backfill + `validate-corpus-frontmatter.sh` | May be the *failing* migration at 0119's completion point — coordinate |
| **0117** | `run-migrations.sh` flag block `if`/`case` → `while`/`shift` | Adjacent, not direct conflict |

- **0116 is already done/merged** (recent commits: `--decisions-file` switch,
  structured stall, plan validated/done). Build against the `--decisions-file`
  switch and `emit_no_input_stall`. The stall text **already names 0119**
  ("resume-safety for partial runs is tracked separately (0119)",
  `interactive-lib.sh:320-322`) — that breadcrumb is a candidate to update once a
  real guarded resume ships. 0116 recorded that "a future change to the
  pre-flight resume-hint format (including via 0119) must be treated as touching
  this task" — so if 0119 alters the pre-flight affordance, reconcile 0116's
  stall text.

**0069** (done — `meta/work/0069-…md`): implemented the interactive resumability
(ADR-0037) — a **per-transformation, interactive-only** session-log replay. It is
**not** a rollback and does **not** track mechanically-mutated paths. 0119 extends
it by adding an **orthogonal** resume axis: safely resuming a *partial mechanical
run* over its own dirty output. This is precisely why a new path-manifest
artefact is needed (the session log captures decisions, not paths).

**0120** (`meta/work/0120-…md`): prevention tests for the no-input stall (0116)
and the backfill/validator sentinel (0118) **only**. It explicitly states 0119
owns guarded-resume coverage and its own tests — hence a relates-to link, not a
blocker.

## Code References

- `skills/config/migrate/scripts/run-migrations.sh:25-26` — `STATE_FILE`/`SKIP_FILE` (ledger paths)
- `skills/config/migrate/scripts/run-migrations.sh:94-168` — clean-tree pre-flight (FORCE gate :95, dirty enum :103-114, session-log branch :116-161, generic refusal :162-166)
- `skills/config/migrate/scripts/run-migrations.sh:170-176` — read ledger back, line by line
- `skills/config/migrate/scripts/run-migrations.sh:281-324` — apply loop (mechanical dispatch :299-323, failure `exit 1` :304, ledger append :320-321)
- `skills/config/migrate/scripts/interactive-lib.sh:174-175` — `date -u` run-identity timestamp format
- `skills/config/migrate/scripts/interactive-lib.sh:320-322` — stall text naming 0119
- `skills/config/migrate/scripts/interactive-lib.sh:362-369` — `migrations-${id}-*` state-file family
- `skills/config/migrate/scripts/interactive-lib.sh:648-651` — interactive twin ledger append
- `scripts/atomic-common.sh:16-32` — `atomic_write`
- `scripts/atomic-common.sh:38-61` — `atomic_append_unique`
- `scripts/atomic-common.sh:177-210` — `atomic_jsonl_append` (lock-serialised; likely overkill)
- `scripts/launcher-helpers.sh:157-167` — fail-closed-on-identity-mismatch precedent
- `0003-relocate-accelerator-state.sh:120,196-218,243` — prefix-strip to relative, bash-3.2 dedup loop, `>>` append
- `skills/config/migrate/scripts/test-migrate.sh:138-154` — failing-stub Test 4 (model for AC1 stub)
- `skills/config/migrate/scripts/test-migrate.sh:350-376` — dirty-tree + FORCE Test 14 (model for AC2/AC3)
- `skills/config/migrate/scripts/test-migrate.sh:1004-1012,791-796` — exact-line `grep -cFx` assertions
- `scripts/test-helpers.sh:257,273` — `assert_stderr_contains`/`_not_contains`
- `scripts/test-atomic-common.sh:47-67` — existing `atomic_append_unique` tests
- `tasks/test/integration.py:8,138-147` — `_EXPECTED_MIGRATE_SUITES = 4` floor

## Architecture Insights

- **Filesystem-as-memory + VCS-as-rollback.** The migration framework
  deliberately has no transaction boundary; mutations land immediately and VCS
  revert is the safety net. The dirty-tree pre-flight is the *single* guard
  enforcing that contract, and `ACCELERATOR_MIGRATE_FORCE=1` is its coarse, all-
  or-nothing escape. 0119's value is making the escape *targeted* — relax the
  guard only for paths this run provably produced — without introducing the
  transaction boundary the design rejects.
- **State is keyed by migration id, never by run.** Every existing state file
  (`migrations-applied`, `-skipped`, `-${id}-session.jsonl`) is id-keyed and
  survives across runs. 0119 introduces the first **run identity** in this layer;
  getting staleness detection right (fail-closed) is the subtle part, because a
  manifest left by a *prior* run over the *same* dirty paths must not be honoured.
- **Two orthogonal resume axes.** 0069 resumes *within* an interactive migration
  (replay undecided prompts). 0119 resumes *across* a partial mechanical batch
  (proceed over self-owned dirty output). They share the `.accelerator/state/`
  area and the append-only line-delimited convention but solve different
  problems.
- **The per-mutation recording timing is the genuinely hard part.** The ledger
  append at `:321` is trivial to mirror, but it records *whole-migration
  completion*. AC1 demands the **partial writes of a migration that fails
  mid-way** be recorded — and the runner today is blind to what a child `bash
  "$f"` process writes (it runs in a separate process and mutates the tree
  directly). Recording mutations "as they happen" therefore requires either the
  migration cooperating (emitting paths back to the runner, e.g. via a frame or a
  shared file it appends to) or the runner diffing the tree around each
  migration. This is the central design decision for /create-plan (see Open
  Questions) and is not answered by any existing mechanism.

## Historical Context

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md` — source research; Fix E (Hypothesis 3) characterised as an amplifier/symptom, secondary to C/D.
- `meta/work/0115-make-interactive-migrations-satisfiable-under-agent-invocation.md` — parent epic; adopted A, rejected D.
- `meta/work/0116-…md` + `meta/plans/2026-06-20-0116-structured-stall-on-no-decision-input.md` — done; the model for phased, TDD, line-anchored `run-migrations.sh` edits; reusable test harness (`setup_sandbox`, `0002-predicate` fixture, `</dev/null` no-input forcing).
- `meta/work/0118-…md` — fix C (backfill sentinel); shares the 0007/completion surface.
- `meta/work/0069-migration-framework-interactive-validation-hooks.md` — done; the resumability 0119 extends.
- `meta/work/0120-prevention-tests-for-agent-invocation-path.md` — prevention tests; explicitly defers guarded-resume coverage to 0119.

## Related Research

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md` (the originating issue research for the whole 0115 epic)

## Open Questions

- **Per-mutation recording mechanism (the key /create-plan decision).** AC1
  requires the *partial* writes of a mid-way-failing migration to be in the
  manifest. The runner cannot see a child migration's writes today. Options:
  (a) migrations append their mutated paths to the manifest themselves (a small
  shared helper sourced by migrations / exposed via env path) — clean ownership,
  but touches every migration and the migration-author contract; (b) the runner
  snapshots the tree (e.g. VCS diff `--name-only`) before/after each migration
  and records the delta — no migration changes, but couples manifest correctness
  to VCS and to the dirty-path enumeration that already exists at `:103-114`;
  (c) a hybrid. The work item's phrasing ("append each path at the moment it is
  mutated") leans toward (a); the "integrates cleanly at the per-migration
  completion point next to the ledger append" assumption leans toward a
  completion-time (b)-style diff. These are in tension and must be reconciled.
- **Manifest granularity & filename.** Per-run single file
  (`migrations-run-paths.txt`) vs per-migration (`migrations-${id}-paths.txt`).
  Per-run is simpler for the pre-flight's "every dirty path owned?" check; the
  work item co-locates "with the per-migration ledger" but leaves the exact
  filename to planning.
- **Run-identity token.** A fresh run id (how generated under the bash-3.2 /
  no-`Math.random` constraints?) vs the `date -u +…Z` start timestamp. Timestamp
  is the established format but two runs in the same second could collide;
  consider timestamp + pid.
- **Dedup / ordering of manifest entries** and whether to assert exact ordering
  in AC1's test (deferred from 0119 review pass 3).
- **Exact stderr marker token** for the resume-affordance message (deferred from
  0119 review) — needs to be greppable and ASCII-only, distinct from the existing
  session-log hint.
- **jj-vs-git untracked asymmetry** (`:106-108` vs `:110-113`): the jj branch
  includes untracked paths, git excludes them. A migration that *creates* new
  files (untracked) will appear dirty under jj but not git — the guarded-resume
  ownership check must behave consistently across both VCSes, or the asymmetry
  will make resume behave differently per VCS.
