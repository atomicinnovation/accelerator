---
type: "plan"
id: "2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic"
title: "Make the Bootstrap Test Suites Fast and Hermetic Implementation Plan"
date: "2026-11-11T19:01:52+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0362"
parent: "work-item:0362"
derived_from: ["codebase-research:2026-11-10-0362-bootstrap-test-suites-fast-and-hermetic"]
relates_to: ["work-item:0361", "work-item:0366", "work-item:0370"]
tags: ["build-system", "performance", "testing", "bootstrap", "minisign", "hooks", "cargo-profile"]
revision: "e28745b60fc6a1b1906d0546dec369bd21bbd9ac"
repository: "accelerator"
last_updated: "2026-11-12T19:00:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Make the Bootstrap Test Suites Fast and Hermetic Implementation Plan

## Overview

Two changes to the suites that run `bin/accelerator`:

1. Build `minisign-verify` at `opt-level = 3` in the dev profile. The debug
   `accelerator-verify` shim, which the bootstrap runs over the whole 76 MB
   debug launcher on every call, then hashes at release speed.
   `test:integration:skill-invocation` drops from ~163 s to ~18 s locally.
2. Move the hooks launcher-dispatch smoke test off the published GitHub
   release and onto the stubbed-downloader installation. Add a negative
   case beside it: under a hook's `--fail-safe`, the bootstrap refuses a
   launcher signed with the wrong key.

Two supporting changes come with the second:

- The installation fixtures the entrypoint and skill-invocation suites each
  wire by hand move into one shared module, so the hooks suite does not
  become a third copy.
- A check that the committed verify shim for the running host accepts a
  release-key signature, and refuses both a signature from another key and
  a payload changed after signing. Removing the GitHub fetch removes the
  only blocking pull-request test that ran that binary.

The work ships as three pull requests:

| PR | Phases | Content | Status |
|---|---|---|---|
| 1 | 1 | The `minisign-verify` override and its measurements | Open as #210 |
| 2 | 2 | Shared installation fixtures (a refactor; no behaviour change) | Not started |
| 3 | 3, 4, 5 | Vendored-shim check, hermetic smoke test, refusal test | Not started |

## Current State Analysis

- `cli/Cargo.toml:286-291` holds the only profile table,
  `[profile.release]`. `minisign-verify` is pinned at `cli/Cargo.toml:99`,
  and both `cli/verify` and `cli/launcher` use it. It has no dependencies of
  its own (`cli/Cargo.lock:3428-3431`), so one package override covers all
  of the hot code.
- `bin/accelerator:435-455` (`ensure_launcher`) re-verifies the cached
  launcher on every call. The shim is always
  `${plugin_root}/bin/accelerator-verify-${platform}`. `make_installation`
  places the debug shim from `build_shim()` there, which is how the debug
  shim enters the chain in the entrypoint and skill-invocation suites.
- `tests/integration/hooks/test_vcs_detect_registration.py:48-82` runs the
  **repository's** `bin/accelerator` with an inherited environment and no
  downloader override. It therefore `curl`s the published `v<version>`
  launcher from GitHub (17 MB; 42 s on the research workstation). It does
  not check that the launcher it ran was the one it fetched, and stdout
  parses as JSON *only if* non-empty.
- That test is also the only blocking pull-request test that runs the
  committed vendored shim (`bin/accelerator-verify-<platform>`) against the
  committed `keys/accelerator-release.pub`. It does so on both integration
  lanes (`ubuntu-latest` and `macos-latest`). The `check-measure-harness`
  job (`.github/workflows/main.yml:392-434`) also runs the real bootstrap
  against a published release on both lanes, but it is `continue-on-error`
  and gates nothing.
- `tests/integration/support/installation.py` already provides everything
  Requirements 2–3 need:
  - `make_installation(..., secret=, launcher_source=)`;
  - `run_bootstrap`, a funnel that builds a minimal explicit environment
    and checks `assert_hermetic`;
  - `sign` and `sig_path`;
  - `download_log`;
  - `VERSION`;
  - `host_platform`;
  - `generate_keys`;
  - `write_downloader`.
- The fixtures around those functions are wired by hand twice:
  `host_platform`, `shim_bin`, `keys`, `downloader` and `make_harness` in
  `tests/integration/entrypoint/test_accelerator_entrypoint.py:83-160`, and
  inline in the `installation` fixture at
  `tests/integration/skill-invocation/test_skill_invocation_conformance.py:68-82`.
  Both pass `make_installation` a `bootstrap` argument it never reads: it
  copies the bootstrap itself.
- `run_bootstrap` passes on no environment. Each test must hand
  `ACCELERATOR_VCS_BIN`, `ACCELERATOR_LOG` and `LAUNCHER_ARGS_OUT` through
  `extra_env`.
- The hooks task (`tasks/test/integration.py:120-133`) passes
  `accelerator_env(vcs_bin=True)`. It is that flag's only caller. Once the
  smoke test is rewritten, no hooks module reads the overlay at all:
  `test_research_guard_registration.py:100-104` strips every
  `ACCELERATOR_*` variable, and `test_launcher_link_refresh.py:81-89`
  composes its own environment.
- `claim_artefact` (`tests/support/artefacts.py:38-53`) copies a binary to
  `<artefacts>/<name>`, and `build_launcher()` memoises its claim of
  `accelerator` at that same path.
- The refusal path under `--fail-safe` (`bin/accelerator:35-71`,
  `:415-459`) behaves like this:
  - exit 0;
  - stderr carries `accelerator: could not fetch and verify the accelerator
    launcher for <alias>` (the backslash-newline sits inside double quotes,
    so the message is one string);
  - one line is appended to `<cache>/.accelerator-unverified.log`;
  - the temporary files are removed, the cache lock is released, and
    nothing reaches the cache path.
- The shim exits 0 on a valid signature. On any other it exits 1 and prints
  `accelerator-verify: signature verification failed: …`
  (`cli/verify/src/main.rs:12-43`).

## Desired End State

- `cli/Cargo.toml` carries `[profile.dev.package.minisign-verify]` with
  `opt-level = 3`, and a unit test pins it together with the dependency it
  names.
- PR 1 records:
  - the shim speed medians (≥ 5×);
  - the local skill-invocation medians (≥ 4×);
  - the CI skill-invocation duration, plus either a ≥ 3× result or a
    linked, ready follow-up item.
- `tests/integration/support/installation_fixtures.py` holds the
  installation fixtures, and the entrypoint, skill-invocation and hooks
  modules all load it.
- A new entrypoint module shows the committed shim for the running host
  accepting a release-key signature, and refusing an attacker-key signature
  and a tampered payload.
- `tests/integration/hooks/test_vcs_detect_hook.py` (renamed from
  `test_vcs_detect_registration.py`) makes no network access and runs no
  repository `bin/` file. Its smoke test serves the working-tree debug
  launcher through the stub downloader, and asserts the download log, the
  cache bytes and the full `SessionStart` hook envelope.
- A new test in the same module shows the bootstrap refusing an
  attacker-signed launcher under `--fail-safe`, with a re-sign control.
- The hooks task passes its suite no environment overlay, and
  `accelerator_env` no longer offers `vcs_bin`.
- `mise run` exits 0.

### Key Discoveries

- **Where the profile table goes.** The vendored-shim drift marker hashes
  the first line that starts with `minisign-verify` (`tasks/build.py:456-460`,
  `:547-590`). The new table goes after `[profile.release]`, below line 99,
  and no line inside it starts with `minisign-verify`. Its header starts
  with `[` and its comment with `#`, so neither matches.
- **`report.rs` still passes.** `cli/vcs-cli/src/report.rs:244-265` slices
  `[profile.release]` up to the next `\n[`. The new table's comment sits
  above its header, so the slice gains that comment. The test fails only
  when the slice holds both `panic` and `abort`, and the comment holds
  neither. Keep it that way if the comment is ever reworded.
- **The launcher is the same variant either way.** `build:cli:dev` builds it
  with `test-loopback` (`tasks/shared/dev_builds.py:48`), as
  `build_launcher()` does. A claimed copy of `cli/target/debug/accelerator`
  is therefore the right binary to serve.
- **The hooks suite must not call `build_launcher()`.** It carries the
  `build:cli:dev` edge (`mise.toml:462-465`), and `build_launcher()`'s
  docstring forbids pairing the two. It also takes `accelerator-vcs` from
  `build:cli:vcs:dev`.
- **A task-built binary needs its own claimed name.** Claimed as
  `accelerator`, the hooks suite's copy would land on the path
  `build_launcher()` has memoised. In one pytest process, after a plain
  `cargo build`, that would swap a launcher without `test-loopback` under
  the entrypoint suite. Phase 4 adds `claim_task_built`, which claims under
  a separate name.
- **`ACCELERATOR_VCS_BIN` stays, set by the test alone.** The launcher
  would otherwise verify the sub-binary against the committed key embedded
  at build time, which no per-run key satisfies. With the override set, the
  resolver short-circuits before any key, cache or network access
  (`cli/launcher/src/main.rs:171-204`). The override logs at INFO, hence
  `ACCELERATOR_LOG=error`.
- **Swallowed adapter failures are caught too.** `vcs-cli` under
  `--fail-safe` emits a bare `{"systemMessage": …}` on an adapter error.
  The `["hookSpecificOutput"]` lookup raises `KeyError` on it.
- **The control re-sign is enough.** `sign()` overwrites `<target>.minisig`,
  and the refused launcher never reached the cache, so the control's second
  call fetches afresh with no cache manipulation.
- **`pytest_plugins` in a test module, not a `conftest.py`.** The three
  suites sit in sibling directories. A `tests/integration/conftest.py`
  would hand the fixtures to every integration suite, and pytest refuses
  `pytest_plugins` in a non-root `conftest.py`. Declared in a test module,
  the plugin registers once for the whole session, however many modules
  name it. Two consequences:
  - a fixture's `module` scope follows the test module that requests it,
    so `host_platform`, `shim_bin` and `keys` keep the scope they have
    today;
  - a module that forgets the declaration still passes in a directory run,
    because a sibling loaded the plugin. Each declaring module is therefore
    also run alone.

  This is the repository's first use of `pytest_plugins`.
- **`hyperfine` is not installed.** Phase 1's shim timing uses a small
  Python script.

## What We're NOT Doing

- Skipping re-verification on cache hits.
- Any dev-profile setting other than the `minisign-verify` override (0366's
  scope).
- Bringing the debug shim under a mise task. `build_shim()` stays, so the
  hooks suite takes its launcher and `accelerator-vcs` from mise edges and
  its shim from an in-pytest build. That split is temporary: 0366 decides
  whether the shim gains a task.
- A replacement end-to-end smoke of a *published* launcher. What that
  leaves without a blocking check on a pull request:
  - the committed public key against a genuine release signature. Only the
    release run checks it (`tasks/release.py:172-180`,
    `tasks/github.py:176-189`), with the darwin-arm64 shim. A bad
    committed key is therefore found after the merge, with the version
    commit and tag already pushed and the draft held back;
  - the `darwin-x64` and `linux-arm64` vendored shims, which no CI lane
    executes. Their guards stay static (drift marker, static-ELF check,
    is-committed check). Phase 3 does run them on a contributor's machine
    of that platform, which is the first execution they get.

  Phase 3 keeps the host's vendored shim executed on both CI lanes.
- Moving `repo_bin_is_untouched` into the shared fixtures. The entrypoint
  and skill-invocation modules keep their own. The hooks module needs none:
  it runs no repository `bin/` file, and `run_bootstrap`'s
  `assert_hermetic` refuses the repository's bootstrap.
- Routing `test_research_guard_registration.py` through `run_bootstrap`, or
  moving its own `cli/target/debug` constants onto `claim_task_built`. It
  still runs a copied bootstrap through its own `subprocess.run`, so the
  funnel's guarantee covers `test_vcs_detect_hook.py`, not the whole hooks
  suite.
- Lifting the bootstrap's file names (cache path, `.tmp-launcher-*`, the
  unverified log) into the support module.
- Honouring `CARGO_TARGET_DIR`. The tests read `cli/target/debug`, as
  `tests/support/artefacts.py` and `tasks/test/helpers.py` already do.
- Changing pytest's temporary-directory retention. The smoke test leaves
  about 150 MB per run under `tmp_path`, of which pytest keeps the last
  three runs. The entrypoint suite's real-launcher tests already retain
  several times that.
- Speed bars for the hooks or entrypoint suites.

## Implementation Approach

Every command in this plan runs in a mise-activated shell. Outside one,
`minisign` and `jj` are absent, and a test that needs `minisign` skips
locally rather than fails.

Phase 1 is a configuration change. Its behaviour is speed, which no unit
test can observe, so the red step is a pin test on the profile table. The
speed itself is verified by recorded measurements. The pin test also guards
the override when 0366 reshapes the same table.

Phase 2 is a refactor. The entrypoint and skill-invocation suites are its
safety net: they pass before it and must pass unchanged after it.

Phase 4 changes one task, with a failing unit test first. Phases 3–5
otherwise add or rewrite tests of code that already exists, so there is no
production code for a failing test to demand. Each of those tests has a
**mutation check** as its red step: a table with one row per assertion.
Each row names a deliberate break to the test's input and the assertion
that must fail. Apply one row, watch that assertion fail, restore, and move
to the next. An assertion that no row turns red is not shown to work, so
every assertion has a row. Where no input can break one assertion alone,
the row says so and edits the expected value instead.

---

## Phase 1: Optimise `minisign-verify` in dev builds (PR 1)

### Overview

Add the package override and pin it with a unit test. Remove a stale timing
figure from a comment. Measure and record the shim, local-suite and CI
speed-ups.

### Changes Required

#### 1. Pin test (red first)

**File**: `tests/unit/tasks/test_cargo_profiles.py` (new)
**Changes**: a module for pins on the workspace's Cargo profiles, after
the precedent of `tests/unit/tasks/test_nextest_filter.py`. 0366 adds its
own dev-profile pins here.

```python
from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML, load_toml


def test_dev_builds_verify_signatures_at_release_speed():
    manifest = load_toml(CLI_WORKSPACE_CARGO_TOML)
    dev_packages = manifest["profile"]["dev"]["package"]

    assert "minisign-verify" in manifest["workspace"]["dependencies"], (
        "the dev-profile override names a package the workspace no longer "
        "depends on, and cargo only warns about that"
    )
    assert dev_packages["minisign-verify"]["opt-level"] == 3
```

Run `uv run pytest tests/unit/tasks/test_cargo_profiles.py` and see it fail
with `KeyError: 'dev'`.

Once the override is in place and the test is green, show the first
assertion red once: temporarily rename the `minisign-verify` key under
`[workspace.dependencies]`, watch that assertion fail, and restore.

#### 2. The override (green)

**File**: `cli/Cargo.toml`
**Changes**: insert directly after the `[profile.release]` table (after line
291, before the `include-flate-compress` comment block).

```toml
# The bootstrap re-verifies the whole launcher on every call, so in debug
# builds the unoptimised signature hash dominates every bootstrap-driven test.
[profile.dev.package.minisign-verify]
opt-level = 3
```

The comment records why one dependency departs from the dev defaults,
which the table alone does not say.

#### 3. Stale comment

**File**: `tests/integration/skill-invocation/test_skill_invocation_conformance.py:48-50`
**Changes**: drop the `~0.1s a go` figure, which was never accurate and
will drift again.

```python
# Collected at import time because parametrisation needs it before fixtures
# run. One case per *distinct* command: the 204 occurrences carry 122 distinct
# texts, and re-running a duplicate exercises nothing new.
```

#### 4. Measurements for the PR description

These are not committed. Each step's before/after pair runs on the same
host. Take no local timing from the first build after the override lands:
it rebuilds `minisign-verify` and everything that depends on it.

**Shim speed** (≥ 5× median, both accept).

On the parent revision (no override), build the shim and the launcher
once, and copy both into a scratch directory:

```bash
SCRATCH="$(mktemp -d)"
cargo build -p accelerator-verify --manifest-path cli/Cargo.toml
mise run build:cli:dev
cp cli/target/debug/accelerator-verify "$SCRATCH/shim-before"
cp cli/target/debug/accelerator "$SCRATCH/launcher"
minisign -G -W -f -p "$SCRATCH/k.pub" -s "$SCRATCH/k.key"
minisign -S -s "$SCRATCH/k.key" -m "$SCRATCH/launcher" \
  -x "$SCRATCH/launcher.minisig"
```

With the override applied, rebuild the shim and copy it to
`"$SCRATCH/shim-after"`. Keep the one `"$SCRATCH/launcher"`: the override
also changes the launcher's own bytes.

Run each shim once untimed, then time 10 verifications of each,
alternating the two. `check=True` stops the script if either refuses:

```bash
uv run python - "$SCRATCH" <<'EOF'
import statistics, subprocess, sys, time
from pathlib import Path
d = Path(sys.argv[1])
shims = ("shim-before", "shim-after")
def verify(shim):
    subprocess.run(
        [d / shim, d / "k.pub", d / "launcher.minisig", d / "launcher"],
        check=True, capture_output=True,
    )
times = {shim: [] for shim in shims}
for shim in shims:
    verify(shim)
for _ in range(10):
    for shim in shims:
        start = time.perf_counter()
        verify(shim)
        times[shim].append(time.perf_counter() - start)
for shim, taken in times.items():
    print(
        shim,
        f"median {statistics.median(taken):.3f}s",
        f"min {min(taken):.3f}s max {max(taken):.3f}s",
    )
EOF
```

**Local suite speed** (≥ 4× median, every case passes): on each revision,
run `mise run test:integration:skill-invocation` once as the discarded
warm-up (it warms the in-pytest shim and launcher builds). Then run
`time mise run --skip-deps test:integration:skill-invocation` 3 times, and
record each wall time and the median.

**CI suite speed**: once the pull request's last `Main` run finishes, read
the ubuntu leg of the `Run integration tests` job:

```bash
JOB=$(gh run view <run-id> --json jobs --jq \
  '.jobs[] | select(.name | test("integration.*ubuntu")) | .databaseId')
gh run view <run-id> --log --job "$JOB" > "$SCRATCH/job.log"
grep -E 'test:integration:skill-invocation.*(Finished in|passed)' \
  "$SCRATCH/job.log"
```

Then take the per-test figures from the same log's timestamps:

```bash
grep -E 'test:integration:skill-invocation.*(session starts|PASSED|FAILED)' \
  "$SCRATCH/job.log" | uv run python -c '
import re, statistics, sys
from datetime import datetime
stamp = re.compile(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{6}")
at = [datetime.fromisoformat(stamp.search(line).group()) for line in sys.stdin]
gaps = [(b - a).total_seconds() for a, b in zip(at, at[1:])]
print(len(gaps), "tests")
print(f"first {gaps[0]:.1f}s")
print(f"median of the rest {statistics.median(gaps[1:]):.2f}s")
'
```

If either `grep` prints nothing, read the task's lines from
`"$SCRATCH/job.log"` and adjust the pattern; do not leave a figure
unrecorded.

Record in the PR:

- mise's `Finished in` duration for the task. This is the figure compared
  with the 1,266 s baseline, which was read from the same line;
- pytest's own summary duration, as a second figure;
- the number of tests, beside the baseline run's 143. If the two differ,
  the per-test median against the baseline's 8.0 s is the like-for-like
  comparison;
- the first test's duration and the median of the rest. The first test
  builds the shim and launcher (154 s in the baseline run), so these show
  whether a miss comes from that fixed build or from the per-test cost.
  The timestamps are the times lines reached the runner, so the median is
  coarse once a test takes a fraction of a second;
- whether the run was the pull request's first with the override. If so,
  it includes the one-off rebuild of `minisign-verify`, which lands in the
  first test's figure.

If the `Finished in` duration is more than 422 s, create a follow-up work
item before merging (`/accelerator:create-work-item`), set it to status
ready, and link it from the PR. Its acceptance criteria must carry:

- the 3× bar;
- the 1,266 s baseline;
- the recorded miss, with the first-test and per-test figures;
- an investigation of CI's per-test cost.

Do not rerun the job to chase the bar.

### Success Criteria

#### Automated Verification

- [x] The pin test passes:
      `uv run pytest tests/unit/tasks/test_cargo_profiles.py`
- [x] The drift marker is unchanged, so the vendor check passes:
      `uv run pytest tests/unit/tasks/test_build.py` and `mise run check`
- [x] The `report.rs` profile slice test passes: `mise run test:unit:cli`
- [x] The entrypoint regressions pass with the optimised shim:
      `mise run test:integration:entrypoint`, including
      `test_non_release_key_signature_is_refused` and
      `test_tampered_cached_launcher_is_refused_and_healed`
- [x] Skill-invocation passes: `mise run test:integration:skill-invocation`
- [x] Full run green: `mise run`

#### Manual Verification

- [x] The pin test was seen to fail with `KeyError: 'dev'` before the
      override, and its dependency assertion was seen to fail once after
- [x] The shim medians are recorded in the PR: both shims accept the
      signature, and the after-median is ≥ 5× faster than the before-median.
      Recorded in #210: 1.189 s before, 0.069 s after (~17×), both accepting
- [x] The local skill-invocation medians are recorded in the PR, ≥ 4×
      faster, with every case passing. Recorded in #210: median 168.4 s
      before, 14.2 s after (~11.9×), 143 passed on every run
- [x] The CI figures from the PR's last `Main` run are recorded. The
      `Finished in` duration is either ≤ 422 s, or a ready follow-up item
      carrying the bar, the baseline and the miss is linked from the PR
      before merge. Recorded in #210: 375.46 s (~3.37× against 1,266 s),
      including the one-off rebuild, so no follow-up item. The build landed
      on the seventh test, the first to use the installation (70.6 s), not
      on the first; the 136 tests after it had a median of 1.57 s

---

## Phase 2: Shared installation fixtures (PR 2)

### Overview

Move the installation fixtures into one module that each bootstrap suite
loads, and drop `make_installation`'s unread `bootstrap` parameter. No
test's behaviour changes.

### Changes Required

#### 1. The shared fixtures

**File**: `tests/integration/support/installation_fixtures.py` (new)
**Changes**: the fixtures now in the entrypoint module, with the factory
taking the launcher to serve rather than a flag that builds one. A suite
that carries the `build:cli:dev` edge can then pass a claimed copy.
`NewInstallation` gives the factory a checked signature wherever a test
takes it.

```python
from pathlib import Path
from typing import Protocol

import pytest

from tests.integration.support.installation import (
    Installation,
    generate_keys,
    make_installation,
    write_downloader,
)
from tests.integration.support.installation import (
    host_platform as resolve_host_platform,
)
from tests.support.artefacts import build_shim


class NewInstallation(Protocol):
    def __call__(
        self,
        label: str = "self",
        *,
        secret: str = "release",
        launcher_source: Path | None = None,
    ) -> Installation: ...


@pytest.fixture(scope="module")
def host_platform() -> str:
    return resolve_host_platform()


@pytest.fixture(scope="module")
def shim_bin() -> Path:
    return build_shim()


@pytest.fixture(scope="module")
def keys(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return generate_keys(tmp_path_factory.mktemp("keys"))


@pytest.fixture
def downloader(tmp_path: Path) -> Path:
    return write_downloader(tmp_path / "downloader.py")


@pytest.fixture
def new_installation(
    tmp_path: Path, keys: Path, shim_bin: Path, host_platform: str
) -> NewInstallation:
    built = 0

    def _new(
        label: str = "self",
        *,
        secret: str = "release",
        launcher_source: Path | None = None,
    ) -> Installation:
        nonlocal built
        built += 1
        return make_installation(
            tmp_path / f"root{built}",
            tmp_path / f"server{built}",
            keys=keys,
            shim=shim_bin,
            alias=host_platform,
            label=label,
            secret=secret,
            launcher_source=launcher_source,
        )

    return _new
```

Each module that uses them declares, below its imports:

```python
pytest_plugins = ("tests.integration.support.installation_fixtures",)
```

#### 2. `make_installation` loses its unread parameter

**File**: `tests/integration/support/installation.py`
**Changes**: remove the `bootstrap: Path` keyword from
`make_installation`. The function already calls
`copy_bootstrap(root / "bin/accelerator")` itself.

#### 3. The entrypoint suite

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**:

- add the `pytest_plugins` declaration;
- delete the local `host_platform`, `shim_bin`, `keys`, `downloader` and
  `make_harness` fixtures, and the `_resolve_host_platform`,
  `generate_keys`, `make_installation`, `write_downloader` and
  `build_shim` imports they alone used;
- make three renames throughout, so the suite has one word for the thing:
  - the fixture `make_harness` to `new_installation`;
  - the type `Harness` to `Installation`, with each
    `Callable[..., Harness]` annotation becoming `NewInstallation`;
  - the local `harness` (188 uses) to `installation`. The module already
    uses `installation` 12 times; check each for a clash in the same
    function;
- at the 6 call sites that pass `real_launcher=True`, take the
  `launcher_bin` fixture and pass `launcher_source=launcher_bin`;
- keep `launcher_bin`, `bootstrap_src` (which `_bare_root` uses) and
  `repo_bin_is_untouched`.

The roots keep their `root{n}` / `server{n}` names under `tmp_path`.

#### 4. The skill-invocation suite

**File**: `tests/integration/skill-invocation/test_skill_invocation_conformance.py`
**Changes**: add the `pytest_plugins` declaration, and build the
module-scoped `installation` from the shared fixtures. It keeps its own
fixture because it serves one installation to every case.

```python
@pytest.fixture(scope="module")
def installation(
    tmp_path_factory: pytest.TempPathFactory,
    keys: Path,
    shim_bin: Path,
    host_platform: str,
) -> Installation:
    base = tmp_path_factory.mktemp("conformance")
    return make_installation(
        base / "root",
        base / "server",
        keys=keys,
        shim=shim_bin,
        alias=host_platform,
        launcher_source=build_launcher(),
        templates=REPO_ROOT / "templates",
    )
```

Drop the `copy_bootstrap`, `generate_keys`, `host_platform` and
`build_shim` imports.

### Success Criteria

Before the refactor, note each suite's summary line: run the two module
commands below and keep the passed and skipped counts.

#### Automated Verification

- [ ] The entrypoint module passes alone:
      `uv run pytest tests/integration/entrypoint/test_accelerator_entrypoint.py -q -rs`
- [ ] The skill-invocation module passes alone:
      `uv run pytest tests/integration/skill-invocation/test_skill_invocation_conformance.py -q -rs`
- [ ] Both suites pass in one process, where the plugin is named twice:
      `uv run pytest tests/integration/entrypoint tests/integration/skill-invocation`
- [ ] No caller still passes `bootstrap=`:
      `grep -rn "bootstrap=" tests/integration` finds nothing
- [ ] `mise run check` passes (ruff, pyrefly)
- [ ] Full run green: `mise run`

#### Manual Verification

- [ ] Each module's passed count matches the count noted before the
      refactor, and neither run reports a skip
- [ ] The pull request's diff of the entrypoint module holds only the
      fixture removals, the three renames and the 6 `launcher_source` call
      sites

---

## Phase 3: The committed shim for this host verifies signatures (PR 3)

### Overview

Run the vendored `bin/accelerator-verify-<platform>` for the running host
against per-run keys. It accepts a release-key signature, and refuses an
attacker-key signature and a payload changed after signing. The
integration job's two lanes then execute the `linux-x64` and `darwin-arm64`
shims on every pull request, with no network.

This phase comes before the smoke-test rewrite so that no revision is left
without an executed vendored shim. It depends only on Phase 2's fixtures.

### Changes Required

#### 1. The vendored-shim tests

**File**: `tests/integration/entrypoint/test_vendored_shim.py` (new)
**Changes**: three tests over a small payload. They run under the existing
`test:integration:entrypoint` task, which needs no new dependency.

```python
"""The committed verify shim for this host, checked against per-run keys."""

import os
import subprocess
from pathlib import Path

import pytest

from tasks.shared.paths import vendored_shim_path
from tests.integration.support.installation import sig_path, sign

pytest_plugins = ("tests.integration.support.installation_fixtures",)

_REFUSAL = "signature verification failed"


@pytest.fixture
def vendored_shim(host_platform: str) -> Path:
    shim = vendored_shim_path(host_platform)
    assert os.access(shim, os.X_OK), f"{shim} is not executable"
    return shim


@pytest.fixture
def payload(tmp_path: Path) -> Path:
    payload = tmp_path / "payload"
    payload.write_bytes(b"launcher bytes")
    return payload


def _verify(
    shim: Path, public_key: Path, target: Path
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [shim, public_key, sig_path(target), target],
        capture_output=True,
        text=True,
        check=False,
    )


def _assert_refused(verification: subprocess.CompletedProcess[str]) -> None:
    assert verification.returncode == 1, verification.stderr
    assert _REFUSAL in verification.stderr


def test_the_vendored_shim_accepts_a_release_key_signature(
    vendored_shim: Path, keys: Path, payload: Path
) -> None:
    sign(keys / "release.key", payload)

    verified = _verify(vendored_shim, keys / "release.pub", payload)

    assert verified.returncode == 0, verified.stderr


def test_the_vendored_shim_refuses_a_non_release_key_signature(
    vendored_shim: Path, keys: Path, payload: Path
) -> None:
    sign(keys / "attacker.key", payload)

    _assert_refused(_verify(vendored_shim, keys / "release.pub", payload))


def test_the_vendored_shim_refuses_a_payload_changed_after_signing(
    vendored_shim: Path, keys: Path, payload: Path
) -> None:
    sign(keys / "release.key", payload)
    payload.write_bytes(b"tampered bytes")

    _assert_refused(_verify(vendored_shim, keys / "release.pub", payload))
```

Notes:

- **The tampered payload is the case that reaches the signature check.** A
  signature from another key can be refused on its key id alone, before
  any hashing. The correctly keyed signature over changed bytes cannot.
- **Exit status 1 and the refusal text together** separate a refused
  signature from a usage error or an unreadable file, which also exit 1.
- **A contributor on `darwin-x64` or `linux-arm64` runs that platform's
  shim**, which no CI lane does. That is intended. The executable-bit
  assertion makes a mode or mount problem read differently from a
  verification failure.

### Red Step (mutation checks)

Apply each row alone, watch the named assertion fail, and restore.

| Break | Assertion that must fail |
|---|---|
| `chmod -x` a copy of the shim and point `vendored_shim` at it | the executable-bit assertion |
| In the accepting test, sign with `attacker.key` | `verified.returncode == 0` |
| In the non-release-key test, verify against `attacker.pub` | `returncode == 1` in `_assert_refused` |
| In the tampered test, delete the `write_bytes` line | `returncode == 1` in `_assert_refused` |
| In a refusing test, delete the `.minisig` before verifying | the refusal text in `_assert_refused` (the shim exits 1 with `cannot read`) |

Then run the module alone with `-rs`; all three tests pass and none is
skipped.

### Success Criteria

#### Automated Verification

- [ ] The new module passes alone:
      `uv run pytest tests/integration/entrypoint/test_vendored_shim.py -v -rs`
- [ ] The entrypoint suite passes: `mise run test:integration:entrypoint`
- [ ] `mise run check` passes
- [ ] Full run green: `mise run`

#### Manual Verification

- [ ] Every row of the mutation table failed at its named assertion before
      being restored
- [ ] The module's run reported no skipped test
- [ ] On PR 3's `Main` run, the new module's three tests pass on both the
      ubuntu and the macOS integration lanes

---

## Phase 4: Hermetic launcher-dispatch smoke test (PR 3)

### Overview

Rebuild the smoke test on the shared fixtures and `run_bootstrap`. It
serves a claimed copy of the working-tree debug launcher, signed with the
per-run release key, through the stub downloader. It then asserts the full
hook contract. The hooks task stops passing an environment overlay, and the
module takes a name that covers what it holds.

### Changes Required

#### 1. The hooks task (red first)

**File**: `tests/unit/tasks/test_hooks_task_environment.py` (new)
**Changes**: a failing test that the hooks task hands its suite no
environment overlay. It stays after the change: each hooks module composes
its own environment, and an overlay added back would be an input none of
them declares.

```python
from invoke import Context

from tasks.test.integration import hooks


def test_the_hooks_suite_is_given_no_environment_overlay(mocker):
    context = mocker.MagicMock(spec=Context)

    hooks(context)

    assert "env" not in context.run.call_args.kwargs
```

It fails today, because the task passes `env=accelerator_env(vcs_bin=True)`.

**File**: `tasks/test/integration.py:120-133`
**Changes**: run the suite with no overlay, and describe only what the
task supplies.

```python
@task
def hooks(context: Context) -> None:
    """Integration tests for the hooks/ subtree.

    The suite runs the launcher, accelerator-vcs and accelerator-research
    built by the build:cli:dev, build:cli:vcs:dev and build:cli:research:dev
    dependencies.
    """
    context.run("uv run pytest tests/integration/hooks -v")
```

**File**: `tasks/test/helpers.py:9-62`
**Changes**: with the test green, remove the `vcs_bin` parameter, its
branch and its docstring paragraph from `accelerator_env`. Nothing else
passes it.

#### 2. A claim for task-built binaries

**File**: `tests/support/artefacts.py`
**Changes**: give `claim_artefact` an optional claimed name, and add one
function that owns the `cli/target/debug` path for a binary a mise edge
built.

```python
def claim_artefact(built: Path, claimed_name: str | None = None) -> Path:
    """Copy a freshly built binary out of cargo's uplift path.

    Only the `open` has to land while the link is live, so a source that has
    vanished into the re-link window is retried rather than fatal.
    """
    claimed = Path(_ARTEFACTS.name) / (claimed_name or built.name)
    ...


def claim_task_built(binary: str) -> Path:
    """Claim a binary that a mise build edge produced before the suite ran.

    Claimed under its own name, because `build_launcher()` memoises its claim
    of `accelerator`: a second copy landing on that path would swap the binary
    under every module already holding it.
    """
    return claim_artefact(
        CLI_TARGET_DIR / "debug" / binary, f"task-built-{binary}"
    )
```

The rest of `claim_artefact`'s body is unchanged.

#### 3. Module rename and rewrite

**File**: `tests/integration/hooks/test_vcs_detect_hook.py`, renamed from
`test_vcs_detect_registration.py`
**Changes**:

- rename the file. Nothing outside `meta/` names it;
- remove `_dispatch_env`, the `LAUNCHER` constant, the local `REPO_ROOT`
  and the `os` import. `subprocess` stays, for `_detect`'s annotation;
- derive `_DETECT_COMMAND` from `_DETECT_ARGS`, so the smoke test runs
  exactly the registered hook command;
- load the shared fixtures;
- rewrite the smoke test;
- leave the registration test's body unchanged.

```python
"""The `SessionStart` vcs-detect hook: registered once, and run end to end.

The hook command runs through a copy of `bin/accelerator` into the launcher
built from this tree, fetched from a stub release server and verified against
a per-run key. The launcher's own verification of `accelerator-vcs` is not
traversed: it checks against the committed key embedded at build time, which
no per-run key can satisfy. Its `dispatch_failure_policy.rs` covers that.
"""

import filecmp
import json
import subprocess
from pathlib import Path

from tests.integration.support.installation import (
    REPO_ROOT,
    VERSION,
    Installation,
    download_log,
    run_bootstrap,
)
from tests.integration.support.installation_fixtures import NewInstallation
from tests.support.artefacts import claim_task_built

pytest_plugins = ("tests.integration.support.installation_fixtures",)

HOOKS_JSON = REPO_ROOT / "hooks/hooks.json"

_DETECT_ARGS = (
    "vcs",
    "detect",
    "--format=hook",
    "--fail-safe",
    "--descriptive",
)
_DETECT_COMMAND = "${CLAUDE_PLUGIN_ROOT}/bin/accelerator " + " ".join(
    _DETECT_ARGS
)
_GIT_REFERENCE_OPENING = (
    "This repository uses git as its version control system."
)
_BOUNDARY_BLOCK_MARKERS = (
    "WORKSPACE BOUNDARY DETECTED",
    "do not edit files in",
    "do not run VCS commands against",
    "do not grep, find, or research files in",
)
_SILENCING_THE_OVERRIDE_INFO_LINE = {"ACCELERATOR_LOG": "error"}


def _launcher_and_signature(alias: str) -> list[str]:
    return [f"accelerator-{alias}", f"accelerator-{alias}.minisig"]


def _served_assets(installation: Installation) -> list[str]:
    return [
        url.rsplit("/", 1)[-1] for url in download_log(installation.server)
    ]


def _cached_launcher(installation: Installation, alias: str) -> Path:
    return installation.root / f"bin/accelerator-launcher-{VERSION}-{alias}"


def _assert_outside_any_checkout(directory: Path) -> None:
    ancestors_vcs_detection_walks = directory.parents[:-1]
    enclosing = [
        ancestor
        for ancestor in ancestors_vcs_detection_walks
        if (ancestor / ".git").exists() or (ancestor / ".jj").exists()
    ]
    assert not enclosing, (
        f"{directory} sits inside a checkout at {enclosing[0]}; move the "
        "temporary root (TMPDIR or --basetemp) outside it"
    )


def _vcs_override_env() -> dict[str, str]:
    return {"ACCELERATOR_VCS_BIN": str(claim_task_built("accelerator-vcs"))}


def _detect(
    installation: Installation,
    downloader: Path,
    *,
    env: dict[str, str] | None = None,
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    return run_bootstrap(
        installation.root,
        installation.server,
        downloader,
        args=_DETECT_ARGS,
        extra_env=env,
        cwd=cwd,
    )


def test_launcher_dispatch_smoke_on_a_plain_non_repo_dir(
    new_installation: NewInstallation,
    downloader: Path,
    host_platform: str,
    tmp_path: Path,
) -> None:
    launcher = claim_task_built("accelerator")
    installation = new_installation(launcher_source=launcher)
    project = tmp_path / "project"
    project.mkdir()
    _assert_outside_any_checkout(project)

    result = _detect(
        installation,
        downloader,
        env=_vcs_override_env() | _SILENCING_THE_OVERRIDE_INFO_LINE,
        cwd=project,
    )

    assert result.returncode == 0, result.stderr
    assert result.stderr == ""
    assert _served_assets(installation) == _launcher_and_signature(
        host_platform
    )
    assert filecmp.cmp(
        _cached_launcher(installation, host_platform), launcher, shallow=False
    ), "the cached launcher is not the one served"
    hook = json.loads(result.stdout)["hookSpecificOutput"]
    assert hook["hookEventName"] == "SessionStart"
    context = hook["additionalContext"]
    assert context.startswith(_GIT_REFERENCE_OPENING), context
    for marker in _BOUNDARY_BLOCK_MARKERS:
        assert marker not in context
```

Notes:

- **The exit status and stderr are asserted first.** A refusal under
  `--fail-safe` exits 0 with the reason on stderr, so the stderr assertion
  is the one that reports it. Checking the cache first would bury it under
  a `FileNotFoundError`.
- **`cwd`** is an empty directory under `tmp_path`.
  `_assert_outside_any_checkout` turns a temporary root inside a checkout
  into a failure that names the cause, where it would otherwise look like
  a detection bug. It skips the filesystem root, as VCS detection does.
- **`ACCELERATOR_CACHE_DIR`** is not set. The cache defaults to
  `<root>/bin`, inside the test's temporary root.
- **No `is_file()` pre-check on the built binaries.** It would race cargo's
  re-link, which is what the claim retries through. An unbuilt tree fails
  with `not built: <path>`.
- **`filecmp`** compares the 76 MB launcher without dumping bytes on
  failure.
- **`_BOUNDARY_BLOCK_MARKERS`** folds the heading into the prohibitions the
  old test already checked. They are one block, emitted together.

### Red Step (mutation checks)

Apply each row alone, watch the named assertion fail, and restore. Rows
that build a repository do so *after* `_assert_outside_any_checkout` has
run, so the guard does not fail first.

| Break | Assertion that must fail |
|---|---|
| Create `tmp_path / ".git"` before the guard | the guard in `_assert_outside_any_checkout` |
| Serve an attacker-signed launcher (`secret="attacker"`) and drop `--fail-safe` from the arguments | `returncode == 0` |
| Drop `_SILENCING_THE_OVERRIDE_INFO_LINE` from the environment | `stderr == ""` (the override's INFO line) |
| Call `_detect` a second time after deleting the cached launcher | the served-assets comparison (four entries) |
| Serve the stub (`new_installation()` with no `launcher_source`) | the byte comparison |
| Point `ACCELERATOR_VCS_BIN` at an executable script that prints `{"systemMessage": "x"}` | the `["hookSpecificOutput"]` lookup |
| Point it at a script that prints the envelope with `"hookEventName": "PreToolUse"` | `hookEventName == "SessionStart"` |
| After the guard, run `jj git init` in `project` | the git-reference opening |
| After the guard, replace `project` with a git worktree (commands below) | the boundary-block markers |

The worktree row:

```bash
rmdir project
git init main
git -C main -c user.name=t -c user.email=t@example.invalid \
  -c commit.gpgsign=false commit --allow-empty -m initial
git -C main worktree add ../project
```

Then run the module alone:
`uv run pytest tests/integration/hooks/test_vcs_detect_hook.py -v -rs`.
Both tests pass and neither is skipped.

### Success Criteria

#### Automated Verification

- [ ] The task test passes:
      `uv run pytest tests/unit/tasks/test_hooks_task_environment.py`
- [ ] The rewritten module passes alone:
      `uv run pytest tests/integration/hooks/test_vcs_detect_hook.py -v -rs`
- [ ] The hooks suite passes: `mise run test:integration:hooks`
- [ ] Nothing still names the removed overlay:
      `grep -rnw "vcs_bin" tasks` finds nothing
- [ ] The entrypoint and hooks suites pass in one process, where both
      claim a launcher:
      `uv run pytest tests/integration/entrypoint tests/integration/hooks`
- [ ] `mise run check` passes (ruff, pyrefly)
- [ ] Full run green: `mise run`

#### Manual Verification

- [ ] The task test was seen to fail before the task change
- [ ] Every row of the mutation table failed at its named assertion before
      being restored
- [ ] The module's run reported no skipped test
- [ ] The hooks suite's duration is recorded in the PR against its 2.7 s
      warm baseline, three times:
      - `mise run --skip-deps test:integration:hooks` alone on a warm cargo
        cache;
      - the task's `Finished in` from one warm full `mise run`, as an
        indication only, since it is one sample of a wait on cargo's lock;
      - the task's `Finished in` from PR 3's ubuntu `Main` run, read as in
        Phase 1

---

## Phase 5: Refusal of a non-release-key launcher under `--fail-safe` (PR 3)

### Overview

Add a test that serves the stub launcher signed with the per-run attacker
key. It asserts every observable sign of refusal under the hook's
`--fail-safe`, then re-signs with the release key as a control. Correct the
sibling module's docstring, which says this cannot be done offline.

### Changes Required

#### 1. The refusal test

**File**: `tests/integration/hooks/test_vcs_detect_hook.py`
**Changes**: add `sign` to the `installation` imports, and add the test
below. The module docstring gains a third paragraph:

```python
"""The `SessionStart` vcs-detect hook: registered once, and run end to end.

The hook command runs through a copy of `bin/accelerator` into the launcher
built from this tree, fetched from a stub release server and verified against
a per-run key. The launcher's own verification of `accelerator-vcs` is not
traversed: it checks against the committed key embedded at build time, which
no per-run key can satisfy. Its `dispatch_failure_policy.rs` covers that.

The hook's refusal of a launcher signed with another key sits here too. Every
hook reaches the bootstrap under `--fail-safe`, where the exit status cannot
show a refusal.
"""
```

```python
def _refusal_diagnostic(alias: str) -> str:
    return f"could not fetch and verify the accelerator launcher for {alias}"


def test_a_launcher_signed_with_a_non_release_key_is_refused_under_fail_safe(
    new_installation: NewInstallation,
    downloader: Path,
    keys: Path,
    host_platform: str,
    tmp_path: Path,
) -> None:
    installation = new_installation(secret="attacker")
    cache = installation.root / "bin"
    unverified_log = cache / ".accelerator-unverified.log"
    launcher_record = tmp_path / "launcher-args"
    recording_launcher_args = {"LAUNCHER_ARGS_OUT": str(launcher_record)}

    refused = _detect(installation, downloader, env=recording_launcher_args)

    assert _served_assets(installation) == _launcher_and_signature(
        host_platform
    ), refused.stderr
    assert refused.returncode == 0, refused.stderr
    assert _refusal_diagnostic(host_platform) in refused.stderr
    assert unverified_log.exists(), refused.stderr
    entries = unverified_log.read_text().splitlines()
    assert len(entries) == 1, entries
    assert _refusal_diagnostic(host_platform) in entries[0]
    assert not launcher_record.exists(), "the refused launcher ran"
    for leftover in (
        "accelerator-launcher-*",
        ".tmp-launcher-*",
        ".accelerator-lock-*",
    ):
        assert not list(cache.glob(leftover)), leftover

    sign(
        keys / "release.key",
        installation.server / f"accelerator-{host_platform}",
    )
    accepted = _detect(installation, downloader, env=recording_launcher_args)

    assert launcher_record.exists(), accepted.stderr
    assert launcher_record.read_text().splitlines() == list(_DETECT_ARGS)
```

Notes:

- **No launcher build and no `ACCELERATOR_VCS_BIN`.** The stub never
  dispatches, so the test needs neither.
- **The cache assertions target names, not an empty directory.** The
  bootstrap, the shim, its staged `accelerator-verify-<alias>-<digest>`
  copy and the unverified log legitimately remain. The lock directory does
  not: the bootstrap releases it before it reports the refusal, and a
  leaked one would stall the next session.
- **The log did not exist before the call**, so one entry is one new
  entry.
- **The control asserts the forwarded argv**, not only that a record
  exists. This shows the stub ran as the hook's dispatch target.
- **Why this module.** The test reuses the smoke test's helpers and the
  hook's own command, and the module is named for the hook, not for its
  registration.

#### 2. Sibling docstring

**File**: `tests/integration/hooks/test_research_guard_registration.py:6-8`
**Changes**: the claim that a real signature failure cannot be produced
offline is now false. Replace the last sentence.

```python
"""The research guard's `PreToolUse` registration, smoke-run end to end.

Each run goes through the real `bin/accelerator` bootstrap into the launcher
built from this tree, reached through the contributor override in a temporary
plugin root, since the bootstrap admits an override only inside its own
`cli/target/`. The launcher's own integrity refusals are exercised in its
`dispatch_failure_policy.rs`.
"""
```

#### 3. PR 3's description

State what moves and what stays, in these terms:

- The smoke test was the only blocking pull-request check of the committed
  public key against a genuine release signature. That check now runs
  only in `prerelease:finalise` / `release:finalise`, with the darwin-arm64
  shim, after the merge and after the version commit and tag are pushed.
  The non-blocking `check-measure-harness` job still runs the real
  bootstrap against a published release on both lanes, as an advisory
  signal.
- `test_launcher_reverify_failure_preserves_draft` shows that a failed
  re-verification keeps the release in draft.
  `test_accepts_a_committed_key_signature` and
  `test_rejects_a_non_committed_key_signature`
  (`tests/integration/tasks/test_github.py:694-728`) show the
  re-verification accepting and refusing, with a patched shim and key.
- The `darwin-x64` and `linux-arm64` vendored shims are executed on no CI
  lane.

### Red Step (mutation checks)

Apply each row alone, watch the named assertion fail, and restore. Planted
files take the decoy names given here. Do not plant the bootstrap's own
lock path, `.accelerator-lock-<platform>`: the bootstrap would then stop
before fetching, wait out its lock timeout, or reclaim the lock and pass.

| Break | Assertion that must fail |
|---|---|
| Delete the served launcher before the first call | the served-assets comparison (one entry) |
| Drop `--fail-safe` from the arguments | `refused.returncode == 0` |
| Sign with the release key (`new_installation()`) | the refusal diagnostic in stderr |
| Add `ACCELERATOR_CACHE_DIR`, naming another directory, to the environment | `unverified_log.exists()` |
| Write `"earlier entry\n"` (with its newline) to the unverified log before the call | `len(entries) == 1` |
| Edit the expected string in the `entries[0]` assertion | the diagnostic in the log entry |
| Write `launcher_record` before the call | `not launcher_record.exists()` |
| Plant `cache / "accelerator-launcher-planted"` | the `accelerator-launcher-*` leftover |
| Plant `cache / ".tmp-launcher-planted"` | the `.tmp-launcher-*` leftover |
| Plant `cache / ".accelerator-lock-planted"` | the `.accelerator-lock-*` leftover |
| Delete the `sign(...)` line | `launcher_record.exists()`, showing the control's stderr |
| Give the control call different arguments | the forwarded-argv comparison |

The log-entry row edits the expectation because no input separates it from
the stderr assertion: both read the one message `fail_integrity` writes.
The planted-lock row shows the glob matches. It does not show that a real
leaked lock would be caught.

Then run the module alone with `-rs`; all three tests pass and none is
skipped.

### Success Criteria

#### Automated Verification

- [ ] The module passes alone:
      `uv run pytest tests/integration/hooks/test_vcs_detect_hook.py -v -rs`
- [ ] The hooks suite passes: `mise run test:integration:hooks`
- [ ] The entrypoint suite is unaffected:
      `mise run test:integration:entrypoint`
- [ ] `mise run check` passes
- [ ] Full run green: `mise run`

#### Manual Verification

- [ ] Every row of the mutation table failed at its named assertion before
      being restored
- [ ] The module's run reported no skipped test
- [ ] PR 3's description carries the three statements above

---

## Testing Strategy

### Unit Tests

- A pin on `profile.dev.package.minisign-verify.opt-level == 3`, together
  with the workspace dependency it names.
- The hooks task hands its suite no environment overlay.
- The existing drift-marker tests (`tests/unit/tasks/test_build.py:239-245`)
  confirm the new table does not move the vendored-shim marker.

### Integration Tests

- **Vendored-shim tests.** The committed shim for the host accepts a
  release-key signature and refuses an attacker-key signature and a
  tampered payload, on both CI lanes.
- **Smoke test.** A real fetch → verify → cache → exec chain over the stub
  transport, with a per-run trust root and the working-tree launcher. It
  asserts the dispatch (exit status, empty stderr), the transport (download
  log), the cache (byte identity) and the hook contract (envelope, git
  reference, no boundary block).
- **Refusal test.** It asserts transport, exit status, stderr, the durable
  unverified record, non-execution and an untainted, unlocked cache, then
  runs a positive control in the same installation.
- **Regressions.** The entrypoint suite's refusal and healing cases, and
  the whole skill-invocation suite, run on the optimised shim and, from
  Phase 2, on the shared fixtures.

### Manual Testing Steps

1. Work through the mutation tables in Phases 3, 4 and 5.
2. Take the shim, local-suite and CI timings in Phase 1.
3. Take the hooks-suite timings in Phase 4.

## Performance Considerations

- **The override forces a one-off rebuild.** `minisign-verify` and
  everything that depends on it (the shim, the launcher and the launcher's
  test binaries) recompile once in each configuration derived from the dev
  profile: the plain debug build, tests, coverage and the lint builds. The
  first full `mise run` and the first CI run after PR 1 pay for that, per
  cargo cache. No local measurement is taken from such a run. The one CI
  measurement may be, and the PR then says so.
- **The hooks suite gains an in-pytest `build_shim()` cargo build.** It is a
  no-op when the cache is warm. Under a full run it can wait on cargo's
  target lock, as the entrypoint and skill-invocation suites already do,
  and `build_shim()` runs quietly, so the wait is invisible. Phase 4
  records the suite's duration against its 2.7 s baseline for that reason.
- **The hooks suite stops fetching 17 MB from GitHub.** Its remaining cost is
  three bootstrap runs: one verifies the 76 MB launcher with the optimised
  shim, and two verify the small stub.
- **The smoke test moves the 76 MB launcher three times** (claim, serve,
  cache), well under a second in all.
- **The vendored-shim module builds nothing.** It generates one key pair
  set and verifies a 14-byte payload three times.

## Migration Notes

- **0366 lands after this.** It must keep the `minisign-verify` override
  while it edits the dev-profile table, and the pin test enforces that. Its
  own dev-profile pins belong in `test_cargo_profiles.py`.
- **The shim has one fixture.** If 0366 brings the debug shim under a mise
  task, `shim_bin` in `installation_fixtures.py` is the one place that
  switches to that edge, for all three suites.
- **Re-measuring before this item still needs the network.** Measuring 0361
  at a commit before PR 3 still fetches from GitHub.

## References

- Work item: `meta/work/0362-make-the-bootstrap-test-suites-fast-and-hermetic.md`
- Codebase research:
  `meta/research/codebase/2026-11-10-0362-bootstrap-test-suites-fast-and-hermetic.md`
- Source research:
  `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
  (A1, A7)
- Review:
  `meta/reviews/plans/2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic-review-1.md`
- Patterns:
  - `tests/integration/entrypoint/test_accelerator_entrypoint.py:206-262`
    (happy path, attacker key)
  - `tests/integration/entrypoint/test_accelerator_entrypoint.py:1217-1237`
    (durable record under `--fail-safe`)
  - `tests/integration/skill-invocation/test_skill_invocation_conformance.py:68-113`
    (module-scoped installation)
  - `tests/unit/tasks/test_nextest_filter.py` (a module pinning a
    configuration file)
