---
type: "pr-description"
id: "167"
title: "[0339] Reap another artifact's temp residue only under its tree lock"
date: "2026-10-17T21:44:26+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0339"
parent: "work-item:0339"
relates_to: ["work-item:0218", "work-item:0222", "work-item:0334", "adr:ADR-0072"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/167"
pr_number: 167
tags: ["cli", "launcher", "cache", "tree", "concurrency"]
revision: "d54d3b730456f02ca3d7b06b4796ddd041c8415b"
repository: "accelerator"
last_updated: "2026-10-17T21:44:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0339] Reap another artifact's temp residue only under its tree lock

## Summary

`reap_orphans` ran under one artifact's tree lock but removed the `.tmp-`
entries of every artifact under `trees/`. A reap for Y could therefore delete
X's half-extracted tree, its `.ref` sidecar or its partial download while X's
materialise was still running. This PR gives each `.tmp-` entry an owning
artifact. The reap then removes another artifact's entries only while it
holds that artifact's tree lock, which it takes without blocking and without
creating the lock file. It also spares every partial archive whose digest is
pinned for its own artifact, and makes sure such a partial can always be
recovered rather than leaving materialise stuck.

## Changes

### Reap (`tree/reap.rs`, `tree/tree_lock.rs`, `tree/resolver.rs`)

- `Reaper` is built from a `HeldTreeLock`, so it knows its own artifact and
  root. `HeldTreeLock::take` / `try_take` replace the resolver's direct
  `take_single_flight` / `try_single_flight` calls.
- `.tmp-` entries are grouped by owner (`Artifact::owning`). For each other
  owner the reap probes that owner's tree lock with `probe_single_flight`:
  - held by someone else: the entries are kept;
  - taken: the lock is held across all of that owner's removals;
  - no lock file: the entries are removed, and no lock file is created;
  - probe error: the entries are kept.
- The reap's own artifact is handled as before, without probing its own lock.
- The single-digest `keep` set is replaced by a `PinnedDigests` lookup. A
  partial archive survives when its digest is the one pinned for its own
  artifact, whichever artifact the reap is for.
- Entries that parse to no known platform are kept. They may belong to a newer
  launcher.
- No reap path can create or remove a `.tmp-*.lock` file.
- A `RemovalObserver` test seam lets tests check which locks are held at each
  removal.

### Naming and locking (`tree/layout.rs`, `tree/lease.rs`, `tree/pins.rs`)

- `Artifact` is a new `(name, platform)` value. `Artifact::owning` splits a
  `.tmp-` name at the first known platform followed by a digest, so
  `linux-x64-musl` entries and lock files have no owner.
- `TreePaths` gains `temp_pointer`, `temp_archive_of` and
  `single_flight_lock_of`. No `format!(".tmp-…")` remains in `src/`.
- Single-flight locks confirm the inode after `flock`. They retry if the file
  they locked was replaced at its path, and they keep both descriptors for the
  life of the guard.
- `probe_single_flight` is a new probe that neither blocks nor creates the
  file.
- `pins::artifact_names` lists the platforms a launcher publishes trees for.

### Partial archives that always recover (`tree/download.rs`, `fetcher.rs`)

- `AssessedPartial` classifies the file at the partial path once:
  - absent: fetched whole;
  - short: resumed;
  - complete: verified locally with no fetch;
  - over-long: fetched whole again;
  - obstructed (a directory or symlink): removed, then fetched whole, never
    written through.
- The fetch API splits into `get_whole` and `resume`. A 416 means
  `AlreadyComplete` only when the `Content-Range` total equals the resume
  offset. Otherwise it is `RangeRejected`, which is final rather than
  retryable, and falls back to a whole fetch.
- A partial whose digest does not match its pin is discarded, so the next run
  starts afresh. A partial that cannot be read or written is replaced in the
  same run.
- The free-space precheck counts only the bytes still to be downloaded, plus
  the extracted copy. An obstructed path counts as the whole archive.
- Stale `dead_code` allows in `fetcher.rs` are dropped.

### Documents

- Adds work item 0339, its review, research, plan, plan review and validation.
- Adds ADR-0072, which covers the claim-gated automatic prune for relocated
  cache roots. It is the decision whose non-blocking lock rule this fix
  applies to the tree reap.
- Reviews and widens 0218. Records that 0218 is blocked by 0339 and that
  launchers older than 0339 are not safe against its orphan-lock removal.
- 0222 records its obligation to hold the tree lock from download through to
  its reap.
- `tasks/lint/store_duplication.py` allowlists `lease.rs`, whose tests rename
  a lock file under its holder.

## Context

- Work item: `meta/work/0339-tree-reap-deletes-another-artifact-s-in-progress-materialise.md`
  (Linear PP-922)
- Plan: `meta/plans/2026-10-16-0339-tree-reap-cross-artifact-lock-probe.md`
- Validation: `meta/validations/2026-10-16-0339-tree-reap-cross-artifact-lock-probe-validation.md`
  (pass). The last five code commits act on its recommendations:
  - `prune`'s docstring now names the no-lock-file arm;
  - a non-regular entry at the partial path is removed before the whole
    fetch;
  - the precheck counts the whole archive for an obstructed path;
  - stale comments and `dead_code` allows are gone.
- Decision: ADR-0072
- Blocks 0218; relates to 0222, 0334 and 0214.

## Testing

- [x] `mise run cli:check` exits 0 at the PR tip
- [x] `cargo test -p accelerator` (the launcher) passes at the PR tip: 603
      passed, 0 failed. `minisign` is on `PATH`, so the `tree_resolution`
      integration tests ran rather than being skipped.
- [x] `mise run` (bare default task) exited 0 end-to-end at `20dd4d85`
      during validation. It has not been re-run since the follow-up commits.
- [x] Each acceptance criterion in 0339 has a named test in `reap.rs` or
      `tests/tree_resolution.rs`. Every reap runs with Y's tree lock held and
      under a time bound, so a blocking probe fails rather than hangs.
- [x] Over-correction guards: the plan records four mutations, each failing
      its named guard before being reverted. These were not re-run during
      validation.
- [x] Manual check against a real cache root: `cache prune` reclaims entries
      and leaves the `browser` and `driver` lock files and the pinned partial
      in place. This is recorded in the plan and was not repeated.
- [ ] `mise run` at the PR tip. CI covers the same lanes.

## Notes for Reviewers

- ⚠️ Mixed-version roots: a launcher from before 0339 still reaps without
  checking locks until it is replaced. Such a launcher also gets a 416 when it
  resumes a complete pinned partial, spends its retry deadline on it, and
  records a sticky `Unreachable` marker. Today a reap for a different artifact
  clears that partial by accident; 0339 launchers no longer do. A 0339
  launcher materialising that artifact clears it reliably. See the plan's
  Migration Notes.
- ⚠️ `cache prune` no longer removes `.tmp-` entries with no known owner.
  Clearing those is the job of 0218's root prune. Every shipped launcher uses
  today's `.tmp-` names, so no older residue becomes unowned.
- Accepted window: when another artifact has no lock file, its entries are
  removed with no lock held. A launcher of the same artifact that starts
  between the scan and the removal can lose a partial it was about to resume.
  The reap module doc records this.
- Still open from validation:
  - each `streaming_attempts` call has its own 900s deadline, so a slow
    `resume` that ends in `RangeRejected` is followed by a fresh full
    deadline;
  - a partial that keeps failing reads with something other than
    `PermissionDenied` or `NotFound` (for example `EIO`) is kept, and fails
    each run with `Extraction`. This is deliberate, so that a transient fault
    never costs a valid download.
- `.accelerator/state/integrations/linear/last-sync.json` changes only because
  of a Linear sync.
