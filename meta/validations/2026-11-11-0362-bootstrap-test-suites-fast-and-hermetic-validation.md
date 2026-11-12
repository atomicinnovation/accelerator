---
type: "plan-validation"
id: "2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic-validation"
title: "Validation Report: Make the Bootstrap Test Suites Fast and Hermetic Implementation Plan"
date: "2026-11-12T20:17:47+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic"
tags: ["build-system", "performance", "testing", "bootstrap", "hooks"]
last_updated: "2026-11-12T20:17:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Make the Bootstrap Test Suites Fast and Hermetic

### Implementation Status

✓ Phase 1: Optimise `minisign-verify` in dev builds — fully implemented
(PR #210, open)
✓ Phase 2: Shared installation fixtures — fully implemented (PR #212, open)
✓ Phase 3: The committed shim for this host verifies signatures — code
complete; CI confirmation pending PR 3
✓ Phase 4: Hermetic launcher-dispatch smoke test — code complete; CI timing
pending PR 3
✓ Phase 5: Refusal of a non-release-key launcher under `--fail-safe` — code
complete; PR description pending PR 3

The result is `partial` only because PR 3 has not been raised. All five
phases' code and local criteria are done. Three criteria can only be met
from PR 3's description and its `Main` run.

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
  changes, but the commit sits on top of PR 3's changes.
- The plan's Phase 4 figures record 29 hooks tests. With Phase 5's test
  there are now 30.

#### Potential Issues:

- None in the implementation.
- The coverage gaps the plan accepts remain, and PR 3's description must
  state them:
  - the committed public key is checked against a genuine release signature
    only after merge;
  - the `darwin-x64` and `linux-arm64` vendored shims run on no CI lane.
- A stale `tests/integration/hooks/__pycache__/test_zz_mutant.cpython-314-pytest-9.0.3.pyc`
  is left from mutation testing in an earlier session. pytest does not
  collect it, and it is gitignored.

### Manual Testing Required:

1. PR 3, once raised:
  - [ ] On its `Main` run, `test_vendored_shim.py`'s three tests pass on both
        the ubuntu and macOS integration lanes (Phase 3)
  - [ ] Record the hooks task's `Finished in` from the ubuntu `Main` run. Put
        it in the description beside the local figures held in the plan
        (1.36 s, 1.38 s and 1.44 s, and 2.28 s under a full run) and the
        2.7 s baseline (Phase 4)
  - [ ] The description carries Phase 5's three statements: where the
        release-signature check now runs, the `test_github.py` coverage of
        re-verification, and the shims no CI lane executes

### Recommendations:

- Consider squashing `lxqztmsv` into PR #212's changes, where the lines it
  fixes were introduced.
- Raise PR 3 from the commits after PR #212. Then tick the three pending
  criteria and set the plan's status to `done`.
