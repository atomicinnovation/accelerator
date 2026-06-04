---
type: codebase-research
id: "2026-06-04-migration-upgrade-failures"
title: "Research: Migration framework failures on consumer upgrade (0003 tmp conflict, dirty-tree re-check, 0006 bash 3.2)"
date: "2026-06-04T13:51:06+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: ""
topic: "Resolving migration failures encountered by a consumer upgrading the Accelerator plugin"
tags: [research, codebase, migrate, run-migrations, bash-compatibility, clean-tree-preflight]
revision: "4730e9347dcaee854d9561c2c0f8e53465b7ad12"
repository: "accelerator"
last_updated: "2026-06-04T13:51:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Migration framework failures on consumer upgrade

**Date**: 2026-06-04T13:51:06+00:00
**Author**: Toby Clemson
**Git Commit**: 4730e9347dcaee854d9561c2c0f8e53465b7ad12
**Branch**: HEAD (detached)
**Repository**: accelerator

## Research Question

A consumer of the plugin (system bash 3.2) upgraded to `1.21.0-pre.56`
and ran `/accelerator:migrate`. They hit a cascade of failures:

1. **0003** failed because both `meta/tmp` and `.accelerator/tmp` existed (move
   conflict).
2. On re-run, the orchestrator's **dirty-tree pre-flight** blocked them — the
   tree was dirty *because the first migration had already applied*. They were
   forced to set `ACCELERATOR_MIGRATE_FORCE=1`.
3. **0004** then complained about uncommitted changes in `meta/work/` — a
   *second, per-migration* dirty-tree check internal to 0004.
4. **0006** failed with `declare: -A: invalid option` — it uses bash-4
   associative arrays; the consumer's system bash is 3.2.
5. **0002** kept reappearing as a perpetual no-op that "stays pending".

The user's explicit directive: *the clean-working-copy check should run once on
invocation of `run-migrations.sh`, not re-checked per migration.* The
orchestrator's single per-invocation gate is retained; **no state is tracked
across `run-migrations.sh` calls**. If a run fails mid-batch, the user
commits-or-cleans before re-running — idempotency exists for manual state-file
replay and safe merge-moves, not to bypass the gate on a dirty re-run.

## Summary

All five symptoms are real and independently fixable. Grounding each against
the framework's own design rationale (ADR-0023) and the test suite:

| # | Symptom | Root cause | Aligns with design intent? |
|---|---------|-----------|----------------------------|
| 1 | 0003 tmp conflict (and all relocations) | Generic `_move_if_pending` fatally aborts when both source and dest exist, instead of merging | **Violates the idempotency contract** (ADR-0023:104-106) — a re-run after a partial move should converge, not fail. Fix: directory moves must merge source into an existing target across *all* migrations |
| 2 | 0004 internal dirty-tree check | 0004 has its own `_preflight_scan_corpus_clean` (`0004:171-207`) that re-evaluates dirtiness mid-batch — it trips because sibling migrations (0001, 0003) earlier in the *same* run legitimately dirtied the tree | **The actual dirty-tree fix.** ADR-0023 makes the orchestrator the *single* clean-tree gate; per-migration re-checks are not sanctioned. **Remove it.** No cross-call state involved. |
| 3 | Orchestrator pre-flight on re-run | Pre-flight (`run-migrations.sh:67-141`) runs once per invocation; after a partial apply a *re-run* is blocked until the tree is clean | **Retained as-is — not a defect.** Single per-invocation gate matches ADR-0023. On mid-batch failure the user commits-or-cleans before re-running. No state tracked across calls. |
| 4 | 0006 `declare -A` | bash-4 associative arrays (`0006:315,361`) on a repo whose `bash` is 3.2 | **Clear bug** — ADR-0016 mandates a bash-3.2 floor for macOS; rest of codebase honours it |
| 5 | 0002 perpetual no-op | `no_op_pending` sentinel keeps 0002 pending forever when `work.id_pattern` lacks `{project}` (stable config) | **Intended-as-tested** (test pins "stays pending"). **Decision: leave as-is** — accepted friction, no code change |

**Systemic finding that explains why #4 shipped broken**: the three migrate
test suites are **not wired into any mise/invoke task or CI job**, and there is
**no bash-3.2 enforcement anywhere** in the repo. CI runs on ubuntu (bash 4+),
so `declare -A` passes silently. Any fix should add regression protection.

## Detailed Findings

### 1 — Relocation moves abort instead of merging (tmp is the trigger)

`0003-relocate-accelerator-state.sh` relocates legacy paths into `.accelerator/`
via a generic helper `_move_if_pending` (lines 32-64). Its conflict branch
(lines 44-56) is **fatal** when both source and destination exist:

```
accelerator migrate: conflict — both 'meta/tmp' and '.accelerator/tmp' exist.
```

The tmp move is just one of seven calls (`0003:197-205`):

- `_move_if_pending meta/tmp .accelerator/tmp` (line 205, gated on
  `paths.tmp` being unset).

**Why both existed**: the consumer had run `/accelerator:describe-pr` earlier in
the session, which wrote a `pr-body-*.md` into the *new* default tmp location
(`.accelerator/tmp/`), while the *legacy* `meta/tmp/` still held stale PR-review
scratch files. tmp is the directory most likely to fill independently, but it is
not unique — any move target can be re-created between runs.

**The real defect is the fatal-on-conflict policy itself, not tmp.** ADR-0023's
per-script contract requires migrations to be *idempotent* — "self-detect no-op
conditions and exit 0" (ADR-0023:104-106). `_move_if_pending` is idempotent for
three of its four states (both-absent, source-absent/dest-present,
source-present/dest-absent) but **fails the both-present state by aborting**. A
relocation that was interrupted partway — or whose target was legitimately
re-created by another skill — should *converge* on a re-run, not deadlock the
whole batch and force manual `rm`.

**Requirement (per the user): directory moves must merge, not fail.** When the
target directory already exists, every migration should merge the source
directory's contents *into* the existing target, then remove the now-empty
source — across all migrations, not as a tmp special-case. This makes the move
genuinely idempotent and re-run-safe.

Implementation notes for the plan:

- A plain `mv src dst` when `dst` already exists **nests** (`dst/src`) rather
  than merging. The merge helper must walk source entries and move each into
  `dst`, creating `dst` if absent and recursing where both sides hold a
  same-named subdirectory.
- **Leaf-file collision rule: the source overwrites the target.** When the *same
  relative file* exists in both source and target, the source file wins
  unconditionally (no byte-compare, no keep-both). The user sees the overwrite
  in their VCS diff and reverts or manually merges if it matters — VCS is the
  safety net, so the helper stays simple (effectively per-file `mv -f`, then
  remove the now-empty source). This collision cannot arise from a clean
  partial-apply anyway (a per-file move is atomic, so a moved file is gone from
  source); it only occurs when the target was independently re-created.
- This generalises beyond 0003. **0004 has the same anti-idempotent policy** in
  a different shape: `_check_collisions` (`0004:259-274`) aborts the whole
  migration if *any* planned destination file already exists, and `_move_if_pending`
  (`0004:276-283`) is file-level. Under the merge requirement, both 0003 and
  0004 should share a single merge-move utility (a natural home is
  `scripts/atomic-common.sh`, which already hosts the atomic-write helpers).

- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64` — `_move_if_pending` (four-state helper; both-present aborts at 44-56)
- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:197-206` — the seven move calls, incl. tmp at 205
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:259-283` — 0004's collision-abort + file-level move
- `scripts/atomic-common.sh` — suggested home for a shared merge-move helper

### 2 — Migration 0004's internal dirty-tree pre-flight (the dirty-tree fix)

0004 carries its own gate, `_preflight_scan_corpus_clean`
(`0004:171-207`, called at line 207). It does two distinct things:

1. **No-VCS hard-fail** (`0004:175-187`): `log_die "0004: no VCS detected"`
   unless `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1`.
2. **Scan-corpus dirty check** (`0004:189-205`): `jj diff --name-only` /
   `git status --porcelain` over configured paths; `exit 1` with
   `"0004: scan corpus has uncommitted changes — commit or stash first"` if dirty.

Item 2 is the per-migration re-check the user's directive targets. Crucially it
fails **even within a single clean-start invocation**: the orchestrator applies
0001 → 0003 → … → 0004 in one run, so by the time 0004 executes, its sibling
migrations have *legitimately* dirtied the tree — and 0004's own check aborts on
exactly the changes the migration batch was supposed to make. This is the
"uncommitted changes in meta/work/" complaint from the report.

**Design grounding**: ADR-0023 designates the orchestrator as the single
clean-tree gate; per-migration dirty re-checks are unsanctioned
(documents-analyser: concern 3 — "the orchestrator is meant to be the single
gate"). A migration must self-detect no-op state and otherwise just do its work
(ADR-0023:104-106); re-policing the tree mid-batch is outside that contract.

**Fix direction**: **remove the entire `_preflight_scan_corpus_clean` block from
0004** — both item 2 (the scan-corpus dirty check) and item 1 (the no-VCS
hard-fail). The dirty check is the per-migration re-check the user's directive
targets; the no-VCS guard is dropped because **we assume users are on VCS** (the
recovery model is VCS revert regardless, so a separate hard-fail adds friction
without protection). This is the entirety of the dirty-tree fix — it needs no
orchestrator change and **no state tracked across `run-migrations.sh` calls**.
The `ACCELERATOR_MIGRATE_FORCE_NO_VCS` env var and its plumbing go away with it.

**Test impact**: **no test exercises the dirty branch** (`"0004: scan corpus has
uncommitted changes"`, line 202) — every 0004 fixture is VCS-less and uses
`ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` (`run_0004`, `test-migrate.sh:1004-1010`),
so removing item 2 breaks nothing. Removing item 1 **does** require test
changes: the no-VCS refusal test (`test-migrate.sh:1103-1108`, asserting
`"no VCS detected"`) must be deleted, and the pervasive `run_0004` helper that
sets `ACCELERATOR_MIGRATE_FORCE_NO_VCS=1` should be simplified (the env var no
longer exists).

- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:154-207`

### 3 — Orchestrator pre-flight is retained as-is (not a defect)

The pre-flight lives in `run-migrations.sh:67-141`. It detects VCS, greps
`meta/`, `.claude/accelerator*.md`, `.accelerator/` for uncommitted changes, and
aborts with `exit 1` unless `ACCELERATOR_MIGRATE_FORCE=1` is set
(`run-migrations.sh:68`). Section 8 (`run-migrations.sh:252-297`) then applies
**all** pending migrations in one invocation. This is exactly the single
per-invocation gate ADR-0023 prescribes (step 1, before the apply loop —
ADR-0023:112-121) and the user wants kept.

**Why the report's "dirty re-run blocked" was *not* an orchestrator bug.** In
the transcript, 0003 aborted (the merge defect, finding 1), so the user re-ran
on a tree already dirtied by 0001 and the pre-flight correctly refused. Under
the agreed model that refusal is **intended**: on a mid-batch failure the user
commits-or-cleans the partial progress, *then* re-runs against a clean tree. The
orchestrator stays stateless — it tracks nothing across invocations.

Once findings 1 (merge), 2 (remove 0004's check), and 4 (bash 3.2) land, the
batch completes in a **single** invocation and the dirty-re-run path is not
exercised at all. The remaining re-run triggers are legitimate stops — e.g. 0002
hard-erroring on a missing `work.default_project_code` (`0002:29-36`) — where
committing the partial state before re-running is the correct, documented
workflow. `ACCELERATOR_MIGRATE_FORCE=1` remains the advanced/CI/no-VCS escape
hatch (`SKILL.md:31,81`), unchanged.

**Idempotency's role here.** Idempotency is *not* a mechanism for surviving
dirty re-runs without committing. Its two real purposes are: (a) letting a user
**manually edit the state files** (`migrations-applied` / `migrations-skipped`)
to re-apply or replay a migration safely, and (b) making the **merge-moves** of
finding 1 converge on re-application. Both coexist cleanly with a strict
per-invocation clean-tree gate.

**Test impact**: none — no orchestrator behaviour changes. The existing
dirty-abort + FORCE-bypass tests (`test-migrate.sh:298-324`, `390-404`) and the
in-flight-session-log tests (`test-migrate-interactive.sh:121-210`) continue to
pin the current, retained behaviour.

- `skills/config/migrate/scripts/run-migrations.sh:67-141` — the retained pre-flight
- `skills/config/migrate/scripts/run-migrations.sh:252-297` — single-invocation apply loop
- `skills/config/migrate/SKILL.md:31` — documents the once-up-front check

### 4 — Migration 0006 uses bash-4 associative arrays

`0006:315` (`declare -A WALKED`) and `0006:361` (`declare -A TEMPLATE_PATHS`)
require bash 4+. The orchestrator dispatches each migration via PATH `bash`
(`run-migrations.sh:273`), so on the consumer's machine 0006 runs under
`/bin/bash` 3.2 and dies at `declare -A`.

**Design grounding**: ADR-0016:38-40,58,167-169 mandates *"must run on macOS
bash 3.2 out of the box"* and explicitly calls out that bash 3.2 *"lacks
associative arrays."* The rest of the codebase honours this with inline
linear-search dedup over indexed arrays — e.g.
`scripts/config-summary.sh:49-50`, `scripts/interactive-harness.sh:20,34`,
`scripts/config-read-agents.sh:52`, and 0003's own `_merge_state_file`
seen/unique loop (`0003:231-241`). `scripts/config-dump.sh:107` is a stray
counter-example that also uses `declare -A` — a latent bash-3.2 bug now **in
scope**: the forbidden-construct lint (finding 6) will flag it, so it must be
ported to indexed arrays alongside 0006.

**What the two arrays do** (must be preserved when porting):

- `WALKED` (`0006:315-327`): keyed by canonical path → first config key that
  claimed it. Iterated in order `plans research_codebase research_issues`. When
  a later key canonicalises to an already-walked path it emits
  `log_warn "0006: paths.$key aliases paths.${WALKED[$canon]}…"` (line 321) +
  stdout `"0006: skipping duplicate walk for paths.$key"` (line 322) and skips
  the second walk.
- `TEMPLATE_PATHS` (`0006:361-373`): keyed by resolved absolute template path →
  first template name. Iterated `plan codebase-research rca`. Duplicate paths
  emit `log_warn "0006: templates.$name and templates.${TEMPLATE_PATHS[$path]}
  resolve to the same file…"` (line 366) and skip the second rewrite.

**Fix direction**: replace each associative array with two parallel indexed
arrays (`WALKED_PATHS[]`/`WALKED_KEYS[]`, `TEMPLATE_PATHS[]`/`TEMPLATE_NAMES[]`)
plus a linear-search helper returning the recorded owner. Preserve: once-only
rewrite, the exact warning strings (including the previously-recorded
key/name), the WALKED-only stdout skip line, and iteration order.

**Test impact**: the dedup behaviour **is tested** and must keep passing:

- `test-migrate.sh:1973-1982` (`paths-alias-research`) — asserts one rewrite,
  `"aliases paths."`, `"skipping duplicate walk"`.
- `test-migrate.sh:1987-1999` (`template-alias`) — asserts one rewrite and
  `"resolve to the same file"` on **stderr**.

- `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh:315-327,361-373`

### 5 — Migration 0002 perpetual no-op

When `work.id_pattern` lacks `{project}`, 0002 emits `MIGRATION_RESULT:
no_op_pending` (`0002:24-27`). The orchestrator (`run-migrations.sh:280-291`)
deliberately keeps no-op-pending migrations **pending** — so 0002 reappears on
every run and the user must `--skip` it.

Note the *inconsistency* within 0002: the "no legacy NNNN files" path exits 0
**without** the sentinel (`0002:66-68`) → marked applied; only the "pattern
lacks `{project}`" path stays pending. The latter depends on **stable config**,
so it stays pending **forever** for users who will never adopt a `{project}`
pattern.

**Design grounding**: the `no_op_pending` sentinel is **undocumented in any
ADR** (documents-analyser: concern 2). ADR-0023's documented model is the
opposite — a no-op migration *exits 0 and becomes applied* (belt-and-suspenders
idempotency, ADR-0023:106,163-165). The perpetual-pending behaviour is therefore
an extension beyond design intent.

But it **is pinned by a test**: `test-migrate.sh:522-535` asserts 0002 "stays
pending" when the pattern lacks `{project}`. The rationale is to let 0002 run
*later* if the user adds `{project}` to their pattern.

**Decision: leave as-is.** The deferral semantics are intentional and pinned by
the test; the friction (re-appearing as pending until `--skip`) is accepted. No
code change for 0002 in this work.

- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:24-27,66-68`
- `skills/config/migrate/scripts/run-migrations.sh:279-291`

### 6 — Shell-lint tooling (the chosen enforcement for #4)

Decided approach: a **grep-based forbidden-construct lint** (option A) plus
**ShellCheck** and **shfmt**, all wired into the build to run **locally and in
CI**, scoped to the **entire codebase** (161 `.sh` files: 108 under `skills/`,
46 under `scripts/`, 7 under `hooks/`).

**Why a custom grep lint is necessary (empirical, pinned shellcheck 0.11.0):**

- ShellCheck in **default bash mode does *not* flag** `declare -A`, `${var^^}`,
  `${var,,}`, or `mapfile` — it treats them as valid bash (only unrelated SC2034
  "unused" noise). So ShellCheck **cannot** enforce the bash-3.2 floor.
- ShellCheck in **POSIX `sh` mode** flags them (SC3044/SC3059) but *also* flags
  every legitimate bash-3.2 construct the migrations rely on (`[[ ]]`, `local`,
  `declare -a`, `BASH_REMATCH`) — too noisy to gate on.
- `bash -n` (syntax-only) under 3.2 **does not catch** `declare -A` — it is a
  runtime builtin-option error, not a parse error.
- Only a custom denylist gives precise, total static coverage. Suggested set:
  `declare -A` / `local -A` / `typeset -A`, `mapfile`, `readarray`,
  `${x^^}`/`${x^}`/`${x,,}`/`${x,}`, `&>>`, `|&`, negative array subscripts.

**Build wiring** (current structure):

- `mise.toml` `[tools]` already pins `shellcheck = "0.11.0"`. **`shfmt` is
  mise-installable** (`aqua:mvdan/sh`) — add a pinned version. The grep lint is
  a committed shell script (no tool to pin).
- mise tasks delegate to Python `invoke` tasks under `tasks/`. Add a `lint`
  family — e.g. `lint:shellcheck`, `lint:shfmt`, `lint:bashisms`, aggregated
  under `lint` — backed by a new `tasks/lint.py` (mirrors `tasks/test/`).
- Hook `lint` into the build: make it a dependency of `default` (currently
  `build:frontend`, `build:server:dev`, `test`) and add a `mise run lint` step
  to CI. CI's `test` job runs `mise run test` on `ubuntu-latest`
  (`.github/workflows/main.yml:15-35`); add lint as a step there or as a
  separate parallel `lint` job (fail-fast, cleaner signal).

**Tool settings to decide in the plan:**

- *ShellCheck*: run with `-x` (follow `source`d files — the codebase already
  carries `# shellcheck source=` directives). Choose a severity gate
  (`--severity=warning` is a reasonable floor; the codebase is already partly
  shellcheck-aware via inline `# shellcheck disable=` directives).
- *shfmt*: pick flags matching the existing 2-space-indent style (e.g. `-i 2
  -ci`); CI uses `-d` (diff, non-zero on drift), local fixing uses `-w`. An
  `.editorconfig` can drive both.
- *Fixtures*: deliberately-malformed / stub scripts under `**/test-fixtures/`
  (e.g. migrate fixtures, the `9001-stub` no-op) must be **excluded** from all
  three tools, or they will produce spurious failures.

**One-time cleanup cost (flag for the plan):**

- Running shfmt repo-wide produces a **large but mechanical** first-pass
  reformat diff across all 161 scripts.
- Running ShellCheck repo-wide will surface a **pre-existing-warning backlog**;
  decide fix-now vs. curated `# shellcheck disable=` vs. severity floor.
- The bashisms lint immediately fails on the **latent `declare -A` in
  `scripts/config-dump.sh:107`** — that script must be ported to bash 3.2 as
  part of this work (same indexed-array treatment as 0006, finding 4).

- `mise.toml` — `[tools]` (shellcheck pinned; add shfmt) and `[tasks.*]`
- `.github/workflows/main.yml:15-35` — the `test` job to extend with lint
- `tasks/test/` — invoke-task layout to mirror for `tasks/lint.py`
- `scripts/config-dump.sh:107` — latent `declare -A` the bashisms lint will catch

## Code References

- `skills/config/migrate/scripts/run-migrations.sh:67-141` — orchestrator clean-tree pre-flight (per-invocation)
- `skills/config/migrate/scripts/run-migrations.sh:252-297` — single-invocation apply loop + no_op_pending handling
- `skills/config/migrate/scripts/run-migrations.sh:273` — dispatches each migration via PATH `bash`
- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64` — `_move_if_pending`; both-present aborts (44-56) instead of merging
- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:197-206` — the seven move calls, incl. tmp at 205
- `skills/config/migrate/migrations/0004-...subject-subcategories.sh:259-283` — 0004's collision-abort + file-level move (same anti-idempotent policy)
- `skills/config/migrate/migrations/0004-...subject-subcategories.sh:171-207` — internal `_preflight_scan_corpus_clean`
- `scripts/atomic-common.sh` — suggested home for a shared merge-move helper
- `skills/config/migrate/migrations/0006-...id-and-author.sh:315-327,361-373` — `declare -A` dedup blocks
- `skills/config/migrate/migrations/0002-...project-prefix.sh:24-27,66-68` — no_op_pending vs plain exit-0
- `scripts/accelerator-scaffold.sh:19-40` — inner/root gitignore scaffold (tmp not ignored)
- `scripts/config-summary.sh:49-50`, `scripts/interactive-harness.sh:20,34` — bash-3.2 dedup idiom to follow

## Architecture Insights

- **Moves must be merges to be idempotent.** ADR-0023:104-106 mandates
  per-migration idempotency, but the relocation helpers fail it by aborting when
  a target already exists. A move that *merges* source into an existing target
  (then removes the source) converges on every re-run regardless of how the
  target came to exist — the correct primitive for a framework whose recovery
  model is "re-run after fixing the cause". A single shared merge-move helper
  should replace both 0003's `_move_if_pending` fatal branch and 0004's
  `_check_collisions` abort.
- **Single-gate contract.** ADR-0023 makes the orchestrator the sole owner of
  the clean-tree gate, checked once per invocation before the apply loop. The
  orchestrator's gate is *retained unchanged*; the fix is removing 0004's
  per-migration re-check (and arguably its no-VCS check), which re-policed the
  tree mid-batch in violation of that contract. No state is tracked across
  invocations.
- **Stateless re-run model.** When a migration stops mid-batch, recovery is the
  user's responsibility: commit-or-clean the partial progress, then re-run
  against a clean tree. The framework deliberately keeps no cross-call session
  state — idempotency exists so a user can manually edit the state files to
  replay a migration, and so merge-moves converge, *not* to let dirty re-runs
  bypass the gate.
- **Bash-3.2 floor is convention, not enforcement.** ADR-0016 sets the floor but
  nothing in CI guards it — CI's `test` job runs on `ubuntu-latest` (bash 5.x),
  and `config-dump.sh:107` already smuggles in a `declare -A`. **No off-the-shelf
  linter targets the "bash 3.2 subset"** (empirically confirmed against the
  pinned shellcheck 0.11.0 — see "Shell-lint tooling" below). The chosen durable
  fix is a **grep-based forbidden-construct lint** (option A) plus general
  shell-lint hygiene (**ShellCheck** + **shfmt**), all wired into the build and
  run repo-wide. See "Shell-lint tooling" for the wiring detail.
- **no_op_pending is an undocumented sentinel.** It is load-bearing in the code
  and tests but absent from the ADR corpus — a documentation gap worth closing
  if the behaviour is retained.

## Historical Context

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — framework
  design; clean-tree pre-flight as single pre-loop gate (75-80, 84-85, 112-121),
  ACCELERATOR_MIGRATE_FORCE as advanced bypass (113-115, 174-176), per-script
  responsibility limited to idempotency (104-106).
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — the bash-3.2
  floor (38-40, 58, 167-169); associative arrays explicitly out of bounds.
- `meta/decisions/ADR-0037-...interactive-contract...` — leaves the clean-tree
  pre-flight unchanged (120).

## Related Research

None located specific to migration upgrade failures; this is the first
write-up. See the ticket-management and ADR initiatives in `MEMORY.md` for
adjacent context on the meta-directory evolution that motivated these
migrations.

## Open Questions

All design questions are settled (recorded under Resolved Decisions below). What
remains are implementation-detail choices for the plan, not open design
questions: ShellCheck severity floor, shfmt flags / `.editorconfig`, the exact
forbidden-construct denylist, fixture-exclusion globs, and whether lint runs as
a CI step or a separate job. The one-time cleanup cost (repo-wide shfmt reformat,
ShellCheck backlog, porting `config-dump.sh`) is called out in finding 6.

## Resolved Decisions

- **Clean-tree gate**: keep the orchestrator's single per-invocation check
  as-is; remove 0004's per-migration re-check; track **no** state across
  `run-migrations.sh` calls. On mid-batch failure the user commits-or-cleans
  before re-running.
- **No-VCS guard**: **assume users are on VCS.** Delete 0004's no-VCS hard-fail
  and the `ACCELERATOR_MIGRATE_FORCE_NO_VCS` env var entirely.
- **Directory moves merge instead of aborting**, across all migrations, via a
  shared merge-move helper.
- **Leaf-file collision rule**: the **source overwrites the target**
  unconditionally; VCS is the safety net (user reverts/merges from the diff).
- **0002 perpetual no-op**: **leave as-is.** The `no_op_pending` deferral is
  intended behaviour (pinned by `test-migrate.sh:522-535`); the friction is
  accepted.
- **Bash-3.2 enforcement**: ship a **grep-based forbidden-construct lint** (A)
  *and* general shell-lint hygiene via **ShellCheck** (C) and **shfmt**, all
  wired into the build to run **locally and in CI**, scoped to the **entire
  codebase**. Option B (running suites under real bash 3.2) is not adopted. See
  finding 6 for wiring and one-time cleanup cost.
