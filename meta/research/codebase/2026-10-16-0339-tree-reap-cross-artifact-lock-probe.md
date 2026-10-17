---
type: "codebase-research"
id: "2026-10-16-0339-tree-reap-cross-artifact-lock-probe"
title: "Tree reap cross-artifact lock probe: implementation surface for 0339"
date: "2026-10-16T11:30:09+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0339"
parent: "work-item:0339"
relates_to: ["codebase-research:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness"]
topic: "Tree reap cross-artifact lock probe: implementation surface for 0339"
tags: ["research", "codebase", "launcher", "tree", "reap", "lease", "single-flight", "concurrency"]
revision: "33b9177c7282c5e5af468ae7cefed73a8a1ee4e3"
repository: "accelerator"
last_updated: "2026-10-16T11:30:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tree reap cross-artifact lock probe: implementation surface for 0339

**Date**: 2026-10-16 11:30 UTC
**Author**: Toby Clemson
**Git Commit**: `33b9177c7282c5e5af468ae7cefed73a8a1ee4e3` (jj change `tulqnutvmuxy`)
**Branch**: none (anonymous jj working-copy change)
**Repository**: accelerator

## Research Question

What does the code look like today around `reap_orphans`, the tree
single-flight lock and its two callers, and what does that imply for
implementing work item 0339 (a reap for one artifact deletes another
artifact's in-progress temp tree, `.ref` sidecar and partial archive)?

## Summary

The bug is as the work item describes. `reap_orphans`
(`cli/launcher/src/launch/outbound/resolve/tree/reap.rs:41-89`) scans the
whole of `trees/` and removes every `.tmp-*` entry that is not a `.lock` and
not a kept archive. It does not look at artifact, platform, lock or age. The
keep set is a single digest matched by substring (`reap.rs:109-114`).

Four facts shape the fix. Three of them correct or sharpen the work item:

- **Caller name**: the post-materialise reap is in
  `MaterialiseTree::materialise` (`resolver.rs:555-624`, reap at `:603-611`),
  not `acquire`. `AcquireSealedTree::acquire` (`resolver.rs:510-551`) takes
  no tree lock and never reaps.
- **No platform set**: the platform is a plain `String`, and nothing
  enumerates the known platforms. The four values appear only as
  `cfg`-selected `HOST_PLATFORM` constants (`resolve/mod.rs:21-29`) and as
  the keys of `pins.toml`. The parsing rule in the work item needs a new
  known-platform list.
- **Reusable probe exists, but private**: the open-without-create plus
  non-blocking `LOCK_EX` that 0339 needs already exists as `probe_flock` in
  `cli/lease/src/lib.rs:225-241`. It is private to the `lease` crate, maps
  every outcome to `Liveness`, and unlocks at once. 0339 needs a variant
  that returns the held guard.
- **Pins for any artifact**: `ExpectedDigests::digest_for(artifact,
  platform)` (`resolver.rs:87-97`) already answers for any pair, in both
  `Compiled` and `Fixed` modes. It is private to `resolver.rs`, so the reap
  needs it passed in as a pin lookup, replacing `keep_digests`.

The removal-observer seam that AC2 needs does not exist. The reaper's only
seams are `Clock` and `LivenessProbe`. `StepObserver` (`resolver.rs:39-69`)
is the closest pattern to follow.

## Detailed Findings

### The reaper today

`reap_orphans(paths, clock, liveness, keep_digests) -> Result<Reclaimed,
TreeError>` (`reap.rs:41-46`):

- **Error policy**: it fails only when `read_dir(trees/)` fails with
  something other than `NotFound` (`reap.rs:48-58`). Per-entry read and
  removal failures are swallowed, and the count rises only on success
  (`remove`, `reap.rs:281-290`). The 0339 rule "probe error keeps the
  entries and the reap returns `Ok`" fits this existing policy.
- **Pointers**: `referenced_generations` (`reap.rs:139-154`) builds the set
  of generations named by any `*.ref` in `trees/`, across all artifacts.
- **Classification** (`classify`, `reap.rs:104-129`):

| Entry name | Classified as | Removed |
|---|---|---|
| `.tmp-*.archive` with a keep digest as substring | `None` | never |
| `.tmp-*.archive` otherwise | `TempArchive` | always |
| `.tmp-*.lock` | `None` | never |
| any other `.tmp-*` (temp tree, `.tmp-<gen>.ref`) | `TempTree` | always |
| contains `-1-`, no sidecar suffix | `Generation` | unreferenced, not `Held`, aged ≥ `AGE_BACKSTOP` |
| `.ref`, `.sealed`, `.sealed.sig`, `.lease`, `claims/` | `None` | sidecars only alongside their generation |

- **Age**: `AGE_BACKSTOP` is 1 hour (`reap.rs:28`). It applies only to
  generations; temp residue has no age gate.
- **Decision rules**: `unclaimed_is_reclaimable`, `superseded_disposition`
  and `orphan_is_reclaimable` (`reap.rs:174-209`) are pure `const fn`s over
  an already-computed `Liveness` and `aged`. Clippy's `missing_const_for_fn`
  lint is denied, which forces `const`. This is where a per-artifact
  disposition rule belongs.

### The tree lock

Lock-related code lives in the launcher's `tree/lease.rs` and the separate
`cli/lease` crate:

- **Path**: `TreePaths::single_flight_lock(name, platform)` builds
  `trees/.tmp-<name>-<platform>.lock` (`layout.rs:131-134`).
- **`try_single_flight`** (`lease.rs:84-95`): `Ok(Some(SingleFlight))`
  means taken, `Ok(None)` means `WOULDBLOCK`, and any other error is
  `Err(TreeError::Lease)`. It opens through `open_lock_file`
  (`lease.rs:97-108`) with `create(true)` and mode `0600`.
- **`take_single_flight`** (`lease.rs:63-71`): blocking `LOCK_EX` through
  the same open. Used by `repair` and `prune`.
- **Guard**: `SingleFlight { _file: File }` (`lease.rs:53-55`). It has no
  `Drop`. Release is the close of the open file description, and the lock
  file stays on disk.
- **Primitive**: `rustix::fs::flock` everywhere. These two call sites
  (`lease.rs:65`, `:88`) are the only flock calls in `cli/launcher/src`.
  Non-Unix stubs are `unimplemented!` (`lease.rs:136-168`).
- **Reusable probe**: `probe_flock` (`cli/lease/src/lib.rs:225-241`) opens
  `read + write` without `create`. `NotFound` gives `Free`, any other open
  error gives `Unknown`, `WOULDBLOCK` gives `Held`, and success unlocks
  immediately and gives `Free`. `probe_shared_lease` wraps it with a
  `statfs` gate that maps NFS to `Unknown` (`lib.rs:166-223`). That gate is
  lease-specific; the work item does not ask for it on tree locks.

Opening for write is what makes AC6's fixture work: a directory placed at
the lock path fails with `EISDIR`. 0334 used the same trick for unleasable
trees, and `cli/lease/src/lib.rs:357-363` tests it for leases.

### The two callers

```text
materialise (resolver.rs:555)          prune (resolver.rs:730)
  verify_writable, ensure_trees_dir      take_single_flight (blocking) :736
  loop:                                  reclaim_unclaimed :758-792
    try_single_flight :593 ──held──►     keep = {expected_digest(artifact)}
    │ waiter bound / sleep               reap_orphans(..)?  :750
    ▼ taken                              (error propagates)
    materialise_locked :186
      stream_archive :251 → .tmp-…archive
      extract → .tmp-<gen>/
      rename, publish_pointer (.tmp-<gen>.ref :452)
    keep = {digest}
    let _ = reap_orphans(..)  :603-611
    (error discarded)
```

- **Lock scope**: the archive download, extraction, sealing, rename and
  pointer publication all run inside `materialise_locked`. The guard `_lock`
  drops at the end of the `if let` scope, after `reap_orphans` returns. A
  tree lock that can be taken therefore means no materialise or `prune` of
  that `(name, platform)` is in progress, as the work item assumes.
- **`prune` does not create `trees/`**: unlike `materialise`, it skips
  `ensure_trees_dir`. `take_single_flight` will therefore fail if `trees/`
  is absent. This is unchanged by 0339, but worth knowing for fixtures.
- **Whole-root loop**: `cache prune` (`launch/cache.rs:143-155`) calls
  `resolver.prune(artifact, …)` once per name in `pins::artifact_names()`
  (`browser`, `driver`), always on the host platform. Each call holds only
  its own lock, so after the fix each call's reap probes the other
  artifact's lock. That lock is free at that point, because the loop is
  sequential.
- **Archive resume**: `download::stream_archive` (`download.rs:40-76`)
  sends a `Range` from the partial file's length and appends on 206. A
  partial archive therefore has value only while its digest is still
  pinned. That is the basis for the per-artifact resumable rule.

### Pinned digests

Pins come from two places:

- **Compiled**: `pins.toml` (`[assembled_sha256.<artifact>] <platform> =
  "<digest>"`, 2 artifacts × 4 platforms) is turned into
  `EXPECTED_TREE_DIGESTS: &[(&str, &str, &str)]` by `cli/launcher/build.rs`.
  `pins::expected_digest_on(artifact, platform)` (`pins.rs:50-60`) looks it
  up.
- **Resolver**: `ExpectedDigests { Compiled, Fixed(BTreeMap<(String,
  String), String>) }` (`resolver.rs:79-97`). Its `digest_for` is private,
  and `TreeResolver::expected_digest` (`:130-132`) binds it to
  `self.platform`.

For the reap to apply "kept when its digest is the one pinned for the
archive's own artifact", it needs a lookup `(name, platform) ->
Option<digest>` passed in, plus an exact comparison against the digest it
parses from the archive name. The current substring match would also spare
an archive of the wrong artifact that happens to contain the digest, which
AC4 forbids.

### Entry names and parsing

There are production builders for every fixture except the temp pointer:

| Entry | Builder | Location |
|---|---|---|
| Generation | `generation_name(name, platform, digest, layout, suffix)` | `layout.rs:160-171` |
| Temp tree `.tmp-<gen>` | `TreePaths::temp_generation` | `layout.rs:126-128` |
| Temp pointer `.tmp-<gen>.ref` | none, inline `format!` in `publish_pointer` | `resolver.rs:452` |
| Partial archive | `TreePaths::temp_archive(name, platform, digest)` | `layout.rs:114-123` |
| Tree lock | `TreePaths::single_flight_lock(name, platform)` | `layout.rs:131-134` |

- ⚠️ **No temp-pointer builder**: AC1 requires fixtures to use production
  builders, so `.tmp-<gen>.ref` needs a `TreePaths` builder extracted from
  `resolver.rs:452` first. That is a refactor under existing tests.
- **No split on `-`**: `layout.rs:137-142` documents that names are never
  split on `-`, because artifact names and platforms both contain hyphens.
  `validated_generation` and `artifact_pointers` parse by `strip_prefix`
  against a known name. The work item's rule — split at the first
  `-<platform>-` after a non-empty name, using a known-platform list — is
  new parsing, and it is consistent with that constraint.
- **Archive suffix**: the archive's grammar is
  `.tmp-<name>-<platform>-<digest>.archive`. Parsing the platform with
  `-<platform>-` works for archives and for temp trees. A temp tree's
  remainder after the platform is `<digest>-1-<suffix>`, optionally with
  `.ref`.
- **Locks need no parse**: a lock name is `.tmp-<name>-<platform>.lock`,
  where the platform is followed by `.lock`, not `-`. Lock files never
  enter the grouping, and the lock path for a group comes from
  `single_flight_lock(name, platform)`.
- **Prefix collision** (AC10): `<Y-name>-extra-<platform>-…` parses to name
  `<Y-name>-extra`. That works because the split is decided by the platform
  token, not by a prefix match against Y's name.

### Test infrastructure

The fixtures split between the reaper's unit tests and the integration
suite:

- **Unit tests in `reap.rs:302-680`** use `tempfile`, the real `FlockProbe`,
  `AtClock` / `well_aged()` / `present()`, `trees()` and `gen_name()` (a
  production builder over `browser` / `linux-x64`). Nothing holds a tree
  lock, and there is no timeout harness.
- **Integration tests in `cli/launcher/tests/tree_resolution.rs`** use
  `Harness` (`:252-337`) with `resolver_with(clock, liveness, steps)` and
  `resolver_expecting(digest, clock)`. Every harness uses
  `ExpectedDigests::Fixed` with one `("driver", HOST_PLATFORM)` entry. AC3
  needs a resolver whose `Fixed` map pins both X and Y.
- **Lock holding**: `the_bounded_waiter_expires_with_materialisation_in_progress`
  (`:620-647`) already holds a tree lock in-process with
  `take_single_flight`, though it builds the path by hand (`:632`). An
  in-process `SingleFlight` is a separate open file description, so it
  conflicts with the reap's probe. No fixture binary is needed.
- **Separate-OFD check** (AC2): `the_single_flight_lock_is_exclusive`
  (`lease.rs:123-132`) checks a lock from a fresh `OpenOptions` open via
  `probe_shared_lease`. ⚠️ The 0334 documents mention a
  `tree_lease::observed_held` helper, but no such function exists in `cli/`
  at this revision. AC2's observer will need its own non-blocking probe from
  a fresh open, or the new non-creating probe.
- **Bounded timeout**: 0334's convention is 60s, after
  `spawn_properties.rs:73-91`, and `await_released` (`:2078-2088`) polls
  with a deadline. A reap on its own needs a different shape: run it on a
  spawned thread and `recv_timeout` on a channel, so a blocking probe fails
  the test instead of hanging it.
- **Observer seam**: none exists on the reaper. `StepObserver` /
  `MaterialiseStep` / `NoSteps` (`resolver.rs:39-69`) are the in-crate
  pattern: a `&dyn` trait object with a no-op production implementation.
- **CI**: `tree_resolution` needs `minisign`; `minisign_or_skip!` panics
  under `CI`.

## Code References

- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:28` - `AGE_BACKSTOP`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:41-89` - `reap_orphans`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:104-129` - `classify`, keep-set substring match at `:109-114`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:174-209` - pure `const fn` disposition rules
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:281-290` - best-effort `remove`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:302-680` - unit tests and fixtures
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:15-26` - `LivenessProbe`, `FlockProbe`
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:53-108` - `SingleFlight`, `take_` / `try_single_flight`, creating `open_lock_file`
- `cli/lease/src/lib.rs:225-241` - `probe_flock`, the non-creating open-and-try pattern
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:39-69` - `StepObserver` seam pattern
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:79-97` - `ExpectedDigests::digest_for`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:555-624` - `materialise`, lock at `:593`, reap at `:603-611`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:730-754` - `prune`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:452` - inline temp-pointer name
- `cli/launcher/src/launch/outbound/resolve/tree/layout.rs:114-171` - name builders
- `cli/launcher/src/launch/outbound/resolve/mod.rs:21-29` - `HOST_PLATFORM` constants
- `cli/launcher/src/launch/outbound/resolve/tree/pins.rs:19-60` - pin accessors
- `cli/launcher/src/launch/cache.rs:143-155` - per-artifact `prune` loop
- `cli/launcher/tests/tree_resolution.rs:252-337` - `Harness`
- `cli/launcher/tests/tree_resolution.rs:620-647` - in-process tree-lock holding

## Architecture Insights

The research points to the following design. It is a recommendation for the
plan, not settled.

```text
reap_orphans(paths, clock, liveness, own: Artifact, pins: &dyn PinLookup,
             removals: &dyn RemovalObserver)
  scan trees/ ─► classify ─► temp entries: parse (name, platform) via KNOWN_PLATFORMS
                    │                      │ unparseable ─► keep
                    │                      ▼
                    │               group by artifact (BTreeMap → sorted, deterministic)
                    │                      │
                    │      own ─────► remove (spare resumable archive)
                    │      other ───► probe_tree_lock(path) (no create, no block)
                    │                   Taken(guard) ─► remove under guard, drop
                    │                   Held         ─► keep
                    │                   Absent       ─► remove, no guard
                    │                   Err          ─► keep
                    ▼
               generations: unchanged (0334 rules)
```

- **Probe beside the existing lock code**: add a non-creating
  `try_single_flight` variant to `tree/lease.rs`, returning an outcome enum
  (`Taken(SingleFlight)`, `Held`, `Absent`) with errors as
  `Err(TreeError::Lease)`. 0218 will reuse it for the root prune and
  orphan-lock removal, and it must report `ENOLCK`-style failures
  separately. That is why errors should not fold into `Held`.
- **Artifact as a domain type**: `(name, platform)` is currently passed as
  two loose strings throughout. 0339 is the first code that parses one out
  of a name, groups by it and compares it with the reap's own artifact. A
  small value type with a `from_temp_entry` parser and a lock-path method
  expresses the work item's vocabulary directly.
- **Pure disposition**: following 0334, the
  own / taken / held / absent / error × resumable decision can be a pure
  `const fn` with one test per cell. The I/O (probe, remove) stays in
  `reap_orphans`.
- **Pin lookup replaces `keep_digests`**: both callers own an
  `ExpectedDigests`. Passing it, or a closure or trait over `digest_for`,
  removes both `keep` constructions (`resolver.rs:604-605`, `:746-749`).
  That way no caller can pass a single digest, which was the pass-2 review
  major.
- **Known platforms**: a `KNOWN_PLATFORMS` constant next to `HOST_PLATFORM`
  in `resolve/mod.rs`, with a test that every `pins::EXPECTED_TREE_DIGESTS`
  platform is in it, keeps the list from drifting. Deriving it from the pins
  table instead would couple parsing to what is pinned. That works against
  the "retired artifact" rule, which only concerns names, not platforms, so
  either source is acceptable.
- **Generations unchanged**: the generation path, which uses `.lease` and
  `AGE_BACKSTOP`, is outside 0339's rules and should keep its 0334
  behaviour.

## Historical Context

- `meta/work/0339-tree-reap-deletes-another-artifact-s-in-progress-materialise.md` - the work item; status `ready`.
- `meta/reviews/work/0339-tree-reap-deletes-another-artifact-s-in-progress-materialise-review-1.md` - verdict APPROVE. It left open the timeout bound value and "expected outcome verified only by proxy".
- `meta/plans/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md` - 0334 plan, done. It established the `LivenessProbe` seam, pure `const fn` rules, the 60s bound and in-process `ExpectedDigests::Fixed` for multi-digest scenarios. It accepted a probe-then-remove window for *leases*; 0339 deliberately rejects that window for *tree locks*.
- `meta/validations/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness-validation.md` - 0334 passed. It notes that unit-test names that duplicate integration-test names are noisy to filter.
- `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md` - root prunes take single-flight locks without blocking, and correctness assumes `flock` works on the root's filesystem.
- `meta/work/0218-bound-cache-root-growth.md` - blocked by 0339. It will reuse the non-creating probe, never remove a held lock file, and needs lock failures reported distinctly.
- `meta/reviews/work/0218-bound-cache-root-growth-review-1.md` - cited as where the bug was found. Its text covers tree-lock coverage and `ENOLCK`, not the race itself.

## Related Research

- `meta/research/codebase/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`

## Open Questions

- ❓ **Pin-lookup shape**: pass `&ExpectedDigests`, which means making
  `digest_for` `pub(crate)` and moving the type out of the private
  `resolver` module, or define a narrow trait in `reap.rs`?
- ❓ **Known-platform source**: a hand-written `KNOWN_PLATFORMS` with a
  drift test, or a list derived from `EXPECTED_TREE_DIGESTS` in `build.rs`?
- ❓ **Timeout bound**: the work item leaves it unstated. A short bound such
  as 5–10s is enough to detect a blocking probe; 60s matches 0334's
  convention for spawned processes.
- **Unverified**: I did not run any test, and did not confirm that
  `SingleFlight`'s descriptor has `FD_CLOEXEC` set. Rust's `File::open` sets
  it by default, and nothing in `lease.rs` clears it for tree locks, so
  holding X's lock in-process while a parallel test spawns `minisign` should
  not leak it.
- **Work item wording**: its Technical Notes and Summary call the
  post-materialise reap "the reap in `acquire`", but the code calls it
  `materialise`. The plan should use `materialise`.
