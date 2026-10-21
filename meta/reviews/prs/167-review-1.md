---
type: "pr-review"
id: "167-review-1"
title: "[0339] Reap another artifact's temp residue only under its tree lock"
date: "2026-10-18T08:02:01+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:167"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["correctness", "safety", "test-coverage", "code-quality", "architecture", "compatibility", "portability", "security"]
review_number: 1
pr_number: 167
tags: []
last_updated: "2026-10-18T08:02:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #167 - [0339] Reap another artifact's temp residue only under its tree lock

**Verdict:** COMMENT

This PR fixes a real cross-artifact data-loss race, and the fix fails safe. Ownership is a domain value (`Artifact::owning`), and "whose reap is this" is a type (`HeldTreeLock`). Probes neither block nor create a lock file, and every uncertain lock state keeps the residue. Tests check which locks are held at the moment of each removal, under a time bound. No lens found a blocking defect.

The findings cluster in three places:

- **Resume budgets**: the deadline and the byte bound both reset for each fetch call or response, instead of covering the whole download.
- **No-lock-file arm**: removing another artifact's residue when it has no lock file rests on cross-version and NFS assumptions that are documented but not enforced.
- **Mixed-version roots**: sparing every pinned partial costs launchers from before 0339 the accidental cleanup that rescued them before.

### Cross-Cutting Themes

- **Resume budgets reset instead of accumulating** (flagged by: architecture, correctness, security) — `StreamLimits::total_deadline` is enforced inside each `streaming_attempts` call. `Download` chains `resume` → `RangeRejected` → `get_whole` → `Sink` → `get_whole`, so one materialise can wait 2–3× 900s. Bytes have the same shape: a 206 body is bounded by `archive_size`, not by `archive_size - held`, so the file on disk can reach almost twice what the free-space precheck reserved. Fix both at the download level: one `Deadline` and one remaining-bytes bound that every call shares.
- **The no-lock-file arm rests on invariants nothing enforces** (flagged by: safety, portability, compatibility, architecture) — `OwnerLock::Absent` removes residue with no lock held. That is safe only if four things hold:
  - lock files are unlinked only by their holder;
  - every launcher confirms the inode after `flock`, which launchers from before 0339 do not;
  - every launcher parses `.tmp-` names the same way;
  - NFS does not answer from a stale negative-lookup cache.

  Keeping partial archives in the `Absent` arm closes the worst window, a reopened partial that ends in a sticky `Extraction` refusal. A test that fails if any production path removes a `.tmp-*.lock` would turn the convention into a guard.
- **"Never probe a lock you already hold" is enforced only by `owner == self.held.artifact()`** (flagged by: architecture, portability) — this is correct while the launcher holds one tree lock and runs one thread. Where `flock` is emulated with POSIX locks, closing the probe's descriptor silently drops the caller's own lock. The generation-lease probe has the same exposure.
- **Partial-archive lifecycle is spread across three modules** (flagged by: architecture, code-quality, safety) — discard rules live in `download.rs` (obstructed, unwritable, `PermissionDenied`) and `resolver.rs` (`UnexpectedDigest`), and the spare rule lives in `reap.rs`. `fetch_whole`, `resume_from` and `digest_partial` call each other, and a partial that keeps failing reads with `EIO` cannot be cleared by any command.
- **`KNOWN_PLATFORMS` is now a cross-version contract** (flagged by: architecture, compatibility, portability) — it must only ever grow, aliases must not end in an existing platform, and a musl build still reports `linux-x64`. None of these rules is stated next to the list.

### Tradeoff Analysis

- **Safety vs compatibility (pinned partials)**:
  - Safety wants the reap to keep more partials, including in the `Absent` arm, so a 0339 launcher never loses progress it is about to resume.
  - Compatibility wants complete pinned partials reclaimed, so launchers from before 0339 stop looping on 416 and recording a sticky `Unreachable`.
  - Recommendation: when the reap has taken another artifact's lock, remove that artifact's complete pinned partial; under `Absent`, keep every partial. This reconciles both, because the `Taken` arm is the only one where removal cannot race.
- **Conservative `EIO` handling vs recoverability**: keeping a partial that fails reads with an unexpected error protects a valid download from a transient fault. But neither `cache prune` nor `repair --force` can clear it. Keep the default, and let `repair` discard the partial explicitly.
- **Test pinning vs behavioural tests**: the private outcome-table tests (`take_outcome`/`try_outcome`/`probe_outcome`, `temp_residue_is_reclaimable`) make the mapping tables readable and fast to check, but they couple the suite to private enums. The project's "test behaviour" rule argues for keeping only the `KeptBeingReplaced` cases that the public API cannot reach.

### Strengths

- ✅ `Reaper` can only be built from a `HeldTreeLock`, so a reap run without its own lock, or under the wrong root, cannot be written.
- ✅ `Artifact` and `TreePaths` own every `.tmp-` name, with no `format!(".tmp-…")` left in `src/`. `temp_entry_names_are_fixed_across_launcher_versions` pins the cross-version grammar.
- ✅ Every uncertain outcome keeps the data:
  - a lock held elsewhere;
  - a probe error;
  - a lock file that keeps being replaced;
  - an entry on an unknown platform, such as `linux-x64-musl`.
- ✅ `lock_confirmed` compares `fstat` of the locked descriptor with `fstat` of a freshly reopened one (the overlayfs and FUSE case), treats `ESTALE` as displacement, caps its retries, and keeps both descriptors open for the life of the guard.
- ✅ 416 and 206 handling fails closed: `AlreadyComplete` only when the `Content-Range` total equals the resume offset. Integrity never depends on range headers, because every outcome is re-hashed and verified against the pin and the minisign signature.
- ✅ A symlink or directory at the partial path is classed as `Obstructed` via `symlink_metadata` and removed, never written through.
- ✅ The `ProbingLockOf` observer checks which lock is held at the moment of each removal. The `aardvark` and `zebra` bystander orphans catch a reap that stops early at a held owner.
- ✅ Every reap test runs under `within(BLOCKING_BOUND, …)` and asserts the set of lock files is unchanged, so a blocking regression fails instead of hanging CI.
- ✅ Cross-artifact probes only try-lock, so two reaps for different artifacts cannot deadlock.

### General Findings

- 🔵 **Architecture**: the reap's safety now depends on a protocol that every launcher sharing a root must follow: holder-only unlink, inode confirmation, and a shared `.tmp-` grammar. Nothing records a protocol version that 0218's orphan-lock removal could check before running on a root where launchers from before 0339 may still be active.
- 🔵 **Compatibility**: the "no launcher unlinks a lock it does not hold" invariant deserves the same treatment as the `.tmp-` names: a statement in `layout.rs`, plus a test that fails if production code removes a `.tmp-*.lock`.
- 🔵 **Security**: `prune`, and the reap with it, never runs `ensure_trees_dir`'s ownership check (owned by the euid, mode 0700, not a symlink). This predates the PR, but the reap now opens other artifacts' lock files read-write in that directory.
- 🔵 **Portability**: the "never probe a lock you hold" hazard also covers `probe_shared_lease`. State the rule once, beside ADR-0072's `flock` assumption.
- 🔵 **Code Quality**: the two copies of `within`, in `tree/testing.rs` and `tests/common/mod.rs`, have already diverged: only one forwards the tracing dispatcher. `testing.rs` also carries a redundant `#[allow(clippy::panic)]` and a nested `#[cfg(test)]`.
- 🔵 **Code Quality**: `download.rs:109`, a 126-column doc line, breaks the 80-column limit. rustfmt does not wrap comments.
- 🔵 **Test Coverage**: replay the plan's four over-correction mutations at the PR tip. Several test refactors landed after they were last run.

### Additional Findings

- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/layout.rs:77` — A future platform alias that ends in a known platform is misattributed instead of left unowned (compatibility)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/download.rs:196` — `digest_partial`'s `NotFound` fallback to a whole fetch is never exercised (test-coverage)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:293` — The partial-archive lifecycle is spread across download, resolver and reap (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/download.rs:170` — Partial-archive recovery is spread across mutually recursive methods (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/layout.rs:227` — Two parallel path APIs: string-pair and `Artifact`-keyed (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:543` — A sleep-based test can pass without exercising the displacement (test-coverage)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:9` — The no-lock-file arm relies on NFS lookup-cache coherence, which the doc does not mention (portability)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:169` — "Never probe a lock you hold" is enforced only by comparing with one held lock (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/fetcher.rs:344` — No fetcher-level test for an accepted 206, or a 206 without `Content-Range` (test-coverage)
- 🔵 `cli/launcher/src/launch/outbound/resolve/fetcher.rs:344` — `resume` checks the 206 status three times; use the named `StatusCode` constants (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/fetcher.rs:409` — Resumed range requests carry no `If-Range` validator (compatibility)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:558` — Outcome-mapping tests mirror private match arms (test-coverage)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:227` — `Locked` allows outcomes that the creating opener and a blocking `flock` cannot produce (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:645` — Lock tests assume per-open-file-description `flock`, so they fail when `TMPDIR` is on NFS (portability)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:1079` — The `temp_residue_is_reclaimable` tests duplicate the behavioural reap tests (test-coverage)
- 🔵 `cli/launcher/src/launch/outbound/resolve/mod.rs:265` — A streaming-only `FetchError::Sink` now reaches the single-file fetch path (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/mod.rs:28` — A musl-target launcher build would still report itself as `linux-x64` (portability)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/layout.rs:19` — Ownership parsing depends on a third hand-maintained platform list (architecture)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/launcher/src/launch/outbound/resolve/tree/download.rs:181-185` — The whole-download deadline is enforced per fetch call, but `Download` chains several calls
**Severity**: major | **Confidence**: high | **Lens**: architecture

🟡 **Architecture**

`StreamLimits::total_deadline` exists so that retries "cannot compound into a half-hour wait". But it is enforced inside each `Fetcher::streaming_attempts` call, and `Download` now chains several calls per materialise:

- `resume_from` can end in `RangeRejected` and then call `fetch_whole`, which starts a fresh 900s budget.
- `fetch_whole` can hit a `Sink` error, discard the partial and call `get_whole` again, with yet another budget.

The bound now sits below the level where the multi-step strategy is decided.

**Impact**: one materialise can wait two to three times the stated limit, with no progress output and no way to cancel. Each new fallback branch adds another full budget. The PR lists the `RangeRejected` case as still open, but the cause is structural.

**Suggestion**: lift the bound to the download level. Create one `Deadline` in `Download`/`stream_archive` and pass it to `get_whole`/`resume`, so that `streaming_attempts` checks the budget that remains instead of starting a new one.

---

### `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:255-263` — Sparing every artifact's complete pinned partial strands launchers from before 0339 on a shared root
**Severity**: major | **Confidence**: medium | **Lens**: compatibility

🟡 **Compatibility**

`is_resumable` now spares every partial archive whose digest matches its own artifact's pin, including one that is already complete (size equal to `archive_size`). A complete pinned partial is left behind whenever a later materialise step fails, such as the attestation fetch.

A launcher from before 0339 that pins the same digest then:

1. resumes the partial and gets a 416;
2. treats the 416 as retryable and uses up its whole retry deadline;
3. records a sticky `Unreachable` marker for the marker TTL.

Today a reap for any other artifact deletes that file by accident. After this PR, a 0339 reap never does.

**Impact**: on a cache root shared across launcher versions, which ADR-0072 assumes, an older launcher can now fail the same fetch repeatedly where it used to recover.

**Suggestion**: when the reap holds another artifact's lock (`OwnerLock::Taken`), reclaim that artifact's *complete* pinned partial. Keep short (resumable) ones. A 0339 launcher loses at most one local re-hash, and older launchers get their cleanup back. Otherwise, record the stranding in the release notes, not only in the plan's Migration Notes.

---

### `cli/launcher/src/launch/outbound/resolve/fetcher.rs:353-361` — A resumed body is bounded by the whole archive size, not the remaining shortfall
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, security

🔵 **Correctness**

On a 206, `copy_body` counts only the appended bytes against `limits.max_bytes`. That limit is `StreamLimits::for_archive(archive_size)` (`download.rs:128`). A server that sends more than the shortfall can therefore append up to `archive_size` bytes on top of the `held` bytes already on disk. This breaks the invariant that "the archive size bounds the body".

🔵 **Security**

The free-space precheck now reserves only `archive_size - held`. A misbehaving or compromised CDN can therefore push disk use for one materialise to almost twice what was reserved, on a volume that may be nearly full. Integrity is not at risk, because the digest check still rejects the file.

**Suggestion**: on the 206 branch, bound the body by what remains. Pass `max_bytes: limits.max_bytes.saturating_sub(from)` to `copy_body` when `ranged`. Optionally, also treat a 206 whose `Content-Range` total differs from `archive_size` as `RangeRejected`.

---

### `cli/launcher/tests/tree_resolution.rs:1870` — Retry-from-agreed-offset test only asserts success, not that it resumed
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`a_resumed_download_that_fails_mid_body_retries_from_the_agreed_offset` only asserts that materialise succeeds. It catches a missing `set_len(from)`, because the archive would then fail its digest. But a regression that falls back to a whole fetch after the mid-body failure would also succeed.

**Impact**: a change that quietly stops resuming after a transient failure would pass, wasting the bytes already downloaded. Resume exists to save those bytes on slow links.

**Suggestion**: also assert the request shape, e.g. `assert_eq!(harness.server.range_hits(&archive_path()), 2)` and `assert_eq!(harness.hits(&archive_path()), 2)`.

---

### `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:14-19` — Reap module doc describes effects in other modules and has a 114-column line
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

The module doc mixes real invariants with descriptions of distant behaviour.

- **Invariants**: a lock file is unlinked only by its holder; a process never re-probes its own lock.
- **Distant behaviour**: the `Extraction` cause "is sticky for the marker TTL", and "as a Refusal it stops a `--fail-safe` crawl".

Line 18 is also 114 columns wide, and `download.rs:109` is 126. rustfmt does not wrap comments, so nothing will catch either line.

**Impact**: the marker and crawl remarks describe behaviour owned elsewhere, and they will go stale silently.

**Suggestion**: keep the invariants and one sentence on the accepted window. Move the downstream effects to the ADR. Re-wrap both doc comments to 80 columns.

---

### `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:150-166` — The owner's lock is held across removals only because the `lock` binding lives to the end of the function
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

In `reap_temp_entries_of`, another artifact's tree lock is held across its removals only because the `OwnerLock::Taken { _guard }` value stays bound to `lock` until the function returns. Nothing marks that binding as load-bearing.

**Impact**: a natural refactor would drop the guard before the removals and reintroduce the exact deletion this PR fixes. Examples:

- inlining `temp_residue_is_reclaimable(&self.lock_of(owner))`;
- moving the loop to a helper that takes only a `bool`.

Only the observer-based test would catch it.

**Suggestion**: make the scope structural, e.g. `lock.while_reclaimable(|| …)`, which runs the removals while it holds the guard. Alternatively, name the binding for its role.

---

### `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:177` — No-lock-file arm still removes partial archives with no lock held
**Severity**: minor | **Confidence**: medium | **Lens**: safety

🔵 **Safety**

When another artifact has no lock file (`OwnerLock::Absent`), the reap removes all of that artifact's unpinned residue with no lock held, partial archives included. A launcher of another version can create the lock and reopen the partial between the scan and the removal. Its appends then go to an unlinked file, and materialise fails with a sticky `Extraction` refusal. That refusal stops a `--fail-safe` crawl.

Only partial archives are exposed. Temp trees and `.ref` pointers carry a per-run random suffix, so no later launcher reopens them. On NFS, a stale negative-lookup cache can also make a held lock look `Absent`, which widens the window.

**Suggestion**: in the `Absent` arm, remove temp trees and pointers but keep partial archives (`owner.partial_archive_digest(entry).is_some()`). Leave those to 0218's root prune.

---

### `cli/launcher/src/launch/outbound/resolve/tree/download.rs:199-202` — A pinned partial that keeps failing reads cannot be cleared by any command
**Severity**: minor | **Confidence**: medium | **Lens**: safety

🔵 **Safety**

A pinned partial that fails reads with anything other than `NotFound` or `PermissionDenied` (e.g. `EIO`) is deliberately kept, and every run fails with `Extraction`. Neither recovery command reaches it:

- `cache prune` spares every partial pinned for its own artifact.
- `repair --force` goes through `materialise_fresh` to this same `digest_partial` call and fails the same way.

**Impact**: the user is stuck in a refusal loop until they find and delete a dotfile under `trees/` by hand.

**Suggestion**: keep the conservative default on the automatic path, but have `repair` discard the pinned partial before it materialises. At minimum, name the file to delete in the `Extraction` detail.

---

### `cli/launcher/src/launch/outbound/resolve/tree/download.rs:234-256` — Symlink check happens at assessment, not when the partial archive is opened
**Severity**: minor | **Confidence**: medium | **Lens**: security

🔵 **Security**

`AssessedPartial::of` checks for a symlink once, via `symlink_metadata`. `open_archive_sink` later reopens the path with plain `OpenOptions` and no `O_NOFOLLOW`. If the path is swapped for a symlink in between, which spans a network round-trip:

- the `Append` arm calls `set_len(from)` on the link's target and appends to it;
- the `Truncate` arm empties the target.

**Impact**: only someone who can already write to `trees/` (which must be owned by the euid with mode 0700) can use this window. It is defence in depth, not a privilege boundary.

**Suggestion**: add `.custom_flags(libc::O_NOFOLLOW)` to both arms. In `Append`, also check `file.metadata()?.is_file()` before calling `set_len`. A swapped-in symlink then becomes a `Sink` error, which the existing discard-and-refetch path already handles.

---

### `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:198-218` — ESTALE is treated as displacement only in the identity check, not in the open and flock steps
**Severity**: minor | **Confidence**: medium | **Lens**: portability

🔵 **Portability**

`is_displacement` treats `ESTALE` as a replaced file, but only `placement` consults it. An `ESTALE` from `open` or from `flock` itself becomes a generic lease error instead. On a root shared by several NFS clients, both steps can return `ESTALE` when another client replaces the lock file.

**Impact**: `take_single_flight` and `try_single_flight` can fail a materialise or prune outright, in a race that the loop already retries when it surfaces one step later. The probe path fails safe; the take paths do not.

**Suggestion**: map `ESTALE` from `open` and `flock` to `Placement::Displaced` and loop again. Add a test next to `a_stale_handle_is_a_displacement`.

---

## Per-Lens Results

### Correctness

**Summary**: The cross-artifact reap logic holds up. Each temp entry gets one unambiguous owner, each other owner's lock is probed without blocking or creating a file, and a taken guard stays alive across all of that owner's removals. Two reaps cannot deadlock. Every branch of the partial-archive state machine recovers, and none recurses. There is one minor gap: the bound on a resumed body is not reduced by the bytes already held.

**Strengths**:
- `Artifact::owning` accepts a platform match only with a non-empty name before it and a 64-hex digest after it. That rejects lock files, nameless entries and `linux-x64-musl`.
- The `OwnerLock::Taken` guard lives across every removal for an owner and drops before the next owner is probed.
- `referenced_generations` is computed before any temp entry is removed.
- `open_archive_sink` in `Append` mode refuses to create or pad a file, and cuts it back to the agreed offset.
- `lock_confirmed` handles `ESTALE`/`ENOENT` as displacement, with a fixed retry bound.
- 416 counts as complete only when the totals match exactly, and a 206 starting at another offset is rejected.
- The free-space precheck matches the actual peak on every branch.

**Comments**:
- `fetcher.rs:353-361` (RIGHT) — minor, medium — A resumed body is bounded by the whole archive size, not the remaining shortfall. *(Merged inline with security.)*

**General Findings**: none.

### Safety

**Summary**: The PR fixes a real data-loss hazard and fails safe: residue is kept whenever ownership or lock state is uncertain. Probes never block, so two reaps cannot deadlock. Two gaps remain, both in recovery paths: the no-lock-file window, and a pinned partial that keeps failing reads and cannot be cleared by any command.

**Strengths**:
- Probes are non-blocking, and a reap holds at most its own lock plus one probed lock.
- Every uncertain outcome keeps the data, and a probe error is logged with the artifact and platform.
- `probe_single_flight` never creates or removes a lock file, and the tests assert the set of lock files is unchanged.
- Inode confirmation with a bounded retry closes the "two holders" window.
- A complete partial now recovers: it is hashed locally, and a digest mismatch discards it.
- A symlink or directory at the partial path is removed, and a failed discard is logged.
- Every reap test runs under a time bound.

**Comments**:
- `reap.rs:177` (RIGHT) — minor, medium — No-lock-file arm still removes partial archives with no lock held.
- `download.rs:199-202` (RIGHT) — minor, medium — A pinned partial that keeps failing reads cannot be cleared by any command.

**General Findings**: none.

### Test Coverage

**Summary**: Coverage is strong and in proportion to the risk. Every reap arm has a behavioural test that runs under the caller's lock with a time bound and a check that the lock-file set is unchanged. The integration tests cover materialise and prune end to end. The gaps are small: a few recovery branches are never exercised, one test is weaker than its name, some tests read private state, and one sleep-based test can pass without exercising its race.

**Strengths**:
- `reap_for_y_at` reaps under Y's lock with a 10s bound and asserts the set of lock files is unchanged.
- The `ProbingLockOf` observer checks that X's lock is held at each removal and released afterwards.
- The `aardvark` and `zebra` bystander orphans catch a reap that stops early at a held owner.
- `Artifact::owning` has edge-case tests, plus a sweep over `EXPECTED_TREE_DIGESTS`.
- `temp_entry_names_are_fixed_across_launcher_versions` pins the `.tmp-` grammar.
- Fetcher 416 tests cover every boundary and assert that no sink was opened.
- `the_free_space_check_counts_the_partial_already_on_disk` asserts the exact byte count.
- Permission-based tests skip when the process ignores file permissions (root, `CAP_DAC_OVERRIDE`), and `within` and `logged` have their own tests.

**Comments**:
- `tests/tree_resolution.rs:1870` (RIGHT) — minor, high — Retry-from-agreed-offset test only asserts success.
- `download.rs:196-198` (RIGHT) — minor, medium — `digest_partial`'s `NotFound` arm is never exercised.
- `fetcher.rs:344-348` (RIGHT) — suggestion, medium — No fetcher-level test for an accepted 206, or a 206 without `Content-Range`.
- `lease.rs:543` (RIGHT) — minor, medium — Sleep-based synchronisation can let the waiting-taker test pass without exercising the displacement.
- `lease.rs:558-627` (RIGHT) — suggestion, medium — Outcome-mapping tests mirror private match arms.
- `reap.rs:1079-1108` (RIGHT) — suggestion, medium — `temp_residue_is_reclaimable` tests duplicate the behavioural reap tests.

**General Findings**:
- suggestion — The mutation guards recorded in the plan were not re-run at the PR tip.

### Code Quality

**Summary**: The PR is well designed from a maintainability standpoint: an `Artifact` domain type, `HeldTreeLock` as proof that a lock is held, small named outcome mappers, and the `get_whole`/`resume` split with typed outcomes. The concerns are minor:

- overlong or stale-prone doc comments;
- a guard whose lifetime is implicit;
- mutually recursive recovery methods;
- parallel string-pair path builders.

**Strengths**:
- `Artifact` and `TreePaths` own every `.tmp-` name.
- `HeldTreeLock` makes "whose reap" a type-level fact.
- `PinnedDigests`, `RemovalObserver`, `Clock` and `LivenessProbe` are injected, so the reaper can be tested in isolation.
- One `lock_confirmed` loop with three small outcome mappers.
- `Terminal::retryable_detail` holds the retry decision in one place, and `FetchError::Sink` maps to `CacheRootUnwritable`.
- `AssessedPartial` is the single source of truth for the precheck and the download.
- `logged` and `within` are shared helpers.

**Comments**:
- `reap.rs:14-19` (RIGHT) — minor, high — The reap module doc describes effects in other modules and has an unwrapped line.
- `download.rs:108-110` (RIGHT) — minor, high — Unwrapped doc-comment line in `stream_archive`.
- `reap.rs:150-166` (RIGHT) — minor, medium — The owner's lock is held only because of the `lock` binding.
- `download.rs:170-211` (RIGHT) — minor, medium — Partial-archive recovery is spread across mutually recursive methods.
- `fetcher.rs:344-366` (RIGHT) — suggestion, high — `resume` checks the 206 status three times.
- `layout.rs:227-229` (RIGHT) — minor, medium — Two parallel path APIs.
- `lease.rs:227-236` (RIGHT) — suggestion, low — `Locked` allows outcomes that cannot occur.

**General Findings**:
- suggestion — The two copies of `within` have diverged, and `testing.rs` carries redundant attributes.

### Architecture

**Summary**: The PR improves the subsystem structurally: `HeldTreeLock` as a typed precondition, centralised naming, and explicit `get_whole`/`resume` with a closed `Resumed` type. The main weakness is the whole-download deadline, which each fetch call enforces while `Download` chains several of them. A smaller one is that the partial-archive lifecycle is spread across three modules.

**Strengths**:
- `Reaper` can only be built from a `HeldTreeLock`.
- `Artifact` and `TreePaths` centralise naming, and the cross-version contract is pinned by a test.
- `lock_confirmed` is a small functional core with pure outcome mappers.
- `PartialArchive::assess` and `additional_bytes` are pure, and the filesystem is read once.
- `streaming_attempts` and `Terminal::retryable_detail` remove duplicated retry logic.
- Injected traits follow existing seams such as `StepObserver`.

**Comments**:
- `download.rs:181-185` (RIGHT) — major, high — The whole-download deadline is enforced per fetch call.
- `resolver.rs:293-298` (RIGHT) — minor, medium — The partial-archive lifecycle is spread across download, resolver and reap.
- `reap.rs:169-172` (RIGHT) — suggestion, low — "Never probe a lock you already hold" is enforced only by comparing with one held lock.
- `mod.rs:265-270` (RIGHT) — suggestion, medium — A streaming-only error variant reaches the single-file fetch path.
- `layout.rs:19` (RIGHT) — suggestion, medium — Ownership parsing depends on a third hand-maintained platform list.

**General Findings**:
- minor — Reap safety depends on every launcher on a shared root following the same protocol, and no protocol version records it.

### Compatibility

**Summary**: The PR is careful about compatibility. The `.tmp-` names are byte-for-byte unchanged and now pinned by a test. Probing and inode confirmation work alongside older launchers, and the Range/416/206 handling follows RFC 9110. The main cost falls on launchers from before 0339 that share a root: they lose the accidental cleanup of complete pinned partials. A platform alias that ends in a known platform can also be misattributed.

**Strengths**:
- The `.tmp-` grammar is unchanged and pinned, and `LAYOUT_VERSION` is documented as not covering it.
- Unknown-platform entries are kept.
- `probe_single_flight` treats `EACCES` or a directory at the lock path as unprobeable, and keeps the entries.
- Inode confirmation allows for overlayfs, FUSE, NFS and POSIX-emulated `flock`.
- 416 handling follows RFC 9110 §15.5.17.
- A complete partial is verified locally, without a request.
- `KNOWN_PLATFORMS` is tested against the pins and the host platform.

**Comments**:
- `reap.rs:255-263` (RIGHT) — major, medium — Sparing every complete pinned partial strands launchers from before 0339.
- `layout.rs:77-86` (RIGHT) — minor, medium — A future alias ending in a known platform is misattributed.
- `fetcher.rs:409` (RIGHT) — suggestion, low — Resumed range requests carry no `If-Range` validator.

**General Findings**:
- minor — Cross-version reap safety depends on no launcher ever unlinking a lock file it does not hold, and only convention enforces that.

### Portability

**Summary**: The launcher changes are careful about portability:

- an `fstat`-versus-`fstat` identity check;
- the confirming descriptor kept open for POSIX-emulated `flock`;
- `ESTALE` treated as displacement;
- a digest-anchored platform parse.

What remains concerns multi-client NFS: `ESTALE` from the `open` and `flock` steps, negative-lookup caching in the no-lock-file arm, and lock tests that assume local-filesystem `flock` semantics.

**Strengths**:
- `placement` compares `fstat` with `fstat`, for overlayfs and FUSE.
- `SingleFlight` keeps both descriptors open.
- `is_displacement` treats `ESTALE` as `NotFound`.
- `Artifact::owning` requires a digest after the platform, so the result does not depend on list order.
- `probe_single_flight` never creates a file.
- Permission tests skip when the process ignores file permissions (root, `CAP_DAC_OVERRIDE`).
- The `#[cfg(unix)]` split is preserved.
- `KNOWN_PLATFORMS` is tied to the pins and to `HOST_PLATFORM`.

**Comments**:
- `lease.rs:198-218` (RIGHT) — minor, medium — `ESTALE` is treated as displacement only in the identity check.
- `reap.rs:9-19` (RIGHT) — minor, low — The no-lock-file arm relies on NFS lookup-cache coherence.
- `lease.rs:645` (RIGHT) — suggestion, medium — Lock tests assume per-open-file-description `flock`.
- `mod.rs:28-34` (RIGHT) — suggestion, low — A musl-target build still reports `linux-x64`.

**General Findings**:
- suggestion — The "never probe a lock you already hold" rule also applies to shared leases, and should be stated once.

### Security

**Summary**: The PR improves the partial-download path's security:

- an obstructed path is removed, never followed;
- every outcome is re-hashed against the pin and the signature;
- range headers are not trusted unless they match the resume offset.

The trust boundary (`trees/` owned by the euid, mode 0700) is unchanged, so both findings are defence in depth.

**Strengths**:
- `AssessedPartial::of` uses `symlink_metadata`, and the discard removes the link, never its target.
- Integrity does not depend on range headers.
- `Content-Range` handling fails closed, and `open_archive_sink` never pads a file.
- Owner names cannot span path components, so there is no path traversal.
- `probe_single_flight` fails safe on any error.
- A sink that cannot be opened is final, not retried.

**Comments**:
- `download.rs:234-256` (RIGHT) — minor, medium — The symlink check happens at assessment, not when the partial is opened.
- `download.rs:128` (RIGHT) — minor, medium — A resumed transfer may append a whole archive's worth of bytes. *(Merged inline with correctness at `fetcher.rs:353`.)*

**General Findings**:
- suggestion — `prune` reaps `trees/` without the ownership check that `materialise` applies.

---
*Review generated by /accelerator:review-pr*
