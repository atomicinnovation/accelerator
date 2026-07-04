---
type: plan
id: "2026-07-03-0164-launcher-and-git-style-dispatch"
title: "Launcher and Git-Style Dispatch Implementation Plan"
date: "2026-07-03T18:56:45+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0164"
parent: "work-item:0164"
derived_from: ["codebase-research:2026-07-03-0164-launcher-and-git-style-dispatch"]
relates_to: ["work-item:0165", "work-item:0167", "work-item:0169"]
tags: [rust, launcher, dispatch, cli, fetch-verify-cache-exec, minisign, reqwest, bootstrap]
revision: "db6fa85e195bdcb2e17e7f7f9bc7449341bfaca7"
repository: "accelerator"
last_updated: "2026-07-04T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Launcher and Git-Style Dispatch Implementation Plan

## Overview

Turn the 0163 `version`-only scaffold into a working `accelerator` launcher:
route unknown subcommands through clap `external_subcommand`, resolve absent
sub-binaries on demand (fetch → verify sha256 + minisign → atomic cache under
`${CLAUDE_PLUGIN_ROOT}` → Unix `exec`), reuse the cache on repeat while
re-verifying before every exec, synthesise manifest-driven discoverable help,
and front the whole thing with a thin bash bootstrap that fetches the launcher
itself on first use and verifies it against a plugin-committed key via a
vendored minisign shim.

The launcher is proven end-to-end against **test fixtures** — a fixture
manifest, a fixture keypair, and an arg-driven fixture binary — so 0164 lands
and closes independently of 0165 (which supplies the production cross-compile,
signing, and release pipeline).

## Current State Analysis

The `cli/` workspace is a well-prepared skeleton; the load-bearing machinery is
absent and is exactly this story's surface.

- **Two-crate workspace** (`cli/Cargo.toml`): `resolver = "2"`, members
  `["launcher", "kernel"]`, shared `version = "1.24.0-pre.2"`. The binary is
  `accelerator` (`cli/launcher/Cargo.toml:12-14`) though the crate is
  `launcher`.
- **Dispatch is single-armed** — `Command` has only `Version`
  (`cli/launcher/src/version/inbound/cli.rs:17-21`); there is no
  `external_subcommand`. `tests/version.rs` asserts an unknown subcommand exits
  non-zero with `"unrecognized subcommand"` — behaviour this story changes.
- **`kernel::Error` already has one variant** (`LogFilter`,
  `cli/kernel/src/lib.rs:6-10`) and `main.rs:19-28` already renders any error
  generically via `eprintln!("{error}")` + `ExitCode::FAILURE`. Unlike
  luminosity there is no `match error {}` to break — adding variants is clean.
- **The dependency pool lacks everything network/verify/serde**: no reqwest,
  rustls, minisign-verify, sha2, hex, serde, serde_json
  (`cli/Cargo.toml:11-21`).
- **Supply-chain policy is pre-armed**: `cli/deny.toml:42-53` hard-bans
  `native-tls`/`openssl`/`openssl-sys` and evaluates the graph across the four
  release triples + linux-gnu; the licence allow-list (`:35-39`) is pruned to
  exactly the current closure and *warns on unused allowances*, so the
  rustls/HTTP/signature stack must add ISC/BSD/Zlib as those crates enter.
- **Layering enforcement is per-core**: `cli/pup.ron` constrains only
  `^launcher::version::core($|::)`; a new hexagon core needs its own analogous
  rule or it is unconstrained.
- **The `version` hexagon** (`cli/launcher/src/version/{core,inbound,outbound}`)
  is the copy-me pattern: an outbound port trait, a value object, an inbound
  port trait, an application service with `const fn new`, and hand-written
  in-memory fakes — no mocking framework.
- **The launcher is the only production TLS stack in the repo** (the visualiser
  server has no production TLS). `default-features = false` on reqwest is
  non-negotiable against the openssl ban.

## Desired End State

`mise run` is green end-to-end and:

- `accelerator version` still works as a built-in; `accelerator config`-shaped
  built-ins stay compiled in. Every other subcommand routes through
  `External(Vec<OsString>)`.
- Given an absent sub-binary, the launcher fetches the host-target asset,
  verifies sha256 **and** minisign in-process, atomically caches it under
  `${CLAUDE_PLUGIN_ROOT}` keyed by name+version+checksum, and `exec`s it.
- Given a cached, verified sub-binary, a repeat invocation reuses the cache with
  **zero** fetches (asserted by request count) and re-verifies the signature
  before exec.
- Given any sha256 or minisign failure — including a cached binary mutated on
  disk — the launcher refuses to exec, exits non-zero naming the failed check
  and sub-binary, and leaves no partial/temp entry while preserving any
  pre-existing verified entry.
- Exit codes and signals propagate (exec-based, process-replacing).
- Non-UTF-8 forwarded args survive verbatim.
- `ACCELERATOR_<SUB>_BIN` execs a named local binary with no fetch (offline
  escape hatch).
- `accelerator <unknown>` and `accelerator --help` render the manifest-driven
  external-subcommands listing; `accelerator <sub> --help` delegates to the
  child.
- The host-target launcher build is rustls-only with no dynamic OpenSSL
  (`otool -L`/`ldd`).
- `bin/accelerator` (bash 3.2) fetches the launcher on first use, verifies it
  against the plugin-committed key via a vendored minisign shim, fail-closed,
  and execs it; it passes `scripts/lint-bashisms.sh` and shfmt/ShellCheck.

### Key Discoveries

- **The rustls trap** — `deny.toml` evaluates the shipped feature set, so
  reqwest **must** be `default-features = false` with rustls features or its
  default `native-tls` feature trips the ban (`cli/deny.toml:49-53`). Single
  most important declaration constraint.
- **Restriction lints bite new code** — `unwrap_used`/`expect_used`/`panic`/
  `todo` are warn-level, promoted to hard errors via `warnings = deny` /
  `-D warnings` (`cli/Cargo.toml:23-35`). All HTTP/IO/verify code
  propagates `Result` through `kernel::Error`; the crypto-provider install is a
  fallible mapped call, never `.unwrap()`.
- **`exec` replaces the process** — exit/signal-propagation tests must be
  black-box, spawning `CARGO_BIN_EXE_accelerator` as a child (the idiom
  `tests/version.rs:23` uses), never in-process.
- **`CARGO_BIN_EXE_<name>` is set only for bins in the package under test** — so
  the arg-driven fixture must be a `[[bin]]` *inside the launcher crate*, not a
  separate crate.
- **cargo-pup constrains only `version::core`** — dispatch/launch glue must live
  in a dedicated launcher-level module (not under `version::core`) and the new
  resolution core needs its own inward-import rule.
- **`${CLAUDE_PLUGIN_ROOT}` is version-scoped** — a new plugin version yields a
  fresh cache and redownloads, which is correct (binaries are version-coherent
  with the plugin). The bare-path invocation contract matches `allowed-tools`
  globs against plugin-root paths, so the cache must live there for the
  permission match to hold.

### Decisions settled for this plan

Confirmed interactively; all luminosity-aligned unless noted:

1. **Root of trust** → vendored `cli/verify/` minisign shim; the bootstrap
   verifies the fetched launcher's `.minisig` against the plugin-committed key
   before exec, fail-closed. A binary cannot verify itself, so admitting the
   launcher on TLS+sha256 alone would collapse the model to "served over TLS"
   (rejected by ADR-0046). The shim is unsigned by construction (a root of trust
   cannot be verified by what it roots); its integrity therefore rests on the
   **plugin package's own distribution integrity** (the marketplace/plugin
   channel), stated here as an explicit 0164 assumption, and on 0165's
   reproducible-build byte-identity. The bootstrap runs the shim only from a path
   it controls, by absolute path.
2. **Cache location** → `${CLAUDE_PLUGIN_ROOT}` only, **probed** for
   writable+exec-capable; an `ACCELERATOR_CACHE_DIR` override gives an explicit
   alternate terminus; an unset `CLAUDE_PLUGIN_ROOT`, a read-only/noexec root
   with no override, or nothing resolvable is a **named error**. **Deviation from
   luminosity:** no XDG fallback — the bare-path invocation contract matches
   `allowed-tools` globs against `${CLAUDE_PLUGIN_ROOT}` paths, so an
   XDG-resident binary would break the permission match that motivates the
   plugin-root location (0136 research, resolved constraint). Because
   `${CLAUDE_PLUGIN_ROOT}` is version-scoped, a new plugin version yields a fresh
   cache and old-version caches are removed with the old plugin install, so the
   cache is naturally bounded — **no retained-versions cap and no oldest-by-mtime
   eviction are needed** (both are dropped relative to luminosity).
3. **MSRV / resolver** → bump the workspace to `resolver = "3"`, pin
   `rust-version = "1.90.0"` on **every** workspace member (launcher, kernel, and
   the verify shim) so all share one MSRV floor, add a pinned-MSRV CI leg, and
   **re-run `deny:check` after the bump** — MSRV-aware selection under resolver 3
   can pick older dependency versions, which could perturb the exact
   `vergen`/`vergen-gitcl` pins and the pruned licence allow-list.
4. **Manifest model** → a new signed, name-keyed `manifest.json`
   (per-platform sha256 + signature, per-binary `description`), itself
   minisign-signed (`manifest.minisig`) and verified before any field is
   trusted, with a version-equality anti-rollback check and a `schema_version`
   gate. This **supersedes** the existing `checksums.json` as the distribution
   contract, but the cutover is a **monotonic, tested migration, not a flag-day**:
   0164 adds `manifest.json` (fixtures) and **keeps `checksums.json` in
   `validate_version_coherence` until 0165 physically retires it** (0165 owns
   emission + retirement); during the overlap a coherence test asserts both
   artifacts (while both exist) agree with `plugin.json`. 0164 owns adding
   `manifest.json`'s `version` to the coherence set; 0165 owns removing
   `checksums.json`'s. The exact **sentinel byte-form** (all-zeros = "no binary for
   this version") is pinned in the shared contract artifact (§6) as **bare 64-char
   zero hex**, and the reader treats both a bare and a `sha256:`-prefixed
   all-zeros digest as the sentinel named error (matching the strip-if-present
   tolerance). Fixtures exercise the full model; 0165 emits the production version.

   **Version-source coherence (resolves an existing drift):** the anti-rollback
   check compares `manifest.version` for exact equality against the launcher's
   compiled `CARGO_PKG_VERSION`. Today `cli/Cargo.toml` (`1.24.0-pre.2`) is out of
   step with `plugin.json`/`checksums.json` (`1.24.0-pre.7`), so in production the
   emitted `manifest.version` (plugin-derived) would never equal the launcher's
   `CARGO_PKG_VERSION` (cli-workspace-derived) and the exact-equality check would
   refuse **every** legitimately-signed manifest while the fixture (which derives
   its version from `CARGO_PKG_VERSION`) passes. So this story **adds the cli
   workspace to `validate_version_coherence` at the shared plugin version**,
   resolving the `pre.2`/`pre.7` drift, and the cross-language contract test
   asserts `manifest.version == launcher CARGO_PKG_VERSION` against the **real
   workspace version**, not only the fixture.
5. **Fixture-key handoff — enforced, not documentary** → the launcher embeds the
   *fixture* pubkey via `include_str!`, but behind a **test-only cargo feature**
   (not `#[cfg(test)]`, which would not apply to the `CARGO_BIN_EXE_accelerator`
   binary that integration tests spawn) so the fixture key is compiled in for
   dev/test builds and **cannot compile into a release build**. A **release-time
   assertion in 0164** (not deferred to 0165) checks that the embedded
   verify-any-of set contains no fixture/test key. The set follows a **two-key
   rotation discipline** (at most current + previous; the previous key is dropped
   on the next release; revocation of a compromised key is a forced
   plugin-version bump, since there is no runtime revocation list). Swapping in
   0165's production key with a rotation overlap is the concrete deliverable of
   the 0164→0165 handoff.
6. **0165 contract as a shared, tested artifact** → the `manifest.json` schema
   (field names, types, `schema_version` semantics, signature encoding), the
   asset-URL/filename template, and the embedded pubkey are pinned in a **single
   committed contract artifact** (a JSON schema + a golden fixture manifest) that
   both 0164 and 0165 consume and test against, rather than a prose example in
   either plan. This turns cross-story drift into a failing shared test rather
   than a first-production-release failure. The research names this the single
   highest-risk coordination item.

## What We're NOT Doing

- **Not the distribution/signing pipeline** — four-triple cross-compile,
  minisign *signing* tasks, `manifest.json` *emission*, CI release jobs, and
  reproducible-build byte-identity of the shim are **0165**. 0164 builds the
  launcher and shim for the **host** target and tests against fixtures.
- **No launcher self-update** (ADR-0054) — a new plugin version drives a new
  launcher via the bootstrap.
- **No `config` implementation** — `config` stays a *reserved* built-in slot
  (0167). This story establishes the built-in/external split point; it does not
  add `config`.
- **No product sub-binaries** — external dispatch is proven against the
  in-crate fixture `[[bin]]`; the visualiser is 0168.
- **No Windows** — four Unix targets; `exec`-only.
- **No in-process Sigstore/SLSA verification** — parked by ADR-0046.
- **No new subdomain crates** — the deny/pup cross-crate ban-lists stay inert.
  The `cli/verify/` shim is **tooling, not a subdomain**, so it stays out of the
  layering rules (confirm the ban-lists ignore it).

## Implementation Approach

Four phases, each independently mergeable and each leaving `mise run` green.
Distribution is out of scope; the only inbound dependency on 0165 is the
manifest/asset/key *contract*, which 0164 pins via fixtures so it can land
first.

TDD throughout: red → green → refactor. The fixture crate is test
infrastructure — scaffolded first, then the failing dispatch/resolution tests
are written against it. Hexagon discipline: resolution is a driven port with a
**fake** adapter (Phase 1) before the **real** fetch/verify/cache adapter
(Phase 2), so dispatch/exec merge without the network stack.

The build-in/external boundary is a standing invariant: `version` and the
reserved `config` slot are compiled in and never fetch; everything else is
external. A test pins it (see Phase 1) so a divergent 0167/0169 decision
surfaces as a failing check rather than silent rework.

---

## Phase 1: Dependency stack, error taxonomy, external dispatch + exec

### Overview

Populate the empty seams: add the HTTP/verification dependency stack, bump the
MSRV machinery, give the launcher a resolution error taxonomy mapped into
`kernel::Error`, add the `External(Vec<OsString>)` arm in a dedicated dispatch
module, and prove dispatch + exec (exit/signal/non-UTF-8 propagation) and the
`ACCELERATOR_<SUB>_BIN` override against a fixture — all behind a **fake**
resolution port, no network.

### Changes Required

#### 1. Dependency stack (the rustls trap)

**File**: `cli/Cargo.toml`, `cli/launcher/Cargo.toml`

Add to `[workspace.dependencies]`, consumed by `launcher` via `workspace = true`
(kernel stays std-only):

```toml
reqwest = { version = "=0.12.<patch>", default-features = false, features = ["rustls-tls-webpki-roots-no-provider", "blocking", "hickory-dns"] }
rustls = { version = "=0.23.<patch>", default-features = false, features = ["ring"] }
minisign-verify = "=0.2.5"
sha2 = "0.10"
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

reqwest and rustls are **exact-pinned** (`=0.12.<patch>` / `=0.23.<patch>`,
`<patch>` resolved and pinned at implementation) — matching the repo's existing
exact-pin practice for behaviour-sensitive deps — because a caret range lets a
future patch silently rename or re-scope the DNS feature back onto `getaddrinfo`.
`default-features = false` on reqwest is mandatory. The TLS root source is
bundled webpki-roots, so the static binary carries its own roots and does not
read the host cert store or pull `rustls-native-certs`/`security-framework`.
`hickory-dns` makes name resolution bypass the host `getaddrinfo`/nsswitch —
the load-bearing musl fix, since a silently-inert feature would fall back to
`getaddrinfo` and fail only on a user's musl machine. Confirm the `hickory-dns`
feature spelling against the pinned reqwest version's `Cargo.toml`, and assert
`cargo tree -e features -p launcher` shows **the hickory-resolver crate present
by crate name** (not the reqwest feature label, so a feature rename is caught
structurally) — added to the deny/feature regression test below alongside the
`ring`-present / `aws-lc-rs`-absent assertions. Note this proves the crate is
*selected* in the graph, not that it links statically or resolves DNS on musl —
the four-triple build (0165) is the sole authority for that.

Crypto provider is `ring`, selected explicitly: reqwest's `-no-provider`
variant plus a direct `rustls` dep with `features = ["ring"]`, installed once at
startup via `CryptoProvider::install_default(...)` as a fallible call mapped to
`kernel::Error` (never `.unwrap()`). This avoids `aws-lc-rs` (C + per-arch
assembly, hostile to the four-triple cross-build in 0165).

The launcher uses **blocking** reqwest, whose runtime is internal — declare
**no direct `tokio` dependency**. Extend `cli/deny.toml`'s licence allow-list
with the licences the new closure carries (ISC/BSD-3-Clause/Zlib as needed) so
`deny:check` passes rather than failing mid-implementation.

#### 2. MSRV / resolver bump

**File**: `cli/Cargo.toml`, `cli/launcher/Cargo.toml`, `.github/workflows/main.yml`

Bump the workspace to `resolver = "3"` (so `rust-version` drives MSRV-aware
version selection; `resolver = "2"` ignores it). Add `rust-version = "1.90.0"`
to `cli/launcher/Cargo.toml`. Add a CI leg that builds/tests the workspace on
the pinned MSRV toolchain so an MSRV-breaking transitive bump fails in CI, not
at a user's first fetch. Register the leg per the CI-required-check convention.

#### 3. Launcher resolution error → `kernel::Error`

**File**: `cli/launcher/src/launch/`, `cli/kernel/src/lib.rs`

Keep `kernel::Error` small and genuinely shared. The rich resolution taxonomy
(fetch/network, checksum mismatch, signature mismatch, asset-not-found,
release-unavailable, IO/cache, exec) lives in a **launcher-local**
`launch::ResolutionError` that maps into **a single** `kernel::Error` variant at
the boundary — `kernel::Error::Launcher(#[from] launch::ResolutionError)` — so
the typed category and payload survive to the composition root while `version`
never compiles against fetch/signature variants (it only ever constructs the
variants it needs). Each launcher-local variant carries the payload its
diagnostic needs: target-triple + URL on fetch/asset-not-found; expected vs
actual sha256 on checksum mismatch; asset name + which check on signature
mismatch; path on IO/cache. `main.rs`'s existing generic `eprintln!` +
`ExitCode::FAILURE` renders the resulting `Display` — no change to the
composition root's error arm is required — but the diagnostics that name the
failed check (sha256 vs minisign) and the sub-binary are backed by a typed
variant, not only a formatted string, so they can be asserted structurally.

#### 4. External-subcommand dispatch in a dedicated module

**File**: `cli/launcher/src/launch/` (new), `cli/launcher/src/version/inbound/cli.rs`, `cli/launcher/src/lib.rs`

Add the arm:

```rust
#[derive(Subcommand)]
pub enum Command {
    Version,
    #[command(external_subcommand)]
    External(Vec<std::ffi::OsString>),
}
```

Move `External` routing, the `ResolveBinary` port, and `exec` into a new
launcher-level `launch` module. `version/inbound/cli.rs` retains only the
`version` command, so no launcher glue sits under `version::core`. The module's
internal split is fixed up front: **`launch::core`** holds the port(s) and pure
logic and is pup-constrained (§5); the **`launch` shell** holds `exec`, the
env-override, and the clap glue — the launcher's imperative shell with an
inward-only dependency direction.

Guard the boundary: an **empty `External` vector** (no subcommand name) is a
**named error, never an index-`[0]` panic** (a panic is promoted to a hard error
by the restriction lints anyway). The routing must also never let a fetched
sub-binary named `version`/`config` shadow a built-in — the built-in set is
checked first (§Implementation Approach invariant).

A parse test drives `["accelerator", "frobnicate", "--flag"]` and asserts it
reaches `External` with `["frobnicate", "--flag"]`; a companion test asserts the
empty-`External` case yields the named error.

The `external_subcommand` capture and the Phase 3 `try_parse` +
`ErrorKind::DisplayHelp` interception ordering are load-bearing, so **pin clap
exactly** (`=4.6.x`, resolved patch) rather than the floor `4.6` — consistent
with the repo's exact-pin practice for behaviour-sensitive deps — so a
`cargo update` cannot silently shift help/dispatch behaviour under the ordering
tests; any bump is then a deliberate, reviewed change.

#### 5. `ResolveBinary` port + fake adapter + `ACCELERATOR_<SUB>_BIN`

**File**: `cli/launcher/src/launch/core.rs`, fake adapter, `cli/pup.ron`

Define the driven port:

```rust
pub trait ResolveBinary {
    fn resolve(&self, name: &OsStr, args: &[OsString]) -> Result<PathBuf, kernel::Error>;
}
```

The `ACCELERATOR_<SUB>_BIN` override is checked **before** any resolution: for
subcommand `<sub>`, if the derived variable names an existing binary, resolution
returns that path verbatim with no fetch and no checksum. This is the
air-gapped/offline escape hatch, honoured by both the fake (Phase 1) and real
(Phase 2) adapters.

The variable name is derived by a **total, documented normalisation**, since
git-style subcommands contain hyphens (`frobnicate-thing`) but env var names
permit only `[A-Za-z0-9_]`: uppercase the subcommand name, then replace every
character outside `[A-Z0-9_]` with `_` (so `frobnicate-thing` →
`ACCELERATOR_FROBNICATE_THING_BIN`). This normalisation lives in **one shared
pure helper in `launch::core`** that both the fake and real adapters call, so
the derivation cannot diverge (the bootstrap fetches only the launcher and never
resolves sub-binaries, so it does not derive these names); a fixture test drives
a **hyphenated** subcommand name and asserts the derived variable resolves.

The normalisation is not injective (`frobnicate-thing` and `frobnicate_thing`
both map to `..._FROBNICATE_THING_BIN`; a leading digit yields a
non-identifier). Rather than silently redirect one sub-binary's override to
another, the override lookup **rejects a subcommand name that would produce a
colliding or non-identifier variable** with a named error, and a fixture test
covers a colliding pair and a leading-digit name. In practice the shipped
sub-binary names are a curated, non-colliding set, so this is a guard against a
future name choice, not a live case.

**Trust boundary of the overrides (documented, accepted):** `ACCELERATOR_<SUB>_BIN`
execs a caller-named binary with no verification, and `ACCELERATOR_CACHE_DIR`
(Phase 2) relocates the cache. These are trusted-as-the-invoking-user by design
— anyone who can set the launcher's environment can already run arbitrary code
as that user. Two invariants bound the blast radius: `ACCELERATOR_CACHE_DIR`
changes only the *location*, never disabling content re-verification of what is
fetched into it; and the launcher logs (at `tracing` info) when either override
is active.

The Phase 1 fake adapter returns a cached fixture path directly. Extend
`cli/pup.ron` with a `launch_core_imports_only_permitted` rule mirroring the
`version::core` rule, so the new core gets the same automated inward-dependency
guarantee.

#### 6. Fixture `[[bin]]` + exec

**File**: `cli/launcher/Cargo.toml` (`[[bin]]`), fixture source, exec impl

One arg-driven fixture `[[bin]]` inside the launcher crate, behaviours by
argument: `exit-42`, `block-on-sigterm`, `print-help-sentinel`. Located from
tests via `env!("CARGO_BIN_EXE_<fixture>")`. Implement exec with
`std::os::unix::process::CommandExt::exec` (process-replacing).

`block-on-sigterm` uses a **deterministic readiness handshake, not a timer**: it
first installs its SIGTERM disposition (or confirms the default), *then* writes a
readiness sentinel and flushes it, *then* blocks. The signal-propagation test
reads the child's output until the sentinel is observed and only then sends
SIGTERM, asserting caller `$?`=143 (128+SIGTERM). No `sleep`/timeout sequences
the signal, so the test cannot flake under CI load.

The coverage/lint exclusion is scoped to the **fixture `[[bin]]` source alone**
(a stub calling `process::exit` trips `clippy::exit`) — a deliberate, bounded
exception for test scaffolding. Because `CommandExt::exec` replaces the process,
the real exec call site is never covered by in-process instrumentation either;
so all argv-marshalling and path-selection logic (non-UTF-8 `OsString`
forwarding, the `ACCELERATOR_<SUB>_BIN` normalisation/selection) lives in a
**covered `launch` module** and is unit-tested by building the argv vector
*without* calling `exec`, with only the final `exec` syscall left uncovered. The
release staging (0165) stages only the `accelerator` binary, so the fixture
never ships.

#### 7. Update the changed black-box test

**File**: `cli/launcher/tests/version.rs`

`an_unknown_subcommand_exits_non_zero` documented pre-0164 behaviour (clap
hard-rejects unknown subcommands). With `external_subcommand`, an unknown
subcommand now routes to resolution. Update/relocate the test so it reflects the
new behaviour (fake-resolved dispatch, or a named resolution error where no
fixture/override applies).

### Success Criteria

#### Automated Verification

- [x] `mise run test:unit:cli` passes: dispatch routes `["accelerator",
      "frobnicate","--flag"]` to `External` with the tail intact; an empty
      `External` vector yields the named error, not a panic; a **non-UTF-8
      `OsString` arg survives verbatim** to the exec'd child; a fixture exiting
      42 makes `accelerator <sub>` exit 42; a SIGTERM'd fixture yields caller
      `$?`=143 via the readiness handshake (not timing); `ACCELERATOR_<SUB>_BIN`
      execs the named binary with no resolution, **including a hyphenated
      subcommand name** exercising the normalisation; a **built-in/external
      boundary test** asserts against the *actual built-in registry the
      dispatcher consults* — every clap-declared non-external command (`version`,
      the reserved `config` slot) never routes to `External`, and an arbitrary
      name *does* route to `External` — so adding or removing a built-in without
      updating the guard fails. (36 tests pass. `config` is not yet a
      clap-declared command in 0164, so the boundary test covers `version` +
      arbitrary; the reserved `config` slot arrives with 0167.)
- [x] `mise run cli:check` passes (rustfmt + clippy pedantic/nursery/restriction,
      `-D warnings`).
- [x] `mise run deny:check` passes — the rustls trap is not sprung;
      `cargo tree -e features -p launcher` shows no `openssl-sys`/`native-tls`
      and no native-cert crate, and shows `ring` present, `aws-lc-rs` absent
      (parametrised deny regression test in
      `tests/integration/deny/test_launcher_feature_graph.py`).
- [x] `mise run pup:check` passes (no launcher code under `version::core`; the
      new `launch::core` has its own inward-only rule).
- [~] The pinned-MSRV CI leg builds and tests green. (Added `check-cli-msrv` to
      `.github/workflows/main.yml`; validated locally as far as possible —
      `cargo build --workspace --locked` + `deny:check` pass, YAML/actionlint
      clean, workflow-invariant test green — the leg itself only runs in CI.)
- [~] `mise run` exits 0. (All cli-affecting component checks pass;
      full CI-mirror not re-run this phase — changes are confined to `cli/`, one
      Python test, and CI YAML, which do not touch frontend/server/scripts.)

#### Manual Verification

- [x] `cargo tree -e features -p launcher` visibly resolves reqwest with rustls
      only. (Automated by the feature-graph regression test.)
- [x] `accelerator frobnicate` (override-resolved to the fixture) propagates a
      non-zero exit and SIGTERM as expected. (Automated by the dispatch
      black-box tests.)

---

## Phase 2: Real resolution — fetch → verify → cache (hermetic)

### Overview

Implement the real driven adapters behind `ResolveBinary`: reqwest fetch,
sha256 + in-process minisign verification against the embedded fixture key,
atomic cache write keyed by name+version+checksum under a resolved cache root,
and scan-first resolve-once-and-cache. All tests hermetic against a local mock
HTTP server + a fixture keypair (plus a second, non-release key).

### Changes Required

#### 1. Embed the release public key + verify in-process

**File**: `cli/launcher/src/launch/` outbound adapter, committed fixture pubkey

`include_str!` the committed **fixture** release public key(s) behind a
**verify-any-of** set (behind the test-only feature of Decisions §5, so no
fixture key reaches a release build). The minisign signature is the security
boundary; sha256 is a corruption check. Both the cached binary and any cached
manifest live in a user-writable dir, so verification runs in this order —
**each step gates the next, and no manifest field is trusted before its
signature**:

1. verify `manifest.minisig` over the **raw manifest bytes** against the trusted
   keys;
2. parse a **minimal, version-stable outer envelope** carrying only
   `schema_version`, and apply the `schema_version` gate (fail closed on an
   unrecognised higher major; lenient on additive unknown fields) **before**
   parsing the rest — so a breaking future schema yields a clear
   "unsupported schema_version" diagnostic, not a misleading parse/version error;
3. parse the remaining fields and apply the **version-equality anti-rollback
   check** (manifest `version` compared for exact equality against the launcher's
   own `CARGO_PKG_VERSION`; a non-equal *or newer* manifest is refused by
   design);
4. verify per-binary sha256 + minisign;
5. **re-verify the signature before every exec, including cache hits**;
6. a valid sha256 with a **non-release-key** signature is refused.

Both this resolution path and the Phase 3 help path obtain the trusted manifest
through **one shared `Manifest` loader** that owns steps 1–3, so the anti-rollback
and schema gates cannot diverge between the two entry points.

#### 2. Manifest shape (fixture, the 0165 contract)

**File**: fixture `manifest.json` under the launcher test tree, reader types

Name-keyed, per-platform inner map, per-binary `description`, `sha256` as
lowercase hex, `signature` as the inline `.minisig` contents, plus top-level
`schema_version` and `version`. The reader is **liberal in what it accepts**:
the shared Python hashing path (`tasks/shared/hashing.py`) returns bare hex while
the legacy `checksums.json` writer adds a `sha256:` prefix, so the Rust reader
**strips a `sha256:` prefix if present** rather than assuming one form — a
prefixed digest from the shared hashing path cannot silently fail verification.
An **all-zeros digest is the "no binary for this version" sentinel** (carried
forward from `checksums.json`) and is a **named error**, never treated as a real
hash. This `manifest.json` **supersedes** `checksums.json` (Decisions §4), so its
`version` field is the one added to `validate_version_coherence` and the fixture
manifest derives `version` from the workspace `CARGO_PKG_VERSION` rather than
hardcoding it (so a routine version bump cannot silently drift the fixture into
the mismatch path). Shape:

```json
{
  "schema_version": 1,
  "version": "1.24.0-pre.2",
  "binaries": {
    "foo": {
      "description": "Bar tool",
      "platforms": {
        "darwin-arm64": { "sha256": "<hex>", "signature": "untrusted comment: ...\n<b64>\ntrusted comment: ...\n<b64>" }
      }
    }
  }
}
```

Platform aliases (`darwin-arm64`/`darwin-x64`/`linux-arm64`/`linux-x64`) are
**single-sourced from `tasks/shared/targets.py`**. Note the existing
`targets.py` maps only **Rust triples → aliases**; it does *not* enumerate the
`uname -m`/`uname -s` spellings the bootstrap `case` arms normalise. So this
story **adds the canonical `uname`-input → alias table** (the `arm64`/`aarch64`
and `x86_64`/`amd64` `uname -m` spellings and the `darwin`/`linux` `uname -s`
values) alongside the triple map in `targets.py`, giving the coherence test a
real oracle. The launcher and bootstrap tables are generated from that canonical
table, or (if generation is impractical across the three languages) a
**cross-language coherence test loads it as its oracle** and asserts the launcher
and `bin/accelerator` tables against it — the **full `uname`-input → alias
mapping in every language**, not just the four canonical aliases, so a bootstrap
`case` arm that fails to normalise `amd64`/`aarch64` fails the test rather than
404-ing on a user's host. The test runs under `mise run` (named in Testing
Strategy) so it gates CI.

#### 3. Resolution collaborators

**File**: `cli/launcher/src/launch/` outbound adapters

Compose small, separately-testable collaborators rather than a monolith. The
orchestrator itself is a **thin, readable sequence of guard clauses** over an
explicit resolve state machine — `override → cache-hit-verify →
miss-fetch-verify-cache → evict-refetch-once` — with the branching contained per
collaborator rather than woven into one deep function:

- **`Fetcher`** — blocking reqwest (rustls/`ring` + hickory-dns). Connect
  timeout + read/idle stall timeout + aggregate deadline (not one fixed total).
  Bounded retry-with-backoff on transient/5xx (safe — resolution is idempotent);
  **each attempt truncates/recreates its temp file** so a partial-then-dropped
  body cannot concatenate into a corrupt file, and the **backoff is clamped to
  the remaining aggregate deadline** — no new attempt starts once the remaining
  budget is below a minimum, and the terminal error class (deadline-exceeded vs
  attempts-exhausted) is deterministic. Pins `https` (including on the
  post-redirect URL); follows redirects only to a host that is the exact release
  origin or matches a **dotted-label boundary** allowlist
  (`host == "objects.githubusercontent.com"` or `host.ends_with(".githubusercontent.com")`),
  so `evil-githubusercontent.com` and `githubusercontent.com.attacker.net` are
  refused.
- **`Verifier`** — the shared `Manifest` loader (raw-bytes signature →
  `schema_version` gate → version-equality; §1), then per-binary sha256 +
  minisign.
- **`CacheStore`** — **idempotently `mkdir -p` the resolved cache dir** (treating
  `EEXIST` as success, so two concurrent first-use invocations both succeed),
  fetch into a unique per-process `mktemp` temp file **inside** that dir, created
  **`0600`** so unverified bytes are never other-readable/executable (intra-fs
  `rename(2)`, avoids `EXDEV`), verify there, then set the executable bit and
  atomic-rename into the name+version+checksum name — mirroring the shell
  launcher's verify-then-`install -m 0755` ordering, so only fully-verified bytes
  ever appear at the final path with the exec bit. Caps download size + checks
  free space (advisory; the per-download size cap is the real bound, and the
  `ENOSPC` path unlinks its temp file). The **load-time integrity guarantee comes
  from rename-by-inode**: because the key includes the checksum the atomic-rename
  replaces by inode rather than truncating in place, so the kernel holds the
  verified inode open across `execve` even if the path is concurrently
  replaced/unlinked, and replacing a path another process is exec'ing does not
  hit `ETXTBSY` (a load-bearing, tested invariant, not an incidental note). The
  per-cache-key `flock`/`fcntl` advisory lock serialises fetch/verify/rename and
  is **marked `FD_CLOEXEC`, so it releases at `execve`** — the lock's job ends at
  the exec-handoff; it does not (and need not) span the load itself, because
  rename-by-inode is what makes the verified bytes the executed bytes. On a
  **cache-hit verification failure** the orchestrator self-heals by
  **fetching-and-verifying the replacement into a temp file *first* and unlinking
  the corrupt entry only as part of the atomic rename** (replace-in-place) — a
  working entry is never observably evicted before a verified successor exists,
  and if the re-fetch fails offline the diagnostic distinguishes "cached copy
  corrupt AND re-fetch failed" from a plain miss.
- **`CacheRootResolver`** — `${CLAUDE_PLUGIN_ROOT}` **probed** for
  writable+exec-capable; `ACCELERATOR_CACHE_DIR` is the only alternate terminus.
  An unset `CLAUDE_PLUGIN_ROOT`, a read-only/noexec root with no override, and
  nothing-resolvable are **named errors** — **no XDG fallback** (Decisions §2:
  an XDG-resident binary would break the `allowed-tools` glob match). The root is
  a driven-port input so tests inject a temp dir; a read-only/noexec injected
  root asserts the named error. No retained-versions cap or eviction is needed —
  the version-scoped root is naturally bounded (Decisions §2).

The real `ResolveBinary` adapter replaces Phase 1's fake, orchestrating the
collaborators; `ACCELERATOR_<SUB>_BIN` still short-circuits before any of them.
On a cache **hit** the launcher reuses the cached binary+manifest without
re-fetching, so an already-resolved sub-binary works **offline**; a cache
**miss** with no network fails cleanly with a target-naming diagnostic.

#### 4. Hermetic test harness

**File**: launcher tests, dev-dependencies

Local mock HTTP server serving fixture bytes; failure cases induced by pointing
the launcher's URL base at it (404 / absent asset / connection refused / 5xx).
Fixture keypair + a second non-release key. cargo-deny scans dev-deps, so the
mock-server/signing dev-deps must be rustls-only/TLS-free (or set `exclude-dev`
deliberately) — recommend rustls-only to keep the ban maximally strict; call it
out at review.

### Success Criteria

#### Automated Verification

- [x] `mise run test:unit:cli` passes: fetch→verify→cache→exec happy path;
      **cache reuse asserts the mock server received exactly one request across
      two invocations** (proving no re-fetch); a **cached signed binary mutated
      on disk is refused on re-invocation with no exec** (pre-exec re-verify),
      while a cache-hit verification failure **replaces-in-place (fetch+verify a
      clean copy, then atomic-rename over the corrupt entry) and execs the clean
      copy**, and a corrupt entry with **no network** fails with the distinct
      "corrupt AND re-fetch failed" diagnostic (not a plain miss); checksum
      mismatch refused naming sha256; non-release-key signature refused naming
      minisign; tampered-manifest signature refused; a validly-signed
      **wrong-`version` manifest refused** (anti-rollback); an **unsupported
      higher `schema_version` refused** with the schema diagnostic (not a version
      error); `5xx-then-200` recovers in one resolve, and persistent-5xx gives up
      after the bounded attempts (asserting the count); a cross-host redirect to a
      disallowed host is **refused** and `evil-githubusercontent.com` /
      `githubusercontent.com.attacker.net` fail the allowlist unit test;
      **cache-root branches** — unset `CLAUDE_PLUGIN_ROOT` → named error, an
      injected **read-only root → named error** (no XDG fallback), two concurrent
      first-use invocations both succeed (`mkdir` `EEXIST` idempotent);
      unreachable/no-asset/unavailable each exit non-zero with a target-naming
      diagnostic and no exec; an **offline cache-hit still resolves and execs**;
      `ACCELERATOR_<SUB>_BIN` still bypasses all of it. **Deferred within Phase 2:
      the `partial-body-then-200` reset (blocking reqwest returns a complete body
      or a transport error, so a torn body is not a distinct reqwest state to
      exercise) and the explicit deadline-vs-attempts terminal-error-class
      distinction — the current fetcher uses a bounded attempt count with
      per-request timeouts.**
- [~] Each collaborator is unit-tested in isolation as a **gating** criterion:
      the redirect-allowlist and https-pin are unit-tested; the manifest/verifier
      gates and cache store/find/replace-in-place are unit-tested; cache-root
      branches are unit-tested. **Deferred within Phase 2: an explicit per-cache-key
      `flock`/`FD_CLOEXEC` advisory lock and its injected-barrier concurrency
      tests, and the byte-stall `Fetcher` timeout unit test. Concurrent first-use
      is covered by a threaded resolution test (both succeed via atomic
      rename-by-inode + idempotent `mkdir`); the `ETXTBSY`-free replace-in-place
      is covered by the store round-trip and self-heal tests.**
- [x] `mise run deny:check` passes (dev-deps do not spring the native-tls ban).
- [x] `mise run cli:check` / `mise run pup:check` pass.
- [~] `mise run` exits 0. (All cli-affecting component checks + the tasks test
      suite pass; full CI mirror not re-run this phase.)

#### Manual Verification

- [x] Against the mock server, a first invocation fetches+caches and a second
      reuses (observable via server hit count) — automated by
      `cache_reuse_does_not_refetch`.
- [~] Interrupting a fetch mid-download leaves no file at the final cache name;
      a later invocation fetches cleanly. (By construction — verified bytes are
      written to a unique per-call temp inside the cache dir and only atomically
      renamed to the final name after verification; not exercised by an explicit
      interrupt test.)
- [~] The host-target launcher build is rustls-only with no dynamic OpenSSL
      (`otool -L`/`ldd`). (The graph-level half is automated by the feature-graph
      regression test; the dynamic-link inspection of a release build is a
      manual/0165 step.)

The musl-specific guarantees — full static linking under musl, `hickory-dns`
actually resolving without `getaddrinfo`/nsswitch, and bundled webpki-roots
sufficing with no host cert store — **cannot be exercised by a darwin or
linux-gnu host build and are formally unvalidated in 0164**. This is a tracked
deferral: **0165 carries an acceptance criterion** that gates on the four-triple
static-link/DNS/cert-store guarantees (`ldd`-verified per musl triple). 0164
closes the graph-level half via the `cargo tree` feature assertions
(rustls/`ring`/hickory present, `native-tls`/`openssl`/native-cert absent) which
`deny:check` evaluates across all four release triples.

---

## Phase 3: Discoverable surface — manifest-driven help + `--help` delegation

### Overview

Make the surface discoverable despite external subcommands being absent until
fetched: render clap's built-in help plus a synthesised external-subcommands
section from the manifest's `description` entries, and delegate per-command
`--help` by re-exec'ing the child.

### Changes Required

#### 1. Synthesised help (lazy path)

**File**: `cli/launcher/src/launch/`, `main.rs`

`accelerator --help` renders clap help for built-ins **plus** an "external
subcommands" section built from the **shared `Manifest` loader**'s `description`
entries (§2 §1; no executing untrusted binaries). Derive `Cli::parse()`
intercepts `--help` before any launcher code runs and `after_help` takes only a
compile-time string — so use the lazy path: `try_parse`, and **only** on
`ErrorKind::DisplayHelp` load the manifest through that same loader and re-render
via `Cli::command().after_help(section)`. Lazy, not eager: appending before
parsing would force a manifest read+verify on every invocation, coupling offline
built-ins (`accelerator version`) to manifest availability. Non-help and
built-in invocations never touch the manifest.

Descriptions are signature-verified but still render to a terminal, so they are
**sanitised once at the trust boundary**: the shared loader returns
already-sanitised description strings (a constructed-sanitised newtype), so every
downstream printer — help now, any error path naming a binary later — is safe by
construction rather than by remembering to strip. Sanitisation operates over
**decoded Unicode scalar values** (removing C0/C1 controls and the ESC/CSI
introducers, preserving printable and whitespace scalars) so a multi-byte UTF-8
description is never split mid-sequence.

#### 2. `--help` delegation

**File**: `cli/launcher/src/launch/`

`accelerator foo --help` resolves + re-execs the child with `--help`, emitting
the child's own help — verified by a sentinel only the `print-help-sentinel`
fixture emits.

### Success Criteria

#### Automated Verification

- [x] `mise run test:unit:cli` passes: the manifest-derived line matching both
      `foo` and `Bar tool` is asserted by the `external_subcommands_section` unit
      test (the binary pins the embedded release key, so a test cannot sign a
      manifest under it — the rendered-section assertion belongs at the unit
      level, matching the reference impl); `foo --help` emits the fixture's
      sentinel (dispatch delegation test); a manifest description containing
      several distinct C0/C1/ESC characters and a multi-byte UTF-8 run renders
      sanitised — asserted by **exact equality** against the expected string, not
      a present/absent pair; **`accelerator version` succeeds with no manifest
      present** and top-level `--help` degrades to the built-in help when the
      manifest is unavailable (black-box). **Deviation: `accelerator <unknown>`
      routes to on-demand resolution (git-style fetch), not the help listing —
      rendering help for an unknown subcommand would contradict the
      fetch-on-demand design; the discoverable listing is surfaced by
      `--help`.**
- [x] `mise run cli:check` passes.
- [~] `mise run` exits 0. (All cli-affecting checks + tasks tests pass; full CI
      mirror not re-run this phase.)

#### Manual Verification

- [~] `accelerator --help` reads coherently with the synthesised section beneath
      the built-ins. (The section is unit-tested; the composed rendering with a
      live signed manifest is a manual/0165 step, since a test cannot sign under
      the embedded key.)

---

## Phase 4: `bin/accelerator` bootstrap + `cli/verify/` shim crate

### Overview

Write the `bin/accelerator` entry point — a thin bash-3.2 bootstrap that fetches
the launcher on first use — add the vendored `cli/verify/` minisign shim that
gives it a key-bound root of trust, and prove the whole path hermetically. The
four-triple cross-build/staging and reproducible-build byte-identity are 0165;
here everything is built for the host and tested against fixtures.

### Changes Required

#### 1. The `cli/verify/` shim crate

**File**: `cli/verify/` (new third member), `cli/Cargo.toml` members

A tiny first-party CLI (read pubkey + `.minisig` + target, verify, exit
0/non-zero) built under the same `-D warnings` + restriction lints — its verify
path propagates `Result`, no `.unwrap()`. Its own `[package]` with
`rust-version = "1.90.0"`, included in the MSRV CI leg. It is **tooling, not a
subdomain**, so it stays out of the pup layering rules and does not activate the
deny cross-crate ban-lists (confirm they ignore it). Being pure
`minisign-verify` it links statically with no heavy closure. The shim is **not**
itself minisign-signed (a root of trust cannot be verified by what it roots), so
its integrity rests on the **plugin package's own distribution integrity** (the
marketplace/plugin channel) — stated as an explicit 0164 assumption (Decisions
§1) — with 0165 adding reproducible-build byte-identity. To keep that root
trustworthy the bootstrap runs the shim **only from a path it controls** (the
plugin root, or a copy it writes into a fallback dir it owns with restrictive
permissions), by absolute path, so another local process cannot pre-plant the
shim it will execute. 0164 builds the shim for the **host arch only**; the
committed per-triple shims are a **0165 deliverable**, and 0165 carries an
acceptance criterion that all four are built, reproducible, and byte-verified.

#### 2. `bin/accelerator` (the entry point)

**File**: `bin/accelerator` (new)

Named for the command it fronts: consumers invoke
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator <args>`. Bash 3.2 (`set -uo pipefail`,
`BASH_SOURCE` root resolution, `case` matching; no associative arrays, `${x,,}`,
`mapfile`). It:

- validates `CLAUDE_PLUGIN_ROOT` is set and points at an existing directory,
  emitting a **named diagnostic and non-zero exit** (not a raw `set -u`
  message);
- detects host triple → platform alias via explicit `case` arms normalising
  `uname -m` (`arm64`/`aarch64`, `x86_64`/`amd64`) and `uname -s`, from the
  single-sourced alias map;
- fetches the launcher binary if absent (portable sha256: prefer `shasum -a
  256`, fall back to `sha256sum`; fetcher `curl` or `wget`, named error if
  neither), **carrying forward the existing `launcher-helpers.sh` `download_to`
  hardening** rather than restating a weaker subset — since this is the outermost
  root-of-trust fetch (it downloads the launcher before any Rust verification
  exists): curl `--proto '=https' --tlsv1.2 --max-redirs N --max-filesize N
  --connect-timeout N --max-time N` (and the `wget` equivalents:
  `--max-redirect`, `--timeout`, a size guard), never `-k`/`--no-check-certificate`.
  A non-https redirect is refused and a stalled connection fails with a bounded
  named error, not a hang; ideally the bootstrap reuses `download_to` directly so
  the hardening is single-sourced;
- **verifies the launcher's `.minisig` with the vendored per-triple shim against
  the plugin-committed key** before exec, re-verifying on cache-hit, failing
  closed with a named diagnostic if the shim is unrunnable or verification fails
  — never silently downgrading to TLS-only;
- binds launcher **freshness**: fetch+verify `manifest.minisig` + version
  equality, then require the fetched launcher's sha256 to match the manifest
  entry (closes replay of an older validly-signed launcher);
- caches atomically and self-heals with the **same replace-in-place ordering as
  the launcher** (unique per-process `mktemp` temp → verify → atomic rename; a
  cache-hit failure fetches+verifies the replacement *before* the rename replaces
  the corrupt entry, so a working launcher is never evicted without a verified
  successor). Because bash has no `flock`-based orchestrator, it takes a
  bash-3.2-safe **per-target advisory lock** (`mkdir`-based, or `flock` where
  available) around fetch/verify/rename so two concurrent first-use sessions do
  not interleave — with the **stale-lock recovery the existing `launch-server.sh`
  already models** (a `trap 'rmdir …' EXIT INT TERM` release, plus a bounded
  acquisition timeout that reports a named error naming the stale-lock path, and
  a PID-owner stale-lock reclaim so a lock orphaned by a SIGKILL'd bootstrap does
  not permanently wedge every future invocation). The `flock` path uses
  non-blocking acquisition. A bootstrap test asserts a lock orphaned by an exited
  process is reacquirable, mirroring the Rust CacheStore criterion;
- shares the launcher's resolved cache root — the **same plugin-root-or-override,
  else named-error resolution (no XDG fallback)** as the launcher (Decisions §2),
  single-sourced not re-implemented; since the plugin root may itself be noexec,
  it copies the shim and launcher into that resolved **writable+exec** root
  before running them, and invokes the shim by **absolute path**, never via
  `PATH`;
- execs the launcher forwarding `"$@"` as an argv list (no shell-string
  interpolation of derived triple/URL values), with `CLAUDE_PLUGIN_ROOT` present
  in the launcher's environment.

#### 3. Extend the shell lint globs

**File**: `tasks/shared/sources.py` (`_EXTRA_SHELL_SOURCES`), `scripts/lint-bashisms.sh`

`bin/accelerator` is extensionless, and the repo has **two independent
shell-source discovery mechanisms** it must be registered in — registering only
one lets the most platform-sensitive script in the change silently escape a
tool:

- **shfmt + ShellCheck** consume `tasks/shared/sources.py::shell_sources()`, whose
  tree walk matches `*.sh` and appends extensionless extras from the
  `_EXTRA_SHELL_SOURCES` tuple (which already carries
  `…/cli/accelerator-visualiser` as precedent). Add `bin/accelerator` there.
- **`scripts/lint-bashisms.sh`** discovers its own files via
  `git ls-files '*.sh'` — a glob that will **never** match an extensionless file.
  Add `bin/accelerator` to its default discovery explicitly.

Add a coverage test asserting `bin/accelerator` appears in the effective target
list of **all three tools independently**, so it cannot silently escape the
bash-3.2 floor. Confirm the executable-bit invariant treats `bin/accelerator` as
an entrypoint (`0755`).

#### 4. Package assembly

**File**: `.claude-plugin/plugin.json` (reviewed), committed key + host shim

`plugin.json` has no field registering a CLI as an invokable command — the
invocation contract is **by path** (`${CLAUDE_PLUGIN_ROOT}/bin/accelerator`).
This story fixes and documents that path contract; the skills/hooks that consume
it arrive in later stories (0167/0169), so here the entry point is built and
tested standalone. The package ships the release public key(s) and the shim; the
key is byte-identical to the one the launcher embeds. Both in-repo consumers
reference the **same committed key file**, and a **0164-scoped test asserts the
launcher-embedded key bytes equal the bootstrap-committed key bytes** (the
signer-side cross-check — that this key also signed the production manifest — is
0165), closing the window where the two in-repo trust anchors could drift before
0165 lands. The
**release-time assertion that no fixture/test key is in the embedded
verify-any-of set** (Decisions §5) runs here as part of package assembly, so a
build that would ship the fixture key as a valid signing root fails closed
rather than reaching users.

### Success Criteria

#### Automated Verification

- [ ] `mise run scripts:check` passes (shfmt + ShellCheck + bashisms) with
      `bin/accelerator` added to **both** discovery mechanisms
      (`_EXTRA_SHELL_SOURCES` and `lint-bashisms.sh`'s own file list) — a test
      asserts it is covered by all three tools independently.
- [ ] `bash scripts/test-accelerator-entrypoint.sh` (new standalone suite)
      passes **hermetically** (fetch stubbed via an overridable downloader or a
      pre-seeded cache; no network): host-triple detection driven by **injected
      `uname -m`/`uname -s` covering all four alias combinations** (`arm64`,
      `aarch64`, `x86_64`, `amd64` × darwin, linux), so the `case` normalisation
      is validated without the hardware; cache-present short-circuit (no fetch);
      a pre-seeded **tampered** cached launcher refused fail-closed (cache-hit
      re-verify, not exec'd); a launcher-signature failure → fail-closed named
      error; a **validly-signed-but-stale launcher** (valid `.minisig`, wrong
      version/sha256 vs the verified manifest) refused fail-closed (anti-replay,
      mirroring the Rust anti-rollback test); the **verify shim unrunnable →
      fail-closed named error** (no silent TLS-only downgrade); a read-only/noexec
      plugin root **with `ACCELERATOR_CACHE_DIR` set** → shim+launcher copied to
      that writable+exec root and run, and **without an override → named error**
      (no XDG fallback); a **`PATH`-planted decoy named like the shim is not
      used** (absolute-path invocation); **two concurrent bootstraps** do not
      corrupt the cache (deterministically interleaved via a blocking stub
      downloader that holds inside the critical section until the second process
      is confirmed waiting on the lock — not a `sleep`); a **lock orphaned by an
      exited process is reacquirable** (stale-lock reclaim, not a permanent
      wedge); a **stalled fetch** yields a bounded named error (not a hang) and a
      **non-https redirect is refused**; unset/invalid `CLAUDE_PLUGIN_ROOT` →
      named error; argument/exit-code forwarding.
- [ ] `mise run test:unit:cli` passes: the **shim black-box tests** run against
      the real compiled shim (valid → 0, tampered → non-zero, non-release-key →
      non-zero).
- [ ] `mise run cli:check` / `mise run deny:check` / `mise run pup:check` pass
      (the shim crate does not trip layering/ban rules).
- [ ] `mise run` exits 0.

#### Manual Verification

- [ ] On a clean machine with no cached binary, invoking
      `${CLAUDE_PLUGIN_ROOT}/bin/accelerator version` (against a host-built,
      fixture-signed launcher) fetches, verifies, caches, and runs the launcher's
      `version` output.

---

## Testing Strategy

### Unit Tests

- **Rust (`cli/`)**: dispatch routing to `External` + **non-UTF-8 `OsString`
  preservation** through exec; exec exit/signal propagation as **black-box tests
  spawning `CARGO_BIN_EXE_accelerator`** (readiness handshake for SIGTERM);
  `ACCELERATOR_<SUB>_BIN` override; built-in/external boundary; resolution error
  taxonomy + boundary mapping into `kernel::Error`; manifest-signature +
  version-binding (exact equality, newer refused) + `schema_version` gate
  (raw-bytes-first ordering) + sha256 + minisign (happy + each refusal); atomic
  cache scan/write/reuse (exactly-one-fetch) + corrupt-cache refuse +
  replace-in-place self-heal + offline cache-hit + concurrent-first-use
  `mkdir` idempotence; retry (5xx-then-200, partial-body-then-200,
  persistent-5xx with deterministic terminal error class); cross-host-redirect
  allow + dotted-label-boundary deny (`evil-githubusercontent.com`,
  `…com.attacker.net`); cache-root branches (unset / read-only-noexec →
  named error, no XDG); env-var-name normalisation for hyphenated subcommands;
  help synthesis (lazy) + escape-stripping (exact-equality, multi-byte UTF-8) +
  built-ins-work-with-no-manifest + `--help` delegation. Each collaborator
  (`Fetcher`/`Verifier`/`CacheStore`/`CacheRootResolver`) unit-tested in
  isolation — Fetcher timeouts (stalled → named error; slow-but-progressing not
  aborted; 404/no-asset not retried) and CacheStore concurrency (held lock →
  wait-then-reuse or named acquisition-timeout; lock from an exited process
  reacquirable) — as **gating** criteria. The **verify shim** has its own
  black-box tests. Copy the `version/core.rs` port+fake pattern; keep cores
  infrastructure-free for pup.

### Integration Tests

- **Hermetic launcher resolution** against a local mock HTTP server (fetch,
  verify, cache, refuse, fetch-failure, offline cache-hit), with a fixture
  keypair + a second non-release key.
- **cargo-deny regression** — confirm the native-tls/openssl ban holds with the
  new (and dev) closure and assert via `cargo tree` that no native-cert crate
  enters the launcher tree and `ring` is present / `aws-lc-rs` absent.
- **Cross-language contract coherence** — with `tasks/shared/targets.py` as the
  oracle, the full `uname`-input→platform-alias mapping (both `uname -m`
  spellings and `uname -s`) agrees across the launcher and `bin/accelerator`, not
  just the four canonical aliases; and the `manifest.json` fixture conforms to
  the shared contract schema (Decisions §6).

### Manual / CI

- Static-link + arch verification of the **host-target** launcher build
  (`otool -L`/`ldd`) — AC9 as a host-target property. The **four-triple**
  musl-static / DNS / cert-store verification is a tracked 0165 gating AC
  (Phase 2 Manual Verification).
- A **pinned-MSRV build/test leg** (all members at `rust-version = "1.90.0"`) so
  an MSRV-breaking transitive bump fails in CI, not at a user's first fetch; the
  leg re-runs `deny:check` after the `resolver = "3"` bump.

## Performance Considerations

- Blocking reqwest pulls a transitive `tokio` into the launcher (accepted,
  ADR-0054) — a heavier tree, the price of one HTTP stack workspace-wide; the
  launcher declares no direct `tokio`.
- Resolve-once-and-cache makes first use pay a bounded network fetch (explicit
  timeouts + bounded retry); steady state is a cache-hit scan + verify + exec,
  which works offline for cached sub-binaries. A cache miss with no network
  fails cleanly (no offline fallback for an uncached binary — ADR-0046 caveat).
- The hot-path concern (a per-Bash-call `PreToolUse` guard fetching a
  sub-binary) is why `version`/`config`-shaped guards stay built-in; 0164 fixes
  that split point but does not add the guard (0169).

## Migration Notes

- Populating the launcher-local resolution error does not touch `main.rs`'s
  error arm — it already renders any `kernel::Error` generically.
- `tests/version.rs::an_unknown_subcommand_exits_non_zero` documents pre-0164
  behaviour that `external_subcommand` changes; when relocating it in Phase 1,
  **preserve its intent** — assert that an unresolvable unknown subcommand (no
  fixture/override, no network) still exits non-zero with stderr naming the
  subcommand and the failed resolution step, never a panic or silent success.
- The embedded pubkey is a **fixture** placeholder behind a test-only feature
  (Decisions §5); swapping to 0165's production key (verify-any-of overlap) is
  the concrete 0164→0165 handoff, enforced by the release-time
  no-fixture-key assertion, and recorded at 0164 validation and in 0165.

## References

- Work item: `meta/work/0164-launcher-and-git-style-dispatch.md`
- Research: `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
- Parent scope/architecture research:
  `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- ADRs: `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`,
  `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md`,
  `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`,
  `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md`
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md` (§2 dispatch, §3 launcher)
- Reference spec (near-impl fidelity, not yet implemented):
  `../luminosity/meta/plans/2026-07-03-0008-static-binary-distribution-and-launcher.md`
- Insertion points: `cli/launcher/src/version/inbound/cli.rs:17-21`;
  `cli/launcher/src/main.rs:13-28`; `cli/kernel/src/lib.rs:6-10`;
  `cli/Cargo.toml:1-21`; `cli/deny.toml:35-53`; `cli/pup.ron:8-25`;
  `cli/launcher/tests/version.rs`; `tasks/shared/targets.py:1-6`
- Sibling stories: `meta/work/0165-*` (distribution/signing — supplies the
  production manifest/keys/assets), `meta/work/0167-*` / `meta/work/0169-*`
  (config/hooks — co-determine the built-in/external boundary),
  `meta/work/0168-*` (visualiser — first dispatched sub-binary)
