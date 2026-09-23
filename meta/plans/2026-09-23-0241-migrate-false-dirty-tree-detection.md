---
type: "plan"
id: "2026-09-23-0241-migrate-false-dirty-tree-detection"
title: "False Dirty-Tree Detection on jj Repositories Implementation Plan"
date: "2026-09-23T16:53:40+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0241"
parent: "work-item:0241"
derived_from: ["codebase-research:2026-09-23-0241-migrate-false-dirty-tree-detection"]
relates_to: ["work-item:0263", "work-item:0119", "work-item:0198", "work-item:0286"]
tags: ["migration", "vcs", "jj", "preflight"]
revision: "9556c9ac4137e3476e17d836d8b9a4ca8ae0ce52"
repository: "accelerator"
last_updated: "2026-09-23T22:51:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# False Dirty-Tree Detection on jj Repositories Implementation Plan

## Overview

On jj, `accelerator migrate` records `@`'s commit id as the run base. That id
changes on every snapshot, so the guarded resume never engages (#97). This
plan:

- moves the jj run base to `@`'s sorted parent commit ids, read from the same
  repository load as the dirty paths it guards;
- makes the jj dirty-path computation agree with `jj status` on the git
  excludes and on `snapshot.max-new-file-size`, including conditional config
  scopes;
- makes the refusal list the paths that blocked it, and say when they belong
  to a stale run.

It also removes the name `ForeignDirt`, which closes 0263. The domain now
speaks of **owned** and **unowned changes**, and of the **run base**.

## Terminology

The work item's "owned dirt" / "foreign dirt" become **owned changes** /
**unowned changes** in code, tests, docs and user-facing text. The value
guarding a run is the **run base** everywhere; "revision" and "run id" no
longer name it. Everything below uses the new terms.

| Old                                  | New                                              |
|--------------------------------------|--------------------------------------------------|
| `PreflightError::ForeignDirt`        | `PreflightError::UnownedChanges(UnownedChanges)` |
| `Ownership::Foreign`                 | `Ownership::Unowned`                             |
| "foreign dirt"                       | "unowned changes"                                |
| `MigrationContext::revision`         | removed; the run base comes from `WorkingCopy`   |
| `Preflight.revision`                 | removed                                          |
| `DirtyPathScanner` (port)            | `WorkingCopy`                                    |
| `VcsDirtyPathScanner`                | `VcsWorkingCopy`                                 |
| `Preflight.scanner`                  | `Preflight.working_copy`                         |
| `ManifestStore::run_id`              | `ManifestStore::recorded_run_base`               |
| `ManifestStore::write_run_id`        | `ManifestStore::record_run_base`                 |
| `RunnerPaths::run_id`                | `RunnerPaths::recorded_run_base`                 |
| `classify(.., base_revision_matches)` | `classify(.., run_base_matches)`                |

The on-disk file `.accelerator/state/migrations-run.id` keeps its name, so
existing repositories need no file migration.

"foreign" survives only where it means something else:

- `m0007/prepass.rs`: a `work_item_id` from another project;
- the visualiser: a cross-origin request.

## Current State Analysis

- **Run base**:
  - `run_default` passes `ctx.revision()` (`cli/migrate-cli/src/main.rs:238`)
    into `Preflight`, before the run lock is taken.
  - `FileMigrationContext::revision`
    (`cli/migrate-adapters/src/context.rs:88-91`) reads `vcs::facts(..).revision`.
    On jj that is `jj_revision` (`cli/vcs-adapters/src/library.rs:659-693`),
    i.e. `@`'s commit id.
  - `MigrationContext::revision` has no other production caller. Test doubles
    implement it in `cli/migrate/tests/{list_and_decisions_file,lifecycle,engine}.rs`
    and `cli/migrate/src/migrations/{m0003,m0008,m0009}.rs`.
  - Corpus stamping reads `vcs::facts` separately
    (`cli/corpus-adapters/src/metadata.rs`) and is untouched.
- **Pre-flight**:
  - `Preflight::run` (`cli/migrate/src/preflight.rs:66-124`) returns the unit
    variant `PreflightError::ForeignDirt`. It computes the blocking paths and
    then drops them.
  - Under force it writes an empty manifest and the run base without
    scanning (`preflight.rs:71-75`).
  - The render prints the fixed `DIRTY_TREE_REFUSAL`
    (`cli/migrate-cli/src/render.rs:15`).
  - `VcsDirtyPathScanner` (`cli/migrate-adapters/src/dirty_path_scanner.rs:36-44`)
    turns any scan error into an empty list, which takes the clean branch.
- **Decisions file**:
  - The stall names `.accelerator/state/migrations-<id>-decisions.txt`
    (`render.rs:98-100`). The runner never writes it.
  - `decisions_file::validate` (`cli/migrate/src/decisions_file.rs:80-128`)
    checks the file against every pending prompt before any migration is
    applied, and fails closed on a missing decision.
  - `.accelerator/state/` is not ignored. Neither `ensure_inner_gitignore`
    (`cli/config-adapters/src/store.rs:787`) nor m0003's scaffold ignores it.
  - `-decisions.txt` is not among the `SESSION_SUFFIXES`
    (`cli/migrate/src/manifest.rs:61-62`).
  - ⚠️ So writing the file the stall names creates an unowned change. That
    blocks the documented resume on git as well as jj. The work item's
    reproduction and its git criterion both require the file to be owned.
- **m0007 re-runs**: it skips files that already carry a strict fence and
  writes a file only when its content changes (`m0007/mod.rs:146-181`). A
  forced run therefore never records files an earlier run already migrated.
- **jj dirty paths**:
  - `snapshot::working_copy_diff` (`cli/vcs-adapters/src/library/snapshot.rs:128-193`)
    diffs the snapshot against `@`'s parent tree.
  - It passes `base_ignores: GitIgnoreFile::empty()` and
    `max_new_file_size: u64::MAX`.
  - Its callers are `dirty_paths::jj_dirty_paths` (migrate pre-flight,
    `work sync`) and `status_log::jj_status` (VCS status renderer).
- **Settings lint**: `lint:vcs-settings:check` (`tasks/lint/vcs_settings.py`)
  exempts `snapshot.rs` and `tracked.rs`. Everything this plan adds outside
  `snapshot.rs` is settings-free, so no exception is needed. That answers the
  work item's open question.
- **Harness**: `Hermetic` (`cli/vcs-test-support/src/hermetic.rs`) isolates the
  `git`/`jj` CLIs only. These tests spawn binaries in the ambient environment:
  - `cli/migrate-cli/tests/dirty_tree_preflight.rs`
  - `cli/vcs-cli/tests/status_log_goldens.rs`
  - `cli/vcs-cli/tests/status_log_parity.rs`

  In-process vcs-adapters tests follow the fixture-binary convention
  (`vcs-adapters-fixture`, `tests/user_name.rs:21-42`) whenever the
  environment matters. Fixture binaries are plain `[[bin]]` entries under
  `tests/fixtures/` without `required-features` (`cli/vcs-adapters/Cargo.toml`).
- **Logging**: `accelerator-migrate` never initialises a tracing subscriber,
  so the scanner's `warn!` is dropped. `vcs-cli` gates `kernel::logging::init()`
  on `ACCELERATOR_LOG` (`cli/vcs-cli/src/main.rs:98-102`).
- **Bash-captured goldens**: `cli/migrate-cli/tests/fixtures/manifest-states/`
  `{stale,unreadable,empty,absent}/stderr` pin the refusal text.
  `fixtures/regenerate.sh`'s `gr_base_rev` records jj's `@` change id as the
  run base.

### jj 0.43.0 behaviour, verified by running the pinned binary

- **Default limit**: 1 MiB inclusive. A file of 1,048,576 B is added and one of
  1,048,577 B is refused.
- **Value forms**: `1024`, `"1024"` and `"1KiB"` behave identically. `0` means
  no limit.
- **Invalid values**: `"abc"` and `-1` fail with
  `Invalid type or value for snapshot.max-new-file-size`.
- **Layer order**: workspace > repo > user. Within the user layer, `conf.d` >
  `config.toml`.
- **Config location**: repo and workspace config live at
  `$XDG_CONFIG_HOME/jj/{repos,workspaces}/<id>/config.toml`, keyed by
  `.jj/repo/config-id` and `.jj/workspace-config-id`. jj creates the id files
  lazily, on `jj config set --repo` or `jj config path`, not on
  `jj git init` or `jj status`.
- **Legacy config**: a legacy `.jj/repo/config.toml` is honoured, and jj turns
  it into a symlink.
- **Copied repositories**: jj copies the config to a new id with the same
  value. Reading the pre-copy id therefore yields the same limit.
- **Limit applies to new files only**: once any snapshot has tracked a file,
  it is reported whatever its size. Every size fixture must use files no
  snapshot has seen.
- **Oversized files in `jj status`**: they are listed as `? path` after a
  "Refused to snapshot" warning, not omitted. `jj status` parity therefore
  compares its change lines (`A`/`M`/`D`/`R`/`C`) only.
- **Renames**: `jj status` prints `R meta/{a.md => b.md}`. `dirty_paths`
  reports both paths.
- **Excludes**: on both colocated and non-colocated repos, the backing
  `info/exclude` applies. `core.excludesFile` replaces the XDG default
  `git/ignore`, which applies only when `core.excludesFile` is unset.
- **Backing repo config**: jj-lib opens the backing repo with
  `gix::open::Options::default()` (`jj-lib/src/git_backend.rs:556-571`), so
  its `config_snapshot()` includes the global git config.

Not verified against the binary:

- `core.excludesFile` given as a relative or `~` path;
- a repository without a git backend.

The work item's reading of jj-cli is taken as given for these, and the parity
tests cover the relative and `~` forms.

## Desired End State

- **Guarded resume on jj**: it engages across `jj status`, owned edits and
  `jj describe @`. It reports a stale manifest across a rewrite of any parent
  or a move of `@` onto different parents. Colocated and non-colocated
  repositories behave the same.
- **Run base**:
  - On jj it is `@`'s parent commit ids, sorted and joined by `+`.
  - On git it stays `HEAD`.
  - It is read after the run lock, from the same repository load as the
    dirty paths.
  - Corpus stamps stay `@` / `HEAD`.
- **Decisions file**: the one the stall names is an owned change while the run
  base is unchanged.
- **Refusal**: it prints the existing text, then `Unowned changes (<n>):` and
  each unowned path byte-sorted, indented two spaces, with control characters
  escaped. When a recorded run base no longer matches, lines say that the
  paths may be a previous run's output and how to recover without force. It
  never lists an owned path.
- **jj dirty paths**: they equal the paths on `jj status`'s change lines for
  every exclude, size and conditional-scope fixture, in all three consumers.
- **Unresolvable limit or excludes**: every consumer logs a `warn` naming the
  source (`snapshot.max-new-file-size`, or the excludes file), visible with
  `ACCELERATOR_LOG=warn`, and falls back to today's behaviour, which
  over-reports changes. No new error reaches the fail-open branch.
- **Removed name**: `ForeignDirt` and `Ownership::Foreign` appear nowhere, and
  0263 is `done`.
- **`CHANGELOG.md`**: it carries `### Fixed` entries for the run base
  (requirement 7), the excludes and the size limit.
- **Verification**: `mise run` exits 0.

### Key Discoveries

- **One load for base and diff**: `snapshot::working_copy_diff` loads the
  `load_at_head` view and diffs against `@`'s parent tree. Returning that
  commit's `parent_ids()` from the same call makes the run base agree by
  construction with the changes it guards.
- **Existing jj-lib API covers the ignores**:
  - `jj_lib::git::get_git_repo(store)` (`jj-lib/src/git.rs:424`) returns the
    backing `gix::Repository`, or errors without a git backend;
  - `git_repo_path()` gives `info/exclude`;
  - `GitIgnoreFile::chain_with_file` ignores missing files.
- **Existing jj-lib API covers the limit**: `HumanByteSize: TryFrom<ConfigValue>`
  (`jj-lib/src/settings.rs:346`) with `StackedConfig::get_value_with`
  (`config.rs:775`) resolves it without `UserSettings`.
- **Existing jj-lib API covers conditional scopes**:
  `jj_lib::config_resolver::resolve(&StackedConfig, &ConfigResolutionContext)`
  (`config_resolver.rs:190`) applies `[[--scope]]` tables and `--when`
  conditions, also without `UserSettings`.
- **Reusable path helpers**: `jj_user_name`'s helpers (`library.rs:780-850`)
  already reproduce the system and user layer paths.
- **Log capture**: `cli/vcs-cli/src/report.rs:138-169` shows the in-process
  `warn!` capture pattern. Binary tests use `ACCELERATOR_LOG=warn` plus
  stderr, as in `status_log_goldens.rs:251-286`.

## What We're NOT Doing

- **Fail-open pre-flight**: the scanner's policy of treating a scan error as
  a clean tree stays. This plan adds no new error that reaches it: the limit
  and excludes resolution warn and fall back instead.
- **`lint:vcs-settings:check`**: no change and no new exemption.
- **Reading more settings from user config**: the snapshot's other settings
  (`fsmonitor.backend`, eol, exec-bit, conflict-marker style) keep coming from
  jj-lib defaults. Only `snapshot.max-new-file-size` comes from the user's
  config stack.
- **Git config trust**: `core.excludesFile` is read as jj-cli reads it, as a
  raw string whatever gix's ownership trust says. A repository-local config
  the user does not own can therefore hide changes, exactly as it does from
  `jj status`. This is accepted for parity.
- **`jj_revision`**: unchanged. Corpus metadata keeps stamping `@`.
- **`user.name` layers**: `jj_user_name` keeps reading the system and user
  layers only.
- **`auto-track` and `snapshot.auto-track`**: stay as today (`EverythingMatcher`).
- **Other config forms**: no Windows config paths, no `--config` arguments
  and no `JJ_*` override layers beyond what `jj_user_name` already reads.
- **Migrate skill guidance**: 0286 owns the rewrite of its dirty-path
  guidance. This plan only swaps the "foreign" wording in
  `skills/config/migrate/SKILL.md`.
- **The stall message**: its "base revision" wording is not reworded.
- **Adopting dirt under force**: a forced run still starts with an empty
  manifest. Pre-upgrade stalls recover by committing the partial output.
- **Content-aware ownership**: ownership stays path-based. Moving `@` onto
  another change with the same parents (`jj edit` to a sibling, `jj new @-`,
  `jj abandon @`) keeps the run base, so a manifested path changed there is
  still owned and the resume engages over it. This matches the git
  semantics, where the manifest owns paths rather than contents. A phase 2
  test pins it.

## Implementation Approach

Four phases, each green under `mise run` and mergeable on its own. The order
is:

- phase 1 reshapes the domain error;
- phase 2 fixes #97;
- phases 3 and 4 bring the jj dirty-path computation to `jj status` parity.

Every step is red → green → refactor: the failing test named in the step comes
first.

Parity with `jj status` is checked through an oracle in `vcs-test-support`
(added in phase 2). Tests whose outcome depends on the environment run the code
in a child process with `Hermetic::apply`, never in the test process. From
phase 3 on, the jj dirty-path computation reads `HOME`, `XDG_CONFIG_HOME`,
`JJ_CONFIG` and the global git config, so every jj dirty-path assertion goes
through `vcs-adapters-fixture only dirty_paths`.

---

## Phase 1: Name the Unowned Changes

### Overview

Rename the ownership vocabulary. Then make the refusal carry and print the
unowned paths, on git and jj (requirement 5).

### Changes Required

#### 1. Rename (behaviour-preserving refactor, existing tests stay green)

**Files**:

- `cli/migrate/src/manifest.rs`: `Ownership::Foreign` → `Ownership::Unowned`,
  and doc wording.
- `cli/migrate/src/preflight.rs`: `ForeignDirt` → `UnownedChanges`, and the
  doc on `run`.
- `cli/migrate-cli/src/main.rs:249`: the match arm.
- `cli/migrate/tests/preflight.rs` and `cli/migrate/tests/manifest.rs`: variant
  names; test names become e.g.
  `unowned_changes_beside_the_runners_bookkeeping_still_refuse` and
  `a_path_outside_every_class_is_unowned`.
- `cli/migrate-cli/tests/dirty_tree_preflight.rs`: module doc and test names,
  e.g. `an_unowned_git_change_refuses`.
- `cli/migrate-adapters/tests/dirty_path_scanner.rs`: doc wording.
- `cli/migrate-cli/tests/fixtures/interactive/foreign-dirty-path/` →
  `unowned-dirty-path/`, plus every reference to it:
  - `fixtures/README.md`;
  - `fixtures/regenerate.sh`, including the `foreign.md` fixture file name
    → `unowned.md`;
  - any test that loads the directory.
- `skills/config/migrate/SKILL.md:270`: "foreign changes" → "changes the run
  does not own".
- `cli/migrate/tests/fixtures/public-api.txt`: regenerate with
  `mise run public-api:update`.

#### 2. Carry the unowned paths

**Red**: extend `cli/migrate/tests/preflight.rs`, then
`cli/migrate-cli/tests/dirty_tree_preflight.rs`.

Domain tests in `cli/migrate/tests/preflight.rs`. The scanner double returns
its paths out of order (`meta/a.md` before `.accelerator/b`), so the sort is
under test:

- An owned manifest path plus unowned `meta/a.md` and `.accelerator/b` gives
  `UnownedChanges { paths: [".accelerator/b", "meta/a.md"], stale_run: false }`
  inside `PreflightError::UnownedChanges`,
  byte-sorted and without the owned path.
- With no manifest, the same unowned changes are all listed, and
  `stale_run` is false.
- With a stale run base, every dirty path is listed, manifested paths and
  session artefacts included, and `stale_run` is true.
- With a manifest but no recorded run base, every dirty path is listed, and
  `stale_run` is false.
- With a manifest and a recorded run base of `None` against a current `None`,
  every dirty path is listed, and `stale_run` is false.
- With a recorded run base but no manifest, every dirty path is listed, and
  `stale_run` is false.
- Runner-managed paths are never listed.

Every case asserts the whole `UnownedChanges` value. The existing
`an_absent_manifest_or_run_id_refuses_even_a_session_artefact` and
`a_recorded_revision_of_none_never_matches_even_a_current_none` do too, not
just the variant.

Binary tests in `dirty_tree_preflight.rs`, now spawning the binary with
`env.apply(&mut command)`, for git, jj `--no-colocate` and jj `--colocate`:

- a seeded manifest and run base matching the current `HEAD` (git), plus
  unowned `meta/a.md` and `.accelerator/b`: stderr contains
  `DIRTY_TREE_REFUSAL`'s text, then `Unowned changes (2):`, then
  `  .accelerator/b` before `  meta/a.md`, and not the owned path;
- no manifest: both paths listed, in that order.

The jj manifest-backed case needs the phase 2 run base and lands there
(phase 2 step 5). In phase 1 the jj tests use a manifest-free setup only.

**Green**: `cli/migrate/src/preflight.rs`.

```rust
pub struct UnownedChanges {
    pub paths: Vec<String>,
    pub stale_run: bool,
}

pub enum PreflightError {
    UnownedChanges(UnownedChanges),
    Failed(MigrationError),
}

impl Preflight<'_> {
    pub fn run(
        &self,
    ) -> Result<(RunLockGuard, PreflightOutcome), PreflightError> {
        // ... lock, force, scan and clean branch unchanged
        let unowned = self.unowned_changes(&dirty)?;
        if !unowned.paths.is_empty() {
            return Err(PreflightError::UnownedChanges(unowned));
        }
        // ... affordance unchanged
    }

    fn unowned_changes(
        &self,
        dirty: &[String],
    ) -> Result<UnownedChanges, PreflightError> {
        let manifest = self.manifest.manifest()?;
        let recorded = self.manifest.run_id()?;
        let current_run = recorded.is_some() && recorded == self.revision;
        let stale_run =
            manifest.is_some() && recorded.is_some() && !current_run;
        let mut paths: Vec<String> = match manifest {
            Some(manifest) if current_run => dirty
                .iter()
                .filter(|path| {
                    classify(path, &self.runner, &manifest, current_run)
                        == Ownership::Unowned
                })
                .cloned()
                .collect(),
            _ => dirty.to_vec(),
        };
        paths.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        Ok(UnownedChanges { paths, stale_run })
    }
}
```

`Display` for `UnownedChanges` stays `"dirty working tree"`.

**Red → green**: `cli/migrate-cli/src/render.rs` gains
`pub fn unowned_changes_refusal(unowned: &UnownedChanges)`, with
unit tests in the same module. It prints:

1. `DIRTY_TREE_REFUSAL`;
2. when `stale_run`, lines saying that a previous run's recorded base no
   longer matches the working copy, so its output is listed too, and that
   if the listed paths are only that run's output, committing them and
   re-running without `ACCELERATOR_MIGRATE_FORCE` resumes it;
3. `Unowned changes (<n>):`;
4. each path indented two spaces. Only characters for which `is_control()`
   holds are escaped, with `char::escape_default`; everything else,
   non-ASCII and quotes included, prints as is.

The unit tests cover the stale lines and their recovery text, the count, a
path containing a newline and an ESC byte, and `meta/café.md` and
`meta/it's.md` printing unchanged. `main.rs` calls it in the
`UnownedChanges` arm.

**Golden updates**: update every assertion on the refusal text. Requirement 6
permits this. The goldens are:

- `interactive/unowned-dirty-path` expected stderr;
- `manifest-states/{stale,unreadable,empty,absent}/stderr`.

#### 3. Close 0263

Set `meta/work/0263-rename-foreigndirt.md` to `done` with `/update-work-item`,
recording `UnownedChanges` / `Ownership::Unowned` as the chosen names.

### Success Criteria

#### Automated Verification

- [x] `rg -n 'ForeignDirt|Ownership::Foreign|(?i)foreign[ _-]?dirt' cli skills`
  returns nothing.
- [x] Domain tests pass:
  `cargo nextest run --manifest-path cli/Cargo.toml -p migrate`.
- [x] Binary tests pass: `cargo nextest run --manifest-path cli/Cargo.toml -p
  accelerator-migrate --features bash-parity dirty_tree_preflight`.
- [x] Public API fixture is current: `mise run public-api:check`.
- [x] `mise run check` exits 0.
- [x] `mise run` exits 0.

#### Manual Verification

- [ ] In a scratch git repo with an unowned `meta/x.md`, `accelerator migrate`
  prints the refusal followed by `Unowned changes (1):` and `  meta/x.md`.

---

## Phase 2: Derive the Run Base from `@`'s Parents

### Overview

Fix #97 (requirements 1, 2 and 7):

- the run base follows `@`'s parents, read after the lock from the same load
  as the dirty paths;
- the stall-named decisions file is an owned change;
- a pre-upgrade stall is reported as stale;
- `CHANGELOG.md` explains the recovery.

### Changes Required

#### 1. `jj status` parity oracle and a hermetic migrate harness

**File**: `cli/vcs-test-support/src/jj_status.rs` (new, exported from `lib.rs`).

```rust
pub fn changed_paths(
    env: &Hermetic,
    root: &Path,
) -> Result<BTreeSet<String>, Error>
```

- Runs `env.jj(&["status"], root)`.
- Parses the lines under `Working copy changes:` whose first token is one of
  `A M D R C`.
- Expands `R prefix/{from => to}suffix` (and the brace-free `R from => to`)
  into both paths.
- Ignores `?` lines.

Its unit tests in the same file cover:

- a plain add;
- a braced rename;
- an unbraced rename;
- a `?` line;
- a status with no changes.

**Also**: `run_base_oracle(env, root)` in the same module. It runs
`jj log --no-graph -r 'parents(@)' -T 'commit_id ++ "\n"'` and returns the
sorted ids joined by `+`.

**Dirty-path fixture subcommand**:
`cli/vcs-adapters/tests/fixtures/vcs_adapters_fixture.rs` gains
`only dirty_paths <dir>`. It prints each path of `InProcessProbe.dirty_paths`
on its own line. On error it prints `error: <message>` to stderr and exits 1.
The parity checks in step 5 use it, so they already run under `env.apply`.

**Shared migrate test support**: `cli/migrate-cli/tests/common/mod.rs` holds
the helpers the binary tests share: the hermetic run helper,
`seed_ambiguous_reference`, `mark_all_migrations_applied` and, from step 5,
`stalled_0007`. `dirty_tree_preflight.rs` and the new test files use it
instead of local copies.

#### 2. Own the decisions file the stall names (git first)

**Red**: a new `cli/migrate-cli/tests/guarded_resume.rs` (`bash-parity`, binary
spawned with `env.apply`). Test
`a_git_stall_resumes_after_writing_its_named_decisions_file`:

1. Seed a git repo with the ambiguous reference and mark 0001–0006 applied.
   Commit, then run with stdin null. The run exits 1 with `MIGRATION STALLED`.
2. Write `accept\n` to the stall-named
   `.accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-decisions.txt`.
3. Run `git status`, then re-run with `--decisions-file <that path>`.
4. Assert: exit 0; stderr starts with `Resuming over this run's own partial
   migration output:` and lists the decisions file with no decision count;
   the applied ledger contains `0007-unify-meta-corpus-frontmatter`; no
   `ACCELERATOR_MIGRATE_FORCE` was set.

Domain tests in `cli/migrate/tests/manifest.rs`:

- `manifest::decisions_file("0007-unify-meta-corpus-frontmatter")` is
  `.accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-decisions.txt`;
- that path classifies as `SessionArtefact` when the run base matches and as
  `Unowned` when it does not;
- `is_session_log` is false for it;
- `.accelerator/state/migrations--decisions.txt` and `meta/x-decisions.txt`
  classify as `Unowned`.

**Green**: `cli/migrate/src/manifest.rs` adds `"-decisions.txt"` to
`SESSION_SUFFIXES` and `pub fn decisions_file(migration_id: &str) -> String`.
`render.rs`'s stall message builds its path with `decisions_file`, so the
name the stall prints and the name the pre-flight owns cannot drift. The
`SessionArtefact` doc widens to "files the current run's session writes or
reads back on resume".

```diff
-const SESSION_SUFFIXES: [&str; 3] =
-    ["-session.jsonl", "-stderr.log", "-resume-state.tmp"];
+const SESSION_SUFFIXES: [&str; 4] = [
+    "-session.jsonl",
+    "-stderr.log",
+    "-resume-state.tmp",
+    "-decisions.txt",
+];
```

`is_session_log` is unaffected: it matches `-session.jsonl` only.

#### 3. Base commits in the vcs adapter

The adapter speaks VCS terms: it reports the commits the working copy is
based on. Joining them into a run base is migrate's business (step 4).

**Red**: `cli/vcs-adapters/tests/base_commits.rs` (`bash-parity`, in-process).

The in-process call is safe in this phase: `snapshot::load` reads only jj-lib
defaults, as `tests/dirty_paths.rs` already relies on. Phase 3 moves these
cases onto the fixture binary. Each jj case runs for both `--no-colocate` and
`--colocate`, and compares the sorted `working_copy_state(..).base_commits`
against the ids the `parents(@)` oracle lists.

| Setup | Expectation |
|---|---|
| single parent | `@-`'s commit id |
| `@`'s parent is `root()` | 40 zeros |
| merge `jj new A B` and `jj new B A`; three-parent merge | equal to the oracle's ids; identical sets across parent orders |
| edit a file then `jj status`; `jj describe -m x` | unchanged |
| `jj describe @- -m y`; `jj new`; `jj commit -m z`; `jj rebase -r @ -d <other>`; `jj edit` to a change with another parent | changed, equal to the oracle's ids |
| `jj edit` to a sibling with the same parent | unchanged |
| merge, then `jj describe` one parent | changed |
| git repo | `[HEAD]` |
| unborn git repo | empty |
| no VCS | empty |
| corrupt `.jj/repo/op_heads` | an error |
| reading the base commits | op heads unchanged (the `op_heads` helper pattern from `dirty_paths.rs:119-126`) |

The same file asserts, for jj and for git, that `working_copy_state`'s
dirty paths equal `dirty_paths`.

`cli/vcs-adapters/tests/library.rs` gains one case (requirement 2): in a jj
repo with a committed parent and a dirty `@`, after `jj status`,
`InProcessProbe.revision` equals `jj log --ignore-working-copy -r @` and is
not among `working_copy_state(..).base_commits`.

**Green**: `cli/vcs-adapters/src/library/snapshot.rs`.
`working_copy_diff` returns a `SnapshotDiff { base_commits, diff }`: the
parent ids of the commit it diffed against and the diff, from the one
`load_at_head` view. `status_log::jj_status` reads `.diff`.

**Green**: `cli/vcs-adapters/src/library.rs` adds, beside `dirty_paths`:

```rust
pub struct WorkingCopyState {
    pub base_commits: Vec<String>,
    pub dirty_paths: Vec<String>,
}

impl InProcessProbe {
    pub fn working_copy_state(
        &self,
        root: &Path,
        kind: VcsKind,
    ) -> Result<WorkingCopyState, Error>;
}
```

There is no standalone base-commits method, so no caller can pair a base
from one read with dirty paths from another. `base_commits` is:

- on git, `HEAD` when it exists, read from the same `gix::Repository` handle
  the status walk uses;
- on jj, `@`'s `parent_ids()` as hex, from `SnapshotDiff`;
- with no VCS, empty.

The doc comment on `WorkingCopyState` states the invariant: both fields
come from one read of the repository, so the dirty paths are relative to
exactly these commits.

#### 4. The run base in the migrate domain

**Red**: unit tests for a new `cli/migrate/src/run_base.rs`:

- `RunBase::from_base_commits(&[])` is `None`;
- one id gives that id;
- `[b, a]` and `[a, b]` both give `a+b`;
- `RunBase::recorded` is an opaque, infallible read of what
  `migrations-run.id` already holds. It trims, maps empty to `None`, and
  accepts a 40-hex git id, a pre-upgrade jj `@` id and the golden
  `stale-revision-sentinel` as they are;
- `Display` of a recorded value gives back the trimmed text.

**Green**:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunBase(String);

impl RunBase {
    pub fn from_base_commits(ids: &[String]) -> Option<Self> {
        if ids.is_empty() {
            return None;
        }
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        Some(Self(sorted.join("+")))
    }

    pub fn recorded(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_owned()))
    }
}
```

`FileManifestStore` reads through `recorded` and writes `<run base>\n`, the
format it writes today. On git a single `HEAD` gives `HEAD`, so git run
bases recorded before the upgrade still match.

**Red**: `cli/migrate/tests/preflight.rs`:

- the force branch records the run base the working copy reports;
- under force, an observation with no run base still gives `Clean` and
  records `None`, so force stays the escape hatch for a damaged working
  copy;
- the run base is read after the lock is acquired (the lock double records
  the order of calls).

**Green**: ports and pre-flight.

- `cli/migrate/src/ports.rs`: `DirtyPathScanner` becomes `WorkingCopy`:

  ```rust
  pub struct WorkingCopyObservation {
      pub run_base: Option<RunBase>,
      pub dirty_paths: Vec<String>,
  }

  pub trait WorkingCopy {
      fn observe(
          &self,
          roots: &[&str],
      ) -> Result<WorkingCopyObservation, MigrationError>;
  }
  ```

  `ManifestStore::run_id` / `write_run_id` become `recorded_run_base` /
  `record_run_base` over `Option<RunBase>`.
- `MigrationContext::revision` is removed, with every double that implements
  it: `cli/migrate/tests/{list_and_decisions_file,lifecycle,engine}.rs` and
  `m0003`, `m0008`, `m0009`. `FileMigrationContext::revision` goes too.
- `cli/migrate/src/manifest.rs`: `classify(.., run_base_matches)`,
  `RunnerPaths::recorded_run_base`, and the doc wording.
- `cli/migrate/src/preflight.rs`: the `revision` field is removed and
  `scanner` becomes `working_copy`. `run` acquires the lock, then calls
  `observe` once. The force branch records `observation.run_base`. The rest
  compares the recorded run base with `observation.run_base`.
- `cli/migrate-adapters/src/dirty_path_scanner.rs` → `working_copy.rs`:
  `VcsWorkingCopy::observe` calls `InProcessProbe.working_copy_state` and
  builds the `RunBase` with `from_base_commits`. On error it keeps today's
  policy (warn, no dirty paths) and reports no run base. The renamed
  `cli/migrate-adapters/tests/working_copy.rs` asserts the run base too:
  `HEAD` on git, and `None` for an unreadable repository.
- `cli/migrate-cli/src/main.rs`: builds `VcsWorkingCopy::new(root, vcs_kind(root))`.
- `cli/migrate/tests/fixtures/public-api.txt` and
  `cli/kernel/tests/fixtures/public-api.txt`: regenerate with
  `mise run public-api:update`.

Under force, the pre-flight now reads the working copy where it used to
skip the scan. It uses only the run base from that read.

**Logging**: add `kernel::logging::init_if_requested()`. It holds the
`ACCELERATOR_LOG` gate and reports a malformed filter, as
`cli/vcs-cli/src/main.rs:98-102` does inline today. `vcs-cli` moves onto it,
and `accelerator-migrate`'s `main` calls it, so the adapter's warn is
visible from this phase on. Binary tests in `guarded_resume.rs` with a
corrupt `.jj/repo/op_heads` and `ACCELERATOR_LOG=warn` assert:

- without force: the warn reaches stderr and the pre-flight takes the clean
  branch, as today's policy decides;
- with `ACCELERATOR_MIGRATE_FORCE=1`: the warn reaches stderr, the
  pre-flight neither refuses nor fails, and `migrations-run.id` holds an
  empty run base while the run is in progress.

**`regenerate.sh`**: `gr_base_rev` records `parents(@)`'s sorted, joined
commit ids on jj, matching `run_base_oracle`.

The existing git guarded-resume test (`dirty_tree_preflight.rs:136`) seeds the
run base with `HEAD` and stays green unchanged.

#### 5. jj guarded resume, end to end

**Red → green**: `cli/migrate-cli/tests/guarded_resume.rs`. Each jj test runs
for `--no-colocate` and `--colocate`. The shared helper
`stalled_0007(env, colocate)` builds the phase-2-step-2 fixture on jj,
committed with `jj commit`, and returns the stall-named decisions path.

A successful run clears `migrations-run.id` (`manifest_store.clear()`,
`main.rs:286`), so every assertion on the recorded run base is made while
the run is stalled. The shared run helper in `common/mod.rs` sets
`ACCELERATOR_LOG=warn` on every spawn, so assertions on the presence or
absence of `WARN` are never vacuous.

| Test | Steps | Assertions |
|---|---|---|
| #97 reproduction | write decisions, `jj status`, re-run `--decisions-file` | resume message; 0007 applied; no force |
| owned edit | also edit an owned file, `jj status` / `jj describe -m x` | resume engages |
| owned and unowned | seed from `run_base_oracle`; add unowned `meta/a.md` and `.accelerator/b` | exit 1; both listed in order; no owned path; no stale line |
| stale by rebase | `jj rebase -r @ -d <sibling>` / `jj describe @- -m y` | exit 1; stale line; `Unowned changes` lists the run's recorded paths |
| fresh run | `jj new` / `jj commit -m z` / `jj edit <change with another parent>`, no scoped changes; re-run with no decisions input | no resume message; the run stalls again on the ambiguous reference; while stalled, `.accelerator/state/migrations-run.id` equals `run_base_oracle` |
| sibling edit | `jj edit` to a sibling with the same parent whose `meta/` content the run did not write | exit 1; that content listed as unowned |
| sibling edit, manifested path | `jj new @-`, then edit a path in the run's manifest | resume engages over it (path-based ownership, as on git) |
| merge | stall on a merge `@`; edit an owned file + `jj status`; then `jj describe <one parent>` | resume engages; then refused |
| clean-run base | clean jj repo holding an ambiguous reference, `accelerator migrate` with stdin null | the run stalls; while stalled, the run base file equals the oracle, for single, two- and three-parent merges and a root parent |
| rename | `jj file track`ed `meta/a.md` renamed to `meta/b.md`, and `meta/x/c.md` to `meta/y/c.md` | both sides listed |
| pre-upgrade stall | see below | see below |

**Pre-upgrade stall**. The fixture is a corpus where 0007 rewrites
`meta/one.md` and then stalls on an ambiguous reference in `meta/two.md`.
After the stall, `migrations-run.id` is overwritten with `@`'s commit id, as
the pre-upgrade binary wrote it. Only the run's changes are present.

1. The `--decisions-file` run exits 1. It prints the refusal with the stale
   line, listing the run's changes including `meta/one.md`. Its stderr has
   no `panicked`. The working copy (the `changed_paths` oracle plus file
   hashes under `meta/`) and the manifest files are byte-identical.
2. A second run is refused the same way.
3. The refusal's stale lines name the recovery.
4. An unrelated `src/unrelated.txt` is also changed. Following the
   `CHANGELOG.md` recovery,
   `jj commit -m "partial 0007" meta .accelerator .claude` leaves
   `src/unrelated.txt` in `@`. A run with `--decisions-file` and no force
   then exits 0, 0007 is applied, `meta/one.md` is unchanged by this run,
   and `meta/two.md` is migrated.

The decisions-file content follows the format `DecisionsFileDecisionSource`
parses, with one line per prompt still pending.

Every jj fixture above also asserts parity: the paths from
`vcs-adapters-fixture only dirty_paths` (run under `env.apply`, before
`jj status` snapshots) equal `jj_status::changed_paths`.

#### 6. `CHANGELOG.md`

Add `### Fixed` under `## [Unreleased]`, in the `- **Headline.** Prose…` style
(see 1.23.0 `Fixed`):

```markdown
- **`accelerator migrate` resumes stalled runs on jj.** The run base is now
  the parents of the working-copy commit, so `jj status`, edits and
  `jj describe` no longer make a stalled run look stale, and the dirty-tree
  refusal lists the unowned changes that blocked it. A jj run that stalled
  before this upgrade is refused as stale. Check that the listed paths are
  only that run's output, set aside anything else under them, then
  `jj commit -m "partial migration" meta .accelerator .claude` and re-run
  `accelerator migrate --decisions-file <path>` without
  `ACCELERATOR_MIGRATE_FORCE`. `jj undo` reverts the commit.
```

### Success Criteria

#### Automated Verification

- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support`
  passes (oracle parsing).
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-adapters
  --features bash-parity base_commits library` passes.
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-migrate
  --features bash-parity guarded_resume dirty_tree_preflight` passes.
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p migrate` passes.
- [ ] `mise run public-api:check` and `mise run check` exit 0.
- [ ] `mise run` exits 0.

#### Manual Verification

- [ ] In a real jj-colocated repo, reproduce #97:
  1. stall 0007;
  2. write the decisions file;
  3. run `jj status`;
  4. re-run with `--decisions-file`.

  The run resumes and 0007 completes.
- [ ] The `CHANGELOG.md` entry reads correctly at 80 columns.

---

## Phase 3: Honour the Git Excludes in the jj Dirty Paths

### Overview

Give the jj snapshot the `base_ignores` that jj-cli builds (requirement 3). The
change reaches all three consumers through `working_copy_diff`. Any failure
to read an excludes source warns and skips that source, so the result
over-reports changes and never errors.

### Changes Required

#### 1. Harness: environment overrides and hermetic binaries

**File**: `cli/vcs-test-support/src/hermetic.rs`. `Hermetic` gains builder
methods, applied by `apply` after the defaults:

- `with_git_global_config(path)`: sets `GIT_CONFIG_GLOBAL`;
- `with_xdg_config_home(path)`, `with_empty_xdg_config_home()` and
  `without_xdg_config_home()`;
- `with_jj_config(path)` and `without_jj_config()`;
- `jj_user_config_dir()`: `<config_home>/jj`.

**Files**:

- `cli/vcs-cli/tests/status_log_goldens.rs`, `status_log_parity.rs`: spawn the
  binary with `env.apply`, so a developer's own excludes or limit cannot leak
  into the goldens.
- `cli/vcs-adapters/tests/dirty_paths.rs`, `base_commits.rs` and the jj case
  in `library.rs`: move onto `vcs-adapters-fixture` under `env.apply`.
- `cli/work-adapters/tests/sync_working_copy_status.rs`: the jj cases
  (`jj_reports_an_uncommitted_file_as_dirty`,
  `a_colocated_checkout_is_read_through_jj`) move onto
  `work-adapters-fixture` under `env.apply`.

After this step no test computes jj dirty paths or base commits in the test
process.

**File**: `cli/vcs-adapters/tests/fixtures/vcs_adapters_fixture.rs` gains
`only base_commits <dir>` and `only revision <dir>`, shaped like phase 2's
`only dirty_paths`. `only base_commits` prints
`working_copy_state(..).base_commits`. Every subcommand now calls
`kernel::logging::init_if_requested()`.

**Work fixture**: a test-only binary `work-adapters-fixture`
(`cli/work-adapters/tests/fixtures/work_adapters_fixture.rs`), a plain
`[[bin]]` under `tests/fixtures/` like its `vcs-adapters` siblings, with the
same Cargo comment on why it carries no feature gate.

- Usage: `work-adapters-fixture <start> <path>...`. It prints
  `<path>\t<dirty|clean|unknown>` from `VcsWorkingCopyStatus::probed_from`.
- It calls `kernel::logging::init_if_requested()`.

#### 2. Resolve the excludes file (pure)

**Red**: unit tests in the new `cli/vcs-adapters/src/library/git_excludes.rs`
for:

```rust
fn excludes_file_path(
    configured: Option<&str>,
    workspace_root: &Path,
    home: Option<&Path>,
    xdg_config_home: Option<&OsStr>,
) -> Option<PathBuf>
```

| Input | Result |
|---|---|
| absolute `configured` | as given |
| `~/ignore-file` | `<home>/ignore-file` |
| `~/ignore-file`, no home | `None` |
| relative `configured` | `<workspace_root>/<configured>` |
| unset, non-empty XDG | `<xdg>/git/ignore` |
| unset, XDG empty or unset | `<home>/.config/git/ignore` |
| unset, no home, no XDG | `None` |

#### 3. Build the base ignores

**Green**: in `git_excludes.rs` (settings-free, so outside the lint's reach):

```rust
pub(super) fn base_ignores(
    workspace_root: &Path,
    store: &Store,
) -> Arc<GitIgnoreFile> {
    let backing = backing_git_repo(store);
    let configured = match &backing {
        Some(repo) => repo.config_snapshot().string("core.excludesFile"),
        None => global_git_config_excludes_file(),
    };
    let mut ignores = GitIgnoreFile::empty();
    if let Some(path) = excludes_file_path(/* ... */) {
        ignores = chain_or_warn(ignores, path);
    }
    if let Some(repo) = &backing {
        let exclude = repo.path().join("info").join("exclude");
        ignores = chain_or_warn(ignores, exclude);
    }
    ignores
}
```

- **`core.excludesFile`**: read as a raw string, as jj-cli does.
  `excludes_file_path` is the single place `~`, relative-path and XDG
  resolution happen.
- **`backing_git_repo`**: maps only jj-lib's "not a git backend" error to
  `None`. Any other `get_git_repo` failure warns and also gives `None`.
- **`global_git_config_excludes_file`**: reads the global and XDG git config
  through gix. An unreadable config warns and gives `None`.
- **`chain_or_warn`**: when `chain_with_file` fails, it warns naming the
  file and returns the chain unchanged.

`snapshot::working_copy_diff` sets
`base_ignores: git_excludes::base_ignores(root, repo.store())` in place of
`GitIgnoreFile::empty()`. The `snapshot.rs` module doc drops the claim that
excludes are unset.

#### 4. Consumer and parity tests

**Red → green**:

`cli/vcs-adapters/tests/dirty_paths_excludes.rs` (`bash-parity`). It drives
`vcs-adapters-fixture only dirty_paths` with `env.apply` plus overrides. Each
case runs for `--no-colocate` and `--colocate`. There is one untracked
`meta/ignored.md` and a control `meta/visible.md`. Each case asserts that
the control is listed, the ignored file is listed or not as stated, stderr
has no `WARN` under `ACCELERATOR_LOG=warn`, and the result equals
`jj_status::changed_paths`:

1. The pattern is in the file `core.excludesFile` names (a global git config
   written by the test).
2. The pattern is in the backing repo's `info/exclude`: `.git/info/exclude`
   when colocated, `.jj/repo/store/git/info/exclude` otherwise.
3. `core.excludesFile` is unset and XDG is non-empty: the pattern is in
   `$XDG_CONFIG_HOME/git/ignore`.
4. `core.excludesFile` is unset and XDG is unset: the pattern is in
   `~/.config/git/ignore`. Repeat with XDG empty.
5. `core.excludesFile = ignore-file` (relative): resolved against the
   workspace root.
6. `core.excludesFile = ~/ignore-file`: resolved against `HOME`.
7. Negative: `core.excludesFile` is set to a file without the pattern, and the
   pattern is only in the XDG default. The file is listed.
8. Negative: a tracked `meta/tracked.md` matching an exclude is modified. It is
   listed.
9. Unreadable: `core.excludesFile` names a directory. The file is listed and
   stderr has a `WARN` naming the path. Parity is not asserted.
10. No git backend: a repository created through jj-lib's simple-backend
    initialisation in the test, with the pattern in the global
    `core.excludesFile`. The file is not listed. The red step settles once,
    against the pinned jj 0.43.0, whether `jj status` opens such a
    repository. The test then either always asserts parity or never does,
    and its name says which.

Consumer tests, for cases 1, 2, 7 and 8, each with the control
`meta/visible.md` present in a second run of the same setup:

- **migrate**: `cli/migrate-cli/tests/preflight_vcs_parity.rs`, spawned with
  `env.apply`.
  - Cases 1 and 2 with only the ignored file: exit 0, no `WARN`. With the
    control added: refused, listing only `meta/visible.md`.
  - Cases 7 and 8 are refused, listing the path.
- **work sync**: `cli/work-adapters/tests/sync_working_copy_status.rs` gains
  cases driving `work-adapters-fixture`. Cases 1 and 2 report `clean` for the
  ignored file and `dirty` for the control. Cases 7 and 8 report `dirty`.
  A locally edited work-item file stays protected by the sync-state hash
  baseline, not by dirtiness, and the existing `LocallyModified` cases
  pin that.
- **renderer**: `cli/vcs-cli/tests/report_excludes.rs` spawns
  `accelerator-vcs status` with `env.apply`. Cases 1 and 2 list the control
  but not the ignored file, and never print `(status unavailable)`. Cases 7
  and 8 list the path.

Git criterion (requirement 6): `dirty_tree_preflight.rs` gains cases where the
only untracked `meta/` file matches `core.excludesFile` or `.git/info/exclude`,
and migrate proceeds. A control run with an extra unignored file is refused.

#### 5. `CHANGELOG.md`

Add to `### Fixed`:

```markdown
- **jj dirty-path checks honour the git excludes.** `accelerator migrate`,
  `work sync` and `accelerator vcs status` now skip untracked files that
  `core.excludesFile`, the XDG `git/ignore` or the backing repo's
  `info/exclude` ignore, as `jj status` does. Such files are no longer
  protected by the migrate pre-flight, so track or back them up before
  migrating.
```

### Success Criteria

#### Automated Verification

- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-adapters
  --features bash-parity` passes (unit and excludes tests).
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p work-adapters
  --features bash-parity` passes.
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-vcs
  --features bash-parity` passes, with goldens unchanged.
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-migrate
  --features bash-parity` passes.
- [ ] `mise run lint:vcs-settings:check` exits 0 with the exemption list
  unchanged.
- [ ] `mise run check` exits 0.
- [ ] `mise run` exits 0.

#### Manual Verification

- [ ] In a jj repo with a global `core.excludesFile` ignoring `*.scratch`,
  `accelerator vcs status` and `jj status` agree on a new
  `meta/x.scratch`.

---

## Phase 4: Honour `snapshot.max-new-file-size` in the jj Dirty Paths

### Overview

Resolve the limit as jj 0.43.0 does for `jj status`, conditional scopes
included, read-only and without `UserSettings`. Pass it to the snapshot
(requirement 4). An unresolvable limit warns and falls back to no limit,
today's behaviour, so it over-reports changes and never errors.

### Changes Required

#### 1. Config sources, injected

**File**: new `cli/vcs-adapters/src/library/jj_config.rs`. It is settings-free.
`jj_user_name`'s path helpers (`library.rs:780-850`) move here.

```rust
pub(super) struct JjConfigEnvironment {
    pub jj_config: Option<OsString>,
    pub user_config_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub system_config_paths: Vec<PathBuf>,
    pub hostname: String,
    pub variables: HashMap<String, String>,
}

impl JjConfigEnvironment {
    pub(super) fn from_process() -> Self;

    fn status_resolution_context<'a>(
        &'a self,
        workspace_root: &'a Path,
        repo_path: &'a Path,
    ) -> ConfigResolutionContext<'a>;
}

pub(super) struct JjConfigSources {
    pub system: Vec<PathBuf>,
    pub user: Vec<PathBuf>,
    pub repo: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
}

impl JjConfigSources {
    pub(super) fn for_workspace(
        workspace_root: &Path,
        repo_path: &Path,
        environment: &JjConfigEnvironment,
    ) -> Self;

    pub(super) fn user_level(environment: &JjConfigEnvironment) -> Self;

    fn resolved(
        &self,
        context: &ConfigResolutionContext,
    ) -> Result<StackedConfig, Error>;
}
```

`from_process` is the only place the process environment is read. It
fills the two fields that jj's `--when.hostnames` and `--when.environments`
conditions match against, the same way jj-cli 0.43 fills them for
`ConfigResolutionContext`:

- **`hostname`**: from `whoami::fallible::hostname()`, empty on failure;
- **`variables`**: a snapshot of `std::env::vars_os()`, keeping only pairs
  that are valid UTF-8.

The red step checks both sources against the jj-cli 0.43.0 source before
the green step relies on them.

**Dependency**: `whoami` is added to `cli/vcs-adapters/Cargo.toml` at the
version jj-cli 0.43.0 depends on. Its licence (`Apache-2.0 OR BSL-1.0 OR
MIT`) and advisories must clear the cargo-deny lane documented in
`tasks/README.md`, and `cli/Cargo.lock` is updated in the same change.

`for_workspace` builds the four layers as follows:

- **system**: `environment.system_config_paths`, only when `jj_config` is
  unset;
- **user**: the `JJ_CONFIG` paths, or the platform `config.toml`, `conf.d`
  and legacy `~/.jjconfig.toml`;
- **repo**: `per_id_config_file(repo_path, "config-id", "config.toml",
  "repos")`;
- **workspace**: `per_id_config_file(workspace_root/.jj,
  "workspace-config-id", "workspace-config.toml", "workspaces")`.

`user_level` builds the system and user layers only. `jj_user_name` builds
its stack from it, and its doc drops the claim that the per-id layout is
disproportionate to replicate. It now says that `user.name` deliberately
reads only the system and user layers.

`per_id_config_file` is a read-only resolver:

- It reads the id file and requires exactly 20 lowercase hex characters.
- It then joins `<user config dir>/jj/<repos|workspaces>/<id>/config.toml`.
- With no id file, it falls back to the legacy file inside `.jj`.
- It never creates, migrates or rewrites anything.

`resolved` loads the layers in `System`, `User`, `Repo`, `Workspace` order
with `load_jj_config_path`. It then passes the stack through
`jj_lib::config_resolver::resolve`, with the context from
`status_resolution_context`. That context is populated as jj-cli populates
it for `jj status`: `home_dir`, `repo_path`, `workspace_path`, `command`
`"status"`, `hostname` and `environment` (jj-lib 0.43.0,
`config_resolver.rs:44-58`). jj-lib evaluates `--when.platforms` itself
from `std::env::consts`.

The `cli/Cargo.toml` comment on the jj-lib pin gains `jj_config.rs` in its
list of modules to re-verify on a bump, since it mirrors jj-cli's
on-disk layout.

#### 2. The limit

```rust
pub(super) enum MaxNewFileSize {
    Bytes(NonZeroU64),
    Unlimited,
}

impl MaxNewFileSize {
    const JJ_DEFAULT: u64 = 1024 * 1024;

    pub(super) fn resolve(
        sources: &JjConfigSources,
        context: &ConfigResolutionContext,
    ) -> Result<Self, UnresolvableLimit>;

    pub(super) fn resolve_or_unlimited(
        sources: &JjConfigSources,
        context: &ConfigResolutionContext,
    ) -> Self;

    pub(super) fn as_snapshot_limit(&self) -> u64;
}
```

- `resolve` reads `snapshot.max-new-file-size` via
  `get_value_with(.., HumanByteSize::try_from)`.
- `NotFound` falls back to `JJ_DEFAULT`.
- A configured `0` becomes `Unlimited` at construction, so "no limit" has
  one representation.
- A bad value or a malformed layer file becomes `UnresolvableLimit`. This is
  a module-private error whose `Display` names `snapshot.max-new-file-size`
  and the source error. The crate's public `Error` is unchanged, since no
  consumer ever receives this failure.
- `resolve_or_unlimited` warns with the error and returns `Unlimited` on any
  failure.
- `as_snapshot_limit` maps `Unlimited` to `u64::MAX` and `Bytes(n)` to `n`.

**Red first**, in-process unit tests in `jj_config.rs`. They build
`JjConfigEnvironment` over a temp dir, so the process environment is never
read:

- **Winning 1 KiB layer setups**:
  - repo 1 KiB over user 1 MiB;
  - workspace 1 KiB over repo 1 MiB;
  - user-only 1 KiB;
  - user 1 KiB over system 1 MiB;
  - system-only 1 KiB.
- **Layer selection**: `JJ_CONFIG` set drops the system layer; `JJ_CONFIG`
  replaces the platform user files; `conf.d` wins over `config.toml`.
- **Value forms**: repo `1024`, `"1024"` and `"1KiB"`.
- **Later layer raises the limit**:
  - user 1 KiB → repo 1 MiB;
  - repo 1 KiB → workspace 1 MiB;
  - system 1 KiB → user 1 MiB.
- **Conditional scopes**:
  - a user `[[--scope]]` with `--when.repositories` naming this repo sets
    1 KiB, which wins;
  - the same scope naming another repo leaves 1 MiB;
  - a `conf.d` file with a file-level `--when.repositories` for another
    repo is not applied;
  - a `--when.commands = ["log"]` scope is not applied;
  - a `--when.hostnames` scope naming the environment's `hostname` sets
    1 KiB, and one naming another host leaves 1 MiB;
  - a `--when.environments` scope matching a variable in `variables` sets
    1 KiB, and one whose variable is absent leaves 1 MiB.
- **Unset and zero**: unset gives 1 MiB, and `0` gives `Unlimited`, which
  `as_snapshot_limit` maps to `u64::MAX`.
- **Invalid**: `"abc"` gives an error whose `Display` contains
  `snapshot.max-new-file-size`. `resolve_or_unlimited` gives `Unlimited`.
- **Malformed layer**: a repo layer that is not valid TOML gives `Unlimited`
  from `resolve_or_unlimited`.
- **`per_id_config_file`**: id file present; malformed id gives `None`;
  legacy `.jj/repo/config.toml`; legacy `.jj/workspace-config.toml`.
- **Read-only**: hashes of the temp user jj config dir,
  `.jj/repo/config.toml`, `.jj/workspace-config.toml`, `.jj/repo/config-id`
  and `.jj/workspace-config-id` are identical before and after
  `for_workspace` + `resolve`.

#### 3. Wire it into the snapshot

**File**: `cli/vcs-adapters/src/library/snapshot.rs`. A new
`snapshot_options(&workspace, &repo)` builds the options from the workspace
root, and `working_copy_diff` calls it before taking the working-copy lock:

```rust
fn snapshot_options(
    workspace: &Workspace,
    repo: &ReadonlyRepo,
) -> SnapshotOptions<'static> {
    let root = workspace.workspace_root();
    let environment = JjConfigEnvironment::from_process();
    let sources = JjConfigSources::for_workspace(
        root,
        workspace.repo_path(),
        &environment,
    );
    let context =
        environment.status_resolution_context(root, workspace.repo_path());
    SnapshotOptions {
        base_ignores: git_excludes::base_ignores(root, repo.store()),
        progress: None,
        start_tracking_matcher: &EverythingMatcher,
        force_tracking_matcher: &NothingMatcher,
        max_new_file_size: MaxNewFileSize::resolve_or_unlimited(
            &sources, &context,
        )
        .as_snapshot_limit(),
    }
}
```

The module doc's "ignoring the user's own `snapshot.max-new-file-size`"
sentence is replaced by one stating that only this key is read from the user's
config stack.

#### 4. End-to-end and parity tests

**Red → green**: `cli/vcs-adapters/tests/dirty_paths_size.rs` (`bash-parity`),
via `vcs-adapters-fixture only dirty_paths` with `env.apply`.

- **Fixture**: fresh untracked `meta/two` (2048 B), `meta/one` (1024 B) and
  `meta/half` (512 B). Config is written to the paths jj uses:
  - `jj config set --repo` / `--workspace` under `Hermetic`;
  - `JJ_CONFIG` for the user layer.
- **Colocation**: both `--no-colocate` and `--colocate`.
- **Assertions**: the listed files, no `WARN` under `ACCELERATOR_LOG=warn`,
  and equality with `jj_status::changed_paths`.

Cases, which are the work item's non-system layer setups plus scopes:

1. repo 1 KiB, user 1 MiB;
2. workspace 1 KiB, repo 1 MiB;
3. user 1 KiB via `JJ_CONFIG`;
4. repo `1024`, `"1024"` and `"1KiB"`;
5. user 1 KiB → repo 1 MiB;
6. repo 1 KiB → workspace 1 MiB;
7. unset: exactly 1,048,576 B is listed and 2 MiB is not;
8. `0`: 2 MiB is listed;
9. legacy `.jj/repo/config.toml` 1 KiB with a user 1 MiB, hashes unchanged
   across the one fixture call;
10. legacy `.jj/workspace-config.toml`, the same;
11. copied repository: `jj config set --repo` 1 KiB and `jj status` at path A,
    `cp -R` to B, fixture at B. It omits the 2 KiB file, lists the 1024 B file,
    and the config paths and hashes are unchanged;
12. a user `[[--scope]]` with `--when.repositories` naming this repo sets
    1 KiB; the same scope naming another repo leaves 1 MiB;
13. a user `[[--scope]]` with `--when.environments` on a variable the test
    sets through `Hermetic` sets 1 KiB; with the variable unset, 1 MiB;
14. a user `[[--scope]]` with `--when.hostnames` naming the output of the
    `hostname` command sets 1 KiB. As a precondition,
    `jj config get snapshot.max-new-file-size` in the repo returns `1KiB`,
    which proves jj matched the host. The fixture then omits the 2 KiB file.

The system-layer setups are covered only by the phase 4 step 2 unit tests,
because neither the fixture nor `jj status` can redirect `/etc/jj`. The parity
criterion excludes them for the same reason.

Consumer tests (requirement 4 across all three):

- **migrate** (`preflight_vcs_parity.rs`, `env.apply`):
  - 1 KiB winning: the refusal lists `meta/one` and `meta/half`, not
    `meta/two`.
  - Unset limit with only a 2 MiB file: exit 0 and no `WARN`. With a
    control `meta/half` added: refused, listing only `meta/half`.
- **work sync** (`work-adapters-fixture`), for each 1 KiB-winning non-system
  setup:
  - only `meta/two` present: `clean`;
  - only `meta/one` present: `dirty`.

  For each 1 MiB-winning setup, only `meta/two` present gives `dirty`. With
  `0`, 2 MiB gives `dirty`.
- **renderer** (`accelerator-vcs status`, `env.apply`): the same lists as
  migrate, and never `(status unavailable)`.
- **invalid limit**: repo `"abc"` plus an unowned change at `meta/a.md` and
  a fresh 2 MiB `meta/two`, with `ACCELERATOR_LOG=warn`. The limit falls back
  to none, so both are changes.

  | Consumer | Outcome | stderr |
  |---|---|---|
  | migrate | exit 1, refusal lists `meta/a.md` and `meta/two` | a WARN line containing `snapshot.max-new-file-size` |
  | work fixture | `dirty` for both | the same |
  | `accelerator-vcs status` | lists both | the same |

#### 5. `CHANGELOG.md`

Add to `### Fixed`:

```markdown
- **jj dirty-path checks honour `snapshot.max-new-file-size`.**
  `accelerator migrate`, `work sync` and `accelerator vcs status` now skip
  new files over the limit, as `jj status` does, including jj's 1 MiB
  default when the key is unset. Such files are no longer protected by the
  migrate pre-flight. Conditional `[[--scope]]` settings apply as they do for
  `jj status`. An invalid value checks every file and logs a warning, shown
  with `ACCELERATOR_LOG=warn`.
```

### Success Criteria

#### Automated Verification

- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-adapters
  --features bash-parity` passes, including `jj_config` unit tests and
  `dirty_paths_size`.
- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p work-adapters -p
  accelerator-vcs -p accelerator-migrate --features bash-parity` passes.
- [ ] The cargo-deny lane passes with `whoami` added.
- [ ] `mise run lint:vcs-settings:check` exits 0 with the exemption list
  unchanged.
- [ ] `mise run check` exits 0.
- [ ] `mise run` exits 0.

#### Manual Verification

- [ ] In a real jj repo with `jj config set --repo snapshot.max-new-file-size
  1KiB` and a new 2 KiB file under `meta/`, the file is absent from
  `accelerator vcs status`, `accelerator migrate` does not refuse over it, and
  `jj status` lists it as `?`.
- [ ] Outside any jj repository, run `jj config path --user` to find the
  user config directory. Then create a scratch repo that has no config id,
  list its `repos/` directory, run `accelerator vcs status` in the repo,
  and list `repos/` again. No new id directory appears. Running
  `jj config path` inside the repo would create one itself, so don't.

---

## Testing Strategy

### Unit Tests

- **Ownership** (`cli/migrate/tests/manifest.rs`, `preflight.rs`): unowned-path
  listing from unsorted input, the stale flag, stale and absent manifests
  and run bases, the decisions-file artefact and its negatives.
- **Run base** (`cli/migrate/src/run_base.rs`): empty, single, order
  independence, and opaque reads of every existing `migrations-run.id` form.
- **Refusal rendering** (`render.rs`): count, stale lines with the recovery,
  control characters escaped, non-ASCII and quotes untouched.
- **Oracle parsing** (`vcs-test-support`): change lines, renames, `?` lines.
- **Excludes path resolution** (`git_excludes.rs`): every `core.excludesFile`
  form and fallback.
- **Limit resolution** (`jj_config.rs`): every layer setup including the
  system layer, layer selection, conditional scopes (repositories,
  commands, hostnames, environments), value forms, `0`, the
  default, invalid values and malformed layers, legacy and id-file
  discovery, and read-only hashes.

### Integration Tests

- **Base commits** (`vcs-adapters/tests/base_commits.rs`): against the
  `parents(@)` oracle, across every jj operation the criteria name.
- **Dirty-path parity** (`dirty_paths_excludes.rs`, `dirty_paths_size.rs`, and
  every `guarded_resume.rs` fixture): equality with `jj status` change lines,
  always through the fixture binary.
- **Consumers**: migrate, `work sync` and the renderer through their own
  binaries or fixture binaries, always under `Hermetic::apply`, each positive
  case paired with a control that must be reported.

### Manual Testing Steps

1. Reproduce #97 on a jj-colocated clone at the pre-change commit, then
   confirm the resume on this branch.
2. Stall on the pre-change binary, upgrade, and follow the `CHANGELOG.md`
   recovery: check the listed paths, commit, then resume without force.
3. Compare `accelerator vcs status` with `jj status` in a repo using a global
   excludes file and a repo-level size limit.

## Performance Considerations

- ⏱️ `working_copy_diff` gains the following per call, all small next to the
  snapshot walk:
  - one `StackedConfig` build and scope resolution, reading up to ~6 small
    TOML files;
  - one hostname lookup and one environment snapshot;
  - two ignore-file reads;
  - one gix config snapshot, already opened by the backend.
- The run base comes from the same load as the dirty paths, so it adds no
  repository load. A forced run now reads the working copy once, where it
  used to skip the scan.

## Migration Notes

- **Pre-upgrade jj stalls**: they are stale by construction, and the refusal
  says so. Recovery is to check the listed paths are only the run's output,
  commit them, and re-run without force, as documented in `CHANGELOG.md`
  (phase 2). Git run bases are unchanged, so git stalls resume across the
  upgrade.
- **Excludes and size limit**: on jj, untracked `meta/` files that the git
  excludes ignore, or that exceed `snapshot.max-new-file-size` (1 MiB when
  unset), no longer count as changes, matching `jj status`. They are no
  longer protected by the migrate pre-flight.

## References

- Work item: `meta/work/0241-migrate-false-dirty-tree-detection.md`
- Research: `meta/research/codebase/2026-09-23-0241-migrate-false-dirty-tree-detection.md`
- Rename: `meta/work/0263-rename-foreigndirt.md`
- Review: `meta/reviews/plans/2026-09-23-0241-migrate-false-dirty-tree-detection-review-1.md`
- Prior plans:
  - `meta/plans/2026-06-21-0119-resume-safe-partial-migration-failure.md`
  - `meta/plans/2026-08-03-0188-library-backed-vcs-adapter.md`
  - `meta/plans/2026-08-31-0198-vcs-agnostic-status-log-renderer.md`
- Issue: https://github.com/atomicinnovation/accelerator/issues/97
