---
type: "codebase-research"
id: "2026-11-18-0365-build-the-vcs-test-fixtures-once-per-test-run"
title: "Build the VCS test fixtures once per test run"
date: "2026-11-18T18:56:36+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0365"
parent: "work-item:0365"
relates_to: ["codebase-research:2026-10-28-full-build-duration-contention-and-slowness", "codebase-research:2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes"]
topic: "Build the VCS test fixtures once per test run"
tags: ["research", "codebase", "vcs-test-support", "vcs-adapters", "vcs-cli", "tasks", "nextest", "mise", "testing", "performance"]
revision: "d70239480bdf055a8e23952e1554c755f7eb5558"
repository: "accelerator"
last_updated: "2026-11-18T18:56:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Build the VCS test fixtures once per test run

**Date**: 2026-11-18T18:56:36+00:00
**Author**: Toby Clemson
**Git Commit**: d70239480bdf055a8e23952e1554c755f7eb5558
**Branch**: none (jj working copy on the `build-system` workspace, not pushed)
**Repository**: accelerator

## Research Question

What does the codebase need to know, and what does it constrain, to
implement work item 0365? The work item asks for three things:

- build the VCS matrix and status-log fixture families once per
  `test:unit:cli` run, in a build step of the invoke task;
- share those builds by adoption or copying, with hash and path guards, a
  fixture log and cleanup;
- give every multi-test `test:*` task a `--filter` option.

The research also answers the work item's open questions.

## Summary

The building blocks exist, but several facts in the codebase change the shape
of the plan.

1. **The matrix cannot be copied, but every matrix consumer can adopt it.**
   The matrix embeds absolute paths in many places. Some come from the
   crate's own code: the manifest, the CG grafted `.jj/repo`, the D1/D3
   `.jj/repo` pointers, the D2 `gitdir:` line, and the HOSTILE config and
   include. Others come from git: every linked worktree's `gitdir`, and every
   submodule URL. All 19 matrix consumers are read-only, with one exception:
   `scrub` writes `poison.gitconfig` into the matrix base. That file's
   location is never asserted, so writing it to the test's own temp
   directory loses nothing. **Every matrix consumer can adopt, and no matrix
   copy is needed.** That leaves the "[each family]" copy criteria with
   nothing to check for the matrix. A full matrix copy would also always
   fail the path scan, by design.
2. **The status-log set mostly copies cleanly, but a few absolute paths need
   a decision.**
   - jj 0.43 writes the secondary-workspace `.jj/repo` pointer and
     `git_target` as relative paths.
   - The jj op store holds no absolute path. This was checked empirically.
   - These do carry absolute paths:
     - the `remote.origin.url` of `git-ahead` and `git-behind`;
     - their reflogs ("clone: from …");
     - `origin.git/config`, which points at `seed`;
     - `adversarial-git`'s `core.hooksPath`;
     - the manifest.

   The path scan in Requirement 3 would flag copies that include those
   states.
3. **The audit table needs two corrections.**
   - `status_log_goldens` does not run `jj status`. The `accelerator-vcs`
     binary uses jj-lib in-process: it takes the working-copy lock and
     snapshots, which can write store objects. It is still a writer, so it
     still needs a copy.
   - `conflicts` writes on two paths. The `jj resolve --list` oracle
     snapshots, and its in-process `InProcessProbe::status_report` on
     `conflict-jj` takes the same lock and runs the same jj-lib snapshot.
     Adding `--ignore-working-copy` to the oracle is therefore not enough on
     its own for `conflicts` to adopt.
   - `accelerator-vcs` lives in `cli/vcs-cli/`.
4. **The `status_log_states` self-test can adopt without change.**
   `adversarial-git` is part of what `States::build_or_adopt` builds, not
   something the test adds. A shared build made with `build_or_adopt`
   already contains it.
5. **The plain-git subset is the first three states of the full set, built
   by the same function.** The two subset tests use only `clean-git`, which
   can come from the shared set. That set holds 19 golden states plus
   `adversarial-git`, so 20 keys.
6. **`Hermetic::rooted_at(base)` writes into `base`.** It creates
   `hermetic-home/` and `hermetic-xdg/` and rewrites `hermetic-jj.toml`. An
   adopting test must root its `Hermetic` in its own temp directory.
7. **The shared trees cannot live under `cli/target/`.**
   `assert_no_repository_ancestor` rejects any base with a `.git` or `.jj`
   ancestor, and the `no-repo` state depends on there being none. This rules
   out every location inside the repo, so the trees belong under the system
   temp directory.
8. **`test:unit:cli` constrains `--filter` and the build step in four ways.**
   - A filtered run with coverage on (the default) will fail the coverage
     floor (`tasks/test/cli.py:95-104`). The work item does not say what a
     filtered run should do about the floor.
   - `cargo llvm-cov` has no `list` mode. Under coverage, `cargo nextest
     list` must run in the `show-env` environment with `CARGO_TARGET_DIR`
     set to the instrumented directory, as the sweep already does. Otherwise
     it compiles a second, uninstrumented build.
   - `measure:build` sums every output line that starts with `Summary [`.
     It also takes a line that is exactly `Finished in …` as the task's
     duration.
   - The zero-spawn precedent built fixtures by running one test through
     `cargo nextest run`. That would now print a second `Summary [` line.
9. **mise 2026.10.8 supports declared task flags.** A `usage` field
   containing `flag "--filter <filter>"` reaches `run` as `$usage_filter`.
   The flag is listed in `--help`, and undeclared words are rejected.
   - Once `usage` is declared, mise stops appending raw arguments.
   - `depends` tasks do not receive the flag.
   - Ruff's A002 rule flags a Python parameter named `filter`.
10. **Copy fidelity: jj compares only file type, size and mtime (ms).** A
    file counts as clean only if its mtime is earlier than `tree_state`'s
    own mtime, so a copy must keep `tree_state`'s mtime too.
    - git always rewrites a copy's index on `git status`, because ctime and
      inode always change. The porcelain output stays the same.
    - gix never writes, and reads "ctime" from birth time.
    - On APFS, `cp -c -R -p` (clonefile) keeps nanosecond mtimes.
    - On ext4 (`ubuntu-latest`) there is no reflink, so a copy is a full
      copy. `std::fs::copy` on Linux does not keep mtimes, so a Rust copier
      would have to set them; `filetime` is already in the lock
      transitively.

## Detailed Findings

### The matrix (`cli/vcs-test-support/src/fixtures.rs`)

- **How the tree is built.**
  - `Matrix::build_in` (`fixtures.rs:167-186`) builds 34 keyed fixtures and
    about 40 top-level entries under one canonicalised base.
  - Before building, it checks:
    - the git version is at least 2.45 (`hermetic.rs:19,334-347`);
    - no ancestor of the base is a repository (`hermetic.rs:280-297`).
  - It then roots `Hermetic` at the base (`fixtures.rs:171`), which puts
    `hermetic-home/`, `hermetic-xdg/` and `hermetic-jj.toml` inside the
    tree.
  - Every `git` and `jj` call runs **sequentially** through
    `Hermetic::git`/`::jj` → `run` (`hermetic.rs:176-268`).
  - The layout table is in the matrix analysis: parents, then git, jj,
    nested, submodule, format and degenerate shapes (`fixtures.rs:202-519`).
- **The adopt path.**
  - `MATRIX_ROOT_VARIABLE = "ACCELERATOR_ZERO_SPAWN_MATRIX"`
    (`fixtures.rs:25`).
  - `matrix_root()` (`:47-57`) returns `(Option<TempDir>, PathBuf)`. If the
    variable is set it returns that path, with no guard and no existence
    check. Otherwise it returns a `vcs-matrix-` temp directory.
  - `build_or_adopt` (`:109-116`) adopts when `fixtures.tsv` exists.
    Otherwise it builds and then `persist`s (`:118-132`).
  - The manifest write is a plain `fs::write`, and there is no locking.
  - `adopt` (`:134-160`) checks only that each line has three fields and
    that the manifest is not empty. **It uses the manifest's absolute paths
    verbatim** (`:150`) and never rebases them onto `base`.
- **Absolute paths written by the crate's code:**
  - the manifest;
  - CG `.jj/repo` (`:633-636`), deliberately rewritten as absolute because
    the relative path breaks after the graft (`:615-618`);
  - D1 `.jj/repo` (`:488`);
  - D2 `.git` `gitdir:` (`:498-501`);
  - D3 `.jj/repo` (`:514`);
  - the HOSTILE `.git/config` pager, fsmonitor, diff, filter and alias
    values, plus an `[include] path` (`:660-683`);
  - `hostile-include`.
- **Absolute paths written by git.** Hermetic sets `GIT_CONFIG_GLOBAL=/dev/null`,
  so `worktree.useRelativePaths` is off.
  - Linked worktrees (WT, CG, NGJ/sub, NGPJ/sub, smw, supwt-wt) write
    absolute `gitdir` pointers in both directions. This was confirmed with
    git 2.56.
  - Submodule URLs (midsrc, super, supm, supw, supwt-wt) are absolute,
    because `add_submodule` passes an absolute source (`:569`).
- **No hashing, copying, event logging or mtime handling exists** anywhere
  in `vcs-test-support`.
  - `sha2 0.11` is a workspace dependency (`cli/Cargo.toml:100`), but this
    crate does not use it.
  - `walkdir` and `filetime` are in the lock only transitively.
  - No reflink or clonefile crate is present.
- **The crate has no `[[bin]]`, by design.** It is "a crate rather than a
  feature on `vcs-adapters`" (`lib.rs:4-6`). Its dependencies are
  `tempfile`, `regex`, `serde` and `toml`.

### The status-log set (`cli/vcs-test-support/src/status_log.rs`)

- **`build_states`** (`:296-315`) builds 19 golden states as siblings under
  `work`, plus the helper directories `seed`, `origin.git` and
  `jj-secondary-main`. Each state has `-status` and `-log` goldens: 38 files
  under `cli/vcs-test-support/fixtures/vcs-status-log/`.
- **`States::build_or_adopt`** (`:504-525`):
  - checks that no ancestor is a repository (`:508`);
  - canonicalises the base;
  - roots `Hermetic` at the base (`:510`);
  - builds the 19 states, then `adversarial-git` (`:516-520`);
  - writes `status-log-states.tsv` (`:527-535`).

  The result has **20 keys**. Unlike the matrix, it does not check the git
  version.
- **Spawn counts.** About 73 spawns per `build_or_adopt`: 47 git and 17 jj
  for `build_states`, plus 9 for `adversarial-git`. Across a whole run
  today, all status-log builders together make about 480 spawns: the full
  set 5 times, `build_or_adopt` once, the plain-git subset twice, cap
  twice, parity once and bookmark once.
- **The plain-git subset.** `build_plain_git_states` (`:40-80`, `pub`) is
  the first call inside `build_states` (`:302`), so it builds the same
  three states with the same code and layout. Its two consumers use only
  `clean-git`.
- **The private sets.** None of them shares a name with the full set:
  - cap (`:326-355`);
  - parity (`:367-407`), whose `parity-git` same-size rewrite is the one
    state that depends on timestamps;
  - bookmark (`:417-437`).
- **Absolute paths.**
  - The `git-ahead` and `git-behind` `remote.origin.url` values (`:104-117`).
  - Their reflogs.
  - `origin.git/config` → `seed`.
  - `adversarial-git` `core.hooksPath` → `<state>/hostile-hooks`
    (`:461-470`).
  - The manifest (`:530`). `adopt` does not rebase it (`:550`).

  Verified in the scratchpad with git 2.56 and jj 0.43:
  - git turns local clone sources, even relative ones, into absolute
    `remote.origin.url` values;
  - `-c core.logAllRefUpdates=false` suppresses the reflog entries;
  - `jj workspace add <abs>` writes a relative `.jj/repo` (`../../j/.jj/repo`);
  - nothing under `.jj/` contains the absolute path.

  Status and log never read remotes (`vcs-adapters/src/library/status_log.rs:229-289`).
  So rewriting the URL after cloning, for example
  `git remote set-url origin ../origin.git`, should leave the goldens
  unchanged. Confirm that against the goldens.
- **Timestamps.** Every expected signal in the full set comes from content,
  size, the index or file presence. Changed mtimes only force a rehash. The
  state most worth checking after a copy is `conflict-jj`, because jj must
  re-parse the materialised conflict markers.

### Consumers (mutation audit, as found)

| Consumer | File | Tests | Today | Effect on the tree | Recommended use |
|---|---|---|---|---|---|
| classify | `cli/vcs-adapters/tests/classify.rs:175-237` | 6 | `build_in` per test | read-only (`InProcessProbe` facts); `:237` asserts 34 fixtures; `unkeyed()` reads parent directories outside the manifest (`:17-19`) | adopt |
| queries | `cli/vcs-adapters/tests/queries.rs:401-589` | 5 of 6 (`:611` uses no fixture) | `build_in` per test | read-only; `:476-485` `set_current_dir` (process state only); `$B` is replaced by `matrix.base` (`:405,:411`) | adopt |
| matrix | `cli/vcs-test-support/tests/matrix.rs:21-141` | 6 | `matrix_root` + `build_or_adopt` | structural reads | adopt |
| scrub | `cli/vcs-adapters/tests/scrub.rs:87,126` | 2 of 3 (`:167` builds its own repo) | `build_in` per test | writes `matrix.base/poison.gitconfig` (`:96-97`, `:133-134`); the location is never asserted; the spawned `vcs-adapters-fixture` runs read-only queries; Hermetic is not applied | adopt, writing the poison config to the test's own temp directory |
| status_log_goldens | `cli/vcs-cli/tests/status_log_goldens.rs` | 4 full-set (`:68,:110,:139,:181`), 2 plain-git (`:227,:299`) | per-test build; Hermetic rooted in the same directory | the binary snapshots jj states in-process through jj-lib, taking the lock and possibly writing store objects (`vcs-adapters/src/library/snapshot.rs:134-197`); git is read-only | copy (partial copies allowed); `:68` iterates the whole map and must skip `adversarial-git`, which has no golden |
| conflicts | `cli/vcs-adapters/tests/conflicts.rs:33-60` | 1 | full `build_states`; uses only `conflict-git`/`conflict-jj` | oracle `git diff` (porcelain, may refresh the index); `jj resolve --list` snapshots; in-process jj-lib snapshot | copy of the two states, unless the hash guard shows a clean-working-copy jj-lib snapshot writes nothing and the oracles move to `git diff-files`/`ls-files -u` and `--ignore-working-copy` |
| status_log_states | `cli/vcs-test-support/tests/status_log_states.rs:12-26` | 1 | `states_root` + `build_or_adopt` | asserts the map holds 4 keys, including `adversarial-git` | adopt |
| cap, bookmark, parity | goldens `:158,:205`; `cli/vcs-cli/tests/status_log_parity.rs:126,156` | 4 | private builders | parity-jj is snapshotted | private build; must still log "private sets" events |

`fixtures::pure_jj` is also called directly in `repository_probe.rs`,
`user_name.rs`, `detect_goldens.rs` and `root_goldens.rs`. Each call builds
into the test's own temp directory, so these are not matrix consumers.
Nothing else calls `build_in`, `build_or_adopt` or the status-log builders.

### `test:unit:cli` today (`tasks/test/cli.py`)

- **How the task runs.**
  - `mise.toml:334-337` declares `run = "invoke test.cli.run"` and
    `depends = ["deps:install:rust-components"]`, with no `usage`.
  - `run` (`cli.py:45-75`) takes no parameters.
  - With coverage off it runs
    `cargo nextest run {_MANIFEST}` (`:75`).
  - With coverage on, `_run_instrumented` (`:86-92`) runs these steps in
    order:
    1. `llvm-cov clean --profraw-only`
    2. `llvm-cov nextest {_MANIFEST} --no-report`
    3. `llvm-cov report --json`
    4. `show-env`
    5. `cargo test --no-run --message-format=json`, run in the instrumented
       environment
    6. the sweep
    7. `_enforce_floor`
  - Each failure raises `Exit` straight away, so "cleanup pass or fail"
    needs a `try/finally` around the whole lane.
- **`_MANIFEST` is shared.** It is `--manifest-path cli/Cargo.toml
  --workspace --exclude accelerator-visualiser --all-features` (`:35-36`),
  and `tasks/test/integration.py:8` imports it for `tracker_contract`. Do
  not fold `--filter` into it.
- **The nextest configuration** (`cli/.config/nextest.toml:1-8`) has only
  `default-filter = 'not binary(=contract)'`, with no setup scripts or
  groups. `-E` intersects with the default filter.
  `tests/unit/tasks/test_nextest_filter.py:37-47` bans the text `--profile`
  and `--ignore-default-filter` anywhere in `cli.py`, comments included.
- **Task tests.** `tests/unit/tasks/test_rust.py:108-431`
  (`TestTestUnitCli`) pins the number and order of `context.run` calls
  (`:212`, `:244`, `:307`, `:370-384`). The new steps will turn these red,
  which is expected under TDD. The argv parsing pattern for new options is
  in `tests/unit/tasks/test_measure.py:3959-3977`.
- **CI.** The `test-unit` job (`.github/workflows/main.yml:20-66`) runs
  `mise run test:unit` on `ubuntu-latest` and `macos-latest` with coverage
  on. It installs `acl` on Linux. Test counts on CI are about 7,400 on
  Ubuntu and 7,401 on macOS, against 7,727 on engywook.
- **Measurement** (`tasks/shared/build_measurement/timeline.py`).
  - `_NEXTEST_SUMMARY` `\s*Summary \[\s*([\d.]+)s\]` is applied with
    `.match` to every line's last segment, and the matches are **summed**
    (`:26,:321-328`).
  - `_FINISHED_IN` is applied with `fullmatch`, and the **first** match
    becomes the task's duration (`:20,:231-269`).
  - The `exited with status N`, `ERROR task failed` and
    `sources up-to-date, skipping` markers matter too.
  - `Compiling … v` lines are counted as units (`:25`).
  - `--isolate` runs `mise run --skip-deps <task>` with no arguments
    (`settings.py:96-105`). The build step must therefore live inside the
    invoke task, not in a mise `depends` task, and the bare invocation must
    build both families.
- **Helpers worth reusing.**
  - `tasks/shared/signals.py`: `unwinding_signals` turns SIGINT, SIGTERM
    and SIGHUP into unwinding, so `finally` blocks run.
  - `tasks/shared/processes.py`: psutil-backed process checks.
  - `tasks/shared/files.py:4-11`: `atomic_write_text`.
  - The pattern is a pure domain module (as in `tasks/shared/coverage.py`
    and `instrumented_build.py`) plus a thin task body that turns failures
    into `Exit`.

### mise task arguments (verified empirically on mise 2026.10.8)

```toml
[tasks."test:unit:cli"]
usage = '''
flag "--filter <filter>" help="nextest filterset"
'''
run = 'invoke test.cli.run ${usage_filter:+--filter "$usage_filter"}'
```

The `run` line above is illustrative and was not tested.

- **What works.**
  - `--filter x` and `--filter=x` both set `usage_filter`; it is unset when
    the flag is omitted.
  - `mise run <task> --help` prints `Usage: <task> [--filter <filter>]`.
  - Undeclared words fail with `unexpected word: --bogus` and exit 1.
  - `mise run <task> -- --filter q` also parses.
- **What changes once `usage` is declared.**
  - Raw arguments are no longer appended to `run`.
  - `depends` tasks do not receive the flag.
  - `{{flag(name=…)}}` treats `--filter` as a boolean, so use the `usage`
    field instead.
- **`mise.toml` does not pin mise.** CI installs it through
  `jdx/mise-action`.
- **Measurement goldens are unaffected.**
  `tests/unit/tasks/fixtures/build-measurement/warm-no-op-tasks.golden.json`
  is frozen and is not compared with the live `mise.toml`.
- **Naming the invoke parameter.** Ruff `select = ["ALL"]` enables A002, so
  an invoke parameter named `filter` is flagged. Whether invoke maps a
  `filter_` parameter to `--filter` was not checked; check it before
  relying on it.

### Inventory for Requirement 5 (leaf `test:*` tasks)

These leaves are not exempt and run more than one test, so they need
`--filter`. All are declared in `mise.toml:308-540`.

| Task | Runner and entry point | Filter syntax | Malformed example |
|---|---|---|---|
| `test:unit:visualiser` | `cargo test` (libtest), run **twice** (`tasks/test/unit.py:10-27`) | libtest substring | none (any string is valid) |
| `test:unit:frontend` | Vitest 3.2.4 through `npm run test` (`unit.py:30-34`) | `-t` regex | `-t "("` (inferred) |
| `test:unit:tasks`, `test:unit:frontend-licenses`, `test:unit:vcs`, `test:integration:tasks` | bare `uv run pytest …` in mise.toml, no invoke task | `-k` | `-k "foo and"` (exit 4) |
| `test:unit:cli` | nextest 0.9.138 / llvm-cov 0.8.7 (`tasks/test/cli.py`) | filterset `-E` | `-E 'test(foo'` |
| `test:unit:design-automation` | `node --test` (`unit.py:115-161`) | `--test-name-pattern` regex | `"("` (inferred) |
| `test:integration:design-automation` (opt-in) | `node --test` (`integration.py:233-290`) | as above | as above |
| `test:integration:visualiser` | `cargo test --test '*'` (`integration.py:14-33`) | libtest substring | none |
| `test:integration:dev` | pytest with xdist `-n auto` (`integration.py:54-57`) | `-k` | `-k "foo and"` |
| `test:integration:entrypoint`, `:evals`, `:pup` (nightly), `:hooks`, `:research`, `:research-exhaustive` (opt-in) | pytest through invoke | `-k` | `-k "foo and"` |
| `test:integration:tracker-contract` (opt-in, needs credentials) | `cargo nextest run --profile contract` (`integration.py:94-101`) | `-E` (intersected with `binary(=contract)`) | `-E 'test(foo'` |
| `test:e2e:visualiser` | Playwright 1.59.1 through `npm run test:e2e` (`tasks/test/e2e.py:22-33`) | `--grep` regex | `--grep "("` (inferred) |
| `test:e2e:visualiser:docker`, `…:docker:update` | Playwright in Docker; the command is built in `docker_visual_command` (`e2e.py:49-97`) and re-parsed by `bash -c` | `--grep` | as above; the value needs `shlex.quote` |

Points the planner must settle for this inventory:

- **Two leaves run a single test but are not exempt.**
  `test:integration:visualiser-os-watcher` (`--exact`) and
  `test:integration:arxiv-queue-stress` (one test, `-E
  'binary(=arxiv_queue_stress)'`) are absent from the exempt list. The bare
  `test` roll-up is not listed either.
- **libtest has no malformed expression.** Requirement 5 allows that ("where
  its syntax has one").
- **The design-automation lanes enforce pass-count floors.** They require
  ≥105 passes and zero skips (`unit.py:44-48,147-161`), and ≥37 passes
  (`integration.py:274-290`). A name filter would trip them, the same
  problem as the coverage floor.
- **Six pins in `tests/unit/tasks/` are affected.** They assert exact
  command strings or tokens:
  - `test_dev_integration_task.py` (`-n auto`);
  - `test_hooks_task_environment.py`;
  - `test_e2e_docker.py`;
  - `test_mise.py:386-391`, which requires every `test:integration:*` task
    to sit in the roll-up or in an allow-list;
  - `test_rust.py`;
  - `test_nextest_filter.py`.
- **Five pytest leaves have no invoke task.** `test:unit:tasks`,
  `test:unit:frontend-licenses`, `test:unit:vcs` and
  `test:integration:tasks` are bare `uv run pytest` in mise.toml. Either
  they declare `usage` in mise alone, or each gets an invoke task, which is
  what Requirement 4's "declared by the invoke task" implies for
  consistency.

### Copy mechanics (verified on APFS; Linux inferred)

- **jj-lib 0.43** (`local_working_copy.rs`).
  - `FileState::is_clean` compares file type, `mtime` (ms) and `size`
    (`:293-310`).
  - Clean also requires `mtime < tree_state.own_mtime`, the mtime of
    `.jj/working_copy/tree_state` (`:1128-1135`, `:1820-1823`).
  - Snapshotting creates `.jj/working_copy/working_copy.lock`, then removes
    it on drop.
  - Plain `jj status` also creates and removes
    `.jj/repo/git_import_export.lock`, which changes directory mtimes.
  - On a read-only tree, plain `jj status` fails.
- **git 2.56.**
  - With `core.checkStat=default`, the index check compares ctime, inode,
    uid, gid and size.
  - So `git status` in a copy refreshes and **rewrites the index**, while
    its porcelain output stays the same.
  - `--no-optional-locks` or `GIT_OPTIONAL_LOCKS=0` avoids the write.
- **gix 0.85** (`gix-index-0.53 src/entry/stat.rs:38-119`).
  - Defaults: `trust_ctime=true`, `check_stat=true`, `use_nsec=false`.
  - "ctime" is filled from `metadata.created()` (birth time), which
    clonefile keeps.
  - gix writes nothing unless `Outcome::write_changes()` is called, and the
    cli never calls it.
- **`cp -c -R -p` on APFS** keeps nanosecond mtimes, including
  `tree_state`'s. `git status --porcelain` and both forms of `jj status`
  matched the source.
- **Rust `std::fs::copy`.**
  - On Apple it tries `fclonefileat`, then falls back to `fcopyfile`.
  - On Linux it uses `copy_file_range` and keeps only permissions.
- **Linux.** ext4 has no reflink, so GNU `cp -a --reflink=auto` falls back
  to a full copy that keeps timestamps. The status-log states are small, so
  a full copy is cheap.

### Fixture-builder entry point: constraints found

- **The precedent prints a `Summary [` line.** The retired zero-spawn task
  built the matrix by running one test through nextest with the root
  variable set: `cargo nextest run -p vcs-test-support -E 'binary(matrix)
  and test(every_recorded_fixture_key_is_built)'`. That was
  `_build_fixture_matrix` in `tasks/test/integration.py`, before commit
  `ec601453daea`. Done today, it would print a second `Summary [` line,
  which `measure:build` adds into the nextest phase.
- **A separate `cargo build -p …` changes the feature set.** It differs from
  `--workspace --all-features`, so it may add compile units or
  configurations, which counts against 0366.
- **Candidate forms:**
  - an `[[example]]` or test target in `vcs-test-support`, built by the same
    workspace test build;
  - a test binary run directly with libtest arguments, located through
    `cargo nextest list --message-format json`;
  - a new `[[bin]]`. This goes against `lib.rs:4-6`'s design note and
    brings in the `tasks/README.md` checklists.
- **Coverage on.** Whatever runs under coverage must use the instrumented
  target directory (`cli/target/llvm-cov-target`, through `show-env` plus
  `CARGO_TARGET_DIR`; see `cli.py:166-191`). Otherwise it triggers a plain
  rebuild.

## Code References

- `cli/vcs-test-support/src/fixtures.rs:25,47-57,109-160,167-186,615-652,660-683` - matrix adopt path, builder, absolute-path writers
- `cli/vcs-test-support/src/hermetic.rs:19,46-68,137-168,176-268,280-297,334-347` - Hermetic layout, env, spawns, ancestor guard, git version check
- `cli/vcs-test-support/src/status_log.rs:24-27,40-80,104-117,296-315,326-437,449-485,504-579` - status-log builders, adversarial state, adopt path, `states_root`
- `cli/vcs-test-support/src/lib.rs:4-6` - why the crate has no binary or features
- `cli/vcs-adapters/src/library/snapshot.rs:19-22,134-197` - in-process jj snapshot (lock plus store writes)
- `cli/vcs-adapters/src/library/status_log.rs:229-305` - git status/log (read-only), jj status via snapshot
- `cli/vcs-adapters/tests/{classify,queries,scrub,conflicts}.rs` - matrix and status-log consumers
- `cli/vcs-cli/tests/status_log_goldens.rs`, `cli/vcs-cli/tests/status_log_parity.rs` - goldens and parity consumers
- `cli/vcs-test-support/tests/{matrix,status_log_states}.rs` - self-tests already on the adopt path
- `cli/.config/nextest.toml:1-8` - default filters only
- `tasks/test/cli.py:34-191` - `test:unit:cli` lanes, floor, sweep
- `tasks/shared/build_measurement/timeline.py:18-328`, `settings.py:65-105` - measurement parsing and isolation
- `tests/unit/tasks/test_rust.py:108-431`, `test_nextest_filter.py:37-47`, `test_measure.py:3959-3977` - task tests and guards
- `mise.toml:308-540` - `test:*` task declarations
- `.github/workflows/main.yml:20-66` - CI `test-unit` job
- `CLAUDE.md:47-50` - "Running a single test" guidance to be rewritten

## Architecture Insights

- **Both families already follow one pattern:** "build, persist a manifest of
  absolute paths, adopt by reading the manifest". It was designed so that
  only the builder needs `git` and `jj`. The explicit hand-over that 0365
  requires fits it directly.
  - The adopt variables' natural successor is a renamed variable that the
    build step sets, which a test reads only to decide between adopting and
    building.
  - The matrix's `matrix_root`/`build_or_adopt` pairing already expresses
    "handed a build or not".
- **One sanctioned way to get a start directory.** The 0185 validation
  recommended this after tests silently built fixtures inline. The same
  helper is the natural place to emit fixture-log events (adoption, copy,
  private build) and to run the copy path scan.
- **Hermetic currently lives inside each fixture tree.** Separating "where
  the fixture is" from "where this test's hermetic root is" is the core
  refactor. `Hermetic::rooted_at` already takes any base, so tests can root
  it in their own temp directory.
- **Writes hide in places that look like reads.** The jj-lib snapshot in
  "read" paths and porcelain `git diff`/`status` index refreshes are
  examples. The permanent before-and-after hash in Requirement 3 is the
  right guard. It hashes contents and modes, not timestamps, so the
  transient lock files jj creates and removes do not disturb it.

## Historical Context

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
  - Proposal B1 (around line 464) suggested a nextest setup script. 0365
    chose an invoke build step instead.
  - The phase is bound by spawn latency and long poles: about 50% CPU, and
    nextest's process model is not the overhead.
  - One matrix build takes 3.3 s wall when run alone and 35–47 s under
    load.
- `meta/reviews/work/0365-build-the-vcs-test-fixtures-once-per-test-run-review-1.md`
  explains the rationale for:
  - the explicit hand-over;
  - the fixture log, which exists because nextest hides output from
    passing tests;
  - the test-only failure and hold switches;
  - scoping the `ZERO_SPAWN` search to `cli/`, `tasks/`, `mise.toml` and
    `.github/`;
  - the permanent guards.
- `meta/plans/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`
  - It gives the current shape of the coverage lane.
  - It warns that `show-env` does not set `CARGO_TARGET_DIR`.
  - It records the `_MANIFEST` sharing gotcha.
- `meta/plans/2026-08-31-0198-vcs-agnostic-status-log-renderer.md`
  - It sets out why the status-log adopt path exists.
  - It describes the adversarial state.
  - It explains why the cap, bookmark and parity sets were kept private.
  - It notes that `conflict-git` needs `git_allow_failure`.
- `meta/prs/61-description.md` and
  `meta/validations/2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs-validation.md`
  - They describe the zero-spawn hand-over through
    `ACCELERATOR_ZERO_SPAWN_MATRIX`.
  - They record that inline fixture building defeated it.
  - They recommend a single start-directory helper and a "fail closed"
    hand-over.
- `meta/plans/2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic.md`
  is a pytest analogue. Two practices transfer: run each consumer alone,
  and record pass counts before and after.
- `meta/work/0391-shorten-the-guarded-resume-migration-tests.md`
  - It lists "build once, give each test a private copy" as a candidate.
    Design the copy helper so it can be reused.
  - 0365's after-record must keep per-test `PASS [ Ns ]` lines usable.

## Related Research

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- `meta/research/codebase/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`
- `meta/research/codebase/2026-08-31-0198-vcs-agnostic-status-log-renderer.md`
- `meta/research/codebase/2026-08-10-0185-converge-corpus-adapters-library-backed-vcs.md`

## Answers to the Work Item's Open Questions

1. **Do the fixtures hold absolute paths a copy would carry back?**
   - **Matrix: yes, extensively.** Rewriting them would change what the
     degenerate and hostile shapes test. Since no matrix consumer needs a
     copy, **adopt everything and do not copy the matrix.**
   - **Status-log set: only a few.**
     - `git-ahead` and `git-behind` remote URL and reflogs;
     - `origin.git` → `seed`;
     - `adversarial-git` `core.hooksPath`;
     - the manifest.

     The jj workspace pointer and `git_target` are relative. The fix is in
     the builder: `git remote set-url` to a relative path after cloning,
     `core.logAllRefUpdates=false` while cloning, and a relative
     `core.hooksPath`. Copies leave the manifest out, or rewrite it. Each
     of these needs a check that the goldens stay byte-identical.
2. **Does the status-log set contain the plain-git subset's states?** Yes,
   produced by the identical function. Both subset tests use only
   `clean-git`.
3. **Adopt or copy, for each conditional row:**
   - `scrub`: adopt (move `poison.gitconfig` to the test's own temp
     directory).
   - `status_log_states`: adopt (adversarial is already part of the shared
     build).
   - `conflicts`: copy (`conflict-git` and `conflict-jj`), because its
     in-process jj side snapshots as well as the oracle. It can adopt only
     if the hash guard shows a clean jj-lib snapshot writes nothing and
     both oracles change to non-writing forms.
4. **How copies are made, where the trees live, and where the log goes:**
   - **Copies:** clonefile on macOS (`cp -c -R -p` or `std::fs::copy`) and
     a full copy on Linux, keeping mtimes (`cp -a`, or `filetime` in Rust),
     including `tree_state`'s.
   - **Trees:** under the system temp directory, never in the repo, because
     of the ancestor guard and `no-repo`. Use one prefix, with a liveness
     marker per run for the interrupted-run sweep, such as a held `flock`
     or a PID plus start time checked through psutil.
   - **Log:** for the planner to choose. It must survive cleanup, for
     example under `cli/target/` or an `.accelerator-measure`-style
     directory. Concurrent test processes need append-safe writes, either
     one line per `O_APPEND` write or a file per test.
5. **Rename or remove the adopt variables?** The hand-over needs a
   variable, so rename them to name the shared build rather than zero-spawn.
   `matrix.rs` and `status_log_states.rs` are their only readers today.
6. **What form does the entry point take?** Open. The constraints are
   above: avoid nextest as the runner, because of the `Summary [` line;
   reuse the workspace test build's profile and features; under coverage,
   run in the instrumented target directory.
7. **Which tasks are in scope for Requirement 5?** See the inventory above.

## Open Questions

- **What should a filtered `test:unit:cli` run do about the coverage
  floor?** The design-automation pass-count floors raise the same question.
  This is not addressed by the work item, and acceptance criteria such as
  `--filter 'test(classify)'` "pass" depend on it.
- **The "[each family]" copy criteria cannot be met for the matrix as
  written.** These are copy fidelity and the planted-path scan. If no
  matrix consumer copies, those criteria are vacuous for the matrix.
  Exercising them on a matrix copy would always fail the scan. The work
  item may need amending to say they apply to families that have copies.
- **Does a jj-lib snapshot of a clean working copy write store objects?**
  This decides whether `conflicts` could adopt. The before-and-after hash
  would show it empirically.
- **Do the status-log builder changes for relative paths leave all 38
  goldens byte-identical?** This needs checking.
- **Does invoke map a `filter_` parameter to `--filter`?** If not, which
  name avoids ruff A002?
- **Does `measure:build`'s mise task loading cope with tasks that declare
  `usage`?** It parses `mise tasks --json` (`ports.mise_tasks`).
