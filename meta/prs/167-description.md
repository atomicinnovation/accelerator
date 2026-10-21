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
revision: "513ba65feb57080b22b8277fb02631367c9bbc30"
repository: "accelerator"
last_updated: "2026-10-21T14:06:05+00:00"
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
creating the lock file. It never removes a partial archive that a launcher
could still resume, and makes sure such a partial can always be recovered
rather than leaving materialise stuck.

## Changes

### Reap (`tree/reap.rs`, `tree/tree_lock.rs`, `tree/resolver.rs`)

- `Reaper` is built from a `HeldTreeLock`, so it knows its own artifact and
  root. `HeldTreeLock::take` / `try_take` replace the resolver's direct
  `take_single_flight` / `try_single_flight` calls.
- `.tmp-` entries are grouped by owner (`Artifact::owning`). For each other
  owner the reap probes that owner's tree lock with `probe_single_flight`:
  - held by someone else: the entries are kept;
  - taken: the lock is held across exactly that owner's removals;
  - no lock file: the temp tree and pointer are removed, every partial
    archive is kept, and no lock file is created;
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
- `TreePaths::temp_archive` and `single_flight_lock` take an `Artifact`. The
  string-pair forms are gone, and no `format!(".tmp-…")` remains in `src/`.
- `KNOWN_PLATFORMS` states its cross-version contract: the list only grows,
  and no alias may end in `-` plus another. A test guards the second rule.
- Single-flight locks confirm the inode after `flock`. They retry if the file
  they locked was replaced at its path, or if `open` or `flock` answers
  `ESTALE`. They keep both descriptors for the life of the guard.
- `probe_single_flight` is a new probe that neither blocks nor creates the
  file.
- `pins::artifact_names` lists the platforms a launcher publishes trees for.

### Partial archives that always recover (`tree/download.rs`, `fetcher.rs`)

- `AssessedPartial` classifies the file at the partial path once, and owns
  its discard:
  - absent: fetched whole;
  - short: resumed;
  - complete: verified locally with no fetch;
  - over-long: fetched whole again;
  - obstructed (a directory or symlink): removed, then fetched whole.
- Every arm that cannot use the partial falls back through one
  `start_afresh`: discard, then a single whole fetch.
- The archive sink opens with `O_NOFOLLOW`, so neither a truncating nor an
  appending open writes through a symlink.
- The fetch API splits into `get_whole` and `resume`. A 416 means
  `AlreadyComplete` only when the `Content-Range` total equals the resume
  offset. Otherwise it is `RangeRejected`, which is final rather than
  retryable, and falls back to a whole fetch.
- One `Deadline` bounds the whole download. `StreamLimits::for_archive`
  starts it, and every fetch a download chains (resume, range rejected,
  whole, sink retry) shares it, so fallbacks no longer each get 900s.
- A ranged body is bounded by the shortfall, `max_bytes - from`, rather than
  the whole archive size.
- A partial whose digest does not match its pin is discarded, so the next run
  starts afresh. A partial that cannot be read or written is replaced in the
  same run. `repair --force` discards any partial first and never resumes.
- The free-space precheck counts only the bytes still to be downloaded, plus
  the extracted copy. An obstructed path counts as the whole archive.

### Test harness

- Both bounded-run helpers (`within`) forward the caller's tracing
  subscriber, so warnings raised in the worker thread reach test assertions.
- The lock-replacement test waits on a channel rather than a sleep, so it
  always exercises the displacement.
- Outcome-table tests that restated private match arms are dropped. Only the
  cases the public locks cannot reach remain.

### Supply-chain advisories

These were published after the branch was cut, and `main` carries the same
versions.

- `rustls-webpki` 0.103.15 → 0.103.16 and `webpki-roots` 1.0.9 → 1.0.10
  clear RUSTSEC-2026-0382 and RUSTSEC-2026-0381 (distrusted emSign
  intermediates). The third-party notices are regenerated to match.
- `npm audit fix` in `docs-site/` clears GHSA-587h-7wmj-h5pm
  (`launch-editor` via `vite`, high).

### Documents

- Adds work item 0339, its review, research, plan, plan review and validation.
- Adds the eight-lens PR review, `meta/reviews/prs/167-review-1.md`.
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
  (pass)
- Review: `meta/reviews/prs/167-review-1.md` (COMMENT, no blocking defect).
  The commits after it act on its three themes:
  - resume budgets now accumulate across the download;
  - the no-lock-file arm keeps partial archives;
  - the partial-archive lifecycle sits in `AssessedPartial`.
- Decision: ADR-0072
- Blocks 0218; relates to 0222, 0334 and 0214.

## Testing

- [x] `mise run cli:check` exits 0 at the PR tip.
- [x] `cargo test -p accelerator` (the launcher) passes at the PR tip: 606
      passed, 0 failed. `minisign` is on `PATH`, so the `tree_resolution`
      integration tests ran rather than being skipped.
- [x] `mise run deny:check`, `notices:check`, `docs:audit:check` and
      `docs:check` exit 0 after the dependency bumps.
- [x] Each acceptance criterion in 0339 has a named test in `reap.rs` or
      `tests/tree_resolution.rs`. Every reap runs with Y's tree lock held and
      under a time bound, so a blocking probe fails rather than hangs.
- [x] Review follow-ups have their own tests:
  - a deadline spent by one fetch bounds the next;
  - a ranged body longer than the shortfall is `TooLarge`;
  - a 206 without `Content-Range`, and an accepted 206;
  - `ESTALE` on open or lock is retried;
  - neither sink opens through a symlink;
  - a forced repair makes no range request.
- [x] Manual check against a real cache root: `cache prune` reclaims entries
      and leaves the `browser` and `driver` lock files and the pinned partial
      in place. This is recorded in the plan and was not repeated.
- [ ] `mise run` (bare default task) at the PR tip. It last exited 0 at
      `20dd4d85` during validation. CI covers the same lanes.
- [ ] Over-correction guards: the plan's four mutations have not been
      replayed since the review follow-ups.

## Notes for Reviewers

- ⚠️ Mixed-version roots: a launcher from before 0339 still reaps without
  checking locks until it is replaced. Such a launcher also gets a 416 when it
  resumes a complete pinned partial, spends its retry deadline on it, and
  records a sticky `Unreachable` marker. Today a reap for a different artifact
  clears that partial by accident; 0339 launchers no longer do. A 0339
  launcher materialising that artifact clears it reliably. The review
  suggested reclaiming complete pinned partials in the taken-lock arm; this
  PR does not.
- ⚠️ `cache prune` no longer removes `.tmp-` entries with no known owner.
  Clearing those is the job of 0218's root prune. Every shipped launcher uses
  today's `.tmp-` names, so no older residue becomes unowned.
- Accepted window: when another artifact has no lock file, its temp tree and
  pointer are removed with no lock held. A launcher that creates the lock
  between the scan and the removal never reopens either, and partial
  archives, the one entry it does reopen, are kept.
- A partial that keeps failing reads with something other than
  `PermissionDenied` or `NotFound` (for example `EIO`) is still kept, so a
  transient fault never costs a valid download. `repair --force` now clears
  it.
- Still open from the review:
  - `prune` does not run `ensure_trees_dir`'s ownership check before opening
    other artifacts' lock files (this predates the PR);
  - resumed range requests carry no `If-Range` validator. Every outcome is
    still re-hashed against the pin.
- `.accelerator/state/integrations/linear/last-sync.json` changes only because
  of a Linear sync.
