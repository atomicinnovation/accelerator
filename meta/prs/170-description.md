---
type: "pr-description"
id: "170"
title: "[0218] Bound a relocated cache root's growth with claim-gated pruning"
date: "2026-10-25T08:07:18+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0218"
parent: "work-item:0218"
relates_to: ["work-item:0334", "work-item:0339", "work-item:0340", "work-item:0341", "work-item:0342"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/170"
pr_number: 170
tags: ["cli", "launcher", "cache", "tree", "prune", "claims", "retention"]
revision: "c2bac5a4db53e138f72dcf35371299a05b5eace3"
repository: "accelerator"
last_updated: "2026-11-09T13:25:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0218] Bound a relocated cache root's growth with claim-gated pruning

## Summary

A cache root relocated with `ACCELERATOR_CACHE_DIR` outlives every plugin
installation that shares it, and every version left sub-binaries, launcher
entries, staged verifier copies, trees and crash residue behind that nothing
reclaimed. This PR implements ADR-0072 for such a root: every invocation
refreshes a version claim, every cold path runs a claim-gated root prune under
locks it only tries, and `accelerator cache prune` runs the same prune waiting
for its locks. The default root (`${ACCELERATOR_PLUGIN_ROOT}/bin`) keeps
ADR-0063's behaviour; its warm dispatch makes exactly the filesystem calls it
made before, pinned by a test.

## Changes

**Breaking**

- `cache prune` reports every failure instead of stopping at the first: a
  lock it cannot take, a removal that fails, a directory it cannot list or a
  reap that cannot start exits non-zero after pruning everything else. A
  failed removal was silently ignored before.
- `cache prune --older-than` also sets the version-claim window, as well as
  the tree-claim window it set before.
- `cache prune` refuses a relocated root other users can write to, removing
  nothing and naming the directory at fault and its fix; it pruned the pinned
  trees and exited 0 before. A materialise in such a root no longer reaps.
- `bin/accelerator` fetches a launcher that vanishes before its final `exec`
  once more, and exits 1 (0 under `--fail-safe`) only when that fails too,
  where it exited 126 or 127 depending on the bash version.

**Root identity and version claims (`resolve/cache_root.rs`, `retention/`)**

- `identify` canonicalises the override and the plugin `bin` and reports a
  `CacheRootKind` of `PluginBin`, `Relocated` or `Unconfirmed`: an override
  that does not resolve yet, or that has no plugin `bin` to compare it with,
  is neither claimed, pruned nor reaped. `retention::for_root` is the only
  code that matches on the kind.
- In a relocated root that passes the safety walk, every invocation, built-ins
  included, writes `claims/<version>.<launcher-id>` before any `find`, at most
  once per `CLAIM_WINDOW/10` (~1.4 days). The write is best-effort, so a
  read-only root still dispatches warm.
- The first claim writes `claims/.since`; until it is one `CLAIM_WINDOW` old
  no version's entries are removed, so a launcher from an earlier 1.24.0
  pre-release sharing the root keeps working through the upgrade.

**Root prune (`retention/root_prune.rs`, `single_file.rs`, `residue.rs`)**

- Under `.accelerator-prune.lock`, single-file prune removes the entries of
  versions no claim from the last 14 days protects, never the running
  version's. It removes a version's last claim only after all of its entries,
  and re-checks a claim's age immediately before unlinking it. A name grammar
  keeps everything the plugin does not produce.
- Every staged verifier copy is kept while another version holds a fresh
  claim. An aged half of an entry and its signature is swept while its
  version is claimed, so a one-sided failure no longer leaks a file for good.
- The residue sweep removes aged probes, plugin `.tmp-` staging and bootstrap
  lock directories whose `pid` is empty or dead; one holding anything else is
  kept and logged.
- A claim whose `lstat` fails for any reason but its being gone makes the
  whole listing unreadable, in both claim readers, which now share one
  `claims_directory` listing.

**Tree prune (`tree/prune.rs`, `tree/reap.rs`, `tree/locks.rs`)**

- `TreePrune` is the one implementation of tree reclamation in both roots:
  `PinnedOnHost` for the default root, `WholeTree` for a relocated one, which
  also covers retired artifacts, other platforms, retired tree layouts, stale
  tree claims, orphan sidecars and orphan lock files.
- A pointer is validated against the generation grammar before the generation
  it names is removed, so an absolute path, `..` or another artifact's
  generation is never followed. The pointer goes before its generation, under
  an exclusive lock on its `.lease`, after the digest's claims are re-read;
  only claims the launcher may sweep are unlinked with it.
- The reaper removes through the filesystem port and reports every removal
  that fails.

**Root safety (`ownership/safe_root.rs`, `ownership/accounts.rs`, ADR-0074)**

- A root is pruned only when it is writable by its owner alone, or through
  the owner's private group, and every ancestor is owned by the user, by root
  or by a uid in `ACCELERATOR_CACHE_TRUSTED_OWNERS` and is not writable by
  others without the sticky bit. Any macOS ACL grant beyond reading refuses,
  every directory's volume is asked whether it ignores ownership, and both
  `getent` queries share one 2 s deadline.
- `claims/`, `trees/` and `trees/claims/` are judged by the walk's own root
  verdict.

**Cold-path triggers and recovery (`resolve/mod.rs`, `core.rs`, `main.rs`)**

- A successful `store` and a successful tree materialise each run a root
  prune with `LockPatience::Try`, so a held lock is skipped, never waited for,
  and a prune problem is logged, never surfaced as a dispatch failure. A cold
  path stops starting removals after about two seconds and leaves the rest to
  later cold paths.
- A dispatch whose entry vanishes between `find` and `exec` re-resolves once
  through the cold path, in every root. `store` replaces a corrupt entry by
  renaming over it, with no unlink first.

**Ports, test seams and the release guard**

- A `CacheFilesystem` port, with trust inspection split into its own
  `TrustInspection` port, reaches every call that retention, start-up, tree
  claims, the reaper and a materialise's bookkeeping make; a `ports_only` test
  bans direct `std::fs` in those modules. Collaborators take narrow port views,
  every prune returns one `PruneReport`, and the `cache prune` use case owns
  the `RootRetention` port.
- The filesystem port's test doubles compile only into debug builds, so no
  release build carries them.
- The `test-loopback` markers in `jira`, `linear` and `research` are now string
  literals read at start-up, so they survive the release profile's strip and
  LTO. The launcher gains the same feature, with a `compile_error!` on release.
  `test:integration:release-guard` proves each guard refuses to compile and
  that a forced build is rejected.

**Docs, measurement and housekeeping**

- `internals.md` gains "A relocated cache root". The `design.md` rows for
  `cache prune` and `ACCELERATOR_CACHE_DIR` are updated. `CHANGELOG.md` has
  nine entries. ADR-0074 (proposed) records the multi-user safety model that
  ADR-0072 left open.
- `meta/measurements/warm-dispatch-8.json` records a warm-dispatch run after
  the change.
- The launcher adds `exacl` (macOS ACLs) and `libc`; the docs site bumps three
  `@babel` packages.
- Work items 0340–0342 capture pruning a group-shared root, which this PR
  deliberately leaves unpruned.

## Context

- Work item: `meta/work/0218-bound-cache-root-growth.md` (PP-747)
- Decision: `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
- Plan: `meta/plans/2026-10-19-0218-bound-cache-root-growth.md`
- Research: `meta/research/codebase/2026-10-18-0218-bound-cache-root-growth.md`
- Validation: `meta/validations/2026-10-19-0218-bound-cache-root-growth-validation.md`
- Builds on 0334 (tree leases on the cold path) and 0339 (the cross-artifact
  lock probe and inode confirmation).

## Testing

- [x] `mise run check` exits 0 across every component, and the launcher's
      whole suite passes; `mise run` end to end is re-run before this lands.
- [x] Each of the 50 acceptance criteria maps to a test that seeds and asserts
      every clause; the map is in the plan's Testing Strategy.
- [x] Mutation checks: each fail-closed branch of the safety walk, the
      cold-path log-once refusal and warnings, an early stop after a removal
      failure, a disabled cold-path budget, the claim and lease ports and each
      `compile_error!` guard fail the tests written for them.
- [x] The A/B/C scenario runs through `bin/accelerator` against a loopback
      release and asserts the exact set of names left in the root.
- [x] Clippy is clean for `x86_64-unknown-linux-musl` as well as the host.
- [x] The stripped release launcher is 16,909,904 bytes against 16,718,752
      for `main` at `8ffa9822` (+1.1 %), both before the latest rebase.

## Notes for Reviewers

- ⚠️ `warm-dispatch-8.json` shows a ~5.8 ms rise in median G-fast, almost all
  in the shim's minisign check of the launcher, which a 1.1 % larger binary
  does not explain. Base and head have not yet been re-timed on one quiet
  host; until they are, the end-to-end warm path is not claimed unchanged.
  The default-root launcher path is unchanged and pinned by
  `an_unset_override_performs_exactly_the_fixed_calls`.
- ⚠️ Launchers from earlier 1.24.0 pre-releases write no claims. Once a root
  has been claim-aware for 14 days, their entries are evicted on every
  claim-aware prune, and they cannot re-resolve. Launchers older than 0339
  also carry the orphan-lock risk `internals.md` names.
- Residual from validation, unrelated to this change:
  `design-adapters::spawn_properties::the_child_s_streams_are_redirected_to_the_bootstrap_log`
  can flake under load.

https://claude.ai/code/session_01CvXx6QsUHQGsrfswrYE3s3
