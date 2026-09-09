---
type: "plan"
id: "2026-09-08-0264-remove-bash-migration-negative-assertion-tests"
title: "Remove Bash-Migration Negative-Assertion Tests Implementation Plan"
date: "2026-09-08T22:19:35+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0264"
parent: "work-item:0264"
derived_from: ["codebase-research:2026-09-08-0264-remove-bash-migration-negative-assertion-tests"]
tags: ["testing", "cleanup", "bash-parity", "migration"]
revision: "7dfe3edcb6be71807f6e0938db464d29051f8653"
repository: "accelerator"
last_updated: "2026-09-09T13:38:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Remove Bash-Migration Negative-Assertion Tests Implementation Plan

## Overview

Remove the negative-assertion tests left over from the shell-to-Rust migration
(epic 0136) that assert the absence of behaviour no longer reachable, together
with the migration-only Python call-site guard and all its wiring. Retiring the
`corpus-adapters` zero-spawn test also retires the dedicated CI suite built
solely for it: the `bash-parity` feature and fixture bin in that crate, the two
`test:integration:zero-spawn*` mise tasks, the backing `integration.py` cluster,
and the `check-zero-spawn` CI job.

The work spans four language toolchains (Rust tests and manifests, the Python
`tasks/` toolchain, `mise.toml`, and the GitHub Actions workflow) but is
overwhelmingly deletion. Every phase lands as its own PR that leaves `mise run
check` and the full suite green; all are order-independent except the 4a→4b pair,
where the additive rewire must precede the cluster deletion.

## Current State Analysis

The shell surface is reduced to the two files named in
`SURVIVING_SHELL_SOURCES`, so the removed absence assertions no longer guard a
reachable regression. Line references below were verified against the current
working-copy revision `7dfe3edcb6be` — they have not drifted from the research
commit `fe3a860c`.

The removal set decomposes into five disjoint units:

- **`cli/work-adapters/tests/zero_spawn.rs`** (85 lines, whole file, gated
  `#[cfg(feature = "bash-parity")]`). Self-contained; the crate's `bash-parity`
  feature stays live via `sync_working_copy_status.rs`.
- **`cli/migrate-cli/tests/no_awk.rs`** (52 lines, whole file, no cfg gate). A
  static source scan; self-documented as a permanent guard, removed on the
  owner's explicit confirmation.
- **`the_probe_shells_out_to_nothing`** in `cli/design-adapters/tests/start_time.rs`
  (lines 91-117, test plus its doc-comment, no cfg gate). Shares no helper with
  the file's three positive tests, which stay.
- **The `corpus-adapters` zero-spawn suite** — a coupled cluster (test file,
  fixture bin, feature, dev-dependency, two mise tasks, the `integration.py`
  functions, and the `check-zero-spawn` CI job) that exists solely to run one
  test. Deleting it orphans `build:cli:fixture-size`, whose only invocation is a
  `depends` edge from the deleted strong task.
- **`tasks/lint/call_site_migration.py`** (whole module) plus its test and its
  full wiring across `tasks/`, `mise.toml`.

### Key Discoveries:

- ⚠️ **`build:cli:fixture-size` is orphaned by the corpus removal.** Its only
  invocation anywhere is the `depends` array of `test:integration:zero-spawn:strong`
  (`mise.toml:356`), which runs only inside the `check-zero-spawn` CI job. The
  guard protects **kept** surface — the `vcs-adapters-fixture`/`-stub`
  static-linking floor (`tasks/build.py:398`). It is rewired into the
  `test:integration` roll-up (via a `test:integration:fixture-size` leaf, Phase
  4a) so its CI execution survives (decision below).
- ⚠️ **`mise.toml` carries call-site-migration wiring the work item's Technical
  Notes name in full but is worth re-stating:** the task block at `579-582` and
  the `lint:check` `depends` entry at `653`. Miss either and `mise run check`
  fails on a missing task.
- **`corpus-adapters-fixture` has no `tasks/build.py` entry**, unlike the kept
  `vcs-adapters-fixture`. It is consumed only by `zero_spawn.rs:211`, so removing
  the `[[bin]]` block, its source, and the test is a clean, self-contained delete
  (`cli/corpus-adapters/Cargo.toml:23-25`).
- **`Collection.from_module(integration)`** (`tasks/test/__init__.py:7`)
  auto-collects the `@task`-decorated functions, so deleting `zero_spawn` /
  `zero_spawn_strong` de-registers them with no explicit `add_task` to touch.
- **`tasks/README.md` documents the retired suite across the whole `### Zero-spawn
  strong form` section (218-288)**, not only the keyword lines the grep finds —
  the keyword-free prose at 220-221, 238-250, and 257-278 describes deleted
  machinery too. The one exception is `280-288`, the **kept** `build:cli:fixture-size`
  guard, which is rehomed rather than removed. The table row at `:724` names the
  deleted CI job. Not executable, so it will not fail `check`, but it would leave
  the canonical task-tree doc describing a removed CI job and removed tasks.
  Folded into Phase 4b step 8 as correctness (which owns the line-level scope).
- **`cli/pup.ron:286` and `:327`** are comments naming the two deleted
  `zero_spawn.rs` files as the runtime complement to the still-active pup rules.
  The rules keep working; the comments become stale.

## Desired End State

Every named negative-assertion test, the corpus-adapters zero-spawn CI suite,
and the call-site-migration guard are gone; the `build:cli:fixture-size` guard
still runs in CI via a surviving lane; every positive parity/golden test, the
`SURVIVING_SHELL_SOURCES` invariant, the bash-3.2 floor lint, the cargo-pup
"must not spawn" rules, and the shared `vcs-adapters-fixture`/`-stub` bins are
untouched. `mise run check` and the full test suite pass.

Verify the end state by:

- Confirming the six delete targets no longer exist and the single-test excision
  landed with the file's positive tests intact.
- Running `mise run check` and the full suite (`mise run`) to green.
- Diffing the positive parity/golden inventory before and after (AC criterion in
  Phase 4b) to prove nothing in the preserve set was touched.

## What We're NOT Doing

- **Not** removing `build:cli:fixture-size`, `cli_fixture_size_check`
  (`tasks/build.py:398`), or its unit tests (`tests/unit/tasks/test_build.py`).
  The guard protects kept surface; it is rewired, not retired, and its behaviour
  is unchanged (it runs cross-platform, verified passing on macOS at 32.6×).
- **Not** removing the shared `vcs-adapters-fixture` / `vcs-adapters-fixture-stub`
  bins or their `tasks/build.py:60-61` build entries. They back `vcs-adapters`'s
  own tests and `vcs-test-support`.
- **Not** removing the `bash-parity` feature from any crate except
  `corpus-adapters`. The other five defining crates retain gated consumers.
- **Not** touching the cargo-pup `work_adapters_is_zero_spawn` /
  `vcs_adapters_is_zero_spawn` rules — only their stale comments.
- **Not** touching `tasks/lint/bare_invocation.py`, `SURVIVING_SHELL_SOURCES`
  and `TestSurvivingShellSources`, the bash-3.2 floor lint, or any positive
  bash-parity/golden test.

## Coverage Retired (Accepted)

The surviving guards do **not** cover everything the removed tests asserted.
This is an accepted loss under the owner's sign-off, not an equivalence — the
removed assertions guarded regression classes no surviving guard catches. Each
is recorded here so the decision is explicit rather than implied.

- **Runtime no-spawn / no-degradation for work-adapters and vcs-adapters
  (Phases 1, 4b).** The deleted `zero_spawn.rs` tests proved more than an absent
  inline spawn: they ran the reference artefact with `git`/`jj` stubbed (and, in
  the strong form, the real binaries removed via sudo) and asserted **both** no
  subprocess spawn **and** byte-identical output — a black-box guard catching a
  spawn via any mechanism, including a transitive library shell-out, plus
  behavioural degradation under a missing or altered VCS binary. The kept
  cargo-pup `work_adapters_is_zero_spawn` / `vcs_adapters_is_zero_spawn` rules
  retain only the import-time slice: *use-path* `std::process` imports (the
  reworded comments say so). A fully-qualified inline
  `std::process::Command::new("git")`, a transitive shell-out, or a degradation
  under an absent binary is now caught by nothing. Reachable, since the adapters
  still read live VCS state.
- **All zero-spawn coverage in corpus-adapters (Phase 4b).** Stronger loss than
  the bullet above: **corpus-adapters has no cargo-pup rule at all** (pup.ron
  names it only in the stale comment being reworded). The deleted
  `corpus-adapters/tests/zero_spawn.rs` was the sole guard on its
  `VcsBackedRepoFactsProbe` metadata-read path, so after removal *any*
  `std::process` use in corpus-adapters — use-path or inline — is caught by
  nothing. It drops from full runtime coverage to zero static coverage, not to
  the use-path floor its siblings retain.
- **Any subprocess in the start-time probe, and the compiled-in tick source
  (Phase 3).** `the_probe_shells_out_to_nothing` forbade `getconf`, `Command::new`,
  and `process::Command` anywhere in `process-probe`'s source **and** positively
  asserted the tick rate is read from compiled-in `sysconf` (`_SC_CLK_TCK`) — not
  just the getconf case. `process-probe` has no pup rule and the
  `design_adapters_read_in_process` rule exempts the `process` module, so two
  classes are now caught by nothing: a regression to any subprocess that still
  returns a correct epoch on the CI host, and a switch away from
  `sysconf(_SC_CLK_TCK)` to a hard-coded tick rate matching the CI host. The
  surviving locale/timezone positive tests only catch a localisation regression,
  and the loss surfaces in the distroless/static-musl containers the probe serves.
- **`awk` shell-out in the migrate crates (Phase 2).** `no_awk.rs` is a
  self-described *permanent* regression guard (unconditional, no `bash-parity`
  gate). Only `migrate-adapters` has a zero-spawn pup rule at all —
  `migrate` and `migrate-cli` have none — and no import rule catches an `awk`
  string literal or a committed `.awk` file even there, so an inline
  `Command::new("awk")` or a `.awk` file is caught by nothing across all three
  crates after removal. Removed on the owner's explicit confirmation, on the
  same basis as the Python guard.
- **Config-cluster and legacy-layout call sites (Phase 5).** The kept
  `bare_invocation` lint forbids pathed / `bash`-wrapped launcher calls — a
  *different* assertion from `call_site_migration`, which forbids `scripts/config-`
  references and stray `--allow-legacy-layout` flags. No static guard catches a
  reintroduced config call site after removal.

## Implementation Approach

Six standalone PRs. Phases 1-3 are isolated Rust deletions. Phase 4 is split
into two: **Phase 4a** adds the fixture-size rewire (purely additive — green on
its own, with the guard running in both the old strong-job lane and the new
leaf), and **Phase 4b** deletes the corpus-adapters zero-spawn cluster. Phase 5
is the Python guard and its wiring. Phases 1-3 and 5 are textually disjoint from
each other and from the Phase 4 pair (Phase 1 and Phase 4b edit different
`pup.ron` comment blocks; Phase 4b and Phase 5 edit different `mise.toml`
regions), so they can land in any order. The one ordering constraint is **4a
before 4b**: 4a must establish the guard's new home before 4b deletes the strong
job that was its only prior invocation, so the guard is never absent from CI.

**Fixture-size rewire decision.** The `build:cli:fixture-size` guard is kept and
rewired into the local CI mirror rather than deleted or left unwired: the
`test-integration` CI job runs `test:integration`, so roll-up membership
preserves its CI coverage. It is wired through a **new `test:integration:fixture-size`
leaf** (depending on the existing `build:cli:fixture-size` task), not by adding
`build:cli:fixture-size` to the roll-up directly. The `test:integration`
membership is a guarded, homogeneous namespace — `tests/unit/tasks/test_mise.py`
asserts every roll-up member is a `test:integration:*` task and classifies each
by launcher need — so a foreign-prefixed member would fail that invariant. The
leaf keeps the namespace *nominally* homogeneous (name prefix) and the guard
reconciled (Phase 4a). The leaf is a depends-only shim whose only purpose is to
satisfy that invariant; its rationale is recorded at the task site and in the
`test_mise.py` reason string so it does not read as accidental redundancy.

The guard runs cross-platform. On macOS `triple="host"`, so only the ≥3× ratio
floor applies (the absolute byte floor is musl-only); this was measured on
`macos-latest`-equivalent hardware (arm64) at 32.6× — the linked artefact is
~10.9 MB against a ~343 KB stub — so the floor holds on Mach-O with an order of
magnitude of headroom, and a breach would require the linked artefact to shrink
~90%, which is exactly the linker-drop regression the guard exists to catch. No
OS-specific scoping is needed.

One cost is accepted. The guard runs `cargo build --release` of the
gix/jj-lib-linked fixtures, unlike the roll-up's dev-build members. On CI this is
cheap relative to the 20-minute sudo-gated job it replaces; **on the local loop
it is a net addition, not a replacement** — the sudo-gated job was CI-only and
never in the bare `default`/`mise run` path, so the leaf slips a release-profile
compile of two large dependency trees (gix, jj-lib) into the canonical local
"done" gate that had no such predecessor. This is *not* warmed by the roll-up's
dev-profile members: cargo compiles the release and dev profiles into separate
artefact trees (`target/release` vs `target/debug`), so a cold or
dependency-changed `mise run` pays a full release compile of gix and jj-lib
(~27s once the dependency trees are built; longer cold). It is judged acceptable,
but the roll-up's cohesion is thereby broadened from "run all integration tests"
to "tests plus one build-linkage guard" — a heterogeneity the topology guard does
not itself capture.

**Guard-home decision (settled):** the roll-up leaf, chosen over a dedicated
CI-only workflow step. The deciding factor is the repo's "done means `mise run`
exits 0 — the full local CI mirror" rule: a contributor who drops the gix/jj-lib
link sees the guard fail on their own machine, not first in CI. The accepted cost
is the local release compile (~27s warm) and the roll-up carrying one
build-linkage guard alongside its integration tests; the CI-only step was rejected
because it would leave `mise run` blind to the regression. If a second build-linkage
guard ever needs the roll-up, evolve the `test_mise.py` invariant to classify such
guards explicitly rather than adding another naming shim.

## Phase 1: Remove the work-adapters zero-spawn test

### Overview

Delete the self-contained `work-adapters` zero-spawn test and reword the stale
pup.ron comment that names it. The crate's `bash-parity` feature stays.

### Changes Required:

#### 1. Delete the test file

**File**: `cli/work-adapters/tests/zero_spawn.rs`
**Changes**: Delete the whole file. It is gated `#[cfg(feature = "bash-parity")]`,
shares no helper module, and has no `[[test]]` registration.

#### 2. Reword the pup.ron comment

**File**: `cli/pup.ron` (lines 283-287)
**Changes**: The comment on `work_adapters_is_zero_spawn` names the deleted file
as closing "the inline `Command::new()` blind spot at run time". Drop that
clause; the crate-wide static `std::process` deny is what now stands alone.

```diff
         // work-adapters spawns no subprocess: its section-diff renders
         // in-process and its VCS identity reads through vcs-adapters' library
-        // path. This crate-wide deny catches use-path std::process imports;
-        // work-adapters/tests/zero_spawn.rs closes the inline
-        // Command::new() blind spot at run time.
+        // path. This crate-wide deny catches use-path std::process imports.
```

### Success Criteria:

#### Automated Verification:

- [x] File is gone: `test ! -e cli/work-adapters/tests/zero_spawn.rs`
- [x] work-adapters compiles and tests pass with the feature enabled:
      `cargo nextest run --manifest-path cli/Cargo.toml -p work-adapters --features bash-parity`
- [x] cargo-pup rules still pass: `mise run pup:check`
- [ ] Read-only check is green: `mise run check`

#### Manual Verification:

- [x] The reworded pup.ron comment reads correctly and names no deleted file.

---

## Phase 2: Remove the migrate-cli no_awk guard

### Overview

Delete the `no_awk.rs` static source scan across the `migrate*` crates, on the
owner's explicit confirmation that the now-pure-Rust migrate crates no longer
need the anti-regression guard.

### Changes Required:

#### 1. Delete the test file

**File**: `cli/migrate-cli/tests/no_awk.rs`
**Changes**: Delete the whole file. No cfg gate, no shared helper, no `[[test]]`
registration.

### Success Criteria:

#### Automated Verification:

- [ ] File is gone: `test ! -e cli/migrate-cli/tests/no_awk.rs`
- [ ] migrate-cli tests pass: `cargo nextest run --manifest-path cli/Cargo.toml -p migrate-cli`
- [ ] Read-only check is green: `mise run check`

#### Manual Verification:

- [ ] None.

---

## Phase 3: Excise `the_probe_shells_out_to_nothing`

### Overview

Remove only the single absence test from `start_time.rs`, keeping the file's
three positive tests, all helpers, imports, and the `TestError` alias.

### Changes Required:

#### 1. Delete the test and its doc-comment

**File**: `cli/design-adapters/tests/start_time.rs` (lines 91-117)
**Changes**: Remove the doc-comment (91-97) and the `the_probe_shells_out_to_nothing`
test (98-117), collapsing the surrounding blank lines so the file keeps a single
blank between the timezone test and `the_probe_is_stable_for_a_live_process`.
The removed test is a static `include_str!` scan of `process-probe/src/lib.rs`;
it shares no helper with the positive tests.

### Success Criteria:

#### Automated Verification:

- [ ] The test name no longer appears:
      `! grep -q the_probe_shells_out_to_nothing cli/design-adapters/tests/start_time.rs`
- [ ] The file's positive tests remain and pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p design-adapters -E 'binary(start_time)'`
      (expect `the_probe_agrees_across_locales`, `the_probe_agrees_across_timezones`,
      `the_probe_is_stable_for_a_live_process`).
- [ ] Read-only check is green: `mise run check`

#### Manual Verification:

- [ ] None.

---

## Phase 4a: Rewire the fixture-size guard into the integration roll-up

### Overview

Purely additive: give the `build:cli:fixture-size` guard a home in the
`test:integration` roll-up via a new `test:integration:fixture-size` leaf, and
reconcile the topology guard. Nothing is deleted here, so the guard runs in both
the old strong-job lane and the new leaf — green on its own. Must land before
Phase 4b, which deletes the strong job.

### Changes Required:

#### 1. Define the fixture-size leaf and add it to the roll-up

**File**: `mise.toml`
**Changes**: Define a `test:integration:fixture-size` leaf whose sole dependency
is the existing `build:cli:fixture-size` guard, and add that leaf to the
`test:integration` roll-up's `depends` array. The leaf carries a comment marking
it as a namespace-invariant shim, and a description naming only its distinct role
(not a copy of `build:cli:fixture-size`'s). The underlying task is unchanged.

```diff
+# A depends-only shim: it lets the fixture-size guard join the test:integration
+# roll-up while keeping the roll-up's guarded invariant that every member is a
+# test:integration:* task. It runs no command of its own — do not delete it as
+# redundant; the assertion lives in build:cli:fixture-size.
+[tasks."test:integration:fixture-size"]
+description = "Roll-up entry point for the gix/jj-lib link-ratio floor; see build:cli:fixture-size"
+depends = ["build:cli:fixture-size"]
+
 [tasks."test:integration"]
 description = "Run all integration tests in parallel"
 depends = [
     "test:integration:visualiser",
     "test:integration:dev",
     "test:integration:entrypoint",
     "test:integration:skill-invocation",
     "test:integration:conformance",
     "test:integration:deny",
     "test:integration:tasks",
     "test:integration:hooks",
+    "test:integration:fixture-size",
 ]
```

#### 2. Classify the leaf in the topology guard and pin its edge

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: Add a `test:integration:fixture-size` entry to `_NO_LAUNCHER_NEEDED`
with a reason that records its *intent* — a build-linkage guard rehomed into the
roll-up, not an integration test, reaching no launcher. This keeps
`test_every_integration_task_declares_its_launcher_need` and
`test_every_integration_task_is_in_the_rollup_or_excluded_with_a_reason`
satisfied (the leaf is a prefix-matched roll-up member classified as needing no
launcher; the zero-spawn entries are still present here and are removed in Phase
4b). Add two regression assertions that pin the full CI-execution chain
(roll-up → leaf → guard), so neither half can be silently dropped in future
maintenance — the leaf's own edge, and the roll-up's reach to the guard (after
Phase 4b the roll-up is the guard's only CI home, and a plausible future
exclusion of the leaf would otherwise drop it from CI without a red gate):

```python
def test_fixture_size_leaf_reaches_the_guard(mise):
    # The leaf's entire protective value is one depends edge; pin it so a
    # future removal fails here rather than passing vacuously green.
    assert "build:cli:fixture-size" in _transitive_depends(
        mise, "test:integration:fixture-size"
    )


def test_fixture_size_guard_runs_in_the_integration_rollup(mise):
    # Pin the roll-up -> guard half of the chain: the roll-up is the guard's
    # only CI home after Phase 4b, so a future exclusion must not silently
    # relocate it out of CI.
    assert "build:cli:fixture-size" in _transitive_depends(
        mise, "test:integration"
    )
```

### Success Criteria:

#### Automated Verification:

- [ ] The leaf resolves and is a roll-up member: `build:cli:fixture-size` appears
      in the resolved dependency graph of `test:integration` (`mise tasks deps
      test:integration`, or grep the roll-up block for
      `test:integration:fixture-size`), and `mise run test:integration:fixture-size`
      passes host-native.
- [ ] The topology guard passes, including both new chain assertions:
      `mise run test:unit:tasks` (the lane that executes
      `tests/unit/tasks/test_mise.py`; `check` and `build-system:check` do not
      run pytest).
- [ ] Read-only check is green: `mise run check`

#### Manual Verification:

- [ ] The `test-integration` CI job runs `test:integration:fixture-size` →
      `build:cli:fixture-size` green on **both** the `ubuntu-latest` and
      `macos-latest` legs (inspect the job log on the PR) — the guard's CI home
      now exists before Phase 4b removes the old lane. (The macOS ratio was
      measured at 32.6× against the 3× floor, so both legs pass.)

---

## Phase 4b: Retire the corpus-adapters zero-spawn cluster

### Overview

Delete the coupled cluster that exists solely to run
`corpus-adapters/tests/zero_spawn.rs`, and reconcile the topology guard by
removing the now-deleted tasks. Its parts are interdependent (deleting the mise
tasks without the CI job, or the test without the fixture bin, leaves a red
intermediate state), so they land together.

**Pre-flight (hard gate).** This phase must not open until Phase 4a has merged.
The 4a→4b order is not mechanically enforced across two separate PRs — 4b applied
alone leaves the topology guard green while `build:cli:fixture-size` (still a live
task) has no CI invocation at all, a silent coverage gap. Before deleting the
strong job, confirm on `main` that `test:integration:fixture-size` exists and that
`build:cli:fixture-size` is in the resolved dependency graph of `test:integration`
(`mise tasks deps test:integration`) — i.e. the `test_fixture_size_guard_runs_in_the_integration_rollup`
assertion added in 4a is present and green.

### Changes Required:

#### 1. Delete the test and fixture source

**Files**: `cli/corpus-adapters/tests/zero_spawn.rs`,
`cli/corpus-adapters/tests/fixtures/corpus_adapters_fixture.rs`
**Changes**: Delete both whole files. The fixture bin is consumed only by
`zero_spawn.rs:211`.

#### 2. Trim the corpus-adapters manifest

**File**: `cli/corpus-adapters/Cargo.toml`
**Changes**: Remove the `[features]` block (lines 12-18, the `bash-parity`
feature and its comment) — `corpus-adapters` is the feature's only defining
crate that loses its sole consumer. Remove the `[[bin]] corpus-adapters-fixture`
block (20-25, with its comment). Remove the `[dev-dependencies]` block (40-44,
`vcs-test-support` and its comment) — its only consumer is the deleted test. The
`package`, `lints`, and `dependencies` sections remain.

#### 3. Remove the integration.py zero-spawn cluster

**File**: `tasks/test/integration.py`
**Changes**: Remove the `zero_spawn` and `zero_spawn_strong` tasks; the helpers
`_compile_zero_spawn_targets`, `_build_fixture_matrix`, `_build_status_log_states`,
`_resolve_vcs_binaries`, `_restore_vcs_binaries`; and the constants
`_SHADOW_OPT_IN`, `_MATRIX_ROOT`, `_STATUS_LOG_ROOT`, `_ABSOLUTE_VCS_PATHS` with
their comments. Then prune the now-unused imports: drop `tempfile`,
`from collections.abc import Sequence`, and `CLI_WORKSPACE_CARGO_TOML` from the
`tasks.shared.paths` import (keep `CARGO_TOML`). Keep `os`, `shlex`, `Path`,
`Context`/`Exit`/`task`, `_MANIFEST`, and the `.helpers` imports — all still used
by the surviving tasks (`_resolve_driver_tree`, `design_automation`, `visualiser`).

#### 4. Remove the two mise tasks

**File**: `mise.toml` (lines 349-357)
**Changes**: Delete the `[tasks."test:integration:zero-spawn"]` and
`[tasks."test:integration:zero-spawn:strong"]` blocks.

#### 5. Reconcile the mise-topology regression guard

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: Phase 4a already added the leaf and its classification; this step
handles the deletions. Remove the `test:integration:zero-spawn` and
`test:integration:zero-spawn:strong` entries from both `_NO_LAUNCHER_NEEDED` and
`_NOT_IN_INTEGRATION_ROLLUP`, **together with the multi-line rationale comment
blocks preceding the `_NOT_IN_INTEGRATION_ROLLUP` entries** (the `~34-fixture
matrix` / "Owned by check-zero-spawn" block and the sudo-shadow block). Left in
place, those comments name a deleted CI job and the removed shadow contract, and
one would orphan above the surviving `test:integration:design-automation` entry
and misattribute to it. After the removal `_integration_tasks` drops to 13 and
both classification invariants (`test_every_integration_task_declares_its_launcher_need`,
`..._is_in_the_rollup_or_excluded_with_a_reason`) stay satisfied. The
`test_fixture_size_leaf_reaches_the_guard` assertion added in Phase 4a is
unaffected.

#### 6. Remove the CI job

**File**: `.github/workflows/main.yml`
**Changes**: Delete the whole `check-zero-spawn` job (lines 355-409, from the
`check-zero-spawn:` key through the `Assert the restore worked` backstop step,
ending before `check-visualiser-frontend:` at line 411) and its entry in the
aggregate `needs:` list (line 617, `- check-zero-spawn`).

#### 7. Reword the pup.ron comment

**File**: `cli/pup.ron` (lines 324-328)
**Changes**: The comment on `vcs_adapters_is_zero_spawn` names
`corpus-adapters/tests/zero_spawn.rs` as closing the runtime blind spot. Drop
that clause, as in Phase 1.

```diff
         // vcs-adapters spawns no subprocess: every port, including the
         // status/log renderings, reads both idioms in the calling process. This
-        // crate-wide deny catches use-path std::process imports anywhere in the
-        // crate; corpus-adapters/tests/zero_spawn.rs closes the inline
-        // Command::new() blind spot at run time.
+        // crate-wide deny catches use-path std::process imports anywhere in the
+        // crate.
```

#### 8. Update the task-tree documentation

**File**: `tasks/README.md`
**Changes**: The retired suite is documented across the whole `### Zero-spawn
strong form` section (218-288), not just a handful of lines. Rewrite the section
as follows:

- Remove the section as it stands (218-278): the heading, the "Two mechanisms
  prove it" framing (220-221), the weak/strong-form descriptions (223-236), the
  three-source target-resolution and `mise which` contract (238-250), the
  roll-up-exclusion rationale (252-255), and the two privileged-mutation /
  ephemeral-runner containment paragraphs (257-278) that name
  `ACCELERATOR_ZERO_SPAWN_MODE`/`_SHADOWED`/`_SHADOW`, the `sudo` shadow/restore,
  the `if: always()` backstop, and `cache: false`. All describe deleted
  machinery.
- Keep the surviving `build:cli:fixture-size` prose (280-288) but rehome it into
  its own short subsection (e.g. `### The gix/jj-lib link-ratio guard`), reframed
  as a standalone linker-drop guard rather than "the third guard" (the two
  mechanisms it counted against are gone). The rehomed prose must preserve **both**
  roles the guard still plays: the host-native ratio floor, now run in the
  `test:integration` roll-up via the `test:integration:fixture-size` leaf; and
  the cross-compile behaviour that is unchanged by this work — the absolute byte
  floor on musl only, with darwin deliberately not gated because its ~9% stripped
  margin would otherwise sit on `prerelease:prepare`'s critical path.
- Remove the `check-zero-spawn` CI-job → local-command table row at `:724`.

### Success Criteria:

#### Automated Verification:

- [ ] All four targets gone:
      `test ! -e cli/corpus-adapters/tests/zero_spawn.rs && test ! -e cli/corpus-adapters/tests/fixtures/corpus_adapters_fixture.rs`
- [ ] `bash-parity`, `corpus-adapters-fixture`, and `vcs-test-support` are absent
      from the manifest:
      `! grep -Eq 'bash-parity|corpus-adapters-fixture|vcs-test-support' cli/corpus-adapters/Cargo.toml`
      (a smoke check only — it does not prove clean section removal; manifest
      well-formedness is proven by the `test:unit:cli` / `check` compile below).
- [ ] The mise tasks are gone:
      `! grep -q 'test:integration:zero-spawn' mise.toml`
- [ ] The CI job and its `needs:` entry are gone:
      `! grep -q 'check-zero-spawn' .github/workflows/main.yml`
- [ ] The guard still runs in the roll-up after the strong-job deletion:
      `mise run test:integration:fixture-size` passes host-native and
      `build:cli:fixture-size` is still in the resolved graph of `test:integration`.
- [ ] The mise-topology guard passes after the task deletion:
      `mise run test:unit:tasks` (the lane that executes
      `tests/unit/tasks/test_mise.py`; `check` and `build-system:check` do not
      run it).
- [ ] The shared vcs-adapters fixtures still build:
      `cargo build --manifest-path cli/Cargo.toml -p vcs-adapters --bin vcs-adapters-fixture --bin vcs-adapters-fixture-stub`
- [ ] corpus-adapters compiles and tests pass without the feature, under the
      all-features CI configuration that would surface a lingering `bash-parity`
      reference: `mise run test:unit:cli` (runs `--workspace --all-features
      --exclude accelerator-visualiser`).
- [ ] Build-system checks pass (integration.py well-formed after the import prune):
      `mise run build-system:check`
- [ ] Workflow lint passes: `mise run lint:workflows:actionlint`
- [ ] cargo-pup rules still pass: `mise run pup:check`
- [ ] The positive parity/golden inventory is unchanged. Capture
      `git ls-files 'cli/**/*parity*.rs' 'cli/**/migration_*.rs' 'cli/**/*golden*.rs'`
      at the parent commit and after the change; the listings must be identical.
      (The `*golden*` glob closes the gap the research flagged — parity- and
      migration-only globs miss the `*_goldens.rs` files.)
- [ ] Read-only check is green: `mise run check`

#### Manual Verification:

- [ ] After `check-zero-spawn` is deleted, the `test-integration` CI job still
      runs `test:integration:fixture-size` → `build:cli:fixture-size` on both
      legs (inspect the job log on the PR) — the guard's CI execution survived
      the removal of its old lane.
- [ ] The reworded pup.ron comment and the README edits name no deleted path.

---

## Phase 5: Remove the call-site-migration Python guard

### Overview

Remove the migration-only Python guard and its full wiring, on the owner's
explicit confirmation. The forward-looking `bare_invocation` lint is untouched.

### Changes Required:

#### 1. Delete the module and its test

**Files**: `tasks/lint/call_site_migration.py`,
`tests/unit/tasks/test_call_site_migration.py`
**Changes**: Delete both whole files.

#### 2. Unwire from the lint package

**File**: `tasks/lint/__init__.py`
**Changes**: Remove `call_site_migration` from the `from . import (...)` block
(line 4) and from `__all__` (line 22).

#### 3. Unwire from the task collection

**File**: `tasks/__init__.py` (lines 128-130)
**Changes**: Remove the `ns_lint.add_collection(Collection.from_module(lint.call_site_migration))`
block.

#### 4. Unwire from mise.toml

**File**: `mise.toml`
**Changes**: Remove the `[tasks."lint:call-site-migration:check"]` block (lines
579-582) and the `"lint:call-site-migration:check"` entry from the `lint:check`
`depends` array (line 653).

### Success Criteria:

#### Automated Verification:

- [ ] Both files gone:
      `test ! -e tasks/lint/call_site_migration.py && test ! -e tests/unit/tasks/test_call_site_migration.py`
- [ ] No `call_site_migration` reference remains in executable wiring:
      `! grep -rEq 'call_site_migration|call-site-migration' tasks/ mise.toml`
      (extended-regex `-E`, not BRE `\|` — on the repo's macOS floor BSD grep
      treats `\|` as a literal and the check would false-pass).
- [ ] The lint aggregate resolves and runs: `mise run lint:check`
- [ ] Build-system checks pass: `mise run build-system:check`
- [ ] The kept guard survives: `mise run lint:bare-invocation:check`
- [ ] Read-only check is green: `mise run check`

#### Manual Verification:

- [ ] None.

---

## Testing Strategy

### Unit Tests:

- Almost entirely deletion. The one exception is in Phase 4a: the `test_mise.py`
  edits that keep the mise-topology guard in step — the leaf's classification plus
  the `test_fixture_size_leaf_reaches_the_guard` and
  `test_fixture_size_guard_runs_in_the_integration_rollup` chain assertions. Phase
  4b removes the deleted zero-spawn entries from `test_mise.py` (step 5). `mise
  run test:unit:tasks` is the gate that exercises `test_mise.py`; `mise run
  build-system:check` (pyrefly + ruff) does not run pytest, so it cannot stand in
  for that gate.
- After Phase 4b's import prune, `tasks/test/integration.py` must remain
  importable; `mise run build-system:check` is the gate for that.
- The broad assertion is that the surviving unit suite still passes:
  `mise run test:unit`.

### Integration Tests:

- Phase 4a wires `build:cli:fixture-size` into `test:integration` via the new
  `test:integration:fixture-size` leaf. Run `mise run test:integration` and
  confirm the fixture-size guard executes and passes as part of the roll-up.

### Manual Testing Steps:

1. On the Phase 4a PR, open the `test-integration` CI job log and confirm the
   `test:integration:fixture-size` → `build:cli:fixture-size` step ran green on
   both the ubuntu and macOS legs — the coverage the `check-zero-spawn` job
   provided, now landed before that job is deleted in Phase 4b.
2. Confirm no PR shows a red intermediate state: each phase's `mise run check`
   and `mise run` are independently green.

## Performance Considerations

Retiring the `check-zero-spawn` CI job removes a 20-minute-timeout job from every
push. The rewired `build:cli:fixture-size` is a host-native build-plus-size
assertion — cheap relative to the sudo-gated strong suite it replaces — so net CI
time falls.

## Migration Notes

None. No data or schema is affected; the change is confined to tests, manifests,
task definitions, and CI configuration.

## References

- Original work item: `meta/work/0264-remove-bash-migration-negative-assertion-tests.md`
- Related research: `meta/research/codebase/2026-09-08-0264-remove-bash-migration-negative-assertion-tests.md`
- Work item review: `meta/reviews/work/0264-remove-bash-migration-negative-assertion-tests-review-1.md`
- Fixture-size guard: `tasks/build.py:398` (`cli_fixture_size_check`), documented at `tasks/README.md:280` pre-change (Phase 4b step 8 rehomes it to a standalone subsection, e.g. "The gix/jj-lib link-ratio guard")
- Retired CI job: `.github/workflows/main.yml:355-409`
- Related work items: 0136 (parent epic), 0174, 0211, 0212, 0245, 0269
