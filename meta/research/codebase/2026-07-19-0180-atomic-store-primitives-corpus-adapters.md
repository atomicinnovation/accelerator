---
type: codebase-research
id: "2026-07-19-0180-atomic-store-primitives-corpus-adapters"
title: "Research: Porting the atomic-store primitives into corpus-adapters (0180)"
date: "2026-07-18T23:21:31+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0180"
parent: "work-item:0180"
relates_to: ["work-item:0166", "work-item:0179", "work-item:0172"]
topic: "Porting the bash atomic-store primitives (atomic_write, mkdir-lock, canonical JSONL) into the corpus-adapters Rust crate"
tags: [research, codebase, rust, corpus, corpus-adapters, atomic-store, jsonl, ports-and-adapters, cargo-deny]
revision: "61e1b9d3905ec815be161d6c33a432143944573f"
repository: "accelerator"
last_updated: "2026-07-19T00:55:00+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Added follow-up resolving the libc/musl concern and open questions 2 (user_value gating) and 4 (port shape) with caller/reader/call-surface evidence"
schema_version: 1
---

# Research: Porting the atomic-store primitives into corpus-adapters (0180)

**Date**: 2026-07-18T23:21:31+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 61e1b9d3905ec815be161d6c33a432143944573f
**Branch**: (working copy `vwkpknxlkspy`, off `main` @ `1361c70c`)
**Repository**: accelerator

## Research Question

Understand everything needed to implement work item **0180 — Atomic-Store
Primitives in corpus-adapters**: the exact semantics of the bash source
primitives being ported (`atomic_write`, the mkdir-based lock, canonical-order
JSONL compose/remove), the target Rust crates (`corpus` / `corpus-adapters`) and
their conventions, the in-repo atomic-write and port-trait precedents to mirror,
the error-taxonomy and test-double patterns, and the toolchain coupling
(`cargo-deny`, `cargo-pup`, workspace deps) that landing `tempfile` + `libc`
implies.

## Summary

The port is well-scoped and the work item's Technical Notes are largely accurate,
but the live codebase contradicts **three** of its framing claims, each of which
changes an implementation decision:

1. **A fallible store port would _not_ be "the first fallible port here."**
   `config`'s `ReadConfigLevel`/`WriteConfigLevel`
   (`cli/config/src/service.rs:29-45`) already return `Result<_, ConfigError>`.
   They are a *closer* structural analog for an `AtomicStore`/`RecordStore` pair
   than `vcs` (which the work item names), because they are a fallible
   reader+writer backing a document-persisting service. Mirror `config`, not
   `vcs`, for the fallible shape.

2. **`cli/deny.toml` is a _denylist_, not an allow-list.** The work item's
   Dependencies and Technical Notes say landing `tempfile`/`libc` "requires
   `cli/deny.toml`'s allow-list to admit them." There is no allow-list to extend:
   neither crate is banned, their `MIT OR Apache-2.0` licenses are already
   covered, `libc` and `rand` 0.9 are **already in the lock transitively**, and
   only `tempfile` is net-new to the graph. The one hard constraint is
   `wildcards = "deny"` — new deps must be pinned to a concrete version.

3. **`StoreError`'s location is _forced_ by cargo-pup, not merely "a live
   decision."** If the store port trait is declared in the `corpus` domain (as
   the notes intend) and is fallible, then `corpus`'s pup rule
   (`corpus_domain_imports_only_permitted`) allows only `std`/`core`/`alloc`,
   `kernel::Error`, and `crate::…` in its `use` paths — so a rich `StoreError`
   referenced by that trait signature **must live inside `corpus` itself**
   (`crate::StoreError`) or be `kernel::Error`. It **cannot** live in
   `corpus-adapters` and still be named by a `corpus`-declared trait. This is the
   real fork in the road, and it diverges from the existing `PatchError`
   precedent (a local adapter-crate error enum).

The rest of the report is the faithful semantics needed to port each primitive,
the precedents to copy, and the smaller discrepancies (fsync, `schema_version`
number encoding, `user_value` gating, 0x7F/0x0B escaping) that affect correctness.

## Detailed Findings

### 1. Bash source semantics — what must be preserved

Source: `scripts/atomic-common.sh` and `scripts/jsonl-common.sh`.

#### `atomic_write` (`scripts/atomic-common.sh:16-32`)

- Temp is created **in the target's directory** via
  `mktemp "$dir/.atomic-write.XXXXXX"` (`:26`) — load-bearing because `rename(2)`
  is atomic only within one filesystem.
- `mkdir -p "$(dirname target)"` first (`:23-24`); buffer all of stdin to the
  temp (`cat >"$tmp"`, `:29`); single `mv "$tmp" "$target"` (`:30`).
- An EXIT trap (`:28`) with the temp path **expanded into the trap string at
  install time** removes the orphan on interruption before the rename; on success
  `trap - EXIT` (`:31`) clears it so the renamed file survives.
- **No `fsync`, no permission preservation.** A crash between `cat` and `mv`
  leaves the prior file intact plus an orphan temp — never a partial destination.

#### mkdir-based lock (`_atomic_lock_acquire`, `:105-143`)

- **Why mkdir**: `flock` is non-POSIX / absent on stock macOS (`:82-89`); a
  successful `mkdir` of the sidecar lockdir *is* exclusive acquisition.
- **Writability pre-check** (`:110-115`): if `dirname(lockdir)` is not writable,
  return early (error) rather than spin the full ceiling — this is AC-5's
  early-error path.
- **RANDOM re-seed** (`:121-122`): `($$ * 31 + RANDOM) XOR nanoseconds`; note
  per-fork uniqueness comes from `date +%N`, and macOS `date` has **no `%N`**, so
  on macOS the nanosecond term is 0 and seeding leans on `$$`+RANDOM. (Falls away
  in Rust — seed a per-thread RNG.)
- **Back-off** (`:123-137`): `base_ms` starts at 4 (`:117`), doubles each
  iteration but stops once `>= 200`, so the sequence is 4→8→16→32→64→128→256 and
  caps at **256 ms**. Jitter each round is `(RANDOM % base_ms) + 1` ms, i.e.
  uniform `1..=base_ms`, slept as `0.%03d` seconds.
- **300 s ceiling** (`:129`): on **accumulated slept-ms** (`waited_ms`), *not*
  wall clock — excludes time in `mkdir`/reclaim syscalls. Exceeding it returns the
  lock-timeout error (AC-5).
- **Owner sentinel** (`:138-142`): after winning, `$BASHPID` is written to
  `<lockdir>/owner`. On bash 3.2 `$BASHPID` is unset → ownership skipped, lock
  degrades to spin-only. In Rust `std::process::id()` always exists (the bash
  3.2 fallback is moot).
- **TOCTOU window**: between `mkdir` success and the owner write, `owner` is
  missing; a competing waiter must treat missing/empty owner as **live** (AC-4).

#### Stale reclaim (`_atomic_lock_reclaim_if_stale`, `:157-163`)

```
owner=$(cat "$lockdir/owner") || return 0     # missing/unreadable → LIVE
[ -n "$owner" ] || return 0                    # empty → LIVE
kill -0 "$owner" && return 0                    # signalable → LIVE
rm -rf "$lockdir"                               # confirmed dead → reclaim
```

- `rm -rf` (not `rmdir`) on both reclaim (`:162`) and release (`:148`) because the
  lockdir carries the `owner` file. `kill -0` → `libc::kill(pid, 0)`.
- **Safety argument** (`:151-156`): a dead process cannot be in the critical
  section; PID reuse can only produce a false "alive" reading, which degrades to
  spin-and-wait — it can never break a genuinely live lock (AC-3).

#### Release via subshell-owned EXIT trap (`:200-209`, `:233-246`)

Both JSONL mutators run the critical section in a `( … )` subshell so the
release trap is scoped to it and doesn't clobber the caller's EXIT trap. Order is
**acquire-then-arm**: `_atomic_lock_acquire … || exit 1` *before* the release
trap is installed (`:201-202`), so a failed acquire never releases a lock it
never held. → Maps cleanly to a **RAII drop guard** in Rust: acquire returns the
guard; `Drop` calls release; a failed acquire yields no guard.

#### Anchored-prefix remove (`atomic_jsonl_remove_by_key`, `:222-246`)

- Reconstructs the exact writer prefix
  `{"transformation_key":"<escaped-key>",` using the **same** `jsonl_json_escape`,
  including the trailing comma so `"foo"` can't false-match `"foobar"` (`:236-238`).
- Filters with awk `index($0,p)!=1` (`:243-244`) — keep lines where the prefix
  does **not** start at byte 1; i.e. drop anchored (position-1) matches. Literal
  substring, not regex → Rust `!line.starts_with(&prefix)`.
- Prefix passed via **`ENVIRON`, not `awk -v`** (`:239-242`) because `-v`
  reprocesses backslash escapes and would corrupt `\"`/`\\` in the escaped key.
- Removing all lines yields an **empty file** (not a deleted one).

#### Escaper (`jsonl_json_escape`, `scripts/jsonl-common.sh:21-50`)

- **Backslash-first ordering** (`:23-31`): `\`→`\\` **must** precede the others,
  then `"`→`\"`, LF→`\n`, CR→`\r`, TAB→`\t`, BS→`\b`, FF→`\f`.
- Any remaining control char (`[[:cntrl:]]`) → `\u%04x` lowercase, 4 digits
  (`:32-48`). Two byte-fidelity gotchas: **vertical tab `0x0B` → ``** (no
  short escape) and **DEL `0x7F` → ``** (`[[:cntrl:]]` matches DEL). All
  higher-plane UTF-8 is passed through unescaped.
- **`serde_json`'s serializer is not byte-identical** to this: it does **not**
  escape `0x7F`. Since 0180 is a clean Rust→Rust cutover (Open Question resolved),
  either a hand-ported escaper or `serde_json` is acceptable *provided the same
  escaper drives both compose and remove* — but AC-7's golden test will pin
  whichever is chosen. If byte-parity with any surviving bash-written record ever
  mattered, only the hand-port matches.

#### Canonical compose order (`jsonl_compose_record`, `:66-149`)

Emitted order (`:129-148`): `transformation_key` (escaped, quoted) →
`schema_version` (**raw/unquoted — a bare JSON number, not escaped or
validated**) → `outcome` (quoted, from the 3-value enum) → `proposed_value`
(escaped, quoted, **always present**) → `user_value` (escaped, quoted, **emitted
iff the `user_value=` pair was supplied**) → `timestamp` (escaped, quoted) →
author extras in declaration order (raw key, escaped value) → `}`. No trailing
newline (the appender adds it).

**Two validation nuances confirmed:**
- `proposed_value` is documented required at `jsonl-common.sh:60` but **omitted
  from the emptiness check at `:115-119`** — bash emits `"proposed_value":""`
  happily. 0180 AC-8 correctly ports to the *documented* spec (reject empty/absent).
- `user_value` is emitted **based on presence**, *not* coupled to
  `outcome=edited`, despite the doc string. **AC-7 as written says "user_value
  (only when outcome=edited)"** — see Open Questions; this is a real ambiguity to
  resolve before implementing the golden test.

### 2. In-repo atomic-write precedents

Three independent implementations exist; the two `cli/` ones are the nearest
precedent, the server one is the fuller-durability reference.

- **`cli/config-adapters/src/store.rs:58-80`** — sync, hand-rolled. Temp in
  `.accelerator/tmp/` named `config-{pid}-{counter}.tmp` where the counter is a
  process-global `static WRITE_COUNTER: AtomicU64` (`:14`); `fs::write` +
  `fs::rename`; **best-effort `remove_file` cleanup on both the write and rename
  failure paths**; **no fsync**; EXDEV avoided structurally (temp and target share
  `.accelerator/`). Test `a_successful_write_leaves_no_stray_temp` (`:335-346`).
- **`cli/launcher/src/launch/outbound/resolve/cache.rs:112-127`**
  (`write_then_rename`) — same shape, temp in the cache root named
  `.tmp-{stem}-{pid}-{seq}` (`static TEMP_SEQ: AtomicU64`, `:13-14`); writes
  `0600` then bumps to `0755`; cleanup only on the rename path; **no fsync**; the
  module header (`:4-6`) documents the ETXTBSY-avoidance rationale (fresh inode
  renamed over the old name).
- **`skills/visualisation/visualise/server/src/file_driver.rs:187-262`** — the
  `tempfile::NamedTempFile::new_in(&parent)` reference. Writes,
  `tmp.as_file().sync_all()` (file fsync), preserves perms, then
  `tmp.persist(&target)` with **explicit `libc::EXDEV` → `CrossFilesystem`
  fail-closed** (`:231-242`), then fsyncs the parent dir (`:244-253`).
  `persist`'s dropped temp auto-cleans on any error. **Out of scope** for a sync
  port: the async/tokio layering, the per-path `tokio::Mutex`, and the SHA-256
  etag optimistic concurrency.

**`tempfile` dependency status**: declared **only** in
`skills/visualisation/visualise/server/Cargo.toml:39` (`tempfile = "3"`). It is
**absent from the entire `cli/` workspace and from `cli/Cargo.lock`.** The two
`cli/` precedents deliberately hand-roll temp naming and, in tests, build temp
dirs from `std::env::temp_dir()` + PID + a local `AtomicU64`. So 0180's chosen
`tempfile::NamedTempFile::new_in` path follows the *server* precedent and
diverges from the *nearest* (cli) precedents — the work item's Drafting Notes
acknowledge this; it is a real, deliberate choice, not an oversight.

**fsync note**: bash `atomic_write` and both cli precedents do **no fsync**. A
byte-faithful port omits it; only the server fsyncs. Not gated by any 0180 AC.

### 3. Port-trait and error-taxonomy precedents

- **Fallible driven ports already exist** — `config`'s
  `ReadConfigLevel::read -> Result<Option<Node>, ConfigError>` and
  `WriteConfigLevel::write -> Result<(), ConfigError>`
  (`cli/config/src/service.rs:29-45`). This refutes the work item's "first
  fallible port" premise and is the closest analog: a fallible reader+writer pair
  behind a document-persisting service.
- **Launcher ports** (`cli/launcher/src/launch/core.rs:173-189`): `ResolveBinary`
  (fallible) + `ExecBinary`; composer `run_external` takes ports by `&impl`
  (static dispatch); rich→kernel mapping is
  `impl From<ResolutionError> for kernel::Error { … Self::Failed(e.to_string()) }`
  (`:167-171`).
- **VCS ports** (`cli/vcs/src/lib.rs:41-78`): `RepoRoot`/`VcsProbe` — infallible
  (`Option`), composed by `facts(start, &dyn RepoRoot, &dyn VcsProbe)` over `&dyn`
  trait objects. Good for the *fact-composer shape* only; both `&impl` and `&dyn`
  composer styles coexist in the workspace.
- **Kernel error** (`cli/kernel/src/lib.rs:9-15`): only two variants,
  `LogFilter(#[from] …)` and `Failed(String)`. **thiserror is used _only_ here.**
  Every subdomain keeps its own rich enum and collapses to
  `Error::Failed(self.to_string())` at the dispatch boundary.
- **Error-enum convention** (hand-written, not thiserror): `#[derive(Debug,
  Clone, PartialEq, Eq)]` (often `#[non_exhaustive]`) + manual `impl Display` +
  bare `impl std::error::Error {}` + optional `From<…> for kernel::Error`.
  Examples: `ConfigError` (`cli/config/src/error.rs:35-108`, with `From`),
  `ResolutionError` (`core.rs:37-171`, with `From`), `PatchError`
  (`cli/corpus-adapters/src/patcher.rs:12-39`, **no** `From` — never crosses the
  kernel boundary), `DocumentError` (`cli/document/src/error.rs:7-42`). `PartialEq,
  Eq` enable exact-value assertions in tests.
- **Test doubles**: canned-`Ok` struct, canned-`Err` struct (or a state enum with
  a `Failing` case as in `FakeReader`, `service.rs:266-281`), and a recording
  double via `RefCell`/`Rc<RefCell<…>>` (`RecordingExec`, `core.rs:287-301`;
  `FakeWriter`, `service.rs:283-297`).

### 4. Target crates — `corpus` (domain) and `corpus-adapters` (shell)

- **`cli/corpus/Cargo.toml`** — `[dependencies]` is **empty** (`:17`). Doc comment
  (`:12-16`): all infra "arrives by injection through ports declared here"; pup
  additionally permits `kernel::Error` "for when a fallible convention needs the
  shared diagnostic; nothing does yet."
- **`cli/corpus-adapters/Cargo.toml`** — deps `corpus`, `document`, `vcs`,
  `vcs-adapters` (path) + `regex`, `time` (`workspace = true`); `[features]
  bash-parity = []` (`:12-18`). Note the **dual dep on domain + adapter** crates
  (`vcs` + `vcs-adapters`) — the pattern if the store port's types live in a
  separate domain crate.
- **Existing corpus ports are infallible**: `Clock`
  (`corpus/src/metadata.rs:13-16`, returns `String`) and `IdScanner`
  (`corpus/src/work_item_id.rs:15-17`, returns `Option<IdScan>`). Notably
  `SystemClock` (the adapter impl) pushes fallibility into **construction**
  (`SystemClock::try_new() -> Result<_, ClockError>`) while keeping the **trait
  methods infallible** — a pattern worth weighing against a fallible port.
- **Persistence seam** — the pure byte-transform half already exists:
  `patch_status(raw: &[u8], new: &str) -> Result<Vec<u8>, PatchError>`
  (`corpus-adapters/src/patcher.rs:48-94`) and `assemble(…) ->
  Option<AssembledDocument>` (`assemble.rs:31-60`) do **no I/O** and use **local
  error enums** (not `kernel::Error`). `atomic_write` is the missing persistence
  half wired downstream of these.
- **Module layout**: `corpus/src/` = `value`, `doc_type`, `linkage`, `slug`,
  `typed_ref`, `metadata`, `work_item_id` (flat `pub use` re-exports).
  `corpus-adapters/src/` = `assemble`, `doc_type`, `document`, `metadata`,
  `patcher`, `scanner` — one module per port/convention.
- **Tests** (`corpus-adapters/tests/`): `metadata.rs` (deterministic fake-port
  tests, item-level `bash-parity` gates), `doc_type_single_source.rs` +
  `parity.rs` (whole-file `#![cfg(feature = "bash-parity")]` differentials),
  `common/mod.rs` (rigging; `require_script` asserts the exec bit).
  **`FakeClock`** (`tests/metadata.rs:27-45`) and inline **`DigitRunScanner`**
  (`corpus/src/work_item_id.rs:141-153`) are the canonical port doubles — unit
  structs with constant/hand-rolled returns, injected as `&dyn`. The `bash-parity`
  gate exists because Rust's harness has **no skip primitive**: a silently-skipped
  differential would read as PASS, so absent-tool differentials are compiled out
  (feature off) or hard-fail (feature on, via `require_script`/`require_file`).

### 5. Toolchain coupling — cargo-deny, cargo-pup, workspace deps

- **`cli/deny.toml` is a denylist** (`[bans].deny`, `:55-69`) — no per-crate
  allow-list. `native-tls`/`openssl*`/`serde-saphyr` are banned; `tempfile`,
  `libc`, `rand` are **not**. Licenses `MIT OR Apache-2.0` already in the license
  allow-list (`:41-52`). `multiple-versions = "warn"` (won't fail),
  `wildcards = "deny"` (**must pin concrete versions**). The
  `{ crate = "serde-saphyr", wrappers = ["document"] }` entry is the pattern *if*
  the team wants to confine `tempfile`/`libc` to `corpus-adapters` via
  `wrappers = ["corpus-adapters"]` — optional, not required.
- **Lock status**: `libc` 0.2.186 and `rand` 0.9.4 are **already present
  transitively** (via ring/tokio/rustls and hickory-dns respectively), so a direct
  dep reuses the same line with no `multiple-versions` warn. **`tempfile` is
  entirely absent** and is the only one that grows the graph.
- **`cli/Cargo.toml`** — `[workspace.dependencies]` (`:13-45`) does **not** list
  `rand`/`libc`/`tempfile`; add pinned entries there if sharing across crates,
  referenced via `{ workspace = true }`.
- **`cli/pup.ron`** — `corpus_domain_imports_only_permitted` (`:57-72`) restricts
  every `use` in the `corpus` crate to `^(std|core|alloc)(::|$)`,
  `^kernel::Error(::|$)`, `^crate(::|$)`. **Consequence**: the store port trait,
  if declared in `corpus`, cannot name `tempfile`/`libc`/`rand` types in its
  signature, and a rich `StoreError` it returns must be `crate::StoreError`
  (declared in `corpus`) or `kernel::Error`. **There is no pup rule for
  `corpus-adapters`** (nor the other `-adapters` crates) — the concrete impl that
  pulls in the three deps belongs there, unconstrained.

## Code References

- `scripts/atomic-common.sh:16-32` — `atomic_write` (same-dir temp + rename + EXIT trap)
- `scripts/atomic-common.sh:105-143` — `_atomic_lock_acquire` (pre-check, reseed, back-off, ceiling, owner write)
- `scripts/atomic-common.sh:157-163` — `_atomic_lock_reclaim_if_stale` (missing/empty owner = live; `kill -0`; `rm -rf`)
- `scripts/atomic-common.sh:200-209`, `:233-246` — subshell-owned release trap + anchored-prefix remove
- `scripts/jsonl-common.sh:21-50` — `jsonl_json_escape` (backslash-first; `\u00xx`; 0x0B/0x7F)
- `scripts/jsonl-common.sh:66-149` — `jsonl_compose_record` (canonical order; `proposed_value`/`user_value` nuances)
- `cli/config-adapters/src/store.rs:14,58-80` — hand-rolled `atomic_write`, `AtomicU64` counter
- `cli/launcher/src/launch/outbound/resolve/cache.rs:4-6,112-127` — `write_then_rename` + ETXTBSY note
- `skills/visualisation/visualise/server/src/file_driver.rs:187-262` — `NamedTempFile::new_in` + `persist` + EXDEV
- `cli/config/src/service.rs:29-45,255-297` — fallible reader/writer ports + fallible fakes
- `cli/launcher/src/launch/core.rs:167-189,261-301` — port traits, `From` mapping, `RecordingExec`
- `cli/kernel/src/lib.rs:9-15` — kernel `Error` taxonomy (only place thiserror is used)
- `cli/config/src/error.rs:35-108` — the hand-written error-enum convention (`Display` + bare `Error` + `From`)
- `cli/corpus-adapters/src/patcher.rs:12-94` — `PatchError` (local enum) + pure `patch_status` byte-transform
- `cli/corpus-adapters/src/assemble.rs:31-60` — pure `assemble` transform (no I/O)
- `cli/corpus/src/metadata.rs:13-16`, `cli/corpus/src/work_item_id.rs:15-17,141-153` — infallible ports + inline fake
- `cli/corpus-adapters/tests/metadata.rs:27-45` — `FakeClock` port double
- `cli/deny.toml:41-69` — license allow-list + bans denylist
- `cli/Cargo.toml:4,13-45` — workspace members + `[workspace.dependencies]`
- `cli/pup.ron:57-72` — `corpus_domain_imports_only_permitted`

## Architecture Insights

- **Ports in the domain, infra in the adapter, injected as `&dyn`.** Every corpus
  port is a trait in `corpus` taken as the last `&dyn` argument by a pure
  function; the concrete impl lives in `corpus-adapters`. The atomic-store port
  follows this exactly. The pup rule *enforces* the purity of the domain crate.
- **Rich errors collapse at the kernel boundary.** thiserror is kernel-only;
  every subdomain hand-writes `Display` + bare `Error` and (if it crosses the
  boundary) a `From<…> for kernel::Error`. A `StoreError` should follow
  `ConfigError`'s shape and, because the store is a leaf infra concern, likely
  add the `From` (unlike `PatchError`, which never crosses).
- **RAII replaces the subshell trap.** The bash "acquire-then-arm EXIT trap in a
  subshell" maps to a lock guard whose `Drop` releases; construction-on-success
  preserves the "never release a lock you didn't hold" invariant for free.
- **Fallibility can live in construction, not the port.** `SystemClock::try_new`
  shows the local idiom of a fallible constructor behind infallible trait methods
  — a genuine alternative to a `Result`-returning port, worth weighing for the
  writability pre-check / lock-timeout surface (though those failures are
  per-call, which pushes toward a fallible method like `config`'s writer).
- **The differential-test posture is deliberate.** `bash-parity` compiles
  differentials out by default and hard-fails when the feature is on but the tool
  is absent — because Rust's harness has no skip. 0180's Rust→Rust round-trip
  (AC-6) is *not* a bash differential and needs no gate; only if a bash oracle
  were ever added would it go behind `bash-parity`.

## Historical Context

- `meta/work/0166-shared-config-corpus-store-crates.md` — parent epic; locked
  "fold atomic-store into `corpus-adapters`, no standalone `store` crate."
- `meta/work/0179-corpus-crates-parsing-conventions.md` +
  `meta/plans/2026-07-11-0179-corpus-crates-parsing-conventions.md` — the blocker
  that created `corpus`/`corpus-adapters`; the nearest crate-design precedent.
- `meta/work/0178-config-crates-native-yaml-reader.md` +
  `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md` — the
  config/config-adapters pattern 0180 mirrors (and the fallible-port precedent).
- `meta/work/0172-migration-engine-subdomain.md` — the clean-cutover consumer;
  0180's byte-parity scope-out depends on 0172 never interleaving a bash and Rust
  writer against one session-log file.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  and `ADR-0045-skills-vs-cli-division-of-labour.md` — the two ADRs 0180 cites;
  `ADR-0052-filesystem-as-message-bus-and-knowledge-corpus.md` is the companion
  rationale for a filesystem-backed store.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  and `2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md` — the
  migration architecture + the shell-script inventory that includes these
  primitives.
- `meta/research/codebase/2026-06-29-0162-rust-toolchain-guard-rails-wiring.md` —
  the cargo-deny / cargo-pup wiring background.
- `meta/reviews/work/0180-atomic-store-primitives-corpus-adapters-review-1.md` —
  the existing review of this work item.

## Related Research

- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md` —
  canonical field order + parsing conventions in the corpus crates (direct
  predecessor).
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md` —
  the config adapter pattern and its fallible reader/writer ports.

## Open Questions

1. **Where does `StoreError` live?** Forced by pup: if the port trait is in
   `corpus` and fallible, `StoreError` must be `crate::StoreError` (in `corpus`)
   or `kernel::Error` — it cannot live in `corpus-adapters` and be named by the
   trait. Options: (a) declare `StoreError` in `corpus` (first rich error in the
   domain crate; diverges from the `PatchError`-in-adapter precedent); (b) return
   `kernel::Error` directly (loses the rich lock-timeout/not-writable taxonomy
   the notes want); (c) keep the *port* infallible and push failure into a
   fallible constructor à la `SystemClock::try_new` (works for setup errors, but
   lock-timeout is per-call, so ill-fitting). Recommend (a), mirroring
   `ConfigError` but sited in `corpus`.

2. **`user_value` gating — presence vs `outcome=edited`.** Bash emits `user_value`
   iff the pair was supplied (decoupled from `outcome`), but AC-7's canonical
   order says "user_value (only when outcome=edited)." These disagree. Since 0180
   already ports `proposed_value` to the *documented* spec (AC-8), consistency
   argues for gating `user_value` on `outcome=edited` too — but that must be an
   explicit decision, because the golden test (AC-7) and the compose API shape
   both depend on it.

3. **Which escaper?** Clean cutover permits any self-consistent escaper, but
   `serde_json` diverges from the bash bytes at `0x7F` (and the port must ensure
   the *same* escaper drives compose and remove). A hand-ported `jsonl_json_escape`
   is the low-risk choice and keeps the door open to any future bash byte-parity;
   `serde_json` is acceptable only because AC-6/AC-7 are Rust→Rust. Decide and pin
   in AC-7's golden record.

4. **Fallible port vs mirroring config's writer signature.** Given
   `WriteConfigLevel::write -> Result<(), ConfigError>` already exists, should the
   store port be a single fallible `write`/`append`/`remove_by_key` trait
   returning `Result<_, StoreError>` (mirroring config) rather than the
   `AtomicStore`/`RecordStore` split the notes sketch from `vcs`? The config pair
   is the stronger precedent.

5. **Confine `tempfile`/`libc` via `wrappers`?** Optional, but the
   `serde-saphyr`/`document` deny entry shows the mechanism
   (`wrappers = ["corpus-adapters"]`) if the team wants to keep these infra deps
   out of every other crate's reachable graph. Not required to compile.

## Follow-up Research 2026-07-19T00:55:00+00:00

Resolves the `libc`/musl concern raised in review and grounds open questions 2
and 4 with caller/reader/call-surface evidence. Q1 (`StoreError` location) is
decided: **it lives in `corpus`**, mirroring `ConfigError`'s hand-written shape.

### `libc` under static-musl — no problem

- `libc` is the **musl-bindings crate**, not a glibc dependency. `libc::kill`,
  `libc::pid_t`, `libc::EXDEV` bind to whichever libc is linked; under a
  `*-unknown-linux-musl` target they bind to the statically-linked musl. The
  crate's build script emits musl cfgs (`cfg(musl_v1_2_3)`, `cfg(musl32_time64)`,
  `cfg(musl_redir_time64)`) — it is the canonical musl FFI crate.
- It is **already compiled and linked into every CLI target** — a
  `liblibc-*.rlib` exists in each release deps dir, pulled transitively by
  getrandom/ring/tokio. A direct dep for `kill(pid, 0)` adds no crate to the
  graph or the link.
- `kill(2)` and `EXDEV` are POSIX and present in musl; the visualiser server
  already uses `libc::EXDEV` on the same rename pattern
  (`server/src/file_driver.rs:231-242`).
- Only remaining constraint (shared with `tempfile`): pin to a concrete version
  because `deny.toml` has `wildcards = "deny"`. `tempfile` is the only genuinely
  new crate; it is musl-clean (fastrand/rustix, no C deps). `deny.toml` targets
  confirmed at `cli/deny.toml:11-17` (both musl triples present).

### Q2 — `user_value` gating: keep the primitive presence-based (bash-faithful)

Evidence (`skills/config/migrate/scripts/interactive-lib.sh`):

- The production invariant "`user_value` present ⟺ `outcome=edited`" is enforced
  in the **caller** `write_session_record` (`:180-182` gates
  `compose_args+=("user_value=$user_value")` on `outcome=edited`), **not** in
  `jsonl_compose_record`, which is purely presence-based (`jsonl-common.sh:90-93`,
  `:138-140`).
- The sole reader `build_resume_state_file` (`:40-126`) is **name-keyed and
  absence-tolerant**: `extract_field($0,"user_value")` returns `""` when absent
  (`:85`) and is emitted into every `RESUMED` row regardless of outcome
  (`:114-123`). No reader depends on the present-iff-edited coupling.
- The visualiser does **not** read this JSONL at all (0-match search of
  `skills/visualisation/visualise/{server,frontend}/src` for `user_value`,
  `transformation_key`, `proposed_value`, `.jsonl`). Its "canonical field order"
  coupling to 0180 is a stated contract obligation, not a runtime consumer.

**Decision**: the Rust composer stays presence-based (emit `user_value` iff
supplied); the outcome-coupling is the consumer's job (0172), as today. This is a
principled asymmetry with AC-8: enforce **field-level validity** (`proposed_value`
non-empty) in the primitive, but leave **cross-field policy** (which outcomes
carry a user value) to the migrate domain. **AC-7 should be reworded** from
"`user_value` (only when `outcome=edited`)" to "only when a `user_value` is
supplied", or the golden test encodes a rule the primitive deliberately omits.

### Q4 — port shape: split `AtomicWrite` + `RecordStore`, lock stays private

Call-surface counts (production, excluding tests):

| Concern | Consumers | Call sites |
|---|---|---|
| `atomic_write` (raw bytes) | ~16 scripts | ~26 |
| JSONL record ops (compose/append/remove-by-key) | 1 (`interactive-lib.sh`) | 3 |
| explicit `_atomic_lock_acquire` | 0 | 0 |

`atomic_write` consumers: `config-common.sh:222,309`; migrations
`0002/0003/0005/0006/0007-*.sh`; `run-migrations.sh:572-573`;
`work-item-sync-apply.sh:168`, `work-item-sync-baseline.sh:120,135,146`; and the
`jira_atomic_write_json`/`linear_atomic_write_json` wrappers
(`jira-common.sh:98-113`, `linear-common.sh:97-111`) fanning out to 6 JSON-cache
sites. The JSONL ops have exactly one production consumer (the migrate session
log, `interactive-lib.sh:187-188,907`). The lock is never surfaced to any
consumer.

**Decision**: split into two ports, mirroring the ratio —
- `AtomicWrite` — `write(path, &[u8]) -> Result<(), StoreError>`; the
  broadly-reused primitive and the home for AC-2's same-dir-temp + single-rename
  semantics. 0170's work-item-sync can depend on this alone.
- `RecordStore` — `append_record` / `remove_by_key`; single-consumer (0172),
  composes `AtomicWrite` + the (private) lock internally. The lock is **not** a
  public port — the 0-external-callers finding confirms it.

Caveat: no Rust consumer exists yet (they land in 0170/0172/0168), so declaring
both traits now is mildly speculative — but the ratio argues against merging, and
keeping `AtomicWrite` free of record concerns is the cheaper default.

### Q3 — escaper: decided → `serde_json`

`serde_json = "1"` is already a workspace dep (`cli/Cargo.toml:39`), so either
choice is dependency-neutral. The two escapers diverge only at `0x7F` (DEL:
bash → ``, serde_json → raw byte). Both satisfy AC-6 (same escaper drives
compose+remove → self-consistent), and raw `0x7F` is legal unescaped JSON so it
round-trips fine.

**Decision: use `serde_json`'s string serializer as the single escaper.**
Implementation guidance so the choice doesn't leak into the record *shape*:

- **Use `serde_json` only for per-value string escaping**, not to serialize the
  whole record. Idiom: `serde_json::to_string(&s)` yields a quoted, escaped
  string — strip the surrounding quotes for both the emitted field value and the
  remove-by-key prefix `{"transformation_key":"<esc>",`. Both paths must go
  through this same call (the parity requirement).
- **Keep record composition manual** (field-by-field, mirroring
  `jsonl_compose_record`), *not* a `#[derive(Serialize)]` struct — this keeps
  three things under direct control: `schema_version` as a **bare JSON number**;
  `user_value` **presence-based** emission (per Q2); and **extras in declaration
  order**. The last is the trap: serde's `#[serde(flatten)]` over a map preserves
  insertion order only with `serde_json`'s non-default `preserve_order` feature
  (indexmap); without it `serde_json::Map` is a `BTreeMap` and sorts extras
  alphabetically, silently breaking AC-7. Manual composition sidesteps this.
- **AC-7's golden record** must be written to match `serde_json`'s output
  (notably `0x7F` stays raw); AC-8's `proposed_value`-required check runs before
  composition regardless of escaper.
