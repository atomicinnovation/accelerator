---
type: "plan-validation"
id: "2026-10-16-0339-tree-reap-cross-artifact-lock-probe-validation"
title: "Validation Report: Tree Reap Cross-Artifact Lock Probe Implementation Plan"
date: "2026-10-17T21:11:41+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-16-0339-tree-reap-cross-artifact-lock-probe"
tags: ["cli", "launcher", "cache", "tree", "reap", "concurrency"]
last_updated: "2026-10-17T21:11:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Tree Reap Cross-Artifact Lock Probe Implementation Plan

### Implementation Status

✓ Phase 1: Naming, Locking and Probe Groundwork - Fully implemented
✓ Phase 2: Per-Artifact Resumable Rule and Partials That Always Recover - Fully implemented
✓ Phase 3: Cross-Artifact Lock Probe - Fully implemented

Every test the plan names exists under its exact name. Each phase was
checked against the code by a separate analyser pass, and the findings
below were spot-checked by hand.

### Automated Verification Results

✓ `mise run` (bare default task) exits 0 end-to-end, at revision
`20dd4d85`, and leaves the working copy unchanged
✓ CLI suite: 5601 tests run, 5601 passed, 2 skipped, with `minisign` on
`PATH`, so the `tree_resolution` tests ran rather than skipping
✓ `rg -n 'format!\(".tmp-' cli/launcher/src` returns nothing
✓ `rg -n 'keep_digests' cli/launcher/src` returns nothing
✓ `resolver.rs` imports neither `take_single_flight` nor
`try_single_flight`
✓ `Resumed` no longer carries `#[allow(dead_code)]`
✓ `lease.rs` is in `ALLOWLIST` in `tasks/lint/store_duplication.py`

The red-before-green observations and the four over-correction mutations
are recorded in the plan as run. They were not re-run here.

### Code Review Findings

#### Matches Plan:

- `Artifact::owning` splits only at a known platform followed by a digest,
  so `linux-x64-musl` entries and lock files have no owner
- Every tree-lock cell of the `take_outcome`, `try_outcome` and
  `probe_outcome` tables matches. That includes `KeptBeingReplaced`
  mapping to `Held` for the probe and to `Ok(None)` for `try`
- `SingleFlight` keeps both descriptors and has one constructor,
  `confirmed`. No descriptor is closed between `flock` and building the
  guard
- `Reaper` holds a `&HeldTreeLock` and has no `paths` field. `lock_of`
  short-circuits on full `Artifact` equality before probing, and the taken
  guard lives for all of that owner's removals
- `referenced_generations` is computed before any removal. No reap path
  can create or remove a `.tmp-*.lock`
- The fetch API splits into `get_whole` and `resume`. A 416 is
  `AlreadyComplete` only when the `Content-Range` total equals `from`. A
  sink that cannot be opened is terminal
- The step-5 digest discard is an `inspect_err` on that one
  `verify_archive_file` call, so step 2's `UnexpectedDigest` cannot
  trigger it
- The precheck asks for `uncompressed_size + partial.additional_bytes()`
- `0218-bound-cache-root-growth.md` has its Dependencies line and the
  rewritten Assumptions entry

#### Deviations from Plan:

- `prune`'s docstring (`resolver.rs:742-746`) says another artifact's
  residue goes "only under that artifact's tree lock, so a concurrent
  materialisation of it is never disturbed". It leaves out the `Absent`
  arm, and "never disturbed" contradicts the window the module doc
  records. `Reaper::reap_orphans`'s docstring does name the `Absent` arm.
  Item 6 of Phase 3 asked for a matching correction here.
- The reap module doc spreads item 6's points over three paragraphs, not
  one, and drops "in the session" after the marker TTL. The content is
  otherwise complete.
- The AC6′ fixture does not plant the A and Z bystander orphans, so it is
  not "the same fixture" as AC6. This does not weaken the warning
  assertion.
- `temp_entry_names_are_fixed_across_launcher_versions` writes the lock
  name as a plain literal, not with `.concat()`. The `rg` check still
  holds.

#### Potential Issues:

- Symlink at the partial path (`download.rs:72-85`, `:232-236`):
  `AssessedPartial::of` treats a symlink as `OverLong`, as the docstring
  says, "replaced rather than resumed". But the `Truncate` sink opens it
  with `create` + `truncate`, which follows the link. The archive is
  therefore written through to the link's target, and a dangling link
  creates its target. It is replaced only if the open fails. This needs
  an attacker or an operator to plant a link inside `trees/`, which the
  launcher owns, but it contradicts the stated intent. Removing the
  non-regular entry before `fetch_whole` would close it.
- Untested recovery path: no test covers `fetch_whole`'s own `Sink`
  path, where it discards, retries and returns `CacheRootUnwritable` on
  a second failure. No test plants a directory at the archive path
  either. The only `Sink` test covers `resume_from`, through
  `an_unwritable_short_partial_is_replaced_in_the_same_run`.
- Precheck for a non-regular `OverLong`: `additional_bytes` returns 0
  for `OverLong`. That is right for a long file, which truncation frees,
  but for a directory or symlink nothing is freed, so the precheck
  undercounts by `archive_size`.
- Deadline per call: each `streaming_attempts` call starts its own
  `total_deadline`. A slow failed `resume` that ends in `RangeRejected`
  is followed by a fresh 900s `fetch_whole`, so the bound covers each
  call, not all of `stream_archive`.
- A partial that keeps failing to read: `digest_partial` keeps the file
  on any error other than `PermissionDenied` or `NotFound`. A persistent
  `EIO` or `ELOOP` therefore fails every run with a sticky `Extraction`.
  The plan chose this so that a transient fault never costs a valid
  download.
- Comment policy, in tests and pre-existing code:
  - `tests/tree_resolution.rs:44-45` describes the removed keep set, so
    it will go stale.
  - The comment in `the_single_flight_lock_is_exclusive`
    (`lease.rs:323`) describes the code and is inaccurate, since
    `probe_shared_lease` reports `Held` rather than blocking.
  - `reap.rs:532-533` reads "and if" where "or if" is meant.
- Stale `dead_code` allows remain on `READ_IDLE_TIMEOUT`, `STREAM_CHUNK`,
  `StreamLimits` and `StreamSink` in `fetcher.rs`, all of which are now
  used. They predate this plan, which did not ask for their removal.

### Manual Testing Required:

The plan records both of these as done:

1. Over-correction guards:
   - [x] Each of the four mutations in Phase 3 item 4 fails its named
     guard, then is reverted
2. Real cache root:
   - [x] `cache prune` reports reclaimed entries and leaves the `browser`
     and `driver` lock files and the pinned partial in place

Neither was repeated during this validation.

### Recommendations:

- Correct `prune`'s docstring so it names the `Absent` arm and drops
  "never disturbed"
- Remove a non-regular entry at the partial path before `fetch_whole`
  opens it. Add a test with a directory and a symlink at that path,
  covering `fetch_whole`'s discard-and-retry and its
  `CacheRootUnwritable` outcome
- Count `archive_size` in the precheck when the `OverLong` entry is not
  a regular file
- Delete or fix the three comments listed above, and drop the stale
  `dead_code` allows in `fetcher.rs`
