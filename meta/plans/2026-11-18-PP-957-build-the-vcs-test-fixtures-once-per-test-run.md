---
type: "plan"
id: "2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run"
title: "Build the VCS Test Fixtures Once per Test Run Implementation Plan"
date: "2026-11-18T19:51:03+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:PP-957"
parent: "work-item:PP-957"
derived_from: ["codebase-research:2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run"]
relates_to: ["codebase-research:2026-10-28-full-build-duration-contention-and-slowness", "work-item:PP-953", "work-item:PP-956", "work-item:PP-958", "work-item:PP-943"]
tags: ["build-system", "performance", "testing", "rust", "vcs", "nextest", "mise"]
revision: "5acdef06a5f1d5abb03423a3667dd1bb5cb1a0b6"
repository: "accelerator"
last_updated: "2026-11-19T09:21:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Build the VCS Test Fixtures Once per Test Run Implementation Plan

## Overview

`test:unit:cli` builds the VCS fixture matrix 19 times and the status-log set
6 or more times per run, once in each test that needs one. This plan:

- builds each family once, in a step of the invoke task that runs before
  nextest's test phase;
- hands the builds to the tests, which read them in place or work in checked
  copies;
- records every build, adoption, copy and private build in a per-run fixture
  log;
- gives every multi-test `test:*` task a `--filter` option.

The work lands as the work item's three increments, in nine phases. Each
phase can be merged on its own:

| Increment | Phases |
|---|---|
| matrix | 1. Workbench and fixture log · 2. The shared build step · 3. `--filter` on `test:unit:cli` |
| status-log | 4. Status-log builders write no absolute paths · 5. Status-log sharing · 6. Fixtures closure (measurement) |
| filters | 7. pytest lanes · 8. cargo and node lanes · 9. Guidance |

## Current State Analysis

The work item describes the cost, and the research document
(`meta/research/codebase/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`)
describes the code. These are the facts this plan depends on.

- **Each test builds its own fixtures.**
  - Matrix consumers call `Matrix::build_in(tempdir)` directly:
    - `classify.rs:177-230`, six tests;
    - `queries.rs:403-589`, five tests;
    - `scrub.rs:89,130`, two tests.
  - The matrix self-test goes through `matrix_root()` and `build_or_adopt`
    (`vcs-test-support/tests/matrix.rs:14-18`).
  - Status-log consumers call `status_log::build_states`
    (`status_log_goldens.rs:74,116,144,186`, `conflicts.rs:38`) or
    `build_plain_git_states` (`status_log_goldens.rs:234,306`).
  - Their self-test goes through `states_root()` and
    `States::build_or_adopt` (`status_log_states.rs:13-14`).
- **The adopt paths are dead.** Nothing sets `ACCELERATOR_ZERO_SPAWN_MATRIX`
  (`fixtures.rs:25`) or `ACCELERATOR_ZERO_SPAWN_STATUS_LOG`
  (`status_log.rs:24`). Their only readers are `matrix_root` and
  `states_root`.
- **`Hermetic::rooted_at(base)` writes into `base`** (`hermetic.rs:46-68`).
  Every builder roots it at the tree it builds, so the isolated `HOME`, XDG
  directory and jj config sit inside the fixture tree.
- **`test:unit:cli` runs one command when coverage is off**
  (`tasks/test/cli.py:75`). With coverage on, it runs a fixed sequence
  (`cli.py:86-92`):
  1. clean the profile data;
  2. `llvm-cov nextest --no-report`;
  3. report the totals;
  4. `show-env`;
  5. the unit listing (`cargo test --no-run --message-format=json` in the
     instrumented environment);
  6. the sweep;
  7. the floor.

  The task takes no options. `_MANIFEST` is shared with
  `tracker_contract` (`integration.py:8,98`).
- **The task tests pin that sequence.** `TestTestUnitCli`
  (`tests/unit/tasks/test_rust.py:108-431`) fakes `context.run` with
  prefix-matched responses (`_responding`, `:143-152`). It unpacks exactly
  five commands (`:212`), and expects one command when coverage is off
  (`:307`). `test_cli_permission_skip.py:8-19` runs the task with
  `MockContext(run=Result(exited=0))`.
- **No mise task declares options.** Four leaf test tasks run `uv run
  pytest …` straight from `mise.toml` (`:319-342`).

### Key Discoveries

- **Every matrix consumer can adopt; none needs a copy.**
  - The matrix embeds absolute paths throughout, written both by the crate
    and by git:
    - linked-worktree gitdirs (`fixtures.rs:228-231,341-357,434-448,627`);
    - submodule URLs (`:568-572`);
    - D1, D2, D3 and CG pointers (`:488,498-501,514,633-636`);
    - the HOSTILE config and include (`:660-683`);
    - the manifest.
  - All its consumers are read-only, except that `scrub` writes
    `poison.gitconfig` into `matrix.base` (`scrub.rs:96-97,133-134`).
    Nothing asserts where that file lives.
- **The status-log set's absolute paths are few, and none reaches a
  golden.** They are:
  - the `remote.origin.url` of `git-ahead` and `git-behind`, and their
    "clone: from" reflogs (`status_log.rs:104-117`);
  - `origin.git`'s URL back to `seed` (`:94-103`);
  - `adversarial-git`'s `core.hooksPath` (`:461-470`);
  - the manifest (`:527-535`).

  jj 0.43 writes the secondary workspace's `.jj/repo` as a relative path.
  None of the 38 goldens contains a path or a URL.
- **The writers among status-log consumers.**
  - The in-process jj snapshot writes tree and blob objects into the store
    (`vcs-adapters/src/library/snapshot.rs:19-22,162-168`). The goldens tests
    (through the binary) and `conflicts` (through `InProcessProbe`) both
    snapshot, so both copy.
  - The `status_log_states` self-test only reads the map, and the shared
    build already contains `adversarial-git` (`status_log.rs:516-520`). So it
    adopts.
- **The fixture builder can be a `[[bin]]` under `tests/fixtures/`.**
  `vcs-adapters` already ships fixture binaries this way
  (`cli/vcs-adapters/Cargo.toml:15-29`). pup's target scan skips
  `/tests/fixtures/` (`tests/integration/pup/test_import_rule.py:2254-2255`),
  and the store-duplication lint scans only `src/`
  (`tasks/lint/store_duplication.py:58-61`). nextest's build compiles bins
  (checked in a scratch crate). `cargo test {_MANIFEST} --no-run
  --message-format=json` with the same arguments then compiles nothing and
  reports each bin's `executable`. So the builder adds no compile
  configuration (0366).
- **A test can identify itself.** nextest 0.9.138 sets `NEXTEST_BINARY_ID`
  and `NEXTEST_TEST_NAME` for every test (checked).
- **nextest's test list is usable for selection.**
  - `cargo nextest list -E 'test(foo'` fails with "failed to parse
    filterset" before compiling anything.
  - `cargo nextest list --message-format json` gives `rust-suites` keyed by
    binary id. Each test case carries `filter-match.status` of `matches` or
    `mismatch`.
- **mise 2026.10.8 can declare and forward an option.**
  - `usage = 'flag "--filter <filter>" help="…"'` with
    `run = 'invoke … ${usage_filter:+--filter "$usage_filter"}'` forwards
    values containing spaces, `&` and `'` intact.
  - The flag appears in `mise run <task> --help`, and an undeclared word
    exits 1.
  - `mise tasks --json` already carries a `usage` key, and `measure:build`
    reads only `depends` and `run` (`task_graph.py:47-67`).
- **invoke can name the option `--filter`.** invoke 3.0.3 maps a `filter_`
  parameter to `--filter` (checked). That avoids ruff A002, which is on
  (`select = ["ALL"]`, no ignore).
- **The shared trees cannot live in the repo.** `assert_no_repository_ancestor`
  (`hermetic.rs:280-297`) rejects any base inside a checkout. They go under
  the system temp directory.
- **`measure:build` reads two kinds of line** from the task's output
  (`tasks/shared/build_measurement/timeline.py:20,26,231-269,321-328`):
  - it sums every line that `.match`es `\s*Summary \[\s*([\d.]+)s\]`;
  - it takes the first line that fully matches `Finished in …` as the
    duration.

  The fixture summary must produce neither.
- **The builder code counts towards the coverage floor.** No
  `ignore-filename-regex` is set (`cli.py:125`). Once tests stop building
  fixtures, the builder code runs only in the `shared-fixtures` bin. The bin
  must therefore run in the instrumented environment, so that its profile
  data is merged into the report.
- **Two crates need adding as dependencies.** `walkdir` 2.5.0 and
  `filetime` 0.2.29 are already in `cli/Cargo.lock`, but have no workspace
  entry. cargo-deny bans wildcards (`cli/deny.toml:113-116`), so they need
  exact versions. `serde_json` and `sha2` are workspace dependencies
  (`cli/Cargo.toml:100,108`).

## Desired End State

**A bare `mise run test:unit:cli`, with coverage on or off:**
1. sweeps away the shared-fixture runs of any earlier, killed run;
2. compiles the workspace tests (the same build nextest would make);
3. runs the `shared-fixtures` bin once, building the matrix and the
   status-log set concurrently under
   `<temp>/accelerator-shared-fixtures-<uid>/<run-id>/` (`<temp>` is the resolved
   system temp directory);
4. hashes each tree;
5. runs nextest with the trees handed over through
   `ACCELERATOR_SHARED_MATRIX` and `ACCELERATOR_SHARED_STATUS_LOG`, and the
   fixture log through `ACCELERATOR_FIXTURE_LOG`.

**Each test that needs a fixture:**
- opens a `Workbench`, its own directory and hermetic root;
- adopts the matrix, adopts the status-log set, or takes a scanned copy of
  part of it;
- records what it did in
  `cli/target/vcs-fixture-logs/<run-id>.jsonl`.

**When the run ends, pass or fail, the task:**
- hashes each tree again;
- removes the run's directory;
- prints the fixture summary;
- fails if a tree changed, a copy named its shared tree, a test built a
  shared family privately, or the log disagrees with the consumer list or
  lacks a built family's shared build.

**A filtered run.** `mise run test:unit:cli --filter '<filterset>'` builds
only the families the selected tests consume. Under coverage, it skips the
report, sweep and floor.

**Running nextest directly.** `cargo nextest run` without the task hands
nothing over. Every test then builds privately in a system-temp workbench and
writes no log.

**Every other multi-test leaf.** Each one that is not exempt takes `--filter`
in its runner's own syntax. `CLAUDE.md` and `tasks/README.md` point readers at
these options.

**How to verify:**
- every acceptance criterion in the work item (the manual checks in
  Phases 3, 5, 6 and 9);
- `mise run` green;
- CI's `test-unit` legs green, each printing a fixture summary that shows
  each family built once.

## What We're NOT Doing

- **Copying the matrix.** Every matrix consumer adopts it. The copy helper,
  the path scan and the copy-fidelity check exist for the status-log set
  only. The work item is amended to match (Phase 1).
- **Sharing the private sets** (cap, bookmark, parity). They stay private
  builds, now logged under "private sets".
- **Overlapping the fixture build with compilation.** The builder is compiled
  by the same build as the tests, so it can't run until that build ends.
  Overlapping them would mean a separate build configuration, which counts
  against 0366. The work item allows overlap but doesn't require it.
- **Using nextest setup scripts,** or building fixtures from inside a test.
- **Enforcing against plain `cargo test`.** It is ruled out by guidance only.
- **`--filter` on the exempt guards, the single-test lanes or the
  roll-ups.**
  - The exempt guards are `release-guard`, `deny`, `fixture-size`,
    `measure`, `skill-invocation` and `conformance`.
  - The single-test lanes are `test:integration:visualiser-os-watcher` and
    `test:integration:arxiv-queue-stress`.
  - The roll-ups are `test:unit`, `test:integration`, `test:e2e` and `test`.
- **Changing what any fixture state contains, or any golden.**
- **Running the rest of the suite when the shared build fails.** A failing
  builder stops `test:unit:cli` before nextest runs, so none of the lane's
  tests report. Today a broken builder fails only the tests that build that
  family. This is a deliberate trade-off: fixture builds are deterministic,
  the builder's error names the family and its cause, and running nextest
  without the failed family's consumers would add a third flow and a
  partial coverage report. To see the rest of the suite while diagnosing,
  run `cargo nextest run` directly (each test builds privately), or use
  `--filter` with a filterset that selects no consumer.

## Implementation Approach

The work runs bottom-up. Each step is driven by a failing test first.

1. **The Rust side.** Introduce the `Workbench` and the hand-over in
   `vcs-test-support`, behind variables that nothing sets yet. Behaviour is
   unchanged, so the phase merges safely.
2. **The Python side.** A `tasks/shared/shared_fixtures/` package, split
   along its domain seams, handles the parts the task needs:
   - `families.py`: the shared families and their hand-over variables;
   - `runs.py`: run directories, liveness locks, the sweep and the hold
     point;
   - `log.py`: reading the fixture log, summarising it, finding problems in
     it, and pruning old logs;
   - `consumers.py`: which families a selection of tests consumes;
   - `session.py`: a run's lifecycle, from opening the root to cleanup.

   Tree digests join `compute_sha256` in `tasks/shared/hashing.py`. Finding
   the builder is a new `UnitListing.executable(target)` in
   `tasks/shared/instrumented_build.py`, which already parses the same cargo
   messages. The run lock builds on the flock primitive in
   `tasks/shared/locking.py`. `tasks/test/cli.py` stays a thin sequence of
   `context.run` calls around the package, and hands the session its
   builder as a callable.
3. **The status-log set.** Change its builders first, so that the set is
   path-free while its goldens stay byte-identical. Then add copies and
   sharing.
4. **The filters.** Give the other test lanes `--filter` through one small
   value object, `Selection`. It renders the user's expression for each
   runner, so quoting is solved once.

### Names used throughout

| Name | Meaning |
|---|---|
| `ACCELERATOR_SHARED_MATRIX` | path of the shared matrix tree handed to tests (replaces `ACCELERATOR_ZERO_SPAWN_MATRIX`) |
| `ACCELERATOR_SHARED_STATUS_LOG` | path of the shared status-log tree (replaces `ACCELERATOR_ZERO_SPAWN_STATUS_LOG`) |
| `ACCELERATOR_SHARED_WORKBENCHES` | directory under which tests open workbenches during a build-step run |
| `ACCELERATOR_FIXTURE_LOG` | the fixture log a test appends events to; unset means log nothing |
| `ACCELERATOR_SHARED_FIXTURES_FAIL` | test-only: the family whose shared build the builder fails (`matrix` or `status-log`); any other value is refused |
| `ACCELERATOR_SHARED_FIXTURES_HOLD` | test-only: a directory; after the build step the task writes `held` there and waits for `release` |
| `<temp>/accelerator-shared-fixtures-<uid>/` | the one location for shared trees, copies and workbenches: `<run-id>.lock` plus `<run-id>/{matrix,status-log,workbenches}/`. One root per user, mode `0700`. `<temp>` is `Path(tempfile.gettempdir()).resolve()`, so on macOS it is the `/private/var/…` spelling and on Linux usually `/tmp`; the summary prints it |
| `cli/target/vcs-fixture-logs/<run-id>.jsonl` | the fixture log, kept after cleanup; the newest 20 logs of finished runs are kept |

---

## Phase 1: Workbench and fixture log

### Overview

This phase is Rust only. It adds:
- the `Workbench`;
- the `Handover`, which describes what the build step hands a test;
- the `FixtureLog`;
- the `SharedFamily` abstraction.

It also moves the 19 matrix consumers onto these. Nothing sets the variables
yet, so every consumer still builds privately, as it does today.

### Changes Required

#### 0. Amend the work item

**File**: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`

These changes record the decision that no matrix copy is made, and the one
test this plan adds to a consumer:
- **Requirement 15.** Move "the hash and path guards" so that the matrix
  increment carries the hash guard, and the path scan and copy helper move
  to the status-log increment.
- **The copy acceptance criteria.** Note on the planted-path and
  copy-fidelity criteria that the matrix meets them by making no copies.
- **The audit table.** Fill in the outcomes:

| Consumer | Uses the shared build by |
|---|---|
| `scrub` | adopting, with `poison.gitconfig` written to the test's own workbench |
| `conflicts` | a copy of `conflict-git` and `conflict-jj` (the in-process jj snapshot writes store objects) |
| `status_log_states` | adopting (`adversarial-git` is already part of the shared build) |
| `status_log_goldens` | a copy (full set, or the plain-git part for the two subset tests) |

- **The test-preservation criterion.** Note that `status_log_states` gains one
  test, `no_state_names_the_directory_it_was_built_in` (Phase 4), which
  guards the path-freedom the copies depend on. Its other test keeps its name
  and assertions.

#### 1. The fixture log (tests first)

**File**: `cli/vcs-test-support/tests/fixture_log.rs` (new)

- **Recording with a location.** A `FixtureLog::at(path)` appends one JSON
  object per `record`, one line each, kebab-case. Two `record` calls give
  two lines.
- **Recording without a location.** `FixtureLog::disabled().record(…)`
  creates no file.
- **Concurrent writers.** Each event goes out in a single `write_all` on a
  file opened with `append(true)`. A test with 8 threads × 50 records
  gets 400 well-formed lines.
- **Naming the test.** `TestName::from_variables` takes `NEXTEST_BINARY_ID`
  and `NEXTEST_TEST_NAME` and gives `vcs-adapters::classify
  every_fixture_classifies_as_expected`. With neither set, it falls back to
  the current thread's name.
- **The log format.** One event of each kind serialises to exactly the
  matching lines in `cli/vcs-test-support/contract/fixture-log.jsonl`,
  with kebab-case field names (`hermetic-root`). The test ignores the kinds
  the task writes (`tree-digest`, `tree-changes`). The Python log tests read
  that same file, so neither side can rename a field or variant without the
  other's tests failing.

- **The variable names.** `Family::handover_variable` for each family, and
  the workbenches and log variable constants, equal their entries in
  `cli/vcs-test-support/contract/variables.json`.

**File**: `cli/vcs-test-support/contract/fixture-log.jsonl` (new):
one line per event kind, grown as kinds are added. The two contract files
sit beside the crate that writes the log, and both test suites name them
through one constant each.

**File**: `cli/vcs-test-support/contract/variables.json` (new):
each family's hand-over variable, the workbenches variable and the log
variable, grown as families are added.

**Files**: `cli/vcs-test-support/src/family.rs` (new), holding `Family`,
`Fixtures` and every inherent method of `Family`, including
`handover_variable`; and `cli/vcs-test-support/src/fixture_log.rs` (new),
holding the rest.

```rust
pub const FIXTURE_LOG_VARIABLE: &str = "ACCELERATOR_FIXTURE_LOG";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Matrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixtures {
    Shared(Family),
    PrivateSets,
}

#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case", rename_all_fields = "kebab-case")]
pub enum Event<'a> {
    SharedBuild { family: Family, tree: &'a Path, git: &'a str, jj: &'a str },
    Adoption { family: Family, test: &'a TestName, hermetic_root: &'a Path },
    PrivateBuild { fixtures: Fixtures, test: &'a TestName, hermetic_root: &'a Path },
}

#[derive(Debug, Clone, Default)]
pub struct FixtureLog(Option<PathBuf>);

impl FixtureLog {
    pub fn at(path: impl Into<PathBuf>) -> Self;
    pub fn disabled() -> Self;
    pub fn from_environment() -> Self;
    pub fn from_value(value: Option<String>) -> Self;
    pub fn location(&self) -> Option<&Path>;
    pub fn record(&self, event: &Event<'_>) -> Result<(), Error>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct TestName(String);

impl TestName {
    pub fn current() -> Self;
    pub fn from_variables(binary: Option<&str>, test: Option<&str>) -> Self;
}

impl Default for TestName {
    fn default() -> Self;
}
```

`TestName::default()` is `TestName::current()`, so a `Handover` built from
its default still logs the running test's name. `Family` has one variant
until Phase 5 adds `StatusLog` together with its `SharedFamily` impl, its
hand-over variable and its Python counterpart. `Family` names only what can
be shared, so a hand-over variable or a forced failure can never name the
private sets. `Fixtures` says which fixtures a
private build made. It serialises as the family's own name, or as
`private-sets`.

**File**: `cli/vcs-test-support/Cargo.toml`
**Changes**: add `serde_json = { workspace = true }`, and update
`cli/Cargo.lock` so that `--locked` clippy passes.

#### 2. The hand-over and the workbench (tests first)

**File**: `cli/vcs-test-support/tests/workbench.rs` (new)

Every test here uses a cheap `SharedFamily` defined in the test: a directory
holding one marker file, built by writing that file. So none of them runs git
or jj. `Family` has no test-only variant, so the cheap family claims
`Family::Matrix`. Each test therefore builds an explicit `Handover` with a
test-local `FixtureLog::at(...)`, never `Workbench::for_this_test()`, so
nothing it records reaches a run's log.

- **The workbench directory.** It is created under the hand-over's
  workbenches directory when one is named, and otherwise under the system
  temp directory. Either way it is canonical, and it is not inside any
  handed tree.
- **The hermetic root.** `hermetic()` is rooted at the workbench: its
  `hermetic-home` lies inside `path()`.
- **Two workbenches have distinct hermetic roots.**
- **Building privately.** With no tree handed for the family, `shared()`
  builds privately in the workbench, with the workbench's own `Hermetic`,
  and records a `private-build` event. The event carries the family, the
  test name and the hermetic root.
- **Adopting.** With a tree handed, `shared()` adopts that tree, builds
  nothing, and records an `adoption` event.
- **A broken hand-over fails closed.** If the handed tree is missing, or
  the family cannot adopt it, `shared()` fails with a message naming the
  variable and the path. It does not fall back to a private build.
- **One spelling of each path.** `Handover::from_variables(lookup)`
  canonicalises each handed path, so a tree handed as a symlinked spelling
  (a directory created in the test, standing in for macOS's `/var` →
  `/private/var`) is adopted, and logged, under its canonical path. A
  handed path that does not exist is kept as given, so `shared()` fails
  naming it. The test passes a map as `lookup`, with a test-local log, so it
  never reads the process environment.
- **An empty value is unset.** A hand-over, workbenches or log variable set
  to the empty string is treated as unset.
- **`from_environment` reads the process.** One test checks that
  `Handover::from_environment()` is `from_variables` over `std::env::var`,
  by setting `ACCELERATOR_SHARED_WORKBENCHES` in its own process. A run
  does set that variable, but nextest gives each test its own process, so
  the override reaches no other test.
- **One fixture per workbench.** A workbench hands out at most one fixture.
  A second `shared()` call on the same workbench fails, naming what it holds
  and what was asked for: `this workbench already holds the matrix
  (adopted); open another workbench for the matrix`. So two fixtures never
  land in the same directory, whether the run hands trees over or not.
  Phase 5 extends the rule to `writable()` and `private_states()` with
  tests of its own.
- **Cleanup.** Dropping the workbench removes its directory.

**File**: `cli/vcs-test-support/src/family.rs`
**Changes**: Add `Family::handover_variable(self) -> &'static str`,
`Family::ALL` (which `Handover::from_variables` iterates), and `Display`
for `Family` and `Fixtures` (`the matrix`, `the private sets`), which the
one-fixture error uses.

**File**: `cli/vcs-test-support/src/handover.rs` (new)

```rust
pub const SHARED_WORKBENCHES_VARIABLE: &str = "ACCELERATOR_SHARED_WORKBENCHES";

#[derive(Debug, Clone, Default)]
pub struct Handover {
    pub trees: BTreeMap<Family, PathBuf>,
    pub workbenches: Option<PathBuf>,
    pub log: FixtureLog,
    pub test: TestName,
}

impl Handover {
    pub fn from_environment() -> Self;
    pub fn from_variables(lookup: impl Fn(&str) -> Option<String>) -> Self;
    pub fn shared_tree(&self, family: Family) -> Option<&Path>;
}
```

`Family::Matrix.handover_variable()` is `ACCELERATOR_SHARED_MATRIX`. Each
family's variable, flag and log name live on `Family`, so a new family is a
new variant plus its `SharedFamily` impl.

**File**: `cli/vcs-test-support/src/workbench.rs` (new)

```rust
pub struct PrivateBuild(());

pub trait SharedFamily: Sized {
    const FAMILY: Family;
    fn adopt(tree: &Path) -> Result<Self, Error>;
    fn build_privately(
        directory: &Path,
        hermetic: &Hermetic,
        token: PrivateBuild,
    ) -> Result<Self, Error>;
}

pub struct Workbench {
    directory: TempDir,
    hermetic: Hermetic,
    handover: Handover,
    holds: Cell<Option<Held>>,
}

pub struct Fixture<'w, F> {
    fixture: F,
    workbench: PhantomData<&'w Workbench>,
}

impl<F> Deref for Fixture<'_, F> {
    type Target = F;
}

impl Workbench {
    pub fn for_this_test() -> Result<Self, Error>;
    pub fn with(handover: Handover) -> Result<Self, Error>;
    pub fn path(&self) -> &Path;
    pub fn hermetic(&self) -> &Hermetic;
    pub fn shared<F: SharedFamily>(&self) -> Result<Fixture<'_, F>, Error>;
    pub fn matrix(&self) -> Result<Fixture<'_, Matrix>, Error>;
}
```

A private build lives inside the workbench, so a `Fixture` borrows it: the
compiler refuses any use of the fixture after the workbench is dropped.
`holds` records which fixture the workbench has handed out (the family, and
whether it was adopted, copied or built), which is what the one-fixture
error names.

`PrivateBuild` has a private field, so only this crate can make one. Code
outside the crate can still *implement* `SharedFamily`, as the cheap test
family in `workbench.rs` does, because it can name the type. But it can
never *call* `build_privately`, so no test can build a shared family
privately except through the workbench, which logs it.

The workbench directory's prefix is `vcs-workbench-`, so the goldens' existing
`fixture_tempdir_path` mask (`masks.toml:50-51`) still matches it.

#### 3. The matrix as a shared family

**File**: `cli/vcs-test-support/src/fixtures.rs`

- Remove `MATRIX_ROOT_VARIABLE`, `matrix_root` and `build_or_adopt`
  (`:23-57,99-116`), along with the module doc's references to a CI job.
- Replace `build_in` with `build_with(base, env)`, which builds with a
  `Hermetic` it is given and runs `assert_git_is_recent_enough`.
  `Workbench::with` runs `assert_no_repository_ancestor` on its directory
  before it roots its `Hermetic`, so a refused workbench gets nothing
  written into it.
- `build_in(base)` and `build_for_sharing(base)` are added in Phase 2, with
  the shared build that first calls them. Added here, they would have no
  caller, and `warnings = "deny"` makes an unused `pub(crate)` function an
  error.
- Extract the manifest into a `Manifest` value
  (`cli/vcs-test-support/src/manifest.rs`) with `write(base)` and
  `read(base)`. It holds the matrix's `Fixture` rows (key, shape, start);
  Phase 4 widens it for the status-log set.
- Make `adopt` public. It now fails when `fixtures.tsv` is missing, naming
  the base.
- Implement `SharedFamily for Matrix`: `adopt` is `Matrix::adopt`, and
  `build_privately` is `Matrix::build_with`.
- Once §4 has moved every consumer onto the workbench, make `build_with`
  `pub(crate)` and remove `build_in`. Only the workbench calls the builder,
  so the compiler stops any test building the matrix behind the log's back.

**File**: `cli/vcs-test-support/tests/matrix_sharing.rs` (new). These tests
run no git:
- A `Manifest` written to a directory reads back with the same keys and
  start directories.
- `Matrix::adopt` of a manifest-only tree gives those keys and start
  directories.
- `adopt` fails on a missing manifest, a malformed line, or an empty
  manifest.

#### 4. Move the matrix consumers onto the workbench

In each consumer, only the fixture-acquisition lines change. Each test opens
`let workbench = Workbench::for_this_test()?;` and then
`let matrix = workbench.matrix()?;`.

| File | Tests | Other change |
|---|---|---|
| `cli/vcs-adapters/tests/classify.rs:175-237` | 6 | none |
| `cli/vcs-adapters/tests/queries.rs:401-589` | 5 (`the_jj_binary_matches_the_resolved_library` is untouched) | none; `$B` is still `matrix.base` |
| `cli/vcs-test-support/tests/matrix.rs:14-18` | 6 | the local `matrix()` helper is removed; each test opens its workbench and matrix in the same two lines as the other consumers |
| `cli/vcs-adapters/tests/scrub.rs:87-138` | 2 | `poison_config = workbench.path().join("poison.gitconfig")` |

**Assertion map** (for the work item's "every assertion still made"
criterion):
- **classify, queries, matrix.** Every `assert!`, `assert_eq!` and
  table check stays in the same function, unchanged.
- **scrub.** The `differences.is_empty()` and control `contains`
  assertions are unchanged. Only the poison config's location moves, and no
  assertion reads that location.
- **Names, counts and goldens.** No test is renamed, added or removed. These
  tests use no golden files.

**File**: `tests/unit/tasks/test_fixture_builds.py` (new)

The compiler stops tests calling the builders once they are `pub(crate)`
(§3). This guard covers the one bypass it cannot see: a `Workbench::with`
whose `Handover` carries a test-local log, which builds a shared family
privately and leaves no trace in the run's log. It scans
`cli/*/tests/**/*.rs` and fails on any `Workbench::with(` outside an
allow-list, naming the file and line and saying to use
`Workbench::for_this_test()`. In Phase 1 the allow-list is
`cli/vcs-test-support/tests/workbench.rs`. Its own tests run the scan over a
small tree written in `tmp_path`: a use in an allowed file passes, and the
same use elsewhere fails.

### Success Criteria

#### Automated Verification

- [x] The new and moved tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support -p vcs-adapters`
- [x] The guard passes: `uv run pytest tests/unit/tasks/test_fixture_builds.py -v`
- [x] Nothing in `cli/`, `tasks/`, `mise.toml` or `.github/` mentions the
      old matrix variable:
      `! rg -n 'ACCELERATOR_ZERO_SPAWN_MATRIX|MATRIX_ROOT_VARIABLE' cli tasks mise.toml .github`
- [x] The cli checks pass: `mise run cli:check`
- [x] The cli lane passes, with coverage at or above its floor:
      `mise run test:unit:cli`
- [x] The full local run passes: `mise run`

#### Manual Verification

- [x] `cargo nextest list` shows the same names and count for `classify`,
      `queries`, `matrix` and `scrub` as at `287b6279`: the listings at both
      revisions are identical (6, 6, 6 and 3 tests).

---

## Phase 2: The shared build step in `test:unit:cli`

### Overview

`test:unit:cli` builds the matrix once and hands it over to the tests. It
guards the tree with a hash, cleans up whatever the outcome, and prints the
fixture summary. Both coverage lanes do this, with no filter yet.

### Changes Required

#### 1. The shared build in the library (tests first)

**File**: `cli/vcs-test-support/tests/shared_build.rs` (new)

- `SharedBuildRequest::parse(["--matrix", "/a"])` gives a request for the
  matrix at `/a`. An unknown argument, a missing value, or no family at all
  is an error that names the problem.
- **A real build error.** A request whose destination lies inside this
  checkout fails `assert_no_repository_ancestor` before any git runs. The
  destination is an existing directory of the crate (its `src/`). The test
  lists `src/`'s entries before and after the build and asserts they are
  unchanged, so nothing has been written into the checkout.
  `build` returns one `FamilyFailure` for the matrix carrying that error, and
  records no event.
- **A successful build.** With an injected builder that succeeds,
  `build_with` returns no failure and records exactly one `shared-build`
  event: the family, its tree, and the `ToolVersions`.
- **A panicking build.** A family whose build panics on its scoped thread
  becomes a `FamilyFailure` naming the panic. Before Phase 5 adds a second
  family, the test injects a builder that panics on its first call and
  succeeds on its second, over two requests. From Phase 5, the other
  family's result is returned alongside it. A panic carrying a formatted
  `String`, and one carrying no string at all (`an unprintable panic`),
  each still fail their family.
- **Logging.** Every test here passes a test-local `FixtureLog::at(...)` or
  `FixtureLog::disabled()`, never `from_environment()`, so none writes to a
  run's log.
- **Tool versions.** `ToolVersions::detect()` returns the first line of
  `git --version` and of `jj --version`.

**File**: `cli/vcs-test-support/src/shared_build.rs` (new)

```rust
pub struct SharedBuildRequest {
    pub trees: BTreeMap<Family, PathBuf>,
}

pub struct FamilyFailure {
    pub family: Family,
    pub cause: String,
}

pub struct ToolVersions {
    pub git: String,
    pub jj: String,
}

impl ToolVersions {
    pub fn detect() -> Result<Self, Error>;
}

impl SharedBuildRequest {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, Error>;
    pub fn build(&self, log: &FixtureLog) -> Vec<FamilyFailure>;
    pub fn build_with(
        &self,
        log: &FixtureLog,
        builder: impl Fn(Family, &Path) -> Result<(), Error> + Sync,
    ) -> Vec<FamilyFailure>;
}
```

`build` is `build_with` over each family's `build_for_sharing`. When
`ToolVersions::detect` fails, `build_with` builds nothing and fails every
requested family with that cause.

**File**: `cli/vcs-test-support/src/fixtures.rs`
**Changes**: Add the two builders Phase 1 deferred, both `pub(crate)`:
- `Matrix::build_in(base)` runs `assert_no_repository_ancestor` before it
  roots a `Hermetic` at `base`, so a refused base gets nothing written into
  it, then calls `build_with`;
- `Matrix::build_for_sharing(base)` creates `base` if it is missing
  (`assert_no_repository_ancestor` canonicalises, so the base must exist),
  calls `build_in` and writes the manifest through `Manifest`.

The "real build error" test above drives `build_in`'s refusal, and the
build step's tests drive `build_for_sharing`.

- Each family's flag is `--<family>`, taken from `Family::flag`. `Family`
  also gains `name` (`matrix`), its name in the log and on the bin's
  stderr.
- `build` builds each requested family on its own scoped thread, using
  `Matrix::build_for_sharing`.
- It records a `shared-build` event for each family that builds, carrying
  the `ToolVersions`.
- `hermetic::tool_version` becomes `pub(crate)` so that `ToolVersions` can
  use it.
- Nothing in `src/` calls `fs::rename`, `NamedTempFile` or `.persist(`, so
  the store-duplication lint stays clean.

**File**: `cli/vcs-test-support/tests/fixtures/shared_fixtures.rs` (new),
plus a `[[bin]] name = "shared-fixtures"` table with `test = false` in
`cli/vcs-test-support/Cargo.toml`, with a comment pointing at
`vcs-adapters`' fixture binaries as the precedent. With
`test = false`, cargo builds no libtest harness for the bin, so the unit
listing names one `shared-fixtures` executable, not two.

```rust
const FORCED_FAILURE_VARIABLE: &str = "ACCELERATOR_SHARED_FIXTURES_FAIL";

fn main() -> ExitCode {
    let request = match SharedBuildRequest::parse(env::args().skip(1)) {
        Ok(request) => request,
        Err(error) => return refuse(&error),
    };
    let failures = match forced_failure() {
        Ok(Some(family)) => vec![forced(family)],
        Ok(None) => request.build(&FixtureLog::from_environment()),
        Err(unrecognised) => return refuse(&unrecognised),
    };
    report(&failures)
}
```

The forced failure lives in the bin alone, so the library's `build` carries
no test switch. The bin prints each failure as `<family>: <cause>` on
stderr, and exits 1 if there was any failure.

**File**: `cli/vcs-test-support/tests/shared_fixtures_bin.rs` (new)
- Runs `CARGO_BIN_EXE_shared-fixtures` with no family, and expects exit 2
  with a usage message.
- Runs it with the forced failure naming `matrix`, and expects exit 1 with
  stderr `matrix: … ACCELERATOR_SHARED_FIXTURES_FAIL …`. Nothing is
  created under the destination. This runs no git.
- Runs it with `ACCELERATOR_SHARED_FIXTURES_FAIL=matrx`, and expects exit 2
  with `ACCELERATOR_SHARED_FIXTURES_FAIL='matrx' is not a family; expected
  matrix` (the list grows with `Family`).
- Runs it with an empty `PATH`, and expects exit 1 with stderr starting
  `matrix: git: `. Nothing is created under the destination.
- Every family-failure is built as `FamilyFailure { family, cause }`; the
  library has no test-only constructor.
- Every case `env_remove`s the fixture log variable, so none writes to the
  run's log.

#### 2. The shared-fixtures package (tests first)

The task's domain lives in `tasks/shared/shared_fixtures/`, one module per
seam, each with its own test file under `tests/unit/tasks/shared/shared_fixtures/`.

**File**: `tests/unit/tasks/shared/shared_fixtures/test_runs.py` (new)

The process cases use a child Python process that starts a run, prints its
run id once the lock is held, and then blocks reading its stdin. The parent
waits for that line before acting, and every wait has a timeout of a few
seconds, so a broken handshake fails the test rather than hanging it. A
fixture starts each child and kills and waits on it in teardown, and a child
exits when its stdin closes, so a failed test never leaves a process holding
a lock.

Cases that rely on file permissions (the unopenable lock, the unremovable
leftover) carry the repo's existing `skipif(os.geteuid() == 0, reason="no
permission stops root")`, as in
`tests/unit/tasks/shared/build_measurement/test_workspace.py`. The root's
owner and mode checks read the root's status through an injected `stat`, so
a foreign owner is simulated without `chown`.

- **`FixtureRuns.open(root)`** runs before anything else touches the
  root. It `lstat`s the path it is given, without resolving it.
  - A missing root is created with mode `0700`; there is nothing to sweep.
  - A root that is a symlink, or not a directory, is refused. A test makes
    the root a symlink to a directory holding a file, and that file
    survives.
  - A root owned by another user, or writable by group or world, is
    refused.
  - A root inside a VCS checkout (a `.git` or `.jj` in any ancestor) is
    refused. `tempfile.gettempdir()` falls back to the current directory
    when no temp directory is writable, and this names that case:
    `the shared root <root> lies inside a checkout; set TMPDIR to a writable
    temp directory`.
  - Two first runs creating the root at once both start: a `mkdir` that
    raises `FileExistsError` falls through to the same `lstat` checks. The
    interleaving is tested directly.

  A refusal raises `SharedFixturesError` naming the path, its owner and its
  mode, and deletes and creates nothing. It ends with the remedy: `move or
  remove <root> if you don't recognise it; the next run recreates it with
  mode 0700`. The temp directory above the root is resolved by the caller,
  so macOS's `/var` → `/private/var` still works; only the root itself must
  not be a link.
- **`FixtureRuns.open(root).start()`.**
  - The run id is a UTC timestamp plus a `uuid4`.
  - It creates `<root>/<run-id>.lock` already locked: the lock file is
    created with `O_EXCL` under a temporary name, `flock`ed, then moved into
    place with `os.link` and an unlink, so an existing lock is never
    replaced. If the lock path exists, `start` fails with
    `SharedFixturesError`.
  - `<root>/<run-id>/` is created with `exist_ok=False` only after that,
    along with `<run>/workbenches/`.
  - `run.tree(Family.MATRIX)` is `<run>/matrix`, and `run.workbenches` is
    `<run>/workbenches`.
  - The run holds its open, `flock`ed descriptor until `close()`.
  - A `flock` that raises an `OSError` other than `BlockingIOError`, or an
    `os.link` that fails with any `OSError` other than `EEXIST` (the
    existing-lock case, refused separately), raises
    `SharedFixturesError` naming the root and saying the temp filesystem
    must support `flock` and hard links; set `TMPDIR` to one that does. The
    run lock deliberately has no `mkdir` fallback, unlike `workspace_lock`,
    because the sweep's liveness rule depends on `flock`. Both are tested by
    injecting the primitive, the link case with `EPERM` and with `ENOSYS`.
- **`sweep_abandoned()`** considers only entries named by the run-id
  grammar (`<UTC timestamp>-<uuid4>`): `<run-id>`, `<run-id>.lock` and
  `<run-id>.lock.tmp…`.
  - Any other entry under the root is left alone, with a warning naming it
    and saying `remove it by hand if no run is using it`. A test puts a
    directory with a foreign name under the root, and it survives the
    sweep.
  - A run whose child was killed with SIGKILL has its directory and lock
    removed.
  - A run whose child is alive is kept.
  - An orphan run directory with no lock is removed. Whether a run is an
    orphan is decided for each run id at the moment of the decision, by
    opening `<run-id>.lock` without `O_CREAT`. Only `ENOENT` then makes the
    directory an orphan; the directory listing is never trusted for it. An
    interleaving test creates the lock after the listing but before the
    decision, and the directory survives.
  - An unlocked `*.lock.tmp` older than a minute is removed, and a younger
    one is kept, so a run that is just starting is never disturbed.
  - A run is abandoned only when the sweep's own non-blocking `flock`
    succeeds. A lock it cannot open (simulated with mode `000`), or a
    `flock` that raises an `OSError` other than `BlockingIOError` (simulated
    by injecting the lock primitive), keeps the run, with a warning naming
    it.
  - The sweep opens locks without `O_CREAT`, so it never recreates a lock
    another sweep has just removed.
  - The races are tested as interleavings, not by timing: an abandoned run
    whose directory is already gone, and one whose lock is already gone,
    are both swept without error.
  - A leftover the sweep cannot remove (a directory made read-only) is made
    writable and retried. If it still can't be removed, the sweep warns,
    names it, says `remove it by hand if no run is using it`, and carries
    on: another run's debris never fails this run.
  - Making a leftover writable walks it with `lstat`, never follows a
    symlink, and adds owner write and execute only to directories: removing
    an entry needs write on its directory, not on the entry. Files and
    symlinks are left alone, so a hard-linked file shared with something
    outside the root keeps its mode. Two cases: a read-only leftover
    holding a symlink to a file outside the root, and one holding a
    read-only file hard-linked from outside it. Both are removed, and both
    outside files keep their modes.
- **`run.close()`** removes the directory, then the lock path, then releases
  the descriptor. It removes the directory with the same routine as the
  sweep: `rmtree`, then the symlink-safe make-writable walk, then one retry.
  A read-only directory a failed test left in its workbench is therefore
  removed, not reported.

**File**: `tasks/shared/locking.py`
**Changes**: Extract `workspace_lock`'s `flock` attempt into a primitive the
run lock and the sweep share. `workspace_lock`'s own tests are unchanged.

**File**: `tests/unit/tasks/shared/test_hashing.py`

- **`tree_digest(path)`** gives a `TreeDigest`: a hash of each entry's
  content and mode by relative path, with symlinks hashed by target, plus an
  overall `value`. It reads each entry's type with `lstat` and opens only
  regular files. Any other type (FIFO, socket, device) is recorded by its
  type and mode, never opened. Cleanup runs with signals deferred, so a
  blocking `open` on a FIFO could not be interrupted.
  - A tree holding a FIFO is digested without blocking, and adding the FIFO
    shows up as an added path.
  - It is stable across calls.
  - `value` changes when a file's content or mode changes, and when an entry
    is added or removed, empty directories included.
  - It does not change when only timestamps change.
  - `later.changes_since(earlier)` lists the added, removed and modified
    relative paths, and is empty for an unchanged tree.

**File**: `tests/unit/tasks/test_instrumented_build.py`

- **`UnitListing.executable("shared-fixtures")`** returns the `executable`
  of the `compiler-artifact` whose target is the `shared-fixtures` bin and
  whose `profile.test` is false. A listing that also carries a
  harness-shaped artifact for the same target (`profile.test: true`) still
  gives the real bin. If there is none, it raises a `LookupError` naming
  the target.

**File**: `tests/unit/tasks/shared/shared_fixtures/test_log.py` (new)

- **The shared format.** Parsing
  `cli/vcs-test-support/contract/fixture-log.jsonl`, the file the
  Rust format test writes against, gives one event of each kind with every
  field read.
- **The task's own events.** `FixtureLog.record_digest(family, tree, phase,
  digest)` appends a `tree-digest` event (`family`, `tree`, `phase` of
  `built` or `removed`, `value`), and `record_changes(family, tree,
  changes)` appends a `tree-changes` event listing the changed paths. Each
  serialises to its line in the shared file.
- **The shared names.** Each variable name the package uses (the hand-over
  variable of each family, the workbenches variable and the log variable)
  equals its entry in `cli/vcs-test-support/contract/variables.json`.
  A Rust test in `fixture_log.rs` checks `Family::handover_variable` and the
  `pub const`s against the same file, so both languages test against one
  artefact rather than against each other's source.
- **`FixtureLog(path).summary(root)`** over a sample log gives these lines:

  ```
  Shared VCS fixtures (log: <path>; shared root: <root>)
    matrix: 1 shared build, 19 adoptions, 0 copies, 0 private builds
    private sets: 0 private builds
  ```

  No line matches `timeline._NEXTEST_SUMMARY`, and no line fully matches
  `timeline._FINISHED_IN`. In Phase 5 a `status-log set` line joins it.
- **`problems(run)`** takes a `FinishedRun`: the tree of each family that
  was built (its build returned and its first digest was taken), the two
  digests of each tree that could be digested both times, whether the run
  was filtered, and whether the tests passed. A family whose build failed is
  not "built", so a builder failure never adds a "no shared build" line. A tree that could not be
  digested a second time is reported once, by the cleanup step that failed,
  and not again by `problems()`. It names
  each of the following, and returns nothing for a clean run.
  - In every run:
    - a tree whose digest after the build differs from its digest before
      removal, listing the changes, at most 10, then "and N more" (`the
      matrix tree <path> changed during the run: added poison.gitconfig;
      modified clean-git/.git/index`). It ends with how to find the writer:
      `the tree has been removed; to find the test that wrote it, rerun each
      consumer alone with mise run test:unit:cli --filter
      'binary_id(<consumer>)'`, naming each of `consumers_of(family)`;
    - any `private-build` event for a shared family, whether or not the run
      built that family. In a filtered run that skipped the family, the
      message is `<test> built the matrix privately; add <binary> to the
      matrix consumers in tasks/shared/shared_fixtures/consumers.py`. In a
      run that handed the family over, it is `<test> built the matrix
      privately although the run handed it over through
      ACCELERATOR_SHARED_MATRIX; its Handover did not come from the
      environment; open it with Workbench::for_this_test(), or give a
      deliberately private Handover a test-local FixtureLog`;
    - a line of the log that does not parse, by line number;
    - a built family with no `shared-build` event for that family's tree,
      which also covers an empty or missing log (`the matrix was built but
      the log records no shared build of <tree>; the hand-over did not reach
      the builder`). A `shared-build` event naming another tree does not
      count.
  - In an unfiltered run:
    - an `adoption` or `copy` from a binary not among that family's
      consumers (`<binary> uses the matrix but is not among its consumers in
      tasks/shared/shared_fixtures/consumers.py; add it there`).
  - In an unfiltered run whose tests passed: a consumer binary that
    recorded no event for its family (`<binary> is listed as a matrix
    consumer but used no matrix; remove it from
    tasks/shared/shared_fixtures/consumers.py`). A failed run is exempt,
    because nextest may have stopped before reaching the binary.
- **`FixtureLogs(dir).create(run_id)`** creates the run's log and nothing
  else.
- **`FixtureLogs(dir).prune(keep=20, live=…)`** prunes to the newest 20
  logs, never pruning the log of a run id in `live`. The caller passes
  `FixtureRuns.live_run_ids()`, so the log module knows nothing of locks.
  It considers only files named `<run-id>.jsonl` by the same run-id grammar
  as the sweep. A test puts a foreign file in the directory, and it
  survives. It unlinks with `missing_ok=True`, so two runs finishing
  together and pruning the same log both succeed; an interleaving test
  removes the log between the listing and the unlink.

**File**: `tests/unit/tasks/shared/shared_fixtures/test_consumers.py` (new)

- `CONSUMERS` names binaries only: every key is a nextest binary id, and
  every value is a family.
- `consumers_of(Family.MATRIX)` is the four matrix binaries.

**File**: `tests/unit/tasks/shared/shared_fixtures/test_runs.py` (the same
file as above, continued)

- **`FixtureRuns.open(root).live_run_ids()`** is the ids of runs whose lock is
  held, using the same liveness rule as the sweep.
- **`HoldPoint(dir, poll=…, timeout=…).wait(run)`** writes `held`
  containing the run directory `run`, and returns once `release` exists. A thread creates
  `release` after a short delay. The poll interval and timeout are
  injected, and the test itself has a timeout.
- **A hold nobody releases.** With a timeout of a few milliseconds and no
  `release`, `wait()` raises `SharedFixturesError("shared VCS fixtures:
  nothing released <dir> within <timeout>")`. The production timeout is 30
  minutes.
- **`HoldPoint.from_environment(environ)`** returns no hold point when
  `ACCELERATOR_SHARED_FIXTURES_HOLD` is unset. When it is set and `CI` is
  set too, it raises `SharedFixturesError` naming the variable and saying
  it is never honoured in CI.

**Files**: `tasks/shared/shared_fixtures/{__init__,errors,families,runs,log,consumers,session}.py`
(new). The package does filesystem work through `fcntl`, `os` and
`shutil`; it runs no subprocess.

**File**: `tasks/shared/shared_fixtures/families.py`

```python
class Family(StrEnum):
    MATRIX = "matrix"

    @property
    def handover_variable(self) -> str: ...

    @property
    def heading(self) -> str: ...

    @property
    def noun(self) -> str: ...


SHARED_FAMILIES: tuple[Family, ...] = tuple(Family)
WORKBENCHES_VARIABLE = "ACCELERATOR_SHARED_WORKBENCHES"
FIXTURE_LOG_VARIABLE = "ACCELERATOR_FIXTURE_LOG"
HOLD_VARIABLE = "ACCELERATOR_SHARED_FIXTURES_HOLD"
```

`heading` names the family in the summary (`matrix`), and `noun` in prose
(`the matrix`).

**File**: `tasks/shared/shared_fixtures/errors.py`

```python
class SharedFixturesError(Exception): ...
```

**File**: `tasks/shared/shared_fixtures/runs.py`

```python
def shared_fixtures_root() -> Path:
    return (
        Path(tempfile.gettempdir()).resolve()
        / f"accelerator-shared-fixtures-{os.getuid()}"
    )


@dataclasses.dataclass
class FixtureRun:
    run_id: str
    directory: Path
    lock: HeldLock

    def tree(self, family: Family) -> Path: ...

    @property
    def workbenches(self) -> Path: ...

    def handover(self, families: Iterable[Family], log: Path) -> dict[str, str]: ...

    def close(self) -> None: ...


class FixtureRuns:
    @classmethod
    def open(
        cls,
        root: Path,
        *,
        lstat=os.lstat,
        flock=try_exclusive_flock,
        link=os.link,
        open_lock=lambda path: os.open(path, os.O_RDWR),
        new_run_id=_new_run_id,
        rmtree=shutil.rmtree,
    ) -> FixtureRuns: ...

    def sweep_abandoned(self) -> None: ...

    def live_run_ids(self) -> frozenset[str]: ...

    def start(self) -> FixtureRun: ...


class HoldPoint:
    @classmethod
    def from_environment(cls, environ: Mapping[str, str]) -> HoldPoint | None: ...

    def wait(self, run: Path) -> None: ...
```

The keyword collaborators of `FixtureRuns.open` default to production
behaviour; the tests inject them to simulate a foreign owner, an
unsupported `flock` or hard link, an existing lock, a lock appearing or
vanishing between the listing and the decision, and an unremovable
leftover. The make-writable walk gives directories owner read as well as
write and execute, so a leftover directory of mode `000` can still be
listed and emptied.

**File**: `tasks/shared/shared_fixtures/log.py`

```python
class DigestPhase(StrEnum):
    BUILT = "built"
    REMOVED = "removed"


@dataclasses.dataclass(frozen=True)
class FixtureSummary:
    def lines(self) -> list[str]: ...


class FixtureLog:
    def __init__(self, path: Path) -> None: ...

    def read(self) -> LogContents: ...

    def summary(self, root: Path) -> FixtureSummary: ...

    def record_digest(
        self, family: Family, tree: Path, phase: DigestPhase, digest: TreeDigest
    ) -> None: ...

    def record_changes(
        self, family: Family, tree: Path, changes: TreeChanges
    ) -> None: ...

    def problems(self, run: FinishedRun) -> list[str]: ...


@dataclasses.dataclass(frozen=True)
class TreeDigests:
    built: TreeDigest
    removed: TreeDigest


@dataclasses.dataclass(frozen=True)
class FinishedRun:
    trees: Mapping[Family, Path]
    digests: Mapping[Family, TreeDigests]
    filtered: bool
    tests_passed: bool


class FixtureLogs:
    def create(self, run_id: str) -> FixtureLog: ...

    def prune(self, keep: int, live: Collection[str]) -> None: ...

    def listed(self) -> list[Path]: ...
```

`read()` gives a `LogContents`: the parsed events (`SharedBuild`,
`Adoption`, `PrivateBuild`, `TreeDigestRecorded`, `TreeChangesRecorded`)
and the numbers of the lines that do not parse. A missing log reads as
empty. A `PrivateBuild` of the private sets has no family. The concurrent
prune is tested by overriding `listed()`.

The two contract files' samples are what the code writes: the
`shared-build` sample's `git` and `jj` are whole first lines
(`git version 2.50.1`, `jj 0.43.0`), and the `tree-digest` sample's value
is the digest of an empty tree.

**File**: `tasks/shared/shared_fixtures/consumers.py`

```python
CONSUMERS: Mapping[str, Family] = {
    "vcs-adapters::classify": Family.MATRIX,
    "vcs-adapters::queries": Family.MATRIX,
    "vcs-adapters::scrub": Family.MATRIX,
    "vcs-test-support::matrix": Family.MATRIX,
}


def consumers_of(family: Family) -> frozenset[str]: ...
```

Consumption is recorded per binary, not per test. A binary that has some
tests that never touch the family (the jj-binary test in `queries`, one
`scrub` test, and later the private-set tests in `status_log_goldens`)
still counts as a consumer. The only cost is that a filtered run selecting
just those tests builds the family anyway. In exchange, renaming, adding or
splitting a Rust test never touches this module.

`STATUS_LOG = "status-log"` joins `Family` in Phase 5, so `SHARED_FAMILIES`
is every family the build step can build at each phase.

**File**: `tasks/shared/hashing.py`

```python
@dataclasses.dataclass(frozen=True)
class TreeDigest:
    entries: Mapping[str, str]

    @property
    def value(self) -> str: ...

    def changes_since(self, earlier: TreeDigest) -> TreeChanges: ...


@dataclasses.dataclass(frozen=True)
class TreeChanges:
    added: Sequence[str]
    removed: Sequence[str]
    modified: Sequence[str]


def tree_digest(tree: Path) -> TreeDigest: ...
```

**File**: `tasks/shared/instrumented_build.py`

```python
class UnitListing:
    def executable(self, target: str) -> Path: ...
```

#### 3. The session (tests first)

The run's lifecycle is domain behaviour, so it lives in the package, not in
the task. Only the builder's invocation needs invoke's `Context`, and the
session takes that as a callable.

**File**: `tests/unit/tasks/shared/shared_fixtures/test_session.py` (new)

Each test builds a `SharedFixtureSession` over `tmp_path` roots, with a fake
builder that creates `<tree>/fixtures.tsv` and writes a `shared-build` line
for that tree, and a body that writes one `adoption` per consumer binary.
Each failure-mode test overrides one of those, or one of the constructor's
injected collaborators (see the sketch below). Output goes to an injected
stream, so tests read the printed lines without capturing stdout.

- **A clean run.** `open(SHARED_FAMILIES, build=…, filtered=False)` builds
  each family, takes a `built` digest of each tree, and yields the hand-over
  environment: each family's tree variable, the workbenches variable and the
  log variable. On exit, the log holds a `removed` digest equal to the
  `built` one for each family and no `tree-changes` event, the run directory
  is gone, the logs are pruned, the summary is printed, and nothing is
  raised.
- **One policy for every failure.** A single `try` covers everything after
  the root is opened: reading the hold switch, creating the log, the build,
  the first digest, the hold and the body. Failures are classified once, by
  kind, not by invoke's types. The session imports nothing from invoke.
  - **An `Exception`** (a builder failure, a refusal, a hold timeout, or the
    body's own failure such as nextest's `Exit`). Each cleanup step runs in
    isolation: the second digest (only for trees that have a first one),
    closing the run, pruning and the summary. A step that raises an
    `Exception` becomes a problem line, except pruning, whose failure is
    only a warning, and the later steps still run.
    `RunOutcome` combines the failure with every problem, and the session
    raises `SharedFixturesError(outcome)` from the original.
  - **An interrupted run.** That is any other `BaseException`, such as
    `KeyboardInterrupt` raised during the hold, *or* any failure at all once
    the injected `interrupted()` reports that a signal has arrived. invoke
    catches `KeyboardInterrupt` inside `context.run` and forwards it to the
    child (`invoke/runners.py:490`), so a Ctrl-C during the build or nextest
    reaches the session as an ordinary command failure. Only `interrupted()`
    tells the two apart. Close the run and print the summary, each inside
    `try/except Exception`. A failure there is printed as a warning naming
    the leftover, which the next sweep removes. Then raise
    `KeyboardInterrupt` (re-raising the original when it was one). The
    second digest and `problems()` are skipped, because tests killed
    mid-write leave trees and log lines that would yield bogus problems.
  - **Signals during cleanup.** As `build_runner.py:93-96` does, cleanup
    enters `deferred_signals(UNWIND_SIGNALS)` only when no signal has
    arrived yet. A first signal during cleanup on the `Exception` path is
    then held until cleanup ends and raised once as `KeyboardInterrupt`.
    After a signal has arrived, the task's `unwind_then_defer` handlers
    already ignore later ones, and the session leaves them in place.
- **`RunOutcome(body_failure, problems).failure()`** is `None` for a clean
  run. With only a body failure it is that message, with only problems it
  is those lines, and with both it is the body's message followed by every
  problem line. A failed cleanup step's line counts as a problem.
- **The builder fails.** The session raises `SharedBuildError`, a
  `SharedFixturesError`, naming the family and its cause. It adds no "no
  shared build" line. The body never runs, no tree is digested, the run
  directory is gone, and the summary is printed. The task, not the session,
  adds the hint for running the tests directly (§4), so the session knows
  nothing of the cli lane's command line.
- **The root is refused.** `FixtureRuns.open` raising `SharedFixturesError`
  propagates as it is. Nothing is created and nothing needs cleaning.
- **The hold.** With a hold point injected, the session prints `shared VCS
  fixtures: held by ACCELERATOR_SHARED_FIXTURES_HOLD; touch <dir>/release to
  continue (run directory: <run>)` and waits after the first digest. A thread
  releases it. Poll interval and timeout are injected, so a broken release
  fails the test instead of hanging it.
- **The hold times out.** With a short injected timeout, the session raises
  `SharedFixturesError` saying nothing released the hold. The run directory
  is gone and the summary is printed.
- **The hold under CI.** The session reads the hold switch from the
  environment mapping it is given. With the hold variable and `CI` both set
  in that mapping, it raises `SharedFixturesError` naming the variable
  before any build or run directory.
- **An interrupt during the hold.** The hold raises `KeyboardInterrupt`; the
  session re-raises it, and the run directory is gone.
- **An interrupt during the build or the body, as invoke reports it.** The
  fake builder (or body) sets the injected `interrupted()` to true, then
  raises an ordinary failure, as `context.run` returns after forwarding
  Ctrl-C. The session raises `KeyboardInterrupt`, not
  `SharedFixturesError`. The run directory is gone, no "no cli tests ran"
  hint or problem line is produced, and after the body the log holds no
  `removed` digest.
- **A failing cleanup step.** The session takes the run's removal and the
  log pruning as injected collaborators, and each test makes one of them
  raise.
  - Removal raises during an interrupt: `KeyboardInterrupt` still
    propagates, and a warning names the leftover.
  - Removal raises after a body failure: the summary still prints, and the
    raised error carries the body's failure and a line naming the leftover.
  - Pruning raises after a clean body: the summary still prints, a warning
    names the pruning failure, and nothing is raised. Losing an old log is
    not a test outcome.
- **A signal during cleanup.** After a body failure, the fake digest sends
  the process a real `SIGINT` with `os.kill(os.getpid(), signal.SIGINT)`.
  The signal is held until cleanup ends: the run directory is gone, the
  summary prints, and then `KeyboardInterrupt` propagates once.
- **The body fails.** When the body raises `Exit("nextest: cli tests failed
  …")`, the session raises `SharedFixturesError` carrying that message, the
  second digest is still taken, and the "listed consumer used nothing"
  check is skipped.
- **A tree a test deleted.** The run directory is still gone, the summary is
  printed, and the error carries the body's failure and one line naming the
  tree that could not be digested.
- **A tree changed during the run.** The session raises an error naming the
  tree and the added path, alongside any body failure. The log holds a
  `built` and a `removed` digest and a `tree-changes` event.
- **The problem cases.** A private build of a shared family, an unlisted
  consumer, a silent consumer (in an unfiltered passing run), and no
  `shared-build` event each raise `SharedFixturesError` with the message
  `problems()` gives.
- **The run's own lines.** The session prints `shared VCS fixtures: building
  matrix` before the build, `shared VCS fixtures: built in <s> s, digested
  in <s> s` after the first digest, and `shared VCS fixtures: digested again
  in <s> s, removed in <s> s` after cleanup. None matches
  `timeline._NEXTEST_SUMMARY` or fully matches `timeline._FINISHED_IN`.
- **The sweep.** A pre-existing abandoned run under the root is removed
  before the new run starts.

**File**: `tasks/shared/shared_fixtures/session.py` (new)

```python
@dataclasses.dataclass(frozen=True)
class RunOutcome:
    body_failure: str | None
    problems: Sequence[str]

    def failure(self) -> str | None: ...


class SharedBuildError(SharedFixturesError): ...


class SharedFixtureSession:
    def __init__(
        self,
        root: Path,
        logs: Path,
        environ: Mapping[str, str],
        *,
        interrupted: Callable[[], bool] = lambda: False,
        hold_point: Callable[[Mapping[str, str]], HoldPoint | None] = HoldPoint.from_environment,
        runs: Callable[[Path], FixtureRuns] = FixtureRuns.open,
        logs_store: Callable[[Path], FixtureLogs] = FixtureLogs,
        digest: Callable[[Path], TreeDigest] = tree_digest,
        out: TextIO | None = None,
    ) -> None: ...

    @contextlib.contextmanager
    def open(
        self,
        families: Collection[Family],
        build: Callable[[Mapping[Family, Path], Path], None],
        filtered: bool,
    ) -> Iterator[Mapping[str, str]]: ...
```

`build` receives each family's tree and the log path, and raises
`SharedFixturesError` on failure; the session wraps that as
`SharedBuildError` (ruff's N818 requires the `Error` suffix). The body passing means the tests passed, which
`problems()` needs. The session calls `hold_point(environ)` inside its
`try`, so a refused hold is handled like any other failure. The keyword
collaborators default to production behaviour. Tests replace them: a hold
point with a short poll and timeout, a `FixtureRuns` whose run removal
raises, a `FixtureLogs` whose pruning raises, a `digest` that sends the
process `SIGINT` on its second call, and an `interrupted` flag. `out`
defaults to `None`, which `print` resolves to `sys.stdout` at each call, so
captured output still reaches the test; every line is flushed, so it lands
in order with nextest's output when piped.
`RunOutcome` and its tests live here, with the policy that uses them.

#### 4. The build step in the task (tests first)

**File**: `tests/unit/tasks/test_rust.py`

The lifecycle is tested in `test_session.py`. These tests cover only what
the task adds: the command sequence, the environments, and the wiring.

- Add `SHARED_FIXTURES = "<tmp>/shared-fixtures"` to `_OUTPUTS` for the
  `cargo test` prefix. The unit-listing fixture gains one `shared-fixtures`
  `compiler-artifact` line whose `executable` is under `build_dir`.
- Add a `shared_fixture_roots` fixture to `tests/unit/tasks/conftest.py`,
  autouse for every test module that runs `test.cli.run`. It monkeypatches
  `test_cli._shared_root` and `test_cli._FIXTURE_LOGS` to `tmp_path` roots,
  so no unit test sweeps the real shared-fixtures root or prunes real logs.
  It also removes `ACCELERATOR_SHARED_FIXTURES_HOLD` and
  `ACCELERATOR_SHARED_FIXTURES_FAIL` from the environment, so a switch
  left exported in a developer's shell never reaches a unit test.
- Add a `FakeSharedFixtures` helper in
  `tests/unit/tasks/shared_fixture_doubles.py`, beside `passed`, `failed`
  and `builder_artefact`, used by every test that runs the task. It is a
  module rather than part of the conftest, because importing from a
  conftest loads it twice. It answers the builder command by creating
  `<tree>/fixtures.tsv` and writing a `shared-build` line for that tree to
  the handed log. It answers the nextest command by writing one `adoption`
  per consumer binary of each handed family, then returning the configured
  nextest result.
- **The coverage-on sequence** becomes the following, and the existing
  `:212` unpacking is rewritten to it:
  1. clean;
  2. `show-env`;
  3. the unit listing (compile and locate the builder, in the instrumented
     environment);
  4. the builder (instrumented environment with its profile file moved
     beside the tests', plus `--matrix <run>/matrix`);
  5. `llvm-cov nextest`, whose `env` carries `ACCELERATOR_SHARED_MATRIX`,
     `ACCELERATOR_SHARED_WORKBENCHES` and `ACCELERATOR_FIXTURE_LOG`;
  6. report;
  7. the unit listing (for the sweep).
- **The coverage-off sequence** is the unit listing with no `env`, then the
  builder, then `cargo nextest run` with the hand-over `env`.
- **A failing builder command** stops the task with `Exit` naming the family
  and its stderr cause, followed by `_RUN_DIRECTLY`: `no cli tests ran; to
  run them while diagnosing, use cargo nextest run {_MANIFEST} (each test
  builds its own fixtures)`. The task owns that hint because it owns
  `_MANIFEST`. No nextest command is run.
- **An interrupt.** The session is built with `interrupted` reading the
  task's `Interruption.arrived`. A fake nextest call that sets it, as
  `unwind_once` does, and then returns non-zero, as invoke does after
  forwarding Ctrl-C, ends the task with `KeyboardInterrupt`, not `Exit`.
- **Translation at the boundary.** Every `SharedFixturesError` from the
  session becomes `Exit(str(error))` raised `from` it, once, in `run`, as
  `tasks/test/cli.py` already does for `CoverageReportError`. A refused root
  and a hold under `CI` each surface as one `Exit` line, with no
  traceback.
- **A failing nextest** raises `Exit("nextest: cli tests failed …")`.
- **Inherited hand-over variables.** With `ACCELERATOR_SHARED_MATRIX`,
  `ACCELERATOR_SHARED_WORKBENCHES` and `ACCELERATOR_FIXTURE_LOG` exported in
  the task's own environment, pointing elsewhere, the builder and nextest
  see only the values the task sets. A variable the task does not set is
  absent from their environment, not empty. invoke's `env=` only adds to
  the inherited environment, so these calls pass `replace_env=True` with a
  copy of `os.environ` that has the hand-over variables filtered out and the
  run's own values added.
- **A recompile.** When the fake nextest call's stderr carries a cargo
  `Compiling ` line, the task raises `Exit` saying nextest recompiled after
  the build step, alongside nextest's own failure if it had one.
- **Signals.** The task runs under `unwind_then_defer(UNWIND_SIGNALS)`. The
  first signal unwinds as `KeyboardInterrupt`, and later signals are
  ignored until the task exits, which is what the helper does today. The
  session's own `deferred_signals` covers a first signal that arrives during
  cleanup.
- **Unchanged checks.** The floor, sweep and report-failure tests keep
  their assertions, with their call counts updated.

**File**: `tests/unit/tasks/test_cli_permission_skip.py`
**Changes**: The task runs against `FakeSharedFixtures` in place of the
bare `MockContext`, so the builder creates its tree and the log carries the
`shared-build` and adoption lines a passing run needs. The autouse
`shared_fixture_roots` fixture applies. The banner assertions are
unchanged.

**File**: `tasks/test/cli.py`

```python
_shared_root = shared_fixtures_root
_FIXTURE_LOGS = CLI_TARGET_DIR / "vcs-fixture-logs"
_UNIT_LISTING = f"cargo test {_MANIFEST} --no-run --message-format=json"


@task
def run(context: Context) -> None:
    _announce_skipped_permission_tests()
    with unwind_then_defer(UNWIND_SIGNALS) as interruption:
        try:
            if coverage_enabled():
                _run_instrumented(context, interruption)
            else:
                _run_plain(context, interruption)
        except SharedBuildError as error:
            raise Exit(f"{error}\n{_RUN_DIRECTLY}", code=1) from error
        except SharedFixturesError as error:
            raise Exit(str(error), code=1) from error


def _run_plain(context: Context, interruption: Interruption) -> None:
    builder = _locate_builder(context, build_environment=None)
    with _session(interruption).open(
        SHARED_FAMILIES, _building_with(context, builder, {}), filtered=False
    ) as handover:
        _run_tests(context, f"cargo nextest run {_MANIFEST}", handover)


def _run_instrumented(context: Context, interruption: Interruption) -> None:
    _clear_previous_profile_data(context)
    build_environment = _instrumented_environment(context)
    builder = _locate_builder(context, build_environment)
    with _session(interruption).open(
        SHARED_FAMILIES,
        _building_with(
            context, builder, _profiling_beside_the_tests(build_environment)
        ),
        filtered=False,
    ) as handover:
        _run_tests(
            context, f"cargo llvm-cov nextest {_MANIFEST} --no-report", handover
        )
    totals = _report_totals(context)
    print(totals.summary_line())
    _sweep_superseded_artefacts(context, build_environment)
    _enforce_floor(totals)


def _session(interruption: Interruption) -> SharedFixtureSession:
    return SharedFixtureSession(
        _shared_root(),
        _FIXTURE_LOGS,
        os.environ,
        interrupted=lambda: interruption.arrived,
    )
```

`_building_with(context, builder, build_environment)` returns the callable
the session builds with. It runs

```
<builder> --matrix <run>/matrix
```

with `build_environment` plus the log variable, `hide=True`, `warn=True`
and `replace_env=True`, every hand-over variable the task does not set
filtered out. A non-zero exit raises `SharedFixturesError(f"shared VCS
fixtures: {stderr}")`. The root is resolved when the session is built, not
when `cli.py` is imported, so other invoke tasks never probe the temp
directory.

Some details:
- **`_instrumented_environment` is the existing `show-env` parse plus
  `CARGO_TARGET_DIR`,** lifted out of `_list_units` so that the builder and
  the sweep share it. Both unit listings use `_UNIT_LISTING`; the first
  passes no `env` when coverage is off.
- **The builder's profile data goes beside the tests'.** `show-env` names
  `LLVM_PROFILE_FILE` in `cli/target/`, but `cargo llvm-cov nextest` writes,
  and `report` reads, profile data in `cli/target/llvm-cov-target/`.
  `_profiling_beside_the_tests` keeps the file-name pattern `show-env`
  gives and moves it into `_INSTRUMENTED_BUILD.parent`. Without it the
  builder's run of the fixture code never reaches the totals.
- **`_run_tests` removes inherited hand-over variables** in the same way as
  the builder, so nextest sees only what this run hands over. It also checks
  nextest's captured stderr for a cargo `Compiling ` line.
- **Nothing in `cli.py` may contain `--profile` or
  `--ignore-default-filter`,** not even its docstrings, because
  `test_nextest_filter.py:37-47` checks the raw text.
- **The docstring adds one sentence:** the matrix is built once per run and
  handed to the tests, and `cargo nextest run` run directly builds per test.

**File**: `tasks/README.md`
**Changes**: Add a `### Shared VCS fixtures` subsection after
"Contract-suite filtering". It covers:
- the build step;
- the run location (one root per user, mode `0700`) and the lock-based sweep;
- the hand-over and log variables;
- the fixture summary, and the resolved root it prints;
- the rule that a test building a shared family privately fails the run, and
  where the consumer list lives;
- that the hand-over and log variables are owned by the task and must not be
  exported;
- what a nextest bump has to re-check: that nextest still sets
  `NEXTEST_BINARY_ID` and `NEXTEST_TEST_NAME`, because every event's test
  and binary come from them. (Recapturing `nextest-list.json` joins this
  list in Phase 3, which adds the capture.)

It also records the fail-fast trade-off from "What We're NOT Doing": a
failing shared build stops the lane before any test runs, and how to see
the rest of the suite while diagnosing.

The two test-only switches get their own small table in this subsection,
labelled as fault injection for checking the build step and never for CI.
"Contributor environment variables" is unchanged.

### Success Criteria

#### Automated Verification

- [x] The task and domain tests pass:
      `uv run pytest tests/unit/tasks/shared/shared_fixtures tests/unit/tasks/shared/test_hashing.py tests/unit/tasks/test_instrumented_build.py tests/unit/tasks/test_rust.py tests/unit/tasks/test_cli_permission_skip.py tests/unit/tasks/test_nextest_filter.py tests/unit/tasks/shared/test_locking.py -v`
- [x] The Rust tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support`
- [x] The build-system and cli checks pass:
      `mise run build-system:check && mise run cli:check`
- [x] The full local run passes: `mise run`

#### Manual Verification

On engywook:

- [x] Coverage on: `mise run --skip-deps test:unit:cli` passes and prints the
      summary.
  - The matrix shows 1 shared build, 19 adoptions, 0 copies and 0 private
    builds.
  - The log has a distinct `hermetic-root` for each adoption, none under
    `<run>/matrix`.
  - The coverage totals are no lower than at `287b6279`: lines 94.44%,
    functions 93.92%, regions 93.18%, against 94.41%, 93.90% and 93.15%.
- [x] Coverage off (`ACCELERATOR_COVERAGE=off`): the same summary, and no
      `Compiling` line after the builder runs.
- [x] Under coverage, the nextest step compiles nothing (no `Compiling` line
      between the builder's run and the `Summary [` line).
- [x] `ACCELERATOR_SHARED_FIXTURES_FAIL=matrix` stops the run before
      nextest's test phase.
  - There is one failure naming the matrix and its cause.
  - The summary is printed, and the log has no `adoption` event.
  - Nothing remains under the root the summary prints.
- [x] A temporary edit that makes one `classify` test write a file into
      `matrix.base` fails the run, and the failure names the matrix tree.
      Revert the edit.

---

## Phase 3: `--filter` on `test:unit:cli`

### Overview

`test:unit:cli` declares `--filter <filterset>` both in mise and in invoke. A
filtered run finds the selected tests first, and builds only the families
they consume. Under coverage it skips the report, sweep and floor. This
completes the matrix increment.

### Changes Required

#### 1. Selection (tests first)

**File**: `tests/unit/tasks/test_selection.py` (new)
- `Selection(None).is_filtered` is false, and `nextest()` is `""`.
- `Selection("test(a b) & binary(=x)").nextest()` is
  `"-E 'test(a b) & binary(=x)'"`.
  `shlex.split` of the rendered option gives back `-E` and the original
  expression. So does an expression containing `'`.
- `Selection("")` and `Selection("  ")` raise `ValueError("--filter needs
  an expression")`. An empty filter never silently means "everything".
- `Selection("x").nothing_selected("test:unit:tasks")` is the one
  message every lane uses when a filter runs no test: `--filter 'x' selected
  no tests; see mise run <task> --help for its syntax`. The nextest lanes
  add `test() matches test names; binary_id() matches test binaries`.

A rendering is the runner's own option and nothing else: no leading space,
and no separator such as npm's `--`. Each lane joins the parts of its
command line itself, leaving out an empty rendering.

**File**: `tasks/shared/selection.py` (new)

```python
@dataclasses.dataclass(frozen=True)
class Selection:
    expression: str | None = None

    @property
    def is_filtered(self) -> bool: ...

    def nextest(self) -> str: ...

    def nothing_selected(self, task: str) -> str: ...
```

`task` is the mise task name the developer ran (`test:unit:tasks`), never
the invoke name (`test.unit.tasks`), because the message points at
`mise run <task> --help`. Each lane passes its mise name as a constant. An
invoke task behind several mise tasks (`research` and
`research-exhaustive`, and `visualiser-docker` behind `:docker` and
`:docker:update`) takes the name as a hidden parameter that each
`mise.toml` `run` line supplies.

**File**: `tests/unit/tasks/test_filters.py` (new)
- `selection_from(None)` is an unfiltered `Selection`.
- `selection_from("x")` is `Selection("x")`.
- `selection_from("")` and `selection_from("  ")` raise
  `Exit("--filter needs an expression")`, with no `ValueError` traceback.

**File**: `tasks/test/filters.py` (new). This sits at the task layer
because it raises invoke's `Exit`. The `Selection` value object in
`tasks/shared/selection.py` stays free of invoke.

```python
def selection_from(filter_: str | None) -> Selection: ...
```

#### 2. Which families the selected tests consume (tests first)

**File**: `tests/unit/tasks/shared/shared_fixtures/test_consumers.py` (extended)

`families_consumed(listing)` works on a `cargo nextest list --message-format
json` capture. The capture is trimmed to the consumer binaries, plus one
non-consumer, and stored as `tests/unit/tasks/fixtures/shared-fixtures/nextest-list.json`,
with the nextest version that produced it recorded beside it.
- The recorded version equals the nextest version pinned in `mise.toml`, so
  a nextest bump fails this test until the capture is retaken.
- When every consumer matches, it returns the matrix.
- When only the non-consumer binary's tests match, it returns nothing.
- When only `the_jj_binary_matches_the_resolved_library` matches, it
  returns the matrix, because its binary is a consumer.
- When only the `vcs-adapters::classify` binary's tests match, it returns
  the matrix.
- It ignores test cases whose `filter-match.status` is `mismatch`, and a
  binary none of whose tests match.

**File**: `tasks/shared/shared_fixtures/consumers.py`

```python
def families_consumed(listing: str) -> frozenset[Family]: ...
```

The list can't drift silently. A consumer missing from `CONSUMERS` fails
every unfiltered run, CI included, as an unlisted adoption, so its author
sees it. In a filtered run that skipped its family, it fails as a private
build. Both messages name the binary and this module.

#### 3. The option and the filtered flow (tests first)

**File**: `tests/unit/tasks/test_rust.py`
- **Filtered, coverage off.**
  `run(lane, filter_="binary_id(vcs-adapters::classify)")` runs
  `cargo nextest list {_MANIFEST} -E 'binary_id(vcs-adapters::classify)' --message-format json`
  first, then the unit listing, then the builder with `--matrix` only, then
  `cargo nextest run {_MANIFEST} -E 'binary_id(vcs-adapters::classify)'`.
- **Filtered, coverage on.** The same sequence runs in the instrumented
  environment (`show-env` plus `CARGO_TARGET_DIR`). The list call alone
  runs with `LLVM_PROFILE_FILE=<dir>/list-%p.profraw`, where `<dir>` is a
  `tempfile.TemporaryDirectory()` that wraps only the list call. Listing
  needs the binaries, not their profile data. `%p` gives each listed binary
  its own file, and the directory is removed when the call returns,
  outside both the shared root and the coverage target directory. The test
  asserts that the directory is gone afterwards. Then:
  - no report, no sweep listing and no floor follow;
  - one line is printed: `coverage: report, sweep and floor skipped for a
    filtered run`.
- **A filter that selects no consumer.** When the list answer has no
  matrix consumer matching, no builder command is run, and the run prints
  `shared VCS fixtures: building nothing (no selected test consumes a
  shared family)`. The hand-over carries only the workbenches and log
  variables, and nextest still runs.
- **A filter that selects no tests.** When no test case in the list answer
  matches, the task raises `Exit(selection.nothing_selected(<task>))` before any
  builder command and before any run directory is created. nextest is
  never run.
- **A failing list.** If `cargo nextest list` exits non-zero, the task
  raises `Exit("nextest: listing the tests for --filter '<expression>'
  failed (a malformed filterset or a build error); see nextest's output
  above")`. The list call is a filtered run's first command, so it also
  compiles the workspace, and a compile error must not read as a bad
  filter. The exit happens before any
  builder command and before any run directory is created. The list call
  runs with `hide="out"`, so nextest's own parse error reaches the
  terminal.

**File**: `tasks/test/cli.py`

```python
@task(
    help={
        "filter_": (
            "nextest filterset selecting the tests to run, "
            "e.g. 'binary_id(vcs-adapters::classify)'; a filtered run skips "
            "the coverage report, sweep and floor"
        )
    }
)
def run(context: Context, filter_: str | None = None) -> None: ...
```

The session is opened with `filtered=selection.is_filtered`. With no family
to share, it runs no builder and prints the "building nothing" line.
`run` builds its `Selection` through `selection_from(filter_)` (see "Task
options" below). `_families_to_share(context, selection, build_environment)` returns
`SHARED_FAMILIES` when the run is unfiltered. When it is filtered, it parses the
list call with `families_consumed`.

**File**: `mise.toml` (`[tasks."test:unit:cli"]`, `:334-337`)

```toml
usage = '''
flag "--filter <filterset>" help="nextest filterset selecting the tests to run, e.g. 'binary_id(vcs-adapters::classify)'; a filtered run skips the coverage report, sweep and floor"
'''
run = 'invoke test.cli.run ${usage_filter:+--filter "$usage_filter"}'
```

Check that mise names the variable `usage_filter` after the flag's long name,
not after its value placeholder. If it doesn't, rename the placeholder to
`<filter>`.

Also check whether mise leaves `usage_filter` unset when `--filter` is not
given. If it does, forward with `${usage_filter+--filter "$usage_filter"}`
(set, not non-empty), so `--filter ''` reaches `Selection` and is refused
rather than silently running everything. If mise always sets it, keep
`:+`, and record in "Task options" that an empty `--filter` is
indistinguishable from none.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: a test that `test:unit:cli` declares `--filter` in `usage`
with the same help as the invoke task, and that its `run` forwards
`$usage_filter` to `--filter`.

**File**: `tasks/README.md`
**Changes**: the "Shared VCS fixtures" subsection adds, to what a nextest
bump must re-check, recapturing
`tests/unit/tasks/fixtures/shared-fixtures/nextest-list.json`, because
filtered runs read its schema. And a `### Task options` subsection under
"Conventions" setting out the pattern:
- declare the flag in mise `usage`;
- forward it in the form this phase settles (`${usage_<name>+…}` if mise
  leaves the variable unset when the flag is absent, otherwise
  `${usage_<name>:+…}`). Phases 7 and 8, and the Phase 9 guard, use the
  same form;
- build the `Selection` through `selection_from(filter_)` in
  `tasks/test/filters.py` (§1), which turns `Selection`'s `ValueError` into
  a one-line `Exit`. Every lane does this, and every lane's tests include a
  `filter_=""` case;
- declare it on the invoke task, with a trailing underscore when the name is
  a builtin;
- note that once `usage` is declared, raw arguments stop being appended, and
  that `depends` tasks never receive the flag;
- note that a mise bump means re-checking that `usage_<name>` is still named
  after the flag's long name.

### Success Criteria

#### Automated Verification

- [x] The tests pass:
      `uv run pytest tests/unit/tasks/test_selection.py tests/unit/tasks/test_filters.py tests/unit/tasks/shared/shared_fixtures/test_consumers.py tests/unit/tasks/test_rust.py tests/unit/tasks/test_mise.py -v`
- [x] The build-system checks pass: `mise run build-system:check`
- [x] The full local run passes: `mise run`

#### Manual Verification

These are the matrix increment's acceptance criteria. Record each outcome
under its criterion in the work item.

- [x] A full run's log shows one matrix shared build, an adoption for each of
      the 19 consumers and no private build. Each `hermetic-root` is
      distinct, and none lies inside `<run>/matrix`.
- [x] The two matrix digests match, and a test made to write fails the run
      (Phase 2's check, repeated at this increment).
- [x] `mise run test:unit:cli --filter 'binary_id(vcs-adapters::classify)'`
      builds the matrix alone, and the six `classify` tests adopt it and
      pass.
- [x] Each of these fails before any fixture is built:
      `mise run test:unit:cli --filter 'test(classify'`,
      `mise run test:unit:cli --bogus`, and `mise run test:unit:cli extra`.
      The first shows nextest's own filterset parse error.
- [x] `mise run test:unit:cli --help` lists `--filter`, and says that a
      filtered run skips the coverage report, sweep and floor.
- [ ] A filter whose value contains a space, `&` and `'` reaches nextest
      intact, on macOS and on an ubuntu host or CI leg (where mise's
      `sh -c` is dash).
- [x] Record what `mise run test:unit --filter x` does. `test:unit` is a
      roll-up: it has no `run`, only `depends`, and declares no `--filter`.
      Then settle it:
      - **mise rejects the argument.** Nothing changes. Record the
        behaviour in "Task options".
      - **mise ignores it silently.** Give each of the four roll-ups an
        empty `usage = ''`, and check by hand that `mise run test:unit
        --filter x` now exits 1 before running anything.
      - **An empty `usage` is still ignored on a run-less task.** Record
        in "Task options" that roll-ups ignore extra arguments.

      Record the outcome in the work item, because Phase 9's guard and
      guidance depend on it.
- [x] `--filter 'binary_id(accelerator-vcs::status_log_parity)'` builds no
      family.
- [x] `--filter 'test(clasify)'` fails with `--filter 'test(clasify)'
      selected no tests`, before any fixture is built.
- [x] Run each matrix row directly with no shared location (the root the
      summary prints is unchanged) and no new log.
      Each passes.
      - `cargo nextest run --manifest-path cli/Cargo.toml --workspace --exclude accelerator-visualiser --all-features -E 'binary_id(vcs-adapters::classify)'`
      - the same with `-E` set to each of:
        - `binary_id(vcs-adapters::queries)`
        - `binary_id(vcs-test-support::matrix)`
        - `binary_id(vcs-adapters::scrub) & !test(=status_honours_the_global_excludes_file)`
- [x] Each of these leaves nothing it created under the root the summary
      prints:
      - a passing run;
      - a run with a test made to fail;
      - a run with `ACCELERATOR_SHARED_FIXTURES_FAIL=matrix`.

      Each also keeps its log, with the `git` and `jj` versions on the
      shared-build event, and prints the summary. In each,
      `rg '^\s*Summary \['` over the output finds only nextest's own line.
- [x] The SIGKILL check:
  1. Run A with `ACCELERATOR_SHARED_FIXTURES_HOLD=$A`, then kill it with
     SIGKILL once `$A/held` appears.
  2. Run B with `ACCELERATOR_SHARED_FIXTURES_HOLD=$B`, filtered to
     `binary_id(vcs-adapters::classify)`.
  3. Start run C, filtered to `binary_id(vcs-adapters::classify)`.

  C removes A's directory and keeps B's. B passes once `touch $B/release`.
- [ ] Coverage off, coverage on, and both CI `test-unit` legs
      (`ubuntu-latest`, `macos-latest`) pass. Each prints a summary showing
      the matrix built once.
- [x] An interim measurement, as an early warning and not a gate:
      `mise run measure:build -- --isolate test:unit:cli --host-settings devtools --runs 3 --keep`,
      compared with the baseline as in Phase 6. Record the nextest-phase and
      task-duration medians, and the build-step and digest times from the
      run's own line, in the work item. If the matrix increment saves much
      less than its share of the target, raise it with 0361 before starting
      Phase 4.

---

## Phase 4: Status-log builders write no absolute paths

### Overview

This phase changes the status-log builders so that no state records the
directory it was built in. Every golden stays byte-identical. A copy of the
set then gives a test the same view a private build does. The phase lands
before any sharing, and only changes what a private build writes into git
metadata that the status and log renderers never read.

### Changes Required

#### 1. The path scan (tests first)

**File**: `cli/vcs-test-support/tests/path_scan.rs` (new)
- `files_naming(tree, needle)` returns every regular file under `tree` whose
  bytes contain `needle`, as UTF-8.
- It also matches every other spelling of the needle that the filesystem
  gives it: the needle as given, and its canonical path. The test creates a
  directory and a symlink to it, and passes the symlinked spelling as the
  needle. A file naming the canonical spelling is found, and so is the
  reverse. This covers macOS's `/var` → `/private/var` and `/tmp` →
  `/private/tmp`, and runs on both CI legs.
- It returns nothing for a tree that doesn't name the needle.
- It reports a symlink whose target contains the needle.

**File**: `cli/vcs-test-support/src/path_scan.rs` (new); add `walkdir =
"2.5.0"` to `vcs-test-support`'s dependencies.

#### 2. A path-free private build (tests first)

**File**: `cli/vcs-test-support/tests/status_log_states.rs`

Add `no_state_names_the_directory_it_was_built_in`. It builds the full set
with `States::build_in(tempdir)`. It then asserts that `files_naming` over
each state directory (and over `seed`, `origin.git` and
`jj-secondary-main`) finds nothing. The manifest and the `hermetic-*`
entries are not scanned. This test fails today on `git-ahead`, `git-behind`,
`origin.git` and `adversarial-git`. It is also expected to fail on
`jj-secondary-main`: jj records each operation's command line in its
operation log, and `jj workspace add` is given an absolute path
(`status_log.rs:157-167`). Confirm the red run names it before changing the
builder.

**File**: `cli/vcs-test-support/src/status_log.rs`

- **The bare origin.** `build_ahead_behind_states` sets the URL back to the
  seed with `git remote set-url origin ../seed`, run in `origin.git`.
- **The clones.** Clone `git-ahead` and `git-behind` with
  `-c core.logAllRefUpdates=false`. Then point each at the bare origin
  with `git remote set-url origin ../origin.git`.
- **The push.** `git push` from `seed` still targets the bare origin's
  absolute path. The path is an argument and isn't stored.
- **The adversarial hooks path.** `build_adversarial_state` sets
  `core.hooksPath` to `hostile-hooks`. git resolves that against the
  working tree, so it is the same directory.
- **The secondary jj workspace.** `build_jj_states` runs `jj workspace add
  --quiet ../jj-secondary` from `jj-secondary-main`, so the operation log
  records a relative path. If the red run also names a file in jj's
  workspace store, record the workspace there in a form relative to the
  repository, or with a path jj writes itself, so that the test goes
  green.
- **The manifest.** `Manifest` holds the matrix's `Fixture` rows (key,
  shape, start), while the status-log set records a key and a path per
  state. Widen it so both families write and read their manifest through
  it, keeping the matrix's `fixtures.tsv` format unchanged.
- **The full-set builder.** Mirror the matrix's split (Phase 1's
  `build_with`, Phase 2's `build_in` and `build_for_sharing`). Extract from
  `States::build_or_adopt`:
  - `States::build_with(base, env)`, which builds the full set and the
    adversarial state with the `Hermetic` it is given, and no manifest;
  - `States::build_in(base)`, which roots a `Hermetic` at `base` and calls
    `build_with`;
  - `States::build_for_sharing(base)`, which creates `base` if missing,
    calls `build_in` and writes the manifest through `Manifest`.

  Keep `build_or_adopt` and `states_root` until Phase 5.
- **Its cost.** A `States::build_for_sharing` run is timed on engywook when
  idle, alone and alongside a matrix build, and recorded in the work item.
  That decides, before Phase 5, whether the status-log set's groups need
  building on scoped threads (Performance Considerations).

### Success Criteria

#### Automated Verification

- [ ] The status-log tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support -p accelerator-vcs -p vcs-adapters`
- [ ] Regenerating the goldens changes nothing:
      `REGENERATE_GOLDENS=1 cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-vcs -E 'test(status_and_log_match_every_captured_state)'`,
      then `jj diff --stat cli/vcs-test-support/fixtures` is empty.
- [ ] The cli checks pass: `mise run cli:check`
- [ ] The full local run passes: `mise run`

#### Manual Verification

- [ ] On `ubuntu-latest` (CI's `test-unit` leg), the status-log goldens and
      the new scan test pass.

---

## Phase 5: Status-log sharing

### Overview

This phase adds the copy helper. The build step builds the status-log set
too, and the status-log consumers adopt it or copy from it. The private
sets log their builds. `STATES_ROOT_VARIABLE` is renamed. This completes the
status-log increment.

### Changes Required

#### 1. The copy helper (tests first)

**File**: `cli/vcs-test-support/tests/copies.rs` (new). These tests drive
the copy's mechanics over a small source tree each test writes by hand, so
they run no git or jj and build no fixture set. The git and jj fidelity
tests come in §2, once the workbench can hand them the shared set.

- **Timestamps, contents and modes match.** Every file, directory and
  symlink in the copy has the same mtime as its source, to the nanosecond.
  Contents and modes are equal too, directories included. The source holds
  nested files, a symlink, and a read-only (`0555`) directory holding a
  file. The read-only directory pins the order in which modes are set. The
  test makes it writable again before cleanup. Under root a read-only
  directory stops nothing, so the test checks `rustix::process::geteuid()`
  (`rustix` is already a workspace dependency). As root it prints that the
  ordering is unchecked and asserts only contents, modes and times.
- **A planted path is reported.** A source file names the source tree's
  absolute path. The copy's `named_by` lists that file.
- **A clean copy.** A source that names no path gives an empty `named_by`.

**File**: `cli/vcs-test-support/src/copies.rs` (new); add `filetime =
"0.2.29"` to `vcs-test-support`'s dependencies.

```rust
pub struct TreeCopy {
    pub root: PathBuf,
    pub named_by: Vec<PathBuf>,
}

impl TreeCopy {
    pub fn is_clean(&self) -> bool;
}

pub fn copy_part(
    shared_tree: &Path,
    entries: &[&str],
    destination: &Path,
) -> Result<TreeCopy, Error>;
```

How `copy_part` works:
- It copies each named top-level entry. `std::fs::copy` keeps permissions,
  and clones the file on APFS. Symlinks are recreated, read with
  `symlink_metadata` so a link is never followed.
- It then sets every directory's mode, and every entry's mtime and atime,
  from its source's `symlink_metadata`, in one post-order pass: a
  directory's mode and times are set only after its contents are copied
  and stamped, so a read-only directory never blocks its own copy.
  Symlinks are stamped with `set_symlink_file_times`, so a link's own times
  are set and its target is untouched.
- Last, it runs `files_naming(destination, shared_tree)` and returns the
  result as data in `named_by`. It decides nothing: `writable` records the
  `copy` event with that list and fails when it is not empty. The
  `copies.rs` planted-path and clean cases assert on `named_by`, and the
  `writable` tests assert the failure.
- Consumer tests copy only through `Workbench::writable`. `copy_part` is
  its mechanism, called directly only by `copies.rs` and `copy_fidelity.rs`,
  which the guard enforces.

#### 2. States as a shared family, and copies through the workbench (tests first)

**File**: `cli/vcs-test-support/tests/workbench.rs`

The cheap test family also implements `CopyableFamily`, with one part. The
new tests:
- **With the family handed.** `writable::<F>(part)` copies only the part's
  entries into the workbench and records a `copy` event with an empty
  `named-by` list. It returns the family rebased onto the copy, holding
  only the keys whose paths lie under a copied entry.
- **With nothing handed.** It builds the part privately, and records a
  `private-build` event.
- **A dirty copy.** It records a `copy` event whose `named-by` lists the
  files that name the shared tree, then fails.
- **A private set.** `private_states(build)` builds into the workbench with
  its hermetic environment, and records a `private-build` event under
  `private-sets`. It returns a `Fixture` borrowing the workbench, like every
  other fixture the workbench hands out.
- **One fixture per workbench, extended.** After any earlier fixture, a
  `writable()` or a `private_states()` call on the same workbench fails
  with the "already holds" message, naming both fixtures. So does a
  `shared()` after either of them.

```rust
pub trait CopyableFamily: SharedFamily {
    type Part;
    fn entries(part: &Self::Part, tree: &Path) -> Result<Vec<String>, Error>;
    fn build_part_privately(
        part: &Self::Part,
        directory: &Path,
        hermetic: &Hermetic,
        token: PrivateBuild,
    ) -> Result<Self, Error>;
    fn rebased(self, from: &Path, onto: &Path, entries: &[String]) -> Self;
}

impl Workbench {
    pub fn writable<F: CopyableFamily>(&self, part: F::Part) -> Result<Fixture<'_, F>, Error>;
}
```

**File**: `cli/vcs-test-support/src/fixture_log.rs`
**Changes**: Add `Event::Copy { family, test, hermetic_root, copy, named_by }`,
where `named_by` is the list of files the path scan found (empty for a clean
copy). Add its line to `fixture-log.jsonl`.

**File**: `cli/vcs-test-support/src/family.rs`
**Changes**: `Family` gains `StatusLog`. `Family::StatusLog.handover_variable()`
is `ACCELERATOR_SHARED_STATUS_LOG`, and `variables.json` gains its entry.

**File**: `cli/vcs-test-support/src/status_log.rs`
- Remove `STATES_ROOT_VARIABLE`, `states_root` and `build_or_adopt`. Make
  `adopt` public; it fails on a missing manifest, a malformed line or an
  empty manifest, naming the base. `tests/status_log_sharing.rs` (new)
  covers each case with a hand-written manifest and no git, as
  `matrix_sharing.rs` does for the matrix.
- Add these:

  ```rust
  pub const ADVERSARIAL_STATE: &str = "adversarial-git";

  pub enum StatusLogPart {
      Everything,
      PlainGit,
      Conflicts,
  }

  impl StatusLogPart {
      pub fn entries(&self, tree: &Path) -> Result<Vec<String>, Error>;
      pub(crate) fn build_privately(&self, directory: &Path, hermetic: &Hermetic) -> Result<States, Error>;
  }
  ```

  The part logic lives here. `impl CopyableFamily for States` delegates to
  these two methods in one line each, so there is one place to change a
  part. `build_privately` is `pub(crate)`, so it is reachable only through
  the sealed `build_part_privately`.

  ```rust
  impl States {
      pub fn with_goldens(&self) -> impl Iterator<Item = (&str, &Path)>;
      pub fn get(&self, key: &str) -> Result<&Path, Error>;
  }
  ```

- **What each part covers.**
  - `Everything`: every top-level entry except the manifest and the
    `hermetic-*` entries.
  - `PlainGit`: `clean-git`, `dirty-git` and `detached-head-git`.
  - `Conflicts`: `conflict-git` and `conflict-jj`, built by
    `build_conflict_states`, which `StatusLogPart` reaches inside the crate.
- `with_goldens` iterates every state except `ADVERSARIAL_STATE`.
- `impl SharedFamily for States` adopts, or builds privately with
  `States::build_with`.
- `impl CopyableFamily for States` has `Part = StatusLogPart`.

**File**: `cli/vcs-test-support/tests/copy_fidelity.rs` (new). A
status-log consumer. These tests check `copy_part`, the mechanism under
`writable`, against real states, so they call it directly. Each test adopts
the set in one workbench, `source = Workbench::for_this_test()` with
`source.status_log_states()`, and copies into a second workbench,
`target`, so a copy never lands on the set it came from. Under the build
step the source is the shared tree; run directly, it is a private build in
`source`'s directory. The adoptions are logged; the direct copies are not,
because only `writable` records a `copy` event.

- **Timestamps match on real states.** A `copy_part` of every state into
  `target` has the same mtimes as the source, to the nanosecond, including
  each jj state's `.jj/working_copy/tree_state`.
- **The copy's status matches its source.** The commands are
  `git --no-optional-locks status --porcelain` in each git state, and
  `jj --ignore-working-copy status` in each jj state. They give the same
  output in the copy as in the source.
- **A snapshot in a copy sees exactly the edits the builder made.** Three
  jj states (`dirty-jj`, `deleted-jj`, `rename-jj`) carry edits that were
  never snapshotted, so any snapshot rewrites `@` with a fresh commit id,
  and their `jj status` output cannot be compared byte for byte. Instead,
  the test copies the whole `Everything` part into `target`, so that
  `jj-secondary` keeps `jj-secondary-main`, its store. It then snapshots
  each jj state with `jj diff --summary -r @`, and compares the summary with
  the edits the builder is known to have made:
  - `dirty-jj`: `A new-file.txt`;
  - `deleted-jj`: `D gone.txt`;
  - `rename-jj`: `D old.txt` and `A new.txt`;
  - every other jj state: empty, and no new operation recorded.

  A dropped or truncated file, or a broken store pointer, in the copy shows
  up as a summary that differs from these expectations.
- **A planted path fails the copy of a real state.** A third workbench
  takes `writable_status_log_states(Conflicts)` as a source it may change,
  plants that copy's own path in its `conflict-git/.git/config`, then copies
  it into a fourth with `copy_part`. The second copy's `named_by` names the
  file.

**File**: `cli/vcs-test-support/tests/status_log_parts.rs` (new). A
status-log consumer.

The private path of each part runs in no task-driven consumer, because under
the build step every consumer adopts or copies. These tests run it. The
private builds use `Workbench::with(Handover { log: FixtureLog::at(<test-local
file>), ..Handover::default() })`, so they never reach the run's log.
- For `PlainGit` and `Conflicts`, `writable_status_log_states(part)` with
  nothing handed builds the part privately. Its keys equal the part's `entries()`
  over the set adopted by a second workbench, `Workbench::for_this_test()`,
  and every state it names exists.
- `writable_status_log_states(part)` on a third `Workbench::for_this_test()`
  gives exactly the same keys as the private build of the same part. Each
  workbench holds one fixture.
- The test-local log holds one `private-build` event per part.

`Everything` and the matrix need no such test. Their private paths are
`States::build_with` and `Matrix::build_with` over the workbench's
`Hermetic`, and the shared build runs those same functions through
`build_in`. Only the one-line delegation in each `build_privately` is left
to the direct-run manual checks.

**File**: `cli/vcs-test-support/src/workbench.rs`

```rust
impl Workbench {
    pub fn status_log_states(&self) -> Result<Fixture<'_, States>, Error>;
    pub fn writable_status_log_states(&self, part: StatusLogPart) -> Result<Fixture<'_, States>, Error>;
    pub fn private_states(
        &self,
        build: fn(&Path, &Hermetic) -> Result<BTreeMap<&'static str, PathBuf>, Error>,
    ) -> Result<Fixture<'_, BTreeMap<&'static str, PathBuf>>, Error>;
}
```

`writable_status_log_states(part)` is `writable::<States>(part)`. Consumer
tests use the named methods (`matrix()`, `status_log_states()`,
`writable_status_log_states()`). The generic `shared::<F>()` and
`writable::<F>()` stay public only because the cheap family in
`workbench.rs` needs them. A copy goes
straight into the workbench's own directory, next to its `hermetic-*`
entries. That reproduces today's layout, where the hermetic
root is the states' parent and is their `GIT_CEILING_DIRECTORIES`.

**File**: `cli/vcs-test-support/src/status_log.rs`
**Changes**: Once §3 has moved every consumer, make `States::build_in`,
`States::build_with`, `States::build_for_sharing`, `build_states`,
`build_plain_git_states`, `build_conflict_states` and
`build_adversarial_state` `pub(crate)`. Only the workbench, `StatusLogPart`
and the shared build call them, so the compiler stops any test building
part of the status-log set behind the log's back. The private-set builders
(`build_cap_states`, `build_bookmark_states`, `build_parity_states`) stay
`pub`, for `private_states`. The Phase 4 path test, which builds with
`States::build_in`, moves onto `status_log_states()` in §3 first.

**File**: `tests/unit/tasks/test_fixture_builds.py`
**Changes**: Phase 1 created this guard (see Phase 1 §4) for the bypasses
the compiler cannot see. It now also:
- allows `copy_part(` only in `copies.rs` and `copy_fidelity.rs`, and
  `Workbench::with(` in `status_log_parts.rs`;
- fails on a *call* to a private-set builder (`build_cap_states(`,
  `build_bookmark_states(`, `build_parity_states(`) anywhere in
  `cli/*/tests/**`. Passing the function by name to `private_states` is
  allowed, so every private-set build goes through the workbench and
  appears in the summary's "private sets" count.

Each new pattern gets a case in the guard's own `tmp_path` tests.

#### 3. Move the status-log consumers and the private sets

| Test | Today | After |
|---|---|---|
| `status_log_goldens::status_and_log_match_every_captured_state` | `build_states` | `writable_status_log_states(Everything)`, iterating `with_goldens()` |
| `…::a_conflict_status_carries_the_conflicted_marker_and_path` | `build_states` | `writable_status_log_states(Everything)` |
| `…::a_rename_renders_as_deleted_old_plus_added_new` | `build_states` | `writable_status_log_states(Everything)` |
| `…::an_empty_history_renders_no_commits` | `build_states` | `writable_status_log_states(Everything)` |
| `…::a_malformed_accelerator_log_still_renders_and_exits_zero` | `build_plain_git_states` | `writable_status_log_states(PlainGit)` |
| `…::fail_safe_has_no_effect_on_a_successful_status_or_log` | `build_plain_git_states` | `writable_status_log_states(PlainGit)` |
| `…::the_log_is_capped_at_five_entries` | `build_cap_states` | `private_states(build_cap_states)` |
| `…::a_jj_bookmark_header_lists_the_byte_sorted_bookmarks` | `build_bookmark_states` | `private_states(build_bookmark_states)` |
| `status_log_parity` (both tests) | `build_parity_states`, `build_cap_states` | `private_states(…)` |
| `conflicts::real_merge_conflicts_are_reported_conflicted` | `build_states` | `writable_status_log_states(Conflicts)`; oracles run with `workbench.hermetic()` |
| `status_log_states::every_status_log_state_is_built` | `states_root` + `build_or_adopt` | `status_log_states()` |
| `status_log_states::no_state_names_the_directory_it_was_built_in` | `States::build_in` | `status_log_states()`, scanning the adopted tree for its own base |

Everywhere `env` was `Hermetic::rooted_at(work)`, it becomes
`workbench.hermetic()`.

**Assertion map:**
- **Unchanged.** Every assertion stays in its function. The goldens, their
  names and the 38 files are unchanged. The test count is unchanged except
  for the one test Phase 4 added, recorded in the work-item amendment.
- **The full-set iteration.** It covers the same 19 keys through
  `with_goldens()`, so the same 38 comparisons run.
  `status_and_log_match_every_captured_state` gains one assertion that
  enforces this: the set of `<state>-<subcommand>` names it compared equals
  the set of full-set golden files on disk. The private-set goldens are
  excluded by name. Write that assertion first, against today's
  `build_states`, so it passes before the move and guards it.
- **`conflicts`.** It keeps both oracles and both `assert_eq!`s.
- **`status_log_states`.** It keeps all four key assertions, and so still
  finds `adversarial-git`.

#### 4. The build step builds the status-log set

**File**: `cli/vcs-test-support/src/shared_build.rs` and its tests
**Changes**: `Family::StatusLog` brings the `--status-log <dir>` flag with
it, through new arms in `Family::flag`, `name`, `handover_variable` and
`Display`. Both families build on their own threads, the status-log set using
`States::build_for_sharing`. The bin's forced failure accepts `status-log`,
and its refusal of an unknown value lists both families.

**Files**: `tasks/shared/shared_fixtures/` and its tests
- `Family` gains `STATUS_LOG = "status-log"`, with entries in its
  `handover_variable`, `heading` (`status-log set`) and `noun` (`the
  status-log set`) maps, so `SHARED_FAMILIES` is both families.
- `CONSUMERS` gains `vcs-adapters::conflicts`,
  `vcs-test-support::status_log_states`,
  `vcs-test-support::copy_fidelity`, `vcs-test-support::status_log_parts`
  and `accelerator-vcs::status_log_goldens` for `Family.STATUS_LOG`.
- The captured `nextest-list.json` grows to include `status_log_goldens`,
  `conflicts` and `status_log_states`. `families_consumed` gains these
  cases:
  - only `the_log_is_capped_at_five_entries` (a private-set test in a
    consumer binary) gives the status-log set;
  - only the `accelerator-vcs::status_log_parity` binary gives no family;
  - only the `vcs-adapters::conflicts` binary gives the status-log set
    alone;
  - one `classify` test plus `conflicts` gives both families.
- The summary counts copies, and `private-sets` private builds.
- `problems` also names each `copy` event whose `named-by` list is not
  empty, with the files it names. A private build under `private-sets` is
  not a problem.

**File**: `tests/unit/tasks/test_rust.py`
**Changes**: The builder call carries `--matrix` and `--status-log`, and the
hand-over carries both tree variables. A filtered run whose selection
consumes only the status-log set passes only `--status-log`.

**File**: `tasks/README.md`
**Changes**: The "Shared VCS fixtures" subsection covers copies, the path
scan and the private sets.

### Success Criteria

#### Automated Verification

- [ ] The Rust tests pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support -p accelerator-vcs -p vcs-adapters`
- [ ] The Python tests pass:
      `uv run pytest tests/unit/tasks/shared/shared_fixtures tests/unit/tasks/test_rust.py tests/unit/tasks/test_fixture_builds.py -v`
- [ ] Nothing in `cli/`, `tasks/`, `mise.toml` or `.github/` mentions the
      old status-log variable:
      `! rg -n 'ACCELERATOR_ZERO_SPAWN_STATUS_LOG|STATES_ROOT_VARIABLE' cli tasks mise.toml .github`
- [ ] The goldens are unchanged: `jj diff --stat cli/vcs-test-support/fixtures`
      is empty.
- [ ] The checks pass: `mise run cli:check && mise run build-system:check`
- [ ] The full local run passes: `mise run`

#### Manual Verification

These are the status-log increment's acceptance criteria. Record each outcome
in the work item, along with the audit table's final outcomes.

- [ ] A full run's log shows the following, with a distinct `hermetic-root`
      for each test using the set, and none inside a shared tree:
  - one status-log shared build;
  - six copies for the goldens and one for `conflicts`, plus one from
    `copy_fidelity`'s planted-path test and one per `status_log_parts`
    part test, each with a clean scan (`copy_fidelity`'s direct
    `copy_part` calls record no event);
  - two adoptions for `status_log_states`, plus one for each
    `copy_fidelity` test and one per `status_log_parts` part test;
  - no private build of the status-log set (the part tests' private builds
    go to their own test-local logs);
  - four private builds under "private sets".
- [ ] The status-log digests match.
- [ ] A temporary edit that writes into the adopted tree from
      `status_log_states` fails the run, and the failure names the tree.
- [ ] A temporary edit planting the shared path in the shared tree's
      `conflict-git/.git/config`, made right after the build through the
      hold switch, fails the run naming the copy.
- [ ] `--filter 'binary_id(vcs-adapters::conflicts)'` builds the status-log
      set alone, and `conflicts` copies its two states and passes.
- [ ] `--filter 'binary_id(vcs-adapters::classify)'` does not build the
      status-log set.
- [ ] Each status-log row run directly through `cargo nextest run -E …`
      passes, with no shared location created and no log written.
- [ ] `ACCELERATOR_SHARED_FIXTURES_FAIL=status-log` stops the run before the
      test phase. There is one failure naming the status-log set, and no
      status-log consumer event.
- [ ] The cleanup, summary and `Summary [` checks from Phase 3 hold for the
      status-log set.
- [ ] The SIGKILL check from Phase 3 holds for the status-log set.
- [ ] Coverage off, coverage on, and both CI `test-unit` legs pass. Each
      summary shows both families built once.
- [ ] Time a filtered run selecting only
      `test(=the_log_is_capped_at_five_entries)`, which builds the
      status-log set although its test doesn't use it. Record the cost in
      the work item (Performance Considerations).

---

## Phase 6: Fixtures closure

### Overview

There are no code changes. This phase records the after-measurement and
checks the work item's two `[fixtures closure]` criteria. Meeting them
releases 0391 and gives 0361 its result.

### Steps

1. Confirm the measurement conditions:
   - engywook is on `devtools`-only host settings;
   - no other measurement or validation session, such as 0364's or 0370's,
     is running;
   - no change affecting `test:unit:cli` (0366, for example) has landed on
     `main` since `287b6279`.

   If such a change has landed, take a fresh before-record at current
   `main` with this item's changes reverted, and capture its nextest
   `Summary [` line.
2. `mise run measure:build -- --isolate test:unit:cli --host-settings devtools --runs 3 --keep`
3. `mise run measure:compare-builds -- meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json <after> --task test:unit:cli`

### Success Criteria

#### Manual Verification

- [ ] The after-record's nextest summary line reports at least 7,727 tests
      run (or the fresh before-record's count).
- [ ] The comparison's first line reports no differing condition.
- [ ] The nextest-phase median is ≤ 156.9 s, and the task-duration median is
      ≤ 167.8 s, both "beyond" noise (or 25 s under a fresh before-record).
- [ ] In the after-record, the cargo units show nothing compiled after the
      builder runs, in the nextest step, so the listing and nextest still
      fingerprint the same build.
- [ ] If the task-duration median misses while the nextest phase meets its
      target, the build-step and digest times on the run's own line show
      where the serial overhead went. Record them.
- [ ] The figures are recorded in the work item, and 0391's blocking edge is
      released.

---

## Phase 7: pytest lanes

### Overview

Before this increment starts, write the in-scope table below into the work
item (Requirement 5). Then give every pytest-driven leaf a `-k` filter. The
four bare `uv run pytest` leaves get invoke tasks so that each declares
`filter_` the same way. Rebase onto 0364's `tasks/test/integration.py`
changes first, and keep out of 0364's CI measurement windows.

### In-scope tasks (written into the work item)

| Task | Runner | Filter syntax | Example | Malformed example | Runner when nothing matches |
|---|---|---|---|---|---|
| `test:unit:cli` | nextest 0.9.138 | filterset (`-E`) | `binary_id(vcs-adapters::classify)` | `test(foo` | fails ("no tests to run"); the task's list sees it first |
| `test:unit:tasks`, `test:unit:frontend-licenses`, `test:unit:vcs`, `test:integration:tasks` | pytest 9.0.3 | `-k` expression | `resume and not slow` | `foo and` | exit 5 |
| `test:integration:dev`, `:entrypoint`, `:evals`, `:pup`, `:hooks`, `:research`, `:research-exhaustive` | pytest 9.0.3 | `-k` expression | `resume and not slow` | `foo and` | exit 5 |
| `test:unit:visualiser`, `test:integration:visualiser` | libtest (cargo test) | substring of the test path | `path_normalisation` | none: any string is valid | exit 0, `0 passed` in every binary |
| `test:integration:tracker-contract` | nextest 0.9.138 | filterset (`-E`), intersected with `binary(=contract)` | a `test(…)` naming one contract test | `test(foo` | fails ("no tests to run") |
| `test:unit:frontend` | Vitest 3.2.4 | test-name regex (`-t`) | a regex naming one `describe` block | `(` | to confirm in Phase 8 |
| `test:unit:design-automation`, `test:integration:design-automation` | `node --test` (node 22.22.2) | test-name regex (`--test-name-pattern`) | a regex naming one test | `(` | exit 0, `# pass 0` |
| `test:e2e:visualiser`, `test:e2e:visualiser:docker`, `test:e2e:visualiser:docker:update` | Playwright 1.59.1 | title regex (`--grep`) | a regex naming one spec's title | `(` | fails ("No tests found") |

Whatever the runner does, every lane fails with `Selection.nothing_selected(task)`
(`--filter '<expression>' selected no tests`) when a filter runs no test.
The last column records what each lane has to translate or detect.

The malformed examples for Vitest, node and Playwright are inferred. Phase 8's
first step confirms each one fails. If a runner accepts its example, the
table records that its syntax has no malformed form. Where the example is
described rather than given, the lane's phase picks a real name from its
suite, and checks by hand that it selects at least one test. Each lane's
help carries its example.

### Changes Required

#### 1. Selection for pytest (tests first)

**File**: `tests/unit/tasks/test_selection.py`
**Changes**: `Selection("a and not b").pytest()` is `"-k 'a and not b'"`;
with no filter it is `""`.

#### 2. Invoke tasks (tests first)

**File**: `tests/unit/tasks/test_pytest_lanes.py` (new)

The tests are parametrised over every pytest lane's invoke task:
- **The six existing lanes:** `integration.dev`, `entrypoint`, `evals`,
  `pup`, `hooks` and `research`/`research_exhaustive`.
- **The four new tasks:** `unit.build_system` (named `tasks`),
  `unit.frontend_licenses`, `unit.vcs` and `integration.build_system`
  (named `tasks`).

What they check:
- With `filter_="x and y"`, the run command ends with `-k 'x and y'`.
- With a filter, a pytest exit of 5 raises
  `Exit(Selection("x and y").nothing_selected(<task>))`, the full shared
  message with its pointer to `--help`. Any other non-zero exit is reported
  as it is today.
  Unfiltered, exit 5 is reported as it is today.
- Without a filter, an existing lane's command is unchanged. Each of the
  four new tasks runs exactly the command its `mise.toml` `run` holds
  today, including its `-v` and its working directory.
- Each task's `help` names "pytest -k expression".
- `filter_=""` exits with `--filter needs an expression` and runs nothing.
- The no-match message names the lane's mise task (`test:unit:tasks`,
  `test:integration:research-exhaustive`, …), checked exactly for each
  lane.
- `test_dev_integration_task.py` (`-n auto`) and
  `test_hooks_task_environment.py` (no `env`) still pass.

**Files**: `tasks/test/unit.py`, `tasks/test/integration.py`
**Changes**: Each lane takes `filter_: str | None = None` and adds
`selection_from(filter_).pytest()` to its command. The new tasks run the commands currently
written in `mise.toml` (`uv run pytest tests/unit/tasks -v`, and so on).

**File**: `mise.toml`
- The four bare tasks become `run = 'invoke test.unit.tasks <forwarding>'`
  and so on, where `<forwarding>` is the form Phase 3 settled.
- Every pytest lane declares, in the same multi-line literal form as
  Phase 3 so the example keeps its shell quotes:

  ```toml
  usage = '''
  flag "--filter <expression>" help="pytest -k expression selecting the tests to run, e.g. 'resume and not slow'"
  '''
  ```

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: Each pytest lane declares `--filter` in `usage` and forwards it.
The test compares the whole help string, quotes included, and the file still
parses as TOML.

### Success Criteria

#### Automated Verification

- [ ] The tests pass:
      `uv run pytest tests/unit/tasks/test_selection.py tests/unit/tasks/test_pytest_lanes.py tests/unit/tasks/test_mise.py tests/unit/tasks/test_dev_integration_task.py tests/unit/tasks/test_hooks_task_environment.py -v`
- [ ] The build-system checks pass: `mise run build-system:check`
- [ ] The full local run passes: `mise run`

#### Manual Verification

For each pytest lane:

- [ ] `mise run <task> --help` lists `--filter` and names pytest's `-k`.
- [ ] `mise run <task> --filter '<a name in the suite>'` runs fewer tests
      than the bare task, and all of them pass.
- [ ] `mise run <task> --filter 'foo and'` fails (pytest exit 4).
- [ ] `mise run <task> --filter 'no_such_test_xyz'` fails with `selected no
      tests`.

---

## Phase 8: cargo and node lanes

### Overview

`--filter` reaches the remaining lanes in each runner's own syntax. The
design-automation lanes skip their pass-count and skip floors when filtered.
The Docker Playwright command quotes the pattern for its inner `bash -c`.

### Changes Required

#### 0. Confirm the malformed examples

Run each runner by hand with its example from the Phase 7 table, and record
the outcome in the work item's table:
- `npx vitest run -t '('`
- `node --test --test-name-pattern '(' …`
- `npx playwright test --grep '('`

Also run `npx vitest run -t 'no_such_test_xyz'` and record its exit status
and summary line in the "nothing matches" column. That decides how
`test.unit.frontend` detects it (below).

#### 1. Selection for each runner (tests first)

**File**: `tests/unit/tasks/test_selection.py`

| Method | Renders | Unfiltered |
|---|---|---|
| `libtest()` | `"'name'"` (cargo test's positional filter) | `""` |
| `vitest()` | `"-t 'name'"` | `""` |
| `node_test()` | `"--test-name-pattern 'name'"` | `""` |
| `playwright()` | `"--grep 'name'"` | `""` |

The tests also cover a value containing `'` and a space for each method. The
lanes add npm's `--` separator themselves, and the Docker payload uses
`playwright()` as it is.

#### 2. The lanes (tests first)

**File**: `tests/unit/tasks/test_cargo_and_node_lanes.py` (new)

- **`test.unit.visualiser`.** With `filter_="path_normalisation"`, both
  `cargo test` runs end with the filter, and without one they are unchanged.
- **The libtest lanes, matching nothing.** The `test result:` lines across
  every binary are summed (`N passed; M failed; K ignored`). When a filtered
  run's total is zero, the lane raises `Exit(selection.nothing_selected(<task>))`.
  A total above zero passes, and an unfiltered run is never checked. The
  test feeds captured output with all-zero and mixed results.
- **`test.integration.visualiser`.** The filter follows `--test '*'`.
- **`test.integration.tracker-contract`.** The command gains
  `-E '<filter>'`, and still uses `--profile contract`. `_MANIFEST` is
  unchanged. With a filter, nextest's documented exit code for no tests
  run (4, `NO_TESTS_RUN`) becomes `Exit(selection.nothing_selected(<task>))`.
  The test feeds exit 4, and any other non-zero exit is reported as today.
- **`test.unit.frontend`.** `npm --prefix … run test -- -t '<filter>'`.
  With a filter, a run that executes no test raises
  `Exit(selection.nothing_selected(<task>))`. Step 0's finding decides the
  mechanism: if Vitest already fails, the lane translates that failure;
  if it exits 0, the lane reads the test count from Vitest's summary line,
  or adds `--reporter=json` if the summary line can't be relied on.
- **The design-automation lanes, filtered.** `node --test` gains
  `--test-name-pattern`, placed before the first test file, because node
  reads every argument after the first file as another file. The test
  asserts that order. A TAP result of `# pass 3`, `# fail 0` and
  `# skipped 102` passes, and one line is printed:
  `design-automation: pass-count and skip floors skipped for a filtered run`.
  `# fail 1` still fails. `# pass 0` with `# fail 0` raises
  `Exit(selection.nothing_selected(<task>))`.
- **The design-automation lanes, unfiltered.** The floors still apply.
- **Every lane.** `filter_=""` exits with `--filter needs an expression` and
  runs nothing. Each lane's no-match message names its own mise task,
  including `test:e2e:visualiser:docker` versus
  `test:e2e:visualiser:docker:update`.
- **`test.e2e.visualiser`.** `npm --prefix … run test:e2e -- --grep
  '<filter>'`. With a filter, Playwright's "No tests found" failure becomes
  `Exit(selection.nothing_selected(<task>))`, on the host and in the Docker
  lanes. Playwright documents no exit code for this, so the lane matches the
  text; the README lists it among the text-matched lanes.

**File**: `tests/unit/tasks/test_e2e_docker.py`
- `docker_visual_command(..., grep="a b's")` gives a payload ending
  `--project visual-regression --grep 'a b'"'"'s'`. Splitting it with
  `shlex.split` the way the inner `bash -c` does gives `--grep`, `a b's`.
- With `update=True`, the payload carries both `--update-snapshots` and the
  grep.
- With no grep, the existing exact-payload assertion (`:103-111`) still
  holds.

**Files**:
- `tasks/test/unit.py`
- `tasks/test/integration.py`
- `tasks/test/e2e.py` (`visualiser`, `visualiser_docker`,
  `docker_visual_command`)

**Changes**: Each task takes `filter_`, builds its `Selection` through
`selection_from(filter_)`, and places its rendering
where its runner reads options: after `--test '*'` for the visualiser
integration lane, after npm's `--` for Vitest and Playwright, and before the
discovered files for `node --test`. `docker_visual_command` gains
`grep: str | None = None`.

**File**: `mise.toml`
- Each lane declares `usage` with help naming its syntax: "libtest
  substring", "nextest filterset", "Vitest -t test-name regex", "node
  --test-name-pattern regex" or "Playwright --grep regex". It also gives the
  lane's example from the Phase 7 table, as `e.g. '…'`.
- Each lane forwards the flag.
- `test:e2e:visualiser:docker:update` becomes
  `run = 'invoke test.e2e.visualiser-docker --update <forwarding>'`, in the
  form Phase 3 settled.

### Success Criteria

#### Automated Verification

- [ ] The tests pass:
      `uv run pytest tests/unit/tasks/test_selection.py tests/unit/tasks/test_cargo_and_node_lanes.py tests/unit/tasks/test_e2e_docker.py tests/unit/tasks/test_mise.py -v`
- [ ] The build-system checks pass: `mise run build-system:check`
- [ ] The full local run passes: `mise run`

#### Manual Verification

For each lane in the Phase 7 table, except `tracker-contract`, which needs
credentials. Check its rendering in the unit tests, and run it by hand only
where credentials are available:

- [ ] `--help` lists `--filter` and names the syntax.
- [ ] A subset filter runs fewer tests, and all of them pass. The Docker
      lanes are checked on a host with Docker; `:update` is checked with a
      filter that selects one snapshot, and its result is reverted.
- [ ] Each lane's malformed example fails, or the work item's table records
      that its syntax has no malformed form.
- [ ] Each lane, given a filter that matches nothing, fails with `selected
      no tests`.

---

## Phase 9: Guidance

### Overview

Rewrite the guidance so that `--filter` is the supported way to run a subset
of tests, and add a guard that every in-scope leaf keeps the option.

### Changes Required

#### 1. A guard over the task tree (tests first)

**File**: `tests/unit/tasks/test_mise.py`

`test_every_multi_test_leaf_declares_a_filter` checks every leaf `test:*`
task, except:
- the six exempt guards;
- the two single-test lanes;
- the four roll-ups (`test:unit`, `test:integration`, `test:e2e`, `test`).

Each one checked must declare `flag "--filter` in `usage`, with help
containing `e.g.`, and forward `$usage_filter`. The exempt set is spelled
out in the test, so adding a task forces a decision.

If Phase 3 gave the roll-ups an empty `usage`, a second test,
`test_every_roll_up_refuses_a_filter`, asserts that each roll-up declares
`usage` and that the `usage` has no `--filter`.

#### 2. Guidance

**File**: `CLAUDE.md`, the `### Running a single test` section. It becomes
"Running a subset of tests", which says:
- every multi-test `mise run test:*` leaf takes `--filter`, in its runner's
  syntax (`mise run <task> --help` names it);
- which tasks are exempt;
- that filters apply only to leaf tasks, and what a roll-up does with
  `--filter` (rejects it, or ignores it, as Phase 3 found);
- that plain `cargo test` is not a supported way to run the tests
  `test:unit:cli` runs (the `cli/` workspace apart from the visualiser),
  because it neither builds the shared fixtures nor matches nextest's
  per-test isolation. The visualiser lanes run `cargo test` themselves, and
  that is unaffected;
- that running an underlying runner directly is a last resort. For `cli/`,
  `cargo nextest run` directly still passes, building fixtures per test.

**File**: `tasks/README.md`
**Changes**: A `### Running a subset of tests` subsection containing:
- the Phase 7 table: task, syntax, example;
- the exempt tasks;
- the rule that a filter which runs no test fails the lane, and which lanes
  detect it from runner output text rather than an exit code (libtest's
  `test result:` lines, node's TAP `# pass`, Playwright's "No tests found",
  and Vitest's summary if Phase 8 finds it needs one), so a runner bump
  knows to re-check them;
- the `cargo test` rule;
- the last-resort note.

Link to it from "Shared VCS fixtures" and from "Task options". Point the
"Coverage floor" procedure's hand-run steps at `--filter`, where it applies.

### Success Criteria

#### Automated Verification

- [ ] The guard passes: `uv run pytest tests/unit/tasks/test_mise.py -v`
- [ ] The full local run passes: `mise run`

#### Manual Verification

- [ ] Read `CLAUDE.md` and `tasks/README.md`. Together they direct readers
      to each task's `--filter`, including `test:unit:cli`'s; name the
      exempt tasks; rule out `cargo test`; and present direct runners only
      as a last resort.
- [ ] Every filters-increment criterion in the work item is ticked, and the
      work item can close.

---

## Testing Strategy

### Unit Tests

- **Rust (`vcs-test-support`):**
  - the fixture log (format against the shared JSONL file, appending under
    concurrency, disabled mode);
  - the test's name;
  - the workbench (where it lives, its hermetic root, adopting versus
    building privately, failing closed, canonical hand-over paths,
    cleanup), exercised with a cheap test family so that no git runs;
  - the manifest round trip;
  - the shared-build request, including a real build error and a panicking
    build; the bin, including the forced failure and an unknown value;
  - the path scan, over a symlinked spelling of the needle;
  - copy fidelity (mtimes, modes, symlinks, `git` and `jj` status, a
    snapshotting `jj status`) and the planted-path refusal;
  - the private path of each status-log part.
- **Python:**
  - run directories, locks and the sweep, using real child processes with a
    readiness handshake, SIGKILL, and each ambiguous-liveness case;
  - tree digests;
  - finding the builder among a bin and a harness-shaped artifact;
  - the shared log format and variable names, against the Rust side;
  - consumer selection over a captured `nextest list`;
  - the summary format against the timeline regexes;
  - finding problems;
  - pruning logs;
  - the hold point;
  - every lane's command rendering, through `Selection`;
  - the floor-skipping branches;
  - the Docker payload quoting.

### Integration Tests

- **The consumers.** The existing VCS consumers act as the integration tests
  of adopting and copying. Each passes under the build step, and when run
  directly.
- **A guard in every run.** The private-build check and the digests act as
  a permanent integration guard on every `test:unit:cli` run.
- **The goldens' own count.** The full-set goldens test asserts that it
  compared every full-set golden file, so a dropped state fails it.

### Manual Testing Steps

These are listed per phase. The ones that can't be automated cheaply are:
- the SIGKILL check with three runs;
- the test-made-to-write and planted-path edits;
- the CI legs;
- the measurement in Phase 6.

## Performance Considerations

- **What it removes.**
  - Eighteen matrix builds per run: about 730 test-seconds, with the
    matrix tests at 35–47 s each under load.
  - Five full and two plain-git status-log builds: about 140 test-seconds.
- **What it adds.** Everything here is serial with the test phase, which is
  why the speed criterion bounds task duration as well as the nextest
  phase.
  - One concurrent shared build before nextest. The matrix alone takes
    3.3 s wall (1.9 CPU-s) on an idle host. Phase 4 times the status-log
    set's build alone and alongside the matrix.
  - Two digests of each tree, before and after the tests.
  - nextest's own no-op cargo fingerprint pass.
  - Under coverage, the sweep's own unit listing after the report, also a
    no-op. It stays, because it carries the existing `rebuilt` guard that
    the sweep's environment still matches the instrumented build.
  - Removing the run directory, and reading the log for the summary.
  - Status-log copies inside the tests: seven for the consumers, plus a few
    for the copy-fidelity and part tests. They are cheap on both filesystems:
    a clonefile on APFS, and a few hundred small files on ext4.
  - Two small private builds in every run, from `status_log_parts`, which
    exercises the private path of `PlainGit` and `Conflicts` on purpose:
    three plain git repositories, a conflicted merge and one jj repository.
    They run in parallel with other tests. If Phase 3's measurement shows
    `status_log_parts` among the slowest tests, its two part tests can share
    one adopted source.
- **Running the new consumers directly costs more.** With nothing handed
  over, each `copy_fidelity` test's source workbench and each
  `status_log_parts` second workbench builds the full status-log set
  privately, about five full builds for those two binaries. That only
  affects `cargo nextest run` run directly, which the guidance already
  presents as a last resort.

  **The budget.** The build step, both digests and cleanup together should
  take at most about 5 s on engywook when idle. The two no-op cargo passes
  are measured separately. Phase 3's interim measurement records how long
  the nextest step takes before its first test starts, and how long the
  sweep's listing takes. The run prints its build
  and first-digest times after the build, and its second-digest and
  removal times after cleanup. Phase 3's interim measurement and Phase 6
  record them.

  If they go over budget, two optimisations are ready, to be taken only if
  measurement shows they are needed:
  - each family's independent groups (the matrix's shape groups,
    `fixtures.rs:172-180`, and the status-log set's seven groups,
    `status_log.rs:302-308`) can build on scoped threads inside
    `build_for_sharing`;
  - the families' digests can run concurrently on a small thread pool.
- **Over-building on some filtered runs.** Because consumption is recorded
  per binary, a filter that selects only a private-set test in
  `status_log_goldens`, or only the jj-binary test in `queries`, builds that
  family anyway. Phase 5's interim checks time one such run. If the
  status-log build costs several seconds, "Running a subset of tests" notes
  that `& !binary_id(...)` avoids it.
- **The compile is not repeated.** The unit listing moves before nextest and
  compiles what nextest would otherwise have compiled. nextest's own build
  then finds everything fresh. Phase 2's manual checks confirm that no
  `Compiling` line follows the builder, with coverage on and off. In every
  lane, filtered or not, a cargo `Compiling ` line in the nextest step's
  captured stderr adds a problem: `nextest recompiled after the build step;
  if sources under cli/ changed during the run, rerun; otherwise the unit
  listing and nextest no longer build the same thing`. A task test
  covers it. On unfiltered coverage-on runs, the post-run sweep listing's
  `rebuilt` count guards it as well, and Phase 6 checks the after-record's
  cargo units.
- **A filtered run adds one listing call.** `cargo nextest list` runs before
  the build, which is unavoidable for selecting families. It lists every
  binary rather than only the consumers, because it also has to find a
  filter that matches nothing.

## Migration Notes

- Nothing outside `cli/` reads the old `ACCELERATOR_ZERO_SPAWN_*` names. A
  developer's shell that exports them is now ignored.
- After Phase 2, an interrupted run leaves its trees until the next run
  sweeps them. The next run of `test:unit:cli` is the safe way to clear
  them, because its sweep respects other runs' locks. The root is printed
  in every summary.
- `cli/target/vcs-fixture-logs/` keeps the newest 20 logs. `cargo clean`
  removes them.
- **Rollback.** Revert Phase 2 or Phase 5's task change, and the consumers
  fall back to private builds with no further change, because nothing hands
  them a tree.

## References

- Work item: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Research: `meta/research/codebase/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Baseline: `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`
- Coverage lane shape: `meta/plans/2026-11-13-PP-955-remove-repeated-work-from-the-rust-test-lanes.md`
- Fixture-binary precedent: `cli/vcs-adapters/Cargo.toml:15-29`
- Adopt-path history: `meta/plans/2026-08-31-PP-728-vcs-agnostic-status-log-renderer.md`,
  `meta/validations/2026-08-10-PP-715-converge-corpus-adapters-on-library-backed-vcs-validation.md`
