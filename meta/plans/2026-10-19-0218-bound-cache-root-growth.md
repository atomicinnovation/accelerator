---
type: "plan"
id: "2026-10-19-0218-bound-cache-root-growth"
title: "Bound a Relocated Cache Root's Growth Implementation Plan"
date: "2026-10-19T18:27:59+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0218"
parent: "work-item:0218"
derived_from: ["codebase-research:2026-10-18-0218-bound-cache-root-growth"]
relates_to: ["plan:2026-10-16-0339-tree-reap-cross-artifact-lock-probe", "plan:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness", "adr:ADR-0072", "adr:ADR-0063", "adr:ADR-0064"]
tags: ["cli", "launcher", "cache", "tree", "prune", "claims", "retention"]
revision: "871d47e55744db989bd17a2addc1cad3ded4f553"
repository: "accelerator"
last_updated: "2026-10-21T20:14:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bound a Relocated Cache Root's Growth Implementation Plan

## Overview

A cache root relocated with `ACCELERATOR_CACHE_DIR` and shared across plugin
upgrades grows without bound: every version leaves single-file entries,
staged verifier copies, trees and crash residue, and almost nothing reclaims
them. This plan implements ADR-0072 for such a root:

- every dispatch refreshes a **version claim**;
- every cold path runs a claim-gated **root prune** under single-flight locks
  tried without blocking;
- `accelerator cache prune` runs the same prune, waiting for its locks.

The default root (`${ACCELERATOR_PLUGIN_ROOT}/bin`) keeps ADR-0063's
behaviour. Four changes apply in every root because they only make existing
paths safer: a bounded `ENOENT` re-resolve, a recorded test of `store`'s
rename-over replacement, tree prune validating a pointer before removing the
generation it names, and the verb no longer creating tree locks for artifacts
it never materialised.

## Current State Analysis

- **No root identity.** `cache_root::candidate` returns the override verbatim
  or `plugin_root.join("bin")` as an untagged `PathBuf`
  (`cli/launcher/src/launch/outbound/resolve/cache_root.rs:58-74`). Nothing
  canonicalises; the shell passes the override through unresolved
  (`bin/accelerator:198-207`).
- **No filesystem seam.** `cache.rs` (`find` `:51-73`, `store` `:81-133`),
  `reverify` (`resolve/mod.rs:100-119`) and `tree/claims.rs` call `std::fs`
  and `geteuid()` directly, so no test can record a dispatch's calls.
- **Single attempt at `exec`.** `run_external` (`core.rs:313-322`) resolves
  once and execs once; `ResolveBinary` (`core.rs:294-302`) has no way to
  force the cold path.
- **No single-file eviction.** `cache.rs`'s module doc says the cache "needs
  no eviction". Launcher entries, shim copies, `.tmp-` residue, probes and
  bootstrap lock directories are never removed by Rust code.
- **Narrow tree prune.** The verb loops over `pins::artifact_names()` on
  `HOST_PLATFORM` (`launch/cache.rs:143-155`), takes each tree lock with
  `O_CREAT` (`tree_lock.rs:20-26` → `lease.rs:85-90,283-292`), removes claims
  only with their digest (`claims.rs:86-97`), and never reaps orphan
  sidecars or lock files.
- **Unvalidated pointer joins.** `artifact_pointers` (`resolver.rs:824-852`)
  only trims a pointer's contents, and `reclaim_unclaimed`
  (`resolver.rs:786-820`) passes them to `remove_dir_all(root.join(…))`
  (`layout.rs:153`). A pointer holding an absolute path or `../` names a
  directory outside `trees/`. `locate` uses the strict `validated_generation`
  (`layout.rs:262`); prune does not, and has no layout-version check.
- **Reusable precedent.** Tree claims (`claims.rs`), `AGE_BACKSTOP`
  (`reap.rs:47`, private), the decision tables (`reap.rs:330-365`), the
  non-creating `probe_single_flight` and inode confirmation from 0339
  (`lease.rs:120-232`), the `HeldTreeLock` proof (`tree_lock.rs`), and
  trait-object seams (`Clock`, `LivenessProbe`, `StepObserver`).
- **No end-to-end trust path.** The real launcher trusts only
  `TrustedKeys::embedded()` (`keys.rs:55-57`) and builds https-only fetchers
  (`fetcher.rs:194-223`), so no test can have it fetch a locally signed
  sub-binary.

## Desired End State

In an override root, every launcher start writes
`claims/<version>.<launcher-id>` before any `find`, throttled to once per
`CLAIM_WINDOW/10`, and a successful
`store` or tree materialise runs a root prune. After every launcher sharing
the root runs this change, the root holds only what a version or digest
claimed within `CLAIM_WINDOW` (14 days) protects, what a held lease protects,
residue younger than `AGE_BACKSTOP` (1 hour), and the kept names the work
item lists. In the default root, a warm dispatch performs exactly the
filesystem calls it performs today.

Verification: every acceptance criterion in
`meta/work/0218-bound-cache-root-growth.md` maps to a named test (see Testing
Strategy), and `mise run` exits 0.

### Key Discoveries:

- `materialise` reaps under the tree lock it already holds
  (`resolver.rs:617-651`, reaping at `:639`). The cold-path prune of its own
  artifact runs there, as `OwnerLock::HeldByCaller` does (`reap.rs:172-184`);
  the rest of the root is pruned after that lock is released, because `reap.rs`
  forbids a process from probing a tree lock it holds.
- `fetch_verify_store` ends with `cache::store` (`resolve/mod.rs:178-186`);
  the cold-path prune goes immediately after it, before the path returns to
  `exec`.
- `try_single_flight` already separates contention (`Ok(None)`) from failure
  (`Err`) (`lease.rs:103-110,247-253`). "Skip on the cold path, report from
  the verb" needs no new error type.
- `launcher_id()` (`main.rs:353-366`) is passed only to `TreeResolver`; the
  dispatch resolver needs it too. The bootstrap execs
  `${cache_dir}/accelerator-launcher-<version>-<platform>`, so the id already
  embeds cache directory, version and platform.
- `_assert_no_test_loopback` (`tasks/build.py:354-371`) byte-scans every
  binary in `_CLI_RELEASE_BINARIES`, which includes `accelerator`, for
  `TEST_LOOPBACK_MARKER`. Today's markers are `#[no_mangle]` statics, which
  the release profile's `strip` and thin LTO likely remove, so Phase 6 moves
  them to string literals read at start-up. `tasks/shared/dev_builds.py:31`
  already shows how the feature reaches debug builds.
- The entrypoint harness's downloader stub resolves every URL by basename
  from `SERVER_DIR` (`tests/integration/support/installation.py:78-92`), and
  `assert_hermetic` (`:252-265`) insists on a `.invalid` release host.
- The shipped Linux launchers are static musl builds
  (`tasks/shared/targets.py:11-12`). musl's `getpwuid_r` and `getgrgid_r`
  read only `/etc/passwd`, `/etc/group` and `nscd`, and load no NSS modules,
  so directory-service accounts are invisible to them; the macOS launchers
  use the system libc, which consults Open Directory.
- Every `cargo test`/clippy run passes `--all-features`
  (`tasks/test/cli.py:11-14`, `tasks/lint/cli.py:7`), so seams are trait
  objects, not features. The only feature this plan adds is the end-to-end
  trust feature, which is inert unless its environment variable is set.

## What We're NOT Doing

- No claims or prune in the default root or a development checkout's `bin/`.
  ADR-0063 still governs those; the verb's tree prune there stays pinned
  artifacts on the host platform.
- No compatibility gating for launchers predating version claims or 0339's
  inode check (see Accepted Risks).
- No re-resolve for a path from an `ACCELERATOR_<SUB>_BIN` override.
- No claim refresh on a `materialise` hit (`resolver.rs:618-637`). Dispatch's
  `acquire` refreshes the claim before the executor calls `cache ensure`.
- No root prune from `cache repair`. It is a user-invoked verb, not a cold
  path the item names. 0222's `cache ensure --from` must opt in or out when
  it lands.
- No change to `bin/accelerator`'s writes. Its only change is the `execfail`
  handling of a missing launcher at the final `exec`. The `AGE_BACKSTOP`
  grace covers its pre-claim launcher entry.
- No pruning of `.accelerator-unverified.log` or of names the plugin does not
  produce.
- No fix for two shas under one version after a corrupt-entry refetch with a
  different manifest sha (research Open Questions). Both stay protected by the
  version's claim.

## Accepted Risks

These are recorded here and stated in `internals.md` in Phase 8:

- **Pre-0339 launchers and orphan-lock removal.** A launcher that does not
  confirm its lock's inode can lock an unlinked inode after tree prune
  removes an orphan lock file. While both hold a lock, a reap can remove the
  older launcher's in-progress residue, and a concurrent materialise of the
  same artifact can corrupt a shared partial archive. Each consequence
  records a sticky `UnexpectedDigest` or `Extraction` marker for that
  artifact, which is Refusal-class and stops a `--fail-safe` crawl. We
  accept this risk by name: no root can tell whether such a launcher shares
  it.
- **A stale claim refreshed between read and unlink.** Prune re-checks a
  claim's age immediately before unlinking it. A refresh landing inside that
  window loses the claim, and the version's next prune may evict its entries.
  The cost is one re-fetch.
- **Pre-claim launchers sharing the root.** A pre-claim launcher never
  writes a claim, so its entries are evicted again after each claim-aware
  prune once they are older than `AGE_BACKSTOP`. Its session re-fetches each
  time, cannot dispatch offline until it is reloaded, and fails a dispatch
  caught between `find` and `exec` rather than re-resolving.
- **Another release's shim copy.** Prune keeps the running plugin's shim
  copy for every platform, and any other copy younger than `AGE_BACKSTOP`. A
  live session of another release whose vendored shim differs, reusing a
  copy staged more than an hour earlier, can lose it between its staging
  check and use. That bootstrap
  fails with `fail_integrity`, and the SessionStart hook reports the recorded
  unverified launcher until the user clears `.accelerator-unverified.log`.
  This happens only across releases that change the vendored shim.
- **A tree claimed after the re-read.** Tree prune re-reads a digest's
  claims immediately before removing its tree, and holds an exclusive lock
  on its `.lease` across the removal. Where `flock` works, a dispatch that
  leased the tree makes that lock fail and the tree is kept. On a filesystem
  whose lease probe reports `Unknown`, an acquire between the re-read and the
  pointer unlink loses its tree, and that dispatch fails. The window is
  microseconds.
- **A bootstrap lock reclaimed after the re-check.** The residue sweep
  re-checks a lock directory immediately before unlinking it. A bootstrap
  that reclaims and recreates it inside that window loses its lock, and two
  bootstraps fetch the launcher concurrently. Both rename identical verified
  bytes into place.
- **A crash between the two unlinks of an entry.** Single-file prune removes
  a `.minisig` and then its entry. A crash or failed unlink between them
  leaves an unpaired entry or a lone `.minisig`, which the grammar cannot
  tell from a foreign name and so keeps for good. The residue is one small
  file per occurrence.
- **A launcher entry stamped with the server's time.** On wget-only hosts
  `fetch` may keep the server's `Last-Modified`, so a just-fetched launcher
  entry can look older than `AGE_BACKSTOP`. Its staging files are protected by
  their writer's pid, which `kill(pid, 0)` can answer only on the pruning host:
  in a root shared across hosts, another host's in-flight staging can still be
  swept, and that bootstrap records an unverified launcher. The finished entry
  is protected by the start-up claim from its first run, and the gap between the
  bootstrap's verification and `exec` is already accepted by the work item.
- **Hosts whose metadata fails the rule.** An ancestor owned by another uid,
  such as a container's host-owned bind mount or GitHub Actions' `/__w`, is
  refused until that uid is listed in `ACCELERATOR_CACHE_TRUSTED_OWNERS`, which
  the refusal names; listing it trusts that account to redirect prune's removals
  within the user's privileges. A rootless container's overflow uid 65534 and a
  macOS volume with "Ignore ownership" (uid 99) are refused even when listed,
  since they stand for many accounts or for none. WSL2 `drvfs` mounts report
  `0777` and NFS with `all_squash` reports `nobody` writable bits no listing
  fixes, so a relocated root there is never pruned; the refusal suggests a
  per-user `ACCELERATOR_CACHE_DIR` on a native filesystem.
- **Accounts no lookup can resolve.** On a Linux host without `getent` at
  `/usr/bin` or `/bin`, such as NixOS or Guix, or an arbitrary-uid container
  no lookup knows, whose accounts are not in `/etc/passwd` and `/etc/group`,
  a root writable through the user's private group is refused until the
  user runs `chmod g-w <root>`, which the refusal names.
- **A user-private group shared by misconfiguration.** A group-writable
  root counts as safe when its group is the user's private group with no
  listed members. Another account whose own primary group is that group,
  which only an administrator can arrange, could still swap entries. We do
  not scan the password database to rule it out.
- **Shared roots are not pruned.** A root other users can create names in
  works as a cache but grows as it does today. Sharing trees across users
  does not work today either: `ensure_trees_dir` refuses a `trees/` that is
  not the user's own (`resolver.rs:949-965`). A pruned, deduplicated shared
  cache needs a group trust model, which is work item 0340 and its stories
  0341 and 0342.
- **A directory swapped between check and removal by the same user.** Prune
  checks that `claims/`, `trees/`, `trees/claims/` and a bootstrap lock
  directory are owned, non-symlink directories, then removes by path. Prune
  runs only in a root no other user can rename entries in (Phase 4 §7), so
  only a process running as the same user can swap one for a symlink inside
  that window. That is not a privilege boundary, and we accept it rather
  than move every removal onto directory descriptors.
- The remaining assumptions in the work item stand as written.

## Implementation Approach

The phases follow the work item's ordering. A phase that changes behaviour
does so only behind something already in place: the recorder and the fixed
list come first, and the recovery and claims land before any prune. Single-file
and tree prune then run only behind the verb, and the cold-path triggers come
last. Phase 6 sits before them as standalone hardening: it fixes the
`test-loopback` release guard for `jira`, `linear` and `research`, which the
launcher's own seam in Phase 7 then relies on. Each phase is one mergeable
change that leaves `mise run` green.

Every test seam arrives in the first phase whose test needs it. Each one is a
trait object with a no-op or real production implementation, following
`StepObserver`'s precedent.

Domain vocabulary, used verbatim in names:

| Term | Type or module | Meaning |
|---|---|---|
| cache root kind | `CacheRootKind::{PluginBin, Relocated}` | what `identify` reports about the root |
| root retention | `retention::RootRetention` | the regime governing a root, chosen once from its kind |
| version claim | `retention::version_claims` | `claims/<version>.<launcher-id>` |
| root prune | `retention::RootPrune` | single-file prune + residue sweep + tree prune |
| tree prune | `tree::TreePrune` | the one implementation of tree reclamation, in every root |
| tree prune regime | `TreePruneRegime::{PinnedOnHost, WholeTree}` | which artifacts a tree prune visits and whether it sweeps bookkeeping |
| lock patience | `LockPatience::{Wait, Try}` | the verb waits, a cold path tries |

`retention` is a new module beside `tree/` at
`cli/launcher/src/launch/outbound/resolve/retention/`. The layering is one
way:

```text
main.rs ──start_up──► retention::for_root ──► RootRetention   (retention/)
                                               ├─ PluginBinRetention
                                               └─ RelocatedRetention
retention/ ──depends on──► tree/                (never the reverse)
tree/ defines AfterMaterialise, TreeLocks, TreePrune and HeldTreeLock
both import ──► filesystem, ownership, single_flight, prune_report,
                retention_windows                 (leaf modules)
```

Five leaf modules sit under `resolve/` and import nothing from `tree/` or
`retention/`:

| Module | Holds |
|---|---|
| `filesystem` | the `CacheFilesystem` port and `EntryMetadata` |
| `ownership` | the `Ownership` seam and `is_trusted_entry` / `is_trusted_directory` |
| `single_flight` | the `SingleFlightLocks` trait, `LockGuard`, `LockProbe`, `LockPatience` |
| `prune_report` | `PruneStep`/`PruneSteps` and the failure types |
| `retention_windows` | the shared constants |

`tree/` holds only tree concepts; vocabulary that single-file and tree prune
share lives in the leaves.

`retention::for_root` is the only code that matches on `CacheRootKind`, and
`main.rs` calls it once, through `start_up`. `ResolverConfig` carries the
cache root's path, not its kind. Every collaborator is a required
constructor argument. `main.rs` builds one `CachePorts`; `StartUp` builds
both resolvers and the retention from it, so one recorder sees `find`,
single-file prune and tree prune alike. No retention holds a resolver.

```text
                     RootPrune::run(patience)
                 /                               \
     root lock (.accelerator-prune.lock)     each (artifact, platform)
        |                                        under its tree lock
   SingleFilePrune ──► ResidueSweep              |
                                             TreePrune
```

---

## Phase 1: Filesystem Port and the Fixed Warm-Path List

### Overview

Route the single-file cache's filesystem calls through a port so a test can
record them, then pin today's default-root warm-dispatch sequence as a fixed
list. This is two commits. The first changes launcher source and is
behaviour-preserving. The second adds only the test and touches no launcher
source.

### Changes Required:

#### 1. The filesystem port (commit 1)

**File**: `cli/launcher/src/launch/outbound/resolve/filesystem.rs` (new)
**Changes**: A port covering the calls `find`, `store` and `reverify` make,
plus the production adapter. Later phases add the methods their tests first
need. Adding a method never changes the calls `find` and `reverify` make, so
the fixed list stays stable.

```rust
pub trait CacheFilesystem {
    fn list(&self, dir: &Path) -> io::Result<Vec<OsString>>;
    fn exists(&self, path: &Path) -> bool;
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn make_executable(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

pub struct SystemFilesystem;
```

`write_private` opens with `O_CREAT|O_TRUNC|O_NOFOLLOW` and mode `0600`.

**File**: `cli/launcher/src/launch/outbound/resolve/cache.rs`
**Changes**: `find` and `store` take `filesystem: &dyn CacheFilesystem` as
their first parameter.

- `find` lists once, keeps today's "first non-UTF-8 name aborts" rule, and
  checks the sidecar with `exists`.
- `store` uses `create_dir_all`, `write_private`, `make_executable`, `rename`,
  and `remove_file` only on a failed rename, exactly as today.
- The module doc's "needs no eviction" sentence is deleted.

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs`
**Changes**: `FetchVerifyCacheResolver::new` and `with_fetcher`, which
production uses, take an `Arc<dyn CacheFilesystem + Send + Sync>` as a
required argument, and `main.rs` passes `SystemFilesystem`. It is an `Arc`
because later phases share the same instance with the retention. `reverify`
reads through it.

**File**: `tasks/lint/store_duplication.py`
**Changes**: `filesystem.rs` joins `cache.rs` in `ALLOWLIST`: the raw
`rename` that `cache.rs`'s publish composes now lives in the port's adapter.

#### 2. The recording filesystem (commit 1)

**File**: `cli/launcher/tests/common/recording_filesystem.rs` (new), declared
from `tests/common/mod.rs`
**Changes**: A `RecordingFilesystem` decorator over `SystemFilesystem`. It
appends a `FilesystemCall` per call to an `Arc<Mutex<Vec<_>>>` and exposes
`calls()` and `calls_touching(&Path)`. Mutating calls are distinguished by
`FilesystemCall::is_mutation()`.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedPath {
    InRoot(PathBuf),
    Outside(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesystemCall {
    List(RecordedPath),
    Exists(RecordedPath),
    Read(RecordedPath),
    ReadToString(RecordedPath),
    CreateDirAll(RecordedPath),
    WritePrivate(RecordedPath),
    MakeExecutable(RecordedPath),
    Rename { from: RecordedPath, to: RecordedPath },
    RemoveFile(RecordedPath),
}
```

Each later method on `CacheFilesystem` arrives with its variant here.

The recorder is built with the cache root and an outer anchor, the test's
temp directory, and canonicalises both at construction. A path under the
cache root, given or canonical, is recorded `InRoot`, relative to it; any
other path under the anchor, given or canonical, is recorded `Outside`,
relative to it; a path under neither is recorded `Outside` with its absolute
path. A recorder self-test records one path under each spelling of the root
and of the anchor, built through a symlinked anchor so both spellings differ
on every host. On macOS a
`tempfile` directory is `/var/folders/…` while its canonical form is
`/private/var/folders/…`, and from Phase 3 the launcher works on the
canonical form, so matching both spellings keeps the lists identical on the
ubuntu and macos legs. The
expected list is therefore independent of the temp directory, and a read of
the plugin `bin` or the shim source is visible as `Outside`.

#### 3. The fixed warm-path list (commit 2: tests only)

**File**: `cli/launcher/tests/resolution.rs`
**Changes**: The constant and a test that passes against today's resolver.
The harness keeps its cache in its own temp directory, so the recorder's
anchor is the harness's work directory. `resolution.rs` now declares
`mod common`.


```rust
fn default_root_warm_dispatch_calls(sha: &str) -> Vec<FilesystemCall> {
    let entry = in_root(format!("{BINARY}-{VERSION}-{sha}"));
    let signature = in_root(format!("{BINARY}-{VERSION}-{sha}.minisig"));
    vec![
        FilesystemCall::List(in_root("")),
        FilesystemCall::Exists(signature.clone()),
        FilesystemCall::Read(entry),
        FilesystemCall::ReadToString(signature),
    ]
}

#[test]
fn a_default_root_warm_dispatch_performs_exactly_the_fixed_calls() { … }
```

Later commits must not modify `default_root_warm_dispatch_calls`.

### Success Criteria:

#### Automated Verification:

- [x] Commit 1 changes no behaviour; existing suites pass:
      `mise run test:unit:cli`
- [x] Commit 2's diff touches only `cli/launcher/tests/`:
      `jj diff -r <commit> --summary`
- [x] `a_default_root_warm_dispatch_performs_exactly_the_fixed_calls` passes
- [x] `mise run cli:check` exits 0

#### Manual Verification:

- [x] The fixed list matches the calls `find` and `reverify` make, read
      against `cache.rs` and `resolve/mod.rs`

---

## Phase 2: `ENOENT` Re-Resolve, Pointer Containment, Replacement

### Overview

In every root, a dispatch whose `exec` fails with `ENOENT` re-resolves once
through the cold path. Tree prune removes only a generation a validated
pointer names, and the orphan reap recognises a generation only by its full
name grammar. `store`'s rename-over replacement gets a recorded test.

### Changes Required:

#### 1. The port

**File**: `cli/launcher/src/launch/core.rs`
**Changes**: `ResolveBinary` gains a required method, so every adapter
states its own re-resolve behaviour and production wiring cannot silently
inherit a no-op. The test fakes each gain one line returning
`ReResolution::Declined`:

```rust
pub trait ResolveBinary {
    fn resolve(&self, command: &ExternalCommand)
        -> Result<PathBuf, ResolutionError>;

    fn re_resolve(&self, command: &ExternalCommand, missing: &Path)
        -> ReResolution;
}

pub enum ReResolution {
    Declined,
    Resolved(PathBuf),
    Failed(ResolutionError),
}

pub fn run_external(
    resolver: &impl ResolveBinary,
    executor: &impl ExecBinary,
    command: &ExternalCommand,
) -> ResolutionError {
    let program = match resolver.resolve(command) {
        Ok(program) => program,
        Err(error) => return error,
    };
    let failure = executor.exec(&program, &command.args);
    if !failure.is_missing_program() {
        return failure;
    }
    match resolver.re_resolve(command, &program) {
        ReResolution::Resolved(program) => {
            executor.exec(&program, &command.args)
        }
        ReResolution::Failed(error) => error,
        ReResolution::Declined => failure,
    }
}
```

`ResolutionError::is_missing_program` matches `Exec` with
`source.kind() == NotFound`. The doc comment on `re_resolve` records why it
skips `find`: a second `find` could return another entry a prune is about to
remove. It also notes why the retry is bounded to one: an entry a prune
removes again between the re-fetch and the second `exec` must not loop.

#### 2. The adapters

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs`
**Changes**: `FetchVerifyCacheResolver::re_resolve` returns `Declined` when
`filesystem.exists(missing)`: the binary is there but cannot run, as with a
missing ELF interpreter, and a re-fetch would not help. Otherwise it returns
`Resolved` or `Failed` from `self.fetch_verify_store(name)`. The check costs
nothing on a successful `exec`.

**File**: `cli/launcher/src/main.rs`
**Changes**: `LazyProductionResolver::re_resolve` returns `Declined` when
`override_path` yields a path. Otherwise it builds the resolver as `resolve`
does and delegates. The construction is extracted into
`fn production_resolver() -> Result<FetchVerifyCacheResolver, ResolutionError>`
so both methods share it.

#### 3. Pointer containment

**File**: `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs`
**Changes**: `artifact_pointers` classifies each pointer's contents through
`validated_generation(contents, artifact, platform, digest, LAYOUT_VERSION)`,
with the digest taken from the pointer's own name:

```rust
enum PointerTarget {
    Current(String),
    OtherLayout,
    Malformed,
}
```

| `validated_generation` | `PointerTarget` | Prune action |
|---|---|---|
| `Ok(generation)` | `Current` | reclaim candidate, subject to the existing claim, pin and lease checks |
| `Err(LayoutUnsupported)` | `OtherLayout` | keep pointer and generation |
| any other `Err` | `Malformed` | keep the pointer, remove nothing it names |

`reclaim_unclaimed` acts only on `Current`. This is the only route from a
pointer's contents to a generation path in any prune.

**File**: `tree/layout.rs`
**Changes**: `read_pointer(path, filesystem) -> Option<String>` is the only
way any prune reads a pointer, including `reap.rs`'s referenced-generations
scan, which passes `&SystemFilesystem`. It calls a new port method,
`CacheFilesystem::read_bounded(path, limit)`, with a `ReadBounded`
`FilesystemCall` variant. `read_bounded` opens with `O_NOFOLLOW|O_NONBLOCK`,
`fstat`s the open file and requires a regular file, and reads at most `limit`
bytes, here 256; a valid generation name is under 128. Anything else is `None`,
which classifies as `Malformed`, so a planted FIFO, device symlink or oversized
file can neither block nor exhaust a prune.

#### 4. Generation name grammar

**File**: `cli/launcher/src/launch/outbound/resolve/tree/layout.rs`
**Changes**:

```rust
pub struct GenerationName<'a> {
    pub artifact: Artifact,
    pub digest: &'a str,
    pub layout: u32,
}

pub fn parse_generation_name(name: &str) -> Option<GenerationName<'_>>
```

It accepts exactly `<n>-<p>-<64hex>-<u32>-<16hex>` with `p` in
`KNOWN_PLATFORMS`, locating the platform as `Artifact::owning` does.

**File**: `tree/reap.rs`
**Changes**: `is_generation` becomes `parse_generation_name(name)` with
`layout == LAYOUT_VERSION`, replacing the `-{LAYOUT_VERSION}-` substring
test. A stricter grammar only keeps more, so this is safe in every root.

#### 5. Tests (red first)

**File**: `cli/launcher/src/launch/core.rs` (unit tests, using the
`RecordingExec`/`FixedResolver` fakes at `:456-548`)

- `a_missing_program_is_re_resolved_once_and_execed_again`
- `a_second_missing_program_surfaces_as_exec`
- `a_failure_other_than_enoent_is_not_re_resolved`
- `a_resolver_that_cannot_re_resolve_surfaces_the_first_failure`

**File**: `cli/launcher/tests/resolution.rs`

- `an_entry_removed_between_find_and_exec_is_refetched_once`: the named test
  the item requires. It runs in the default root here; Phase 3
  parameterises it by root kind and Phase 7 re-runs it with the post-store
  prune active. A `RemovingAfterResolve` decorator
  removes the resolved entry, and an `ExecUnlessMissing` fake returns
  `ENOENT` for a missing path and records the program otherwise. The test
  asserts one successful exec of the refetched path and one asset hit.
- `an_existing_but_unrunnable_entry_is_not_refetched`: `ExecUnlessMissing`
  returns `ENOENT` for a path that exists; no asset is fetched and the first
  failure surfaces
- `a_corrupt_entry_is_replaced_by_a_rename_with_no_unlink_before_it`: seed,
  poison, resolve with the recorder. `calls_touching(entry)` filtered to
  mutations equals `[Rename { to: entry, .. }]`, and the entry then holds the
  verified bytes. Parameterised by kind in Phase 3 and re-run in Phase 7, as
  above; the mutation filter on the entry path keeps prune's own calls out of
  the assertion.

**File**: `cli/launcher/src/main.rs` (unit test)

- `an_override_path_is_never_re_resolved`
- `an_unwritable_cache_root_is_re_resolved_through_the_cold_path`: with no
  `ACCELERATOR_<SUB>_BIN` override and an unwritable `ACCELERATOR_CACHE_DIR`,
  `re_resolve` returns `Failed(CacheRootUnavailable)`. The env-mutating
  `main.rs` tests run serially.

**File**: `cli/launcher/tests/tree_resolution.rs`

Each test plants an aged, lease-free victim directory and an unclaimed
pointer, plus a valid, unclaimed, aged tree of this layout as a positive
control. It runs the verb and asserts the victim survives and the control is
removed in the same run:

- `a_pointer_naming_an_absolute_path_deletes_nothing_outside_the_root`
- `a_pointer_escaping_with_dot_dot_deletes_nothing`
- `a_pointer_naming_another_artifacts_generation_is_kept`
- `a_malformed_pointer_is_kept`
- `a_symlinked_fifo_or_oversized_pointer_is_malformed_and_kept`
- `another_layouts_tree_is_kept_and_this_layouts_twin_is_removed`
- `an_aged_foreign_directory_resembling_a_generation_survives_the_reap`:
  `backup-2021-1-final` and `<n>-<unknown platform>-<64hex>-1-<16hex>`

**File**: `tree/layout.rs` (unit tests): `parse_generation_name` accepts every
known platform and rejects an unknown platform, a short or uppercase digest,
a non-numeric layout and a 15-hex suffix.

### Success Criteria:

#### Automated Verification:

- [ ] The new tests fail before the change and pass after:
      `mise run test:unit:cli`
- [ ] The fixed list is unchanged and its test still passes
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] Deleting a cached sub-binary between two dispatches in a scratch
      `ACCELERATOR_CACHE_DIR` re-fetches it once with no user-visible error

---

## Phase 3: Cache Root Identity and Version Claims

### Overview

Tag the cache root with its kind, decided by comparing device and inode, and
have every dispatch in a relocated root refresh its version claim before `find`.
Lift the shared retention constants. Nothing is pruned yet.

### Changes Required:

#### 1. Retention constants

**File**: `cli/launcher/src/launch/outbound/resolve/retention_windows.rs`
(new, a leaf module)
**Changes**: `pub const AGE_BACKSTOP` (1 hour), `pub const CLAIM_WINDOW`
(14 days) and `pub const REFRESH_FRACTION: u64 = 10`, each with its existing
doc text. `tree/reap.rs`, `tree/claims.rs` and `retention/` import them, and
`claims::CLAIM_WINDOW` is re-exported so callers are unchanged. The
integration tests' `BACKSTOP_SECONDS` (`tests/tree_resolution.rs:1391`)
becomes `retention_windows::AGE_BACKSTOP.as_secs()`.

#### 2. Cache root identity

**File**: `cli/launcher/src/launch/outbound/resolve/filesystem.rs`
**Changes**: the port gains what identity and claims need:

```rust
fn entry(&self, path: &Path) -> io::Result<EntryMetadata>;
fn resolve(&self, path: &Path) -> io::Result<ResolvedDirectory>;
fn create_private_dir(&self, path: &Path) -> io::Result<()>;
fn set_modified(&self, path: &Path, seconds: u64) -> io::Result<()>;

pub struct EntryMetadata {
    pub kind: FileKind,
    pub owner: u32,
    pub group: u32,
    pub mode: u32,
    pub modified_seconds: Option<u64>,
}

pub enum FileKind { File, Directory, Symlink, Other }

#[derive(PartialEq, Eq)]
pub struct DirectoryIdentity { pub device: u64, pub inode: u64 }

pub struct ResolvedDirectory {
    pub canonical: PathBuf,
    pub identity: DirectoryIdentity,
}
```

`entry` is `symlink_metadata`. `resolve` is one `realpath` followed by a
`stat` of the result: it returns the canonical path, every symlink in it
resolved, and `(st_dev, st_ino)`. Two paths name the same directory exactly
when their identities are equal, which also holds across a bind mount or a
macOS firmlink alias, where canonical path strings differ. `FileKind` is
named apart from `launch/core/tree_entry.rs`'s archive `EntryKind`.
`create_private_dir` is a `mkdir` with mode
`0700` that returns `AlreadyExists` unchanged. Every rule that reads
`modified_seconds` treats `None` as not aged for a removal and as fresh for
protection, and treats a future mtime as age zero, so an unreadable or skewed
mtime can only keep. `FilesystemCall` gains
`Entry`, `Resolve`, `CreatePrivateDir` and `SetModified`.

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRootKind {
    PluginBin,
    Relocated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheRoot {
    pub path: PathBuf,
    pub kind: CacheRootKind,
}

pub fn identify(
    config: &CacheRootConfig,
    filesystem: &dyn CacheFilesystem,
) -> Result<CacheRoot, ResolutionError>
```

`identify` calls `candidate`, then:

| Override | Override `resolve` | Plugin `bin` `resolve` | Kind |
|---|---|---|---|
| unset | not called | not called | `PluginBin` |
| set | fails | not called | `PluginBin` |
| set | resolves | fails or plugin root unset | `Relocated` |
| set | resolves | resolves, equal | `PluginBin` |
| set | resolves | resolves, different | `Relocated` |

The second row's doc comment is one sentence of why: claiming and pruning
are skipped until the cold path's `create_dir_all` makes the override
resolvable.

`main.rs::run` calls `identify` once per process, before `dispatch`, and
passes the result to the three sites that call `candidate` today (`:81`,
`:323`, `:417`). An identification error is carried, not raised, and is
surfaced only by a path that needs the root, so built-ins that never touch
the cache behave as today.

#### 3. Version claims

**File**: `cli/launcher/src/launch/outbound/resolve/ownership.rs` (new, a
leaf module)

```rust
pub trait Ownership {
    fn effective_uid(&self) -> u32;
}

pub struct ProcessOwnership;

pub const fn is_trusted_entry(entry: &EntryMetadata, uid: u32) -> bool

pub fn is_trusted_directory(
    path: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> bool
```

`is_trusted_entry` is the pure rule: `FileKind::Directory`, not a symlink, owned
by `uid`, and not group- or world-writable. It is the same write rule as the
existing `layout::is_privately_owned` (`mode & 0o022 == 0`); Phase 4 widens it
for a user-private group, when the inputs for that arrive. So `trees/`,
`trees/claims/`, `claims/` and lock directories cannot be renamed into by
another user even inside a safe root. `is_trusted_directory` reads `entry` and
applies it. `trees/claims/` is now created with `create_private_dir`. Version
claims, tree claims, tree prune and the residue sweep all use it.

**File**: `cli/launcher/src/launch/outbound/resolve/retention/version_claims.rs`
(new)

```rust
pub const VERSION_CLAIMS_DIR: &str = "claims";

pub struct VersionClaim<'a> {
    pub version: &'a str,
    pub launcher_id: &'a str,
}

pub fn refresh(
    root: &Path,
    claim: &VersionClaim<'_>,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
    clock: &dyn Clock,
)

pub struct FreshVersions { … }

pub enum ClaimsListing {
    Read(Vec<ListedClaim>),
    Unreadable(io::ErrorKind),
}

pub fn list(
    root: &Path,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> ClaimsListing
```

- **`refresh`.** It makes one `entry` call on the claim and returns if the
  claim is valid (see Validity) and younger than
  `CLAIM_WINDOW / REFRESH_FRACTION`. A young claim that is not valid falls
  through to the write. A failed write is logged with `tracing::debug!`, not
  `warn!`, because a read-only root is a supported case.
  Otherwise it calls `create_private_dir(claims)` and checks with `entry`
  that `claims/` is a directory, not a symlink, owned by
  `ownership.effective_uid()`. Only then does it call `write_private`, then
  `set_modified(claim, clock.now_seconds())`, so the refresh never relies on
  `O_TRUNC` of an empty file advancing its mtime. Every error is ignored.
- **Trusted directory.** `refresh` and `list` check `claims/` with
  `ownership::is_trusted_directory`.
- **`list`.** A `NotFound` listing is `Read(vec![])`. Any other listing error
  is `Unreadable`, so single-file prune can refuse to treat it as "no claims".
  A `claims/` directory that is not trusted is `Unreadable(PermissionDenied)`.
- **Parsing.** A claim name splits at its last `.` when what follows is
  exactly 16 lowercase hex characters, so a version's own dots survive.
- **Validity.** A claim counts as valid only when it is a regular file owned
  by the effective uid. `FreshVersions::within(window, clock)` is computed
  from the valid claims.

**File**: `cli/launcher/src/launch/outbound/resolve/tree/claims.rs`
**Changes**: in every root, `write_private` adds `custom_flags(O_NOFOLLOW)`, and
`refresh` writes only when `trees/claims/` passes
`ownership::is_trusted_directory(dir, &SystemFilesystem, &ProcessOwnership)`,
the same predicate every other trust check uses. Tree-claim refresh stays on
`std::fs`: it is on the dispatch path, and nothing records it. After the write,
`refresh` sets the claim's mtime to `clock.now_seconds()` with
`File::set_modified`.

**File**: `retention/root_retention.rs` (new)
**Changes**: the regime port, holding only what this phase's tests demand.
`CachePorts` is defined in `retention/ports.rs`:

```rust
pub trait RootRetention {
    fn claim_running_version(&self);
}

pub struct CachePorts {
    pub filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
    pub ownership: Arc<dyn Ownership + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
}

pub struct PluginBinRetention { root: Option<PathBuf>, ports: Arc<CachePorts> }

pub struct RelocatedRetention {
    root: PathBuf,
    running_version: String,
    plugin_root: Option<PathBuf>,
    ports: Arc<CachePorts>,
}

pub fn for_root(root: Option<&CacheRoot>, ports: Arc<CachePorts>)
    -> Arc<dyn RootRetention + Send + Sync>
```

`CachePorts` is the one bundle of ports `main.rs` builds; the resolvers and
both retentions borrow from it. It holds only what this phase needs; Phases
4, 5 and 7 add fields as their tests demand, and those are the retentions'
final fields. An identification error selects
`PluginBinRetention`, so a root that cannot be identified is never claimed
or pruned. `PluginBinRetention::claim_running_version` makes no call.
`RelocatedRetention` computes the launcher id and calls `refresh`. Computing
the id only there keeps its `current_exe` and hash off the default root's
path. Later phases add `prune`, `after_store` and `after_materialise`.

**File**: `cli/launcher/src/launch/start_up.rs` (new)

```rust
pub struct StartUp {
    cache_root: Result<CacheRoot, ResolutionError>,
    retention: Arc<dyn RootRetention + Send + Sync>,
    ports: Arc<CachePorts>,
}

pub fn start_up(config: &CacheRootConfig, ports: Arc<CachePorts>)
    -> StartUp

impl StartUp {
    pub fn dispatch_resolver(
        &self,
        config: DispatchConfig,
        held_leases: HeldLeases,
    ) -> Result<FetchVerifyCacheResolver, ResolutionError>;
    pub fn tree_resolver<'a>(&'a self, config: TreeConfig<'a>)
        -> Result<TreeResolver<'a>, ResolutionError>;
    pub fn retention(&self) -> &dyn RootRetention;
}
```

`TreeConfig<'a>` borrows the keys, fetcher and step observer the caller
builds lazily, as `run_cache` and `acquire_consumed_trees` do today; lease
liveness comes only from `CachePorts`, from Phase 4 on.

`start_up` calls `identify`, which resolves both roots once; a relocated
root is then carried as its canonical path, so every later step, the safety
walk included, uses a path with no symlink in it. It then calls `for_root`,
then `claim_running_version`,
all through `ports.filesystem`. It builds nothing else: the resolvers are
still built lazily, when a command first needs one, but only through
`StartUp`'s constructors. `run`, `LazyProductionResolver` (which now holds
`&StartUp`), `run_cache` and `acquire_consumed_trees` all use them, and so do
the recorded tests, so the recorded sequence is production's sequence.

**File**: `cli/launcher/src/main.rs`
**Changes**: `run` calls `start_up` before `dispatch`, for every command,
built-ins included, so a version used only
through `config` or `cache` still claims its launcher entry. Claim-before-
`find` holds because `run` precedes any resolution. `launcher_id()` moves to
`retention::launcher_id()`.

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs`
**Changes**: `ResolverConfig` keeps the cache root as a `PathBuf`, now taken
from `StartUp::cache_root`. The kind never reaches resolver logic.

#### 4. Tests (red first)

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
(unit tests): one test per row of the identity table. Two more show a
symlink and a trailing slash naming the plugin `bin` both resolving to
`PluginBin`. A third gives the two paths equal `DirectoryIdentity` through a
fake port, standing in for a bind mount, and expects `PluginBin`.

**File**: `cli/launcher/src/launch/outbound/resolve/retention/version_claims.rs`
(unit tests):

- `a_missing_claim_is_written`
- `a_claim_older_than_a_tenth_of_the_window_is_rewritten`
- `an_unknown_or_future_mtime_never_removes_or_unprotects`
- `refreshing_an_empty_back_dated_claim_moves_its_mtime_to_now`, on the real
  filesystem, so it runs on both the ubuntu and macos CI legs
- `a_younger_claim_is_not_rewritten` (sentinel bytes survive, as
  `claims.rs:208-235`)
- `a_read_only_root_never_fails_the_refresh`
- `a_symlink_or_foreign_owned_claim_is_not_valid`
- `a_young_foreign_owned_claim_does_not_suppress_the_refresh`: the
  `WritePrivate` attempt is recorded

- `is_trusted_entry` table (unit, `ownership.rs`): `0700` and `0755` pass;
  `0775` and `0777` fail; a foreign owner, a symlink and a non-directory
  fail

Both foreign-owned tests keep `claims/` owned by the effective uid and make
only the one claim file foreign. They use `ReportingFilesystem`, a
`CacheFilesystem` decorator in `resolve::filesystem::testing`, an
always-compiled `#[doc(hidden)] pub mod`, since integration tests link the
library without `cfg(test)` and features are not used as seams. It is inert
unless a test constructs it, and the launcher is outside `public-api:check`.
Its `reporting(path, EntryMetadata)` reports synthetic metadata for one
path, and from Phase 4 `reporting_acl(path, AccessAcl)` scripts the ACL
answer. Being always compiled, it is held to the workspace's production
lints: it recovers a poisoned lock with `into_inner` rather than
`unwrap`, documents its errors, and carries no blanket `#[allow]`. Phase
4's `ScriptedFilesystem` builds on
it, so unit and integration tests share one overlay; neither changes the
process's effective uid.
- `the_plugin_bin_retention_claims_nothing` (in `root_retention.rs`)
- `an_unidentifiable_root_selects_the_plugin_bin_retention`
- `a_symlinked_or_foreign_version_claims_directory_is_not_written_through`
- `a_claim_exactly_a_tenth_of_the_window_old_is_rewritten` (the boundary:
  `age >= CLAIM_WINDOW / REFRESH_FRACTION` rewrites, as `claims.rs:99-104`)
- `for_root_chooses_the_retention_from_the_root_kind` (in
  `root_retention.rs`)
- `claim_names_keep_the_versions_own_dots`
- `malformed_launcher_id_suffixes_do_not_parse`: 15 hex, uppercase hex, no
  dot
- `an_unreadable_claims_directory_is_reported_as_unreadable`
- `a_claims_directory_owned_by_another_uid_is_unreadable`

**File**: `tree/claims.rs` (unit tests):

- `a_symlinked_tree_claim_leaves_its_target_unmodified`
- `refreshing_an_empty_back_dated_tree_claim_moves_its_mtime_to_now`
- `a_symlinked_tree_claims_directory_is_not_written_through`

**File**: `cli/launcher/tests/resolution.rs` (recorded). Each test calls
`start_up` with a recorder, then `StartUp::dispatch_resolver`, the same
constructor `run` uses:

- `an_override_root_dispatch_writes_its_claim_before_find`: covers a missing
  claim and one older than `CLAIM_WINDOW/10`. The `WritePrivate(claims/…)`
  call precedes `List(root)`.
- `an_override_root_dispatch_with_a_young_claim_does_not_rewrite_it`
- `a_fresh_claim_costs_one_metadata_read_beyond_the_fixed_list`: the calls
  equal `[Resolve(InRoot("")), Resolve(Outside("plugin/bin")),
  Entry(InRoot("claims/…"))]` followed by `default_root_warm_dispatch_calls`.
  The young-claim path makes no trusted-directory read of `claims/`; that
  check runs only before a write.
- `naming_the_plugin_bin_adds_only_the_canonical_resolution`: the calls equal
  `[Resolve, Resolve]` followed by the fixed list, with no claim read or
  write.
- `an_unset_override_performs_exactly_the_fixed_calls`: the Phase 1 test,
  now composed with `identify`.
- `a_read_only_override_root_still_dispatches_warm`
- `an_entry_removed_between_find_and_exec_is_refetched_once` and
  `a_corrupt_entry_is_replaced_by_a_rename_with_no_unlink_before_it` become
  parameterised over both root kinds.
- `a_development_checkout_relocated_elsewhere_claims_its_version`: the plugin
  root is a temp "checkout" whose `bin/` differs from the override.

**File**: `cli/launcher/tests/` (built-launcher test):

- `a_built_in_command_refreshes_the_version_claim`: `accelerator version`
  with a scratch `ACCELERATOR_CACHE_DIR` leaves `claims/<version>.<16hex>`.

### Success Criteria:

#### Automated Verification:

- [ ] All Phase 3 tests pass and the fixed list is untouched:
      `mise run test:unit:cli`
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] A dispatch with `ACCELERATOR_CACHE_DIR` set in a scratch directory
      leaves `claims/<version>.<16hex>` and nothing else new

---

## Phase 4: Single-File Prune and Residue Sweep Behind the Verb

### Overview

Add the root lock, the allowlist grammar for top-level shapes, single-file
prune and the residue sweep. In a relocated root, `accelerator cache prune`
runs them under the root lock, waiting. No dispatch prunes yet.

### Changes Required:

#### 1. Top-level name grammar

**File**: `cli/launcher/src/launch/outbound/resolve/retention/root_entries.rs`
(new)
**Changes**: A pure classifier over one listing:

```rust
pub enum RootEntry<'a> {
    SubBinary { stem: &'a str },
    LauncherEntry { version: &'a str },
    ShimCopy { platform: &'a str, digest: &'a str },
    ShimStaging { pid: i32 },
    StoreStaging,
    LauncherStaging { pid: i32 },
    Probe,
    BootstrapLock,
    Unrecognised,
}

pub fn classify<'a>(name: &'a str, listed: &BTreeSet<&str>) -> RootEntry<'a>
```

| Shape | Variant |
|---|---|
| `<x>-<64hex>` with `<x>-<64hex>.minisig` listed | `SubBinary` |
| `accelerator-launcher-<v>-<p>`, `p` in `KNOWN_PLATFORMS`, with `.minisig` listed | `LauncherEntry` |
| `accelerator-verify-<p>-<64hex>`, `p` known | `ShimCopy` |
| `accelerator-verify-<p>-<64hex>.staging.<pid>` | `ShimStaging { pid }` |
| `.tmp-<x>-<64hex>-<digits>-<digits>[.minisig]` | `StoreStaging` |
| `.tmp-launcher-<pid>[.minisig]` | `LauncherStaging { pid }` |
| `.accelerator-probe-<digits>[-<digits>]` | `Probe` |
| `.accelerator-lock-<p>`, `p` known | `BootstrapLock` |
| anything else, including `.minisig` files, `claims`, `trees` | `Unrecognised` |

The sub-binary's `.minisig` and the launcher entry's `.minisig` are removed
with their entry and never classified alone. A staging name's `<pid>` is
parsed with Phase 4 §4's `parse_pid`; one that does not parse, or is below 1,
makes the name `Unrecognised`, so it is kept. The classifier tests name
`.tmp-launcher-0`, `.tmp-launcher--1`, `.tmp-launcher-abc` and
`accelerator-verify-<p>-<64hex>.staging.abc` among the `Unrecognised` cases.
`top_level_names_are_fixed_across_launcher_versions` pins every shape, as
`temp_entry_names_are_fixed_across_launcher_versions` does for `trees/`.

Four shapes are written by `bin/accelerator`, not by Rust:
`.tmp-launcher-$$[.minisig]`, `accelerator-verify-<p>-<sha>.staging.$$`,
`.accelerator-probe-$$` and `.accelerator-lock-<p>/pid`.
`tests/unit/tasks/test_bootstrap_name_coherence.py` extracts those templates
and the pid file's `printf` format from `bin/accelerator`, and asserts each
against the shape this table lists, so drift in the shell fails a test.

#### 2. Pure decisions

**File**: `retention/single_file.rs` (new)

```rust
pub struct ProtectionContext {
    pub fresh: FreshVersions,
    pub running_shims: RunningShims,
    pub live_bootstraps: LiveBootstraps,
}

pub fn protection(entry: &RootEntry<'_>, context: &ProtectionContext)
    -> Protection
```

`SingleFilePrune` builds one `ProtectionContext` per prune, through
`fresh_versions`, `running_shims` and `live_bootstraps` helpers, so `run`
reads as its numbered steps.

`protection` is the one place any entry's protection is decided. A
`SubBinary` is `Protected` when its stem ends with `-{v}-{64hex}` for some
fresh version `v`. A `LauncherEntry` is `Protected` when its version is
fresh. A `ShimCopy { platform, digest }` is `Protected` when `platform` is in
`KNOWN_PLATFORMS` and `running_shims` maps it to `digest`, or when that
platform's source could not be hashed. A `ShimStaging` or `LauncherStaging`
is `Protected` while `live_bootstraps` holds its pid: the bootstrap that
wrote it is still running, and on a wget host the file's mtime is the
server's upload time, not its age. Other variants are `Unprotected`. Names
are never split into name and version.

```rust
pub enum Disposition { Remove, Keep }

pub enum Protection { Protected, Unprotected }

pub enum Age { Aged, Young }

pub const fn disposition(
    entry: &RootEntry<'_>,
    protection: Protection,
    age: Age,
) -> Disposition
```

`RunningShims` maps each platform to
`sha256(${plugin_root}/bin/accelerator-verify-<platform>)`, or to
`Unhashable`. A copy matching its platform's source is the one every
bootstrap on that platform runs, even on another host sharing the root, so
removing it would race a live bootstrap into `fail_integrity`, which records
a lasting unverified-launcher notice.

| Entry | Rule |
|---|---|
| `SubBinary`, `LauncherEntry` | remove when unprotected and the entry and `.minisig` are both aged |
| `ShimCopy` | remove when unprotected and aged; the running plugin's copy for its platform is protected |
| `ShimStaging`, `LauncherStaging` | remove when unprotected and aged |
| `StoreStaging` | remove when aged; the launcher wrote it, so its mtime is local |
| others | keep (the residue sweep owns `Probe`, `BootstrapLock`) |

#### 3. Single-file prune

**File**: `retention/single_file.rs`
**Changes**: `SingleFilePrune::run(&self) -> Reclaimed`, which the root
lock's proof guards. It runs in this order:

1. `steps.reached(PruneStep::AboutToList)`
2. List the root once, then `steps.reached(PruneStep::Listed)`.
3. Build `RunningShims`: for each platform with a `ShimCopy` in the listing,
   hash `${plugin_root}/bin/accelerator-verify-<platform>` through the
   filesystem port. That is one hash of about 480 KB on a single-platform
   root and at most four on a mixed one, once per prune, on cold paths and in
   the verb, never on the warm path. When the plugin root is unset, or a
   platform's source cannot be read, that platform's copies are all kept.
   Build `LiveBootstraps` by asking `BootstrapHolderLiveness` about each pid
   that a listed staging name carries. A dead pid reused by another process
   reads as live, which only keeps the file longer.
4. Read claims with `version_claims::list`. If the listing is `Unreadable`,
   skip the `SubBinary` and `LauncherEntry` removals, and report
   `claims unreadable ({kind}); kept every versioned entry` in the
   `PruneReport`. Shim copies and staging residue are still removed.
5. Remove each listed entry whose disposition is `Remove`: the `.minisig`
   first, then the entry, so a concurrent `find` stops matching before the
   binary goes.
6. Remove stale version claims. A stale claim is a valid claim, with a name
   that parses as `<version>.<16hex>`, not refreshed within the window. The
   rule is a pure function with a table test:

   ```rust
   pub const fn stale_claim_disposition(
       has_other_fresh_claim: bool,
       every_entry_removed: bool,
   ) -> Disposition
   ```

   It removes the claim when the version still has a fresh claim, and
   otherwise only if every listed entry of that version was removed in step
   5. Before unlinking, the step re-reads the claim's `entry` and skips it if
   it is now fresh.

Removing only listed entries after reading claims is what makes a concurrent
`store` safe: an entry stored after listing is never touched, and one stored
before listing was preceded by its claim. The struct's doc comment states this
invariant.

#### 4. Residue sweep

**File**: `retention/residue.rs` (new)

```rust
pub trait BootstrapHolderLiveness {
    fn is_alive(&self, pid: i32) -> bool;
}

pub struct KillZeroProbe;
```

`KillZeroProbe` sends `kill(pid, 0)` through `rustix`. `ESRCH` means dead;
any other result, including `EPERM`, means alive.

`CacheFilesystem` gains `remove_dir`, and `FilesystemCall` gains `RemoveDir`.

`ResidueSweep::run` uses the same listing, under the root lock:

- **Probe files.** A `Probe` older than `AGE_BACKSTOP` is removed.
- **Bootstrap lock directories.** A `BootstrapLock` that is a trusted
  directory, older than `AGE_BACKSTOP`, whose `pid` is missing, empty, or
  names a dead process is removed with `remove_file(pid)` then `remove_dir`.
  An untrusted one is kept, and so is one whose `pid` does not parse.
- **Pid grammar.** `parse_pid(contents) -> Option<i32>` trims trailing ASCII
  whitespace, because the bootstrap writes `printf '%s\n' "$$"`, and accepts
  only an integer of 1 or more. `0` and negative values are unparseable:
  `kill` gives them process-group meanings.
- **Re-check before unlinking.** Immediately before `remove_file(pid)` the
  sweep re-reads the directory's `entry` and the `pid` contents, and skips the
  removal if the mtime moved, the directory is no longer older than
  `AGE_BACKSTOP`, or the pid changed. A bootstrap reclaiming the same
  directory between decision and unlink is then kept.

#### 5. The root lock and the lock seam

**File**: `resolve/single_flight.rs` (new, a leaf module) for the trait and
its values, and `tree/locks.rs` (new) for `FlockLocks`, the production
implementation over the `lease.rs` primitives, so the leaf imports nothing
from `tree/`

```rust
pub trait SingleFlightLocks {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError>;
    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError>;
    fn wait_existing(&self, path: &Path)
        -> Result<Option<LockGuard>, TreeError>;
    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError>;
}

pub struct LockGuard(Box<dyn Send>);

pub enum LockProbe { Taken(LockGuard), Held, Absent }

pub enum LockPatience { Wait, Try }
```

```rust
// tree/locks.rs
pub struct FlockLocks;
```

`SingleFlightLocks` is the path-level primitive, and `CachePorts::locks` is
its one instance, serving the root lock and, from Phase 5, tree locks.
`LockGuard` wraps whatever guard an implementation returns and releases it on
drop, so a test decorator can wrap the inner guard and see the release.
Retention never names the primitive outside two types: `HeldRootLock::acquire`
here and `TreeLocks` in Phase 5. The root lock opens with `O_NOFOLLOW` and
`fstat`s the result. In a root only its owner can write, a symlinked or
foreign-owned `.accelerator-prune.lock` can only be the same user's doing,
so it is a `LockFailure`, reported by the verb and skipped on cold paths;
prune never unlinks a lock it did not create.

**File**: `retention/root_lock.rs` (new)

```rust
pub struct HeldRootLock { _guard: LockGuard }

pub const ROOT_LOCK: &str = ".accelerator-prune.lock";
```

`wait_existing` is a new `tree/lease.rs` function: a blocking
`lock_confirmed(path, open_existing, LockExclusive)`, where `None` means the
file is absent. `Locked::KeptBeingReplaced` maps to `Err`, so the verb reports
it and a cold path skips the artifact; it never reaches the lock-free arm.
`wait_existing_maps_a_churning_lock_to_an_error` pins this beside `take_outcome`
and `try_outcome`. `HeldRootLock::acquire(root, patience, locks)` returns
`Result<Option<HeldRootLock>, TreeError>`.

Lock order is one rule: the root lock is released before any tree lock is
taken or waited on, and a holder of a tree lock only ever tries the root
lock. `RootPrune::run` holds `HeldRootLock` in a block around single-file
prune and the residue sweep, so it drops before tree prune starts.

#### 6. Prune steps seam

**File**: `resolve/prune_report.rs` (new, a leaf module), so both prunes can
report steps without importing each other

```rust
pub enum PruneStep { AboutToList, Listed, RootLockHeld, TreeLockHeld }

pub trait PruneSteps {
    fn reached(&self, step: PruneStep);
}

pub struct NoPruneSteps;
```

#### 7. `RootPrune` and the verb

**File**: `cli/launcher/src/launch/outbound/resolve/ownership.rs`
**Changes**: `is_trusted_entry` gains a `private_gid: Option<u32>`
parameter and admits a group-writable entry whose group is `private_gid`;
`private_gid` is the effective gid when `primary_group_is_private()` holds,
otherwise `None`. `Ownership` gains two methods:

```rust
pub trait Ownership {
    fn effective_uid(&self) -> u32;
    fn effective_gid(&self) -> u32;
    fn primary_group_is_private(&self) -> bool;
}
```

`ProcessOwnership::primary_group_is_private` makes one `getpwuid_r` and one
`getgrgid_r` through a direct `libc` dependency, since `rustix` exposes no
group database. It holds when the effective gid's group is named as the
user's login and its member list is empty or holds only the user: the
user-private-group convention of Ubuntu, Debian, Fedora and RHEL. It never
enumerates the password database, which can be slow under LDAP or SSSD.
`cli/launcher/Cargo.toml` adds `libc = { workspace = true }`; `libc` is
already a workspace dependency, so no `deny.toml` or cargo-pup change is
needed.

The rule itself is a pure function, so it is tested without the system's
databases:

```rust
pub fn is_private_group(
    login: &str,
    group_name: &str,
    members: &[&str],
) -> bool
```

`ProcessOwnership` resolves the login and group through an `AccountLookup`
seam with two arms, `LibcLookup` and `GetentLookup { executables, timeout }`,
so tests point the second at stub scripts. It tries them in up to three
steps, stopping at the first that answers, and caches the answer in a field
of the `ProcessOwnership` instance `main.rs` builds once, never a static, so
tests in one binary cannot see each other's answers:

1. `getpwuid_r` and `getgrgid_r`, growing their buffers on `ERANGE` up to a
   fixed bound. (NixOS and Guix install `getent` outside the fixed paths of
   step 2, so their directory-service accounts reach step 3; see Accepted
   Risks.)
2. On Linux, when step 1 finds no entry, `getent passwd <uid>` and
   `getent group <egid>`, run from the fixed root-owned paths `/usr/bin/getent`
   then `/bin/getent`, never through `PATH`, with a 2 s timeout. The child runs
   with stdin null, `env_clear()` plus `LC_ALL=C`, in its own process group,
   under a small launcher-local supervisor that kills the group and reaps the
   child on timeout. `config-adapters`' `command_runner` is not reused: its
   `Supervision` is private and its public runner runs `bash -c` found through
   `PATH`, which `pup.ron` already forbids the launcher from naming. An answer
   counts only when it is exactly one line under 64 KiB whose numeric id field
   equals the queried uid or egid; a non-zero exit (2 is "not found"),
   unparseable or oversized output and the timeout all fall to step 3. The
   accepted lines feed the same `is_private_group` rule. This step exists
   because the shipped Linux launchers are static musl builds, whose lookups
   read only `/etc/passwd`, `/etc/group` and `nscd`, so SSSD, FreeIPA, Active
   Directory, LDAP and systemd-homed accounts are invisible to step 1. `getent`
   goes through the host's NSS.
3. Otherwise the answer is "not private", and the refusal names the cause
   (`could not resolve group <gid>`) and the remedy `chmod g-w <root>`.

A lookup that fails can only refuse. No warm path runs any of this: it runs
only on a prune of a group-writable relocated root.

The private-group exception applies only to a directory with no access ACL,
because on Linux a directory's group mode bits are its ACL mask when it has one,
so an entry such as `user:bob:rwx` would show as group-writable. The check is a
`CacheFilesystem::access_acl` port method, so the overlay can test it, returning
`AccessAcl::{None, Posix, Other}`; on macOS the adapter classifies the entries
itself and returns `Other` only for an allow-write entry, so the overlay scripts
the same value the adapter produces. On Linux it uses `lgetxattr`:
`system.posix_acl_access` present is `Posix`, and `system.nfs4_acl` or any other
ACL attribute, as on NFSv4, SMB or ZFS with `acltype=nfsv4`, is `Other`;
`ENODATA` or `ENOTSUP` for every ACL attribute is `None`. A `Posix` directory
falls back to the strict `mode & 0o022 == 0` rule, and an `Other` directory is
refused, since its mode bits only approximate access. On macOS, where ACLs never
appear in mode bits, the root and every ancestor are read with
`acl_get_link_np`, and a directory is refused when an allow entry grants write,
`add_file`, `add_subdirectory` or `delete_child` to anyone but the owner; deny
entries, such as a home directory's `everyone deny delete`, are ignored.

A directory is *safe to prune in* when its `entry` reports a directory owned
by the effective uid or by root, and it is not writable by others unless
one of these holds:

| Directory | Writable by group | Writable by world |
|---|---|---|
| root | safe only when its group is the effective gid and `primary_group_is_private()` | never safe |
| ancestor | safe through the user's private group, or with the sticky bit | safe with the sticky bit |

The root itself must also be owned by the effective uid; only its ancestors may
be owned by root, as `/` and `/tmp` are, or by a uid the operator lists in
`ACCELERATOR_CACHE_TRUSTED_OWNERS`. A directory's owner can always rename or
`chmod` what it holds, so an ancestor owned by anyone else could swap the root;
relaxing this for an effective uid of 0 would let an ordinary user steer root's
deletions, so the widening is explicit instead. A CI job whose workspace is
owned by its runner (GitHub Actions' `/__w`, uid 1001) or a container whose
bind mount is owned by the host user lists that uid. Listing a uid trusts that
account fully: as an ancestor's owner it can rename or `chmod` within it, so it
could redirect prune's removals within the user's privileges. The overflow uid
65534 and uid 99 stand for every unmapped host account and every local user,
so they are refused even when listed, and the refusal never offers them; a
root on a volume mounted with ownership ignored (`MNT_IGNORE_OWNERSHIP` in
`statfs` on macOS), where permission bits are not enforced, is refused
outright. The variable is
comma-separated decimal uids, parsed strictly; a malformed value is ignored
with a warning, so it can only refuse.

`start_up` reads it once into a `TrustedOwners` value passed to
`SafePruneRoot::check`, so tests set it through that argument rather than the
environment. A cache root is therefore pruned only
when it is its owner's alone. The sticky bit stops other users renaming entries,
but not creating names first, and several writers in the root use predictable
names and follow what they find there: the bootstrap's probe, shim staging and
launcher staging, and the launcher's own probe. So a root any other user can
create names in is refused, sticky or not, while a sticky `/tmp` above a
per-user root is fine. A shared cache such as a `/tmp/accelerator` or
`/var/cache/accelerator` made writable by other users still works as a cache,
but is never pruned; a per-user root beneath it, such as
`/var/cache/accelerator/$USER`, is.

`SafePruneRoot::check` applies that rule to the canonical root and to each
of its ancestors up to `/`, one `entry` per path component, as sshd's
`StrictModes` does after its `realpath`. The root is the canonical path
`start_up` resolved, so it holds no symlink: macOS `/var` and `/tmp`, or a
Linux `/home` that is a link, are walked as their targets, `dir-link` and
`dir-link/` get the same verdict, and no link is trusted for anything. In a
directory another user can write to without these exceptions, that user can
rename `trees/`, or the root itself, and put a symlink in its place at any
moment, so every per-directory check would be checking the attacker's
choice.

`UnsafeRoot` names the path that failed and why: group-writable by a named
shared group, world-writable without the sticky bit, or owned by a named
user. Each comes with its remedy, such as `chmod g-w <path>`.

**File**: `retention/ports.rs`, `CachePorts`' one home from Phase 3 on

```rust
pub struct CachePorts {
    // ... Phase 3's filesystem, ownership, clock
    pub bootstrap_holders: Arc<dyn BootstrapHolderLiveness + Send + Sync>,
    pub locks: Arc<dyn SingleFlightLocks + Send + Sync>,
    pub steps: Arc<dyn PruneSteps + Send + Sync>,
    pub lease_liveness: Arc<dyn LivenessProbe + Send + Sync>,
    pub pins: Arc<dyn PinnedDigests + Send + Sync>,
}
```

**File**: `retention/root_prune.rs` (new)

```rust
pub struct RootPrune<'a> {
    pub safe: &'a SafePruneRoot,
    pub running_version: &'a str,
    pub plugin_root: Option<&'a Path>,
    pub claim_window: Duration,
    pub ports: &'a CachePorts,
}

pub struct PruneReport {
    pub reclaimed: u64,
    pub lock_failures: Vec<LockFailure>,
    pub unreadable_claims: Vec<UnreadableClaims>,
    pub removal_failures: Vec<RemovalFailure>,
}
```

`LockFailure`, `UnreadableClaims` and `RemovalFailure` are defined in the
`prune_report` leaf, so single-file and tree prune can both produce them;
`PruneReport` aggregates them.

```rust

impl RootPrune<'_> {
    pub fn run(&self, patience: LockPatience, held_leases: &HeldLeases)
        -> PruneReport
}
```

`RootPrune` prunes `safe.path()` and nothing else, so the path that was
checked is the path that is pruned. Until Phase 5 replaces it with
`TreePrune`, `run` ends by calling `tree::prune_pinned_on_host` for the root,
so a relocated root's verb keeps today's tree prune when this phase ships
alone.

This phase's `run` covers the root lock and its two prunes. Phase 5 adds tree
prune. The root-safety check is a proof value, defined in the `ownership`
leaf beside `is_safe_prune_root` so `tree/` can require it, rather than a
call each site must remember:

```rust
pub struct SafePruneRoot { root: PathBuf }

impl SafePruneRoot {
    pub fn check(
        root: &Path,
        filesystem: &dyn CacheFilesystem,
        ownership: &dyn Ownership,
        trusted_owners: &TrustedOwners,
    ) -> Result<SafePruneRoot, UnsafeRoot>;
    pub fn path(&self) -> &Path;
}

pub enum PruneOutcome { Refused(UnsafeRoot), Ran(PruneReport) }
```

`check` takes only the two leaf ports it uses, so the `ownership` leaf depends
on nothing in `retention/`. `RootPrune`, `TreePruneRegime::WholeTree` and, in
Phase 7, `TreePrune::prune_held` and a relocated root's `ReapPermit` each
require a `&SafePruneRoot`, so none can be built without the check.
`RelocatedRetention` obtains it once per prune; an unsafe root yields
`PruneOutcome::Refused` and removes nothing. Claims are still written, so a root
made safe later keeps its protection. `FreshVersions` always contains
`running_version`, whatever the claims say, as tree prune always spares the
running launcher's pinned digest. `RootPrune` never asks which kind of root it
is in: only `RelocatedRetention` constructs one.

**File**: `retention/root_retention.rs`
**Changes**: `RootRetention` gains
`fn prune(&self, older_than: Option<Duration>) -> PruneOutcome`.
`PluginBinRetention` always returns `Ran`.

| Retention | `prune` |
|---|---|
| `PluginBinRetention` | `tree::prune_pinned_on_host`, today's loop over pinned artifacts on the host |
| `RelocatedRetention` | `RootPrune::run(LockPatience::Wait)` with `claim_window` set to `older_than` when given |

`older_than` replaces `CLAIM_WINDOW` for version and tree claims alike, as
the verb does today. A value above 14 days widens the window and keeps more.
The verb passes an empty `HeldLeases`, since it consumes no trees.

**File**: `tree/resolver.rs` and `launch/cache.rs`
**Changes**: the per-artifact body of `TreeResolver::prune`
(`reclaim_unclaimed` then `reap_orphans`) moves unchanged into
`tree::prune_pinned_artifact(paths, artifact, collaborators, older_than)`
over plain collaborators: clock, lease liveness and pins. It keeps today's
static tree lock until Phase 5 introduces `TreeLocks`. The verb's loop over
`pins::artifact_names()` on `HOST_PLATFORM` moves from `launch/cache.rs`
into `tree::prune_pinned_on_host`, which calls it per artifact.
`TreeResolver::prune` delegates to `prune_pinned_artifact` until Phase 5
deletes them, and `PluginBinRetention::prune` and Phase 4's `RootPrune` call
`prune_pinned_on_host` with their root and `CachePorts`, so no retention
holds a resolver and nothing waits on the lazy tree-resolver construction in
`run_cache`.

**File**: `cli/launcher/src/launch/cache.rs`
**Changes**: `prune` calls `retention.prune(older_than)` with no branching.
It writes `reclaimed {n} entries`, then a line per lock failure, per
unreadable claims directory and per removal failure on stderr. For an unsafe
root it prints the `UnsafeRoot` reason and remedy instead, for example
`not pruning: <path> is group-writable by the shared group staff; run chmod
g-w <path>`. Cold paths log it once per process, not on every prune. It
returns `kernel::Error::Failed` when a lock or a removal failed, or the root
is unsafe. An unreadable claims
directory alone exits 0, because it removed nothing wrongly. `cache::run`
takes `&dyn RootRetention` beside the tree resolver, and `main.rs::run_cache`
passes the one chosen in `run`.

`n` counts removed entries. A sub-binary or launcher entry counts once with
its `.minisig`, and a generation once with its sidecars. A claim, a residue
file and a lock file each count once. A `RemovalFailure` holds the path and
`io::ErrorKind` of any removal error other than `NotFound`.

Every removal emits `tracing::debug!(path, rule)`, where `rule` names the
decision-table row that allowed it, so a deletion can be traced to its cause.
Tree prune and the cold paths do the same.

#### 8. Tests (red first)

Unit tests in `root_entries.rs` and `single_file.rs` cover every row of both
tables. Integration tests in a new `cli/launcher/tests/root_prune.rs` use a
`RootHarness` that seeds names, back-dates them with `File::set_modified`
and the `FixedClock`, and drives `RootPrune` and `cache::run`. It
creates its root and every directory it seeds (`claims/`, `trees/`,
`trees/claims/`, lock directories) with explicit modes, so the developer's
umask and group database cannot decide a result; every unsafe-root and
private-group test, here and in Phase 7, goes through the overlay and a fake
`Ownership`. It canonicalises its tempdir and asserts, as a precondition, that
`SafePruneRoot::check` accepts it, so a test never passes or fails on the
host's temp-path ancestry.

Ownership and modes a test cannot create for real (a foreign-owned claim, a
root-owned ancestor, a shared group) come from a metadata overlay on the
test filesystem, `ScriptedFilesystem::reporting(path, EntryMetadata)`, which
makes `entry()` report synthetic metadata for that one path. The root lock's
ownership check reads through `ports.filesystem.entry` before opening, so
the overlay reaches it too. Every refusal test asserts that `UnsafeRoot`
names the exact path and reason it targets, so it cannot pass on an
unrelated refusal. Each item's
criterion maps to a test:

- `only_the_unclaimed_versions_entries_are_removed` (A unclaimed, B claimed)
- `two_claimed_versions_lose_nothing`
- `a_symlinked_and_a_foreign_owned_claim_protect_nothing` (through the
  overlay, on the claim file only)
- `pre_claim_entries_younger_than_the_backstop_survive` (W: launcher entry
  and one sub-binary young, one old)
- `a_version_is_matched_by_suffix_never_by_splitting_the_name`
  (`x-1.24.0-pre.78-<hex>` kept, `x-y-1.24.0-<hex>` removed)
- `stale_claims_go_independently_and_a_last_claim_goes_after_its_entries`:
  this test records the calls and asserts that W's `RemoveFile` calls
  precede `RemoveFile(claims/W.…)`.
- `plugin_staging_entries_are_removed_only_once_aged`
- `launcher_staging_with_a_live_pid_survives_however_old`: back-dated by a
  day, named with a pid the seam reports alive
- `launcher_staging_with_a_dead_pid_goes_once_aged`
- `shim_staging_with_a_live_pid_survives_however_old` and
  `shim_staging_with_a_dead_pid_goes_once_aged`
- `the_running_plugins_shim_copies_survive_for_every_platform`: a host copy
  and an other-platform copy, each matching its own source, survive
- `stale_shim_copies_go_on_every_platform`: a host-platform copy and an
  other-platform copy, each with another digest, and a copy whose digest
  matches a different platform's source, are all removed once aged, and the
  same copies freshly staged survive; shim staging residue goes only once
  aged
- `an_unreadable_shim_source_keeps_that_platforms_copies`
- `the_running_version_survives_older_than_zero`
- `the_running_version_survives_without_a_valid_claim`
- `older_than_narrows_the_version_claim_window` (another version's claim, 2
  days old, with `--older-than 86400`, and without the flag)
- `older_than_beyond_the_window_widens_it`: a 20-day-old claim survives
  `--older-than` of 30 days
- `an_entry_exactly_at_the_backstop_is_aged` and
  `a_claim_exactly_at_the_window_is_stale` (the boundaries: `age >=
  AGE_BACKSTOP` is aged and `age < window` is fresh, as `reap.rs:419-426` and
  `claims.rs:76`)
- `foreign_names_survive_every_trigger`: foreign `.tmp-*`, an unpaired
  `*-*-<64hex>`, a foreign dotfile, an old foreign file in `claims/`, and,
  from Phase 5, a foreign file, a foreign directory, a non-generation `.ref`
  and an old foreign file in `trees/claims/`. An unclaimed aged version's
  entries are removed in the same run as the positive control. Phase 4 runs
  it through the verb; Phase 7 adds the cold store and the materialise.
- `the_kept_names_survive_beside_an_unclaimed_version`: a live-`pid` lock
  directory, a young probe, `.accelerator-unverified.log`, `trees/`, an
  unknown-platform launcher entry, a lone `.minisig`
- `an_unreadable_claims_directory_removes_no_entries`: the verb names the
  error kind on stderr and exits 0
- `old_probe_files_are_swept_and_young_ones_kept`
- `stale_bootstrap_locks_with_empty_or_dead_pids_are_swept`: the
  `pid`-liveness seam, a missing `pid`, a live `pid` and a young dead one.
  Every fixture writes the pid as bash does, `"<pid>\n"`.
- `an_unparseable_or_non_positive_pid_keeps_the_lock_directory`: `abc`, `0`,
  `-1`
- `a_lock_directory_reclaimed_before_the_unlink_is_kept`: `ScriptedFilesystem`
  rewrites the pid on the second read of it
- `kill_zero_probe_maps_errno_to_liveness` (unit, `residue.rs`): the current
  pid is alive; a spawned and reaped child's pid is dead; pid 1 is alive
  through `EPERM`, skipped when running as root
- `a_symlinked_bootstrap_lock_leaves_its_target_untouched`
- `a_held_tree_temp_generation_is_untouched_by_single_file_prune`
- `a_store_after_listing_survives_a_paused_prune`: `PruneSteps` pauses at
  `Listed` through a channel handshake while another thread claims and
  stores.
- `a_store_before_listing_survives_a_paused_prune`: pauses at `AboutToList`.
- `a_store_without_a_claim_behind_a_paused_prune_is_removed`: the control
  showing the two tests above can fail. It pauses at `AboutToList`.
- `single_file_prune_lists_the_root_before_reading_claims`: the recorder
  shows `List(InRoot(""))` before the first call touching `claims/`. This is
  the ordering the concurrent-store argument rests on; the paused tests
  alone cannot catch a reordering.

In all three, the prune's `FixedClock` stands `AGE_BACKSTOP` plus ten minutes
past the stored entry's mtime and far inside `CLAIM_WINDOW` of its claim, so
only the claim protects the entry, with a margin no scheduling delay can
eat.
- `the_verb_waits_for_a_held_root_lock`: the lock is held from another
  thread, the verb blocks, then prunes once the lock is released (`within`).
- `the_verb_reports_a_failed_root_lock_and_exits_non_zero`: a `FailingLocks`
  seam returns `ENOLCK`.
- `is_private_group` table: empty member list, self only, another member,
  name mismatch (unit, `ownership.rs`)
- `the_process_ownership_answers_for_the_current_user`: a smoke test whose
  answer is compared with an `id -un` and `id -gn` oracle on both legs, plus
  the member list from `getent group` on Linux and
  `dscacheutil -q group -a gid <gid>` on macOS
- `is_trusted_entry` widening table: `0775` with the private gid passes,
  `0775` with another gid or `private_gid` `None` fails, `0777` always fails
- `a_private_group_writable_bootstrap_lock_is_swept` and
  `a_shared_group_writable_bootstrap_lock_is_kept`: the bootstrap's
  `mkdir`-created lock directory is `0775` under umask 002, the realistic
  case for the widening
- `a_root_writable_by_another_private_looking_group_is_refused`: the root's
  group is not the effective gid while `primary_group_is_private` holds; the
  refusal names that group
- `a_private_group_writable_ancestor_is_accepted` and
  `a_shared_group_writable_ancestor_is_refused` (through the overlay)
- `the_getent_fallback_times_out_to_a_refusal`: the `getent` arm pointed at
  a stub that sleeps falls to step 3 within a short test timeout, and the
  stub's pid answers `ESRCH` afterwards
- `the_production_getent_executables_are_fixed_absolute_paths`: the default
  list is exactly `/usr/bin/getent` and `/bin/getent`
- `the_getent_child_runs_with_a_cleared_environment`: a stub that dumps its
  environment sees only `LC_ALL=C`

Each stub test asserts, through a marker file the stub writes, that its stub
actually ran, so none passes on an answer it never asked for.
- `the_getent_fallback_rejects_mismatched_or_malformed_answers`: a wrong id
  field, two lines and oversized output each fall to step 3
- `an_acl_granting_another_user_write_is_not_private` (through the overlay's
  `access_acl`), and a macOS ancestor with an allow-write ACE is refused while
  a deny-only ACE is accepted
- `the_system_filesystem_reports_acls` (real filesystem): a fresh tempdir
  reports `None` on both legs. On macOS, `chmod +a "everyone deny delete"`
  reports `None`, as a home directory does, and `chmod +a "everyone allow
  add_file"` reports `Other`. On Linux, `setfacl -m u:nobody:rwx` reports
  `Posix`; without `setfacl` the test skips locally but fails under CI, as
  `minisign_or_skip!` does (`tree_resolution.rs:83-96`), and the CI job
  installs the `acl` package
- `the_getent_fallback_parses_passwd_and_group_lines` (unit, over captured
  lines)
- `the_getent_fallback_is_used_only_when_libc_has_no_entry`: a seam over the
  lookup order
- `an_unresolvable_account_refuses_with_the_cause_and_remedy`: through the
  ownership seam
- `the_getent_fallback_resolves_the_current_user` (Linux only, a smoke
  test)
- `a_root_reached_through_a_symlinked_ancestor_is_pruned`: the macOS
  `/var`-style case, built with a symlinked intermediate directory and
  driven through `start_up` with the non-canonical path, so it fails if
  `start_up` stops canonicalising
- `a_sticky_world_writable_ancestor_is_accepted` and
  `a_root_owned_ancestor_is_accepted` (through the overlay)
- `a_symlinked_root_with_an_unsafe_target_is_not_pruned`
- `a_symlinked_or_foreign_root_lock_is_a_lock_failure` (through the
  overlay): the lock file and the symlink's target are untouched
- `a_relocated_root_verb_still_reclaims_an_unclaimed_superseded_pinned_tree`:
  Phase 4's `RootPrune` keeps today's tree prune until Phase 5
- `a_shared_group_writable_root_is_not_pruned_automatically`
- `a_user_private_group_writable_root_is_pruned` (through the ownership
  seam)
- `a_symlinked_root_is_pruned_through_its_target`, with and without a
  trailing slash, through `start_up` with the link's path
- `a_root_under_a_writable_ancestor_is_not_pruned`
- `the_unsafe_root_message_names_the_condition_and_remedy`
- `an_ancestor_owned_by_a_trusted_owner_is_accepted`
- `an_ancestor_owned_by_an_untrusted_owner_names_the_variable`: the refusal
  prints `ACCELERATOR_CACHE_TRUSTED_OWNERS=<uid>`
- `a_trusted_owner_does_not_admit_a_writable_ancestor`
- `a_trusted_owner_does_not_admit_a_foreign_root`
- `a_malformed_trusted_owners_value_only_refuses`
- `TrustedOwners` parser table: empty, a leading space, a trailing comma,
  `-1`, hex and overflow are all malformed; `1001,1000` parses
- `the_overflow_uid_and_uid_99_are_refused_even_when_listed`
- `a_built_launcher_warns_on_a_malformed_trusted_owners_value`: shows
  `start_up` reads the variable
- `a_listed_owner_reaches_the_relocated_prune`: `StartUp` built with an
  injected `TrustedOwners` and an overlay ancestor owned by the listed uid
  prunes through `RelocatedRetention`; the unlisted case is refused
- `a_user_owned_world_writable_root_is_not_pruned`: a `1777` root the user
  owns is refused, naming `chmod 700` and the per-user remedy
- `a_per_user_root_under_a_sticky_shared_parent_is_pruned`: the
  `/var/cache/accelerator/$USER` layout
- `a_root_owned_sticky_root_is_not_pruned` (through the overlay, on the
  canonical root only), with the refusal naming the owner and the per-user
  remedy
- `a_root_owned_0755_root_is_not_pruned` (through the overlay): owner 0 on
  the root itself is refused, though ancestors may be root-owned
- `a_root_owned_by_another_uid_is_not_pruned` (through the overlay, on the
  canonical root only)
- `the_verb_refuses_an_unsafe_root_and_exits_non_zero`
- `claims_are_still_written_in_an_unsafe_root`
- `the_verb_reports_a_failed_removal_and_exits_non_zero`: `ScriptedFilesystem`
  fails one `remove_file` with `PermissionDenied`
- `the_reclaimed_count_counts_an_entry_and_its_signature_once`

A `ScriptedFilesystem` decorator in `tests/common/` scripts the nth call of a
given kind on a given path:

```rust
impl ScriptedFilesystem {
    pub fn failing(self, call: CallKind, path: &Path, nth: usize,
        error: io::ErrorKind) -> Self;
    pub fn before(self, call: CallKind, path: &Path, nth: usize,
        action: impl Fn() + Send + Sync + 'static) -> Self;
    pub fn fired(&self) -> Vec<ScriptedEvent>;
}
```

`CallKind` is `FilesystemCall::kind()`, not a hand-kept parallel enum. The
recorder, `ScriptedFilesystem` and the recording `SingleFlightLocks` append
to one shared timeline, the recorder's `Arc<Mutex<Vec<_>>>`, as
`FilesystemCall`, `ScriptedEvent` and `LockReleased(path)` entries, so
ordering between them can be asserted. Each test asserts through `fired()`
that its script ran, and through that timeline that the scripted event sits
immediately before the guarded call, so it can
pass neither because the script never ran nor because it ran before the
decision. It drives the defensive branches:

- `a_claim_refreshed_before_unlink_is_kept`: refreshes the claim's mtime
  before its second `entry`, the re-read just before the unlink
- `a_last_stale_claim_survives_an_unremoved_entry`
- `a_failed_removal_keeps_the_versions_claim`
- `a_removal_failing_with_enoent_is_not_an_error`

### Success Criteria:

#### Automated Verification:

- [ ] All Phase 4 tests pass: `mise run test:unit:cli`
- [ ] Default-root verb tests in `tests/tree_resolution.rs` pass unchanged
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] `accelerator cache prune` in a scratch relocated root seeded with an
      old version's entries removes them and prints the count

---

## Phase 5: Widened Tree Prune Behind the Verb

### Overview

In a relocated root, tree prune visits every artifact and platform found
under `trees/`. It reclaims unclaimed trees regardless of artifact name,
platform or supersession, and reaps stale tree claims, orphan sidecars and
orphan tree locks. In every root, the verb stops creating tree locks for
artifacts it never materialised.

### Changes Required:

#### 1. Artifact discovery

**File**: `cli/launcher/src/launch/outbound/resolve/tree/layout.rs`
**Changes**:

```rust
pub enum TreesEntry<'a> {
    Pointer { artifact: Artifact, digest: &'a str },
    Generation(GenerationName<'a>),
    Sidecar { artifact: Artifact, generation: &'a str },
    TempResidue { artifact: Artifact },
    Lock { artifact: Artifact },
    Unrecognised,
}

pub fn classify_trees_entry(name: &str) -> TreesEntry<'_>
```

`TreesEntry` is named for the `trees/` index, apart from
`launch/core/tree_entry.rs`'s archive entries. `classify_trees_entry` is the
one classifier for names under `trees/`, as
`root_entries::classify` is at the top level. It recognises:

- pointers: `<n>-<p>-<64hex>.ref`
- generations of any layout version: `<n>-<p>-<64hex>-<u32>-<16hex>`, through
  Phase 2's `parse_generation_name`
- a generation's `.sealed`, `.sealed.sig` and `.lease`
- temp residue, through today's `Artifact::owning`
- tree locks: `.tmp-<n>-<p>.lock`

The platform `p` must be in `KNOWN_PLATFORMS`; an unknown platform is
`Unrecognised`, so it is kept. Discovery and every lock-table state come from
this one classification of the shared index.

#### 2. Tree prune

**File**: `cli/launcher/src/launch/outbound/resolve/tree/prune.rs` (new)

```rust
pub enum TreePruneRegime<'a> {
    PinnedOnHost(&'a PluginBinRoot),
    WholeTree(&'a SafePruneRoot),
}

pub struct TreePrunePorts<'a> {
    pub held_leases: &'a HeldLeases,
    pub filesystem: &'a dyn CacheFilesystem,
    pub ownership: &'a dyn Ownership,
    pub clock: &'a dyn Clock,
    pub lease_liveness: &'a dyn LivenessProbe,
    pub pins: &'a dyn PinnedDigests,
    pub locks: TreeLocks<'a>,
    pub steps: &'a dyn PruneSteps,
    pub removals: &'a dyn RemovalObserver,
}

pub struct TreePrune<'a> {
    pub paths: TreePaths,
    pub regime: TreePruneRegime<'a>,
    pub claim_window: Duration,
    pub ports: TreePrunePorts<'a>,
}

impl TreePrune<'_> {
    pub fn run(&self, patience: LockPatience) -> TreePruneReport;
    pub fn prune_held(&self, held: &HeldTreeLock, safe: &SafePruneRoot)
        -> TreePruneReport;
}
```

This is the only tree reclamation in the launcher, and `TreePruneRegime` is
its only switch:

| Regime | Artifacts visited | Bookkeeping sweeps (orphan sidecars, orphan tree locks, stale tree claims) |
|---|---|---|
| `PinnedOnHost` (default root) | pinned artifacts on `HOST_PLATFORM`, today's scope | none |
| `WholeTree` (relocated root) | every artifact and platform under `trees/` | all three |

`PluginBinRoot` (in the `ownership` leaf, beside `SafePruneRoot`) is introduced
here: it has a private field, only `identify` mints one, for
`CacheRootKind::PluginBin`, and its `trees_paths()` is the only source of a
default root's `TreePaths`. So neither regime can be built from a bare path.

`tests/tree_resolution.rs` moves with it in Phase 5's tests-only commit: the
`Harness` cache becomes `<tempdir>/bin`, and the harness mints its
`PluginBinRoot` through `identify` with plugin root `<tempdir>` and no
override. `prune_plugin_bin` takes that `PluginBinRoot`, and Phase 7 routes
`Harness::resolver_*`, today a struct literal (`tree_resolution.rs:351-376`),
through `StartUp::tree_resolver` when `RootTrust` arrives.

`TreePruneReport` is `tree`'s own type; `RootPrune` aggregates it into
`PruneReport`, so `tree` never imports `retention`. `WholeTree` carries the
`SafePruneRoot`, so a whole-tree prune cannot be built without the check,
and under `WholeTree` the prune's `TreePaths` come from
`SafePruneRoot::trees_paths()`, so it prunes the root that was checked;
`PinnedOnHost` needs none, because the plugin `bin` is outside the gate.

`HeldLeases` (in `tree/lease.rs`) is the set of generation directories this
process holds shared leases on. Every lease probe `TreePrune` and its
`Reaper` make goes through it first: a held generation is `Held` without a
probe, since probing a lease the process holds would release it under
emulated `flock`.

`TreePrune`'s listings, metadata reads, pointer reads and removals all go
through `ports.filesystem`, so the recorder and `ScriptedFilesystem` see tree
prune as they see single-file prune. `CacheFilesystem` gains
`remove_tree(path)` (`remove_dir_all`) and `FilesystemCall` gains
`RemoveTree`. `removals` is notified before each removal, as the `Reaper`
already does. The `Reaper` keeps its own `std::fs` reads, as today.

`CachePorts` gains `removals` in this phase; `lease_liveness`, `pins` and
`locks` arrived in Phase 4. Both retentions build `TreePrunePorts` by
borrowing from it, with `TreeLocks` constructed over `CachePorts::locks`.

A process holds at most one tree lock at a time. `run` takes each artifact's
lock inside that artifact's own iteration, so its guard drops before the next
lock is taken. `prune_held` runs steps 1 and 2 for the one artifact whose lock
the caller already holds, and takes no other tree lock. Together these keep
the rule `reap.rs` states: a process never probes or retakes a tree lock it
holds, because with emulated `flock` that releases the first.

`run` and `prune_held` both check `trees/` with `is_trusted_directory`
first; an untrusted `trees/` skips all tree prune. They then read tree
claims once:

```rust
pub enum TreeClaimsListing {
    Read(TreeClaimIndex),
    Unreadable(io::ErrorKind),
}

pub fn list(
    paths: &TreePaths,
    filesystem: &dyn CacheFilesystem,
    ownership: &dyn Ownership,
) -> TreeClaimsListing
```

`NotFound` is `Read` of an empty index, and an untrusted `trees/claims/` is
`Unreadable(PermissionDenied)`. On `Unreadable`, `run` skips step 1 and the
stale tree-claim sweep, records an `UnreadableClaims` in the report, and still
runs the orphan, sidecar and lock steps, which do not depend on claims.

It then lists `trees/` once into an index the per-artifact steps share, and
discovers artifacts from it in sorted order. For each artifact it takes a lock
according to this table:

| Artifact state | `Wait` | `Try` |
|---|---|---|
| has a pointer, a generation or temp residue `Artifact::owning` attributes to it | `locks.take_tree` | `locks.try_take_tree`; `None` skips it |
| sidecars or lock file only | `locks.wait_existing_tree` | `locks.probe_tree`; `Held` skips it |
| absent lock file, sidecars only | lock-free | same |
| nothing present (a pinned artifact under `PinnedOnHost`) | skip, no lock call | same |
| `None` from `wait_existing_tree`, or `Absent` from `probe_tree` | lock-free, as the row above | same |
| `Err` from any of the above | record `LockFailure`, skip | record, skip |

An untrusted `trees/` or `trees/claims/` is recorded as an
`UntrustedDirectory` in the report, which the verb prints and cold paths
`warn!`, so a refused prune is never indistinguishable from "nothing to
reclaim".

Every row that yields a lock yields a `HeldTreeLock`, so a lock-only
artifact reaches the orphan-lock step like any other.

Temp residue counts as something to lock because it shows a materialise of
that artifact was attempted, so taking the lock, creating it if missing,
never creates a lock for an artifact the launcher never materialised. It
keeps today's default-root behaviour, where the verb reaps an artifact's
orphan even when no lock file exists. A lock created this way is young, so
step 3 keeps it. A later `WholeTree` prune removes it once it is aged and no
residue remains. `PinnedOnHost` never sweeps lock files, as today.

`run` models the table's outcome as a value, so only an artifact whose lock
this prune took can reach step 3:

```rust
enum ArtifactLock {
    Taken(HeldTreeLock),
    LockFree,
    Skipped,
    Failed(LockFailure),
}
```

Each numbered step below is a named method taking the `ArtifactLock` it
needs. Under the lock, it does the following for each artifact:

1. **Reclaim unclaimed trees.** For each of the artifact's pointers, classify
   its contents as a `PointerTarget`, which moves here from `resolver.rs`.
   Keep the pointer when:
   - it is `Malformed`;
   - it is `OtherLayout` (both pointer and generation are kept);
   - the digest is `pins.pinned_digest(artifact)`;
   - the index holds a valid claim on the digest within `claim_window`;
   - `!reap::unclaimed_is_reclaimable(liveness, aged)`.

   Otherwise, immediately before removing anything, it re-reads that
   digest's claims from `trees/claims/` through the filesystem port and
   keeps the tree if a fresh valid claim has appeared since the index was
   read, or if the re-read is `Unreadable`. The index decides every keep; the
   re-read guards every removal, at one listing per tree removed. After
   unlinking the pointer it tries a non-blocking exclusive lock on the
   generation's `.lease` and holds it across `remove_tree`; when that lock is
   contended, a dispatch leased the tree after the probe, so the generation
   is left for a later reap. It then removes the pointer first, so a
   concurrent `locate` misses and re-materialises rather than leasing a
   generation about to go, and only then the generation and its sidecars.
2. **Orphan sidecars** (`WholeTree` only). A `.sealed`, `.sealed.sig` or
   `.lease` whose generation is absent is removed when the lease probe reports
   `Free` or `Unknown` and the sidecar is older than `AGE_BACKSTOP`. A new
   `reap::orphan_sidecar_is_reclaimable(liveness, aged)` holds that rule as a
   `const fn` beside the existing tables.
3. **Orphan tree lock** (`WholeTree` only, `run` only, never `prune_held`).
   The shared index decides first. Only when it shows nothing of the artifact's
   left and the lock file aged does the step re-list the artifact's names under
   the held lock to confirm. If no pointer, generation or temp entry that
   `Artifact::owning` attributes to the artifact remains, and the lock file is
   older than `AGE_BACKSTOP`, it unlinks the lock while holding it, then drops
   the guard.

The orphan reap is root-wide, so it is a separate stage of `run`, not a
per-artifact step: `reap_orphans_under(&HeldTreeLock)` runs once,
immediately after the loop produces its first `ArtifactLock::Taken` and
before that artifact's step 1. When `run` takes no lock but the index holds
temp residue or a generation no pointer names, of any artifact, `run` takes
the first pinned artifact's lock with `take_tree` and reaps under it, as
today's verb does by creating each pinned artifact's lock. When there is
neither, nothing is reaped and no lock is created.
`prune_held` does not reap; `materialise` already has, under the same lock.

After the artifacts, under `WholeTree`, it removes each regular file in
`trees/claims/` whose name parses as `<64hex>.<16hex>` and whose mtime is
outside `claim_window`, re-checking the mtime just before the unlink. The module
doc states the precondition, without citing any work item: unlinking a lock file
it holds is safe only while every launcher confirms its lock's inode after
taking it, and a held tree lock means a materialise or prune is in progress.

**File**: `tree/locks.rs`
**Changes**: tree locks get an artifact-level type over the path primitive:

```rust
pub struct TreeLocks<'a> { primitive: &'a dyn SingleFlightLocks }

impl TreeLocks<'_> {
    pub fn take_tree(&self, paths: &TreePaths, artifact: &Artifact)
        -> Result<HeldTreeLock, TreeError>;
    pub fn try_take_tree(&self, paths: &TreePaths, artifact: &Artifact)
        -> Result<Option<HeldTreeLock>, TreeError>;
    pub fn wait_existing_tree(&self, paths: &TreePaths, artifact: &Artifact)
        -> Result<Option<HeldTreeLock>, TreeError>;
    pub fn probe_tree(&self, paths: &TreePaths, artifact: &Artifact)
        -> Result<TreeLockProbe, TreeError>;
}

pub enum TreeLockProbe { Taken(HeldTreeLock), Held, Absent }
```

`TreeLocks` derives the lock path from the artifact, so a guard and its
artifact can never disagree, and it is the only constructor of
`HeldTreeLock`. `HeldTreeLock` holds a `LockGuard`. Nothing outside `tree/`
takes a tree lock by path. `tree_lock.rs`'s static `HeldTreeLock::take` and
`try_take` are deleted, so the compiler moves every caller, `repair`
included, onto `TreeLocks`.

**File**: `tree/resolver.rs`
**Changes**: `TreeResolver` gains `locks: TreeLocks<'a>`, borrowed from
`CachePorts::locks` by `StartUp::tree_resolver`. `materialise` and `repair` take
their locks through `self.locks`, and `self.reaper(&held)` passes the same
`TreeLocks` to `Reaper::new`, so every tree lock and probe the launcher makes
goes through the one injected primitive, and the self-probe guard sees the
materialise path too.

`TreePrune` reports `PruneStep::TreeLockHeld` after taking each artifact's
lock.

**File**: `tree/reap.rs`
**Changes**: `Reaper::new` takes the `TreeLocks` it is given, and
`Reaper::lock_of` probes through `TreeLocks::probe_tree` instead of calling
`probe_single_flight` directly, so a test can see every probe.
`OwnerLock::Taken`, which holds a raw `SingleFlight` today
(`reap.rs:67-73`), holds the probe's `HeldTreeLock` instead.
`OwnerLock::while_reclaimable` still consumes it, so another artifact's lock
is released as soon as its entries are removed, as main does today.

**File**: `retention/root_prune.rs`
**Changes**: `RootPrune` builds a `TreePrune` with `WholeTree(self.safe)`
and its `held_leases`, and `run` runs it after the root-locked work, in place
of Phase 4's call to `tree::prune_pinned_on_host`. A held root lock never blocks
tree prune.

#### 3. One tree prune in every root

**File**: `retention/root_retention.rs`
**Changes**: `PluginBinRetention::prune` runs `TreePrune` with
`PinnedOnHost`, waiting.

**File**: `tree/resolver.rs`
**Changes**: `TreeResolver::prune`, `reclaim_unclaimed` and
`artifact_pointers` are deleted. The default root therefore inherits
`TreePrune`'s lock table: it takes a lock only when the artifact has a pointer,
a generation or temp residue, so the verb no longer creates tree locks for
artifacts it never materialised, and a missing `trees/` is no longer an
`ENOENT`. It also inherits the fail-closed tree-claims listing.
`claims::claimed_within` loses its last caller and is deleted.

The swap is two commits, so the default-root regression tests guard it:

1. **Tests only.** The 17 direct `resolver.prune(ARTIFACT, older_than)` calls
   in `tests/tree_resolution.rs` move to a helper that carries what each
   call site injects today:

   ```rust
   struct PluginBinSeams {
       clock: Box<dyn Clock>,
       lease_liveness: Box<dyn LivenessProbe>,
       pins: Box<dyn PinnedDigests>,
   }

   fn prune_plugin_bin(harness: &Harness, seams: PluginBinSeams,
       artifact: &str, older_than: Option<Duration>) -> Reclaimed
   ```

   Each existing constructor maps to a seams value: `resolver_with`,
   `resolver_expecting` and `resolver_also_pinning_bystander`, plus the
   `AdvancingClock` at `:656` and the second resolver pinning `"e"*64` at
   `:2640-2684`. In this commit the helper calls
   `tree::prune_pinned_artifact` for the one `artifact` each call site names
   today, so its scope is unchanged and every assertion is unchanged.
2. **The swap.** The helper drops its `artifact` parameter and calls
   `TreePrune { regime: PinnedOnHost(&plugin_bin_root) }` with the seams, which
   visits every pinned host artifact, as the verb's loop always has, and
   `TreeResolver::prune` is deleted. `PluginBinSeams` gains `patience`, read for
   the first time here; it is `Wait` everywhere except the bystander tests.
   `cache_prune_reclaims_owned_orphans_and_keeps_unowned_ones` (`:949`) passes
   unchanged, because residue takes the lock. Assertions are unchanged except
   one: `another_artifacts_materialise_survives_this_ones_prune` holds
   BYSTANDER's lock while BYSTANDER is pinned and has residue, so its seams use
   `Try` and it asserts BYSTANDER is skipped and survives, where today it was
   never visited. `another_artifacts_pinned_archive_survives_a_prune` (`:1045`)
   leaves BYSTANDER's lock free: BYSTANDER sorts first and has residue, so
   `TreePrune` takes its lock and the reap judges its archive as `HeldByCaller`,
   where only `is_resumable`, the pin rule, spares it. The test adds an unpinned
   archive beside it and asserts the pinned one survives and the unpinned one
   goes, so it still fails if the pin rule is dropped. Those are the two changed
   tests; the others seed only `ARTIFACT` and are unaffected. The verb's own
   `run_cache` calls gain the `&dyn RootRetention` argument and keep their
   assertions.

The verb leg of `the_plugin_bin_root_is_never_claimed_or_pruned` moves into
this phase, so the default-root criterion is checked when the default-root
verb changes.

#### 4. Tests (red first)

In `tests/root_prune.rs` (relocated root), with the tree harness helpers
from `tests/tree_resolution.rs` moved into `tests/common/trees.rs` so both
files share them:

- `only_aged_unknown_and_free_unpinned_trees_are_reclaimed`: held, young
  unknown, old unknown, free and pinned-free, through `FixedLiveness`
- `old_orphans_sidecars_and_retired_locks_go_and_young_generations_stay`
- `unclaimed_retired_and_foreign_platform_trees_go_and_claimed_ones_stay`
- `only_tree_claims_outside_the_window_are_removed`
- `an_unreadable_tree_claims_directory_reclaims_no_claimed_tree`: mode `000`
  on `trees/claims/`, skipped when running as root; the verb reports it and
  exits 0, and orphan residue is still reaped
- `only_old_unheld_orphan_sidecars_are_removed`
- `orphan_tree_locks_go_only_once_aged_and_never_for_a_current_tree`
- `a_tree_lock_is_kept_while_its_partial_archive_remains`: a pinned partial
  archive, an aged lock and no pointer all survive
- `a_lockless_partial_archive_waits_for_its_own_lock`: an unpinned partial
  archive of a retired artifact with no lock file, plus an earlier-sorting
  artifact with an unclaimed tree, so the reap runs under that artifact's
  lock and finds the retired one's lock `Absent`. The first prune keeps the
  archive, as `is_spared` requires, and creates the retired artifact's lock
  through the residue row. The second prune reaps the archive under that
  lock, and the third, once the lock has aged, removes it. When the retired
  artifact is the only one, its own lock is the reaping holder, and one
  prune removes the archive
- `a_retired_partial_archive_goes_with_its_lock`: the same seed unpinned
  but with an aged lock file already present;
  one prune removes the archive in the reap stage and then the aged lock
- `another_layouts_tree_is_kept_and_this_layouts_twin_is_removed`
- `foreign_names_survive_every_trigger` gains its `trees/` seed
- `a_symlinked_trees_or_tree_claims_directory_is_not_pruned_through`: the
  verb prints the `UntrustedDirectory` line, and a sibling single-file entry
  of an unclaimed version is removed as the positive control
- `a_residue_only_default_root_is_still_reaped`: under `PinnedOnHost`, an
  orphan temp generation of an unpinned artifact is reaped under the first
  pinned artifact's lock
- `a_tree_leased_during_removal_is_kept`: a second open file description
  takes the shared lease after the probe; the contended exclusive lock keeps
  the generation
- `the_plugin_bin_verb_reports_a_failed_tree_lock_and_prunes_the_rest`:
  `FailingLocks` fails one pinned artifact; the verb prunes the others,
  prints the failure and exits non-zero, as the CHANGELOG states
- the Phase 2 pointer-containment tests, including
  `a_symlinked_fifo_or_oversized_pointer_is_malformed_and_kept`, re-run
  against a relocated root so `TreePrune`'s pointer path is held to the same
  rule
- `each_artifact_state_and_patience_uses_the_tables_lock_call`: a recording
  locks seam over every (artifact state × patience) cell
- `the_verb_reclaims_single_file_entries_and_an_unclaimed_tree`
- `older_than_narrows_the_tree_claim_window`
- `the_verb_waits_for_a_held_tree_lock`
- `the_verb_reports_a_failed_tree_lock_and_exits_non_zero` (`ENOLCK` for
  artifact X; Y is still pruned)
- `a_residue_only_artifact_is_locked_and_reaped`: an orphan temp generation
  (not a partial archive, which `is_spared` keeps for any artifact whose
  lock file is absent at reap time, `reap.rs:258-267`)
  with no pointer and no lock file is reaped, and the lock this creates is
  removed by a prune once it is aged
- `a_user_private_group_root_prunes_its_group_writable_trees`: a `trees/`
  the overlay reports group-writable by the private gid, holding an
  unclaimed tree, is reclaimed; reported writable by a shared gid, nothing
  is
- `a_lock_only_artifact_reaches_the_orphan_lock_step`: an aged lock file with
  no pointer, generation or residue is removed under `wait_existing_tree`
  and under `probe_tree`
- `a_symlinked_trees_directory_is_not_pruned_by_prune_held`
- `a_tree_claim_exactly_at_the_window_is_stale`
- `a_tree_removal_failure_is_reported`: `ScriptedFilesystem` fails one
  `remove_tree`
- `a_tree_claimed_after_the_index_was_read_is_kept`: `ScriptedFilesystem`
  writes a fresh claim after the index read and before the re-read, with
  `FixedLiveness` at `Unknown` and an aged generation
- `the_pointer_goes_before_its_generation`: the recorder shows the pointer's
  `RemoveFile` before the generation's `RemoveTree`

Unit tests in `tree/layout.rs` cover `classify_trees_entry` for every recognised
shape, an unknown platform, a malformed digest and another layout.

In `tests/tree_resolution.rs` (every root):

- `the_verb_creates_no_lock_for_an_artifact_it_never_materialised`, run in
  both root kinds
- `the_verb_tolerates_a_missing_trees_directory`
- `an_unreadable_tree_claims_directory_reclaims_nothing_in_the_plugin_bin`
- `the_plugin_bin_verb_still_reclaims_an_unclaimed_superseded_tree`
  (existing behaviour, pinned)

Every unlink `TreePrune` makes goes through `removals`, which it shares with
its `Reaper`: pointers, generations, sidecars, tree claims and lock files.

A guard test, `no_production_path_unlinks_a_tree_lock_it_does_not_hold`,
records removals through that observer across every phase-5 path. At the
moment of each lock-file removal, the observer tries the lock without
blocking from a separate open file description, as `ProbingLockOf` in
`reap.rs` does, and asserts contention. This follows the PR #167 review
suggestion. `a_claim_refreshed_before_unlink_is_kept` also runs against the
tree-claim sweep, now that tree prune reads through the port. That variant
puts the claim on a digest with no tree, so step 1 never reads it and the
scripted call is the sweep's own re-check, the claim's second `entry`.
`a_tree_claimed_after_the_index_was_read_is_kept` scripts the second
`List(trees/claims)`, the re-read after the index.

A second guard, `no_prune_path_probes_or_takes_a_lock_it_holds`, uses a
recording `SingleFlightLocks` that wraps each `LockGuard` it hands out, so it
tracks which paths are held until each guard drops. It fails on any `take`,
`try_take`, `wait_existing` or `probe` of a held path. Phase 5 runs it over the
verb; Phase 7 extends it to a cold store and a materialise.

### Success Criteria:

#### Automated Verification:

- [ ] All Phase 5 tests pass: `mise run test:unit:cli`
- [ ] Existing `tree_resolution.rs` prune tests pass with unchanged
      assertions after commit 1, and after commit 2 except the cases that
      commit lists
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] In a scratch relocated root, a planted retired-artifact tree with an
      old lock file is removed by `accelerator cache prune`, lock included

---

## Phase 6: Release Markers That Survive Stripping

### Overview

`jira`, `linear` and `research` mark a `test-loopback` build with a
`#[no_mangle] static … : u8`, and `_assert_no_test_loopback` scans each staged
release binary for the bytes `TEST_LOOPBACK_MARKER`. The release profile sets
`strip = true` and `lto = "thin"` (`cli/Cargo.toml:260-261`), so the symbol
name is stripped and the unused static can be dropped, and the scan likely
finds nothing. The compile guard keys on `debug_assertions`, which
`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true` turns back on in a release
build, leaving the blind scan as the only backstop. This phase gives the three
existing binaries a marker that survives and proves the guard on real release
artefacts. It is independent of the cache work, and Phase 7 uses the same
pattern for the launcher.

### Changes Required:

#### 1. A retained marker

**Files**: `cli/jira-cli/src/main.rs`, `cli/linear-cli/src/main.rs`,
`cli/research-cli/src/main.rs`
**Changes**: each replaces its `#[no_mangle] pub static` with a string
literal that live code reads:

```rust
#[cfg(feature = "test-loopback")]
const TEST_LOOPBACK_MARKER: &str = "ACCELERATOR_JIRA_TEST_LOOPBACK_MARKER";

fn main() -> ExitCode {
    #[cfg(feature = "test-loopback")]
    std::hint::black_box(TEST_LOOPBACK_MARKER);
    // ... unchanged
}
```

The literal's bytes sit in `.rodata`, which stripping keeps, and the
`black_box` read stops LTO from discarding them. The existing comment above
each `compile_error!` is rewritten to say why the literal is read in `main`:
so that a release build carrying the feature still contains its bytes. It
names no build task. That reason is the one non-obvious fact here.

**File**: `tasks/build.py`
**Changes**: `_assert_no_test_loopback`'s docstring describes a string marker
read at start-up rather than a symbol.

#### 2. Tests (red first)

**File**: `tests/integration/release_guard/test_release_guard.py` (new),
behind a new `test:integration:release-guard` leaf. The leaf depends on the
Rust toolchain, as `build:cli:fixture-size` does, and is registered following
`tasks/README.md`'s checklist. `test:integration:tasks` stays Python-only.

- `test_a_release_binary_built_with_test_loopback_is_rejected`, parametrised
  over `accelerator-jira`, `accelerator-linear` and `accelerator-research`.
- `test_a_default_release_binary_passes_the_loopback_guard`: the control.

A module-scoped fixture builds both variants with the same profile
environment, `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true`, so the second
build reuses the first build's dependency graph and only the binary crates
and their LTO link rebuild. `CARGO_TARGET_DIR` is the stable
`cli/target/release-guard`, which nothing else reads, so a binary that trusts
a test key never lands in `cli/target/release/` for another task to measure
or run. Being stable, it persists locally between runs instead of piling up
in pytest's temp area.

**File**: `.github/workflows/main.yml`
**Changes**: the integration job's rust-cache step lists
`cli -> target/release-guard` beside `cli` under `workspaces`, so CI keeps
the release-guard dependency graph between runs and only the four binaries
relink. Against today's
static marker, the first test is red. The leaf runs on both CI legs, which
covers both linker families the shipped artefacts use.

⏱️ The leaf's first run on a cache miss costs one thin-LTO release build of
the dependency graph, a few minutes per CI leg. Later runs relink only the
four binaries.

### Success Criteria:

#### Automated Verification:

- [ ] The rejection test fails against the static marker and passes after
      the change; the default-build control passes throughout:
      `mise run test:integration:release-guard`
- [ ] The existing `test_build.py` guard unit tests pass unchanged:
      `mise run test:unit:tasks`
- [ ] The `test-loopback` integration suites of the three CLIs pass:
      `mise run test:unit:cli`
- [ ] `mise run check` exits 0

#### Manual Verification:

- [ ] `strings` on a stripped release build with the feature shows the
      marker for each of the three binaries

---

## Phase 7: Cold-Path Triggers and the End-to-End Scenario

### Overview

Run a root prune on every cold path in a relocated root, trying each lock
without blocking. Add the guarded `test-loopback` launcher feature and prove
the A/B/C scenario through `bin/accelerator`.

### Changes Required:

#### 1. After a successful `store`

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs`
**Changes**: a narrow port beside the resolver, which `RootRetention`
extends, mirroring `AfterMaterialise`:

```rust
pub trait AfterStore {
    fn after_store(&self, held_leases: &HeldLeases);
}
```

`StartUp::acquire_and_resolve(config, artifacts)` acquires the consumed trees
through `StartUp::tree_resolver`, builds `HeldLeases::of(&acquired)` and the
dispatch resolver from it, and returns both. `run` calls it in place of
`export_consumed_trees` plus a separate resolver, so the wiring lives in the
library, where a test can reach it. It hands `HeldLeases` to
`StartUp::dispatch_resolver`, which hands it to the resolver with the
`Arc<dyn AfterStore>`. `PluginBinRetention` makes no call;
`RelocatedRetention` runs `RootPrune::run(LockPatience::Try, held_leases)`
and
logs each `LockFailure`, `UnreadableClaims`, `RemovalFailure` and
`UnsafeRoot` with
`tracing::warn!`. `FetchVerifyCacheResolver::new` takes the `AfterStore`
and the `HeldLeases` as required arguments, so production wiring cannot
silently omit them. `fetch_verify_store` calls
`after_store.after_store(&held_leases)` after `cache::store` succeeds and
still returns the stored path.

#### 2. After a materialise

**File**: `tree/after_materialise.rs` (new)

```rust
pub trait AfterMaterialise {
    fn reap_permit(&self) -> Option<ReapPermit>;
    fn after_materialise(&self, held: HeldTreeLock);
}

pub struct ReapPermit<'a> { paths: &'a TreePaths }

pub enum RootTrust<'a> {
    PluginBin(&'a PluginBinRoot),
    Relocated(&'a SafePruneRoot),
}

impl ReapPermit<'_> {
    pub fn for_root(trust: RootTrust<'_>,
        filesystem: &dyn CacheFilesystem, ownership: &dyn Ownership)
        -> Option<ReapPermit<'_>>;
    pub fn paths(&self) -> &TreePaths;
}
```

There is one constructor. `RootTrust::PluginBin` carries Phase 5's
`PluginBinRoot`, not a bare `TreePaths`, as `TreePruneRegime::PinnedOnHost`
already does. A relocated root's `TreePaths`
come only from `SafePruneRoot::trees_paths()`, so the paths a permit reaps are
the paths of the root that was checked, and no caller can pair a relocated root
with the plugin-bin arm. `TreeResolver` holds the `RootTrust` it was built with,
from `StartUp::tree_resolver`.

`tree` owns this port and `retention` implements it, so the dependency still
points one way. Taking `held` by value makes releasing it part of the contract.
A `ReapPermit` exists only when `trees/` passes `is_trusted_directory`, and in a
relocated root only alongside a `SafePruneRoot`. `Reaper::reap_orphans` and
`reap_superseded`, which removes the generation a forced re-materialise
replaces, both require one, and reap only `permit.paths()`. `repair`, the other
caller of `reap_superseded` (`resolver.rs:723-726`), asks the hook's
`reap_permit()` as `materialise` does. `TreePrune`'s reap stage builds its
permit from the trusted-`trees/` check it already makes; in a relocated root
that stage only ever runs inside `RootPrune`, which holds a `SafePruneRoot`.

**File**: `tree/resolver.rs`
**Changes**: `TreeResolver` takes `&dyn AfterMaterialise`. In `materialise`
it asks the hook for a permit, runs its own `reap_orphans` and
`reap_superseded` under `held` only with one, then hands `held` to the
hook. The reap stays inside
`materialise`, so wiring cannot drop it; the permit decides whether this
root may be reaped.

**File**: `retention/root_retention.rs`
**Changes**: `RootRetention: AfterMaterialise`.

| Retention | `reap_permit()` |
|---|---|
| `PluginBinRetention` | granted when `trees/` is trusted, as it is in any plugin `bin` the user can write |
| `RelocatedRetention` | granted only with a `SafePruneRoot` and a trusted `trees/` |

In the default root this reaps exactly as today wherever the user owns
`trees/`; where they do not, as in a system-wide install, they could not
have created anything to reap. In a relocated root it makes "no prune path in
an unsafe root removes anything" true of the reap too.

| Retention | `after_materialise(held)` |
|---|---|
| `PluginBinRetention` | `drop(held)`; the reap has already run |
| `RelocatedRetention` | with a `SafePruneRoot`, `prune_held(&held, &safe)`; then `drop(held)` and `RootPrune::run(LockPatience::Try, &HeldLeases::none())` |

A materialise holds no leases: dispatch's `acquire` never materialises, and
returns `None` for an absent tree (`resolver.rs:557-558`); materialising runs
in its own `accelerator cache ensure` process. Only a cold store can run
while its process holds leases.

The materialise's own artifact is pruned under `held`, after the reap
`materialise` already ran. `held` is dropped before the rest of the root is
pruned, so the materialise never holds its lock while another artifact's trees
are removed. A second visit to its artifact from `run` is harmless.

#### 3. The `test-loopback` feature

**File**: `cli/launcher/Cargo.toml`
**Changes**: `test-loopback = []`, with no comment: the `compile_error!`
enforces what a comment would say.

**File**: `cli/launcher/src/main.rs`
**Changes**:

```rust
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
const TEST_LOOPBACK_MARKER: &str = "ACCELERATOR_LAUNCHER_TEST_LOOPBACK_MARKER";

#[cfg(feature = "test-loopback")]
fn admits_test_release(base_url: &str) -> bool
```

The marker follows Phase 6's pattern. `admits_test_release` parses the URL
with `reqwest::Url`, already a dependency, and holds only for scheme `http` and
`host_str() == Some("127.0.0.1")`. Both relaxations depend on that one
condition. `release_keys()` and `release_fetcher()` replace the direct
`TrustedKeys::embedded()` calls at `main.rs:84`, `:326` and `:421` and the
`Fetcher::new()` calls at `:86`, `:327` and `:422`. The help path (`:104`)
keeps `Fetcher::for_help()`:

- **`release_keys()`.** Under the feature, when `admits_test_release` holds
  and `ACCELERATOR_TEST_RELEASE_PUBLIC_KEY` names a readable file, it logs
  `tracing::warn!("{TEST_LOOPBACK_MARKER}: trusting the test release key at
  {path}")`, reads the file, and passes its contents to
  `TrustedKeys::from_public_key_files`, which takes key contents, not paths.
  Otherwise it returns the embedded key, logging a key variable it ignored. The
  log line is the marker's live read.
- **`release_fetcher()`.** Under the feature, when `admits_test_release`
  holds, it returns `Fetcher::admitting_loopback_http()`. That new
  constructor builds with `require_https = false` and production limits.

`main.rs` is the only file with `#[cfg(feature …)]` arms.

**File**: `tasks/shared/dev_builds.py`
**Changes**: `LAUNCHER` becomes `DevBinary("accelerator", package="accelerator",
loopback_seam=True)`, so every debug launcher build carries the feature.

**File**: `bin/accelerator`
**Changes**: the final `exec` intercepts only a missing launcher:

```bash
shopt -s execfail
exec "${launcher}" "$@"
exec_status=$?
if [[ ! -e "${launcher}" ]]; then
	fail "the launcher ${launcher} vanished before it could run"
fi
exit "${exec_status}"
```

The branch tests the file, not the status: bash 3.2 reports 126 for an
`exec` of a missing file, the same as for an unexecutable one, where later
bash reports 127. A launcher entry removed between verification and `exec`
reaches `fail`, which honours `--fail-safe`. Any other `exec` failure, such
as `EACCES` on a `noexec` mount, `ENOEXEC` or `ETXTBSY`, exits with bash's
own status, exactly as today, so a launcher that can never run stays visible
even under `--fail-safe`. None of these goes through
`fail_integrity`: they are not trust-chain failures. `execfail` is
bash-3.2-safe, and nothing is written. Older bootstraps keep the 126/127
behaviour.

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: `test_a_launcher_removed_before_exec_honours_fail_safe`: a
stub verifier that deletes the launcher after verifying it, then a
`--fail-safe` dispatch exits 0 with the diagnostic.
`test_a_launcher_removed_before_exec_fails_with_the_diagnostic`: the same
stub without `--fail-safe` exits 1 with the "vanished" message.
`test_an_unexecutable_launcher_still_fails_under_fail_safe`: a stub verifier
that clears the launcher's execute bit after verifying it, then a
`--fail-safe` dispatch exits 126.

**File**: `tests/support/artefacts.py`
**Changes**: `build_launcher()` passes `--features accelerator/test-loopback`.

**File**: `tasks/build.py`
**Changes**: `_assert_no_test_loopback`'s docstring names the launcher. The
check itself already covers `accelerator`.

**File**: `tasks/README.md`
**Changes**: the list of loopback-seamed binaries (`:254`) gains the
launcher.

#### 4. Tests (red first)

In `cli/launcher/src/main.rs` (unit tests, under the feature):

- `a_non_loopback_base_url_ignores_the_test_key`
- `a_lookalike_host_is_not_loopback`: `http://127.0.0.1.example` and
  `http://127.0.0.1@example`
- `a_missing_key_file_falls_back_to_the_embedded_key`
- `a_non_loopback_base_url_keeps_https_only`: `release_fetcher()` keeps
  `require_https` for a non-loopback or lookalike base URL

`tests/integration/release_guard/`'s
`test_a_release_binary_built_with_test_loopback_is_rejected` and its control
gain `accelerator` as a fourth parameter, built in the same invocation.

`tests/integration/entrypoint/`: `assert_hermetic` still rejects a release
host that is neither `.invalid` nor loopback.

In `tests/root_prune.rs`, driving the real resolvers in-process:

- `a_cold_dispatch_prunes_before_exec`: an `exec` fake snapshots the root,
  and the unclaimed version's entries are already absent.
- `a_held_root_lock_does_not_delay_a_cold_dispatch`: single-file entries
  survive, and `exec` is reached within `BLOCKING_BOUND`. That 10 s bound
  guards against blocking, not latency.
- `a_materialise_with_the_root_lock_held_still_prunes_other_trees`
- `a_materialise_releases_its_lock_before_pruning_other_artifacts`: a
  second thread's `try_take` on the materialised artifact succeeds while the
  prune is paused at another artifact
- `no_prune_path_probes_or_takes_a_lock_it_holds`, extended to a cold store
  and a materialise
- `a_cold_dispatch_skips_a_held_tree_lock_and_prunes_the_rest` (X held, Y
  removed)
- `a_materialise_prunes_the_superseded_tree_single_files_and_probes`
- `a_materialise_in_an_unsafe_root_reaps_nothing`: an orphan temp
  generation, and on a forced re-materialise the superseded generation,
  survive in a shared-group-writable relocated root
- `a_materialise_with_an_untrusted_trees_directory_reaps_nothing`, in both
  root kinds: `trees/` is untrusted only through the overlay, so
  `ensure_trees_dir`'s own checks pass and the materialise returns `Ok`; the
  same seed with `trees/` trusted has its orphan temp generation and
  superseded generation removed
- `a_forced_repair_in_an_unsafe_root_reaps_nothing`, paired with an
  owner-only root where the superseded generation is removed
- `prune_held_removes_the_superseded_tree_before_the_lock_is_released`: the
  recorder shows its `RemoveTree` before the `TreeLocks` guard drops, so the
  test fails if `prune_held` does nothing and `run` removes it later
- `a_cold_store_never_probes_its_own_held_leases` and
  `acquired_trees_are_never_probed_by_the_cold_store_prune`: the second goes
  through `StartUp::acquire_and_resolve`, the path `run` uses, acquiring two
  trees, then cold-stores a sub-binary. A lease probe is only reached for a
  generation step 1 would otherwise reclaim, so both seed it: `CachePorts`
  pins no digest for the acquired artifacts, and an advancing clock moves
  past `CLAIM_WINDOW` after `acquire` refreshes their claims. A
  `LivenessProbe` wrapper fails on any probe of a leased generation and
  answers `Free` for every other, and the leased generations survive. A
  control builds the resolver with `HeldLeases::none()` and shows the
  wrapper fires, so the test can fail. This pins the `HeldLeases::of`
  wiring
- `an_unsafe_root_is_not_pruned_from_any_trigger`: the group-writable and
  foreign-uid roots of Phase 4, now through a cold store and a materialise.
  The materialised artifact's superseded tree and an orphan temp generation
  both survive; an owner-only root, the positive control, has both removed
- `a_cold_prune_behind_a_paused_prune_returns_without_touching_anything`:
  the first prune pauses at `RootLockHeld`, the cold-path second returns,
  and the seeded entries survive. The verb's second prune blocks until the
  first is released.
- `an_enolck_root_lock_skips_single_file_work_on_the_cold_path`
- `an_enolck_tree_lock_skips_only_that_artifact_on_cold_paths`: cold store
  and Y's materialise both proceed, X survives, Y's unclaimed tree goes.
- `the_plugin_bin_root_is_never_claimed_or_pruned`: the default-root
  criterion. It seeds every listed shape, runs a cold dispatch, a materialise
  and the verb, and asserts no claim and every seeded entry survives, while
  an unclaimed superseded tree of a pinned artifact is removed by the verb,
  as today, as the positive control. It is
  run with the root named directly, through a symlink, and with a trailing
  slash.
- `the_plugin_bin_orphan_reap_is_unchanged`: built through
  `StartUp::tree_resolver`, so a wrongly wired hook fails it; the old
  unreferenced generation goes and the young one stays.
- `a_development_checkout_relocated_elsewhere_prunes_on_its_cold_path`
- `foreign_names_survive_every_trigger` and
  `the_kept_names_survive_beside_an_unclaimed_version` become parameterised
  over the trigger: the verb, a cold store and a materialise
- `an_entry_removed_between_find_and_exec_is_refetched_once` and
  `a_corrupt_entry_is_replaced_by_a_rename_with_no_unlink_before_it` re-run
  in a relocated root with the post-store prune active

The end-to-end scenario goes in
`tests/integration/entrypoint/test_accelerator_entrypoint.py`, as
`test_a_relocated_root_keeps_only_the_current_versions_footprint`:

1. **Release server.** A `ThreadingHTTPServer` on `127.0.0.1:0` serves
   `SERVER_DIR`. That directory holds the launcher assets the downloader stub
   already reads, plus a manifest for the launcher's crate version, its
   signature, and an `accelerator-visualiser-<platform>` `#!/bin/sh` asset.
   Everything is signed with the run's `release` key.
2. **Seeded history.** The fixture creates the relocated root with mode
   `0700` and asserts it, so the test does not depend on the developer's
   umask. The relocated root is seeded with A's and B's
   history: entries, launcher entries, version claims, trees with pointers,
   claims, sidecars and locks, shim copies under other digests, and
   interrupted stores as `.tmp-` files. All
   of it is back-dated past `CLAIM_WINDOW`, and the residue past
   `AGE_BACKSTOP`.
3. **C's trees.** C's own trees are planted under the digests from
   the repository-root `pins.toml`, which `cli/launcher/build.rs` reads,
   with fresh tree claims.
4. **Dispatch.** The test's `make_harness(real_launcher=True)` (as at
   `test_accelerator_entrypoint.py:130`) builds the installation, and
   `run_bootstrap(…, extra_env={ ACCELERATOR_CACHE_DIR,
   ACCELERATOR_RELEASE_BASE_URL: http://127.0.0.1:…,
   ACCELERATOR_TEST_RELEASE_PUBLIC_KEY })` dispatches `visualiser`.
5. **End state.** The test asserts the bootstrap exits 0, the stub's marker
   is on stdout, the loopback server served the sub-binary asset exactly
   once, and the root holds exactly the item's list: the running plugin's
   staged shim copy included, and `.accelerator-unverified.log` only if
   present, since only an integrity failure creates it.

`assert_hermetic` (`installation.py:252-268`) also admits a
`http://127.0.0.1` release host.

`tests/unit/tasks/test_dev_builds.py`:
`test_a_build_without_seams_passes_no_feature_flag` builds `accelerator-vcs`
instead of `LAUNCHER`. The existing
`test_a_binary_carries_the_loopback_seam_exactly_when_its_package_has_it`
already covers the launcher once its `DevBinary` changes.

### Success Criteria:

#### Automated Verification:

- [ ] All Phase 7 Rust tests pass: `mise run test:unit:cli`
- [ ] The scenario passes: `mise run test:integration:entrypoint`
- [ ] A release launcher built with `test-loopback` is rejected by the
      guard, and a default one passes:
      `mise run test:integration:release-guard`
- [ ] `mise run check` exits 0

#### Manual Verification:

- [ ] `cargo build --release -p accelerator --features test-loopback` fails
      with the `compile_error!` message
- [ ] In a scratch relocated root shared by two installed plugin versions, a
      first-use dispatch on the newer one leaves the older one's entries
      intact while its claim is fresh
- [ ] Time a cold dispatch against a scratch relocated root seeded with two
      unclaimed `browser` trees, both from an interactive dispatch and from a
      SessionStart hook, and record the figures in the PR description

---

## Phase 8: Documentation and Measurement

### Overview

Document the new behaviour where users read about `ACCELERATOR_CACHE_DIR`,
record a post-change measurement, and run the full default task.

### Changes Required:

#### 1. `docs-site/src/content/docs/design.md`

- The `prune` row in the verbs table (`:211`): "Reclaim what no fresh claim
  protects. In a relocated root this covers single-file entries, trees of
  any artifact or platform, stale claims, orphan sidecars and locks, and
  crash residue. In the default root it covers unclaimed trees of the
  current artifacts. Prints how many entries it removed, and exits non-zero
  when a lock cannot be taken or a removal fails."
- The `ACCELERATOR_CACHE_DIR` row (`:264`) gains five statements:
  - a relocated root is pruned automatically on cold paths;
  - a version idle for 14 days loses its entries and re-fetches;
  - `--older-than` below ~1.4 days can evict versions other sessions are
    using; they re-fetch on their next dispatch, and the running version is
    never evicted;
  - what `accelerator cache prune` now removes;
  - a cache root is pruned only when it is writable by its owner alone, or
    through the owner's private group, and every directory above it is
    owned by the user, by root or by a uid listed in
    `ACCELERATOR_CACHE_TRUSTED_OWNERS`, and not writable by others without
    the sticky bit; CI and container examples show the variable; a shared
    location such as `/var/cache/accelerator` still works as a cache but is
    not pruned, so on a shared host give each user
    `ACCELERATOR_CACHE_DIR=<shared>/$USER`; the refusal names the directory
    and the fix.

#### 2. `docs-site/src/content/docs/internals.md`

- The `ACCELERATOR_CACHE_DIR` row (`:263`) carries the same five
  statements.
- A paragraph after `:273-284` explains why a warm dispatch in a relocated
  root still tolerates a read-only directory: the claim write is
  best-effort. It also states the accepted pre-0339 lock-file risk.

#### 3. `CHANGELOG.md`

Add `[Unreleased]` entries:

- **Changed.** A relocated `ACCELERATOR_CACHE_DIR` is pruned automatically
  on cold paths, keeping what a version or tree claim from the last 14 days
  protects. A root is pruned only when it is writable by its owner alone, or
  through the owner's private group, and no directory above it is writable by
  others without the sticky bit. A root shared by several users, sticky or not,
  works as a cache but is not pruned; give each user their own, such as
  `/var/cache/accelerator/$USER`. In a relocated root, `cache prune` also
  removes single-file entries, other artifacts' and platforms' trees, stale
  claims, orphan sidecars and locks, and crash residue.
- **Breaking.** `cache prune` no longer stops at the first lock it cannot
  take: it prunes every other artifact, prints the count, and then exits
  non-zero. A removal that fails, which was silently ignored, now also exits
  non-zero. `--older-than` also sets the version-claim window. When the
  launcher vanishes before it can run, `bin/accelerator` now exits 1, or 0
  under `--fail-safe`, where it exited 126 or 127, depending on the bash
  version. Every other `exec` failure exits
  as before.
- **Changed.** A dispatch whose sub-binary vanishes between lookup and
  `exec` re-fetches it once, in every cache root.
- **Breaking.** `cache prune` in a relocated root that other users can
  write to, or that sits under a directory they can, now removes nothing
  and exits non-zero, printing the directory at fault and the fix, where it
  pruned the pinned trees and exited 0. In such a root a materialise no
  longer reaps orphan residue or a superseded generation either. A root
  writable only through a private group that no lookup can resolve is
  refused the same way, with the `chmod g-w` remedy, and so is a root under
  a directory owned by another uid, as in a container's bind mount or a CI
  workspace, until `ACCELERATOR_CACHE_TRUSTED_OWNERS` lists that uid.
- **Fixed.** The default-root verb no longer creates tree locks for artifacts
  it never materialised, except to reap an interrupted materialise's
  residue, and no longer errors when `trees/` is missing. Tree prune never
  follows a pointer outside `trees/`, keeps malformed and other-layout
  pointers, and reclaims nothing when `trees/claims/` cannot be read, where
  it reclaimed claimed trees.
- **Changed.** A plugin `bin/trees/` that is a symlink or owned by another
  user is no longer pruned or reaped.
- **Migrations.** Sessions of older launchers sharing a relocated root have
  their entries evicted after each prune by a newer launcher, and re-fetch.
  Reloading the plugin ends this.

#### 4. Measurement

Run `mise run measure:warm-dispatch` and commit the resulting
`meta/measurements/warm-dispatch-{N}.json`, whatever its gating result. The
commit message says it is a record, not evidence of this change's cost.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run docs:check` exits 0
- [ ] `CHANGELOG.md`'s `[Unreleased]` section carries the five entries
- [ ] A new `meta/measurements/warm-dispatch-*.json` is committed
- [ ] `mise run` exits 0 end-to-end

#### Manual Verification:

- [ ] Both `ACCELERATOR_CACHE_DIR` entries read as one consistent account in
      the rendered docs site

---

## Testing Strategy

### Unit Tests:

- Pure rules: the name grammar, `parse_generation_name`, `protection`,
  `disposition`, `orphan_sidecar_is_reclaimable`, claim-name parsing and the
  identity table. Each is tested exhaustively without a filesystem.
- Seams: `Ownership`, `BootstrapHolderLiveness`, `SingleFlightLocks` (a
  `FailingLocks` returning `ENOLCK` for a chosen path), `PruneSteps` (a
  channel-handshake pause), `LivenessProbe` (`FixedLiveness`), `Clock`
  (`FixedClock`), `ScriptedFilesystem` (injected failures and refreshes) and
  `RemovalObserver` (the lock-held probe at removal).
- Boundaries: one exactly-at-threshold case per rule, with `FixedClock` at the
  threshold: the claim refresh interval, `AGE_BACKSTOP`, claim freshness and the
  tree-claim window. Each pins the side the boundary falls on, as `claims.rs`
  and `reap.rs` already do. Time is advanced through the clock, not by
  back-dating where avoidable (`reap.rs:415-418`).

### Integration Tests:

- `tests/resolution.rs`: the recorded call lists, claims before `find`, the
  re-resolve and the replacement invariant.
- `tests/root_prune.rs`: every relocated-root criterion, the verb, cold
  paths and concurrency.
- `tests/tree_resolution.rs`: no-new-lock and the default-root regressions.
- The entrypoint suite: the A/B/C end-to-end scenario through
  `bin/accelerator`.

### Acceptance Criterion Map:

| Criterion (work item order) | Test | Phase |
|---|---|---|
| A unclaimed, B claimed | `only_the_unclaimed_versions_entries_are_removed` | 4 |
| A and B claimed | `two_claimed_versions_lose_nothing` | 4 |
| entries absent at `exec` | `a_cold_dispatch_prunes_before_exec` | 7 |
| root lock held, cold store | `a_held_root_lock_does_not_delay_a_cold_dispatch` | 7 |
| claim written before `find` | `an_override_root_dispatch_writes_its_claim_before_find` | 3 |
| young claim not rewritten | `an_override_root_dispatch_with_a_young_claim_does_not_rewrite_it` | 3 |
| symlink and foreign claims | `a_symlinked_and_a_foreign_owned_claim_protect_nothing` | 4 |
| default root untouched | `the_plugin_bin_root_is_never_claimed_or_pruned` | 7 |
| default orphan reap | `the_plugin_bin_orphan_reap_is_unchanged` | 7 |
| safety changes in default root | re-resolve, replacement and no-new-lock tests run in both kinds | 2, 5 |
| default verb reclaims superseded | `the_plugin_bin_verb_still_reclaims_an_unclaimed_superseded_tree` | 5 |
| `ENOENT` re-resolve | `an_entry_removed_between_find_and_exec_is_refetched_once` | 2 |
| second failure / non-`ENOENT` | `a_second_missing_program_surfaces_as_exec`, `a_failure_other_than_enoent_is_not_re_resolved` | 2 |
| foreign names kept | `foreign_names_survive_every_trigger` (verb, cold store, materialise) | 4, 5, 7 |
| kept names beside unclaimed | `the_kept_names_survive_beside_an_unclaimed_version` (verb, cold) | 4, 7 |
| plugin `.tmp-` staging | `plugin_staging_entries_are_removed_only_once_aged`, `launcher_staging_with_a_live_pid_…`, `…_with_a_dead_pid_goes_once_aged` | 4 |
| shim copies | `the_running_plugins_shim_copies_survive_for_every_platform`, `stale_shim_copies_go_on_every_platform`, `an_unreadable_shim_source_keeps_that_platforms_copies` | 4 |
| unsafe root not pruned | `a_shared_group_writable_root_…`, `a_user_private_group_writable_root_is_pruned`, `a_symlinked_root_…`, `a_root_under_a_writable_ancestor_…`, `a_user_owned_world_writable_root_is_not_pruned`, `a_per_user_root_under_a_sticky_shared_parent_is_pruned`, `a_root_owned_sticky_root_is_not_pruned`, `a_root_owned_by_another_uid_is_not_pruned`, `the_verb_refuses_an_unsafe_root_…`, `an_unsafe_root_is_not_pruned_from_any_trigger` | 4, 7 |
| `--older-than` | `older_than_narrows_the_version_claim_window`, `older_than_narrows_the_tree_claim_window` | 4, 5 |
| unreadable tree claims fail closed | `an_unreadable_tree_claims_directory_…` (both roots) | 5 |
| running version protected | `the_running_version_survives_older_than_zero`, `the_running_version_survives_without_a_valid_claim` | 4 |
| built-ins claim | `a_built_in_command_refreshes_the_version_claim` | 3 |
| verb removes both kinds | `the_verb_reclaims_single_file_entries_and_an_unclaimed_tree` | 5 |
| verb waits for locks | `the_verb_waits_for_a_held_root_lock`, `the_verb_waits_for_a_held_tree_lock` | 4, 5 |
| materialise with root lock held | `a_materialise_with_the_root_lock_held_still_prunes_other_trees` | 7 |
| no self-probe of a held lock | `no_prune_path_probes_or_takes_a_lock_it_holds` | 5, 7 |
| X held, Y pruned | `a_cold_dispatch_skips_a_held_tree_lock_and_prunes_the_rest` | 7 |
| lease outcomes | `only_aged_unknown_and_free_unpinned_trees_are_reclaimed` | 5 |
| orphans, sidecars, retired lock | `old_orphans_sidecars_and_retired_locks_go_and_young_generations_stay` | 5 |
| retired and other platform | `unclaimed_retired_and_foreign_platform_trees_go_and_claimed_ones_stay` | 5 |
| claim appearing during a prune | `a_tree_claimed_after_the_index_was_read_is_kept`, `the_pointer_goes_before_its_generation` | 5 |
| tree claim ages | `only_tree_claims_outside_the_window_are_removed` | 5 |
| sidecar ages and holds | `only_old_unheld_orphan_sidecars_are_removed` | 5 |
| lock outlives its residue | `a_tree_lock_is_kept_while_its_partial_archive_remains`, `a_retired_partial_archive_goes_with_its_lock` | 5 |
| lock file ages | `orphan_tree_locks_go_only_once_aged_and_never_for_a_current_tree` | 5 |
| other layout kept | `another_layouts_tree_is_kept_and_this_layouts_twin_is_removed` | 2, 5 |
| foreign generation-like directory | `an_aged_foreign_directory_resembling_a_generation_survives_the_reap` | 2 |
| symlinked directories and claims | `a_symlinked_tree_claim_…`, `a_symlinked_bootstrap_lock_…`, `a_symlinked_trees_or_tree_claims_directory_…` | 3–5 |
| pointer containment | `a_pointer_naming_an_absolute_path_…`, `a_pointer_escaping_with_dot_dot_…`, `a_pointer_naming_another_artifacts_…`, `a_malformed_pointer_is_kept` | 2, 5 |
| probe ages | `old_probe_files_are_swept_and_young_ones_kept` | 4 |
| bootstrap names coherent with the shell | `test_bootstrap_name_coherence.py` | 4 |
| bootstrap lock directories | `stale_bootstrap_locks_with_empty_or_dead_pids_are_swept` | 4 |
| materialise runs root prune | `a_materialise_prunes_the_superseded_tree_single_files_and_probes` | 7 |
| paused prune vs store | `a_store_after_listing_…`, `a_store_before_listing_…`, `a_store_without_a_claim_…` | 4 |
| two concurrent prunes | `a_cold_prune_behind_a_paused_prune_returns_without_touching_anything` | 7 |
| loopback guard on real artefacts | `test_a_release_binary_built_with_test_loopback_is_rejected` (jira, linear, research, launcher) | 6, 7 |
| A/B/C end-to-end | `test_a_relocated_root_keeps_only_the_current_versions_footprint` | 7 |
| claim removal order | `stale_claims_go_independently_and_a_last_claim_goes_after_its_entries` | 4 |
| pre-claim backstop | `pre_claim_entries_younger_than_the_backstop_survive` | 4 |
| suffix matching | `a_version_is_matched_by_suffix_never_by_splitting_the_name` | 4 |
| read-only override root | `a_read_only_override_root_still_dispatches_warm` | 3 |
| corrupt-entry rename | `a_corrupt_entry_is_replaced_by_a_rename_with_no_unlink_before_it` | 2 |
| unset override fixed list | `an_unset_override_performs_exactly_the_fixed_calls` | 1, 3 |
| override naming plugin `bin` | `naming_the_plugin_bin_adds_only_the_canonical_resolution` | 3 |
| `ENOLCK` root lock | `an_enolck_root_lock_skips_…`, `the_verb_reports_a_failed_root_lock_…` | 4, 7 |
| `ENOLCK` tree lock | `an_enolck_tree_lock_skips_…`, `the_verb_reports_a_failed_tree_lock_…` | 5, 7 |
| development checkout | `a_development_checkout_relocated_elsewhere_…` (claims, prunes) | 3, 7 |
| held `trees/.tmp-` generation | `a_held_tree_temp_generation_is_untouched_by_single_file_prune` | 4 |
| fresh claim warm cost | `a_fresh_claim_costs_one_metadata_read_beyond_the_fixed_list` | 3 |
| documentation | Phase 8 edits, `CHANGELOG.md`, `docs:check` | 8 |
| measurement | committed JSON | 8 |
| `mise run` green | full default task | 8 |

### Manual Testing Steps:

1. Point `ACCELERATOR_CACHE_DIR` at a scratch directory and dispatch any
   sub-binary. Confirm `claims/` appears and contains one claim.
2. Plant an old version's entries, back-dated two weeks, and dispatch a
   sub-binary not yet cached. The old entries are gone before it runs.
3. Unset `ACCELERATOR_CACHE_DIR` and repeat step 2 against the plugin
   `bin/`. Nothing is claimed or removed.

## Performance Considerations

- ⏱️ **Default root:** the warm path is unchanged. The fixed-list test proves
  it.
- ⏱️ **Relocated root, warm:** each process adds two `stat` calls, one
  `lstat` and the launcher-id hash, plus one `open` per `CLAIM_WINDOW/10`
  (~1.4 days). `identify` runs once per process, so a tree-consuming
  dispatch pays this once.
- ⏱️ **Cold path:** it gains one listing each of the root, `trees/` and
  `trees/claims/`, one lock attempt per artifact, one SHA-256 of a ~480 KB
  verifier shim per platform with a staged copy (usually one), and removals.
- ⏱️ **Tree removal before `exec`:** the first cold path after an upgrade in
  a long-shared root removes every unclaimed tree, synchronously, before the
  dispatch execs. Each `browser` or `driver` tree is hundreds of MB and
  thousands of files: roughly 0.2–1 s per tree on local disk, and more on a
  network filesystem. Later cold paths remove none until another version
  goes stale. The work item accepts this cost, and its A/B/C criterion needs
  every unclaimed tree gone after one dispatch, which rules out spreading the
  removals. In practice the batch lands on the first SessionStart hook
  dispatch of a newly installed version, because its hook sub-binaries are
  not yet cached, and it runs inside that hook's timeout. Phase 7's manual
  verification times both an interactive and a hook-triggered cold path.

## Migration Notes

There is no data migration. The first prune by a claim-aware launcher in a
relocated root evicts pre-claim versions' entries older than `AGE_BACKSTOP`,
and a live pre-upgrade session re-fetches after each claim-aware prune,
unable to dispatch offline until it is reloaded. The rollout window's
risks are listed under Accepted Risks.

## References

- Original work item: `meta/work/0218-bound-cache-root-growth.md`
- Codebase research:
  `meta/research/codebase/2026-10-18-0218-bound-cache-root-growth.md`
- Decision:
  `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
- Lease and layout rules:
  `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
- Precedent plans:
  `meta/plans/2026-10-16-0339-tree-reap-cross-artifact-lock-probe.md`,
  `meta/plans/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`
- Claims precedent: `cli/launcher/src/launch/outbound/resolve/tree/claims.rs`
- Feature-guard precedent: `cli/research-cli/src/main.rs:71-76`,
  `tasks/build.py:354-371`
