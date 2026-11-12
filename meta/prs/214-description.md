---
type: "pr-description"
id: "214"
title: "[0362] Run the hooks smoke test offline and check the committed verify shim"
date: "2026-11-12T20:19:49+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0362"
parent: "work-item:0362"
relates_to: ["work-item:0361", "work-item:0366", "work-item:0370"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/214"
pr_number: 214
tags: ["build-system", "testing", "bootstrap", "hooks", "minisign", "hermetic"]
revision: "f6ff2a043df78c50eb42074726ab199a57d8fec1"
repository: "accelerator"
last_updated: "2026-11-12T21:40:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0362] Run the hooks smoke test offline and check the committed verify shim

## Summary

The hooks suite's launcher-dispatch smoke test ran the repository's own
`bin/accelerator`, so it downloaded the published launcher from GitHub (17 MB,
42 s on the research workstation). It also never checked that the launcher it
ran was the one it fetched. This PR rebuilds the test on a stub release
server, which serves the launcher built from this tree, signed with a per-run
key. The test now asserts the full `SessionStart` hook contract. Beside it, a
new test shows the bootstrap refusing a launcher signed with another key under
`--fail-safe`, the mode every hook runs in.

That download was also the only blocking pull-request check that executed the
committed verify shim. A new test keeps that shim executed on both CI lanes,
with no network.

This is the third of three PRs for 0362, stacked on #212, which is stacked
on #210.

## Merge Danger

**Door:** two-way

Only tests, one test task and the plan are touched. Reverting brings back the
network-bound smoke test, and nothing is stored or migrated.

**Blast Radius:** narrow

- **No shipped code changes.** `bin/accelerator`, `cli/` and `hooks/` are
  untouched.
- **One task changes.** `test:integration:hooks` no longer passes the suite
  an environment overlay. No hooks module read it after this change: the
  research-guard test strips every `ACCELERATOR_*` variable, and the
  link-refresh test builds its own environment. A unit test pins this.
- **A pull request now checks less against the real release.** See the
  first note for reviewers.

## Changes

- **The committed verify shim for this host is tested**
  (`tests/integration/entrypoint/test_vendored_shim.py`, new). It runs
  `bin/accelerator-verify-<platform>` against per-run keys and a small
  payload. The shim must accept a release-key signature, and refuse both an
  attacker-key signature and a payload changed after signing.
  - The tampered payload is the case that reaches the hash: a signature from
    another key can be refused on its key id alone.
  - Each refusal asserts exit status 1 *and* the
    `signature verification failed` text, because a usage error or an
    unreadable file also exits 1.
  - The ubuntu and macOS integration lanes therefore execute the
    `linux-x64` and `darwin-arm64` shims on every pull request.
- **The smoke test runs offline.**
  `test_vcs_detect_registration.py` is renamed to `test_vcs_detect_hook.py`,
  since it now covers the hook's behaviour, not only its registration. The
  rewritten smoke test:
  - serves a claimed copy of the launcher that `build:cli:dev` builds, signed
    with the per-run release key, through the stub downloader;
  - asserts exit 0 and empty stderr first, because a refusal under
    `--fail-safe` exits 0 with its reason on stderr;
  - asserts that exactly the launcher and its signature were downloaded, and
    that the cached launcher is byte-identical to the one served;
  - parses the `hookSpecificOutput` envelope, so the bare `systemMessage`
    that a swallowed adapter error produces fails the test. It then checks
    the event name, the git reference text and the absence of a workspace
    boundary block;
  - runs in an empty directory, after a guard that fails with a clear
    message if pytest's temporary root sits inside a checkout;
  - runs exactly the registered hook command: `_DETECT_COMMAND` is now built
    from the same argument tuple.
- **A launcher signed with another key is refused under `--fail-safe`.**
  The new test serves the stub launcher signed with the per-run attacker
  key, then asserts:
  - only the launcher and signature were fetched, and the call exited 0;
  - the refusal diagnostic is on stderr, and the unverified log gained
    exactly one entry carrying it;
  - the refused launcher never ran;
  - the cache holds no launcher, no `.tmp-launcher-*` file and no
    `.accelerator-lock-*` directory. A leaked lock would stall the next
    session.

  As a control, the test re-signs the same asset with the release key and
  runs the hook again. The stub must then run and receive exactly the hook's
  arguments.
- **`ACCELERATOR_VCS_BIN` is set by the test, not the task.** The launcher
  would otherwise verify `accelerator-vcs` against the committed key built
  into it, which no per-run key can satisfy. With the override, the
  sub-binary is resolved before any key check. The override logs at INFO, so
  the smoke test sets `ACCELERATOR_LOG=error` to keep stderr empty.
- **`claim_task_built`** (`tests/support/artefacts.py`) claims a binary that
  a mise build edge produced under the name `task-built-<binary>`.
  `build_launcher()` memoises its own claim at `accelerator`. In one pytest
  process, a second claim at that path would swap a launcher built without
  `test-loopback` under the entrypoint suite. `claim_artefact` gains an
  optional claimed name to support this.
- **`accelerator_env` loses `vcs_bin`.** The hooks task was its only caller.
- **Comments and docstrings.**
  - The research-guard module's docstring no longer says a real signature
    failure cannot be produced offline.
  - A stale "same harness" reason in `test_mise.py` and a comment that only
    repeated a test name are removed. Both lines came from #212.
- **The plan and its validation.** The plan records Phases 3–5 as
  implemented. The validation report marks the implementation complete,
  apart from the three criteria that need this PR (see Testing).

## Context

- Work item: `meta/work/0362-make-the-bootstrap-test-suites-fast-and-hermetic.md`
- Plan: `meta/plans/2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic.md`
  (Phases 3–5)
- Validation:
  `meta/validations/2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic-validation.md`
- Stacked on #212 (shared installation fixtures), which is stacked on #210
  (`minisign-verify` at `opt-level = 3` in dev builds)

## Testing

- [x] Each module that loads the shared fixtures passes alone with `-rs` and
  no skips. A module that forgets the `pytest_plugins` declaration would
  still pass in a directory run, so each was also run on its own:
  - `test_vendored_shim.py`: 3 passed
  - `test_vcs_detect_hook.py`: 3 passed
  - `test_accelerator_entrypoint.py`: 72 passed
  - `test_skill_invocation_conformance.py`: 143 passed
- [x] The entrypoint and hooks suites pass in one process, where both claim
  a launcher: `uv run pytest tests/integration/entrypoint tests/integration/hooks`
  (108 passed).
- [x] `uv run pytest tests/unit/tasks/test_hooks_task_environment.py`
  passes. It was seen to fail before the task change.
- [x] `grep -rnw "vcs_bin" tasks` finds nothing.
- [x] `mise run check` passes.
- [x] `mise run` exits 0, with no formatter changes. Hooks passed 30,
  entrypoint 78 and skill-invocation 143.
- [x] Mutation checks. These tests cover code that already exists, so no
  failing test could come first. Instead, every assertion was shown to fail
  under a deliberate break, and the break was then reverted:
  - the vendored-shim module, 5 breaks;
  - the smoke test, 9 breaks, including a fake `accelerator-vcs` that prints
    a bare `systemMessage`, a `jj` repository, and a git worktree;
  - the refusal test, 12 breaks.

  Each failed at the assertion the plan names.
- [x] Hooks-suite duration, against its 2.7 s warm baseline:
  - `mise run --skip-deps test:integration:hooks` alone on a warm cargo
    cache: 1.36 s, 1.38 s and 1.44 s wall time, with pytest's summary
    reporting 0.85–1.10 s. These runs had 29 tests, before the refusal test
    was added;
  - within full `mise run` runs, the task's `Finished in` was 2.28 s with 29
    tests, then 2.00 s and 2.08 s with 30. Each is one sample of a wait on
    cargo's lock, so read them as an indication only.
- [x] On this PR's `Main` run (43299443166), the three vendored-shim tests
  pass on both the ubuntu and the macOS integration lanes.
- [x] The hooks task on this PR's `Main` run took 66.53 s on ubuntu and
  37.28 s on macOS, with 30 passed. On the three runs before this PR it took
  11.4–14.6 s on ubuntu and 11.9–15.5 s on macOS.
  - **The extra time is a wait on cargo's lock, not work.** The smoke test
    took 58.8 s on ubuntu, and every other hooks test about a second or less.
    It is the suite's first request for the debug shim, and its in-pytest
    `cargo build` waited while `test:integration:visualiser` compiled its
    tests (20:28:20 to about 20:29:16). The smoke test passed at 20:29:22.
  - **The integration job was no slower.** It took 11m43s on ubuntu and
    24m22s on macOS, against 15–20 min and 21–28 min on the three runs
    before. The hooks suite does not hold the job up.
  - If 0366 gives the debug shim its own mise task, the suite would take the
    shim from a build edge and this wait would go.

## Notes for Reviewers

- **What moves off the pull-request path.**
  - **The real release key.** The old smoke test was the only blocking
    pull-request check of the committed public key against a genuine release
    signature. That check now runs only in `prerelease:finalise` /
    `release:finalise`, with the darwin-arm64 shim. That is after the merge,
    and after the version commit and tag are pushed. The non-blocking
    `check-measure-harness` job still runs the real bootstrap against a
    published release on both lanes, as an advisory signal.
  - **The release re-verification is still tested.**
    `test_launcher_reverify_failure_preserves_draft` shows that a failed
    re-verification keeps the release in draft.
    `test_accepts_a_committed_key_signature` and
    `test_rejects_a_non_committed_key_signature` in
    `tests/integration/tasks/test_github.py` show the re-verification
    accepting and refusing, with a patched shim and key.
  - **Two shims still run on no CI lane.** No lane executes the `darwin-x64`
    or `linux-arm64` vendored shim. Their guards stay static: the drift
    marker, the static-ELF check and the is-committed check. A contributor
    on one of those platforms now runs that platform's shim locally, which
    is the first time either is executed.
- **The hooks suite does not build the launcher itself.** It already
  carries the `build:cli:dev` and `build:cli:vcs:dev` edges, and
  `build_launcher()`'s docstring forbids pairing the two. It claims those
  task-built binaries instead. It still builds the debug shim in pytest
  through `shim_bin`; whether the shim gains its own mise task is 0366's
  decision.
- **Disk use.** The smoke test leaves about 150 MB under `tmp_path` per run,
  and pytest keeps the last three runs. The entrypoint suite's real-launcher
  tests already keep several times that.
- **Merge order.** Merge #210, then #212, then this PR. GitHub retargets each
  PR as its base merges.
