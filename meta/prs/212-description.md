---
type: "pr-description"
id: "212"
title: "[0362] Share the installation fixtures across the bootstrap suites"
date: "2026-11-12T19:22:06+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0362"
parent: "work-item:0362"
relates_to: ["work-item:0361", "work-item:0366", "work-item:0370"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/212"
pr_number: 212
tags: ["build-system", "testing", "bootstrap", "refactor", "pytest-fixtures"]
revision: "398108820863e526b71ecc841d12496fcdff176d"
repository: "accelerator"
last_updated: "2026-11-12T19:22:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0362] Share the installation fixtures across the bootstrap suites

## Summary

Two suites build fixture installations that run the real bootstrap: the
entrypoint suite and the skill-invocation suite. Each wired the host
platform, verify shim, per-run keys and stub downloader by hand. This PR
moves those fixtures into one shared module that both suites load. No
test's behaviour changes.

This is the second of three PRs for 0362, and it is stacked on #210. The
third moves the hooks launcher-dispatch smoke test onto these fixtures.
Without this PR, that suite would become a third hand-wired copy.

## Merge Danger

**Door:** two-way

The change touches only test support and two test modules. Reverting it
restores the hand-wired fixtures, and nothing is stored or migrated.

**Blast Radius:** narrow

- **No production code changes.** Only `tests/` and the plan are touched.
- **The fixture scopes are unchanged.** `host_platform`, `shim_bin` and
  `keys` are still module-scoped, and `downloader` and the factory are still
  per-test. A fixture's `module` scope follows the test module that requests
  it, even when the fixture is defined in a plugin.
- **The pass counts match.** Run alone, the entrypoint module passed 72
  tests and the skill-invocation module 143, before and after the
  refactor. Neither run skipped anything.

## Changes

- **`tests/integration/support/installation_fixtures.py`** (new). This module
  holds the `host_platform`, `shim_bin`, `keys`, `downloader` and
  `new_installation` fixtures. It also holds `NewInstallation`, a `Protocol`
  that types the factory's signature for every test that takes it.
  - The factory now takes the launcher to serve (`launcher_source=`) instead
    of a `real_launcher=True` flag that built one. That lets a suite with the
    `build:cli:dev` edge pass its own claimed copy, which the hooks suite in
    the next PR needs.
- **The entrypoint suite.**
  - It loads the shared module through `pytest_plugins`, and its local copies
    of those fixtures are deleted.
  - Three renames give the suite one word for what it builds:
    `make_harness` → `new_installation`, `Harness` → `Installation`, and the
    local `harness` → `installation`.
  - The six tests that served the real launcher now take the existing
    `launcher_bin` fixture and pass `launcher_source=launcher_bin`.
  - `launcher_bin`, `bootstrap_src` and `repo_bin_is_untouched` stay local.
- **The skill-invocation suite.** Its module-scoped `installation` fixture
  now uses the shared `keys`, `shim_bin` and `host_platform`. It keeps its own
  fixture because one installation serves every case. Its `invoke` fixture
  still writes its own downloader, because the shared `downloader` is
  per-test and `invoke` is module-scoped.
- **`make_installation` loses its `bootstrap` parameter.** The function
  copies the bootstrap into the root itself and never read the parameter.
  Both callers had been building a copy only to pass it in. This has its own
  commit.

The plan records Phase 2 as implemented.

## Context

- Work item: `meta/work/0362-make-the-bootstrap-test-suites-fast-and-hermetic.md`
- Plan: `meta/plans/2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic.md`
  (Phase 2)
- Stacked on #210, which builds `minisign-verify` at `opt-level = 3` in dev
  builds

## Testing

- [x] The entrypoint module passes alone, 72 passed with no skips, as before:
  `uv run pytest tests/integration/entrypoint/test_accelerator_entrypoint.py -q -rs`
- [x] The skill-invocation module passes alone, 143 passed with no skips, as
  before:
  `uv run pytest tests/integration/skill-invocation/test_skill_invocation_conformance.py -q -rs`
- [x] Both suites pass in one process, where two modules name the plugin:
  `uv run pytest tests/integration/entrypoint tests/integration/skill-invocation`
  (218 passed)
- [x] `grep -rn "bootstrap=" tests/integration` finds nothing.
- [x] `mise run check` passes.
- [x] `mise run` exits 0, with no formatter changes.
- [x] The entrypoint module's diff holds only the intended changes. I
  checked this with a script that reversed the renames and compared what was
  left. The only differences were the fixture removals, the import changes,
  the six `launcher_source` call sites, and lines ruff re-wrapped because the
  new names are longer.

## Notes for Reviewers

- **This is the repository's first use of `pytest_plugins`.** It is
  declared in each test module, not in a `conftest.py`. The suites sit in
  sibling directories, so a shared `tests/integration/conftest.py` would hand
  these fixtures to every integration suite. Pytest also refuses
  `pytest_plugins` in a non-root `conftest.py`.
- **A missing declaration can still pass in a directory run.** Pytest
  registers the plugin once for the whole session, however many modules
  declare it. If a sibling module has already loaded it, a module that
  forgets the declaration still passes. That is why each module was also run
  alone.
- **Most of the entrypoint diff is the rename.** `harness` appeared 188
  times, and ruff re-wrapped some lines that the longer name pushed past 80
  columns. The lines that change behaviour are the fixture block near the top
  and the six `new_installation(launcher_source=launcher_bin)` calls.
- **Merge order.** Merge #210 first. GitHub then retargets this PR onto
  `main`.
