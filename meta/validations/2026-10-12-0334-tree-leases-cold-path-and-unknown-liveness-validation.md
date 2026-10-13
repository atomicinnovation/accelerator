---
type: "plan-validation"
id: "2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness-validation"
title: "Validation Report: Tree Leases on the Cold Path and Under Unknown Liveness Implementation Plan"
date: "2026-10-13T13:59:49+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness"
tags: ["runtime", "cache", "lease", "prune", "design", "tree-lease"]
last_updated: "2026-10-13T16:05:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Tree Leases on the Cold Path and Under Unknown Liveness

### Implementation Status

✓ Phase 1: Extract `tree-lease` - Fully implemented
✓ Phase 2: Descriptor floor and warm-path proof - Fully implemented
✓ Phase 3: Cold-path leases - Fully implemented
✓ Phase 4: Liveness-probe seam - Fully implemented
✓ Phase 5: Honour `Unknown` - Fully implemented
✓ Phase 6: Cross-launcher criteria and close-out - Fully implemented

### Automated Verification Results

✓ `mise run` exits 0 end-to-end on a clean `cli/target`. Every format, lint,
type, `pup`, `deny`, `public-api`, `crate-dependencies` and `notices` lane
passes, and the run leaves no changes in the working copy.
✓ CLI workspace: 5,357 tests run, 5,357 passed, 2 skipped.
✓ Python lanes: `test:unit:tasks` (3,549), `test:integration:skill-invocation`
(141), `test:integration:deny` (116), `test:integration:tasks` (78),
`test:integration:entrypoint` (66), `test:integration:conformance` (30),
`test:integration:hooks` (29), `test:integration:dev` (17),
`test:integration:research` (4).
✓ Every new suite passes: `tree-lease` unit tests, `executor_runtime` (7),
`tree_leases` (8), the `reap.rs` rule, boundary and sidecar tests, and the
`tree_resolution` seam, `Unknown` table, superseded and cross-launcher tests.

Two earlier runs failed for reasons outside the implementation, and both are
resolved:

- The branch's work item first reused ID `0330`, which trunk already assigns
  to `0330-skill-command-grants-that-survive-across-queries.md`, so
  `this_repositorys_own_corpus_is_clean` reported `DUPLICATE-ID`. The item
  and every reference to it were renumbered to 0334.
- Ten `github::octocrab_client` tests failed with `Err("Other")` because
  `cli/target/debug/deps` held 705,899 stale entries. macOS's first keychain
  use (`SecTrustSettingsCopyCertificates` → `CFBundleGetMainBundle`) scans the
  executable's directory. That scan took ~8s per process and failed when run
  in parallel. Removing `cli/target` fixed it.

Not re-run here: the red-before-green checks the plan asks for in Phases 2-6
(each test failing before its production change). The plan marks them done.
This validation saw only the final state.

### Code Review Findings

#### Matches Plan:

- `cli/tree-lease` owns `hold_shared_lease`, `SharedLease`, `LeaseError`
  (hand-written `Display`/`Error` with `source()`) and `observed_held`, with a
  non-unix stub. It is registered in the workspace, `_EXPECTED_DECLARATIONS`,
  `pup.ron` (`tree_lease_holds_leases_only` plus the command-runner
  alternation), the probe pair and `_EXEMPT_MEMBERS`.
- `hold_shared_lease` raises the descriptor with `fcntl_dupfd_cloexec` to
  `LOWEST_LEASE_DESCRIPTOR` (10) before `flock`, then clears `FD_CLOEXEC`.
  `process.rs` asserts `IDENTITY_FD < LOWEST_LEASE_DESCRIPTOR` at compile
  time.
- The launcher keeps only `impl HeldLease for tree_lease::SharedLease` and
  `From<LeaseError> for TreeError`. `resolver.rs` changes only the import,
  call path and probe plumbing.
- `TreeLeases::hold` (`design-adapters/src/ensure.rs`) checks the lease path
  against the layout first, so a lease outside the layout is never created.
  It leases, then re-checks the tree with `symlink_metadata`. A gone tree's
  lease file is removed only when `observed_held` reads `Ok(false)`.
- `ensure_and_pin` runs at most `ENSURE_ATTEMPTS = 2` times and drops the
  whole `HeldTrees` of a gone attempt. It returns `TreesReclaimed`, which
  downgrades with no marker, or `Failed(cause)`, which goes through the
  unchanged `classify_cause` path. `PinnedResolution` keeps `_leases` alive in
  `run` through `act_on`.
- `LivenessProbe`/`FlockProbe` live in the outbound `lease.rs`.
  `TreeResolver.liveness` is threaded through `reap_orphans`,
  `reap_superseded`, `reclaim_unclaimed` and `materialise_with_waiter`. No
  reclaimer calls `probe_liveness` directly.
- `reap.rs` has the three `const fn` rules side by side
  (`unclaimed_is_reclaimable`, `superseded_disposition`,
  `orphan_is_reclaimable`), plus `generation_is_aged` and `restart_backstop`
  (`utimensat` with `UTIME_OMIT` for atime, mtime from the injected clock).
  `reap_orphans` removes the sidecars of each generation it reclaims.
- Both fixtures close inherited descriptors through `Dir::new` over a
  self-opened `/dev/fd`, skip the listing's own descriptor, and use distinct
  exit statuses. Fixture naming follows the `<crate-dir>-fixture-*` rule.
- Work item 0334 records the descriptor-floor scope deviation, the test
  split, the corrected accepted-gap assumption and the backstop limits. It
  also records the descendant answer: Chromium holds no lease, and the daemon
  holds the leases on fds 10 and 11.

#### Deviations from Plan:

- `LOWEST_LEASE_DESCRIPTOR` is declared as `i32` at crate level rather than as
  `RawFd` inside the `cfg(unix)` module. This is an improvement: the
  `IDENTITY_FD` assertion compiles on every target.
- `SharedLease::file()` is `pub(crate)` and `#[cfg(test)]`, which the plan
  left unspecified.
- `SupersededDisposition` derives `Debug, Clone, Copy, PartialEq, Eq` so the
  rule-cell tests can `assert_eq!` on it.
- The `reap.rs` rule-cell tests reuse the integration tests' names (for
  example `an_unclaimed_held_generation_is_kept_once_aged`). That is harmless
  across binaries but noisy when filtering.

#### Potential Issues:

- An `OutsideTreeLayout` tree is not existence-checked, so the executor can
  proceed on a missing tree with a warning, and the Node spawn then fails
  downstream. The plan specifies exactly this (never touch a foreign path),
  and `cache ensure` is a trusted producer, so the risk is low.
- `close_inherited_descriptors`/`descriptors_above_stdio` are duplicated
  verbatim across the two fixture binaries in different crates. This is
  acceptable for test fixtures, and a shared test-support crate would cost
  more than it saves.
- The work item's frontmatter `status` is still `ready`, although every
  acceptance criterion is ticked.

### Manual Testing Required:

1. Cold path on a real install:
   - [ ] With an empty scratch `ACCELERATOR_CACHE_DIR`, run one
         `inventory-design` crawl step, then confirm with
         `lsof <cache>/trees/*.lease` that the daemon's pid holds both lease
         files
2. Cross-version prune (scratch root only):
   - [ ] Point a second plugin version at the same root, run
         `accelerator cache prune --older-than 0`, and confirm the first
         version's generations survive while its daemon runs
   - [ ] Stop the daemon (`daemon-stop` or `kill` its process group), prune
         again, and confirm the generations are reclaimed

### Recommendations:

- Prune `cli/target` periodically (`cargo sweep` or `cargo clean`). Past
  several hundred thousand entries, every test that touches the macOS keychain
  slows down and can fail.
- Move work item 0334 to `done` once the PR merges.
