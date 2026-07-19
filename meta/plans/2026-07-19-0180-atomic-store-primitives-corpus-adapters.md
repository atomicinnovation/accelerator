---
type: plan
id: "2026-07-19-0180-atomic-store-primitives-corpus-adapters"
title: "Atomic-Store Primitives in corpus-adapters Implementation Plan"
date: "2026-07-19T00:41:09+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0180"
parent: "work-item:0180"
blocked_by: ["work-item:0179"]
derived_from: ["codebase-research:2026-07-19-0180-atomic-store-primitives-corpus-adapters"]
relates_to: ["work-item:0166", "work-item:0172"]
tags: [rust, corpus, corpus-adapters, atomic-store, jsonl, store]
revision: "8306bd0b581b2faa05410d735f16eb03cd98ddfa"
repository: "accelerator"
last_updated: "2026-07-19T09:16:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Atomic-Store Primitives in corpus-adapters Implementation Plan

## Overview

Port the load-bearing atomic-store concurrency primitives from the bash library
into the Rust `corpus`/`corpus-adapters` hexagon: `atomic_write` (same-directory
temp file + atomic rename), the mkdir-based lock (PID-owner reclaim, jittered
back-off, acquisition ceiling), and canonical-order JSONL compose /
remove-by-key. The domain crate `corpus` declares the error taxonomy and the two
driven ports; `corpus-adapters` supplies the filesystem infrastructure. Every
primitive is delivered test-first.

## Current State Analysis

The primitives live in bash today:

- `scripts/atomic-common.sh:16-32` — `atomic_write`: `mkdir -p` the target's
  directory, `mktemp` a temp *in that same directory*, buffer stdin to it, single
  `mv`, EXIT-trap cleanup of the orphan on interruption before the rename. No
  fsync, no permission preservation.
- `scripts/atomic-common.sh:105-163` — the mkdir-based lock: a writability
  pre-check, per-call RANDOM re-seed, jittered exponential back-off (base 4 ms
  doubling to a ~256 ms cap, jitter uniform `1..=base`), a 300 s ceiling on
  accumulated slept-ms, an `owner` sentinel holding the holder PID, and
  `_atomic_lock_reclaim_if_stale` which reclaims a lockdir only once its owner PID
  is confirmed dead (missing/empty owner is treated as *live*).
- `scripts/atomic-common.sh:200-246` — subshell-owned EXIT-trap release and the
  anchored-prefix `atomic_jsonl_remove_by_key` (drop lines where the
  `{"transformation_key":"…",` prefix starts at byte 0).
- `scripts/jsonl-common.sh:21-149` — `jsonl_json_escape` (backslash-first escape
  ordering, control chars as `\u00xx`) and `jsonl_compose_record` (canonical
  field order: `transformation_key`, `schema_version`, `outcome`,
  `proposed_value`, optional `user_value`, `timestamp`, then author extras).

The Rust target crates exist (created by 0179). `corpus`
(`cli/corpus/Cargo.toml`) is dependency-free; its ports (`Clock`, `IdScanner`)
are infallible. `corpus-adapters` holds the imperative shell and a `bash-parity`
feature gate for differential suites. Neither has any store primitive yet. The
pure byte-transform half of persistence already exists
(`corpus-adapters/src/patcher.rs`, `assemble.rs`); `atomic_write` is the missing
I/O half.

## Desired End State

`corpus-adapters` provides atomic writes, the mkdir-based lock with PID-owner
reclaim and jittered back-off, and canonical-order JSONL compose / remove — all
behind driven-port traits declared in `corpus`, all with deterministic tests.
`mise run cli:check` and the `corpus-adapters` test suite pass on **both darwin
(dev) and the linux CI runner**; the new `tempfile`/`libc`/`rand` dependencies
land cleanly under `cargo-deny` and `cargo-pup`. The primitives use only POSIX
constructs (mkdir-as-mutex, `libc::kill`, `rename`, `EXDEV`), matching the
`cli/` target set (apple-darwin + `*-unknown-linux-musl`); musl **runtime**
behaviour of `libc::kill` and `NamedTempFile` remains unverified until the cli
cross-compile build lands (only the dependency graph is musl-checked by
`cargo-deny`), so the lock and the cross-filesystem branch are deliberately
POSIX-only and not portable to a future Windows target without rework.

Verification: `cd cli && cargo test -p corpus -p corpus-adapters` is green on
darwin and on the linux CI runner, and `mise run cli:check` (workspace rustfmt +
clippy + the pup/deny guards) exits 0.

### Key Discoveries

- `corpus` currently carries **no** `kernel` dependency — the `Cargo.toml`
  comment (`cli/corpus/Cargo.toml:12-16`) notes the pup rule permits
  `kernel::Error` "but nothing uses it yet, so the dependency is not carried."
  `StoreError`'s `From<StoreError> for kernel::Error` is the first use and adds
  the dep.
- The pup rule `corpus_domain_imports_only_permitted` (`cli/pup.ron:57-72`)
  restricts every `use` in `corpus` to `^(std|core|alloc)`, `^kernel::Error`,
  `^crate`. There is **no** pup rule for `corpus-adapters`, so `serde_json`,
  `tempfile`, `libc`, and `rand` all belong there, not in the domain.
- `cli/deny.toml` is a **denylist**, not an allow-list. `tempfile`/`libc`/`rand`
  are not banned; their `MIT OR Apache-2.0` licenses are already permitted. The
  one hard constraint is `wildcards = "deny"` (`cli/deny.toml:57`) — caret ranges
  like `"3"`/`"0.2"`/`"0.9"` are fine; only a literal `"*"` fails. `libc` and
  `rand` are already in the lock transitively; only `tempfile` grows the graph.
- `config`'s `ReadConfigLevel`/`WriteConfigLevel`
  (`cli/config/src/service.rs:29-45`) are the closest port precedent: a fallible
  reader+writer pair behind a document-persisting service, with fake ports
  (`FakeReader`/`FakeWriter`, `:255-297`) using a `Failing` state and an
  `Rc<RefCell<…>>` recorder.
- `metadata::render` puts serialization/rendering in the **adapter** while the
  `ArtifactMetadata` fact and the `Clock` port stay in the **domain**. The
  canonical JSONL byte-layout is rendering, so `compose_record` and its
  serde_json escaper live in `corpus-adapters`; `corpus` owns only the `Record`
  type and the ports.
- `file_driver.rs:187-262` is the `NamedTempFile::new_in(&parent)` + `persist` +
  `libc::EXDEV` fail-closed reference. `NamedTempFile`'s dropped temp auto-cleans
  on any error — this is the RAII replacement for the bash EXIT trap.
- The `serde_json` string serializer diverges from `jsonl_json_escape` only at
  `0x7F` (DEL: bash → ``, serde_json → raw byte). Both are valid JSON and
  both round-trip; the clean-cutover ruling makes serde_json acceptable provided
  the *same* escaper drives compose and remove.

## What We're NOT Doing

- **No standalone `store` crate** — 0166 locked the fold into `corpus-adapters`.
- **No `bash-parity` differential suite** for these primitives. The clean-cutover
  ruling makes the Rust→Rust round-trip (AC-6) the full parity requirement; there
  is no bash oracle to diff against.
- **No fsync and no permission preservation** in `atomic_write` — the bash source
  and both `cli/` precedents omit them; only the async server driver fsyncs, and
  that layer is out of scope. This is a conscious durability-vs-atomicity trade:
  a reader never sees a partial file (atomicity holds), but a crash immediately
  after the rename can still lose the just-written data, and without a parent-dir
  fsync some filesystems can lose the renamed entry entirely on power loss. For a
  re-runnable dev-workflow session log this is acceptable; a future
  crash-durability consumer must fsync the file and parent dir as
  `file_driver.rs` does. Orphan temp files left in the target directory after a
  SIGKILL/OOM between `stage` and `persist` (the same limitation as the bash EXIT
  trap) are expected and safe to delete.
- **No async/tokio, no per-path mutex, no SHA-256 etag** — the whole `cli/`
  workspace is synchronous `std::fs`; we take only the same-dir-temp + rename +
  EXDEV-fail-closed shape from `file_driver.rs`.
- **No public lock port** — the lock has zero external callers and stays private
  to the adapter.
- **No `atomic_append_unique` / `atomic_remove_line`** (the non-JSONL line
  helpers, `atomic-common.sh:38-80`) — out of this work item's scope; only
  `atomic_write` and the JSONL record ops are ported.
- **No cross-field `outcome`↔`user_value` coupling** in the primitive —
  `user_value` emission is presence-based; the coupling is the migrate
  consumer's concern (0172).

## Implementation Approach

Three phases, each independently mergeable and green under `mise run cli:check`:

1. Declare the domain contracts (`StoreError`, `Record`, the two ports) in
   `corpus`. No infrastructure, so it lands with only the `kernel` dep.
2. Implement `atomic_write` and the `AtomicWrite` port in `corpus-adapters`,
   adding `tempfile`/`libc`. A `stage`/`persist` split is the deterministic
   fault-injection seam for the interruption invariant.
3. Implement the mkdir-lock, the JSONL compose/remove, and the `RecordStore`
   port in `corpus-adapters`, adding `rand`/`serde_json`. The lock's acquisition
   ceiling and liveness check are injected so the contended-lock and
   dead-holder-reclaim tests are deterministic and never wait 300 s.

Test-driven throughout: each behaviour gets its failing test before the code.
Tests return `Result<(), _>` and use `?` — never `unwrap`/`expect`/`panic` — to
satisfy the workspace clippy lints (`cli/Cargo.toml:47-59`, `warnings = "deny"`).

## Phase 1: Domain contracts in `corpus`

### Overview

Add the store error taxonomy, the record data types, and the two driven-port
traits to the `corpus` domain crate. No filesystem code; this phase defines the
contract the adapter implements in Phases 2–3.

No Rust consumer exists yet (the callers land in 0170/0172/0168), so the
`RecordStore` method shapes are provisional: sanity-check them against 0172's
concrete session-log access pattern before treating them as frozen. `AtomicWrite`
is the broadly-reused, low-risk half and can be considered stable.

### Changes Required

#### 1. `StoreError` and the two ports

**File**: `cli/corpus/src/store.rs` (new)
**Changes**: A hand-written error enum mirroring `ConfigError`'s shape
(`Display` + bare `impl Error` + `From` for the kernel boundary), plus the two
port traits. `#[non_exhaustive]` and `PartialEq, Eq` so tests assert exact
values.

```rust
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;

use crate::record::Record;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotWritable { path: String },
    LockTimeout { path: String },
    CrossFilesystem { path: String },
    Validation { detail: String },
    Io { path: String, detail: String },
}

impl Display for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWritable { path } => {
                write!(formatter, "cannot write under '{path}': not writable")
            }
            Self::LockTimeout { path } => {
                write!(formatter, "lock acquisition timed out on '{path}'")
            }
            Self::CrossFilesystem { path } => write!(
                formatter,
                "atomic rename to '{path}' crossed a filesystem boundary"
            ),
            Self::Validation { detail } => {
                write!(formatter, "invalid record: {detail}")
            }
            Self::Io { path, detail } => {
                write!(formatter, "I/O error on '{path}': {detail}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

impl From<StoreError> for kernel::Error {
    fn from(error: StoreError) -> Self {
        Self::Failed(error.to_string())
    }
}

pub trait AtomicWrite {
    /// # Errors
    /// [`StoreError`] when the destination directory is not writable, the rename
    /// crosses a filesystem boundary, or the write fails.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError>;
}

pub trait RecordStore {
    /// # Errors
    /// [`StoreError`] on validation failure, lock-acquisition timeout, or I/O.
    fn append_record(
        &self,
        path: &Path,
        record: &Record,
    ) -> Result<(), StoreError>;

    /// # Errors
    /// [`StoreError`] on lock-acquisition timeout or I/O.
    fn remove_by_key(&self, path: &Path, key: &str)
        -> Result<(), StoreError>;
}
```

#### 2. The record data types

**File**: `cli/corpus/src/record.rs` (new)
**Changes**: The pure record model the composer renders. `Outcome` is the
three-value enum; `proposed_value` is a required `String` (emptiness is rejected
by the composer in Phase 3); `user_value` is `Option<String>` (presence-based);
`extras` is an ordered `Vec<(String, String)>`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Accepted,
    Edited,
    Skipped,
}

impl Outcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Edited => "edited",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub transformation_key: String,
    pub schema_version: u32,
    pub outcome: Outcome,
    pub proposed_value: String,
    pub user_value: Option<String>,
    pub timestamp: String,
    pub extras: Vec<(String, String)>,
}
```

#### 3. Wire the modules and the dependency

**File**: `cli/corpus/src/lib.rs`
**Changes**: Add `pub mod record;` / `pub mod store;` and the flat re-exports
(`Record`, `Outcome`, `StoreError`, `AtomicWrite`, `RecordStore`), matching the
existing `pub use` block.

**File**: `cli/corpus/Cargo.toml`
**Changes**: Add `kernel = { path = "../kernel" }` and update the comment that
currently states the kernel dependency "is not carried."

### Success Criteria

#### Automated Verification

- [x] `corpus` compiles with the new modules: `cd cli && cargo build -p corpus`
- [x] `StoreError` `Display` + kernel-mapping tests pass for **all five**
      variants (`NotWritable`, `LockTimeout`, `CrossFilesystem`, `Validation`,
      `Io`), so `CrossFilesystem` — whose runtime path is only inspection-verified
      in Phase 2 — is exercised here rather than being dead in tests:
      `cd cli && cargo test -p corpus store`
- [x] `Outcome::as_str` test passes: `cd cli && cargo test -p corpus record`
- [x] Clippy and the pup domain-import guard pass:
      `mise run lint:cli:check && mise run pup:check`
- [x] Workspace formatting is clean: `mise run format:cli:check`

#### Manual Verification

- [x] `corpus/Cargo.toml`'s dependency comment reflects that `kernel` is now
      carried for the `StoreError` boundary mapping.

---

## Phase 2: `atomic_write` and the `AtomicWrite` port

### Overview

Implement the same-directory-temp + atomic-rename write in `corpus-adapters` and
the `AtomicWrite` port over it. The `stage`/`persist` split is the deterministic
fault-injection seam the interruption invariant is tested through.

### Changes Required

#### 1. The atomic write and its seam

**File**: `cli/corpus-adapters/src/store.rs` (new)
**Changes**: A `pub(crate)` `atomic_write` composed of a private `stage` (create
parent, temp-in-parent, write bytes) and `persist` (rename, delegating error
classification to `classify_persist_error` so the EXDEV fail-closed branch is
unit-testable without a real cross-mount). The `FileCorpusStore` unit struct
(named for symmetry with `config-adapters`' `FileConfigStore`) implements
`AtomicWrite` and carries a `Default` impl so it clears
`clippy::new_without_default` under `warnings = "deny"`. `NamedTempFile`'s drop
cleans the orphan on any early return — the RAII replacement for the EXIT trap.
The free `atomic_write` is `pub(crate)`: cross-crate consumers write through the
`AtomicWrite` port, not the concretion, and keeping it unexported also keeps it
out of `missing_errors_doc`'s scope (only the public trait method needs the
`# Errors` section, which it has). `io`/`show` are the shared error-mapping
helpers used across both phases.

```rust
use std::fs;
use std::io::Error as IoError;
use std::io::Write as _;
use std::path::Path;

use corpus::{AtomicWrite, StoreError};
use tempfile::NamedTempFile;

pub struct FileCorpusStore;

impl FileCorpusStore {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for FileCorpusStore {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let staged = stage(path, bytes)?;
    persist(staged, path)
}

fn stage(path: &Path, bytes: &[u8]) -> Result<NamedTempFile, StoreError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
    }
    let dir = parent.unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            StoreError::NotWritable { path: show(dir) }
        } else {
            io(dir, &error)
        }
    })?;
    temp.write_all(bytes).map_err(|error| io(dir, &error))?;
    Ok(temp)
}

fn persist(temp: NamedTempFile, path: &Path) -> Result<(), StoreError> {
    temp.persist(path)
        .map(|_| ())
        .map_err(|error| classify_persist_error(path, &error.error))
}

fn classify_persist_error(path: &Path, error: &IoError) -> StoreError {
    if error.raw_os_error() == Some(libc::EXDEV) {
        StoreError::CrossFilesystem { path: show(path) }
    } else {
        io(path, error)
    }
}

fn show(path: &Path) -> String {
    path.display().to_string()
}

fn io(path: &Path, error: &IoError) -> StoreError {
    StoreError::Io { path: show(path), detail: error.to_string() }
}

impl AtomicWrite for FileCorpusStore {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        atomic_write(path, bytes)
    }
}
```

#### 2. Dependencies

**File**: `cli/Cargo.toml`
**Changes**: Add caret-pinned `tempfile = "3"` and `libc = "0.2"` to
`[workspace.dependencies]`.

**File**: `cli/corpus-adapters/Cargo.toml`
**Changes**: Add `tempfile = { workspace = true }` and
`libc = { workspace = true }`.

**File**: `cli/corpus-adapters/src/lib.rs`
**Changes**: `pub mod store;` and re-export `FileCorpusStore` (the port impl).
The free `atomic_write` stays `pub(crate)` and is **not** re-exported — consumers
go through the `AtomicWrite` port.

### Success Criteria

#### Automated Verification

- [x] `corpus-adapters` compiles: `cd cli && cargo build -p corpus-adapters`
- [x] Atomic-write tests pass: `cd cli && cargo test -p corpus-adapters store`
- [x] `cargo-deny` admits the new deps: `mise run deny:check`
- [x] Clippy and pup pass: `mise run lint:cli:check &&
      mise run pup:check`
- [x] Full single-component gate: `mise run cli:check`

Tests to write first (all inline `#[cfg(test)]` in `store.rs`, so they reach the
private `stage`/`persist` seam):

- The temp is created in the **target's** directory, and a successful write
  leaves **no stray temp** (list the dir afterwards, as
  `a_successful_write_leaves_no_stray_temp` does). Each test uses a unique
  working directory via `tempfile::tempdir()` (RAII-cleaned) so the suite is
  robust under cargo's parallel runner and the runner's temp layout (macOS
  private `/var/folders` vs a shared linux `/tmp`).
- A write through the public `AtomicWrite::write` **replaces** existing
  destination content via a single rename (pins the composition, not just the
  `stage`/`persist` helpers).
- A write **creates the parent directory** when absent (`mkdir -p` parity).
- **Interruption before the rename** (call `stage`, drop the returned temp
  without `persist`): seed the destination with known bytes first and assert
  those exact bytes are **unchanged** afterwards; and, on the fresh-path variant,
  assert the destination is **still absent** — never a partial destination, and
  no stray temp. This is the AC-2 fault-injection seam.
- `classify_persist_error` maps a synthetic
  `std::io::Error::from_raw_os_error(libc::EXDEV)` to
  `StoreError::CrossFilesystem` and any other errno to `StoreError::Io`,
  giving the AC-2 cross-filesystem branch deterministic coverage without a real
  cross-mount.

#### Manual Verification

- [x] AC-2's end-to-end cross-mount rename is confirmed only by inspection
      against the `file_driver.rs:231-242` precedent (a genuine cross-mount is
      not portable in CI); the errno-classification logic itself is unit-covered
      via `classify_persist_error`.

---

## Phase 3: The lock, the JSONL composer, and the `RecordStore` port

### Overview

Implement the private mkdir-lock (RAII guard, single-winner PID-owner reclaim,
jittered back-off, injectable ceiling and liveness), the canonical-order JSONL
composer and its serde_json escaper, and the `RecordStore` port that wires them
over `atomic_write`. The lock (mkdir-as-mutex, `libc::kill` liveness) is
deliberately POSIX-only, matching the bash source and the `cli/` darwin+musl
target set.

### Changes Required

#### 1. The mkdir-based lock

**File**: `cli/corpus-adapters/src/lock.rs` (new)
**Changes**: `LockOptions` (default `ceiling_ms = 300_000`, `base_ms = 4`,
`cap_ms = 256`) makes the ceiling injectable (AC-5). (The shared `_ms` field
suffix may trip pedantic `clippy::struct_field_names` under `warnings = "deny"`;
if it fires, resolve with a scoped `#[allow(clippy::struct_field_names)]` — the
suffix names the unit and is worth keeping.) A private
`acquire_with(lockdir, opts, is_alive)` takes the liveness predicate by injection
so the reclaim tests never depend on real PIDs; the public `acquire` supplies the
`libc::kill`-backed predicate. `LockGuard`'s `Drop` removes the lockdir
(`remove_dir_all`, ignoring errors — the `rm -rf` parity, because the lockdir
holds the `owner` sentinel). Acquisition distinguishes `AlreadyExists` (contended
→ back off) from a `PermissionDenied` error (fail fast as `NotWritable`, the
writability pre-check without a separate probe) from any other `create_dir`
error (fail fast as `Io`, preserving the underlying errno detail rather than
mislabelling e.g. `ENOSPC`/`ENOTDIR` as "not writable"). Two hardening changes
over the bash source: `claim` **propagates** an owner-write failure — releasing
the just-created lockdir and returning `Io` rather than leaving a permanently
un-reclaimable lockdir with no owner sentinel — and `reclaim_if_stale` is
**single-winner**: it renames the stale lockdir to a per-process discard name and
only the process whose atomic `rename` wins re-checks the owner and removes it,
so two waiters that both observed the same dead holder cannot both `remove_dir_all`
and cascade into deleting a freshly re-acquired lock.

```rust
use std::fs;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use corpus::StoreError;

#[derive(Debug, Clone, Copy)]
pub struct LockOptions {
    pub ceiling_ms: u64,
    pub base_ms: u64,
    pub cap_ms: u64,
}

impl Default for LockOptions {
    fn default() -> Self {
        Self { ceiling_ms: 300_000, base_ms: 4, cap_ms: 256 }
    }
}

pub struct LockGuard {
    lockdir: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.lockdir);
    }
}

pub fn acquire(
    lockdir: &Path,
    opts: LockOptions,
) -> Result<LockGuard, StoreError> {
    acquire_with(lockdir, opts, process_is_alive)
}

fn acquire_with(
    lockdir: &Path,
    opts: LockOptions,
    is_alive: impl Fn(i32) -> bool,
) -> Result<LockGuard, StoreError> {
    let mut waited_ms = 0u64;
    let mut base_ms = opts.base_ms;
    loop {
        match fs::create_dir(lockdir) {
            Ok(()) => return claim(lockdir),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                reclaim_if_stale(lockdir, &is_alive);
                if waited_ms > opts.ceiling_ms {
                    return Err(StoreError::LockTimeout {
                        path: lockdir.display().to_string(),
                    });
                }
                let jitter = jitter_ms(base_ms);
                std::thread::sleep(Duration::from_millis(jitter));
                waited_ms += jitter;
                if base_ms < opts.cap_ms {
                    base_ms = (base_ms * 2).min(opts.cap_ms);
                }
            }
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                return Err(StoreError::NotWritable {
                    path: lockdir.display().to_string(),
                });
            }
            Err(error) => {
                return Err(StoreError::Io {
                    path: lockdir.display().to_string(),
                    detail: error.to_string(),
                });
            }
        }
    }
}

fn claim(lockdir: &Path) -> Result<LockGuard, StoreError> {
    let owner = std::process::id().to_string();
    if let Err(error) = fs::write(lockdir.join("owner"), owner) {
        let _ = fs::remove_dir_all(lockdir);
        return Err(StoreError::Io {
            path: lockdir.display().to_string(),
            detail: error.to_string(),
        });
    }
    Ok(LockGuard { lockdir: lockdir.to_path_buf() })
}

fn reclaim_if_stale(lockdir: &Path, is_alive: &impl Fn(i32) -> bool) {
    let Some(pid) = dead_owner(lockdir, is_alive) else {
        return;
    };
    let discard = discard_path(lockdir);
    if fs::rename(lockdir, &discard).is_err() {
        return;
    }
    if dead_owner(&discard, is_alive) == Some(pid) {
        let _ = fs::remove_dir_all(&discard);
    }
}

/// The owner PID only when it is present, parseable, and confirmed dead. A
/// missing, empty (holder mid-acquisition), or unparseable `owner` file yields
/// `None` — treated as a *live* holder, so PID-reuse or the acquisition window
/// can never break a genuinely held lock (AC-3/AC-4).
fn dead_owner(lockdir: &Path, is_alive: &impl Fn(i32) -> bool) -> Option<i32> {
    let owner = fs::read_to_string(lockdir.join("owner")).ok()?;
    let pid = owner.trim().parse::<i32>().ok()?;
    (!is_alive(pid)).then_some(pid)
}

fn discard_path(lockdir: &Path) -> PathBuf {
    let nonce = rand::random::<u64>();
    let mut name = lockdir.as_os_str().to_owned();
    name.push(format!(".{}.{nonce:x}.reclaim", std::process::id()));
    PathBuf::from(name)
}

/// `kill(pid, 0)`: `0` → alive; `EPERM` → alive (exists but not signalable by
/// this process); `ESRCH` → gone. Only a confirmed `ESRCH` is treated as dead.
fn process_is_alive(pid: i32) -> bool {
    // SAFETY: `kill` takes two scalar arguments and returns a scalar; it
    // dereferences no pointers and cannot violate memory safety.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    IoError::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn jitter_ms(base_ms: u64) -> u64 {
    rand::rng().random_range(1..=base_ms.max(1))
}
```

Deliberate divergences from bash, all hardening the "never break a live lock"
invariant (AC-3): a missing/empty/unparseable owner and an `EPERM` from
`kill(pid, 0)` are all treated as live; the owner-write failure releases rather
than orphaning the lockdir; and reclaim is single-winner (the per-attempt `nonce`
in `discard_path` guarantees a leftover discard can never wedge a later reclaim
with `ENOTEMPTY`). One **residual** race remains and is inherent to mkdir-lock
reclaim (it exists in the bash source too, more crudely): the window between
`dead_owner` returning `Some` and the `rename` is an ordinary scheduling gap
(potentially milliseconds), so a reclaimer can `rename` a lockdir that a live
holder recreated in that gap. Its post-rename `dead_owner` re-check finds the
holder alive and declines to remove the discard, but the original path is now
free — so a fresh waiter can acquire concurrently with the displaced holder (a
possible lost update on the read-modify-write), and the displaced holder's `Drop`
can then remove the new holder's lockdir. This requires a holder to die
mid-operation **and** two or more contending waiters; the blast radius is one
re-runnable session-log record. It is **acceptable, not eliminated**, under the
clean-cutover usage (one writer process per file at a time — see Migration
Notes); a fully race-free reclaim would need an atomic compare-and-remove the
filesystem does not offer. The multi-waiter reclaim stress test below exercises
the common single-winner path.

#### 2. The JSONL composer and remove prefix

**File**: `cli/corpus-adapters/src/jsonl.rs` (new)
**Changes**: `compose_record(&Record) -> Result<String, StoreError>` renders the
canonical field order manually, escaping every string value through one
`escape_value` (serde_json). Both `compose_record` and `remove_prefix` emit the
anchored opener through the **single** `record_opener` helper, so the writer and
the remover cannot drift on the `{"transformation_key":"<esc>",` bytes — the
load-bearing parity requirement is enforced structurally rather than by two
hand-kept-in-sync literals. `user_value` is emitted **presence-based** (iff
supplied); the cross-field `outcome=edited` coupling is deliberately the migrate
consumer's concern (0172) — see "What We're NOT Doing". Field-level validation
rejects an empty `transformation_key`, `proposed_value`, or `timestamp` (AC-8
plus the other bash-required fields; `schema_version` and `outcome` are
type-guaranteed non-empty) and validates extras keys against the bash
`^[a-z][a-z0-9_]*$` field-name contract. Both functions are plain `pub fn` inside
the **private** `jsonl` module (matching `lock::acquire`): they are unreachable
outside the crate, so `missing_errors_doc` does not fire, and plain `pub` avoids
`clippy::redundant_pub_crate`, which flags `pub(crate)` inside a private module.

```rust
use corpus::{Record, StoreError};

const RESERVED: [&str; 6] = [
    "transformation_key", "schema_version", "outcome",
    "proposed_value", "user_value", "timestamp",
];

fn escape_value(value: &str) -> Result<String, StoreError> {
    let quoted = serde_json::to_string(value).map_err(|error| {
        StoreError::Validation { detail: error.to_string() }
    })?;
    Ok(quoted[1..quoted.len() - 1].to_owned())
}

fn is_valid_extras_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn require_non_empty(value: &str, field: &str) -> Result<(), StoreError> {
    if value.is_empty() {
        return Err(StoreError::Validation {
            detail: format!("{field} is required and must be non-empty"),
        });
    }
    Ok(())
}

/// The anchored opening bytes shared by the composer and the remover:
/// `{"transformation_key":"<escaped>",`. `transformation_key` is always followed
/// by `schema_version`, so the trailing comma is invariant; routing both paths
/// through this one function is what makes the writer and remover agree
/// byte-for-byte.
fn record_opener(key: &str) -> Result<String, StoreError> {
    Ok(format!("{{\"transformation_key\":\"{}\",", escape_value(key)?))
}

fn push_string_field(
    out: &mut String,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    out.push_str(",\"");
    out.push_str(key);
    out.push_str("\":\"");
    out.push_str(&escape_value(value)?);
    out.push('"');
    Ok(())
}

pub fn compose_record(record: &Record) -> Result<String, StoreError> {
    require_non_empty(&record.transformation_key, "transformation_key")?;
    require_non_empty(&record.proposed_value, "proposed_value")?;
    require_non_empty(&record.timestamp, "timestamp")?;
    for (key, _) in &record.extras {
        if RESERVED.contains(&key.as_str()) {
            return Err(StoreError::Validation {
                detail: format!("reserved key '{key}' in extras position"),
            });
        }
        if !is_valid_extras_key(key) {
            return Err(StoreError::Validation {
                detail: format!("invalid extras key '{key}'"),
            });
        }
    }

    let mut out = record_opener(&record.transformation_key)?;
    out.push_str("\"schema_version\":");
    out.push_str(&record.schema_version.to_string());
    out.push_str(",\"outcome\":\"");
    out.push_str(record.outcome.as_str());
    out.push('"');
    push_string_field(&mut out, "proposed_value", &record.proposed_value)?;
    if let Some(user_value) = &record.user_value {
        push_string_field(&mut out, "user_value", user_value)?;
    }
    push_string_field(&mut out, "timestamp", &record.timestamp)?;
    for (key, value) in &record.extras {
        push_string_field(&mut out, key, value)?;
    }
    out.push('}');
    Ok(out)
}

pub fn remove_prefix(key: &str) -> Result<String, StoreError> {
    record_opener(key)
}
```

#### 3. The `RecordStore` implementation

**File**: `cli/corpus-adapters/src/store.rs`
**Changes**: `FileCorpusStore` is **extended** here to carry a `LockOptions`
field (`new()` uses the default; `with_lock_options` overrides it for tests, so
the contended-lock and reclaim tests inject a small `ceiling_ms` rather than
waiting 300 s), and implements `RecordStore`. The field is introduced in this
phase — not Phase 2 — because it is first *used* here: an unused field would trip
`dead_code` under `warnings = "deny"`. The change is internal to this work item
(the phases merge in sequence and no external crate consumes `FileCorpusStore`
between them), so it is an extension, not a break to a released API; the
`AtomicWrite` impl and the public constructor names are unchanged. `new()` drops
`const` because `LockOptions::default()` is not `const`.

`append_record` mkdir-`p`s the parent, **acquires the lock, then** reads existing
content, **normalises the separator** (inserts a `\n` when the file is non-empty
and does not already end in one, so a record is never glued onto a
newline-less last line), appends `compose_record(record) + "\n"`, and
`atomic_write`s.
`remove_by_key` fixes a lost-update hazard: it takes an unlocked existence
fast-path (an absent file is a no-op needing no lockdir), and otherwise
**acquires the lock and only then reads the authoritative content** — never
rewriting a pre-lock snapshot — retains lines that do **not** `starts_with` the
anchored prefix, and `atomic_write`s the result (an all-removed file becomes
empty, not deleted). `lockdir(path)` is `<path>.lockdir` — byte-identical to the
bash `${target}.lockdir` sidecar, so the mkdir-lock still mutually excludes
across the bash and Rust implementations as cheap defence-in-depth. The
`LockGuard` releases on scope exit.

New imports in `store.rs`: `std::io::ErrorKind`, `std::path::PathBuf`,
`corpus::{Record, RecordStore}`, `crate::jsonl::{compose_record, remove_prefix}`,
`crate::lock::{self, LockOptions}`.

```rust
pub struct FileCorpusStore {
    lock: LockOptions,
}

impl FileCorpusStore {
    #[must_use]
    pub fn new() -> Self {
        Self { lock: LockOptions::default() }
    }

    #[must_use]
    pub const fn with_lock_options(lock: LockOptions) -> Self {
        Self { lock }
    }
}

impl Default for FileCorpusStore {
    fn default() -> Self {
        Self::new()
    }
}

fn lockdir(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".lockdir");
    PathBuf::from(name)
}

impl RecordStore for FileCorpusStore {
    fn append_record(
        &self,
        path: &Path,
        record: &Record,
    ) -> Result<(), StoreError> {
        let line = compose_record(record)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
        }
        let _guard = lock::acquire(&lockdir(path), self.lock)?;
        let mut content = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(io(path, &error)),
        };
        if content.last().is_some_and(|byte| *byte != b'\n') {
            content.push(b'\n');
        }
        content.extend_from_slice(line.as_bytes());
        content.push(b'\n');
        atomic_write(path, &content)
    }

    fn remove_by_key(
        &self,
        path: &Path,
        key: &str,
    ) -> Result<(), StoreError> {
        if !path.exists() {
            return Ok(());
        }
        let prefix = remove_prefix(key)?;
        let _guard = lock::acquire(&lockdir(path), self.lock)?;
        let existing = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(());
            }
            Err(error) => return Err(io(path, &error)),
        };
        let mut out = String::with_capacity(existing.len());
        for line in existing.lines() {
            if !line.starts_with(&prefix) {
                out.push_str(line);
                out.push('\n');
            }
        }
        atomic_write(path, out.as_bytes())
    }
}
```

#### 4. Dependencies and module wiring

**File**: `cli/Cargo.toml`
**Changes**: Add caret-pinned `rand = "0.9"` to `[workspace.dependencies]`.

**File**: `cli/corpus-adapters/Cargo.toml`
**Changes**: Add `rand = { workspace = true }` and
`serde_json = { workspace = true }`.

**File**: `cli/corpus-adapters/src/lib.rs`
**Changes**: private `mod jsonl;` and private `mod lock;` (both reached only
through `crate::` from `store.rs`; their `pub fn` items are unreachable outside
the crate because the modules are private, so nothing new is re-exported —
consumers use the `RecordStore` port).

### Success Criteria

#### Automated Verification

- [x] `corpus-adapters` compiles: `cd cli && cargo build -p corpus-adapters`
- [x] Lock tests pass: `cd cli && cargo test -p corpus-adapters lock`
- [x] JSONL composer tests pass: `cd cli && cargo test -p corpus-adapters jsonl`
- [x] `RecordStore` round-trip tests pass:
      `cd cli && cargo test -p corpus-adapters --test store`
- [x] `cargo-deny` admits `rand`: `mise run deny:check`
- [x] Full single-component gate: `mise run cli:check`

Tests to write first:

- **Lock, inline in `lock.rs`** (via `acquire_with` with an injected `is_alive`
  and a small `ceiling_ms`):
  - AC-3: an owner PID reported dead is reclaimed and the lock is acquired; an
    owner PID reported live is **never** reclaimed (acquisition times out).
  - AC-4: a lockdir with a **missing**, **empty**, or **unparseable** (e.g.
    `owner` contains `not-a-pid`) sentinel is treated as live — `is_alive` is
    never consulted for the missing/empty cases — → `LockTimeout` under the
    ceiling. The unparseable case guards the deliberate hardening divergence.
  - AC-5: a permanently-held lock (`is_alive` always true) returns
    `LockTimeout` bounded by the injected ceiling; a `create_dir` error that is
    not `AlreadyExists` (parent is a regular file — deterministic and
    root-independent) returns a **fail-fast** error (`Io`, not `LockTimeout`)
    **without** entering the back-off loop (assert the call returns promptly and
    the variant is not `LockTimeout`).
  - Single-winner reclaim: two threads (`std::thread::scope`), both with a small
    injected `ceiling_ms`, race `acquire` against a lockdir whose injected
    `is_alive` reports the owner dead, and **both hold their guard until join** so
    the outcome is well-defined — assert exactly one `Ok(guard)` and one
    `Err(LockTimeout)`, and no panic; exercises the rename-then-remove path under
    contention.
  - `claim` owner-write-failure hardening: force the post-`create_dir` owner
    write to fail (e.g. pre-create a **directory** at `<lockdir>/owner` so
    `fs::write` errors), then assert `acquire` returns `Io` **and** the lockdir no
    longer exists (the release-not-orphan invariant); if no deterministic
    injection is feasible, record it as inspection-only.
  - `jitter_ms(base)` always returns a value in `1..=base` across many
    iterations (a deterministic bound check, no wall-clock assertion).
  - The guard's `Drop` removes the lockdir (a subsequent `acquire` succeeds).
  - The `PermissionDenied` → `NotWritable` arm is covered behaviourally only by
    inspection (a genuine unwritable directory is root-dependent and flaky in
    CI); its `Display`/kernel mapping is pinned by the Phase 1 five-variant test.
- **Composer, inline in `jsonl.rs`**:
  - AC-7: a golden-record assertion pinning the exact emitted bytes for the full
    canonical order — `transformation_key`, `schema_version` (bare number),
    `outcome`, `proposed_value`, `user_value` **only when supplied**, `timestamp`,
    then extras in declaration order. Include a second golden case with
    `outcome = Accepted` **and** a supplied `user_value` to pin that emission is
    presence-based, decoupled from `outcome`.
  - AC-8: a record with an **empty** `proposed_value` is rejected with a
    `Validation` error; likewise an empty `transformation_key` and an empty
    `timestamp`.
  - A reserved key or a malformed key in the extras position is rejected.
  - Escaping of backslash, double-quote, and control characters is asserted
    against **hand-written literal expected bytes** (including the raw `0x7F`
    byte serde_json leaves unescaped) — never recomputed via a live
    `serde_json::to_string` call at test time, which would be tautological. The
    AC-7 golden records use the same literal-expected discipline.
- **`RecordStore`, black-box in `cli/corpus-adapters/tests/store.rs`** (each test
  uses a unique `tempfile::tempdir()` working directory):
  - AC-6: `append_record` then `remove_by_key` for adversarial
    `transformation_key` values (backslashes, double-quotes, control chars)
    round-trips to an empty file — escape parity, not just field order.
  - Anchored-prefix false-match guard: append records keyed `"foo"` and
    `"foobar"`, `remove_by_key("foo")`, and assert the `"foobar"` record
    **survives** intact — pins the trailing-comma anchor and `starts_with`
    against an over-broad match (data-loss regression guard).
  - `append_record` twice then `remove_by_key` for one key leaves exactly the
    other record.
  - `remove_by_key` on an absent or empty file is a no-op.
  - Byte-level append shape: a fresh `append_record` produces exactly
    `compose_record(record) + "\n"` (LF only, no `\r\n`); a second append onto a
    file whose last line lacks a trailing newline still yields two distinct
    lines (pins the separator, given this repo's mixed-line-ending sensitivity).
  - Concurrency (the automated guard the manual stress loop replaces): a
    `std::thread::scope` spawning N threads, each `append_record`-ing a distinct
    record to one path through a `FileCorpusStore` with the **default** (generous)
    `ceiling_ms` — never a small one, so N-1 waiters backing off under a loaded CI
    runner cannot exhaust the ceiling and spuriously `LockTimeout` — joins, then
    asserts the file has exactly N lines, each parses as JSON, and the **set of
    `transformation_key`s equals the N distinct keys written** (pins content, not
    just count, so a lost update that preserved line count is still caught).
    Deterministic despite non-deterministic thread timing; the regression guard
    for the lock's core serialisation purpose.

#### Manual Verification

- [x] The automated `std::thread::scope` concurrency test above is the primary
      guard; optionally, a longer hand-run soak (many rounds of parallel
      `append_record` to one path) confirms no record is lost and no partial line
      is ever observable on disk under sustained contention.

---

## Testing Strategy

### Unit Tests

- `corpus`: `StoreError` `Display` strings and the `From<StoreError> for
  kernel::Error` mapping; `Outcome::as_str`.
- `corpus-adapters/store.rs`: the atomic-write invariants and the
  interruption-before-rename seam (Phase 2).
- `corpus-adapters/lock.rs`: reclaim, the mid-acquisition live window, the
  bounded-ceiling timeout, and the fail-fast early error (Phase 3), all via the
  injected liveness predicate and a small ceiling — no real-PID dependence, no
  wall-clock timing assertions.
- `corpus-adapters/jsonl.rs`: the golden canonical order, the
  `proposed_value`-required rejection, extras validation, and escape parity.

### Integration Tests

- `corpus-adapters/tests/store.rs`: the compose→remove round-trip over the public
  `RecordStore`, including adversarial keys, the anchored-prefix false-match
  guard, the byte-level append shape, and the `std::thread::scope` concurrency
  guard — all exercising the real lock and `atomic_write`. Each test isolates
  itself in a unique `tempfile::tempdir()` (RAII-cleaned) rather than a fixed
  name under `std::env::temp_dir()`, so the suite is robust under cargo's
  parallel runner and across the macOS-private vs shared-linux `/tmp` layouts.

### Manual Testing Steps

1. Build the workspace and run `cd cli && cargo test -p corpus -p
   corpus-adapters`.
2. Stress the append path: a short loop spawning several concurrent
   `append_record` calls to one path, then assert every record is present and
   each line is well-formed JSON.
3. Confirm `mise run cli:check` exits 0 (rustfmt + clippy + deny + pup).

## Performance Considerations

The lock's back-off matches the bash design intent (base 4 ms doubling to a
~256 ms cap, jitter uniform `1..=base`) to absorb CI scheduler starvation without
a thundering herd. The critical section is a single read-modify-write, held for a
few milliseconds. The exact wall-clock timing is deliberately not gated by any
test (per AC-5) to avoid flakiness.

Both `append_record` and `remove_by_key` read the whole file, transform it in
memory, and `atomic_write` the whole file back under the lock — inherent to the
atomic-rename design, which makes each operation O(file size) and building an
n-record log O(n²). This is fine for the bounded migrate session log (the single
intended consumer), but a future high-volume consumer would need a different
persistence strategy; the `RecordStore` port is not designed for append-heavy
large files.

## Migration Notes

The clean-cutover ruling (0180 Open Questions, propagated onto 0172) requires
that a given atomic-store JSONL file is written by **either** the bash primitives
**or** this Rust port, never both concurrently. Byte-parity with bash-written
records is therefore out of scope; the migrate consumer (0172) must cut each
session-log file over atomically. If that guarantee cannot hold, AC-6/AC-7 and
the byte-parity scope-out must be revisited.

This primitive does not enforce the invariant, so the plan pins the following
guard rails around the gap:

- **Sequencing constraint**: 0172's atomic per-file cutover must be in place
  **before** `FileCorpusStore`/`RecordStore` is wired to any production
  session-log path. Until then, `remove_by_key` is only ever run against
  Rust-written files (a Rust `remove_by_key` over a bash-written line whose key
  contains `0x7F` would silently fail to match, because bash escaped it ``
  while serde_json reconstructs the raw byte).
- **Defence-in-depth**: `lockdir(path)` is byte-identical to bash's
  `${target}.lockdir`, so the mkdir-lock still mutually excludes across the two
  implementations even if a cutover slips — turning a would-be interleaved
  read-modify-write into serialised access.
- **Escaper fallback**: if the clean-cutover premise is ever relaxed, the
  migration path is to replace the serde_json escaper with a hand-ported
  `jsonl_json_escape` (byte-compatible at `0x7F`) and re-pin AC-7's golden
  record. serde_json is chosen now only because AC-6/AC-7 are Rust→Rust.

## References

- Original work item: `meta/work/0180-atomic-store-primitives-corpus-adapters.md`
- Research:
  `meta/research/codebase/2026-07-19-0180-atomic-store-primitives-corpus-adapters.md`
- Parent epic: `meta/work/0166-shared-config-corpus-store-crates.md`
- Source bash: `scripts/atomic-common.sh:16-246`, `scripts/jsonl-common.sh:21-149`
- Precedents: `cli/config/src/service.rs:29-45`, `cli/config/src/error.rs:35-108`,
  `cli/corpus-adapters/src/patcher.rs:12-94`,
  `skills/visualisation/visualise/server/src/file_driver.rs:187-262`,
  `cli/config-adapters/src/store.rs:58-80`
- Toolchain: `cli/pup.ron:57-72`, `cli/deny.toml:41-69`, `cli/Cargo.toml:13-45`
- ADRs: ADR-0045, ADR-0053
