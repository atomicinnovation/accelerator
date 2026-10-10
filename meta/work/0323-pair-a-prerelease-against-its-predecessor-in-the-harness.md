---
type: "work-item"
id: "0323"
title: "Pair a prerelease against its predecessor in the harness"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0324", "work-item:0325"]
relates_to: ["work-item:0299"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0323: Pair a prerelease against its predecessor in the harness

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Teach the warm-dispatch harness to measure a named prerelease `V` against its
baseline version on one host, interleaving both releases on both backends, and
to report each backend's `Δ_merge` with a reproducible 95% paired interval.
This is the measurement the per-prerelease lane runs; it cancels runner
differences that an unpaired absolute figure cannot.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI. `V`, baseline
version, `G`, backend, farm and `Δ_merge` are as 0219's Vocabulary defines
them.

The harness measures one version, the checked-out tree's own: the dispatched
launcher reads `.claude-plugin/plugin.json` relative to its plugin root, and
the session, its cache root, manifest, witnesses and teardown are all bound to
that one root. Its three variants pair the fast backend against `B`, the
recovered shell guard; the fallback backend is never paired.

## Requirements

- The harness takes `V` and a baseline version explicitly, rather than reading
  the tree's own version.
- **Baseline version** — when not given explicitly, the harness resolves it
  from committed records as the most recent prerelease before `V` with a valid
  record on that platform, or `V`'s immediate predecessor when the platform
  has none.
- Both releases are fetched, verified and primed into their own plugin roots,
  one farm per backend each; session capture, restore and verify cover both
  roots.
- Four variants — `V` and baseline, each on fast and fallback — are sampled
  interleaved in an order drawn from a recorded **interleave seed**.
- Per backend the harness reports
  `Δ_merge = median(G_V) / median(G_baseline)` with a 95% interval from the
  paired percentile bootstrap, at no fewer than 10,000 resamples, from a
  recorded **bootstrap seed** independent of the interleave seed.
- Per backend it reports C1-C4 statistics for `V`.
- The in-session stability band is computed over each backend's
  `V`/baseline pairs.
- The record lives at `meta/measurements/warm-dispatch/<V>/<platform>.json` and
  carries `V`, the baseline version, the platform key, the runner-identity
  tuple, both seeds, the resample count, per-backend `Δ_merge` and interval,
  per-backend C1-C4, the instrument floors, the session's wall-clock duration
  from start to teardown verify, and validity with any reason.
- The harness runs on a git-only checkout: the engine choice reaches every VCS
  call, not only the smoke check.
- The paired mode does not recover `B`, compute C5/C6 or the robustness ratio,
  or decompose budget terms; the existing single-version mode keeps all of
  these.

## Acceptance Criteria

- [ ] Given `V` and a baseline version, when a session completes, then its
      record names both, and the measured binaries report versions equal to
      them.
- [ ] Given a completed session, then re-deriving the variant order from the
      interleave seed reproduces the sample order, and re-running the bootstrap
      on the samples file with the recorded bootstrap seed and resample count
      reproduces each backend's `Δ_merge` and interval exactly.
- [ ] Given a platform with no valid record, when no baseline version is given,
      then the session pairs against `V`'s immediate predecessor; given the
      immediate predecessor has no valid record but an earlier prerelease does,
      then it pairs against the most recent such prerelease.
- [ ] Given a session, valid or invalidated, then its record carries the
      runner-identity tuple and a wall-clock duration that includes teardown.
- [ ] Given a git-only checkout with no `jj` repository, when a paired session
      runs with the git engine, then it completes and the guarded-path check
      is evaluated with git rather than passing vacuously.
- [ ] Given a paired session, then no `cargo` build of the tree runs.
- [ ] Given the existing single-version invocation, then its record is
      unchanged in shape.

## Open Questions

None.

## Dependencies

- Blocked by: 0322.
- Blocks: 0324, 0325.
- External system: the release host, from which every session fetches two
  signed releases.

## Assumptions

- A baseline version's signed artefacts stay fetchable from the release host.

## Technical Notes

- Single-version assumptions: the cache root is `plugin_root/bin`
  (`tasks/measure.py:597`); the dispatch argv is fixed to
  `plugin_root/bin/accelerator` (`tasks/measure.py:1430-1436`);
  `warm_cache_gaps` (`tasks/measure.py:1581-1607`), `check_preconditions`
  (`tasks/measure.py:1650`) and `plugin_version` (`tasks/measure.py:3112`) all
  use one version. `ACCELERATOR_CACHE_DIR` is a rejected override
  (`tasks/shared/measurement.py:605`), so a second cache needs a second root.
- `Variant` has exactly `B`, `G-fast` and `G-fallback`
  (`tasks/shared/measurement.py:211-215`); `generate_schedule`
  (`tasks/shared/measurement.py:254-286`) pairs only `B` with `G-fast`.
- `paired_ratio_interval` (`tasks/shared/measurement.py:113-143`) is reusable
  as is. Analysis today shares one `random.Random(SEED)` across C1-C6 and the
  drift band (`tasks/measure.py:2259`), and the schedule reuses the same seed
  (`tasks/measure.py:2442`), so `Δ_merge` needs its own generator to be
  reproducible alone.
- `build_rig` calls `recover_baseline` with the default `engine="jj"`
  (`tasks/measure.py:1412`); only `smoke_report` takes an engine. The guarded
  diff runs `jj diff --summary` with `check=False`
  (`tasks/measure.py:556-563, 663`), so it returns empty on a non-jj checkout.
- `close_the_budget` → `decompose_terms` runs `cargo test --release`
  (`tasks/measure.py:2051-2060, 2924-2963`), which the paired mode skips.
- `record_provenance` (`tasks/measure.py:2799`) records no `ImageOS`,
  `ImageVersion` or `RUNNER_ARCH`. `ImageOS` is `macos15` on both arm64 and
  Intel macOS, so `RUNNER_ARCH` tells them apart.
- `elapsed_s` is taken at analysis (`tasks/measure.py:1817`) and excludes
  post-floors and teardown.

## Drafting Notes

- `B` and the term decomposition are dropped from the paired mode because the
  lane's comparison is release against release; carrying them would add a
  `cargo` build and a recovered-guard fixture to every CI session for figures
  no signal reads.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- `meta/work/0299-bring-the-cli-workspace-s-crate-dependencies-into-line-with.md`
  — the one-off release-against-release comparison
