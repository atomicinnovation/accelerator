---
type: pr-description
id: "66"
title: "Claim built binaries out of cargo's uplift path"
date: "2026-08-17T08:05:56+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
pr_url: "https://github.com/atomicinnovation/accelerator/pull/66"
pr_number: 66
tags: [tests, cargo, build-system, flakiness]
revision: "d94680ab79164e5781c732694803655cda938cea"
repository: "accelerator"
last_updated: "2026-08-17T08:05:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Claim built binaries out of cargo's uplift path

## Summary

`test:integration:entrypoint` intermittently failed its whole module-scoped shim fixture — 52 errors from one `pytest.fail("not built: cli/target/debug/accelerator-verify")` against a binary that plainly existed, after a `cargo build` that had exited 0.

Cargo unlinks and re-hardlinks `cli/target/debug/<bin>` at the end of **every** build, a no-op one included, so that path is intermittently absent whenever anything else builds in the same workspace. Measured directly: five successive no-op `cargo build -p accelerator-verify` invocations left the path missing on 290 consecutive stat attempts, across 6 distinct inodes. `_cargo_build` released cargo's target lock and then stat-ed that shared path, so a sibling cargo invocation blocked on the very same lock proceeded and re-linked precisely into the check — a correlated race, not a uniformly unlikely one.

Suites now copy each freshly built binary into a per-process directory and run that copy. Nothing was wrong with the shim, the launcher, or the entry point.

## Changes

- **`claim_artefact` copies a freshly built binary out of cargo's uplift path, retrying while the source sits in the re-link window.** Only the `open` has to land while the link is live — unix unlink semantics keep an in-flight read valid — so a vanished source is retried rather than fatal, and no lock or barrier over cargo is needed. The 5-second deadline is three orders of magnitude above the millisecond window it covers, so anything approaching it means the binary was never built at all; `not built` survives as the terminal message for that case.
- **The claim also closes a wider latent failure the original check only hinted at.** The entrypoint suite execs the shim repeatedly over minutes, so a sibling re-link could have broken an exec mid-run, not merely fixture setup. A private copy cannot be pulled out from under a running test.
- **`_cargo_build` memoises per binary name, so a pytest process invokes cargo once.** The entrypoint suite dropped from 156s to 28s, because it no longer queues behind cargo's target lock per fixture. The accepted consequence: editing `cli/` mid-process will not rebuild — pytest processes are short-lived and the next run rebuilds.
- **Three sibling suites carried the identical racy pattern and are now routed through the shared build.** `tests/unit/tasks/test_signing.py`, `tests/unit/tasks/test_manifest.py` and `tests/integration/tasks/test_github.py` each hand-copied the build-then-stat sequence, and all three run concurrently with the entrypoint lane under `mise run` — fixing one lane would have left the same failure live in three others.
- **The shared apparatus is extracted to `tests/support/`, split by concern.** `artefacts.py` owns binaries built from `cli/` (`build_shim`, `build_launcher`, `claim_artefact`); `tools.py` owns external-tool preconditions (`require`, `in_ci`). Dependencies now point one way — suites and `tests/integration/support/installation.py` both depend on `tests/support`, and no unit suite imports integration support. `installation.py` keeps only what it is documented to own: the bootstrap fixture apparatus.
- **Four hand-copied `require`/`in_ci` pairs collapse into one.** They were behaviourally identical, differing only in message ordering in `test_github.py`.
- **`tests/integration/entrypoint/test_built_artefacts.py` pins the three properties.** The claimed path lies outside `cli/target`, is stable across calls, and the claim retries through a source that vanishes — the last driven by patching `shutil.copy2` to raise `FileNotFoundError` twice before delegating, so the retry is proved deterministically rather than by timing.

## Context

No work item: this is a test-infrastructure defect found by a failing `mise run` rather than planned work. It touches only files under `tests/`, and no shipped skill, script, hook, or crate.

## Testing

- [x] `mise run` (the bare default) exits 0 end-to-end, with zero `ERROR task failed` lines.
- [x] The entrypoint suite passes 57/57 **while a concurrent `cargo build` loop hammers the workspace** — the exact condition that produced the failure. Against the old code that same condition is what the 290-miss measurement describes.
- [x] The affected suites together: 123 passed across entrypoint, signing, manifest and github; `test:integration:skill-invocation` 128 passed.
- [x] `mise run build-system:check` clean (ruff, formatting, pyrefly).
- [ ] CI's own parallelism and the linux lane are unverified locally — the fix removes a dependency on inter-process timing rather than adding one, so CI should be strictly less exposed, but only a CI run proves it.
- [ ] Not investigated: whether other suites read binaries straight out of `cli/target` by paths this change does not cover. The four that build via `cargo build` are covered; a path-literal reader elsewhere would still be exposed.

## Notes for Reviewers

**The first full run was not green, and the two failures are unrelated to this change.** `test:integration:dev` failed `test_orphan_reach_with_only_server_recorded` and `test_sigterm_ignoring_frontend_is_sigkilled` on a server readiness timeout; both pass in isolation (17/17), and the dev suite imports neither support module. Contributing factor: five orphaned `circusd` processes were alive on the machine from earlier sessions, four of them roughly four days old and two owned by a different jj workspace. They have been reaped, and the re-run was green.

**One trap worth knowing.** The first full run reported exit 0 while a task had failed, because the invocation was piped to `tail` — the pipeline's status is `tail`'s. Verification here captured `mise run`'s own exit code instead.

**Where the new suite lives is a judgement call open to review.** `test_built_artefacts.py` sits under `tests/integration/entrypoint/` because that is the lane the defect broke and the lane that runs it; `tests/support/` has no collected home of its own. If a suite per support module is preferred, it would need a task edge to be run at all.

**`build_launcher`'s documented constraint is unchanged and still load-bearing.** A suite calling it must not also gain a `build:cli:dev` dependency, or the two contend on cargo's target lock and the asserted edge goes inert. The claim removes the *stat* race, not that contention.
