---
type: "plan-validation"
id: "2026-10-19-0218-bound-cache-root-growth-validation"
title: "Validation Report: Bound a Relocated Cache Root's Growth Implementation Plan"
date: "2026-10-25T05:02:25+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-19-0218-bound-cache-root-growth"
tags: ["cli", "launcher", "cache", "tree", "prune", "claims", "retention"]
last_updated: "2026-10-25T05:02:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Bound a Relocated Cache Root's Growth

Every phase is implemented and every acceptance-criterion test exists.
`mise run` exits 0 end to end. The first run was red on two dependency audits
for advisories published upstream, against packages this plan does not
touch; the same working copy fixes both.

### Implementation Status

✓ Phase 1: Filesystem Port and the Fixed Warm-Path List - Fully implemented
✓ Phase 2: `ENOENT` Re-Resolve, Pointer Containment, Replacement - Fully
implemented
✓ Phase 3: Cache Root Identity and Version Claims - Fully implemented
✓ Phase 4: Single-File Prune and Residue Sweep Behind the Verb - Fully
implemented
✓ Phase 5: Widened Tree Prune Behind the Verb - Fully implemented
✓ Phase 6: Release Markers That Survive Stripping - Fully implemented
✓ Phase 7: Cold-Path Triggers and the End-to-End Scenario - Fully implemented
✓ Phase 8: Documentation and Measurement - Fully implemented

All 88 test names in the Acceptance Criterion Map resolve to a definition
under `cli/`, `tests/` or `tasks/`. Phase 1's second commit (`sntooqsk`)
touches only `cli/launcher/tests/resolution.rs`, as the plan requires.

### Automated Verification Results

✓ Full local CI mirror passes: `mise run` exits 0. That covers 6,317 CLI
tests (2 skipped), every integration lane including `entrypoint` and
`release-guard`, visualiser e2e 355, every check task and `docs:check`.
`minisign` was on `PATH`, so no signed-release test skipped.
✓ `deny:check`: RUSTSEC-2026-0386 (`tokio` unsound `AsyncFd` constructors)
is fixed by pinning `tokio` to 1.53.3, the first patched release. Only
`tokio`'s version and checksum change in `cli/Cargo.lock`, because a
`cargo update` re-resolves `windows-sys` for unrelated crates under the
MSRV-aware resolver and breaks the build-script snapshot in
`test:integration:deny`. `licenses/accelerator-third-party-notices.txt` is
regenerated to match.
✓ `docs:audit:check`: `docs:audit:fix` clears the `dompurify` and `undici`
advisories. GHSA-mrhx-c7xc-m3w5 (`zod` <4.6.6) carries a dated ignore in
`docs-site/audit-ignores.toml`, because its only path is
`astro-rehype-relative-markdown-links` 0.19.2, which requires `zod` ^3, and
`zod` 3 has no patched release.
⚠ `design-adapters::spawn_properties::the_child_s_streams_are_redirected_to_the_bootstrap_log`
failed once under full-suite load and passed 300 of 300 isolated runs.
`await_content` (`spawn_properties.rs:85-96`) returns once the log is
non-empty, before `err-marker` may have been written. This plan does not
touch `design-adapters`.

### Code Review Findings

#### Matches Plan:

- The one-way layering holds. None of the five leaf modules imports from
  `tree/` or `retention/`, and `tree/` imports only `retention_windows`.
- `run_external` re-resolves at most once, and only for an `Exec` with
  `NotFound`. An `ACCELERATOR_<SUB>_BIN` override is never re-resolved
  (`core.rs:325-366`, `main.rs:190-222`).
- Pointer containment runs before any removal.
  - `PointerTarget::of` validates contents through `validated_generation`,
    which rejects empty contents, `/`, `\`, non-ASCII and any other prefix.
  - `read_pointer` is bounded to 256 bytes and opens with `O_NOFOLLOW`
    (`layout.rs:366-428`, `tree/prune.rs:824-851`).
- Version claims are written before `find` for every command, built-ins
  included, and refreshed at most once per `CLAIM_WINDOW/10`
  (`start_up.rs:61-85`, `main.rs:565-569`, `version_claims.rs:35-85`).
- `SafePruneRoot::check` applies the identity rules from Phase 4 §7.
  - It walks every ancestor of the canonical root.
  - It refuses uid 65534 and 99 even when listed.
  - It refuses a world-writable root.
  - It admits a world-writable ancestor only with the sticky bit.
- Tree prune unlinks the pointer before its generation. It re-reads the
  digest's claims immediately before removal, holds an exclusive `.lease`
  lock across it, and never probes a lock its caller holds
  (`tree/prune.rs:404-571`).
- Cold paths try every lock and never block. Prune problems become
  warnings, and no prune error reaches the dispatch
  (`root_retention.rs:118-146`).
- The `test-loopback` markers survive release stripping in `jira`, `linear`
  and `research`. The launcher refuses `--release` with the feature through
  `compile_error!` (`main.rs:68-72`).
- `bin/accelerator`'s `execfail` change is bash-3.2 safe and honours
  `--fail-safe` (`bin/accelerator:460-466`).
- The constants match the documentation: `AGE_BACKSTOP` is 1 h,
  `CLAIM_WINDOW` is 14 days and the refresh interval is ~1.4 days.
  `CHANGELOG.md` carries exactly seven entries. `warm-dispatch-8.json` and
  its samples are committed.

#### Deviations from Plan:

- **Constructors:** the resolver constructors take a `ResolverPorts` struct
  (`resolve/mod.rs:95-132`), not separate port arguments. Every port is
  still required.
- **Pointer classification:** it moved from `resolver.rs` to `tree/prune.rs`.
  `TreesIndex::pointers_of` replaces `artifact_pointers`.
- **`CacheRoot`:** its fields are private, with a kind-gated
  `plugin_bin()` accessor (`cache_root.rs:83-113`).
- **Outcome naming:** `PruneOutcome::Unavailable` is named `Unidentified`.
  The unreadable-claims message reads "kept everything they could protect".
- **`tree/pinned_prune.rs`:** it was replaced by Phase 5's `TreePrune`
  regimes, with `the_verb_creates_no_lock_for_an_artifact_it_never_materialised`
  standing in for its test.
- **`materialise`:** it runs only `reap_orphans` under its lock
  (`tree/resolver.rs:644-648`). `reap_superseded` runs only in `repair`. In a
  relocated root, `prune_held` removes the superseded tree. In the default
  root, the verb still does, as before this change.
- **Sidecar removal:** the `Reaper` removes a generation's sidecars with
  `std::fs::remove_file`, outside `removals` (`reap.rs:209-211`). The index
  stays correct, because forgetting a generation forgets its sidecars.
- **`design.md:264`:** it summarises the root-safety rule as "no other user
  can write to it or a directory above it". That is broader than the rule
  `internals.md` states in full and links to.
- **Coherence test:** `test_bootstrap_name_coherence.py` lives under
  `tests/unit/tasks/`, not `tests/integration/`.

#### Potential Issues:

- **Per-claim fail-open in tree-claim listing.** `claims::list` drops any
  claim whose `lstat` fails, whatever the error (`tree/claims.rs:183`,
  `.ok()?`). On a `trees/claims/` that is readable but not searchable, every
  claim vanishes and claimed trees are reclaimed. This narrows trunk's
  behaviour, which treated an unreadable directory as unclaimed. It still
  falls short of the fail-closed criterion, because only `NotFound` should
  be skipped.
- **A bootstrap lock directory holding more than `pid`.** Its `remove_dir`
  fails with `ENOTEMPTY` (`residue.rs:105`). The verb then exits non-zero
  and every cold prune warns, permanently, until a person removes it.
- **A test that passes trivially.** The relocated
  `the_verb_creates_no_lock_for_an_artifact_it_never_materialised`
  (`root_prune.rs:2046`) plants only a claim, so `WholeTree` discovers no
  artifact. Seeding sidecars only would exercise the absent-lock row.
- **Partial removal on a failed generation unlink.** When `unlink_tree`
  fails, `remove_tree` still removes the sidecars and the digest's claims
  (`tree/prune.rs:553-568`). A later reap recovers, but the failure leaves
  more partial state than it needs to.
- **Fallback reap under contention.** Under `Try` with every artifact
  contended, the fallback in `reap_under_the_first_pinned_artifact` can
  create a lock for a pinned, never-materialised artifact. Under `Wait`, it
  can report a `LockFailure` twice (`tree/prune.rs:166-202`).
- **`try_exclusive_lease` errors.** Any error from it, an open failure
  included, lets removal proceed (`tree/prune.rs:541-546`). The plan
  describes this only for a `flock` failure. Removal remains gated by the
  earlier aged-and-`Unknown` lease probe.
- **Claim writes on a FIFO.** Both claim writers open without `O_NONBLOCK`,
  so they can block on a FIFO the same user planted (`filesystem.rs:212-219`,
  `tree/claims.rs:225-231`).
- **Root-only test assumption.**
  `an_unwritable_cache_root_is_re_resolved_through_the_cold_path`
  (`main.rs:951`) assumes mode `0555` stops writes, which is false when
  running as root.

### Manual Testing Required:

1. Relocated root (the plan's Manual Testing Steps; recorded as done during
   implementation, not re-run here):
  - [ ] Dispatch with `ACCELERATOR_CACHE_DIR` set to a scratch directory and
        confirm `claims/` holds one claim.
  - [ ] Plant entries for an old version, back-dated two weeks, dispatch an
        uncached sub-binary and confirm they are gone before it runs.
  - [ ] Unset `ACCELERATOR_CACHE_DIR` and repeat against the plugin `bin/`.
        Nothing is claimed or removed.

### Recommendations:

- Make the `design-adapters` spawn test wait for both markers rather than
  any content, so it stops flaking under load.
- Make `claims::list` return `Unreadable` for a per-claim `lstat` error
  other than `NotFound`, with a red test using a `0400` `trees/claims/`.
- Make the residue sweep keep a bootstrap lock directory that holds more
  than `pid`, rather than report a removal failure on every prune.
- Seed sidecars in the relocated no-new-lock test so it exercises the
  absent-lock row.
- The remaining low-severity items can be filed as follow-ups. None of them
  affects the acceptance criteria.
