---
type: "plan"
id: "2026-10-16-0339-tree-reap-cross-artifact-lock-probe"
title: "Tree Reap Cross-Artifact Lock Probe Implementation Plan"
date: "2026-10-16T15:56:21+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0339"
parent: "work-item:0339"
derived_from: ["codebase-research:2026-10-16-0339-tree-reap-cross-artifact-lock-probe"]
relates_to: ["plan:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness"]
tags: ["cli", "launcher", "cache", "tree", "reap", "concurrency"]
revision: "51ab2d9e870fb271111cc49fcd7b4ff5491c91b6"
repository: "accelerator"
last_updated: "2026-10-17T16:32:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tree Reap Cross-Artifact Lock Probe Implementation Plan

## Overview

`reap_orphans` runs while holding one artifact's tree lock, yet it removes
the `.tmp-` entries of every artifact under `trees/`. A reap for Y can
therefore delete X's half-extracted tree, its `.ref` sidecar or its partial
download while X's materialise is still running.

The fix gives each `.tmp-` entry an owning artifact:

- The reap treats its own artifact's entries as it does today.
- It removes another artifact's entries only while holding that
  artifact's tree lock, taken without blocking and without creating the
  lock file.
- A partial archive survives whenever its digest is the one pinned for its
  own artifact.

Keeping pinned partials must never leave materialisation stuck. A
complete partial is therefore verified without being fetched again. One
whose digest does not match is discarded.

## Current State Analysis

- `reap_orphans(paths, clock, liveness, keep_digests)`
  (`cli/launcher/src/launch/outbound/resolve/tree/reap.rs:41-89`) scans
  `trees/`. It removes every `TempTree` and `TempArchive` that `classify`
  returns (`reap.rs:104-129`), with no check on artifact, lock or age.
  `classify` returns early for every `TEMP_PREFIX` name, so the generation
  test never sees `.tmp-` entries.
- The keep set is matched by substring (`reap.rs:109-114`). Each caller
  passes only its own pinned digest: `materialise` at `resolver.rs:604-605`
  and `prune` at `resolver.rs:746-749`.
- There are exactly two callers of `reap_orphans`. One is
  `MaterialiseTree::materialise` (`resolver.rs:603-611`), which discards
  the result; the other is `TreeResolver::prune` (`resolver.rs:750-751`).
  `cache prune` calls `prune` once per artifact
  (`launch/cache.rs:148-152`). `repair` takes the tree lock directly
  (`resolver.rs:678-680`) but does not reap orphans.
- The tree lock covers a whole materialise. `try_single_flight` is taken at
  `resolver.rs:593`, using `lock_path` (`:576`), and the guard drops only
  after `reap_orphans` returns.
- `try_single_flight` and `take_single_flight` (`lease.rs:63-108`) both
  open the lock file with `create(true)`. Neither checks that the path
  still names the file it locked. `cli/lease/src/lib.rs:225-241` has the
  open-without-create pattern.
- Nothing lists the known platforms. Only the `cfg`-selected
  `HOST_PLATFORM` (`resolve/mod.rs:21-29`) and the `pins.toml` keys name
  them.
- Every `.tmp-` name has a production builder in `TreePaths`
  (`layout.rs:114-134`) except the temp pointer, which `publish_pointer`
  builds inline (`resolver.rs:452`). These names are unchanged since the
  grammar was introduced (`d06ec4db`), so every shipped launcher uses
  them.
- `materialise_fresh` leaves `temp_archive` in place after any error from
  step 5 to the rename (`resolver.rs:261-329`). The step 5 failures that
  leave it include a failed attestation fetch, which runs before
  `verify_archive_file`. `stream_archive` (`download.rs:46-47`) resumes
  from whatever is on disk:
  - A corrupt prefix fails every later run.
  - A complete file makes it send `Range: bytes=<size>-`. A real server
    answers 416, which the fetcher reports as `Unreachable`.

  Today these leftover files are cleared only by accident, when a reap for
  another artifact removes them.
- `try_get_streaming` treats every status that is not a success as
  `Retryable`, 416 included (`fetcher.rs:363-366`). `StreamedBody.partial`
  is a `bool` that says only whether a 206 answered a range.
- `ArtifactPlatformEntry::archive_size` is documented only as a bound on
  the download (`manifest.rs:73-74`). The publisher happens to write the
  exact size (`tasks/manifest.py:171`).
- Step 3's precheck (`resolver.rs:239-242`) asks for `archive_size +
  uncompressed_size`, even when the partial already holds some of those
  bytes. `available_bytes` reads the real filesystem, so nothing can
  inject a value for it in tests.
- The mock server answers 200 with the full body when the range starts at
  or past the end (`tests/common/mod.rs:188`). That hides the 416.
- `cache_prune_reclaims_orphan_residue` (`tests/tree_resolution.rs:880-893`)
  asserts that `.tmp-orphan` is removed.
- `reap.rs` tests already capture `tracing` output with `logged()`
  (`reap.rs:606-637`), and use it to test the `restart_backstop` warning
  (`:639-649`).
- The 0218 work item accepts that removing a retired artifact's lock file
  can race an older launcher (`meta/work/0218-bound-cache-root-growth.md:524-526`).

## Desired End State

The reap can run only with a `HeldTreeLock` for its own artifact. It
groups every `.tmp-` entry by the artifact that owns it, and decides each
group as follows:

```text
.tmp- entry ──parse──► owner? ──none──► keep
                         │
                         ▼
               own artifact ──────────────► reclaimable
               other artifact ─probe lock─► Taken  ──► reclaimable (guard held)
                                            Absent ──► reclaimable (no guard)
                                            Held   ──► keep group
                                            Err    ──► keep group, warn
                         │
             reclaimable ▼
   partial archive pinned for its owner ──► keep
   anything else                        ──► remove
```

The generation path (pointers, leases, `AGE_BACKSTOP`) keeps its 0334
behaviour, and never considers a `.tmp-` name. No reap creates or removes
a `.tmp-*.lock` file. Every tree lock this launcher takes, or a probe
returns, is confirmed to be the file now at its path.

A partial archive at the published size is verified without being
fetched again. When the server answers a resumed range with a 416, a
`Content-Range` total equal to the range start counts as complete. Any
other 416 refetches the whole file. A partial longer than `archive_size`
is also fetched again in full. Recovery therefore never depends on
`archive_size` being exact, or on a server sending an optional header.
A partial whose digest does not match is discarded. The free-space
precheck counts the bytes the partial already holds.

Verify with the 12 acceptance criteria of 0339. Each maps to a named test
below, and `mise run` exits 0.

### Key Discoveries:

- `flock` locks belong to the open file description. A lock held by the
  test thread therefore conflicts with the reap's probe in the same
  process (`tests/tree_resolution.rs:620-647`).
- Where `flock` is emulated with POSIX record locks (Linux NFS without
  `local_lock=flock`, and macOS NFS), closing any descriptor for a file
  drops the process's lock (`cli/lease/src/lib.rs:137-139`, `:202-203`). A
  lock check may therefore open a second descriptor, but it must not close
  it while the lock is held.
- Opening a directory read-write fails with `EISDIR`. That gives AC6 its
  "probe error" fixture (`cli/lease/src/lib.rs:357-363`).
- After the platform, every owned `.tmp-` name continues with `-`, a 64-hex
  digest, and then either `-` (a generation or its temp pointer) or
  `.archive`. The parse therefore accepts a platform only where it is
  followed by such a digest:
  - A newer launcher's `linux-x64-musl` entry has no owner.
  - A name that contains its platform, including at the end, still splits
    at the right place.

  Searching for `-{platform}` without a trailing hyphen stops one match
  from consuming the hyphen the next one needs.
- If the parsed `(name, platform)` split is wrong, the stem is still the
  full string before the digest. `single_flight_lock_of` then names the
  same lock file that the writing launcher used.
- `TreePaths::temp_archive("", platform, digest)` yields
  `.tmp--<platform>-<digest>.archive`, which AC11 uses.
- `Harness` is `Sync` (`tree_resolution.rs:650-665`), so `Arc<Harness>`
  can move into a detached thread. A blocked scoped thread stalls the
  scope's join, so bounded runs cannot use one.
- `logged()` installs its subscriber with
  `tracing::subscriber::with_default`, which affects only the current
  thread. A bounded run must therefore carry the caller's dispatcher into
  the thread it spawns, or the bounded work's warnings go uncaptured.
- `http_test_support::Route::Headers` serves any status with any headers,
  so the fetcher's unit tests can return a 416 with a `Content-Range`.
- `minisign_or_skip!` returns early when `minisign` is missing and `CI` is
  unset (`tree_resolution.rs:68-81`). Every run of `tree_resolution` below
  therefore uses `minisign` on `PATH` and `CI=1`.
- Clippy `pedantic` and `nursery` warn, `warnings = "deny"`, and the
  workspace pins `rust-version = "1.90.0"`. `missing_const_for_fn` forces
  the disposition rule to be a `const fn`. A dead binding or import fails
  `cli:check`. `str::split_at_checked` (1.80) is available, and it returns
  `None` rather than panicking at a non-char boundary.
- The launcher crate has no `public-api.txt` snapshot. Unused `pub` items
  raise no `dead_code`, so Phase 1 can land before anything uses it.
- Each phase writes its acceptance-criteria tests against the API as it
  stands, so they fail for a behavioural reason. The item that changes
  the behaviour also changes the API, and moves the one test helper,
  `reap_for_y`, across with it.

## What We're NOT Doing

- Changing generation reclamation, leases or `AGE_BACKSTOP`.
- Root prunes and orphan-lock removal, which are 0218's work. This plan
  makes launchers from 0339 onwards safe against orphan-lock removal. It
  also records in 0218 that launchers older than 0339 are not safe.
- Making `accelerator cache ensure --from` hold the tree lock (0222).
- Logging unowned entries. They are expected whenever a newer launcher
  shares the root.
- An NFS `statfs` gate on tree locks. ADR-0072 already assumes `flock`
  works on the cache root.
- Moving the remaining `(name, platform)` string callers of `TreePaths`
  onto `Artifact`. The lock and partial-archive builders move; the rest
  can follow when touched.
- Shortening how long another artifact's lock is held while a large
  orphaned tree is removed. A concurrent materialise may hit its bounded
  waiter and degrade for one run. That cannot fail permanently, and the
  module doc records it.

## Implementation Approach

The fix lands in three independently mergeable phases. Each one is driven
red-green-refactor:

| Phase | Adds | Production behaviour change | ACs closed |
|---|---|---|---|
| 1 | `testing`, `KNOWN_PLATFORMS`, `temp_pointer`, `Artifact`, inode-checked locks, `probe_single_flight`, `HeldTreeLock` | tree locks retry if their file was replaced; `publish_pointer` refactor | groundwork |
| 2 | `Reaper` with `PinnedDigests`, grouping by owner, `get_whole`/`resume`, `PartialArchive`, complete-partial verification, mismatched- and unreadable-partial discard | per-owner resumable rule; unowned entries kept; partials can no longer leave materialisation stuck | 3, 4, 7, 11 |
| 3 | `Reaper` built from `HeldTreeLock`, lock probe, `RemovalObserver` | other artifacts' entries gated on their tree lock | 1, 2, 5, 6, 8, 9, 10, 12 |

Phase 2 is safe without Phase 3:

- Other artifacts' temp trees are still removed unconditionally, as today.
- Partial archives are spared more precisely.
- The two partials that would otherwise be kept forever, complete and
  corrupt, are now handled where they are found.

The reap groups entries by owner in a `BTreeMap<Artifact, _>`, so the
groups are visited in a fixed order. A taken guard lives exactly as long
as that owner's removals. Probes never block, so two reaps cannot
deadlock. At worst, each sees the other's lock held and keeps the other's
entries.

Bounded tests run on a detached thread with a 10s `recv_timeout`.

## Phase 1: Naming, Locking and Probe Groundwork

### Overview

This phase adds the domain vocabulary the reap needs:

- the known platforms;
- an artifact parsed from a `.tmp-` entry;
- a temp-pointer builder;
- tree locks checked against their inode;
- a lock probe that neither blocks nor creates the file;
- a held tree lock that proves which artifact, and which root, it guards.

Two production changes ship in this phase. `publish_pointer` uses the new
builder. The single-flight locks retry if the file they locked has been
replaced.

### Changes Required:

#### 1. Test support

**File**: `cli/launcher/src/launch/outbound/resolve/tree/testing.rs`
(`#[cfg(test)] #[allow(clippy::expect_used, clippy::panic)] mod testing;`
in `tree/mod.rs`)

Move `Buffer` and `logged()` from `reap.rs` tests into this module, so
`reap.rs` and `download.rs` share them. The existing
`a_backstop_that_cannot_be_restarted_is_warned` covers the move.

**Tests first**:

- `a_run_that_outlasts_its_bound_fails_instead_of_hanging`, marked
  `#[should_panic(expected = "the probe blocked")]`. It runs a closure that
  calls `take_single_flight` on a lock the test thread holds, under a 100ms
  bound.
- `a_warning_raised_within_a_bounded_run_reaches_the_callers_capture`. It
  runs `logged(|| within(.., || tracing::warn!("x")))` and asserts the
  output contains `WARN`.
- `a_panic_within_a_bounded_run_is_reraised_not_reported_as_blocked`,
  marked `#[should_panic(expected = "boom")]`. The bounded closure panics
  with `"boom"`.

```rust
pub const BLOCKING_BOUND: Duration = Duration::from_secs(10);

#[allow(clippy::panic)]
pub fn within<T: Send + 'static>(
    bound: Duration,
    what: &str,
    run: impl FnOnce() -> T + Send + 'static,
) -> T {
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = sender.send(tracing::dispatcher::with_default(&dispatch, run));
    });
    match receiver.recv_timeout(bound) {
        Ok(outcome) => outcome,
        Err(RecvTimeoutError::Timeout) => panic!("{what} blocked"),
        Err(RecvTimeoutError::Disconnected) => match worker.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => panic!("{what} ended without an outcome"),
        },
    }
}
```

#### 2. Known platforms

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs`

```rust
/// Every platform alias a launcher build publishes trees for, so a cache
/// entry's name can be split at its platform.
pub const KNOWN_PLATFORMS: [&str; 4] =
    ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"];
```

**File**: `tree/pins.rs`
**Tests first**: `every_pinned_platform_is_a_known_platform` and
`the_host_platform_is_a_known_platform`.

#### 3. Temp pointer builder

**File**: `tree/layout.rs`

**Test first**: `a_temp_pointer_is_its_temp_generation_with_the_pointer_suffix`.
Also add `paths.temp_pointer(&generation)` to
`no_sidecar_is_named_like_a_single_file_signature`.

```rust
#[must_use]
pub fn temp_pointer(&self, generation: &str) -> PathBuf {
    self.root
        .join(format!("{TEMP_PREFIX}{generation}{POINTER_SUFFIX}"))
}
```

**File**: `resolver.rs:452`: `let temp = paths.temp_pointer(generation);`

#### 4. `Artifact` value type and its builders

**File**: `tree/layout.rs`

**Tests first**, all using the production builders:

- `a_temp_tree_is_owned_by_its_artifact`, and the same for
  `a_temp_pointer_…` and `a_partial_archive_…`.
- `an_artifact_name_extending_another_owns_its_own_entries`:
  `browser-extra`.
- `an_artifact_name_containing_a_platform_owns_its_own_entries`:
  `driver-linux-x64-shim`, on both `linux-x64` and `darwin-arm64`.
- `an_artifact_name_ending_in_its_platform_owns_its_own_entries`:
  `foo-linux-x64` on `linux-x64`.
- `a_hyphenated_platform_is_split_whole`: `darwin-arm64`.
- `an_entry_on_a_platform_extending_a_known_one_has_no_owner`: a partial
  archive and a temp generation on `linux-x64-musl`.
- `an_entry_with_no_known_platform_has_no_owner`: `.tmp-orphan-residue`.
- `an_entry_with_no_name_before_its_platform_has_no_owner`:
  `temp_archive("", "linux-x64", digest)`.
- `a_lock_file_has_no_owner`.
- `a_partial_archive_names_its_digest` and
  `a_temp_tree_names_no_archive_digest`.
- `every_pinned_artifact_owns_what_each_builder_names`: for every pair in
  `EXPECTED_TREE_DIGESTS`, `owning` on each of `temp_generation`,
  `temp_pointer` and `temp_archive_of` gives back that pair.
- `temp_entry_names_are_fixed_across_launcher_versions`: the exact
  literal file names of `single_flight_lock_of`, `temp_archive_of`,
  `temp_generation` and `temp_pointer` for one fixed artifact, platform,
  digest and suffix. The expected names are built with `[..].concat()`,
  not `format!`, so the phase's `rg` check holds over tests too.

```rust
/// The `(name, platform)` pair a tree lock is keyed on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Artifact {
    name: String,
    platform: String,
}

impl Artifact {
    #[must_use]
    pub fn new(name: &str, platform: &str) -> Self {
        Self {
            name: name.to_owned(),
            platform: platform.to_owned(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn platform(&self) -> &str {
        &self.platform
    }

    /// `None` includes a newer launcher's entry on a platform this one does
    /// not know, which is nobody's residue here.
    #[must_use]
    pub fn owning(temp_entry: &str) -> Option<Self> {
        let rest = temp_entry.strip_prefix(TEMP_PREFIX)?;
        KNOWN_PLATFORMS.iter().find_map(|platform| {
            rest.match_indices(&format!("-{platform}"))
                .find(|(at, matched)| {
                    *at > 0 && is_digest_led(&rest[at + matched.len()..])
                })
                .map(|(at, _)| Self::new(&rest[..at], platform))
        })
    }

    #[must_use]
    pub fn partial_archive_digest<'e>(
        &self,
        temp_entry: &'e str,
    ) -> Option<&'e str> {
        temp_entry
            .strip_prefix(TEMP_PREFIX)?
            .strip_prefix(&self.entry_stem())?
            .strip_prefix('-')?
            .strip_suffix(ARCHIVE_SUFFIX)
    }

    fn entry_stem(&self) -> String {
        format!("{}-{}", self.name, self.platform)
    }
}

fn is_digest_led(after_platform: &str) -> bool {
    after_platform
        .strip_prefix('-')
        .and_then(|rest| rest.split_at_checked(DIGEST_LEN))
        .is_some_and(|(digest, rest)| {
            is_wellformed_digest(digest)
                && (rest.starts_with('-') || rest == ARCHIVE_SUFFIX)
        })
}
```

On `TreePaths`, add `single_flight_lock_of(&Artifact)` and
`temp_archive_of(&Artifact, digest)`, built on `entry_stem`. Make
`single_flight_lock` and `temp_archive` delegate to them. Derive `Clone`
on `TreePaths`.

Extend the `layout.rs` module doc with one paragraph: the `.tmp-` names
are a contract between launcher versions sharing a root, since each reaps
the others' residue by parsing them, and a `LAYOUT_VERSION` bump does not
cover them.

#### 5. Locks checked against their inode

**File**: `tree/lease.rs` (`unix`)

The three lock functions share one loop. It opens the file, locks it, and
then checks that the path still names the file it locked. The check
compares like with like: it opens the path again, without creating it,
and compares the `(dev, ino)` that `fstat` reports for both descriptors.
Comparing `fstat` with a path `stat` can disagree on some overlayfs and
FUSE mounts, where every lock would then fail. An identity error never
turns into a silent retry.

The confirming descriptor is not closed after the check. It lives in the
guard alongside the locked one. On Linux NFS without `local_lock=flock`,
and on macOS NFS, `flock` is emulated with POSIX record locks, which
belong to the process. Closing any descriptor for the file then drops the
lock (`cli/lease/src/lib.rs:137-139`). Both descriptors therefore close
together, when the lock is released.

**Tests first**:

- **Placement checks.** Each test opens a file, optionally unlinks or
  replaces its path, then calls `placement(&file, path)`:
  - `a_locked_file_still_at_its_path_is_in_place`
  - `a_locked_file_unlinked_from_its_path_is_displaced`
  - `a_locked_file_replaced_at_its_path_is_displaced`
- **The shared loop.** These are driven through `lock_confirmed` with an
  injected opener, which counts its calls in an `Arc<AtomicUsize>`:
  - `a_lock_whose_file_is_replaced_once_locks_the_replacement`: the first
    open returns a file whose path is then replaced. Two opens, then
    `Taken` of the replacement.
  - `a_lock_whose_path_is_unlinked_reports_it_absent`: the second open
    finds nothing. Two opens, then `Absent`.
  - `a_lock_that_keeps_being_replaced_says_so`: every open is displaced.
    Under `testing::within` and `testing::logged`, the loop makes exactly
    `MAX_LOCK_ATTEMPTS` opens, returns `KeptBeingReplaced`, and logs a
    `WARN` that names the lock path.
  - `a_confirmed_lock_keeps_its_confirming_descriptor`: a structural check
    that the `Taken` guard holds two descriptors with matching
    `(dev, ino)`. It reads `_file` and `_confirming` directly, so it
    carries `#[allow(clippy::used_underscore_binding)]`. Local filesystems cannot reproduce NFS's lock emulation,
    so this test cannot see a close between `flock` and the guard being
    built. `SingleFlight::confirmed` being the only constructor is what
    rules that out.
  - `an_identity_error_is_a_lease_error`: the opener returns an unrelated
    temp file, and the path's parent is a regular file, so reopening the
    path fails with `ENOTDIR`. Exactly one open, then a `TreeError::Lease`
    whose detail names the lock path.
- **The public lock**:
  `taking_a_lock_whose_file_is_replaced_while_waiting_locks_the_replacement`.
  1. The test thread holds file A's lock at the path.
  2. A thread under `testing::within` calls `take_single_flight`.
  3. After a 100ms pause, so that the taker is blocked on A, the test
     renames a fresh file B over the path and drops A.
  4. The returned guard locks B: a non-blocking `flock` on B from a new
     description gives `WOULDBLOCK`.

  This is red today, because `take_single_flight` returns a lock on the
  unlinked A. Before the rename, the test asserts that the bounded run has
  not returned. That proves the taker is still waiting, but not that it
  opened A. The injected-opener loop tests are the authoritative check of
  the retry, and this test pins the wiring of the public function.
- **The per-caller mappings** are pure functions, `take_outcome`,
  `try_outcome` and `probe_outcome`, each from `Locked`. Each has one test
  per cell of the table below, named for the cell (for example
  `try_treats_a_lock_that_keeps_being_replaced_as_contention`). Probe
  mapping `KeptBeingReplaced` to `Held` is the cell the reap's safety
  relies on.
- **Test fixtures** rename a fresh file over the lock path to replace it.
  `lease.rs` therefore joins `ALLOWLIST` in
  `tasks/lint/store_duplication.py`, with the reason that the renames are
  test-only and the module writes no data.

```rust
const MAX_LOCK_ATTEMPTS: usize = 8;

enum Placement {
    InPlace(File),
    Displaced,
}

fn placement(held: &File, path: &Path) -> Result<Placement, TreeError> {
    let identity_error = |error: std::io::Error| {
        lease(&format!("cannot read the identity of {}: {error}", path.display()))
    };
    let confirming = match File::open(path) {
        Ok(confirming) => confirming,
        Err(error) if is_displacement(&error) => return Ok(Placement::Displaced),
        Err(error) => return Err(identity_error(error)),
    };
    let held_identity = match identity(held) {
        Ok(held_identity) => held_identity,
        Err(error) if is_displacement(&error) => return Ok(Placement::Displaced),
        Err(error) => return Err(identity_error(error)),
    };
    Ok(if held_identity == identity(&confirming).map_err(identity_error)? {
        Placement::InPlace(confirming)
    } else {
        Placement::Displaced
    })
}

fn is_displacement(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
        || rustix::io::Errno::from_io_error(error) == Some(rustix::io::Errno::STALE)
}

fn identity(file: &File) -> std::io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    file.metadata().map(|metadata| (metadata.dev(), metadata.ino()))
}

enum Locked {
    Taken(SingleFlight),
    Held,
    Absent,
    KeptBeingReplaced,
}

fn lock_confirmed(
    path: &Path,
    open: impl Fn(&Path) -> std::io::Result<Option<File>>,
    operation: FlockOperation,
) -> Result<Locked, TreeError> {
    for _ in 0..MAX_LOCK_ATTEMPTS {
        let Some(file) = open(path).map_err(|error| {
            lease(&format!("cannot open the single-flight lock: {error}"))
        })?
        else {
            return Ok(Locked::Absent);
        };
        match flock(file.as_fd(), operation) {
            Ok(()) => match placement(&file, path)? {
                Placement::InPlace(confirming) => {
                    return Ok(Locked::Taken(SingleFlight::confirmed(file, confirming)))
                }
                Placement::Displaced => {}
            },
            Err(rustix::io::Errno::WOULDBLOCK) => return Ok(Locked::Held),
            Err(error) => {
                return Err(lease(&format!(
                    "cannot take the single-flight lock: {error}"
                )))
            }
        }
    }
    tracing::warn!(
        lock = %path.display(),
        "the single-flight lock file keeps being replaced"
    );
    Ok(Locked::KeptBeingReplaced)
}
```

**Test first**: `is_displacement` has one pure test per cell. `NotFound`
and `ESTALE` (built with `std::io::Error::from_raw_os_error`) displace;
`EACCES` and `ENOTDIR` do not. The tests are
`a_missing_file_is_a_displacement`, `a_stale_handle_is_a_displacement`,
`a_denied_access_is_not_a_displacement` and
`a_path_through_a_file_is_not_a_displacement`. On a root shared by several NFS clients,
another client's unlink shows up as `ESTALE` on a descriptor that is
still open, so it is a displacement, not a fault.

`SingleFlight` gains the second descriptor. Its one constructor is the
only place the two descriptors are paired:

```rust
pub struct SingleFlight {
    _file: File,
    /// Closed only with `_file`: where `flock` is emulated with POSIX
    /// locks, closing any descriptor for the file drops the lock.
    _confirming: File,
}

impl SingleFlight {
    const fn confirmed(held: File, confirming: File) -> Self {
        Self { _file: held, _confirming: confirming }
    }
}
```

Each caller maps every outcome explicitly. None of them uses
`unreachable!`, and each mapping only moves the guard.

| `Locked` | `take_single_flight` (`open_creating`, blocking) | `try_single_flight` (`open_creating`) | `probe_single_flight` (`open_existing`) |
|---|---|---|---|
| `Taken(guard)` | `Ok(guard)` | `Ok(Some(guard))` | `Taken(guard)` |
| `Held` | `Err(Lease)`: a blocking lock reported contention | `Ok(None)` | `Held` |
| `Absent` | `Err(Lease)`: the opener reported no file | `Err(Lease)`: the same | `Absent` |
| `KeptBeingReplaced` | `Err(Lease)`: the file keeps being replaced | `Ok(None)`: treated as contention | `Held` |

Mapping `KeptBeingReplaced` to `None` in `try_single_flight` keeps the
bounded waiter on its non-sticky `MaterialisationInProgress` path.
Contention there is not a fault, so it should not become a sticky
`Lease`.

`open_creating` and `open_existing` are free functions of the path. The
opener therefore always opens exactly the path that `placement` checks.
`open_creating` keeps the old `open_lock_file`'s options (read, write,
create, no truncate, `0o600`) and replaces it. The `Absent` and
`KeptBeingReplaced` faults share two error constructors, `no_lock_file()`
and `kept_being_replaced()`.

#### 6. Lock probe that neither blocks nor creates the file

**File**: `tree/lease.rs`

**Tests first** (in `unix::tests`; every probe of a held lock runs through
`testing::within(BLOCKING_BOUND, "the probe", ..)`):

- `probing_an_absent_lock_finds_it_absent_and_creates_nothing`.
- `probing_a_held_lock_finds_it_held`.
- `probing_a_free_lock_takes_it_until_the_guard_drops`.
- `probing_a_lock_that_cannot_be_opened_is_an_error`: a directory sits at
  the lock path.

`SingleFlightProbe` is defined outside both `cfg` modules:

```rust
pub enum SingleFlightProbe {
    Taken(SingleFlight),
    Held,
    Absent,
}
```

```rust
/// Try `LOCK_EX` without blocking and without creating the lock file, so a
/// probe of a retired artifact's lock leaves nothing behind.
///
/// # Errors
///
/// [`TreeError::Lease`] if the lock file exists but cannot be opened for
/// writing, its identity cannot be read, or locking fails for a reason
/// other than contention.
pub fn probe_single_flight(
    path: &Path,
) -> Result<SingleFlightProbe, TreeError> {
    lock_confirmed(path, open_existing, FlockOperation::NonBlockingLockExclusive)
        .map(probe_outcome)
}

fn probe_outcome(locked: Locked) -> SingleFlightProbe {
    match locked {
        Locked::Taken(guard) => SingleFlightProbe::Taken(guard),
        Locked::Held | Locked::KeptBeingReplaced => SingleFlightProbe::Held,
        Locked::Absent => SingleFlightProbe::Absent,
    }
}

fn open_existing(path: &Path) -> std::io::Result<Option<File>> {
    match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
```

Export it with `pub use unix::{probe_single_flight, ...}`. Add an
`unimplemented!` stub, and add it to `pub use stubs::{..}`
(`lease.rs:168`).

#### 7. Held tree lock

**File**: `cli/launcher/src/launch/outbound/resolve/tree/tree_lock.rs`
(new, composing `lease`'s primitives, which stay free of tree-domain
types). The module is not `cfg`-gated, since `lease` has non-Unix stubs;
its tests are `#[cfg(all(test, unix))]`.

**Tests first** (every case where the lock is held elsewhere runs
through `testing::within`):

- `holding_a_tree_lock_holds_its_artifacts_single_flight_lock`: while a
  `HeldTreeLock::take(&paths, a)` is alive, a probe of
  `single_flight_lock_of(&a)` gives `Held`. `artifact()` is `a`, and
  `paths().root()` is the root it was taken under.
- `trying_a_free_tree_lock_takes_it`: `Some`, with the right `artifact()`,
  and a probe then gives `Held`.
- `trying_a_tree_lock_held_elsewhere_gives_none`.

```rust
/// A tree lock this process holds, the artifact it guards and the root it
/// was taken under; the only way to say whose reap is running.
pub struct HeldTreeLock {
    paths: TreePaths,
    artifact: Artifact,
    _guard: SingleFlight,
}

impl HeldTreeLock {
    pub fn take(paths: &TreePaths, artifact: Artifact) -> Result<Self, TreeError>;
    pub fn try_take(paths: &TreePaths, artifact: Artifact) -> Result<Option<Self>, TreeError>;
    pub const fn artifact(&self) -> &Artifact;
    pub const fn paths(&self) -> &TreePaths;
}
```

`take` and `try_take` both build the value through one private
constructor, `holding(paths, artifact, guard)`, which clones `paths`.

### Success Criteria:

#### Automated Verification:

- [x] New tests fail before their code and pass after: `cargo test --manifest-path cli/Cargo.toml -p accelerator --lib tree::`
- [x] `a_run_that_outlasts_its_bound_fails_instead_of_hanging` passes within ~1s
- [x] Integration suite still passes: `CI=1 cargo test --manifest-path cli/Cargo.toml -p accelerator --test tree_resolution`
- [x] `rg -n 'format!\(".tmp-' cli/launcher/src` returns nothing
- [x] `mise run cli:check` exits 0
- [x] `mise run check` exits 0

#### Manual Verification:

- [x] The only production behaviour changes are `publish_pointer` using
      `temp_pointer`, and the single-flight locks confirming their inode

---

## Phase 2: Per-Artifact Resumable Rule and Partials That Always Recover

### Overview

This phase makes four changes:

- It makes a partial archive always recoverable:
  - a complete one is verified without being fetched, whether the
    manifest size or a server's 416 says it is complete;
  - an over-long one is fetched again in full;
  - one whose digest does not match is discarded;
  - the free-space precheck counts the bytes the partial already holds.
- It replaces the single-digest keep set with a pin lookup that asks
  about each archive's own artifact.
- It groups `.tmp-` entries by owner.
- It keeps entries with no owner.

Other artifacts' temp trees are still removed unconditionally.

### Changes Required:

#### 1. Partials that always recover

The work goes in three steps:

1. **Step 1a** reshapes the API and changes no behaviour.
2. **Step 1b** adds tests that fail on their assertions, not on
   compilation.
3. **Step 1c** adds the behaviour.

**Step 1a: refactor the fetch API and the partial assessment (no
behaviour change)**

**File**: `cli/launcher/src/launch/outbound/resolve/fetcher.rs`

`get_streaming` splits into two entry points. Neither can express an
outcome that its request cannot produce.

```rust
pub(super) struct Whole {
    pub sha256: [u8; 32],
}

pub(super) enum SinkMode {
    Truncate,
    Append { from: u64 },
}

#[allow(dead_code)]
pub(super) enum Resumed {
    Whole(Whole),
    Appended,
    AlreadyComplete,
    RangeRejected,
}

pub(super) fn get_whole<'sink>(
    &self,
    url: &str,
    limits: &StreamLimits,
    open_sink: &mut dyn FnMut() -> std::io::Result<Box<dyn StreamSink + 'sink>>,
) -> Result<Whole, FetchError>;

pub(super) fn resume<'sink>(
    &self,
    url: &str,
    limits: &StreamLimits,
    from: u64,
    open_sink: &mut dyn FnMut(SinkMode) -> std::io::Result<Box<dyn StreamSink + 'sink>>,
) -> Result<Resumed, FetchError>;
```

`FetchError` gains a `Sink(String)` variant. A sink that cannot be opened
is terminal: the fault is local, so retrying the network cannot fix it.

Step 1a constructs neither `AlreadyComplete` nor `RangeRejected`, so
`Resumed` keeps the `#[allow(dead_code)]` that `StreamedBody` has today.
Step 1c removes it.

Every ranged attempt opens its sink with `SinkMode::Append { from }`, so
a retry always appends from the agreed offset. A 206 counts as `Appended`
only when its `Content-Range` starts at `from`; any other start is
`RangeRejected`. Move the existing fetcher
tests onto `get_whole`. Where they assert `body.bytes`, assert the length
of what the sink received instead.

**File**: `tree/download.rs`

```rust
pub enum PartialArchive {
    Absent,
    Resumable(u64),
    Complete,
    OverLong,
}

impl PartialArchive {
    #[must_use]
    pub const fn assess(on_disk: Option<u64>, archive_size: u64) -> Self;

    /// The partial's bytes are either kept or truncated away before the
    /// download writes, so the peak need is the shortfall, never more.
    #[must_use]
    pub const fn additional_bytes(&self, archive_size: u64) -> u64;

}

pub struct AssessedPartial {
    path: PathBuf,
    archive_size: u64,
    state: PartialArchive,
}

impl AssessedPartial {
    pub fn of(path: &Path, archive_size: u64) -> Self;
    #[must_use]
    pub const fn additional_bytes(&self) -> u64;
}
```

**Tests first** (pure):

- **`assess`**: `no_file_is_absent`, `an_empty_file_is_absent`,
  `a_short_file_is_resumable_from_its_length`,
  `a_file_of_the_archive_size_is_complete` and
  `a_longer_file_is_over_long`.
- **`additional_bytes`**:
  - `an_absent_partial_needs_the_whole_archive`;
  - `a_resumable_partial_needs_the_rest_of_the_archive`;
  - `a_complete_partial_needs_nothing_more`;
  - `an_over_long_partial_needs_nothing_more_since_truncation_frees_it`.

`materialise_fresh` assesses the partial once, with
`AssessedPartial::of(&temp_archive, archive_size)`. That value carries the
path, the archive size and the `PartialArchive`, so `stream_archive` takes
it alone, and the path cannot disagree with its assessment. `of` uses
`symlink_metadata`, and treats anything that is not a regular file as
`OverLong`, so it is replaced rather than resumed.

`stream_archive` calls one helper for each assessment, so the decision
tables below have one place each:

| Helper | Calls | Purpose |
|---|---|---|
| `fetch_whole` | `get_whole` | download the whole archive |
| `resume_from(n)` | `resume` | continue a partial from `n` |
| `digest_partial` | `digest_file` | hash what is already on disk |

Step 1a keeps today's behaviour for every assessment, and the existing
suite covers it.

**Step 1b: tests, red on their assertions**

**File**: `cli/launcher/tests/common/mod.rs`

- The mock answers `416 Range Not Satisfiable` with
  `Content-Range: bytes */<len>` for a range that starts at or past the
  end, as a real server does.
- A new `Route::FailFirstRange(body)` answers the first ranged request
  with a `206` head whose `Content-Length` is the full suffix length,
  writes half of the suffix, flushes, and returns without the
  read-to-end guard, mirroring `http_test_support`'s `Route::Truncated`
  (`http-test-support/src/lib.rs:473-483`). A `Content-Length` matching
  only the half sent would make the read end cleanly, and nothing would
  retry. Later requests are served normally.

**File**: `cli/launcher/tests/tree_resolution.rs`

`Harness` keeps the `minisign` path and the secret key. A
`ManifestEntry { signature, archive_size, uncompressed_size }` fixture
replaces `manifest_json`'s positional arguments.
`Harness::serve_manifest(&ManifestEntry)` renders the manifest, signs it,
and routes both `/manifest.json` and `/manifest.minisig`.
`happy_harness` serves the default entry through it.

**File**: `fetcher.rs` tests, using `http_test_support::Route::Headers`.

These are red today, because every 416 ends in `Unreachable`:

- `a_range_starting_at_the_end_is_already_complete`: a request ranged
  from `N` is answered `416` with `Content-Range: bytes */N`. Expect
  `AlreadyComplete`, and no sink opened.
- `a_416_for_a_shorter_resource_rejects_the_range`: `bytes */M` with
  `M < N` gives `RangeRejected`.
- `a_416_for_a_longer_resource_rejects_the_range`: `bytes */M` with
  `M > N` gives `RangeRejected`.
- `a_416_without_a_content_range_rejects_the_range`: gives
  `RangeRejected`.
- `a_206_from_a_different_offset_rejects_the_range`: a 206 whose
  `Content-Range` starts at `N + 1` gives `RangeRejected`, and no sink is
  opened.
- `a_server_ignoring_the_range_answers_whole`: `resume(from = N)` against
  `Route::Bytes { status: 200, .. }` returns `Resumed::Whole`. The sink is
  opened once with `SinkMode::Truncate` and receives the full body. This
  pins the path that keeps resume correct behind a CDN that ignores
  ranges.
- `a_sink_that_cannot_be_opened_is_not_retried`: an opener that fails
  gives `FetchError::Sink` after exactly one request.

**File**: `tree_resolution.rs`. Each of these tests is red against today's
code unless it is marked otherwise.

- `a_complete_partial_is_verified_without_refetching`:
  1. Plant all of `harness.archive_bytes` at
     `temp_archive(ARTIFACT, HOST_PLATFORM, &harness.archive_sha)`.
  2. Assert that `materialise` succeeds and the archive route records no
     hit.

  Red, because the 416 surfaces as `Unreachable`.
  `a_failed_winner_releases_the_lock_and_the_next_makes_progress` goes
  red for the same reason once the mock is fixed.
- `a_complete_partial_under_a_loose_size_bound_is_confirmed_by_the_server`:
  serve `archive_size` as the real length plus 1024, and plant the
  complete archive. `materialise` succeeds. The archive route records
  exactly one hit, the range request, so a whole refetch fails the test.
- `a_partial_longer_than_the_served_file_under_a_loose_bound_is_refetched_whole`:
  under the same loose bound, plant the archive plus 10 bytes.
  `materialise` succeeds after one range request and one full fetch.
- `an_over_long_partial_is_refetched_whole`: plant the archive plus one
  byte past `archive_size`. `materialise` succeeds with no range request.
- `a_resumed_download_that_fails_mid_body_retries_from_the_agreed_offset`:
  1. Route the archive with `FailFirstRange`.
  2. Plant the first half of the archive.
  3. Assert that `materialise` succeeds.

  Red today: the retry appends after the failed attempt's bytes, and the
  digest no longer matches.
- `a_corrupt_partial_archive_is_discarded_so_the_next_run_recovers`:
  1. Plant the first half of the archive with one byte flipped.
  2. `materialise` returns `Err(UnexpectedDigest)`.
  3. Assert that the temp archive is gone.
  4. Assert that a second `materialise` succeeds.
- `a_complete_corrupt_partial_is_discarded_so_the_next_run_recovers`: the
  same steps, using the whole archive with one byte flipped.
- `an_unreadable_partial_is_discarded_so_the_next_run_recovers`:
  1. Plant the complete archive with mode `0o000`. Then try `File::open`
     on it, and skip the test if that succeeds: a process with
     `CAP_DAC_OVERRIDE` reads it anyway.
  2. `materialise` fails with `TreeError::Extraction`, whose detail
     contains `cannot re-read`.
  3. Assert that the temp archive is gone.
  4. Assert that a second `materialise` succeeds.
- `an_unwritable_short_partial_is_replaced_in_the_same_run`: plant the
  first half at mode `0o444`, with the same readability-based skip rule.
  `materialise` succeeds: the append sink fails to open, the partial is
  discarded, and a whole fetch follows. Red today, because the open
  failure is retried until `Unreachable`.
- `a_partial_shorter_than_its_offset_is_replaced_not_padded`: the
  `download.rs` unit test opens an `Append { from: 10 }` sink over a
  5-byte file. It fails, and the file is not extended.
- `a_partial_whose_signature_fails_is_kept_for_the_next_run`:
  1. Plant the complete, correct archive.
  2. Call `serve_manifest` with that entry's `signature` taken from other
     bytes. The re-signed manifest passes its own check at step 2.
  3. `materialise` fails with `Attestation`, with a detail containing
     `does not verify under`. Assert that the temp archive survives.
  4. Restore the default entry. `materialise` succeeds with no archive
     hit.

  This one goes green as soon as step 1c lands. It guards the narrow scope
  of the discard: a mutation that discards on any verification error
  fails it.
- In `a_manifest_naming_a_different_digest_is_a_refusal_not_a_fetch`,
  plant a partial at `temp_archive(ARTIFACT, HOST_PLATFORM, &"0".repeat(64))`,
  the digest the resolver expects. Assert that this exact path survives.
  This guard passes before and after step 1c. It catches a discard
  mis-scoped to any `UnexpectedDigest`.
- `the_free_space_check_counts_the_partial_already_on_disk`:
  1. Serve `uncompressed_size = 1 << 50`, and plant half the archive.
  2. Assert that `materialise` returns `Err(DiskShortfall { needed, .. })`
     with `needed == (1 << 50) + len - len / 2`.

  Red, because today `needed` counts the whole archive again.
- In `a_corrupt_archive_is_refused_before_anything_is_extracted`, assert
  that the temp archive is gone.

**Step 1c: behaviour**

**File**: `fetcher.rs`

- **`resume`**: a 416 is never retried. If the `Content-Range` total
  equals `from`, the result is `AlreadyComplete`. Otherwise, including a
  missing or unparsable header, the result is `RangeRejected`. A 206
  whose `Content-Range` start is not `from` is also `RangeRejected`.
- **Both entry points**: a failed `open_sink` returns
  `FetchError::Sink`, which is never retried.
- **`#[allow(dead_code)]`**: remove it from `Resumed`.

**File**: `tree/download.rs`

`stream_archive` acts on the assessment:

| Assessment | Action |
|---|---|
| `Absent` | `fetch_whole` |
| `Resumable(n)` | `resume_from(n)` |
| `Complete` | `digest_partial`, with no fetch |
| `OverLong` | `fetch_whole`, which truncates |

`resume_from` acts on the `Resumed` result:

| `Resumed` | Action |
|---|---|
| `Whole(whole)` | use `whole.sha256` |
| `Appended` or `AlreadyComplete` | `digest_partial` |
| `RangeRejected` | `fetch_whole` once, which truncates |
| `FetchError::Sink` | `discard_partial_archive`, then `fetch_whole` once |

`fetch_whole` gets the same treatment. On `FetchError::Sink`, which can
happen when a directory sits at the path, it calls
`discard_partial_archive` and tries once more. A second failure is
returned.

- **The `SinkMode::Append { from }` sink** opens without `create`. It
  fails when the file is shorter than `from`, and otherwise calls
  `set_len(from)` to shrink it before appending. A failed attempt's bytes
  therefore never stay in the file, and a partial that shrank or vanished
  is replaced, never padded with zeros.
- **`digest_partial`** hashes with `std::io::copy` into the hasher, which
  retries `Interrupted`. It discards the partial only when the error
  belongs to the file itself (`PermissionDenied`). On `NotFound` it falls
  back to `fetch_whole`. On any other error it keeps the partial and
  returns the error, so a transient fault never costs a valid download.
- **`discard_partial_archive(path: &Path)`** removes the file, or the
  directory if one sits there. When it cannot, it logs `tracing::warn!`
  with the path and the error. It returns nothing, so a failed removal
  can never replace the caller's error.

**Test first**: `a_partial_that_cannot_be_discarded_is_warned`. Put a
non-empty directory inside a read-only parent at the path, and use
`testing::logged`.

The discard rules live in two layers, and both docstrings name them. In
`stream_archive`, a partial that cannot be read or written is replaced.
In `materialise_fresh`, a partial that fails its digest is discarded.

**File**: `resolver.rs` (`materialise_fresh`)

- **Step 5**: when the step-5 call to `verify_archive_file` returns
  `TreeError::UnexpectedDigest`, call `discard_partial_archive` on the temp
  archive, then return the error. The rule is attached to that call site
  with `inspect_err`, not to the variant, so step 2's `UnexpectedDigest`
  cannot trigger it. Other
  verification failures keep the file. Bytes whose digest matches the pin
  are the release bytes, so the next run checks them again without
  downloading. The manifest-disagreement `UnexpectedDigest` at step 2
  (`:230`) comes before any partial is touched, so it never discards.
- **Step 3**: the precheck asks for
  `uncompressed_size + partial.additional_bytes()`.

#### 2. Bounded reap test harness

**File**: `reap.rs` (`tests`)

```rust
const PLATFORM: &str = "linux-x64";

fn y() -> Artifact {
    Artifact::new("driver", PLATFORM)
}

#[derive(Default)]
struct Pins(BTreeMap<Artifact, String>);

fn reap_for_y_at(
    root: &Path,
    clock: impl Clock + Send + 'static,
    pins: Pins,
) -> Result<Reclaimed, TreeError> {
    let paths = TreePaths::under(root);
    let _caller_holds =
        take_single_flight(&paths.single_flight_lock_of(&y())).expect("Y's lock");
    let locks_before = lock_files(&paths);
    let root = root.to_owned();
    let outcome = testing::within(BLOCKING_BOUND, "the reap", move || {
        let paths = TreePaths::under(&root);
        let keep = pins.0.get(&y()).cloned().into_iter().collect();
        reap_orphans(&paths, &clock, &FlockProbe, &keep)
    });
    assert_eq!(lock_files(&paths), locks_before, "a lock file changed");
    outcome
}

fn reap_for_y(root: &Path, pins: Pins) -> Result<Reclaimed, TreeError> {
    reap_for_y_at(root, present(), pins)
}
```

The red step deliberately drives today's API. It derives the keep set
from Y's pin, exactly as production does today.

`lock_files` returns the sorted `.tmp-*.lock` names. `plant_residue(paths,
&Artifact) -> Residue` uses the production builders to create a temp
tree, its `.ref` sidecar and a partial archive at `"f".repeat(64)`. It
returns their exact paths.

Move the existing generation tests onto `reap_for_y_at` with their
`well_aged()` or `present()` clock. Add
`a_temp_pointer_reaped_alongside_still_protects_its_generation` as a
characterisation test that passes today.

#### 3. Acceptance tests, red against today's reap

- AC7 `the_reaps_own_residue_goes_but_its_resumable_archive_stays`. This
  is a characterisation test and passes today.
- AC3 `another_artifacts_pinned_archive_survives`: X's archive at X's
  pinned digest. X's lock file exists but is free. Red, because the keep
  set holds only Y's digest.
- AC4 `an_archive_pinned_only_for_another_artifact_is_reclaimed`: an
  archive named for X, at Y's pinned digest. X's lock file exists but is
  free. Red, because the substring match spares it.
- AC11 `entries_with_no_known_owner_survive`, run with `well_aged()`. The
  fixtures are:
  - `.tmp-orphan-residue`;
  - `temp_archive("", PLATFORM, digest)`;
  - an archive on `linux-x64-musl`;
  - a `temp_generation` on `linux-x64-musl`, which contains `-1-`.

  Red, because today every `.tmp-` entry goes.
- Rename `a_partial_archive_for_the_wanted_digest_is_spared` to
  `a_partial_archive_pinned_for_its_own_artifact_is_spared`.

#### 4. `Reaper`, pin lookup and grouping by owner

**File**: `reap.rs`

```rust
pub trait PinnedDigests {
    fn pinned_digest(&self, artifact: &Artifact) -> Option<String>;
}

pub struct Reaper<'a> {
    paths: &'a TreePaths,
    clock: &'a dyn Clock,
    liveness: &'a dyn LivenessProbe,
    pins: &'a dyn PinnedDigests,
}

impl Reaper<'_> {
    pub fn reap_orphans(&self) -> Result<Reclaimed, TreeError> {
        let names = entry_names(self.paths.root())?;
        let referenced = referenced_generations(self.paths.root());
        let mut reclaimed = Reclaimed::default();
        for (owner, entries) in temp_entries_by_owner(&names) {
            self.reap_temp_entries_of(&owner, &entries, &mut reclaimed);
        }
        self.reap_generations(&names, &referenced, &mut reclaimed);
        Ok(reclaimed)
    }

    fn reap_temp_entries_of(
        &self,
        owner: &Artifact,
        entries: &[&str],
        reclaimed: &mut Reclaimed,
    ) {
        for entry in entries {
            if !is_resumable(entry, owner, self.pins) {
                remove(&self.paths.root().join(entry), reclaimed);
            }
        }
    }
}

fn is_resumable(
    entry: &str,
    owner: &Artifact,
    pins: &dyn PinnedDigests,
) -> bool {
    owner.partial_archive_digest(entry).is_some_and(|digest| {
        pins.pinned_digest(owner).as_deref() == Some(digest)
    })
}
```

- `entry_names` keeps the existing `read_dir` handling.
- `referenced` is computed once, before any removal.
- `temp_entries_by_owner` builds a `BTreeMap<Artifact, Vec<&str>>` with
  `Artifact::owning` alone. Lock files and unowned entries parse to
  `None`, so they are kept.
- `reap_generations` is the existing generation arm. `classify` shrinks to
  `is_generation`, which first rejects any name starting with
  `TEMP_PREFIX`, then applies today's test. `Residue`, the keep set and
  the `.archive` and `.lock` checks all go.

`reap_for_y_at` now builds `Reaper::new(&paths, &clock, &FlockProbe,
&pins)`, with `PinnedDigests` implemented for `Pins`. The `reap_orphans`
docstring says that a partial archive survives when its digest is pinned
for its own artifact, and that an entry with no known owner survives.

#### 5. Callers

**File**: `resolver.rs`

```rust
impl reap::PinnedDigests for ExpectedDigests {
    fn pinned_digest(&self, artifact: &Artifact) -> Option<String> {
        self.digest_for(artifact.name(), artifact.platform())
    }
}
```

Both `keep` constructions go. Both calls become
`Reaper::new(&paths, self.clock, self.liveness, &self.expected_digests)
.reap_orphans()`. In `prune`'s docstring (`:723-724`), say instead that
every artifact's pinned partial archive is spared.

#### 6. End-to-end resumable rule and the rewritten prune test

**File**: `cli/launcher/tests/tree_resolution.rs`

Add the following:

- `const BYSTANDER: &str = "browser";`
- `fn bystander_digest() -> String { "b".repeat(64) }`. It is distinct
  from `harness.archive_sha`, which today's substring keep set would
  spare.
- `Harness::resolver_also_pinning_bystander(&self)`.
- `within_bound<T: Send + 'static>(harness: Arc<Harness>, run: impl
  FnOnce(&Harness) -> T + Send + 'static) -> T`. It snapshots the sorted
  `.tmp-*.lock` names before and after, and asserts that they are equal.
  Its body is a `within` in `tests/common/mod.rs` with the same `Timeout`,
  `Disconnected` and `resume_unwind` handling as `testing::within`, and
  the same two tests (a blocked run, and a re-raised panic). An
  integration test cannot reach a `#[cfg(test)]` module, and a support
  crate for one function is not worth registering. Two copies with
  matching tests is the tradeoff accepted here. Every test that uses
  it first creates `ARTIFACT`'s lock file, with `take_single_flight`
  followed by dropping the guard, because a materialise creates its own
  lock file.

**Tests first** (with `BYSTANDER`'s lock file created by
`take_single_flight` and its guard dropped):

- AC3(a) `another_artifacts_pinned_archive_survives_a_materialise`: plant
  `temp_archive(BYSTANDER, HOST_PLATFORM, &bystander_digest())`, then
  materialise `ARTIFACT` within the bound.
- AC3(b) `another_artifacts_pinned_archive_survives_a_prune`.
- `cache_prune_reclaims_owned_orphans_and_keeps_unowned_ones` replaces
  `cache_prune_reclaims_orphan_residue`. An `ARTIFACT` `temp_generation`
  is reclaimed and reported, and `trees/.tmp-orphan` survives.

The AC3 tests and the prune test are red against today's code.

### Success Criteria:

#### Automated Verification:

- [ ] Step 1a leaves the existing suite green
- [ ] Every test in items 1, 3 and 6 that is marked red is observed failing on its assertion, for its stated behavioural reason, before the step that fixes it, with `CI=1` and `minisign` on `PATH`
- [ ] Fetcher, download, resolver and reap unit tests pass: `cargo test --manifest-path cli/Cargo.toml -p accelerator --lib resolve::`
- [ ] Full launcher suite passes: `CI=1 cargo test --manifest-path cli/Cargo.toml -p accelerator`
- [ ] `rg -n 'keep_digests' cli/launcher/src` returns nothing
- [ ] `mise run check` exits 0

#### Manual Verification:

- [ ] Apart from the move, the diff leaves generation handling
      unchanged: `is_reclaimable_generation`, the sidecar removal and
      `referenced_generations`

---

## Phase 3: Cross-Artifact Lock Probe

### Overview

The reap is built from a `HeldTreeLock`, so it knows its own artifact and
root. It probes each other owner's tree lock, and holds any lock it takes
for the whole of that owner's removals. A `RemovalObserver` test seam lets
AC2 see which locks are held at each removal.

### Changes Required:

#### 1. Disposition rule

**File**: `reap.rs`

**Tests first**, one per cell: `own_residue_is_reclaimable`,
`residue_under_a_taken_lock_is_reclaimable`,
`residue_with_no_lock_file_is_reclaimable`,
`residue_under_a_lock_held_elsewhere_is_kept` and
`residue_under_an_unprobeable_lock_is_kept`.

```rust
enum OwnerLock {
    HeldByCaller,
    Taken { _guard: SingleFlight },
    Absent,
    HeldElsewhere,
    Unprobeable(TreeError),
}

const fn temp_residue_is_reclaimable(lock: &OwnerLock) -> bool {
    matches!(
        lock,
        OwnerLock::HeldByCaller | OwnerLock::Taken { .. } | OwnerLock::Absent
    )
}
```

#### 2. Removal observer seam

**File**: `reap.rs`

**Test first**: `every_counted_removal_was_observed`. Every entry
counted in `Reclaimed` was seen by the observer first. In this fixture
nothing fails to remove, so the two sets are equal. In general the
counted entries are a subset of the observed ones.

```rust
/// Sees each entry the reap is about to remove; a generation's sidecars go
/// with it, unseen.
pub trait RemovalObserver {
    fn removing(&self, entry: &Path);
}

pub struct UnobservedRemovals;

impl RemovalObserver for UnobservedRemovals {
    fn removing(&self, _entry: &Path) {}
}
```

`Reaper::new` defaults to `&UnobservedRemovals`, and tests opt in with
`.observed_by(&observer)`. `remove` becomes a `Reaper` method that calls
`observer.removing(path)` first. `reap_for_y_at` gains an
`observer: impl RemovalObserver + Send + 'static`.

#### 3. Acceptance tests, red against Phase 2's reap

These tests run through `reap_for_y_at`, which still holds Y's lock on the
test thread and calls Phase 2's `reap_orphans()`.

The bystanders are A = `aardvark` and Z = `zebra`, on either side of
X = `browser`. Each has an orphan temp tree, and a lock file that exists
but is free. A recording observer pushes onto an
`Arc<Mutex<Vec<Observed>>>`.

- AC1 `another_artifacts_residue_survives_while_its_tree_lock_is_held`,
  with `well_aged()`. The test thread holds X's lock. Expected: `Ok`, X's
  three entries survive, and A's and Z's orphans are gone. Red.
- AC2 `another_artifacts_residue_is_removed_under_its_tree_lock`. X's lock
  is free, and X also has an archive at its pinned digest. When the
  observer sees one of X's three exact `plant_residue` paths, it probes
  X's lock and records `(path, held)`. Expected:
  - exactly those three paths are recorded, all of them held;
  - the pinned archive survives;
  - a later probe gives `Taken`.

  Red on the held assertion.
- AC5 `a_retired_artifacts_residue_goes_without_creating_its_lock`:
  `retired` is pinned and has no lock file. Its unpinned entries go, its
  pinned archive survives, and the set of lock files is unchanged. This is
  an over-correction guard that passes before and after.
- AC6 `another_artifacts_residue_survives_an_unprobeable_lock`: a
  directory sits at X's lock path. Expected: X's entries survive, A's and
  Z's orphans are gone, and `Ok`. Red.
- AC6′ `an_unprobeable_lock_is_warned_with_its_artifact`: the same
  fixture, wrapped in `testing::logged`. `within` carries the capture onto
  the reap's thread. The output contains `WARN`, `browser` and
  `linux-x64`. Red, because Phase 2 emits no warning.
- AC8 `own_name_on_another_platform_survives_while_its_lock_is_held`:
  `driver` on `darwin-arm64`. Red.
- AC9 `own_name_on_another_platform_goes_while_its_lock_is_free`: an
  over-correction guard that passes before and after.
- AC10 `an_artifact_extending_the_own_name_is_another_artifact`:
  `driver-extra`, with its lock held. Red.

**File**: `cli/launcher/tests/tree_resolution.rs`

Hold `BYSTANDER`'s lock and plant its temp tree, its `.ref` sidecar and an
archive at `"f".repeat(64)`.

- AC12(a) `another_artifacts_materialise_survives_this_ones_materialise`.
- AC12(b) `another_artifacts_materialise_survives_this_ones_prune`.

Both are red against Phase 2.

#### 4. Lock-gated reap built from the held lock

**File**: `reap.rs`

```rust
pub struct Reaper<'a> {
    held: &'a HeldTreeLock,
    clock: &'a dyn Clock,
    liveness: &'a dyn LivenessProbe,
    pins: &'a dyn PinnedDigests,
    observer: &'a dyn RemovalObserver,
}

impl<'a> Reaper<'a> {
    #[must_use]
    pub const fn observed_by(self, observer: &'a dyn RemovalObserver) -> Self {
        Self { observer, ..self }
    }

    pub const fn new(
        held: &'a HeldTreeLock,
        clock: &'a dyn Clock,
        liveness: &'a dyn LivenessProbe,
        pins: &'a dyn PinnedDigests,
    ) -> Self;

    fn reap_temp_entries_of(
        &self,
        owner: &Artifact,
        entries: &[&str],
        reclaimed: &mut Reclaimed,
    ) {
        let lock = self.lock_of(owner);
        if let OwnerLock::Unprobeable(error) = &lock {
            tracing::warn!(
                artifact = owner.name(),
                platform = owner.platform(),
                %error,
                "cannot probe the tree lock; keeping its temp residue"
            );
        }
        if !temp_residue_is_reclaimable(&lock) {
            return;
        }
        for entry in entries {
            if !is_resumable(entry, owner, self.pins) {
                self.remove(&self.held.paths().root().join(entry), reclaimed);
            }
        }
    }

    fn lock_of(&self, owner: &Artifact) -> OwnerLock {
        if owner == self.held.artifact() {
            return OwnerLock::HeldByCaller;
        }
        match probe_single_flight(&self.held.paths().single_flight_lock_of(owner)) {
            Ok(SingleFlightProbe::Taken(guard)) => OwnerLock::Taken { _guard: guard },
            Ok(SingleFlightProbe::Absent) => OwnerLock::Absent,
            Ok(SingleFlightProbe::Held) => OwnerLock::HeldElsewhere,
            Err(error) => OwnerLock::Unprobeable(error),
        }
    }
}
```

The `paths` field goes. `reap_orphans`, `reap_generations` and `remove`
read the root only through `self.held.paths()`, so the reap cannot scan a
root other than the one its lock was taken under.

`reap_for_y_at` now takes `HeldTreeLock::take(&paths, y())` on the test
thread, moves it into the bounded run, and builds `Reaper::new(&held,
..)` from it. On a local filesystem, AC7 then also proves that the reap
does not probe its own lock: such a probe opens a second file description
and would see `Held`. Where `flock` is emulated with process-owned POSIX
locks, the same probe would succeed, and dropping its guard would release
the caller's lock. `lock_of`'s `HeldByCaller` short-circuit is what
prevents that, not the probe.

Confirm that each over-correction guard can fail, using local changes
that are not committed:

- mapping `Absent` to keep fails AC5;
- swapping `try_single_flight` in for the probe fails AC5's check on the
  lock-file set;
- comparing names only in `lock_of` fails AC8;
- mapping another owner with the same name to `HeldElsewhere` fails AC9.

#### 5. Callers

**File**: `resolver.rs`

- **`materialise`**: build the `Artifact` once, before the bounded-waiter
  loop. `try_single_flight(&lock_path)` becomes
  `HeldTreeLock::try_take(&paths, artifact.clone())`. Remove the
  `lock_path` binding (`:576`) and the `try_single_flight` import (`:27`).
- **`prune` and `repair`**: `take_single_flight(..)` becomes
  `HeldTreeLock::take(..)`. Every tree lock in the resolver is then taken
  through `HeldTreeLock`, and `take_single_flight` leaves the resolver's
  imports.
- **Both reaps**: `Reaper::new(&held, self.clock, self.liveness,
  &self.expected_digests).reap_orphans()`.

#### 6. Docs and the 0218 invariant

Rewrite the module doc's second paragraph (`reap.rs:4-6`). The new
paragraph says:

- The reap runs under its own artifact's lock, which `HeldTreeLock`
  proves.
- Another artifact's temp residue goes only under that artifact's lock,
  taken without blocking, or when no lock file exists.
- "No lock file" is safe only while lock files are unlinked solely by a
  process that holds them, and every process that takes the lock confirms
  its inode afterwards. A launcher that does not confirm can hold an
  orphaned inode while another process holds the new file, so neither
  lock excludes the other.
- Holding another artifact's lock while removing a large tree can make its
  bounded waiter report materialisation in progress, which is not sticky.
- In the `Absent` arm, nothing excludes a launcher that creates the lock
  and reopens an existing partial archive between the reap's scan and its
  removal. Such a launcher must be one that pins the same artifact. Its
  appends go to an unlinked file, and its materialise fails with
  `Extraction`. That cause is sticky for the marker TTL in the session.
  It is also in the Refusal class, so it stops a `--fail-safe` crawl
  rather than degrading it. The window needs a launcher of another
  version that still pins a retired artifact, and any future removal of
  lock files inherits it.
- A process must never probe or retake a tree lock it already holds.
  Where `flock` is emulated with process-owned POSIX locks, the second
  lock succeeds, and closing its descriptors releases the first.

Keep this invariant only here: `probe_single_flight`'s doc says only what
the probe does.

Make the matching correction to `prune`'s docstring.

**File**: `meta/work/0218-bound-cache-root-growth.md`

- **Dependencies**: add a line. Orphan-lock removal must unlink a lock
  file only while holding it. It is safe against concurrent materialises
  only for launchers from 0339 onwards, which confirm the inode after
  locking.
- **Assumptions** (`:524-526`): replace the accepted race with an older
  launcher with a statement of its consequence. A launcher that does not
  confirm its inode can lock an orphaned inode after the lock file is
  unlinked, so it no longer excludes a launcher that holds the recreated
  file. While both hold a lock, the following can happen:
  - any reap can remove that launcher's in-progress residue, through the
    `Absent`, `Taken` or own-artifact arms;
  - a concurrent materialise of the same artifact can append to the same
    partial archive and corrupt it.

  Each consequence records a sticky marker for that artifact, which lasts
  for the marker TTL in the session. Its cause, `UnexpectedDigest` or
  `Extraction`, is in the Refusal class, so it also stops a `--fail-safe`
  crawl rather than degrading it. 0218 must either accept this by
  name, or remove lock files only once no launcher without the inode check
  shares the root.

### Success Criteria:

#### Automated Verification:

- [ ] AC1, AC2, AC6, AC6′, AC8, AC10 and AC12 are observed failing for a behavioural reason before item 4, with `CI=1` and `minisign` on `PATH`
- [ ] AC5 and AC9 pass before and after item 4
- [ ] Reap tests pass: `cargo test --manifest-path cli/Cargo.toml -p accelerator --lib tree::`
- [ ] AC12 tests pass: `CI=1 cargo test --manifest-path cli/Cargo.toml -p accelerator --test tree_resolution survives_this_ones`
- [ ] Full launcher suite passes: `CI=1 cargo test --manifest-path cli/Cargo.toml -p accelerator`
- [ ] `mise run check` exits 0
- [ ] `mise run` (bare default task) exits 0 end-to-end

#### Manual Verification:

- [ ] Each over-correction mutation in item 4 fails the guard named for
      it, then is reverted
- [ ] `cache prune` on a populated real cache root still reports reclaimed
      entries, and leaves the `browser` and `driver` lock files in place

---

## Testing Strategy

### Unit Tests:

- `testing.rs`: a run that outlasts its bound fails rather than hanging,
  and a warning raised inside a bounded run reaches the caller's capture.
- `layout.rs`:
  - ownership for every entry kind, including hyphenated names and
    platforms, names that extend, contain or end in a platform, platforms
    that extend a known one, unparseable names and lock files;
  - every pinned pair round-tripping through every builder;
  - literal file names.
- `lease.rs`:
  - placement for a file still at its path, unlinked or replaced;
  - the shared lock loop, with counted opens:
    - retrying onto a replacement;
    - reporting `Absent` after an unlink;
    - reporting and warning `KeptBeingReplaced` after `MAX_LOCK_ATTEMPTS`;
    - failing on an identity error after one open;
  - a confirmed lock keeping its confirming descriptor;
  - every cell of `take_outcome`, `try_outcome` and `probe_outcome`;
  - `is_displacement` for `NotFound`, `ESTALE`, `EACCES` and `ENOTDIR`;
  - `take_single_flight` locking a file that replaces the one it waited on;
  - the four probe outcomes, each bounded.
- `fetcher.rs`:
  - a 416 whose total equals the range start is `AlreadyComplete`;
  - a shorter or longer total, no total, or a 206 starting elsewhere is
    `RangeRejected`;
  - a 200 to a ranged request is `Whole`, with a truncating sink;
  - a sink that cannot be opened is not retried.
- `tree_lock.rs`: `take`, plus `try_take` when the lock is free and when it
  is held, each bounded.
- `download.rs`: a partial that cannot be discarded is warned.
- `download.rs`:
  - `PartialArchive::assess` for every length class, and
    `additional_bytes` for each assessment;
  - an append sink over a file shorter than its offset fails without
    extending the file.
- `reap.rs`:
  - one test per disposition cell;
  - every counted removal having been observed;
  - a temp pointer still protecting its generation;
  - AC1, AC2 and AC4–AC11, plus AC6′, through `reap_for_y_at`. That
    helper encodes Y's lock held by its caller, the 10s bound and an
    unchanged set of lock files.

### Integration Tests:

These run with `CI=1` and `minisign` on `PATH`, and the mock answers 416
past the end.

- Partial archives:
  - a complete partial is verified without refetching;
  - one under a loose size bound is confirmed by the server's 416;
  - one longer than the served file is refetched whole after the 416;
  - an over-long one is refetched whole;
  - a resumed download that fails mid-body retries from the agreed offset;
  - a corrupt prefix, a corrupt complete file, or an unreadable partial is
    discarded so the next run recovers;
  - an unwritable short partial is replaced within the same run;
  - the step-2 manifest refusal leaves a planted partial in place;
  - a partial whose signature fails is kept for the next run, under a
    manifest re-signed by the harness;
  - the free-space check counts the bytes the partial already holds.
- AC3 and AC12, for both `materialise` and `prune`. The bystander is
  pinned to a distinct digest and its lock file exists. `within_bound`
  asserts the set of lock files is unchanged.
- `cache prune` reclaims owned orphans and keeps unowned ones.

### Manual Testing Steps:

1. Apply each over-correction mutation, confirm that its guard fails, then
   revert it.
2. Run `accelerator cache prune` against a real cache root. Confirm that
   the lock files and any pinned partial archives remain.

## Performance Considerations

⏱️ Each reap adds one `open`, one `flock` and two `stat` calls for every
other owner present in `trees/`. That is at most a handful today. Taking
a tree lock adds one `open` and two `fstat` calls, and the guard holds a
second descriptor until it releases the lock. A complete partial now costs
one local re-hash of up to about 120MB, plus one range request when the
manifest size is a loose bound, instead of a failed network request.

## Migration Notes

The fix creates no on-disk state. Old launchers sharing a root still reap
unconditionally until they are replaced, and they do not confirm the
inode after locking.

⚠️ Every launcher leaves a complete partial at the pinned digest when a
later step of materialise fails, such as the attestation fetch. A
launcher from before 0339 that resumes that file gets a 416, treats it as
retryable, and spends its whole retry deadline. It then records a sticky
`Unreachable` marker for that artifact, which lasts for the marker TTL in
the session. `Unreachable` is Failed-class, so a `--fail-safe` crawl
degrades rather than stopping.
Today such a partial is cleared by accident, when a reap for a different
artifact runs. A reap for the same artifact spares it, because that
artifact's own pinned digest is in the keep set. 0339 launchers stop
clearing it by accident.

The partial is reliably cleared in two ways:
- a launcher from 0339 onwards materialises that artifact;
- someone deletes `trees/.tmp-<name>-<platform>-<digest>.archive`.

⚠️ `cache prune` no longer removes `.tmp-` entries with no known owner.
That covers entries from newer launchers on platforms this one does not
know. Clearing them is the job of 0218's root prune. Every shipped
launcher uses today's `.tmp-` names (`layout.rs` unchanged since
`d06ec4db`), so no older residue becomes unowned.

## References

- Work item: `meta/work/0339-tree-reap-deletes-another-artifact-s-in-progress-materialise.md`
- Research: `meta/research/codebase/2026-10-16-0339-tree-reap-cross-artifact-lock-probe.md`
- Prior plan: `meta/plans/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`
- Review: `meta/reviews/plans/2026-10-16-0339-tree-reap-cross-artifact-lock-probe-review-1.md`
- Decision: `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
- Dependent: `meta/work/0218-bound-cache-root-growth.md`
- Non-creating probe precedent: `cli/lease/src/lib.rs:225-241`
- In-process lock holding: `cli/launcher/tests/tree_resolution.rs:620-647`
