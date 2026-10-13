---
type: "plan-validation"
id: "2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness-validation"
title: "Validation Report: Tree Leases on the Cold Path and Under Unknown Liveness Implementation Plan"
date: "2026-10-13T13:59:49+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness"
tags: ["runtime", "cache", "lease", "prune", "design", "tree-lease"]
last_updated: "2026-10-13T13:59:49+00:00"
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
⚠️ Phase 6: Cross-launcher criteria and close-out - Implemented, but its
"`mise run` exits 0" criterion does not hold (see below)

### Automated Verification Results

✗ `mise run` exits 1. The one failure is
`accelerator-corpus::frontmatter_goldens this_repositorys_own_corpus_is_clean`:
`DUPLICATE-ID — 'work-item:0330'`. Trunk already carries
`meta/work/0330-skill-command-grants-that-survive-across-queries.md` (from
"Capture the skill-grant permission work items"), and this branch adds
`meta/work/0330-tree-leases-are-not-held-on-the-cold-path-or-honoured-when.md`
with the same ID. nextest fail-fast then cancelled 4,575 tests. The branch's
work item has since been renumbered to 0334.
✓ Every format, lint, type, `pup`, `deny`, `public-api`,
`crate-dependencies` and `notices` lane in the same run passed, and the run
left no formatting changes in the working copy.
✓ Python lanes in the same run: `test:unit:tasks` (3,549),
`test:integration:entrypoint` (66), `test:integration:dev` (17),
`test:integration:conformance` (30), `test:integration:hooks` (29),
`test:integration:research` (4).
✓ Full CLI workspace re-run with `--no-fail-fast`: 5,346 of 5,357 pass. The 11
failures are the duplicate-ID test above and 10 tests in
`github::octocrab_client`, where `OctocrabClient::with_base_uri` returns
`Err("Other")` after ~28s. Those 10 also fail in isolation and outside the
sandbox. The branch changes nothing under `cli/github`, and its `Cargo.lock`
diff only adds workspace edges, so the failures are environmental. I did not
confirm this on trunk.
✓ Every new suite passes: `tree-lease` unit tests, `executor_runtime` (7),
`tree_leases` (8), the `reap.rs` rule, boundary and sidecar tests, and the
`tree_resolution` seam, `Unknown` table, superseded and cross-launcher tests.

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

- Work-item ID collision: `0330` was claimed twice in `meta/work/`. Resolved
  by renumbering this branch's work item, and every reference to it, to 0334.
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
3. Environment:
   - [ ] Confirm `github::octocrab_client` fails the same way on trunk on this
         host, to rule this branch out

### Recommendations:

- Re-run `mise run` now that the work item is renumbered to 0334.
- Investigate the `octocrab_client` construction failure separately. It is
  not caused by this branch, but it will also keep `mise run` red on this
  host.
- Move work item 0334 to `done` once the PR merges.
