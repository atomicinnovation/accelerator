---
type: "plan-validation"
id: "2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic-validation"
title: "Validation Report: Make the Bootstrap Test Suites Fast and Hermetic Implementation Plan"
date: "2026-11-12T20:17:47+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic"
tags: ["build-system", "performance", "testing", "bootstrap", "hooks"]
last_updated: "2026-11-12T21:40:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Make the Bootstrap Test Suites Fast and Hermetic

### Implementation Status

✓ Phase 1: Optimise `minisign-verify` in dev builds — fully implemented
(PR #210, open)
✓ Phase 2: Shared installation fixtures — fully implemented (PR #212, open)
✓ Phase 3: The committed shim for this host verifies signatures — fully
implemented (PR #214, open)
✓ Phase 4: Hermetic launcher-dispatch smoke test — fully implemented
(PR #214)
✓ Phase 5: Refusal of a non-release-key launcher under `--fail-safe` — fully
implemented (PR #214)

The first version of this report was `partial`, because three criteria
needed PR 3. #214 is now open, its description carries Phase 5's three
statements, and its `Main` run (43299443166) supplied the other two. Every
success criterion in the plan is met.

### Automated Verification Results

✓ Full run: `mise run` exited 0 (334.9 s) and left the working copy
unchanged. Skill-invocation passed 143, entrypoint 78 and hooks 30.
✓ `uv run pytest tests/unit/tasks/test_cargo_profiles.py`: 1 passed
✓ `uv run pytest tests/unit/tasks/test_build.py`: 47 passed (drift marker
unchanged)
✓ `uv run pytest tests/unit/tasks/test_hooks_task_environment.py`: 1 passed
✓ Each module that declares `pytest_plugins` passes alone with `-rs`, with
no skips:
  - `test_vendored_shim.py`: 3
  - `test_vcs_detect_hook.py`: 3
  - `test_accelerator_entrypoint.py`: 72
  - `test_skill_invocation_conformance.py`: 143
✓ The plugin named twice in one process:
  - `tests/integration/entrypoint tests/integration/skill-invocation`: 221
    passed
  - `tests/integration/entrypoint tests/integration/hooks`: 108 passed
✓ `grep -rn "bootstrap=" tests/integration`: no source hits
✓ `grep -rnw "vcs_bin" tasks`: no hits
✓ `mise run check`: passed as part of the full run
✓ #214's `Main` run 43299443166 succeeded on every job:
  - the three vendored-shim tests passed on both the ubuntu (`linux-x64`)
    and macOS (`darwin-arm64`) integration lanes;
  - the hooks task passed 30 on both lanes.

### Code Review Findings

#### Matches Plan:

- `cli/Cargo.toml` carries `[profile.dev.package.minisign-verify]` with
  `opt-level = 3`. It sits after `[profile.release]`, and its comment
  contains neither `panic` nor `abort`.
- `tests/integration/support/installation_fixtures.py` matches the planned
  module exactly. The entrypoint, vendored-shim, skill-invocation and hooks
  modules all declare it.
- `make_installation` no longer takes `bootstrap`.
- In the entrypoint module:
  - six call sites pass `launcher_source=launcher_bin`;
  - no `make_harness`, `Harness`, `harness` or `real_launcher` remains;
  - `launcher_bin`, `bootstrap_src` and `repo_bin_is_untouched` are kept.
- `test_vendored_shim.py` matches the plan verbatim.
- `tasks/test/integration.py` (`hooks`) passes no `env`, and
  `accelerator_env` no longer offers `vcs_bin`.
- `claim_artefact` takes an optional claimed name, and `claim_task_built`
  claims under `task-built-<binary>`.
- `test_vcs_detect_registration.py` is renamed to `test_vcs_detect_hook.py`.
  Its contents match the plan's code for Phases 4 and 5, apart from ruff
  joining `_served_assets` onto one line.
- The docstring of `test_research_guard_registration.py` matches the plan.

#### Deviations from Plan:

- `pyproject.toml` turns `PytestAssertRewriteWarning` into an error, and
  `tests/conftest.py` registers `installation_fixtures` for assertion
  rewriting (commit `llpkrltw`, Phase 2). The plan did not anticipate that a
  plugin module imported by a test module is not rewritten. The change is
  an improvement, and the PR #212 description explains it.
- `tests/unit/tasks/test_mise.py`: the skill-invocation reason string read
  "same harness", which named the removed entrypoint fixture. It now reads
  "as entrypoint" (commit `lxqztmsv`). The `# cold cache` comment in the
  entrypoint module was also dropped. Both lines came from PR #212's
  changes, but the commit ships in #214.
- The plan's Phase 4 figures record 29 hooks tests. With Phase 5's test
  there are now 30.

#### Potential Issues:

- **The hooks task is slower on CI, but the job is not.** On #214's run it
  took 66.53 s on ubuntu and 37.28 s on macOS. On the three runs before #214
  it took 11.4–14.6 s on ubuntu and 11.9–15.5 s on macOS.
  - On ubuntu, the smoke test alone took 58.8 s. It is the suite's first
    request for `shim_bin`, and its in-pytest `cargo build` waited on
    cargo's lock while `test:integration:visualiser` compiled its tests
    (20:28:20 to about 20:29:16). The smoke test passed at 20:29:22. The
    entrypoint suite had already built the shim at 20:28:16, so the wait was
    not a compile.
  - The plan's performance considerations anticipated this wait, and the
    plan sets no speed bar for the hooks suite.
  - The integration job took 11m43s on ubuntu and 24m22s on macOS, against
    15–20 min and 21–28 min on the three runs before. The hooks suite is not
    what holds the job up.
  - If 0366 gives the debug shim a mise task, `shim_bin` becomes a build
    edge and the wait goes.
- **Coverage the plan accepts.** #214's notes for reviewers state both gaps:
  - the committed public key is checked against a genuine release signature
    only after merge;
  - the `darwin-x64` and `linux-arm64` vendored shims run on no CI lane.
- **`test_the_suite_runs_the_bootstrap_on_the_bash_floor` skips on the
  ubuntu lane.** The entrypoint suite reported 77 passed and 1 skipped on
  ubuntu, and 78 passed on macOS. The same skip appears on #212's run, so
  #214 did not introduce it.
- **A stale cache file** from mutation testing in an earlier session is
  left at
  `tests/integration/hooks/__pycache__/test_zz_mutant.cpython-314-pytest-9.0.3.pyc`.
  pytest does not collect it, and it is gitignored.

### Manual Testing Required:

1. PR 3 (#214):
  - [x] On its `Main` run, `test_vendored_shim.py`'s three tests pass on both
        the ubuntu and macOS integration lanes (Phase 3)
  - [x] The hooks task's `Finished in` from the ubuntu `Main` run is
        recorded in the description: 66.53 s, beside the local figures and
        the 2.7 s baseline (Phase 4)
  - [x] The description carries Phase 5's three statements

### Recommendations:

- Merge #210, then #212, then #214. GitHub retargets each PR as its base
  merges.
- Give 0366 the hooks-suite lock wait as one more reason to put the debug
  shim under a mise task.
