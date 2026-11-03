---
type: "codebase-research"
id: "2026-10-18-0218-bound-cache-root-growth"
title: "Codebase surface for bounding a relocated cache root's growth (0218)"
date: "2026-10-18T16:24:49+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0218"
parent: "work-item:0218"
relates_to: ["codebase-research:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness", "codebase-research:2026-10-16-0339-tree-reap-cross-artifact-lock-probe", "codebase-research:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"]
topic: "Codebase surface for bounding a relocated cache root's growth (0218)"
tags: ["research", "codebase", "launcher", "cache", "tree-cache", "claims", "prune", "bootstrap"]
revision: "96341212dc7c2e4bc3403d5d9472c429bc336b38"
repository: "accelerator"
last_updated: "2026-10-18T16:24:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Codebase surface for bounding a relocated cache root's growth (0218)

**Date**: 2026-10-18 17:24 BST
**Author**: Toby Clemson
**Git Commit**: 96341212dc7c2e4bc3403d5d9472c429bc336b38
**Branch**: detached (`build-system` workspace)
**Repository**: accelerator

## Research Question

What does the codebase look like today for every surface work item 0218
touches? That covers version claims, single-file prune, widened tree prune,
the residue sweep, cold-path root prune, the `ENOENT` re-resolve, the
filesystem-call recorder and the documentation. What must an implementation
plan build on, change or work around?

## Summary

Most of the tree-side machinery 0218 extends already exists and is merged.
That includes claims, the lease-liveness seam, the age backstop, the
non-creating lock probe from 0339 and inode confirmation. The single-file
side has none of it: no claims, no prune, no filesystem seam. Six findings
shape the plan more than the work item's text suggests:

- **Root identity.** Nothing in the Rust code tells an override root from
  `${ACCELERATOR_PLUGIN_ROOT}/bin`: `cache_root::candidate` returns an
  untagged `PathBuf`, and nothing canonicalises it. The shell passes
  `ACCELERATOR_CACHE_DIR` through verbatim, even when relative. A new domain
  type for the cache root's kind, decided by canonical comparison, is the
  first piece 0218 needs.
- **No filesystem seam.** `cache.rs`, `cache_root.rs` and `claims.rs` call
  `std::fs` and `geteuid()` directly. The filesystem-call recorder therefore
  needs a launcher source change, a port through which the warm path's calls
  flow. The work item already allows a separate commit for that.
- **Tree materialise is not on the dispatch path.** Dispatch only *acquires*
  trees (`core.rs:327-355`). Materialise runs only through
  `cache ensure`/`repair`, which the design executor invokes. "Every cold path
  … a tree materialise" therefore means those verbs' materialise.
- **Today's verb gets three of 0218's rules wrong.**
  - `cache prune` creates tree locks with `O_CREAT` for every pinned artifact.
  - It fails with `ENOENT` when `trees/` is missing.
  - It visits only `pins::artifact_names()` on `HOST_PLATFORM`.
  - It aborts on the first lock error, and no error type separates `ENOLCK`
    from other lock failures.
- **`run_external` has no seam between `resolve` and `exec`.** The `ENOENT`
  re-resolve and the `find`/`exec` removal hook both need one. The
  `ResolveBinary` port also offers no way to force the cold path.
- **The end-to-end criterion cannot use a locally signed fixture as-is.** The
  real launcher trusts only the embedded release key
  (`keys.rs:11-12,55-57`), so a sub-binary "fetched from a local fixture"
  through `bin/accelerator` will not verify. See Open Questions.

## Detailed Findings

### Cache root derivation and the override/default distinction

- **Rust.** `CacheRootConfig::from_env` reads `ACCELERATOR_CACHE_DIR` and
  treats an empty value as unset (`cache_root.rs:36-44`). `candidate()` then
  returns the override unchanged, or `plugin_root.join("bin")`
  (`cache_root.rs:58-74`).
  - Two tests pin that `candidate` does no I/O (`cache_root.rs:228,345`).
  - `candidate()` has three call sites: dispatch (`main.rs:81-83`), the
    `cache` verbs (`main.rs:323-325`) and tree acquisition (`main.rs:417-420`).
  - `plugin_root` comes from `ACCELERATOR_PLUGIN_ROOT` unchanged
    (`cli/config-adapters/src/store.rs:230-232`).
- **Shell.** `bin/accelerator:95-117` resolves `plugin_root` fully (symlink
  loop, then `pwd -P`) and exports it at `:122`. `resolve_cache_dir`
  (`:198-207`) uses `ACCELERATOR_CACHE_DIR` verbatim: no `pwd -P`, no
  trailing-slash strip, and relative paths stay relative.
- **Consequence.** The default root reaches Rust already canonical; the
  override does not. The canonical comparison 0218 specifies must happen in
  Rust, and only when `ACCELERATOR_CACHE_DIR` is set. That keeps the
  unset-variable warm path's call list unchanged, as the criterion requires.
  Canonicalising a relative override resolves it against the working
  directory, which matches what the shell does.
- **Dev checkouts.** Neither side special-cases them. The plugin root is the
  repo root, so the default root is the repo's `bin/`, whose residue is
  gitignored at `.gitignore:40-62`.

### Single-file cache: `find`, `store`, `reverify`

All references are to `cli/launcher/src/launch/outbound/resolve/cache.rs`
unless noted.

- **`find`** (`:51-73`).
  - It makes one `read_dir(root)`, matches the prefix `"{name}-{version}-"`
    plus a 64-hex remainder (`is_sha256_hex`, `:25-27`), and `stat`s the
    sidecar with `.exists()` (`:64`). It returns the **first** match in
    `read_dir` order.
  - The first non-UTF-8 name aborts the whole scan with `None` (`:56`).
  - `name` is the bare dispatched token (e.g. `visualiser`), and `version` is
    `env!("CARGO_PKG_VERSION")` (`resolve/mod.rs:52`).
- **Warm-path cost today.** `find`'s `read_dir` and `stat`s, then `reverify`
  (`resolve/mod.rs:100-119`). `reverify` makes a full `fs::read` of the
  binary and a `read_to_string` of the sidecar, then `verify_binary`
  (`verifier.rs:29-50`). This is the fixed list the recorder must capture.
- **`store`** (`:81-133`).
  - `create_dir_all(root)`, then temporary names
    `.tmp-{name}-{version}-{sha}-{pid}-{seq}[.minisig]` (`:37-39`, `:95-99`).
    `TEMP_PREFIX` comes from `cli/kernel/src/lib.rs:15`.
  - `write_then_rename` runs for the binary **then** the signature
    (`:103-109`, `:118-133`). Write is `0600`, then `chmod 0755`, with no
    fsync.
  - The temporary file is removed only if the rename fails. A failed write or
    chmod leaves it behind.
  - There is no unlink before rename. Replacement relies on rename atomicity
    (module doc `:4-6`). This is the invariant 0218 must test with the
    recorder.
- **Corrupt-entry replacement** (`resolve/mod.rs:213-239`). A `reverify`
  mismatch or I/O error falls through to `fetch_verify_store`. The new entry
  lands on the same path only when the manifest sha equals the corrupt
  entry's sha (`cache.rs:211-220`, `tests/resolution.rs:609`). If the shas
  differ, the old entry stays, and `find` may keep returning it first.
- **Ordering windows relevant to `.minisig` pairing.**
  - `store` renames the binary before the sidecar, so a fresh entry briefly
    has no `.minisig`.
  - The shell renames the sidecar first (`bin/accelerator:432`), so a
    `.minisig` briefly has no entry.
  - Under 0218's pairing rule both transient states read as "not a
    single-file entry" and are kept, which is safe.

### Dispatch, cold path and exec

- **Flow.** `run` (`main.rs:444-464`) calls `export_consumed_trees` (for
  `design` only), which holds leases across exec. `dispatch` then calls
  `run_external` (`launch/mod.rs:202-206`, `core.rs:313-322`):

  ```rust
  match resolver.resolve(command) {
      Ok(program) => executor.exec(&program, &command.args),
      Err(error) => error,
  }
  ```

- **`LazyProductionResolver::resolve`** (`main.rs:72-92`). The override
  `ACCELERATOR_<SUB>_BIN` short-circuits with no cache access
  (`outbound/mod.rs:21-47`). Otherwise it builds `FetchVerifyCacheResolver`.
  This is where a version-claim refresh "before `find`" would sit, or inside
  `FetchVerifyCacheResolver::resolve` before `cache::find`
  (`resolve/mod.rs:201`).
- **Cold path.** `fetch_verify_store` (`resolve/mod.rs:147-187`) runs:
  1. `verify_writable` probe
  2. manifest fetch and verify
  3. asset fetch
  4. `verify_binary`
  5. `cache::store`
  6. return the path

  Root prune "after a successful `store`, before `exec`" fits immediately
  after `cache::store` at `:178`.
- **`UnixExec::exec`** (`outbound/exec.rs:12-23`) wraps the `io::Error` from
  `Command::exec()` in `ResolutionError::Exec { program, source }`
  (`core.rs:40-96`, display `:173-177`). It classifies as `Failed`
  (`core.rs:215`, pinned at `:625-628`).
- **Re-resolve gaps.**
  - `ResolveBinary` (`core.rs:294-302`) has only `resolve`. A forced cold
    resolve needs a new port method, or a resolver mode that skips `find`.
  - The override short-circuit has no cache to re-resolve from; see Open
    Questions.
  - No test drives a real `UnixExec` failure end-to-end.
- **Test fakes.**
  - `core.rs` tests have `RecordingExec`, `FixedResolver` and
    `FailingResolver` (`core.rs:456-548`).
  - `main.rs` tests have `FailingResolver<F>` and `UnreachableExec`
    (`main.rs:651-746`).
  - `tests/dispatch.rs` runs the real binary against `accelerator-fixture`.

### Tree claims (precedent for version claims)

All references are to `tree/claims.rs` unless noted.

- **Path and window.**
  - A claim's path is `trees/claims/<digest>.<launcher-id>` (`:38`, layout
    `:137-139`).
  - `CLAIM_WINDOW` is public and equals 14 days (`:20`).
  - `REFRESH_FRACTION` is private and equals 10; the rewrite is skipped while
    the claim's age is below `CLAIM_WINDOW/10` (`:24`, `:99-105`).
- **Validity.** `is_valid_claim` uses `symlink_metadata`, rejects symlinks and
  requires `uid == geteuid()` (`:107-115`). There is no uid seam today.
  `layout.rs:360` also calls `geteuid()` directly.
- **Read and drop.**
  - `claimed_within(paths, digest, window, clock)` (`:54-82`) returns false
    when the directory is unreadable.
  - `drop_all` (`:86-97`) removes every `"{digest}."` claim with no validity
    check.
  - There is no per-file stale-claim removal. 0218 adds it for both tree and
    version claims.
- **Best-effort writes.** Every error is ignored (`:42-44`, `:133-141`). That
  matches 0218's rule that a read-only root never fails a dispatch.
- **`launcher_id`** (`main.rs:353-366`). The first 8 bytes of SHA-256 over
  `current_exe()`, as 16 hex characters, falling back to `CARGO_PKG_VERSION`.
  Today it is passed only to `TreeResolver` (`main.rs:341,435`). Version
  claims need it on the dispatch resolver path too.
- **Directory hardening.** `trees/` is created `0700` and ownership-checked
  (`resolver.rs:949-975`, `layout.rs:30-33`). The proposed `<root>/claims/`
  sits outside `trees/` and inherits the root's mode, so it gets no such
  check unless 0218 adds one.
- **Claim-refresh gap.** `materialise`'s early returns on a `locate` hit
  (`resolver.rs:618-637`) do not call `write_claim`. A `cache ensure` hit
  therefore does not refresh the tree claim. `acquire` does
  (`resolver.rs:565-567`).

### Tree reap, tree prune and the verb

All references are to `tree/reap.rs`, `tree/resolver.rs`, `tree/lease.rs` and
`launch/cache.rs`.

- **`AGE_BACKSTOP`** is private, 1 hour (`reap.rs:47`). Integration tests
  duplicate it as `BACKSTOP_SECONDS = 3600` (`tests/tree_resolution.rs:1391`).
  0218 lifts it to a module both reapers share.
- **Decision tables** (`reap.rs:330-365`), all pure `const fn`:

  | Rule | Held | Free | Unknown |
  |---|---|---|---|
  | `unclaimed_is_reclaimable` | keep | reclaim | reclaim once aged |
  | `superseded_disposition` | Keep | Reclaim | Defer |
  | `orphan_is_reclaimable` | keep | reclaim once aged | reclaim once aged |

  0218's tree-prune rule ("free, or unknown and older than `AGE_BACKSTOP`")
  is `unclaimed_is_reclaimable`.
- **`reap_orphans`** (`reap.rs:137-146`). Its scope is fixed by name
  classification:
  - Generations must carry `-{LAYOUT_VERSION}-` and no sidecar suffix
    (`is_generation`, `:281-291`).
  - Temp residue is grouped by `Artifact::owning` (`layout.rs:77-86`), which
    yields nothing for lock files and unknown platforms.
  - It never classifies sidecars whose generation is absent. Lone `.lease`
    files from `acquire`'s miss path therefore accumulate, and 0218's sidecar
    rule covers them.
- **`Reaper`** takes a `&HeldTreeLock` as proof of the caller's own lock.
  `lock_of` short-circuits `HeldByCaller` and probes other artifacts with
  `probe_single_flight`, which does not create files (`reap.rs:91-119`,
  `:172-184`). A probe error becomes `Unprobeable`: a warning, residue kept.
- **`TreeResolver::prune`** (`resolver.rs:765-782`).
  1. `HeldTreeLock::take`, which blocks and **creates** the lock
     (`tree_lock.rs:20-26` → `take_single_flight` → `open_creating`,
     `lease.rs:85-90,283-292`).
  2. `reclaim_unclaimed` (`resolver.rs:786-820`):
     - It skips the launcher's own pinned digest, claimed digests and
       non-reclaimable leases.
     - It then removes the pointer, generation, sidecars and `drop_all`
       claims, ignoring every removal error.
     - It visits only pointers prefixed `"{artifact}-{platform}-"`
       (`artifact_pointers`, `:824-852`).
  3. `reap_orphans`.
- **`cache prune` verb** (`launch/cache.rs:143-155`).
  - It loops over `pins::artifact_names()` (today `browser`, `driver`) on
    `resolver.platform` (`HOST_PLATFORM`, `main.rs:335`). The `?` aborts the
    loop on the first error.
  - It prints `reclaimed {n} entries` only on full success. Errors exit 1
    through `report` (`main.rs:466-472`).
  - `--older-than` (`inbound/cli.rs:71-76`) replaces only the claim window.
- **Verb gaps against 0218.**
  - It creates lock files for never-materialised artifacts. The fix is to
    use `open_existing`/`probe_single_flight`-style opening when no pointer
    or generation exists.
  - It calls neither `ensure_trees_dir` nor `verify_writable`, so a missing
    `trees/` fails with `ENOENT` and exits 1.
  - It never visits retired artifacts or other platforms.
  - It never reaps individual stale claims, orphan sidecars or lock files.
- **Lock errors** (`lease.rs:193-232`). `lock_confirmed` maps
  `WOULDBLOCK` to `Held`, retries `ESTALE` up to `MAX_LOCK_ATTEMPTS = 8`, and
  turns every other errno into `TreeError::Lease { detail }`.
  - `ENOLCK` is not distinguished at the type level.
  - `probe_single_flight` returns `Err` rather than folding errors into
    `Held`. 0339's research says it does this so 0218 can report lock
    failures.
  - The "skip on cold path, non-zero from verb" rule needs the error kept
    separate from contention, which `try_single_flight`'s `Ok(None)` already
    does.
- **Inode confirmation** (0339, merged). `placement` compares `(dev, ino)`
  after locking, and `SingleFlight` keeps two descriptors because POSIX
  emulation drops the lock on any close (`lease.rs:63-77,138-166`).
  Orphan-lock removal must unlink only while holding the lock.

### Pins and platforms

- `build.rs:57-101` turns `pins.toml` into
  `EXPECTED_TREE_DIGESTS: &[(&str, &str, &str)]`, included at `pins.rs:9`.
  - Lookup functions: `artifact_names()`, `expected_digest(artifact)` (host)
    and `expected_digest_on(artifact, platform)` (`pins.rs:19-60`).
  - "The digest the running launcher pins for that artifact" on another
    platform is available through `expected_digest_on`.
  - Tests use `ExpectedDigests::Fixed` (`resolver.rs:73-99`).
- `KNOWN_PLATFORMS` (`resolve/mod.rs:37-38`): `darwin-arm64`, `darwin-x64`,
  `linux-arm64`, `linux-x64`. The bash bootstrap (`:125-137`) and
  `tasks/shared/targets.py:4-28` mirror it, and
  `tests/unit/tasks/test_manifest_contract.py:60-74` keeps the Rust and
  Python copies coherent. 0339 recorded that the list may only grow.

### Shell bootstrap residue in the cache root

| Shape | Writer | Lifecycle today |
|---|---|---|
| `accelerator-launcher-<ver>-<platform>[.minisig]` | `bin/accelerator:404-405,432` | never removed |
| `.tmp-launcher-<pid>[.minisig]` | `:416-417` | removed on fetch/verify failure; left on `mv` failure or kill |
| `accelerator-verify-<platform>-<64hex>` | `:328,345-350` | stale digests ignored, never removed (test near entrypoint `:1590`) |
| `…-<64hex>.staging.<pid>` | `:345-346` | left on `fail_integrity` or kill |
| `.accelerator-lock-<platform>/pid` | `:356-402` | released by the trap; reclaimed by the next cold path when `kill -0` fails |
| `.accelerator-probe-<pid>` | `:183-194` | removed immediately |
| `.accelerator-unverified.log` | `:61-71,249-252` | append-only; read by `hooks/launcher-link-refresh.sh:35-38` |

Notes:

- The staged shim is checked at `:336` and used at `:409-411`/`:427`. That
  is the window 0218's assumption about a pruned shim copy names.
- The bootstrap lock's stale check treats `EPERM` from `kill -0` as dead
  (`:391-393`). For a Rust `pid`-liveness seam, mapping `EPERM` to alive is
  the conservative choice. An empty `pid` is a normal transient state
  between `mkdir` and the write at `:375`, which `AGE_BACKSTOP` covers.
- The Rust probe is `.accelerator-probe-<pid>-<seq>`
  (`cache_root.rs:124-161`). It is removed always, but a kill can leave it.
- The launcher-entry version comes from `plugin.json` (`:141-144`), while
  sub-binary entries use `CARGO_PKG_VERSION`. The two agree in a release.
  The entrypoint harness, however, writes `VERSION = "9.9.9-test"`
  (`tests/integration/support/installation.py`). A version claim keyed on
  `CARGO_PKG_VERSION` therefore will not protect that harness's launcher
  entry; only `AGE_BACKSTOP` will.

### Existing seams, hooks and test infrastructure

- **Trait seams.** The launcher style is trait objects passed as `&dyn`.
  `resolver.rs:36-71` explains why they are not cargo features: `--all-features`
  runs in `cli:check` and `test:unit:cli`.
  - `Clock` (`core/tree.rs:363-368`)
  - `LivenessProbe` (`tree/lease.rs:21-34`)
  - `StepObserver`/`NoSteps`
  - `RemovalObserver` (`reap.rs:55-65`), the nearest thing to a recorder
  - `ExpectedDigests::Fixed`
  - injected open/lock closures in `lock_confirmed`
- **Feature-gated seams exist only outside the launcher.** `research-cli`'s
  `test-loopback` has a release-build `compile_error!` guard
  (`cli/research-cli/src/main.rs:71-76`). The launcher's only feature is
  `bash-parity`.
- **Pause hooks.**
  - Channel handshakes inside lock closures (`lease.rs:562-587`)
  - `Barrier` starts (`tree_resolution.rs:717-746`)
  - `within(BLOCKING_BOUND = 10s)` (`tree/testing.rs:40-58`,
    `tests/common/mod.rs:302-320`)
- **No syscall recording exists.** The nearest patterns:
  - the `Recording` `RemovalObserver` (`reap.rs:569-584`)
  - lock-file-set snapshots (`reap.rs:547-598`)
  - read-only roots as a write-absence witness (`tree_resolution.rs:2062-2100`)
  - the Python `FilesystemWitness` (`tasks/measure.py:649-699`)
- **Back-dating.** There is no `filetime` dependency. Tests use
  `File::set_modified` (`tree_resolution.rs:1589-1596`) or advance the
  injected clock (preferred, `reap.rs:415-418`).
- **Harnesses.**
  - `tests/resolution.rs` `Harness` (`:100-211`) seeds with `cache::store`
    and counts probes.
  - `tests/tree_resolution.rs` (`:274-777`) builds resolvers, plants
    sidecars, partials and generations, and holds locks
    (`bystander_in_progress`). It also has `run_cache` for the verb, and
    `launcher-fixture-lease-holder` for real held leases (`:2551-2620`).
  - The bootstrap end-to-end suite is
    `tests/integration/entrypoint/test_accelerator_entrypoint.py`, on
    `tests/integration/support/installation.py` (`make_installation`,
    `run_bootstrap` with `extra_env`, `real_launcher=True`). Both need
    `minisign` and fail hard under `CI`.

### Documentation and measurement

- **`docs-site/src/content/docs/design.md`.**
  - `:203-211` is the `accelerator cache` verbs table; `prune` reads
    "Reclaim the cache roots Claude Code's orphan sweep never reaches".
  - `:264` is the `ACCELERATOR_CACHE_DIR` environment row (trust-relevant,
    local filesystem only, `flock` unreliable on NFS).
  - `:271-275` covers ephemeral environments.
- **`internals.md`.** `:263` is the `ACCELERATOR_CACHE_DIR` row in "Offline,
  mirrored and read-only installs" (`:255-313`). It has no prune text. The
  read-only warm-path guarantee at `:273-284` must survive 0218: claim writes
  are best-effort.
- **`measure:warm-dispatch`** (`mise.toml:389-396`, `tasks/measure.py:1130-1154`).
  - It writes `meta/measurements/warm-dispatch-{N}.json` (`:1483-1508`).
  - It refuses every `ACCELERATOR_*` variable except
    `ACCELERATOR_RELEASE_BASE_URL`
    (`tasks/shared/measurement.py:605-620`, `measure.py:1610-1635`).
  - It watches only `plugin_root / "bin"`.
  - The cache-root entry classifier (`measure.py:767-786`) flags leaked
    `.tmp-` files and orphaned locks. The post-change run measures the
    default root only, as 0218 states.

## Code References

- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:36-74` — override vs default root selection; no canonicalisation
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:124-161` — Rust probe file `.accelerator-probe-<pid>-<seq>`
- `cli/launcher/src/launch/outbound/resolve/cache.rs:51-73` — `find`
- `cli/launcher/src/launch/outbound/resolve/cache.rs:81-133` — `store`, rename-without-unlink
- `cli/launcher/src/launch/outbound/resolve/mod.rs:100-119` — `reverify` (warm-path reads)
- `cli/launcher/src/launch/outbound/resolve/mod.rs:147-243` — `fetch_verify_store` and `resolve`
- `cli/launcher/src/launch/outbound/resolve/keys.rs:11-12,55-57` — embedded release key is the only production trust root
- `cli/launcher/src/launch/core.rs:294-322` — `ResolveBinary`, `ExecBinary`, `run_external`
- `cli/launcher/src/launch/core.rs:40-96,173-177,215` — `ResolutionError`, `Exec` display and class
- `cli/launcher/src/launch/outbound/exec.rs:12-23` — `UnixExec`
- `cli/launcher/src/main.rs:72-92` — `LazyProductionResolver::resolve`
- `cli/launcher/src/main.rs:321-366` — `run_cache` wiring and `launcher_id`
- `cli/launcher/src/launch/cache.rs:143-155` — `cache prune` verb loop
- `cli/launcher/src/launch/inbound/cli.rs:71-76` — `--older-than`
- `cli/launcher/src/launch/outbound/resolve/tree/claims.rs:20-141` — tree claims
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:47` — `AGE_BACKSTOP`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:91-365` — `Reaper`, classification, decision tables
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:588-652` — `materialise` and post-materialise reap
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:765-852` — `prune`, `reclaim_unclaimed`, `artifact_pointers`
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:85-292` — single-flight take/try/probe, `lock_confirmed`, openers
- `cli/launcher/src/launch/outbound/resolve/tree/tree_lock.rs:20-41` — `HeldTreeLock::take`/`try_take`
- `cli/launcher/src/launch/outbound/resolve/tree/layout.rs:77-86,137-247` — `Artifact::owning`, tree path shapes
- `cli/launcher/src/launch/outbound/resolve/tree/pins.rs:19-60` — pin enumeration
- `cli/lease/src/lib.rs:38-48,166-241` — `Liveness`, `probe_shared_lease`, NFS → `Unknown`
- `bin/accelerator:198-219,300-352,354-457` — cache root, shim staging, lock directory, launcher entry
- `tests/integration/support/installation.py` — bootstrap end-to-end harness
- `cli/launcher/tests/tree_resolution.rs:274-777,1381-1391,2551-2620` — tree harness, liveness fake, lease holder
- `cli/launcher/tests/resolution.rs:100-211` — single-file resolver harness
- `docs-site/src/content/docs/design.md:203-211,264` and `internals.md:263,273-284` — docs to update
- `tasks/shared/measurement.py:605-620` — `ACCELERATOR_*` refusal in the measurement harness

## Architecture Insights

- **Ports and adapters, with seams as traits.** Every existing test seam
  (`Clock`, `LivenessProbe`, `StepObserver`, `RemovalObserver`) is a trait
  injected into a resolver struct, with a no-op production implementation.
  The uid seam, the `pid`-liveness seam, the lock-failure seam, prune-pause
  hooks and the filesystem-call recorder should follow the same pattern
  rather than cargo features, for the `--all-features` reason documented at
  `resolver.rs:36-71`.
- **Proof-carrying locks.** `HeldTreeLock` is required to build a `Reaper`,
  so the lock discipline is enforced by type. A root prune that iterates
  artifacts, or reuses a materialise's own lock, needs the same proof plus
  the `HeldByCaller` short-circuit so it never probes a lock it holds.
- **Pure decision rules separate from I/O.** Reclaim decisions are `const fn`
  over `Liveness` and an age flag. Single-file prune, which decides on claim
  freshness, age and `.minisig` pairing, and the residue sweep fit the same
  shape and can be unit-tested without a filesystem.
- **Name grammars are cross-version contracts.** `.tmp-` names under `trees/`
  are pinned by `temp_entry_names_are_fixed_across_launcher_versions`.
  0218's allowlist of top-level shapes extends that contract and deserves
  the same pinning.
- **Best-effort writes, fail-closed reads.** Claim writes ignore every error;
  `claimed_within` returns false on an unreadable directory. For version
  claims, an unreadable `claims/` must therefore keep entries, which the
  "unclaimed and older than `AGE_BACKSTOP`" gate does not do on its own.
  The prune must skip when it cannot list claims rather than treat that as
  "no claims".

## Historical Context

- `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
  (accepted 2026-10-15) — the decision 0218 implements.
  - It covers override roots only, version claims on every dispatch,
    non-blocking root prune on cold paths, and a waiting verb.
  - It records the rejected options (newest N, evict during `store`, LRU,
    SessionStart prune) and the accepted negatives.
- `meta/decisions/ADR-0063-plugin-version-scoped-artifact-cache.md` — still
  governs the default root and dev checkouts. It rejected "keep current pin
  plus previous" because coexisting versions would reap each other.
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
  — the lease gates reclamation, and a launcher never reaps another layout
  version.
- `meta/plans/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`
  (done, PR #162).
  - It introduced the `cli/lease` crate, the `LivenessProbe` seam and the
    decision tables.
  - It documented the gaps 0218 inherits: no claim refresh on `materialise`
    hits, and lone `.lease` sidecars.
- `meta/plans/2026-10-16-0339-tree-reap-cross-artifact-lock-probe.md`
  (done, PR #167).
  - It introduced `probe_single_flight`, inode confirmation and the "held
    tree lock means materialise or prune in progress" contract.
  - It left ownerless `.tmp-` entries and orphan-lock removal to 0218.
  - The PR #167 review suggests a test failing if any production path
    removes a `.tmp-*.lock` outside the holder discipline.
- `meta/work/0223-bounding-the-default-runtime-cache-root.md` — established
  that Claude Code's orphan sweep bounds the default root.
- `meta/reviews/work/0218-bound-cache-root-growth-review-1.md` — approved
  after six passes. The pass-6 fixes (pre-claim `AGE_BACKSTOP` gate, suffix
  matching instead of splitting names, per-file claim removal, pinned-digest
  control, layout-version pointer check) are all reflected in the item.

## Related Research

- `meta/research/codebase/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`
- `meta/research/codebase/2026-10-16-0339-tree-reap-cross-artifact-lock-probe.md`
- `meta/research/codebase/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`

## Open Questions

- **End-to-end trust root.** The all-versions criterion has C dispatch a
  sub-binary "fetched from a local fixture" through `bin/accelerator`. The
  real launcher verifies only against the embedded release key, so either:
  - the test uses pre-signed release fixtures, which is not feasible;
  - a test-only trust seam is added, which needs a release-build guard like
    `research-cli`'s `test-loopback`; or
  - the criterion reaches the store through a sub-binary override, which
    bypasses the cache and defeats the test.

  The plan must pick one.
- **Version identity.** Version claims key on which version: `plugin.json`
  (the launcher entry) or `CARGO_PKG_VERSION` (sub-binary entries and the
  resolver)? They agree in releases but not in the entrypoint harness
  (`9.9.9-test`).
- **Re-resolve with an override.** Should the `ENOENT` re-resolve apply when
  the path came from `ACCELERATOR_<SUB>_BIN`? There is no cache to
  re-resolve through, and a re-resolve would refetch something the user
  overrode.
- **Forced cold resolve.** "Re-resolves once through the cold path" means
  bypassing `find`, or else a `find` race could return a second stale
  entry. `ResolveBinary` needs a way to express that.
- **Unreadable `claims/`.** When the claims directory cannot be listed,
  single-file prune must not treat every version as unclaimed. The work item
  does not state this; the plan should.
- **`claims/` hardening.** The plan should decide whether the new
  `<root>/claims/` gets the `0700`/ownership treatment `trees/` has.
- **Multiple shas per version.** The corrupt-entry case where the manifest
  sha differs from the corrupt entry's sha leaves two `{name}-{version}-*`
  entries. Both are protected by the version's claim, and `find` may return
  the corrupt one first. This is outside 0218's scope but interacts with
  its corrupt-entry criterion.
- **Materialise hit and claims.** A `materialise` hit does not refresh its
  tree claim (`resolver.rs:618-637`). Automatic tree prune makes that gap
  more consequential for a version that only ever hits.
