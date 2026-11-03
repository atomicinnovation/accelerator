---
type: "work-item"
id: "0362"
title: "Make the bootstrap test suites fast and hermetic"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "task"
priority: "high"
parent: "work-item:0361"
blocks: ["work-item:0366", "work-item:0370"]
relates_to: ["work-item:0165", "work-item:0182", "work-item:0368"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "testing", "ci"]
last_updated: "2026-11-10T12:22:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0362: Make the bootstrap test suites fast and hermetic

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

`test:integration:skill-invocation` spends about 90% of its time in the
debug-built `accelerator-verify` shim, whose `minisign-verify` dependency
hashes the 76 MB debug launcher unoptimised on every call. Building that
one crate at `opt-level = 3` removes the largest single cost in the local
build, and is expected to do the same on CI. This item confirms the CI
saving only if it meets the CI bar; otherwise a follow-up item takes over
that check. The hooks smoke test instead
downloads the published launcher from GitHub on every run. Serving it the working-tree launcher
through the stubbed downloader makes it hermetic, and a new negative case
shows that the bootstrap still refuses a launcher signed with the wrong key.

## Context

Terms used throughout:

- **Bootstrap**: `bin/accelerator`, the bash entry point the hooks and
  skills call. It fetches, verifies, caches and execs the launcher.
- **Launcher**: the Rust `accelerator` binary the bootstrap fetches,
  which dispatches commands such as `vcs detect` to sub-binaries.
- **Shim**: `accelerator-verify`, the small binary the bootstrap calls to
  check the launcher's minisign signature.
- **Stub downloader**: the test fixture that stands in for `curl`,
  serving release assets from a temporary directory and logging each
  request.
- **Stub launcher**: the test fixture's fake launcher, which records its
  arguments instead of dispatching anything.
- **Claimed copy**: a binary copied out of `cli/target/debug` by
  `claim_artefact` (`tests/support/artefacts.py`), so that a concurrent
  cargo re-link cannot change it under the test.
- **Committed release shim**: the release-built `accelerator-verify`
  checked into `bin/` for each platform, such as
  `bin/accelerator-verify-darwin-arm64`, which installed plugins use.
- **Warm build cache**: `cli/target` already holds the debug builds the
  suite needs, so a run compiles nothing.
- **Warm no-op build**: a full build run with no source change since the
  previous one, as 0368 defines it; its duration is 0361's yardstick.
- **Full run**: the bare `mise run` default task, with every suite
  competing for the host.
- **Secondary VCS workspace**: a jj workspace or git worktree other than
  the main checkout, inside which `vcs detect` adds boundary
  prohibitions.

Three suites run the bootstrap. Two rows of the research's
Recommended Work table apply. A1 (Requirement 1) makes
`test:integration:skill-invocation` and `test:integration:entrypoint`
faster. A7 (Requirements 2–3) makes the launcher-dispatch smoke test in
`test:integration:hooks` hermetic and adds a negative case beside it.
The A7 row asks only for the hermetic smoke test. The negative case is
added because removing the published-release fetch leaves the hooks
suite with no test that the bootstrap refuses a badly signed launcher
under `--fail-safe`, the mode every hook uses.

`test:integration:skill-invocation` runs 143 `config` commands through the
real bootstrap. On every call, including cache hits, the bootstrap verifies
the cached launcher's minisign signature. The shim and the 76 MB launcher
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

`test_launcher_dispatch_smoke_on_a_plain_non_repo_dir` runs the
repository's own `bin/accelerator` with an empty cache and the real
downloader. So the bootstrap `curl`s the published `v<version>` launcher,
17 MB, from GitHub. The fetch alone took 42 s, and the test took 53–66 s.
The test depends on the network, fails offline, and exercises the published
launcher rather than the one under test.

The published-release fetch adds no confidence in the launcher about to
ship. On a working branch, `plugin.json` names the last published
prerelease, so the test downloads a launcher that has already shipped.
Its one incidental value is a per-pull-request check of the committed
public key against a genuine release signature. `prerelease:finalise` and
`release:finalise` make the same check, re-verifying every published
launcher against `keys/accelerator-release.pub` before the release leaves
draft.

## Requirements

1. Add `[profile.dev.package.minisign-verify] opt-level = 3` to
   `cli/Cargo.toml`, so debug builds of the `accelerator-verify` shim hash
   at release speed (A1). Ship this first, as its own pull request, ahead
   of Requirements 2–3.
2. Run the hooks smoke test through the stubbed-downloader installation
   that `make_installation` in `tests/integration/support/installation.py`
   builds: a copy of `bin/accelerator` in a temporary plugin root, the
   real debug shim, a per-run release keypair whose public half replaces
   the committed `keys/accelerator-release.pub` in that root, and a
   launcher signed with a chosen key and served by the stub downloader
   (A7). Two of `make_installation`'s existing options set what is served;
   neither needs adding:
   - `launcher_source` names the launcher binary the stub downloader
     serves. Left unset, it serves the stub launcher, which records its
     arguments to the file named by `LAUNCHER_ARGS_OUT` whenever it runs.
   - `secret` chooses the signing key. It defaults to `"release"`, the
     key the installation trusts; `"attacker"` signs with a per-run
     attacker key instead.

   The smoke test sets `launcher_source` to the claimed copy of the
   working-tree debug launcher from the `build:cli:dev` output, and keeps
   the default `secret`.
3. Add a negative case to the hooks suite (A7). It is its own test and
   builds its own installation through `make_installation`, rather than
   reusing the smoke test's: the same kind of components, set up afresh.
   It leaves `launcher_source` unset, so the stub launcher is served, and
   sets `secret="attacker"`. It needs no launcher build or
   `ACCELERATOR_VCS_BIN`.

   The bootstrap must refuse the launcher, not exec it. The entrypoint
   suite's refusal test, `test_non_release_key_signature_is_refused`,
   runs without `--fail-safe`. This case runs under `--fail-safe`, as the
   hooks invoke the bootstrap, so it also covers the bootstrap's record of
   refusals, `.accelerator-unverified.log` in the cache directory.

   Requirements 2 and 3 land together, in one pull request after
   Requirement 1's.

Out of scope:

- Skipping re-verification on cache hits. Per-call verification is a
  trust-chain decision, and 0361 rules out weakening any check.
- Dev-profile changes to `cli/Cargo.toml` beyond the `minisign-verify`
  override, which belong to 0366.
- A replacement end-to-end smoke of a published launcher. Requirement 2
  removes the published-launcher fetch on purpose: it only ever fetched
  the last published prerelease, never the launcher about to ship (see
  Context). What moves is a per-pull-request check of the committed
  public key against a genuine release signature. 0361 accepts this as a
  check that moves rather than disappears: `prerelease:finalise` and
  `release:finalise` keep it, and catch a bad key before a release leaves
  draft.

## Acceptance Criteria

- [ ] **Shim speed.** Given one signed debug launcher, when the debug shim built before
      Requirement 1 and the one built after it each verify it 10 times on
      the same host, then both accept the signature and the second's
      median is at least 5× faster (1.15 s against 0.06 s on the research
      workstation), with both medians recorded in the pull request that
      lands Requirement 1.
- [ ] **Local suite speed.** Given a warm build cache and one discarded
      warm-up run per revision, when `mise run --skip-deps test:integration:skill-invocation` runs
      alone 3 times before and after Requirement 1 on the same host, then
      every case passes and the median is at least 4× faster (163 s today
      on the research workstation), with both medians recorded in the pull
      request that lands Requirement 1.
- [ ] **CI suite speed.** Given the last run of the `Main` workflow
      (`.github/workflows/main.yml`) on the pull request that lands
      Requirement 1 before it merges, when its ubuntu `Run integration
      tests` job runs, then `test:integration:skill-invocation` passes,
      its duration is recorded in that pull request, and either:
      - it is at least 3× faster than its 1,266 s baseline (under 422 s);
        or
      - it is not, and before that pull request merges, a follow-up item
        exists at status ready, is linked from the pull request, and has
        acceptance criteria that carry this 3× bar, the 1,266 s baseline
        and the recorded miss (see Technical Notes).

      The criterion judges that one run on the GitHub-hosted runner; a
      miss goes to the follow-up rather than to reruns of the job. Under
      the second outcome this item stops verifying the CI saving, and the
      follow-up's bar becomes the only check of it.
- [ ] **Hermetic smoke test.** Given the working-tree debug launcher, when
      `test:integration:hooks` runs, then the launcher-dispatch smoke test
      passes, calls the bootstrap with `vcs detect --format=hook
      --fail-safe --descriptive` (the `SessionStart` hook's command) from
      a temporary directory outside any checkout, only through
      `run_bootstrap` so that `assert_hermetic` refuses a real downloader
      or a resolvable release URL, and asserts that:
      - the stub downloader's log holds only the launcher asset for the
        host's platform and its `.minisig` signature;
      - the cached launcher is byte-identical to the claimed copy the stub
        downloader served;
      - the launcher dispatched: exit status 0, empty stderr, and stdout
        that parses as the `SessionStart` hook JSON
        (`hookSpecificOutput.hookEventName` is `SessionStart`), which,
        for `vcs detect`, only `accelerator-vcs` emits. The stub launcher
        emits none, so this rules out the stub having been served by
        mistake;
      - the hook JSON's `additionalContext` starts with `This repository
        uses git as its version control system.` and does not contain
        `WORKSPACE BOUNDARY DETECTED`. Outside any checkout, `vcs detect`
        falls back to git mode and finds no boundary, so this is the
        output on every host. It fails if `accelerator-vcs` resolved a
        jj checkout, or added the boundary block it writes inside a
        secondary VCS workspace.
- [ ] **Refusal under `--fail-safe`.** Given an installation whose stub downloader serves the stub launcher
      signed with the per-run attacker key, when the test calls the
      bootstrap with the same hook command as the smoke test, `vcs detect
      --format=hook --fail-safe --descriptive`, then:
      - the stub downloader's log shows both the launcher and its
        signature were served, so the refusal comes from verification;
      - the exit status is 0, as `--fail-safe` requires of every hook;
      - stderr carries `could not fetch and verify the accelerator
        launcher`;
      - `.accelerator-unverified.log` gains one entry, and that entry
        contains `could not fetch and verify the accelerator launcher`;
      - the stub launcher never runs: with `LAUNCHER_ARGS_OUT` passed
        through `run_bootstrap`'s `extra_env`, it writes no record;
      - the cache directory afterwards holds no launcher and no
        `.tmp-launcher-*` file, so no later call can pick up the refused
        binary;
      - as a control in the same test, after these assertions: re-signing
        the served stub launcher with the installation's release key
        (`sign` in `tests/integration/support/installation.py`) and
        calling the bootstrap again makes the stub launcher run and write
        its record. This shows the installation's shim works and the stub
        records when it runs, so the earlier refusal came from the
        attacker key and nothing else.
- [ ] **Entrypoint regression.** Given the optimised debug shim, when `test:integration:entrypoint`
      runs, then `test_non_release_key_signature_is_refused` and
      `test_tampered_cached_launcher_is_refused_and_healed` still pass.

## Open Questions

None.

## Dependencies

- Blocked by: none. This item used to wait for 0368's warm no-op
  baseline, which 0361 measures its 200 s target against, so that the
  baseline would be taken before this item's saving and 0361's
  before/after comparison would not absorb it. That baseline is recorded
  at
  `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`,
  so this item may merge.
- Blocks: 0366, which reshapes what this item builds on: the
  `build:cli:dev` and `build:cli:vcs:dev` edges the hooks suite uses, the
  in-pytest builds, and the debug launcher's size, which the speed ratios
  measure against. This item lands first; 0366 must then keep the hooks
  suite's launcher and `accelerator-vcs` builds working. 0366's
  Requirement 2 removes in-pytest cargo builds that mise already covers.
  This item adds one to the hooks suite, the debug shim through
  `build_shim()`, which falls outside that requirement because no mise
  task builds the shim. If 0366 brings the debug shim under a mise task,
  the hooks, entrypoint and skill-invocation suites take that edge in
  place of `build_shim()`, and the shim keeps the `minisign-verify`
  override. Both items also edit the dev-profile table in
  `cli/Cargo.toml` (see Technical Notes).
- Blocks: 0370, which measures the macOS host settings once this item,
  0363 and 0367 have each landed or been abandoned. It waits for all
  three requirements: Requirement 1 shortens skill-invocation, and
  Requirement 2 removes the smoke test's network fetch, so each changes
  the full-run duration 0370 measures against.
- Consumed by 0361's closing warm no-op comparison. Both requirements
  change what that comparison measures: Requirement 1 shortens
  skill-invocation, and Requirement 2 removes the smoke test's network
  fetch. Re-measuring at a commit before this item still needs GitHub
  for that fetch, so it can fail offline.
- Relies on 0165's release-time re-verification. Once this item lands,
  `prerelease:finalise` and `release:finalise` are the only check of the
  committed public key against a genuine release signature, so they must
  keep re-verifying every published launcher.
  `test_launcher_reverify_failure_preserves_draft` in
  `tests/integration/tasks/test_github.py` covers that they do.
- Hands off, only if Requirement 1 misses the CI bar: a follow-up item
  takes over the 3× bar from the CI suite speed criterion (see Technical
  Notes). Neither this item nor its block on 0366 waits for that
  follow-up.
- Depends on the GitHub-hosted ubuntu runner for the CI suite speed
  criterion. Its single-run variance, and any change to its hardware or
  image, are accepted: a miss goes to the follow-up rather than to reruns.

## Assumptions

- Raising the optimisation level of one crate changes speed only, not
  verification behaviour: the shim compiles the same code.
- CI's uniform ~8 s per case comes from the same per-call verification.
  The research says only that the pattern fits; it did not profile CI.
- The Linux debug launcher, probably larger than the 76 MB macOS one
  because it embeds its DWARF, does not erode the speed-up below 3×. Its
  size was not measured.

## Technical Notes

- Per-call verification: `bin/accelerator:409-435`.
- Debug shim and launcher built inside pytest:
  `tests/support/artefacts.py:56-93`.
- `minisign-verify` dependency: `cli/verify/Cargo.toml:22`.
- Today's smoke test:
  `tests/integration/hooks/test_vcs_detect_registration.py:30-81`.
- `bin/accelerator` does not read `ACCELERATOR_BIN`. The `accelerator_env()`
  overlay in `tasks/test/helpers.py` sets it for suites that choose their
  entry point from it; today's smoke test calls `bin/accelerator` directly
  and ignores it.
- The entrypoint and skill-invocation suites already serve releases from a
  stubbed downloader through `make_installation` and `run_bootstrap`.
  `run_bootstrap`'s `assert_hermetic` precondition refuses the repo's own
  `bin/accelerator`, which is why the rewritten smoke test runs a copy.
- The `.invalid` release URL that `assert_hermetic` enforces also confines
  the launcher: `release_base_url()` reads the same
  `ACCELERATOR_RELEASE_BASE_URL` (`cli/launcher/src/main.rs:55-59`), so
  neither the bootstrap nor the launcher can fetch a release asset from
  the network. That, `assert_hermetic` and the download-log assertion
  together are the hermeticity check; no separate offline run is needed.
- Serve the launcher that `test:integration:hooks` already builds through
  its `build:cli:dev` edge (`mise.toml:457-460`; the task itself is
  `tasks/test/integration.py:114-127`), copied out of `cli/target/debug`
  with `claim_artefact`. The sibling `test_research_guard_registration.py`
  already uses that build. Do not call `build_launcher()`: its docstring
  forbids a suite from both calling it and carrying the edge.
- Build the shim with `build_shim()`. No mise task builds the debug shim.
- `accelerator-vcs` comes from the existing `build:cli:vcs:dev` edge.
- Keep the `ACCELERATOR_VCS_BIN` override. The launcher verifies the
  sub-binaries it fetches against the committed public key embedded at
  build time, which no per-run key can satisfy. The override execs the
  local `accelerator-vcs` unverified instead.
- `make_installation`'s root is enough for `vcs detect`. With
  `ACCELERATOR_VCS_BIN` set, the launcher's resolver returns the override
  before it looks up a cache root, its embedded key or the network
  (`cli/launcher/src/main.rs:72-92`), and the `vcs`, `vcs-adapters` and
  `vcs-cli` crates never read the plugin root.
- The shim speed and local suite speed criteria use medians because a
  single timing of a 0.06 s verification sits within process-start noise.
  The CI suite speed criterion uses its one named run.
- If the pull request that lands Requirement 1 misses the CI bar,
  Requirement 1 still merges for its measured local saving. That pull
  request records the measured duration and links a follow-up item that
  investigates CI's per-case cost and carries the 3× bar as its own
  criterion. That meets the CI suite speed criterion's second outcome, so
  the CI bar no longer holds this item open. The item still closes, and
  clears its blocks on 0366 and 0370, only once Requirements 2–3 have
  landed too.
- Under `--fail-safe`, `fail` exits 0 (`bin/accelerator:51-54`). Exit 0
  therefore checks the hook contract but cannot tell a refusal from a
  success, so the negative case shows the refusal through the stub
  launcher's run record, stderr and the unverified log. On a failed
  verification, `fetch_and_verify` (`bin/accelerator:415-433`) deletes
  its temporary launcher and signature before anything reaches the
  launcher's cache path, which is what the empty-cache assertion checks.
- Outside any checkout, `vcs detect` falls back to git mode
  (`cli/vcs/src/mode.rs`), and `--descriptive` emits the git command
  reference with no boundary block (`cli/vcs-cli/src/detect.rs`).
  `fail_integrity` (`bin/accelerator:61-71`) appends the timestamped
  entry. `test_non_release_key_signature_is_refused` in the entrypoint
  suite shows the attacker-key set-up; it runs without `--fail-safe`, so
  its non-zero exit status shows the refusal on its own.
- Release-time re-verification: `tasks/release.py:172-180` and
  `tasks/github.py:176-189`.
- 0366 may add dependency-wide settings under `[profile.dev.package."*"]`.
  Cargo resolves each setting separately, with the named package winning,
  so the two compose; expect a textual conflict in the same table only.

## Drafting Notes

- Grouped the two research rows because both affect suites that run
  `bin/accelerator`, as agreed in conversation. Requirement 1 is the
  larger and cheaper change, hence the instruction to ship it first.
- Kept them as one item after review. Requirements 2–3 are small, and
  Requirement 1 merges first as its own pull request, so the CI saving
  does not wait on them. The item stays in progress until Requirements
  2–3 land, and they inherit Requirement 1's priority. 0366 waits for the
  whole item, Requirements 2–3 included, as decided in review.
- Set priority to high because Requirement 1 is the largest single saving
  locally and on CI's critical path.
- The CI bar is 3×, below the 4× local bar, because the research did not
  measure the fix on CI and the Linux debug launcher's size is unknown.
  If the CI saving does not materialise, the bar moves to a follow-up
  item rather than holding this one open.
- The local speed criteria are same-host ratios, so they hold on other
  contributors' machines. Each bar sits well below the measured speed-up,
  to leave room for host noise: 5× against ~19× for the shim (1.15 s to
  0.06 s), and 4× against 9× for skill-invocation.
- The smoke test has no speed bar. Its baseline is mostly network time,
  so a same-host ratio would measure the connection. The hermetic smoke
  test criterion's download-log assertion already proves the download,
  which was its cost, is gone.
- The entrypoint suite has no speed bar. Its measured gain, 18 s to 14 s,
  is too small for a same-host ratio to survive host noise; the
  entrypoint regression criterion covers it instead.
- Dropped the `ACCELERATOR_BIN` route from Requirement 2. Calling the
  launcher directly would skip `bin/accelerator`, and the bootstrap's
  dev-launcher override (`ACCELERATOR_LAUNCHER_BIN`, behind
  `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` and an untracked marker) skips
  fetch, cache and verification. Only the stubbed downloader keeps the
  trust chain real.
- Dropping the published-release fetch is deliberate. Release-time
  re-verification already covers the published assets, and the fetched
  launcher was never the one about to ship.
- The negative case replaces a "nothing is mocked out" criterion that no
  check could confirm.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
  (A1 and A7 are rows of its Recommended Work table)
- Related: 0361, 0366, 0368 (warm no-op baseline)
- Prior work: 0165 (release-time re-verification), 0182 (removed a bootstrap
  test that fetched the real release)
