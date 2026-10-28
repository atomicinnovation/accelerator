---
type: "work-item"
id: "0362"
title: "Make the bootstrap test suites fast and hermetic"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "high"
parent: "work-item:0361"
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "testing", "ci"]
last_updated: "2026-10-28T16:18:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0362: Make the bootstrap test suites fast and hermetic

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

The suites that run `bin/accelerator` spend most of their time in one place:
a debug-built `minisign-verify` shim verifying a debug-built launcher. The
hooks smoke test separately downloads the published launcher from GitHub on
every run. Optimising one crate in dev builds and pointing the smoke test at
the local launcher removes the largest single cost in both the local build
and CI.

## Context

`test:integration:skill-invocation` runs 143 `config` commands through the
real bootstrap. On every call, including cache hits, the bootstrap verifies
the cached launcher's minisign signature. The verifier and the 76 MB launcher
are debug builds made inside pytest, so each verification takes 1.15 s. The
committed release shim does the same check in 0.06 s.

| Suite | Today | With `opt-level = 3` for `minisign-verify` (measured) |
|---|---|---|
| `test:integration:skill-invocation`, alone | 163 s | 18 s |
| `test:integration:skill-invocation`, in a full run | 252 s | 19 s |
| `test:integration:entrypoint`, alone | 18 s | 14 s |

On CI's ubuntu runner, skill-invocation takes 1,266 s, a median of 8.0 s per
case. That makes the 23-minute integration job the critical path of every
workflow run.

`test_launcher_dispatch_smoke_on_a_plain_non_repo_dir` sets
`ACCELERATOR_VCS_BIN` but not `ACCELERATOR_BIN`, and starts with an empty
cache. So the bootstrap `curl`s the published `v<version>` launcher, 17 MB,
from GitHub. The fetch alone took 42 s, and the test took 53–66 s. The test
depends on the network, fails offline, and exercises the published launcher
rather than the one under test.

## Requirements

1. Add `[profile.dev.package.minisign-verify] opt-level = 3` to
   `cli/Cargo.toml`, so debug builds of the verify shim hash at release
   speed (A1). Ship this first, on its own, if the rest of the item is not
   ready.
2. Make the hooks smoke test run the locally built launcher, through
   `ACCELERATOR_BIN`, or a stubbed downloader, so it no longer fetches a
   release (A7).

## Acceptance Criteria

- [ ] Given a debug build of `accelerator-verify`, when it verifies the
      debug launcher's signature, then it completes in under 0.2 s on the
      reference machine (1.15 s today).
- [ ] Given a warm workspace, when `mise run --skip-deps
      test:integration:skill-invocation` runs alone, then it completes in
      under 30 s (163 s today).
- [ ] Given a pull-request run of the main workflow, when the ubuntu
      integration job runs, then `test:integration:skill-invocation` takes
      under 300 s (1,266 s today).
- [ ] Given no network access, when `test:integration:hooks` runs, then it
      passes and makes no request to `github.com`.
- [ ] The bootstrap suites still exercise the real shim, the real bootstrap
      and real minisign signatures, with no verification mocked out.

## Open Questions

- Does anything rely on the hooks smoke test exercising the published
  release, as opposed to the dispatch path through the local launcher?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- Raising the optimisation level of one crate changes speed only, not
  verification behaviour: the shim compiles the same code.

## Technical Notes

- Per-call verification: `bin/accelerator:409-435`.
- Debug shim and launcher built inside pytest: `tests/support/artefacts.py:56-93`.
- `minisign-verify` dependency: `cli/verify/Cargo.toml:22`.
- Smoke test: `tests/integration/hooks/test_vcs_detect_registration.py:30-70`.
- The entrypoint and skill-invocation suites already serve releases from a
  stubbed downloader (`ACCELERATOR_BOOTSTRAP_DOWNLOADER` in
  `tests/integration/support/installation.py`), which is a pattern the smoke
  test can reuse.
- The Linux debug launcher is probably larger than the 76 MB macOS one,
  because it embeds its DWARF, but its size was not measured.

## Drafting Notes

- Grouped A1 and A7 because both affect suites that run `bin/accelerator`,
  as agreed in conversation. A1 is the larger and cheaper change, hence the
  instruction to ship it first.
- Set priority to high because A1 is the largest single saving locally and
  on CI's critical path.
- The 300 s CI target is a proposal. The research projects most of the
  1,266 s disappears, but did not measure the fix on CI.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
