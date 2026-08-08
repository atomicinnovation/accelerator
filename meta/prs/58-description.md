---
type: pr-description
id: "58"
title: "Raise the file-descriptor limit before the zig cross-compiles"
date: "2026-08-08T15:48:43+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
relates_to: ["work-item:0165"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/58"
pr_number: 58
tags: []
revision: "f62edd18cbb829785e5db832f023c24dddf4506c"
repository: "accelerator"
last_updated: "2026-08-08T15:48:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Raise the file-descriptor limit before the zig cross-compiles

## Summary

`mise run build:cli:cross-compile` failed from a stock macOS shell with `ProcessFdQuotaExceeded` partway through linking. The cause was not the build: zig's linker opens every object file of a link simultaneously, a release link of the `cli/` workspace passes it several hundred (~640 for `accelerator`), and macOS's launchd default soft limit is 256 descriptors. Both cross-compile tasks now raise `RLIMIT_NOFILE` before their first `cargo zigbuild`, so no contributor needs a `ulimit -n` in their shell profile.

## Changes

- **`tasks/shared/limits.py`** (new) — `raise_descriptor_limit()` reads `RLIMIT_NOFILE` and raises the soft limit toward `LINK_DESCRIPTOR_TARGET` (65536: comfortably above the largest link in the workspace, but deliberately not "as high as possible" since the soft limit is inherited by every child and some tools size per-descriptor bookkeeping off it). The clamping is split out as `descriptor_limit_raise_to(soft, hard, *, wanted)`, a pure function over the `getrlimit` pair, so the thresholds are unit-testable without mutating the test runner's own limits — the same shape as `assert_fixture_size_floor` in `tasks/build.py`.
- **`tasks/build.py`** — `raise_descriptor_limit()` at the top of both `cli_cross_compile` and `server_cross_compile`, before the first `cargo zigbuild`. `setrlimit` is inherited across `fork`/`exec`, so one call in the task process covers cargo, rustc and the zig wrapper beneath them; nothing needs to be threaded through the `context.run` calls.
- **`tests/unit/tasks/test_limits.py`** (new) — nine tests: the macOS 256-descriptor case, clamping to a finite hard limit (Linux) versus `RLIM_INFINITY` (macOS), the no-op cases where the limit already suffices, the never-lower invariant when the hard limit is at or below the soft limit, plus live-limit tests that lower this process to 256, raise through the helper, and restore in a `finally`.
- **`tasks/README.md`** — a "File-descriptor limit on the cross-compiles" subsection under Conventions, recording why the call exists so it does not read as removable noise.

## Context

No work item drives this; it came out of diagnosing a local `build:cli:cross-compile` failure. It relates to `work-item:0165`, which built the cross-compile tasks being amended here.

The failure was environment-specific rather than nondeterministic, which is what made it confusing: the task passed when run from a shell with a raised limit and failed from an ordinary interactive shell on the same machine. `launchctl limit maxfiles` reports 256 as the system default soft limit, and the first casualty in the log was `config-adapters-fixture` at 257 object files — one more than the limit.

Semantics worth flagging: the raise is best-effort. It never lowers an adequate limit, clamps to the hard limit, and warns rather than aborting if `setrlimit` is refused. That is deliberately fail-open, unlike the static-linking and fixture-size assertions alongside it in `tasks/build.py`, which fail closed. Those guard artefact correctness, where a silent pass ships a broken binary; this one is an environment accommodation, and a build under an already-generous limit or with few enough objects links fine without it. Aborting would convert a survivable condition into a hard stop, and the link itself fails loudly if the limit really was the constraint.

## Testing

- [x] The decisive check — the full task under the exact limit that broke it: `(ulimit -n 256; mise run build:cli:cross-compile)` exits 0, with zero occurrences of `ProcessFdQuotaExceeded` in the log. Relinks were forced (`touch` on `cli/launcher/src/main.rs` and `cli/config-adapters/src/lib.rs`, the two crates that failed originally) so this is not a cache no-op.
- [x] The failure was reproduced before the fix at the same limit, confirming causation rather than correlation: same crate, same error. Raising to 4096 with nothing else changed made the identical link succeed.
- [x] `mise run check` exits 0.
- [x] `uv run pytest tests/unit/tasks/` — 728 passed, including the 9 new tests.
- [x] All of the above re-run on the current base after the branch rebased onto `main` (the PR #56 merge and the `accelerator-work` addition to `_CLI_RELEASE_BINARIES`), not only on the base the change was written against.
- [ ] Not verified: behaviour on a Linux host, where the hard limit is a finite ceiling rather than `RLIM_INFINITY`. That path is covered by unit tests over the pure clamping function, not by a real cross-compile.

## Notes for Reviewers

- The fail-open choice is the main judgment call worth a second opinion — see Context. Easy to flip if the house preference is to fail closed alongside the neighbouring assertions.
- Scope is limited to the two tasks that invoke `cargo zigbuild`. `cli_fixture_size_check` is untouched: it builds host-native through the system linker, which does not hold every object open, and its failure mode was never this.
- CI is unaffected today — the release cross-compile lane already passes on `macos-latest` — so this is not a fix for a broken pipeline. The raise is a no-op wherever the limit already suffices, and gives that lane headroom as object counts grow.
- `LINK_DESCRIPTOR_TARGET` is a judgment call, not a measurement. 65536 is roughly 100x the largest current link; if that reads as too generous for a value every child inherits, it is a one-line change.
