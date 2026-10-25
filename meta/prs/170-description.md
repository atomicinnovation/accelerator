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
revision: "72ce477a5fd7cb9edaacf4c80d10e9eec40240ff"
repository: "accelerator"
last_updated: "2026-10-25T08:07:18+00:00"
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
  lock it cannot take or a removal that fails exits non-zero after pruning
  everything else. A failed removal was silently ignored before.
- `cache prune` refuses a relocated root other users can write to, removing
  nothing and naming the directory at fault and its fix; it pruned the pinned
  trees and exited 0 before. A materialise in such a root no longer reaps.
- `bin/accelerator` exits 1 (0 under `--fail-safe`) when the launcher vanishes
  before its final `exec`, where it exited 126 or 127 depending on the bash
  version.

**Root identity and version claims (`resolve/cache_root.rs`, `retention/`)**

- `identify` canonicalises the override and the plugin `bin` and reports a
  `CacheRootKind` of `PluginBin` or `Relocated`. `retention::for_root` is the
  only code that matches on it, choosing a `RootRetention` once at start-up.
- In a relocated root, every invocation, built-ins included, writes
  `claims/<version>.<launcher-id>` before any `find`, at most once per
  `CLAIM_WINDOW/10` (~1.4 days). The write is best-effort, so a read-only
  root still dispatches warm.

**Root prune (`retention/root_prune.rs`, `single_file.rs`, `residue.rs`)**

- Under `.accelerator-prune.lock`, single-file prune removes the entries of
  versions no claim from the last 14 days protects, never the running
  version's. It removes a version's last claim only after all of its entries,
  and re-checks a claim's age immediately before unlinking it. A name grammar
  keeps everything the plugin does not produce.
- The residue sweep removes aged probes, plugin `.tmp-` staging and bootstrap
  lock directories whose `pid` is empty or dead.
- The root lock is released before any tree lock is taken.

**Tree prune (`tree/prune.rs`, `tree/locks.rs`)**

- `TreePrune` is the one implementation of tree reclamation in both roots:
  `PinnedOnHost` for the default root, `WholeTree` for a relocated one, which
  also covers retired artifacts, other platforms, stale tree claims, orphan
  sidecars and orphan lock files.
- A pointer is validated against the generation grammar before the generation
  it names is removed, so an absolute path, `..` or another artifact's
  generation is never followed. The pointer goes before its generation, under
  an exclusive lock on its `.lease`, after the digest's claims are re-read.
- The verb no longer creates tree locks for artifacts it never materialised,
  and an unreadable `trees/claims/` now reclaims nothing.

**Root safety (`ownership/safe_root.rs`, `ownership/accounts.rs`)**

- A root is pruned only when it is writable by its owner alone, or through
  the owner's private group, and every ancestor is owned by the user, by root
  or by a uid in `ACCELERATOR_CACHE_TRUSTED_OWNERS` and is not writable by
  others without the sticky bit. macOS ACLs and "Ignore ownership" volumes
  are checked; private-group membership is resolved through `getent` with a
  2 s bound.

**Cold-path triggers and recovery (`resolve/mod.rs`, `core.rs`, `main.rs`)**

- A successful `store` and a successful tree materialise each run a root
  prune with `LockPatience::Try`, so a held lock is skipped, never waited for,
  and a prune problem is logged, never surfaced as a dispatch failure.
- A dispatch whose entry vanishes between `find` and `exec` re-resolves once
  through the cold path, in every root. `store` replaces a corrupt entry by
  renaming over it, with no unlink first.

**Test seams and the release guard**

- A `CacheFilesystem` port, plus a recording filesystem, scripted filesystem
  and recording locks for tests.
- The `test-loopback` markers in `jira`, `linear` and `research` are now string
  literals read at start-up, so they survive the release profile's strip and
  LTO. The launcher gains the same feature, with a `compile_error!` on release.
  `test:integration:release-guard` builds each one and checks it is rejected.

**Docs, measurement and housekeeping**

- `internals.md` gains "A relocated cache root". The `design.md` rows for
  `cache prune` and `ACCELERATOR_CACHE_DIR` are updated. `CHANGELOG.md` has
  seven entries.
- `meta/measurements/warm-dispatch-8.json` records a warm-dispatch run after
  the change.
- `tokio` 1.53.1 → 1.53.3 for RUSTSEC-2026-0386, changing only its lock
  entry. The docs site takes npm's non-breaking `dompurify` and `undici`
  fixes, and carries a dated ignore for `zod` GHSA-mrhx-c7xc-m3w5, whose only
  path requires `zod` ^3, which has no patched release.
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

- [x] `mise run` exits 0 end to end: 6,318 CLI tests, every integration lane
      including `entrypoint` and `release-guard`, every check and
      `docs:check`.
- [x] Each of the 50 acceptance criteria maps to a test that seeds and asserts
      every clause; the map is in the plan's Testing Strategy.
- [x] Mutation checks: making the default root prune as a relocated root
      does, creating a lock for a sidecar-only artifact, and running the
      residue sweep without the root lock each fail the tightened tests. The
      tests these replaced passed under the first two.
- [x] The A/B/C scenario runs through `bin/accelerator` against a loopback
      release and asserts the exact set of names left in the root.
- [x] A cold dispatch against a relocated root seeded with two unclaimed
      `browser` trees (2,000 files and 256 MiB each, debug builds,
      Apple-silicon Mac) removed both: `visualiser` took 1.62 s against
      1.31 s with no trees, and
      `vcs detect --format=hook --fail-safe --descriptive` took 1.48 s
      against 1.29 s.
- [x] Manual relocated-root checks from the plan, run during implementation:
      a first dispatch leaves one claim; an old version's back-dated entries
      are gone before a cold dispatch runs; the plugin `bin/` is never claimed
      or pruned.

## Notes for Reviewers

- ⚠️ The first cold path after an upgrade removes every unclaimed tree
  synchronously before `exec`, roughly 0.2–1 s per `browser` or `driver` tree
  on local disk. It usually lands on the first SessionStart hook of a new
  version. The work item accepts this, because its A/B/C criterion needs
  every unclaimed tree gone after one dispatch.
- ⚠️ Launchers older than this change write no claims. While one shares a
  relocated root, its entries are evicted after each claim-aware prune and
  its session re-fetches until the plugin is reloaded. Launchers older than
  0339 also carry the orphan-lock risk `internals.md` names.
- `warm-dispatch-8.json`'s `closure_verdict` is `false` (raw ratio 1.84). The
  work item asks for the JSON whatever its gating result. The default-root
  warm path is unchanged and pinned by
  `an_unset_override_performs_exactly_the_fixed_calls`.
- Residual issues from validation, none against an acceptance criterion:
  - `tree/claims.rs:183` drops a claim whose `lstat` fails for any reason
    other than `NotFound`, so a readable but unsearchable `trees/claims/`
    still fails open per claim.
  - A bootstrap lock directory holding more than `pid` makes the residue
    sweep report a removal failure on every prune.
  - `design-adapters::spawn_properties::the_child_s_streams_are_redirected_to_the_bootstrap_log`
    can flake under load. It is unrelated to this change, which leaves
    `design-adapters` untouched.

https://claude.ai/code/session_01CvXx6QsUHQGsrfswrYE3s3
