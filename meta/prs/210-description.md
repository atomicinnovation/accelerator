---
type: "pr-description"
id: "210"
title: "[0362] Verify launcher signatures at release speed in dev builds"
date: "2026-11-12T18:24:51+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0362"
parent: "work-item:0362"
relates_to: ["work-item:0361", "work-item:0366", "work-item:0370"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/210"
pr_number: 210
tags: ["build-system", "performance", "testing", "bootstrap", "minisign", "cargo-profile"]
revision: "66ff5bb06686a3ef1f461e62cc34dbf5ce849642"
repository: "accelerator"
last_updated: "2026-11-12T18:59:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0362] Verify launcher signatures at release speed in dev builds

## Summary

On every call, `bin/accelerator` re-verifies the whole cached launcher with
the `accelerator-verify` shim. In the suites that drive the bootstrap, that
shim is a debug build, and it hashes the 76 MB debug launcher with an
unoptimised `minisign-verify`. That hash took most of the time in every
bootstrap-driven test.

This PR builds `minisign-verify` at `opt-level = 3` in the dev profile. Each
verification drops from ~1.19 s to ~0.07 s. `test:integration:skill-invocation`
drops from ~168 s to ~14 s locally.

This is the first of three PRs for 0362. The next moves the installation
fixtures into one shared module. The one after makes the hooks smoke test
hermetic and adds a refusal test.

## Evidence

All local figures come from one host (darwin-arm64), with the before and
after builds measured on the same machine.

**Shim speed.** Each shim verified the same signed 78.9 MB debug launcher once
untimed, then 10 times timed, alternating with the other shim. Both shims
accepted the signature every time.

| Shim | Median | Min | Max |
| --- | --- | --- | --- |
| Before (no override) | 1.189 s | 1.181 s | 1.236 s |
| After (`opt-level = 3`) | 0.069 s | 0.067 s | 0.076 s |

The after-median is ~17× faster. The bar is ≥ 5×.

**Local suite speed.** Each revision had one discarded warm-up run. Then
`mise run --skip-deps test:integration:skill-invocation` ran 3 times.

| Revision | Run 1 | Run 2 | Run 3 | Median |
| --- | --- | --- | --- | --- |
| Before | 167.8 s | 168.4 s | 168.8 s | 168.4 s |
| After | 14.3 s | 14.2 s | 13.7 s | 14.2 s |

All 143 tests passed on every run. The median is ~11.9× faster. The bar is
≥ 4×.

**CI suite speed.** These figures come from the ubuntu `Run integration tests`
job in
[`Main` run 43279755256](https://github.com/atomicinnovation/accelerator/actions/runs/43279755256),
attempt 1, on `6d042c7`.

| | Baseline | This PR |
| --- | --- | --- |
| mise `Finished in` | 1,266 s | 375.46 s |
| pytest summary | — | 373.27 s |
| Tests | 143 | 143 |
| Median time per test | 8.0 s | 1.56 s |

The task's time is ~3.37× faster than the baseline, inside the ≤ 422 s (3×)
bar, so no follow-up item is needed.

- **The build lands on the seventh test.** That is the first test that uses
  the installation, and it took 70.6 s. The six corpus-integrity tests before
  it don't build anything. In the baseline run, the same build took 154 s.
- **This run includes the one-off rebuild.** It was the first run with the
  override on a cargo cache restored from `main`, and its log shows
  `Compiling minisign-verify v0.2.5`. So the build time above includes that
  rebuild.
- **The remaining tests are steady.** The 136 tests after the build have a
  median of 1.57 s each. The timestamps record when log lines reached the
  runner, so per-test figures this small are coarse.
- **The run's overall conclusion is `failure`.** That comes only from the
  advisory `Check measurement harness (macos-latest)` job, which was
  cancelled. Every blocking job passed. Attempt 2 re-runs only that job.

## Merge Danger

**Door:** two-way

The change is one profile table and one unit test. Reverting it restores the
old dev builds, and nothing is stored or migrated.

**Blast Radius:** narrow

- **Release binaries are unchanged.** `[profile.release]` doesn't inherit
  package overrides from `[profile.dev]`, so the signed artefacts build exactly
  as before.
- **Dev builds rebuild once.** The first build in each dev-derived
  configuration recompiles `minisign-verify` and everything that depends on
  it: the shim, the launcher and the launcher's test binaries. That applies to
  plain debug, tests, coverage and lint, and to each cargo cache, CI's
  included.
- **The vendored-shim drift marker is unchanged.** The marker hashes the first
  line that starts with `minisign-verify`. The new table sits below the
  dependency pin, and none of its lines starts that way.

## Changes

- **`cli/Cargo.toml`.** A new `[profile.dev.package.minisign-verify]` table
  with `opt-level = 3`, after `[profile.release]`. A two-line comment explains
  why one dependency departs from the dev defaults.
- **`tests/unit/tasks/test_cargo_profiles.py`** (new). This test pins the
  override. It also checks that `minisign-verify` is still a workspace
  dependency, because cargo only warns about an override for a package the
  workspace no longer has. The upcoming dev-profile work (0366) will add its
  own pins here.
- **`test_skill_invocation_conformance.py`.** A comment drops its `~0.1s a go`
  figure, which was never accurate.

The branch also carries 0362's research, plan and reviews, and the plan
records Phase 1 as implemented.

## Context

- Work item: `meta/work/0362-make-the-bootstrap-test-suites-fast-and-hermetic.md`
- Research:
  `meta/research/codebase/2026-11-10-0362-bootstrap-test-suites-fast-and-hermetic.md`
- Plan: `meta/plans/2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic.md`
  (Phase 1)
- Reviews: `meta/reviews/work/0362-…-review-2.md` and
  `meta/reviews/plans/2026-11-11-0362-…-review-1.md`

## Testing

- [x] The pin test failed with `KeyError: 'dev'` before the override and
  passes after it. Its dependency assertion failed once, with the dependency
  renamed temporarily, and passed again once it was restored.
- [x] `uv run pytest tests/unit/tasks/test_build.py` passes, so the drift
  marker is unchanged.
- [x] `mise run test:unit:cli` passes, including the `report.rs` test that
  slices `[profile.release]`.
- [x] `mise run test:integration:entrypoint` passes on the optimised shim (75
  passed). That includes `test_non_release_key_signature_is_refused` and
  `test_tampered_cached_launcher_is_refused_and_healed`.
- [x] `mise run test:integration:skill-invocation` passes (143 passed).
- [x] `mise run` exits 0, with no formatter changes.
- [x] The CI skill-invocation figures from this PR's last `Main` run are
  recorded above. The task took 375.46 s, inside the ≤ 422 s bar.

## Notes for Reviewers

- **The `report.rs` slice now includes the new comment.**
  `cli/vcs-cli/src/report.rs` slices `[profile.release]` up to the next
  `\n[`, so the slice gains the new table's comment. Its test fails only if
  that slice contains both `panic` and `abort`. The comment contains neither,
  and it should stay that way if anyone rewords it.
- **0366 must keep this override.** 0366 reshapes the same dev-profile table,
  and the pin test fails if the override disappears.
