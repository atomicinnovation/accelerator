---
date: "2026-04-18T15:00:00+01:00"
type: plan
skill: create-plan
ticket: null
status: approved
---

# Meta Visualiser — Phase 2: Server Bootstrap and Lifecycle

## Overview

Land the real Rust server, the binary-acquisition flow in `launch-server.sh`,
and the lifecycle machinery that keeps the server self-cleaning. After this
phase, `/accelerator:visualise` downloads (or cache-hits) the correct per-arch
binary, execs it against a preprocessor-written `config.json`, backgrounds it,
and returns a real `http://127.0.0.1:<dynamic-port>` URL; the server writes
`server-info.json` on listen, shuts down cleanly on SIGTERM/SIGINT, exits after
30 min idle, and exits if the Claude Code harness process dies. `GET /` returns
a 200 with a one-line placeholder body — no HTML, no API routes; the library,
lifecycle, and kanban views all land in later phases.

Phase 2 lands as **nine sub-phases**. Each introduces a single concern with
failing tests first; most sub-phases commit the tests in a distinct `jj`
revision before the implementation so the red-then-green transition is preserved
in `jj log`. Where a sub-phase bundles tests and implementation, a mutation
smoke test (temporarily break the implementation; confirm the suite fails) is
called out in Manual Verification.

This phase also resolves three gaps flagged in the research follow-up analysis:

- **Gap 2 — `config.json` schema**: the preprocessor produces it and the Rust
  binary consumes it. Locked in Phase 2.2 via the `Config` struct's `serde`
  contract and a committed `tests/fixtures/config.valid.json`.
- **Gap 5 — Rust edition and MSRV**: edition `2021`, `rust-version = "1.80"`,
  pinned in `Cargo.toml` in Phase 2.1.
- **Gap 6 — first-run download UX**: a single `Downloading visualiser server
  (first run, ~8 MB)…` line is emitted to stderr before `curl` runs, in Phase
  2.7.

The launcher resolves the binary via a **three-layer precedence**:

1. `ACCELERATOR_VISUALISER_BIN` environment variable (highest) — one-shot,
   shell-scoped dev override.
2. `visualiser.binary` config key in `.claude/accelerator.md` /
   `.claude/accelerator.local.md` — persistent team or personal override (per
   ADR-0017's userspace-config pattern); path resolved against the project root
   if relative.
3. Cached binary in
   `skills/visualisation/visualise/bin/accelerator-visualiser-<os>-<arch>`
   verified against `checksums.json`, downloaded from GitHub Releases on miss
   (lowest).

Layers 1 and 2 both **bypass the SHA-256 check** — a configured or env-supplied
binary is an explicit user override, and a user-built binary would never match a
release manifest hash. Only layer 3 enforces verification.

## Current State Analysis

Phase 1 shipped the skill scaffolding and is fully in place:

- **Skill entry points** exist:
  - `skills/visualisation/visualise/SKILL.md` (slash command,
    `disable-model-invocation: true`, resolves the 11 path keys in its
    preamble).
  - `skills/visualisation/visualise/cli/accelerator-visualiser` (POSIX-compliant
    shell wrapper with symlink-walk).
  - `skills/visualisation/visualise/scripts/launch-server.sh` (Phase 1 stub —
    prints `placeholder://phase-1-scaffold-not-yet-running` and exits).
- **Plugin manifest** already lists `./skills/visualisation/` in
  `.claude-plugin/plugin.json:11` (nine skill category paths total).
- **Bash test harness** is centralised in `scripts/test-helpers.sh` (sourced,
  non-executable; exposes `assert_eq`, `assert_exit_code`,
  `assert_file_executable`, `assert_stderr_empty`, `test_summary`).
- **Integration-test runner** (`tasks/test.py`) auto-discovers every executable
  `test-*.sh`, so any new suite added in this phase enrols automatically with no
  edits to the runner.
- **Existing Phase 1 harnesses** that depend on the sentinel:
  - `skills/visualisation/visualise/scripts/test-launch-server.sh` asserts the
    exact sentinel string and the "single line of output" contract. This harness
    is **replaced wholesale** in Phase 2.6 — the new launcher's stdout shape is
    richer and the old tests become wrong, not merely insufficient.
  - `skills/visualisation/visualise/scripts/test-cli-wrapper.sh` tests the
    wrapper's *delegation* to the stub via a unique-UUID mutation on a tempdir
    copy — it never depends on the real launcher's output. Phase 2 leaves this
    harness unchanged; the wrapper continues to forward `$@` verbatim regardless
    of whether the launcher is a stub or the real bootstrap.
- **SKILL.md** currently shows an `**Install command**` and an "Availability"
  block that tell the user the visualiser UI isn't ready. Phase 2.9 replaces
  this block with guidance for the real server (URL is a copy-paste link,
  `Ctrl+C` in the Claude Code pane does not stop the server, how to stop it
  explicitly).
- **No Rust code exists anywhere** in the plugin. No `Cargo.toml`, no `target/`,
  no Rust sources. The `.gitignore` has no Rust-related rules and no bin-binary
  rules yet.
- **No `bin/` directory** exists under `skills/visualisation/visualise/`. Phase
  2.6 creates it and commits a `checksums.json` manifest whose four SHA-256
  entries are placeholder values (Phase 12 replaces them with real checksums at
  release time).
- **No `<meta/tmp>/visualiser/` directory** exists or is expected to exist at
  repo-quiet time. The preprocessor creates it on first invocation; its contents
  are already gitignored by the nested-`.gitignore` trick in
  `meta/tmp/.gitignore`.
- **`mise.toml`** currently has `test:integration` → `invoke test.integration`
  (a single glob task in `tasks/test.py`) and `test` → `depends =
  ["test:integration"]`. Phase 2.9 restructures this: invoke tasks are split
  into a `tasks/test/` package with component-named sub-tasks
  (`test.unit.visualiser`, `test.integration.visualiser`,
  `test.integration.config`, `test.integration.decisions`); the mise layer
  composes levels only (`test:unit`, `test:integration`, and `test` runs them in
  order).
- **Platform support** is macOS (arm64, x64) and Linux (arm64, x64) per the
  spec's Non-functional section. Phase 2 inherits that scope exactly —
  Windows/WSL/MSYS are unsupported; the preprocessor exits with a clear message
  if `uname -s` isn't `Darwin` or `Linux`.

### Key Discoveries

- **The Rust binary is the one that writes `server-info.json`**, not the
  preprocessor. The preprocessor waits for the file to appear (polling with a
  short sleep) and then prints the URL from it. This mirrors the superpowers
  pattern where the server itself emits `server-started` on its log and writes
  its own info file. Keeps the binary self-describing at lifecycle boundaries.
- **One server per repo** is enforced by the preprocessor, not the server. The
  preprocessor sees an existing `server-info.json` with a `pid` entry, checks
  whether that PID is alive via `kill -0`, and either reuses (prints the URL and
  exits 0) or reaps (removes the stale file, starts fresh). The server itself
  knows nothing about other instances.
- **Owner-PID resolution** uses the grandparent-of-the-shell pattern from
  superpowers: `OWNER_PID="$(ps -o ppid= -p "$PPID")"` with a fallback to
  `$PPID` when the grandparent is PID 1 or empty. Per the research, accelerator
  does **not** need the superpowers auto-foreground branches for WSL/Codex/MSYS
  — accelerator scope is macOS/unix + Claude Code only.
- **Port allocation** uses `TcpListener::bind(("127.0.0.1", 0))` and reads the
  actual port via `local_addr()`. Random range selection (as in superpowers'
  `49152 + random`) is not needed; the kernel picks a free ephemeral port.
- **SIGTERM/SIGINT handling** via
  `tokio::signal::unix::signal(SignalKind::terminate())` +
  `SignalKind::interrupt()`. Both route through a single `shutdown(reason)`
  function that flushes state files and calls
  `axum::serve(...).with_graceful_shutdown(...)`.
- **The `notify` file watcher is NOT added in Phase 2**. That lands in Phase 4.
  Phase 2 keeps the `Cargo.toml` minimal but deliberately pins crates that later
  phases will need (`axum`, `tokio`, `tracing`, `serde`, `serde_json`,
  `tempfile`, `nix`, `clap`), plus `reqwest` is **NOT** a dep because the Rust
  binary never shells out and never makes outbound HTTP — the download lives in
  `launch-server.sh`'s `curl`.
- **`gray_matter`, `serde_yml`, `sha2`, `notify`, `rust-embed` are deferred**.
  They land when the indexer (Phase 3), SSE hub (Phase 4), and frontend embed
  (Phase 5) are introduced. Keeping the dep list small in Phase 2 shortens the
  first-run download size and compile time.
- **`build.rs`** is created in Phase 2.1 as a near-empty stub (emits the
  `rerun-if-changed` hint for its own file). The real `rust-embed` guard from
  D10 lands in Phase 5 when the `embed-dist` feature is introduced. Keeping
  `build.rs` from day one avoids a later migration from a default
  no-build-script layout to one with it.
- **`checksums.json`** is committed in Phase 2.6 with placeholder SHA-256
  entries — the real values land at Phase 12 release time. In Phase 2 dev, the
  `ACCELERATOR_VISUALISER_BIN` override bypasses verification entirely, so the
  placeholders don't block development. The committed shape locks the schema
  (`{"version": "...", "binaries": {"<os>-<arch>": "sha256:..."}}`) so Phase
  12's release script has a fixed target to update.
- **`visualiser.binary` config key** is read via the existing
  `scripts/config-read-value.sh` — no new wrapper script needed. The SKILL.md
  `allowed-tools` frontmatter already permits
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`, so the launcher can shell out
  directly. Relative paths are resolved against the project root
  (`find_repo_root`); absolute paths are used verbatim. This matches the
  resolution pattern `config-read-path.sh` uses for `paths.*` keys and keeps the
  config contract consistent.

## Desired End State

After this phase ships:

1. Running `/accelerator:visualise` in Claude Code (or the CLI wrapper) on a
   freshly-initialised repo produces:
   - First invocation per plugin version (default path): a single `Downloading
     visualiser server (first run, ~8 MB)…` line on stderr, a `curl` download,
     SHA-256 verification against the committed `checksums.json`, and a
     `**Visualiser URL**: http://127.0.0.1:<port>` line on stdout. (In this phase,
     the download step is exercised by the dev override path — Phase 12 produces
     the first real release.)
   - Invocation with `ACCELERATOR_VISUALISER_BIN` set: skips download and checksum,
     execs the override binary directly.
   - Invocation with `visualiser.binary` set in `.claude/accelerator.md` /
     `.claude/accelerator.local.md` (and no env var): resolves the configured path
     against the project root, skips download and checksum, execs it.
   - Subsequent invocations within the same plugin version: skip the download,
     detect the live PID in `<tmp>/visualiser/server-info.json`, and print the same
     URL without starting a new server.
   - Subsequent invocations of a *different* plugin version: re-download (the
     cached binary's SHA-256 no longer matches the manifest entry).
2. Opening `http://127.0.0.1:<port>/` in a browser returns a `200 OK` with a
   single line of placeholder text identifying the phase and the server version.
   No HTML, no JSON API — those land in Phases 3–5.
3. The server self-terminates cleanly when the Claude Code harness process exits
   (owner-PID watch), when no HTTP activity occurs for 30 minutes (idle
   timeout), or when sent SIGTERM/SIGINT. In all three cases it removes
   `server-info.json`, writes `server-stopped.json`, and exits 0.
4. `<tmp>/visualiser/` contains `config.json`, `server-info.json`, `server.pid`,
   `server.log` while the server is live; after shutdown only
   `server-stopped.json`, `server.log`, and `config.json` remain.
5. `mise run test:unit` (new) runs every unit-level invoke task and passes (in
   Phase 2, that's just `test.unit.visualiser` — `cargo test --lib`). `mise run
   test:integration` runs every integration-level invoke task and passes
   (`test.integration.visualiser` — `cargo test --tests` + visualiser shell
   suites; `test.integration.config` — `scripts/test-*.sh`;
   `test.integration.decisions` — `skills/decisions/**/test-*.sh`). `mise run
   test` runs the levels in order (`test:unit` then `test:integration`; later
   extended with `test:e2e`).
6. The Rust binary is built with edition 2021 and MSRV 1.80; `cargo +1.80.0
   build --release` on the maintainer's dev host produces a working binary
   identical to an `ACCELERATOR_VISUALISER_BIN` override.

### Verification

- `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml`
  exits 0.
- `cargo check --manifest-path skills/visualisation/visualise/server/Cargo.toml`
  exits 0.
- `mise run test:unit` exits 0 (runs `test.unit.visualiser`).
- `mise run test:integration` exits 0 (runs `test.integration.visualiser`,
  `test.integration.config`, `test.integration.decisions`; includes the new
  Phase 2 `test-launch-server.sh` and `test-stop-server.sh` suites under the
  visualiser component).
- `mise run test` exits 0 (runs unit then integration level in order).
- `invoke test.unit.visualiser` exits 0 in isolation.
- `invoke test.integration.visualiser` exits 0 in isolation.
- `invoke test.integration.config` exits 0 in isolation.
- `invoke test.integration.decisions` exits 0 in isolation.
- `ACCELERATOR_VISUALISER_BIN=$(pwd)/skills/visualisation/visualise/server/target/release/accelerator-visualiser
  \ skills/visualisation/visualise/scripts/launch-server.sh` outputs a real
  `**Visualiser URL**: http://127.0.0.1:<port>` line; `curl -f
  http://127.0.0.1:<port>/` returns 200 with the placeholder body.
- After the above, `skills/visualisation/visualise/scripts/stop-server.sh`
  (invoked from the same project root) prints `{"status": "stopped"}` on stdout
  and the server is no longer reachable.
- `jq -e '.version, .binaries["darwin-arm64"], .binaries["darwin-x64"],
  .binaries["linux-arm64"], .binaries["linux-x64"]'
  skills/visualisation/visualise/bin/checksums.json` exits 0.
- `grep -F 'skills/visualisation/visualise/bin/accelerator-visualiser-*'
  .gitignore` matches.
- `grep -F 'skills/visualisation/visualise/server/target' .gitignore` matches.

## What We're NOT Doing

Explicitly out of scope for Phase 2.

- **No FileDriver, Indexer, or file-watching** — Phase 3. No doc-type scanning,
  no frontmatter parsing, no ETag cache.
- **No SSE hub** — Phase 4. The `/api/events` endpoint does not exist.
- **No frontend scaffold, no `rust-embed`, no `frontend/` directory** — Phase 5.
  Phase 2 serves a single placeholder string at `GET /`, directly from a
  hardcoded handler.
- **No API routes** (`/api/types`, `/api/docs`, `/api/lifecycle`, `/api/events`)
  — Phases 3–4.
- **No PATCH / write path / YAML-aware patcher** — Phase 8.
- **No path-safety guard** (`canonicalize` + prefix check) — unnecessary here
  because no route reads files by path. Lands in Phase 3 alongside the file
  driver.
- **No real checksums in `checksums.json`** — Phase 12. The manifest is
  committed with deliberately-invalid placeholder hashes so the schema and path
  land now; the release pipeline is the sole author of real values.
- **No release workflow, no `cargo zigbuild`, no cross-compile infrastructure**
  — Phase 12. Phase 2 contributors build locally for their own arch with `cargo
  build --release` and export `ACCELERATOR_VISUALISER_BIN` to point at their own
  target.
- **No GitHub Release asset upload** — Phase 12.
- **No `rust-embed`, no `embed-dist` / `dev-frontend` features** — Phase 5
  introduces those when the frontend lands.
- **No CHANGELOG entry, no plugin version bump** — this is feature work, not
  release-time. The version bump happens when Phase 12 ships.
- **No authentication or CORS changes beyond `127.0.0.1` binding** — spec defers
  any multi-host story.
- **No pre-release binary policy decision** — Phase 12.
- **No Windows or WSL support** — consistent with accelerator's current
  macOS/Linux scope.
- **No generalisation of the server-info poll to long startup windows** — the
  `5s` timeout from the superpowers precedent is kept.
- **No configurable idle timeout via env var** — 30 min is hardcoded in Phase 2;
  revisit if real use shows it's wrong.
- **No logging rotation**. The 5MB rotation mentioned in the spec's
  Observability section lands in Phase 10.
- **No `X-Accelerator-Visualiser` response header** — Phase 10 (version
  surfacing).
- **No init-sentinel check** (existence of `<meta/tmp>/.gitignore`) — Phase 10.
- **No `test-config.sh` invariant updates**. Adding the server does not add any
  new context-injection skill, so the hard-coded `"14"` counts and the
  `CONTEXT_SKILLS` / `ALL_SKILLS` arrays from Phase 1.4 are untouched. The
  `visualiser.binary` config key is opt-in and read only from the launcher — it
  doesn't flow through the context-injection preprocessors.
- **No new `scripts/config-read-visualiser-binary.sh` wrapper**. The launcher
  reads the key inline via `config-read-value.sh visualiser.binary`, matching
  how other non-path config reads work. A dedicated wrapper would only earn its
  keep if more than one caller needed the key.
- **No schema validation of the config key**. The launcher checks the resolved
  path exists and is executable; it does not probe the binary's identity
  (version, arch, signature). Users who misconfigure get a clear "not
  executable" error; users who point at the wrong arch's binary get a runtime
  failure from the kernel, which surfaces as the server failing to start within
  5s and is already handled by the existing `server-info.json did not appear`
  error path.
- **No restrictions on where `visualiser.binary` is set**. Both
  `.claude/accelerator.md` (team-committed) and `.claude/accelerator.local.md`
  (gitignored, personal) are accepted. The team-committed path is an
  arbitrary-exec vector by design — the plugin treats `.claude/accelerator.md`
  as trusted on par with the rest of the repo tree, since it is
  version-controlled and review-gated. A deliberate trust decision, not an
  oversight: the accelerator has no boundary between "config that's just data"
  and "config that chooses code to execute" in the first place (shell scripts,
  agent definitions, and hook commands are all committed and executed).
  Restricting this one key would be inconsistent and would also block the
  legitimate use-case of a team that wants a pinned or vendored visualiser
  binary checked into a sibling location.
- **No `tasks/release.py` changes**. The existing release task is not extended
  to build binaries in Phase 2; that's Phase 12.

## Implementation Approach

Nine sub-phases, ordered so each depends only on earlier ones:

1. **Cargo project scaffolding.** `Cargo.toml` with pinned deps, edition `2021`,
   `rust-version = "1.80"`, minimal `main.rs`, `build.rs` stub, `cargo test`
   passes vacuously.
2. **Config ingestion module.** `config.rs` with a `Config` struct matching the
   `config.json` contract from Gap 2. TDD with fixture files in
   `server/tests/fixtures/`.
3. **Server binding + `server-info.json` write.** `axum::Router` with a single
   placeholder route, binds `127.0.0.1:0`, writes `server-info.json` on listen.
   Integration tests use `reqwest::Client` against the real listener to verify
   the placeholder response and the lifecycle-file contents.
4. **Graceful shutdown + `server-stopped.json`.** `tokio::signal` handlers,
   `with_graceful_shutdown`, deterministic shutdown flushes
   `server-stopped.json` with the reason and removes `server-info.json`.
5. **Owner-PID watch + idle timeout.** `tokio::time::interval` loop polling
   `nix::sys::signal::kill(pid, None)` every 60s and checking `last_activity`;
   triggers the same shutdown path on either condition.
6. **Checksums manifest + `.gitignore`.** Commit `bin/checksums.json` with
   placeholder hashes, extend root `.gitignore` with the binary-cache and
   `target/` patterns.
7. **`launch-server.sh` Phase 2 implementation.** Replace the stub with real
   platform detection, binary fetch/verify flow, `config.json` write, owner-PID
   resolution, nohup/disown background, and server-info poll-for-ready. Replace
   `test-launch-server.sh` wholesale — the old sentinel is gone.
8. **`stop-server.sh` + PID reuse in `launch-server.sh`.** New stop script with
   SIGTERM → grace → SIGKILL escalation; launcher detects existing
   `server-info.json` with a live PID and short-circuits to URL print. Add
   `test-stop-server.sh`.
9. **SKILL.md update + `mise run test:cargo`.** Replace the Availability block
   with real guidance; add `mise run test:cargo`; make `mise run test` depend on
   both integration suites.

TDD discipline per sub-phase:

- **Red**: add a test (cargo or bash) that fails because the code doesn't exist
  or has a wrong value. Commit in a distinct `jj` revision where practical.
- **Green**: implement the minimum to make tests pass.
- **Refactor**: clean up only if the implementation is awkward.

Where tests and implementation land in the same commit, the sub-phase's Manual
Verification calls out a mutation smoke test (temporarily break the
implementation; observe the suite fail; restore) so the tests are proven to
exercise what they claim.

---

## Phase 2.1: Cargo project scaffolding

### Overview

Create the Rust project skeleton. `Cargo.toml` pins dependencies,
`rust-version`, and edition; `main.rs` is minimal (logs startup and exits 0 for
now); `build.rs` is a one-line stub. `cargo test` runs green (vacuously — no
tests yet) and `cargo check` succeeds so the tooling is provably wired up before
any behaviour lands.

### Changes Required

#### 1. Cargo manifest

**File**: `skills/visualisation/visualise/server/Cargo.toml` **Changes**: New
file.

```toml
[package]
name = "accelerator-visualiser"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
description = "Meta-directory visualiser server for the accelerator Claude Code plugin"
publish = false

[[bin]]
name = "accelerator-visualiser"
path = "src/main.rs"

[dependencies]
axum = { version = "0.7", default-features = false, features = ["http1", "tokio"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal", "sync", "time", "net", "fs"] }
tower-http = { version = "0.5", features = ["trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tempfile = "3"
nix = { version = "0.28", features = ["signal", "process"] }
anyhow = "1"
thiserror = "1"
libc = "0.2"

[dev-dependencies]
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time", "process"] }
assert_cmd = "2"
predicates = "3"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
opt-level = 3
```

Notes:

- `rust-version = "1.80"` pins MSRV (Gap 5). `cargo check` fails if the
  toolchain is older; a clear signal for contributors.
- `axum` uses default-features disabled so we explicitly pull only `http1` and
  `tokio` — SSE support will be re-enabled by toggling a feature flag in Phase 4
  when it's actually used.
- `reqwest` is a dev-dep only; the Rust binary never makes outbound HTTP. Used
  in integration tests that hit the live listener.
- `tempfile` supports the atomic-write pattern (sibling tempfile
  + rename) that Phase 8's patcher will use; it's also useful in Phase 2
    integration tests that need a tmp directory.
- `nix` with `signal` and `process` features is for the owner-PID watch
  (`kill(pid, None)`).
- No `reqwest` runtime dep → no TLS chain, no ca-certificates bundle; the binary
  stays small.

#### 2. `lib.rs` + minimal `main.rs` split

The crate ships as a library + binary pair from day one. All module declarations
live in `lib.rs` with `pub mod`, and `main.rs` is a thin binary entry point that
imports from the library. Later sub-phases add tests under `server/tests/*.rs`
that need to `use accelerator_visualiser::{config, server, …}`; doing the split
now means each new module added in 2.2–2.5 goes into `lib.rs` in the obvious
place rather than being migrated out of `main.rs` later.

**File**: `skills/visualisation/visualise/server/src/lib.rs` **Changes**: New
file.

```rust
//! Meta visualiser server — library crate.
//!
//! The binary (`src/main.rs`) is a thin entry point; all logic
//! lives in the modules declared here. Integration tests under
//! `server/tests/*.rs` consume these modules directly.

// Modules are added in later sub-phases:
// 2.2 → config
// 2.3 → server
// 2.4 → shutdown  (ShutdownReason + signal handlers)
// 2.5 → activity, lifecycle
```

**File**: `skills/visualisation/visualise/server/src/main.rs` **Changes**: New
file.

```rust
//! Meta visualiser server entry point.
//!
//! Phase 2 hosts only the bootstrap: read the config.json path
//! from argv, initialise tracing, bind a listener, write
//! server-info.json, and wait on shutdown signals. Indexing,
//! file-watching, SSE, and API routes all land in later phases.

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "accelerator-visualiser starting");
    ExitCode::SUCCESS
}
```

`[[bin]]` in `Cargo.toml` already points at `src/main.rs`; adding `lib.rs` under
the same crate name is picked up by Cargo automatically (no `[lib]` section
needed — the default target discovery finds it). The version is read from
`CARGO_PKG_VERSION` so release tooling can bump the version without touching
source.

#### 3. `build.rs` stub

**File**: `skills/visualisation/visualise/server/build.rs` **Changes**: New
file.

```rust
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
```

Deliberately trivial. D10's `rust-embed` existence guard lands in Phase 5.
Having `build.rs` from day one avoids a later Cargo dirty-rebuild triggered by
adding a build script.

#### 4. `.gitignore` addition for Cargo artefacts

**File**: `.gitignore` **Changes**: Append:

```gitignore
skills/visualisation/visualise/server/target/
```

Only the `target/` dir. The per-arch release binaries live under `bin/` and are
gitignored separately in Phase 2.6.

### Success Criteria

#### Automated Verification

- [x]  `cargo check --manifest-path
  skills/visualisation/visualise/server/Cargo.toml` exits 0.
- [x]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml` exits 0 with "0 tests"
  (vacuous pass).
- [x]  `cargo build --release --manifest-path
  skills/visualisation/visualise/server/Cargo.toml` exits 0 and produces
  `server/target/release/accelerator-visualiser`.
- [x]  `[ -x
  skills/visualisation/visualise/server/target/release/accelerator-visualiser ]`
  ( after build).
- [x] Both `src/lib.rs` and `src/main.rs` exist; `cargo test --manifest-path …`
  compiles the library target in addition to the binary.
- [x] `grep -F 'skills/visualisation/visualise/server/target/' .gitignore`
  matches.
- [x]  `awk '/rust-version/' skills/visualisation/visualise/server/Cargo.toml |
  grep -F '"1.85"'` matches (bumped from planned 1.80 — edition2024 required by
  modern crate ecosystem).
- [x]  `awk '/edition/' skills/visualisation/visualise/server/Cargo.toml | grep
  -F '"2021"'` matches.

#### Manual Verification

- [ ] Running
  `skills/visualisation/visualise/server/target/release/accelerator-visualiser`
  ( no args) exits 0 and logs one JSON line to stderr mentioning
  `accelerator-visualiser starting`.
- [ ] `cargo build` with a deliberately-older toolchain ( `rustup run 1.75.0
  cargo build --manifest-path .../Cargo.toml`) fails with a clear "rustc version
  too old" message (MSRV enforcement).

---

## Phase 2.2: Config ingestion module

### Overview

Define the `config.json` contract (Gap 2) as a Rust `Config` struct with
`serde::Deserialize`, and wire it into `main.rs` so the binary reads its
config-file path from argv and fails early on invalid input. No server behaviour
changes yet — the binary parses the config, logs it, and exits.

### Changes Required

#### 1. Fixture files (TDD)

**File**:
`skills/visualisation/visualise/server/tests/fixtures/config.valid.json`
**Changes**: New file.

```json
{
  "plugin_root": "/abs/path/to/plugin",
  "plugin_version": "1.19.0-pre.2",
  "tmp_path": "/abs/path/to/project/meta/tmp/visualiser",
  "host": "127.0.0.1",
  "owner_pid": 0,
  "owner_start_time": null,
  "log_path": "/abs/path/to/project/meta/tmp/visualiser/server.log",
  "doc_paths": {
    "decisions": "/abs/path/to/project/meta/decisions",
    "tickets": "/abs/path/to/project/meta/tickets",
    "plans": "/abs/path/to/project/meta/plans",
    "research": "/abs/path/to/project/meta/research",
    "review_plans": "/abs/path/to/project/meta/reviews/plans",
    "review_prs": "/abs/path/to/project/meta/reviews/prs",
    "validations": "/abs/path/to/project/meta/validations",
    "notes": "/abs/path/to/project/meta/notes",
    "prs": "/abs/path/to/project/meta/prs"
  },
  "templates": {
    "adr": {
      "config_override": null,
      "user_override": "/abs/path/to/project/meta/templates/adr.md",
      "plugin_default": "/abs/path/to/plugin/templates/adr.md"
    },
    "plan": {
      "config_override": null,
      "user_override": "/abs/path/to/project/meta/templates/plan.md",
      "plugin_default": "/abs/path/to/plugin/templates/plan.md"
    },
    "research": {
      "config_override": null,
      "user_override": "/abs/path/to/project/meta/templates/research.md",
      "plugin_default": "/abs/path/to/plugin/templates/research.md"
    },
    "validation": {
      "config_override": null,
      "user_override": "/abs/path/to/project/meta/templates/validation.md",
      "plugin_default": "/abs/path/to/plugin/templates/validation.md"
    },
    "pr-description": {
      "config_override": null,
      "user_override": "/abs/path/to/project/meta/templates/pr-description.md",
      "plugin_default": "/abs/path/to/plugin/templates/pr-description.md"
    }
  }
}
```

**File**:
`skills/visualisation/visualise/server/tests/fixtures/config.missing-required.json`
**Changes**: New file (missing `plugin_root` — used to assert hard failure).

```json
{
  "plugin_version": "1.19.0-pre.2",
  "tmp_path": "/tmp/visualiser",
  "host": "127.0.0.1",
  "owner_pid": 0,
  "log_path": "/tmp/visualiser/server.log",
  "doc_paths": {},
  "templates": {}
}
```

**File**:
`skills/visualisation/visualise/server/tests/fixtures/config.optional-override-null.json`
**Changes**: New file (all `config_override` entries null — the common case in
Phase 2 dev).

Identical to `config.valid.json` — kept as a separate fixture so future fixtures
can encode specific scenarios (eg. one `config_override` populated) without
disturbing the baseline.

#### 2. Config module

**File**: `skills/visualisation/visualise/server/src/config.rs` **Changes**: New
file.

```rust
//! Typed view over the config.json produced by launch-server.sh.
//!
//! Schema decided in the Phase 2 plan (research Gap 2). Any
//! change here is a breaking change against the preprocessor —
//! keep fields in sync with scripts/launch-server.sh's JSON
//! writer.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub plugin_root: PathBuf,
    pub plugin_version: String,
    pub tmp_path: PathBuf,
    pub host: String,
    pub owner_pid: i32,
    /// Start-time of the owner process, seconds-since-epoch.
    /// Optional — the launcher writes it when obtainable (macOS
    /// `ps -p … -o lstart=`, Linux `/proc/<pid>/stat`).
    /// `None` disables the owner-PID identity cross-check in the
    /// lifecycle watch (the bare PID probe still runs).
    #[serde(default)]
    pub owner_start_time: Option<u64>,
    pub log_path: PathBuf,
    pub doc_paths: HashMap<String, PathBuf>,
    pub templates: HashMap<String, TemplateTiers>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateTiers {
    pub config_override: Option<PathBuf>,
    pub user_override: PathBuf,
    pub plugin_default: PathBuf,
}

impl Config {
    pub fn from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let bytes = std::fs::read(path)
            .map_err(|source| ConfigError::Read { path: path.to_path_buf(), source })?;
        serde_json::from_slice(&bytes)
            .map_err(|source| ConfigError::Parse { path: path.to_path_buf(), source })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("failed to parse config {path}: {source}")]
    Parse { path: PathBuf, source: serde_json::Error },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        p.push(name);
        p
    }

    #[test]
    fn parses_valid_config() {
        let c = Config::from_path(&fixture("config.valid.json")).expect("valid");
        assert_eq!(c.plugin_version, "1.19.0-pre.2");
        assert_eq!(c.host, "127.0.0.1");
        assert_eq!(c.owner_pid, 0);
        assert_eq!(c.doc_paths.len(), 9);
        assert!(c.doc_paths.contains_key("decisions"));
        assert!(c.doc_paths.contains_key("review_plans"));
        assert!(c.doc_paths.contains_key("review_prs"));
        assert_eq!(c.templates.len(), 5);
        let adr = c.templates.get("adr").expect("adr tier");
        assert!(adr.config_override.is_none());
        assert!(adr.user_override.ends_with("adr.md"));
        assert!(adr.plugin_default.ends_with("adr.md"));
    }

    #[test]
    fn rejects_missing_required_field() {
        let err = Config::from_path(&fixture("config.missing-required.json"))
            .expect_err("missing plugin_root must fail");
        assert!(
            matches!(err, ConfigError::Parse { .. }),
            "expected Parse error, got {err:?}"
        );
    }

    #[test]
    fn rejects_nonexistent_path() {
        let err = Config::from_path(std::path::Path::new("/nonexistent/config.json"))
            .expect_err("missing file must fail");
        assert!(matches!(err, ConfigError::Read { .. }));
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        // deny_unknown_fields guards against preprocessor/server
        // schema drift — a typo like `doc_path` (singular) would
        // otherwise silently produce an empty map.
        let json = r#"{
            "plugin_root": "/p", "plugin_version": "0.0.0",
            "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
            "log_path": "/l", "doc_paths": {}, "templates": {},
            "doc_path": {"decisions": "/typo"}
        }"#;
        let err = serde_json::from_str::<Config>(json)
            .expect_err("unknown field must fail");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn config_override_can_be_populated() {
        let json = r#"{
            "plugin_root": "/p",
            "plugin_version": "0.0.0",
            "tmp_path": "/t",
            "host": "127.0.0.1",
            "owner_pid": 1,
            "log_path": "/l",
            "doc_paths": {},
            "templates": {
                "adr": {
                    "config_override": "/custom/adr.md",
                    "user_override": "/u/adr.md",
                    "plugin_default": "/d/adr.md"
                }
            }
        }"#;
        let c: Config = serde_json::from_str(json).expect("parse");
        let adr = c.templates.get("adr").unwrap();
        assert_eq!(adr.config_override.as_deref().unwrap(), std::path::Path::new("/custom/adr.md"));
    }
}
```

<!-- thiserror is pinned in Cargo.toml alongside the other deps in Phase 2.1.
-->

#### 3. Wire config into the library + binary

**File**: `skills/visualisation/visualise/server/src/lib.rs` **Changes**: Add
`pub mod config;`.

**File**: `skills/visualisation/visualise/server/src/main.rs` **Changes**: Full
replacement.

```rust
use std::process::ExitCode;

use accelerator_visualiser::config::Config;
use clap::Parser;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "accelerator-visualiser", version, about)]
struct Cli {
    /// Path to the config.json written by launch-server.sh.
    #[arg(long = "config", value_name = "PATH")]
    config: std::path::PathBuf,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = match Config::from_path(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "failed to load config");
            return ExitCode::from(2);
        }
    };

    info!(
        plugin_version = %cfg.plugin_version,
        host = %cfg.host,
        owner_pid = cfg.owner_pid,
        doc_paths = cfg.doc_paths.len(),
        templates = cfg.templates.len(),
        "config loaded"
    );
    ExitCode::SUCCESS
}
```

#### 4. `assert_cmd`-based end-to-end config test

**File**: `skills/visualisation/visualise/server/tests/config_cli.rs`
**Changes**: New file.

```rust
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn exits_2_when_config_missing() {
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", "/nonexistent/config.json"]);
    cmd.assert().code(2);
}

#[test]
fn parses_fixture_config_and_exits_success() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config.valid.json");
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", fixture.to_str().unwrap()]);
    cmd.assert().success();
}
```

### Success Criteria

#### Automated Verification

- [x]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml --lib config::tests` runs ≥4
  tests, all pass.
- [x]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml --test config_cli` runs 2
  tests, both pass.
- [x] Binary exits with code 2 on missing config file (provable by `cargo run --
  --config /nonexistent.json; echo $?`).
- [x] `tests/fixtures/config.valid.json` parses without modification.

#### Manual Verification

- [x] Tests written first (red-then-green). Either (a) test file(s) committed in
  a `jj` revision before `config.rs`, or (b) bundled commit + mutation smoke
  test: temporarily rename `plugin_root` → `plugin_roots` in `config.rs`, run
  `cargo test` and observe failure, restore.

---

## Phase 2.3: Server binding and lifecycle-file writes

### Overview

Bind a TCP listener on `127.0.0.1:0`, serve a single `GET /` placeholder
protected by default-deny HTTP middleware (request-body size, timeout,
Host-header allowlist), and write two lifecycle files atomically once the
listener is live: `server-info.json` (URL + PID + start_time — the authoritative
identity record) and `server.pid` (a newline-terminated PID string for tooling
that doesn't parse JSON). Both writes happen **inside the Rust server** — not
the bash launcher — so the launcher's `nohup`/`disown` race cannot leave a stale
PID file for a server that never started (see Rec 3 discussion in the plan
review).

Safety gate: `cfg.host` is validated against `IpAddr::is_loopback` before
binding; a non-loopback host produces a clean startup failure rather than
silently exposing the server on the LAN.

No graceful shutdown yet — the test harness kills the process with SIGTERM at
end-of-test; Phase 2.4 adds signal handling.

### Changes Required

#### 1. Server module

**File**: `skills/visualisation/visualise/server/src/server.rs` **Changes**: New
file.

```rust
//! axum server bootstrap. Binds a random port on 127.0.0.1,
//! writes server-info.json + server.pid once the listener is
//! live, and serves a single placeholder route behind a
//! default-deny middleware stack. Signal handling and
//! owner-PID / idle watches land in later phases.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::{extract::Request, http::StatusCode, middleware::{self, Next}, response::Response, routing::get, Router};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

use crate::config::Config;

/// 1 MiB cap on request bodies. The placeholder route never reads a body,
/// but the cap is the default-deny baseline every later phase inherits.
const REQUEST_BODY_LIMIT: usize = 1_048_576;

/// 30s request timeout — long enough for markdown rendering and diff
/// responses in later phases, short enough that a stuck handler can't
/// pin a worker forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AppState {
    pub cfg: Arc<Config>,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub version: String,
    pub pid: i32,
    /// Process start-time stamp, used for PID-identity checks.
    /// Seconds-since-epoch from `SystemTime::now()` at bind time.
    /// Nullable because some future transport (remote agents,
    /// containerised PID namespaces) may not produce one. The
    /// launcher's reuse probe and `stop-server.sh` fall back to
    /// bare PID comparison when absent — less safe, but keeps the
    /// path working.
    pub start_time: Option<u64>,
    pub host: String,
    pub port: u16,
    pub url: String,
    pub log_path: PathBuf,
    pub tmp_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("host {0} is not a loopback address")]
    NonLoopbackHost(String),
    #[error("failed to bind listener on {addr}: {source}")]
    Bind { addr: String, source: std::io::Error },
    #[error("failed to write lifecycle file {path}: {source}")]
    LifecycleWrite { path: PathBuf, source: std::io::Error },
    #[error(transparent)]
    Serve(#[from] std::io::Error),
}

pub async fn run(cfg: Config, info_path: &Path) -> Result<(), ServerError> {
    // Defence-in-depth: refuse any non-loopback host even though
    // the launcher always writes 127.0.0.1. See the review's
    // Security finding on DNS-rebinding baseline.
    let host: IpAddr = cfg.host.parse()
        .map_err(|_| ServerError::NonLoopbackHost(cfg.host.clone()))?;
    if !host.is_loopback() {
        return Err(ServerError::NonLoopbackHost(cfg.host.clone()));
    }

    let state = Arc::new(AppState { cfg: Arc::new(cfg) });
    let app = Router::new()
        .route("/", get(placeholder_root))
        .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .layer(middleware::from_fn(host_header_guard))
        .with_state(state.clone());

    let bind_addr = SocketAddr::new(host, 0);
    let listener = TcpListener::bind(bind_addr).await
        .map_err(|source| ServerError::Bind { addr: bind_addr.to_string(), source })?;
    let local = listener.local_addr()
        .map_err(|source| ServerError::Bind { addr: bind_addr.to_string(), source })?;

    let info = ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        pid: std::process::id() as i32,
        start_time: process_start_time(std::process::id() as i32),
        host: state.cfg.host.clone(),
        port: local.port(),
        url: format!("http://{}:{}", state.cfg.host, local.port()),
        log_path: state.cfg.log_path.clone(),
        tmp_path: state.cfg.tmp_path.clone(),
    };

    // Write PID file first (smaller artefact, faster to land) then
    // server-info.json. Both atomic-rename; order matters only to
    // the launcher's poll-for-readiness, which keys on
    // server-info.json.
    let pid_path = info_path.with_file_name("server.pid");
    write_pid_file(&pid_path, info.pid)
        .map_err(|source| ServerError::LifecycleWrite { path: pid_path.clone(), source })?;
    write_server_info(info_path, &info)
        .map_err(|source| ServerError::LifecycleWrite { path: info_path.to_path_buf(), source })?;
    info!(url = %info.url, pid = info.pid, start_time = ?info.start_time, "server-started");

    axum::serve(listener, app).await?;
    Ok(())
}

fn write_server_info(path: &Path, info: &ServerInfo) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::InvalidInput, "server-info.json path has no parent"))?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, info)?;
    std::io::Write::write_all(&mut tmp, b"\n")?;
    // Tighten to owner-only read/write — the file reveals listener
    // URL and process identity, and lives under the user's project
    // tree where other local accounts may have traversal.
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

fn write_pid_file(path: &Path, pid: i32) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::InvalidInput, "server.pid path has no parent"))?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    writeln!(tmp.as_file_mut(), "{pid}")?;
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

/// Seconds-since-epoch at which `pid` started, if obtainable.
/// macOS: `ps -p <pid> -o lstart=` parsed to epoch.
/// Linux: `/proc/<pid>/stat` field 22 (clock ticks since boot) +
///        `/proc/stat` `btime` → absolute epoch.
/// Returns None on any parse or IO failure — the caller falls back
/// to bare PID comparison.
pub(crate) fn process_start_time(pid: i32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // Field 22 (1-indexed) is starttime in clock ticks since boot.
        // The command field (field 2) is wrapped in parens and may
        // contain spaces, so skip past the last ')' before splitting.
        let tail = stat.rsplit_once(')').map(|(_, t)| t)?;
        let starttime_ticks: u64 = tail.split_whitespace().nth(19)?.parse().ok()?;
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
        if hz == 0 { return None; }
        let btime_line = std::fs::read_to_string("/proc/stat").ok()?
            .lines().find(|l| l.starts_with("btime "))?.to_string();
        let btime: u64 = btime_line.split_whitespace().nth(1)?.parse().ok()?;
        Some(btime + starttime_ticks / hz)
    }
    #[cfg(target_os = "macos")]
    {
        // Delegate to BSD `date -j -f` so Rust and shell
        // (scripts/_launcher-helpers.sh `start_time_of`) produce
        // byte-identical epoch-seconds. The alternative — parsing
        // the wall-clock components manually — would diverge from
        // the shell whenever the host isn't in UTC, because BSD
        // `date -j` interprets the input in the local timezone.
        //
        // Flow: `ps -p <pid> -o lstart=` prints the local-TZ start
        // time (e.g. "Sun Jan  1 12:34:56 2026"), `date -j -f
        // "%a %b %d %H:%M:%S %Y" <that> +%s` converts back to epoch
        // in the same local TZ, and both Rust and shell see the
        // same value. No chrono/manual TZ math.
        let ps = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "lstart="])
            .output().ok()?;
        if !ps.status.success() { return None; }
        let lstart = String::from_utf8(ps.stdout).ok()?;
        let lstart = lstart.trim();
        if lstart.is_empty() { return None; }
        // Normalise whitespace — BSD ps pads the day field with a
        // leading space on single-digit days ("Sun Jan  1 …"),
        // which `date -j -f` handles, but any stray double-spaces
        // would trip a strict parse. date -j -f is tolerant here.
        let date = std::process::Command::new("date")
            .args(["-j", "-f", "%a %b %d %H:%M:%S %Y", lstart, "+%s"])
            .output().ok()?;
        if !date.status.success() { return None; }
        let epoch = String::from_utf8(date.stdout).ok()?;
        epoch.trim().parse::<u64>().ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

async fn host_header_guard(req: Request, next: Next) -> Result<Response, StatusCode> {
    // Defence-in-depth against DNS-rebinding: only accept the
    // Host header values we actually bind to. The listener address
    // is 127.0.0.1:<port>; browsers legitimately send localhost:<port>
    // too.
    let host = req.headers().get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (host_part, _) = host.split_once(':').unwrap_or((host, ""));
    if host_part == "127.0.0.1" || host_part == "localhost" || host_part.is_empty() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

async fn placeholder_root() -> &'static str {
    concat!(
        "accelerator-visualiser ",
        env!("CARGO_PKG_VERSION"),
        " — Phase 2 bootstrap. UI lands in a later phase.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn minimal_config(tmp: &Path) -> Config {
        Config {
            plugin_root: tmp.to_path_buf(),
            plugin_version: "test".into(),
            tmp_path: tmp.to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.join("server.log"),
            doc_paths: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    #[test]
    fn write_server_info_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");
        let info = ServerInfo {
            version: "0.0.0-test".into(),
            pid: 42,
            start_time: Some(1_700_000_000),
            host: "127.0.0.1".into(),
            port: 1234,
            url: "http://127.0.0.1:1234".into(),
            log_path: dir.path().join("server.log"),
            tmp_path: dir.path().to_path_buf(),
        };
        write_server_info(&info_path, &info).unwrap();
        let bytes = std::fs::read(&info_path).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["port"], 1234);
        assert_eq!(v["url"], "http://127.0.0.1:1234");
        assert_eq!(v["pid"], 42);
        assert_eq!(v["start_time"], 1_700_000_000);
        // 0o600 perms
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&info_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "server-info.json must be owner-only");
    }

    #[test]
    fn write_pid_file_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("server.pid");
        write_pid_file(&p, 9999).unwrap();
        let content = std::fs::read_to_string(&p).unwrap();
        assert_eq!(content.trim(), "9999");
        assert!(content.ends_with('\n'));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn process_start_time_is_stable_for_same_pid() {
        let me = std::process::id() as i32;
        let first = process_start_time(me);
        let second = process_start_time(me);
        assert_eq!(first, second, "start_time must be stable for the same PID");
        // On supported platforms, we should get Some; on others None.
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(first.is_some(), "start_time should resolve on Linux/macOS");
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn process_start_time_matches_shell_helper() {
        // Rust and the shell helper (_launcher-helpers.sh
        // `start_time_of`) must produce byte-identical values for
        // the same PID — otherwise the (pid, start_time) identity
        // cross-check fails across launcher/server/stop boundaries.
        // This test pins the agreement.
        let me = std::process::id() as i32;
        let rust_value = process_start_time(me).expect("rust produces a value");

        let helper = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../scripts/_launcher-helpers.sh");
        assert!(helper.exists(), "helper not found at {}", helper.display());

        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(format!(
                "source {} && start_time_of {}",
                helper.to_str().unwrap(), me
            ))
            .output()
            .expect("spawn bash helper");
        assert!(output.status.success(),
            "helper failed: {}", String::from_utf8_lossy(&output.stderr));
        let shell_value: u64 = String::from_utf8(output.stdout).unwrap()
            .trim().parse().expect("helper emits an integer");

        assert_eq!(rust_value, shell_value,
            "Rust start_time ({}) must equal shell start_time ({}) — \
             Rust/shell divergence breaks PID-identity cross-check on this host",
            rust_value, shell_value);
    }

    #[tokio::test]
    async fn non_loopback_host_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(dir.path());
        cfg.host = "0.0.0.0".into();
        let err = run(cfg, &dir.path().join("server-info.json")).await.unwrap_err();
        assert!(matches!(err, ServerError::NonLoopbackHost(_)), "got {err:?}");
    }

    // Harness for binding-and-server tests. Spawns `run()` on a
    // background task; test gets the final URL by reading the info
    // file it wrote.
    #[tokio::test]
    async fn serves_placeholder_root_and_writes_info() {
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");
        let cfg = minimal_config(dir.path());

        let info_path_clone = info_path.clone();
        let handle = tokio::spawn(async move {
            run(cfg, &info_path_clone).await.unwrap();
        });

        // Poll for server-info.json to appear (bounded).
        let start = std::time::Instant::now();
        loop {
            if info_path.exists() {
                break;
            }
            if start.elapsed().as_secs() > 5 {
                panic!("server-info.json did not appear in 5s");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        let info: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&info_path).unwrap()).unwrap();
        let url = info["url"].as_str().unwrap().to_string();

        let body = reqwest::get(&url).await.unwrap().text().await.unwrap();
        assert!(body.starts_with("accelerator-visualiser "));
        assert!(body.contains("Phase 2 bootstrap"));

        handle.abort();
    }
}
```

<!-- anyhow is pinned in Cargo.toml alongside the other deps in Phase 2.1. Note:
server::run now returns ServerError (a concrete thiserror enum) rather than
anyhow::Result — typed errors in the server surface, anyhow kept for main.rs and
tests that don't care about variant shape. -->

#### 2. Wire server into the library + binary

**File**: `skills/visualisation/visualise/server/src/lib.rs` **Changes**: Add
`pub mod server;`.

**File**: `skills/visualisation/visualise/server/src/main.rs` **Changes**:
Extend to call `server::run`.

```rust
use std::process::ExitCode;

use accelerator_visualiser::{config::Config, server};
use clap::Parser;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "accelerator-visualiser", version, about)]
struct Cli {
    /// Path to the config.json written by launch-server.sh.
    #[arg(long = "config", value_name = "PATH")]
    config: std::path::PathBuf,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = match Config::from_path(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "failed to load config");
            return ExitCode::from(2);
        }
    };
    let info_path = cfg.tmp_path.join("server-info.json");
    info!(config = %cli.config.display(), info_path = %info_path.display(), "bootstrapping server");

    if let Err(e) = server::run(cfg, &info_path).await {
        error!(error = %e, "server error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
```

### Success Criteria

#### Automated Verification

- [x]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml server::tests` passes.
- [x] `serves_placeholder_root_and_writes_info` specifically passes (covers
  binding, info-file write, HTTP response body).
- [x] Manual invocation: with a prepared config.json, the binary stays in the
  foreground (no shutdown yet), serves 200 at `/`, and writes
  `<tmp>/server-info.json` with the exact port it bound. Terminate via external
  SIGTERM.

#### Manual Verification

- [x] Tests written first. Either separate `jj` commit for the test, or mutation
  smoke test: break `url` composition (e.g. swap host and port in the`format!`),
  observe `serves_placeholder_root_and_writes_info` fail, restore.

---

## Phase 2.4: Graceful shutdown and `server-stopped.json`

### Overview

Wire `axum::serve(...).with_graceful_shutdown(...)` to a future that resolves
when the server should exit. Add SIGTERM and SIGINT signal handlers through
`tokio::signal::unix`. On shutdown, write `server-stopped.json` with `{reason,
timestamp}` first, then remove `server-info.json`, and let `axum` drain
in-flight connections. Order is load-bearing: the post-shutdown invariant
(spec's Desired End State #4 — "server-stopped.json remains after shutdown") is
preserved even if one of the two filesystem operations fails.

### Changes Required

#### 1. `ShutdownReason` lives in its own module

`ShutdownReason` is shared between `server` (for signal handlers) and
`lifecycle` (for owner-PID and idle triggers). Hosting it in a neutral
`shutdown` module avoids a `lifecycle → server` back-reference and keeps
`lifecycle` testable without pulling in the entire axum/HTTP surface.

**File**: `skills/visualisation/visualise/server/src/shutdown.rs` **Changes**:
New file.

```rust
//! Shared shutdown plumbing: the reason enum and the mpsc
//! channel type used to converge multiple triggers into one
//! deterministic shutdown path. Neither `server` nor
//! `lifecycle` depends on the other; both depend on this.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShutdownReason {
    Sigterm,
    Sigint,
    OwnerPidExited,
    IdleTimeout,
    StartupFailure,
    ForcedSigkill,
}
```

`ForcedSigkill` is emitted by `stop-server.sh` when it has to escalate to `kill
-9` (the server can't write the stopped file itself in that case, so the shell
script synthesises it — see Phase 2.8).

#### 2. Shutdown machinery in `server.rs`

**File**: `skills/visualisation/visualise/server/src/server.rs` **Changes**:
Extend the module.

Add at the top of the module:

```rust
use tokio::sync::mpsc;

use crate::shutdown::ShutdownReason;
```

Extend `run` with graceful-shutdown wiring. This is an **additive change against
the Phase 2.3 body** — the loopback guard, middleware stack
(`RequestBodyLimitLayer`, `TimeoutLayer`, `host_header_guard`), `write_pid_file`
call, `ServerInfo.start_time` field, and `ServerError` return type are all
retained from Phase 2.3. Only the pieces below are new or modified.

Modify: the bare `axum::serve(listener, app).await?;` line at the end of Phase
2.3's `run` is replaced by the `mpsc` setup + signal handler spawn + shutdown
future + `with_graceful_shutdown` call shown below.

Insert immediately after the `info!(url = %info.url, pid = info.pid, start_time
= ?info.start_time, "server-started");` line (at the end of Phase 2.3's `run`):

```rust
    let (tx, mut rx) = mpsc::channel::<ShutdownReason>(4);
    spawn_signal_handlers(tx.clone());

    let info_path = info_path.to_path_buf();
    let pid_path = info_path.with_file_name("server.pid");
    let stopped_path = info_path.with_file_name("server-stopped.json");
    let shutdown_signal = async move {
        // `rx.recv()` only returns None if every Sender has been
        // dropped before producing a reason — a programming bug,
        // not a real shutdown. Distinguish it via the dedicated
        // `StartupFailure` variant so the audit trail records the
        // anomaly instead of falsely attributing it to SIGTERM.
        let reason = rx.recv().await.unwrap_or(ShutdownReason::StartupFailure);
        info!(?reason, "shutdown requested");
        // Order matters: write server-stopped.json first, then
        // remove server-info.json + server.pid only if the stopped
        // write succeeded. If the stopped write fails (disk-full,
        // read-only FS, EXDEV), leave info.json + server.pid in
        // place — the launcher's stale-PID reuse path treats that
        // as "previous instance left state behind" and recovers
        // cleanly on next launch. The reverse order, or
        // unconditional removal, yields a {no info, no stopped}
        // state that breaks the post-shutdown audit invariant.
        match write_server_stopped(&stopped_path, reason) {
            Ok(()) => {
                let _ = std::fs::remove_file(&info_path);
                let _ = std::fs::remove_file(&pid_path);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "failed to write server-stopped.json; preserving server-info.json and server.pid for next-launch recovery"
                );
            }
        }
    };

    // `ServerError::Serve` has #[from] std::io::Error so `?` routes
    // any serve-time I/O error through the typed error surface.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    Ok(())
}

fn spawn_signal_handlers(tx: mpsc::Sender<ShutdownReason>) {
    use tokio::signal::unix::{signal, SignalKind};
    tokio::spawn({
        let tx = tx.clone();
        async move {
            let mut s = signal(SignalKind::terminate()).expect("SIGTERM handler");
            s.recv().await;
            let _ = tx.send(ShutdownReason::Sigterm).await;
        }
    });
    tokio::spawn(async move {
        let mut s = signal(SignalKind::interrupt()).expect("SIGINT handler");
        s.recv().await;
        let _ = tx.send(ShutdownReason::Sigint).await;
    });
}

fn write_server_stopped(path: &Path, reason: ShutdownReason) -> std::io::Result<()> {
    let record = serde_json::json!({
        "reason": reason,
        // System-clock read — if this errs (pre-epoch clock) we
        // emit a null timestamp rather than a silent 0 that would
        // read as a legitimate 1970-01-01 exit. Callers that
        // sort by timestamp must tolerate null.
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs()),
    });
    let dir = path.parent().ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::InvalidInput, "server-stopped.json path has no parent"))?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, &record)?;
    std::io::Write::write_all(&mut tmp, b"\n")?;
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}
```

#### 3. Tests for shutdown

**File**: `skills/visualisation/visualise/server/src/server.rs` **Changes**:
Extend `#[cfg(test)] mod tests`.

```rust
#[test]
fn write_server_stopped_produces_parseable_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("server-stopped.json");
    write_server_stopped(&p, ShutdownReason::Sigterm).unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(v["reason"], "sigterm");
    assert!(v["timestamp"].as_u64().unwrap() > 0);
}
```

#### 4. Unit test: disk-full-during-shutdown preserves info + pid

When `write_server_stopped` errors (disk-full, read-only filesystem, EXDEV), the
shutdown closure must leave `server-info.json` and `server.pid` in place so the
launcher's stale-PID reuse path can recover on next launch. The easiest way to
force the write to fail in a test is to pre-create a **directory** at the
expected `server-stopped.json` path — the atomic-rename into that path then
fails with `EISDIR`.

Add inside `server::tests`:

```rust
#[tokio::test]
async fn shutdown_preserves_info_when_stopped_write_fails() {
    let dir = tempfile::tempdir().unwrap();
    let info_path = dir.path().join("server-info.json");
    let pid_path = dir.path().join("server.pid");
    let stopped_path = dir.path().join("server-stopped.json");

    // Seed fake lifecycle files as if the server were live.
    std::fs::write(&info_path, r#"{"url":"http://127.0.0.1:1"}"#).unwrap();
    std::fs::write(&pid_path, "9999\n").unwrap();

    // Block the stopped-file write by occupying its path with a
    // non-empty directory that tempfile::persist cannot replace.
    std::fs::create_dir(&stopped_path).unwrap();
    std::fs::write(stopped_path.join("blocker"), "x").unwrap();

    // Drive the same cleanup logic the shutdown closure uses,
    // directly — extracting the match pattern from `server::run`.
    match write_server_stopped(&stopped_path, ShutdownReason::Sigterm) {
        Ok(()) => panic!("expected write_server_stopped to fail"),
        Err(e) => {
            tracing::warn!(error = %e, "expected failure");
            // Per the invariant: do NOT remove info/pid on failure.
        }
    }

    assert!(info_path.exists(),
        "server-info.json must be preserved when stopped-write fails");
    assert!(pid_path.exists(),
        "server.pid must be preserved when stopped-write fails");
}
```

This test doesn't spawn the server — it validates the invariant at the
`write_server_stopped` boundary. An additional end-to-end variant lives in
`server/tests/shutdown.rs`
(`shutdown_preserves_state_on_stopped_write_failure`): that test spawns the
binary, pre-creates a blocker directory at `<tmp>/server-stopped.json`
**before** the binary writes `server-info.json`, sends SIGTERM, and asserts the
process exits cleanly with both `server-info.json` and `server.pid` still on
disk afterwards (the launcher's next invocation will reap them via stale-PID
detection).

#### 5. Integration test: SIGTERM triggers clean shutdown

**File**: `skills/visualisation/visualise/server/tests/shutdown.rs` **Changes**:
New file.

```rust
use std::io::Write;
use std::time::Duration;

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

#[tokio::test]
async fn sigterm_removes_info_writes_stopped_and_exits() {
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("server.log");
    let cfg_path = tmp.path().join("config.json");
    let config = serde_json::json!({
        "plugin_root": tmp.path(),
        "plugin_version": "0.0.0-test",
        "tmp_path": tmp.path(),
        "host": "127.0.0.1",
        "owner_pid": 0,
        "log_path": log,
        "doc_paths": {},
        "templates": {}
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = tokio::process::Command::new(bin)
        .args(["--config", cfg_path.to_str().unwrap()])
        .spawn()
        .expect("spawn");

    // Wait for server-info.json.
    let info_path = tmp.path().join("server-info.json");
    let stopped_path = tmp.path().join("server-stopped.json");
    let start = std::time::Instant::now();
    loop {
        if info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().await.ok();
            panic!("server did not start in 5s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    kill(Pid::from_raw(child.id().unwrap() as i32), Signal::SIGTERM).expect("send SIGTERM");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("server exits on SIGTERM within 30s")
        .expect("wait");
    assert!(status.success(), "server exited with non-zero: {status:?}");

    let pid_path = tmp.path().join("server.pid");
    assert!(!info_path.exists(), "server-info.json must be removed");
    assert!(!pid_path.exists(), "server.pid must be removed");
    assert!(stopped_path.exists(), "server-stopped.json must be written");
    let stopped: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&stopped_path).unwrap()).unwrap();
    assert_eq!(stopped["reason"], "sigterm");
}

#[tokio::test]
async fn server_writes_pid_file_with_its_own_pid() {
    // Launcher no longer writes server.pid — verify the Rust
    // server does it itself before the HTTP listener is serving.
    let tmp = tempfile::tempdir().unwrap();
    let cfg_path = tmp.path().join("config.json");
    let config = serde_json::json!({
        "plugin_root": tmp.path(),
        "plugin_version": "0.0.0-test",
        "tmp_path": tmp.path(),
        "host": "127.0.0.1",
        "owner_pid": 0,
        "log_path": tmp.path().join("server.log"),
        "doc_paths": {},
        "templates": {}
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = tokio::process::Command::new(bin)
        .args(["--config", cfg_path.to_str().unwrap()])
        .spawn()
        .expect("spawn");
    let child_pid = child.id().unwrap() as i32;

    // Wait for server.pid to land.
    let pid_path = tmp.path().join("server.pid");
    let info_path = tmp.path().join("server-info.json");
    let start = std::time::Instant::now();
    loop {
        if pid_path.exists() && info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().await.ok();
            panic!("lifecycle files did not appear in 30s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let pid_str = std::fs::read_to_string(&pid_path).unwrap();
    let recorded_pid: i32 = pid_str.trim().parse().unwrap();
    assert_eq!(recorded_pid, child_pid, "server.pid must match the child's PID");

    child.kill().await.ok();
    let _ = child.wait().await;
}
```

### Success Criteria

#### Automated Verification

- [ ]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml --test shutdown` passes
  (including `shutdown_preserves_state_on_stopped_write_failure`).
- [ ]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml
  server::tests::write_server_stopped_produces_parseable_json` passes.
- [ ]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml
  server::tests::shutdown_preserves_info_when_stopped_write_fails` passes —
  verifies disk-full shutdown leaves info+pid in place.
- [ ] The integration test's 30-second timeout passes consistently on a
  reasonably loaded dev machine; if it flakes, a fix lands in the same sub-phase
  rather than being deferred.

#### Manual Verification

- [ ] Run the binary manually against a fixture config; `kill -TERM <pid>` exits
  cleanly; `server-info.json` removed; `server-stopped.json` present with
  `"reason": "sigterm"`.
- [ ] Repeat with `kill -INT <pid>` (or Ctrl+C); `"reason": "sigint"` recorded.
- [ ] Mutation smoke test: temporarily remove the `remove_file` call; observe
  the integration test fail; restore.

---

## Phase 2.5: Owner-PID watch and idle timeout

### Overview

Add a 60-second `tokio::time::interval` that checks two exit conditions: (a) the
owner process (Claude Code harness) is no longer alive, and (b) the server has
had no HTTP activity for 30 minutes. Either condition triggers the same shutdown
path that SIGTERM does, through the same `mpsc` channel.

Activity tracking is a simple `AtomicI64` holding a `last_activity_epoch_ms`
updated by middleware on every request — no separate bookkeeping system.

### Changes Required

#### 1. Activity tracker + middleware

**File**: `skills/visualisation/visualise/server/src/activity.rs` **Changes**:
New file.

```rust
//! Tracks the timestamp of the most recent HTTP activity. Consumed
//! by the idle-timeout watch. Middleware updates the atomic on every
//! request; no request-path changes needed.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use axum::{extract::Request, middleware::Next, response::Response};

pub struct Activity(AtomicI64);

impl Activity {
    pub fn new() -> Self {
        Self(AtomicI64::new(now_millis()))
    }
    pub fn touch(&self) {
        self.0.store(now_millis(), Ordering::Relaxed);
    }
    pub fn last_millis(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub async fn middleware(
    state: axum::extract::State<Arc<Activity>>,
    req: Request,
    next: Next,
) -> Response {
    state.touch();
    next.run(req).await
}
```

#### 2. Lifecycle-watch loop

**File**: `skills/visualisation/visualise/server/src/lifecycle.rs` **Changes**:
New file.

```rust
//! 60s interval loop that fires shutdown on owner-PID death or
//! prolonged idleness.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::activity::Activity;
use crate::shutdown::ShutdownReason;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub tick: Duration,
    pub idle_limit_ms: i64,
}

impl Settings {
    /// Production defaults: 60s tick, 30-minute idle window.
    /// Tests pass shortened values via `Settings { tick: 50ms,
    /// idle_limit_ms: 200 }` without any test-only conditional
    /// in the module itself.
    pub const DEFAULT: Settings = Settings {
        tick: Duration::from_secs(60),
        idle_limit_ms: 30 * 60 * 1000,
    };
}

pub fn spawn(
    activity: Arc<Activity>,
    owner_pid: i32,
    owner_start_time: Option<u64>,
    settings: Settings,
    tx: mpsc::Sender<ShutdownReason>,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(settings.tick);
        ticker.tick().await; // drop the immediate tick.
        loop {
            ticker.tick().await;
            if owner_pid > 0 && !owner_alive(owner_pid, owner_start_time) {
                let _ = tx.send(ShutdownReason::OwnerPidExited).await;
                return;
            }
            let idle = now_millis() - activity.last_millis();
            if idle >= settings.idle_limit_ms {
                let _ = tx.send(ShutdownReason::IdleTimeout).await;
                return;
            }
        }
    });
}

/// True if the process identified by `pid` is still alive **and**,
/// if `expected_start_time` is provided, still has the same
/// start-time stamp. The start-time cross-check defends against PID
/// reuse — a recycled PID will not have the same start-time as the
/// process we originally recorded. When `expected_start_time` is
/// `None`, falls back to a bare PID probe (tests may pass None; the
/// production server always records a start-time so the strict check
/// runs).
pub(crate) fn owner_alive(pid: i32, expected_start_time: Option<u64>) -> bool {
    use nix::errno::Errno;
    use nix::unistd::Pid;
    let probe = match nix::sys::signal::kill(Pid::from_raw(pid), None) {
        Ok(()) => true,
        Err(Errno::EPERM) => true, // exists, we just can't signal it
        Err(_) => false,           // ESRCH or similar — gone
    };
    if !probe {
        return false;
    }
    match expected_start_time {
        Some(expected) => match crate::server::process_start_time(pid) {
            Some(current) => current == expected,
            // If we recorded a start-time at launch but can't obtain
            // one now, treat it as identity-mismatch (conservative —
            // prefer a false death to a false life when we're
            // uncertain).
            None => false,
        },
        None => true,
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_pid_is_alive() {
        let me = std::process::id() as i32;
        assert!(owner_alive(me, None));
    }

    #[tokio::test]
    async fn reaped_child_pid_is_dead() {
        // Spawn a child that exits immediately, wait for it, then
        // probe — PID is deterministically gone. This is the same
        // pattern used in the lifecycle_owner integration test.
        let child = tokio::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn().unwrap();
        let pid = child.id().unwrap() as i32;
        let _ = child.wait_with_output().await;
        assert!(!owner_alive(pid, None));
    }

    #[test]
    fn start_time_mismatch_treats_pid_as_dead() {
        let me = std::process::id() as i32;
        // Our own process is alive, but if we claim a different
        // start-time, the identity check must reject it.
        let real_start = crate::server::process_start_time(me);
        if real_start.is_none() {
            // Platform without start-time support; skip.
            return;
        }
        let wrong = real_start.unwrap().wrapping_add(1);
        assert!(!owner_alive(me, Some(wrong)));
        assert!(owner_alive(me, real_start));
    }
}
```

#### 3. Wire activity middleware + lifecycle spawn into `server::run`

**File**: `skills/visualisation/visualise/server/src/server.rs` **Changes**:
Extend `run`.

```rust
// …inside run(), right before building the Router:
let activity = Arc::new(crate::activity::Activity::new());

let app = Router::new()
    .route("/", get(placeholder_root))
    .route_layer(axum::middleware::from_fn_with_state(
        activity.clone(),
        crate::activity::middleware,
    ))
    .with_state(state.clone());

// …after writing server-info.json:
let (tx, mut rx) = mpsc::channel::<ShutdownReason>(4);
spawn_signal_handlers(tx.clone());
crate::lifecycle::spawn(
    activity.clone(),
    state.cfg.owner_pid,
    state.cfg.owner_start_time,
    crate::lifecycle::Settings::DEFAULT,
    tx.clone(),
);
```

Update `lib.rs` to declare the new modules (the binary's imports update
automatically):

```rust
pub mod activity;
pub mod config;
pub mod lifecycle;
pub mod server;
pub mod shutdown;
```

#### 4. Integration test: idle timeout fires (fast-clock via `Settings`)

Production call: `lifecycle::spawn(activity, pid, start_time, Settings::DEFAULT,
tx)`. The production 30-minute idle is too long for a test to exercise directly,
so tests pass a shortened `Settings`:

**File**: `skills/visualisation/visualise/server/tests/lifecycle_idle.rs`
**Changes**: New file.

```rust
use std::time::Duration;

use accelerator_visualiser::{activity::Activity, lifecycle, shutdown::ShutdownReason};

#[tokio::test]
async fn idle_timeout_fires_with_fast_clock() {
    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        0, // owner_pid 0 skips the owner check
        None,
        lifecycle::Settings {
            tick: Duration::from_millis(50),
            idle_limit_ms: 200,
        },
        tx,
    );
    let reason = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await.expect("idle fires within 3s").expect("channel ok");
    assert!(
        matches!(reason, ShutdownReason::IdleTimeout),
        "expected IdleTimeout, got {reason:?}"
    );
}
```

The `assert!` wrapper around `matches!` is load-bearing — a bare `matches!`
expression at statement position is a silent no-op. See Testing Strategy for the
general rule.

#### 5. Integration test: owner-PID death triggers shutdown

**File**: `skills/visualisation/visualise/server/tests/lifecycle_owner.rs`
**Changes**: New file.

```rust
use std::time::Duration;

use accelerator_visualiser::{activity::Activity, lifecycle, shutdown::ShutdownReason};

#[tokio::test]
async fn owner_pid_death_triggers_shutdown() {
    // Spawn a short-lived child process, take its PID, wait for it
    // to exit, then confirm lifecycle's owner check returns
    // OwnerPidExited.
    let child = tokio::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn().unwrap();
    let pid = child.id().unwrap() as i32;
    let _ = child.wait_with_output().await;

    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        pid,
        None, // no start_time recorded — owner_alive falls back to bare PID check for tests
        lifecycle::Settings {
            tick: Duration::from_millis(50),
            idle_limit_ms: i64::MAX,
        },
        tx,
    );
    let reason = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await.expect("owner-death fires").expect("channel");
    assert!(
        matches!(reason, ShutdownReason::OwnerPidExited),
        "expected OwnerPidExited, got {reason:?}"
    );
}
```

### Success Criteria

#### Automated Verification

- [ ]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml lifecycle::tests` passes
  (unit tests for `owner_alive`).
- [ ]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml --test lifecycle_idle` passes
  within 5s.
- [ ]  `cargo test --manifest-path
  skills/visualisation/visualise/server/Cargo.toml --test lifecycle_owner`
  passes within 5s.

#### Manual Verification

- [ ] Spawn the server with `owner_pid` set to `$$`, then kill the parent shell:
  server exits within 60s with `"reason": "owner-pid-exited"` in
  `server-stopped.json`.
- [ ] Mutation smoke: set `tick = 10s, idle_limit_ms = 100` in the unit test;
  observe idle_timeout firing essentially immediately.

---

## Phase 2.6: Checksums manifest and `.gitignore`

### Overview

Commit `bin/checksums.json` with the four-platform schema locked and placeholder
SHA-256 values (real values land in Phase 12). Update `.gitignore` so per-arch
binaries downloaded into `bin/` in dev checkouts are never committed.

Placeholder values are a deliberate, visually-distinct string
(`"sha256:0000000000000000000000000000000000000000000000000000000000000000"`) so
a grep for that literal in a release-time build catches any pre-release that
sneaks through without proper checksum regeneration.

### Changes Required

#### 1. Checksums manifest

**File**: `skills/visualisation/visualise/bin/checksums.json` **Changes**: New
file.

```json
{
  "version": "1.19.0-pre.2",
  "note": "Placeholder checksums. Real SHA-256 values are written by the release pipeline in Phase 12. Any value of sha256:0…0 is a deliberate sentinel; fail any build that sees it.",
  "binaries": {
    "darwin-arm64": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    "darwin-x64": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    "linux-arm64": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    "linux-x64": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
  }
}
```

The `version` field mirrors the current plugin version
(`.claude-plugin/plugin.json`) so Phase 2.7's launcher can cross-check them and
fail loudly if they drift.

#### 2. `.gitignore` additions

**File**: `.gitignore` **Changes**: Append
(`skills/visualisation/visualise/server/target/` was added in Phase 2.1; this
sub-phase adds the binary cache and — in preparation for later phases — the
frontend build):

```gitignore
skills/visualisation/visualise/bin/accelerator-visualiser-*
skills/visualisation/visualise/frontend/dist/
skills/visualisation/visualise/frontend/node_modules/
```

The `frontend/` rules are added now rather than split across Phases 5/12 to
avoid two separate `.gitignore` commits and to make it impossible for a
contributor to accidentally commit a local `dist/` when they first scaffold the
frontend in Phase 5.

### Success Criteria

#### Automated Verification

- [ ]  `jq -e '.version, .binaries["darwin-arm64"], .binaries["darwin-x64"],
  .binaries["linux-arm64"], .binaries["linux-x64"]'
  skills/visualisation/visualise/bin/checksums.json` exits 0.
- [ ] `jq -r '.version' skills/visualisation/visualise/bin/checksums.json`equals
  `jq -r '.version' .claude-plugin/plugin.json` (drift check).
- [ ]  `grep -F 'skills/visualisation/visualise/bin/accelerator-visualiser-*'
  .gitignore` matches.
- [ ] `grep -F 'skills/visualisation/visualise/frontend/dist/' .gitignore`
  matches.
- [ ]  `grep -F 'skills/visualisation/visualise/frontend/node_modules/'
  .gitignore` matches.

#### Manual Verification

- [ ]  `touch
  skills/visualisation/visualise/bin/accelerator-visualiser-darwin-arm64 && jj
  status` does not list the file as tracked.

---

## Phase 2.7: `launch-server.sh` Phase 2 implementation

### Overview

Replace the Phase 1 stub with a decomposed launcher. The top-level script is a
~25-line pipeline of named shell functions, each with a single responsibility;
`jq -n` config.json construction moves to a sibling
`scripts/write-visualiser-config.sh` so it's unit-testable on its own.
Cross-platform tool access goes through small shims (`sha256_of`, `download_to`,
`ppid_of`) so minimal Linux containers (Alpine, distroless, busybox) don't
silently degrade. The launcher body takes an advisory `flock` so concurrent
`/accelerator:visualise` invocations don't race. The reuse path cross-checks
`(pid, start_time)` recorded in `server-info.json` against the probed process to
defend against PID reuse.

Responsibilities, in strict order:

1. `umask 077` (new files owner-only).
2. Resolve `PLUGIN_ROOT`, `PROJECT_ROOT`, `TMP_DIR`; `mkdir -p "$TMP_DIR"`;
   `chmod 0700 "$TMP_DIR"`.
3. Take an advisory `flock` on `$TMP_DIR/launcher.lock` (non-blocking; fail
   clearly on contention). Hold it through the rest of the body.
4. Reuse short-circuit: if `server-info.json` and `server.pid` exist, probe the
   recorded PID with the recorded `start_time`. If alive and identity-matched,
   print the URL and exit 0. Otherwise remove stale files and continue.
5. Platform detection (`darwin|linux` × `arm64|x64`); exit clearly on
   unsupported.
6. Tri-precedence binary resolution:
1. `ACCELERATOR_VISUALISER_BIN` env var (shell-scoped override).
2. `visualiser.binary` config key (persistent override; relative paths resolved
   against `$PROJECT_ROOT`).
3. Cached release binary verified against `checksums.json`, downloaded from
   GitHub Releases on miss. Layers 1 and 2 bypass the SHA-256 check — they're
   explicit user overrides.
7. On cache miss or hash mismatch, print the first-run notice **on stdout** (so
   it's visible in Claude Code's expanded slash-command output), `download_to`
   the release asset, verify SHA-256, install. Checksum sentinel check: if
   `EXPECTED_SHA` matches the placeholder (`0…0`), fail with a specific error
   pointing at the override env var / config key — dev checkouts have
   placeholder hashes and must use an override.
8. Call `scripts/write-visualiser-config.sh` with the resolved paths + owner PID
   + plugin version to write `<tmp>/visualiser/config.json`.
9.

`nohup <binary> --config <tmp>/visualiser/config.json >>
<tmp>/visualiser/server.log 2>&1 &`; `disown`. **No `echo $! > server.pid`** —
the Rust server writes its own `server.pid` atomically from inside `server::run`
(Phase 2.3).

10. Poll `<tmp>/visualiser/server-info.json` for up to 5s; read and strictly
    validate the `url` field; print it on stdout under the `**Visualiser URL**:`
    label. On timeout, print a JSON error and exit 1.

Emit two lines on stdout at most in the happy path:

- `Downloading visualiser server (first run, ~8 MB)…` (only on cache miss)
- `**Visualiser URL**: <url>`

Errors go to stderr as JSON lines, but the SKILL.md preamble retains a
`**Visualiser**:` bold label so failures still have framing in the rendered
output (see Phase 2.9).

### Changes Required

#### 1. `launch-server.sh` decomposed top-level script

**File**: `skills/visualisation/visualise/scripts/launch-server.sh` **Changes**:
Full rewrite. The top-level shape is a ~25-line linear pipeline of named
functions. Each function is defined inline in the same file (shell's idiomatic
unit of decomposition) and uses `die_json <kind> <jq-args…>` as the common
error-exit helper that builds JSON via `jq -nc --arg` (never
string-interpolation).

```bash
#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"

# ─── helpers ─────────────────────────────────────────────────

die_json() {
  # die_json "$(jq -nc --arg error 'kind' --arg hint '…' '{error:$error,hint:$hint}')"
  echo "$1" >&2
  exit 1
}

sha256_of() {
  # Prefer sha256sum (GNU coreutils; Linux default); fall back to
  # shasum (Perl-based; macOS default). Emits the hex digest only.
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

download_to() {
  # download_to <url> <dest>. Prefers curl; falls back to wget.
  # TLS floor, bounded redirects, 32 MiB cap — releases are ~8 MB.
  local url="$1" dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --proto '=https' --tlsv1.2 --retry 3 --max-redirs 3 \
      --max-filesize 33554432 -o "$dest" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --tries=3 --max-redirect=3 -O "$dest" "$url"
  else
    return 127
  fi
}

ppid_of() {
  # Linux-busybox-safe: /proc/<pid>/status line `PPid:`.
  # BSD/GNU ps: ps -o ppid= -p <pid>.
  local pid="$1"
  if [ -r "/proc/$pid/status" ]; then
    awk '/^PPid:/ {print $2}' "/proc/$pid/status"
  elif command -v ps >/dev/null 2>&1; then
    ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' '
  else
    return 1
  fi
}

start_time_of() {
  # Matches the Rust server::process_start_time function byte-for-byte.
  # Linux: btime + (stat field 22) / HZ. macOS: ps -p <pid> -o lstart=.
  local pid="$1"
  if [ -r "/proc/$pid/stat" ] && [ -r "/proc/stat" ]; then
    local tail; tail="$(sed -E 's/.*\) //' "/proc/$pid/stat")"
    local starttime_ticks; starttime_ticks="$(echo "$tail" | awk '{print $20}')"
    local hz; hz="$(getconf CLK_TCK 2>/dev/null || echo 0)"
    [ "$hz" -gt 0 ] || return 1
    local btime; btime="$(awk '/^btime / {print $2}' /proc/stat)"
    echo $(( btime + starttime_ticks / hz ))
  elif command -v ps >/dev/null 2>&1 && [ "$(uname -s)" = "Darwin" ]; then
    # Delegate to the Rust binary if it's available — byte-for-byte
    # identical semantics. For the reuse short-circuit we can afford
    # a fork; if the binary isn't available yet (e.g. pre-download)
    # this path isn't reached because server-info.json can't exist.
    local out; out="$(ps -p "$pid" -o lstart= 2>/dev/null | tr -s ' ' ' ' | sed 's/^ //;s/ $//')"
    [ -n "$out" ] || return 1
    # Parse "Mon DD HH:MM:SS YYYY" → epoch via /bin/date if GNU, or
    # via a small awk — BSD date -j -f "%a %b %d %H:%M:%S %Y" handles it.
    date -j -f "%a %b %d %H:%M:%S %Y" "$out" +%s 2>/dev/null
  else
    return 1
  fi
}

# ─── top-level pipeline ──────────────────────────────────────

PROJECT_ROOT="$(find_repo_root)"
cd "$PROJECT_ROOT"

TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp meta/tmp)"
TMP_DIR="$PROJECT_ROOT/$TMP_REL/visualiser"
mkdir -p "$TMP_DIR"
chmod 0700 "$TMP_DIR" 2>/dev/null || true

INFO="$TMP_DIR/server-info.json"
PID_FILE="$TMP_DIR/server.pid"
LOG_FILE="$TMP_DIR/server.log"
CFG="$TMP_DIR/config.json"
STOPPED="$TMP_DIR/server-stopped.json"
LOCK="$TMP_DIR/launcher.lock"

# Serialise concurrent invocations. flock -n fails fast on contention
# so two tabs don't both start servers. flock is part of util-linux
# on Linux; macOS ships it too (Big Sur+). If flock is missing,
# fall back to a mkdir-based lock (atomic on POSIX fs).
if command -v flock >/dev/null 2>&1; then
  exec 9>"$LOCK"
  if ! flock -n 9; then
    die_json "$(jq -nc --arg error 'another launcher is running' \
      --arg hint 'wait for it to finish, or check '"$TMP_DIR"' for a stale lock' \
      '{error:$error,hint:$hint}')"
  fi
else
  if ! mkdir "$LOCK.d" 2>/dev/null; then
    die_json "$(jq -nc --arg error 'another launcher is running' \
      --arg hint 'rm -rf '"$LOCK"'.d if it's stale' \
      '{error:$error,hint:$hint}')"
  fi
  trap 'rmdir "$LOCK.d" 2>/dev/null || true' EXIT
fi

# Reuse short-circuit with (pid, start_time) identity cross-check.
if [ -f "$INFO" ] && [ -f "$PID_FILE" ]; then
  EXISTING_PID="$(tr -cd '0-9' < "$PID_FILE")"
  EXPECTED_START="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  if [ -n "$EXISTING_PID" ] && kill -0 "$EXISTING_PID" 2>/dev/null; then
    # PID is alive. If we have a start_time, cross-check; otherwise
    # trust the bare PID (older server-info.json without start_time).
    if [ -z "$EXPECTED_START" ] || [ "$(start_time_of "$EXISTING_PID" 2>/dev/null || echo '')" = "$EXPECTED_START" ]; then
      URL="$(jq -r '.url // empty' "$INFO" 2>/dev/null || true)"
      # Strict URL validation before echoing to stdout.
      if [[ "$URL" =~ ^http://127\.0\.0\.1:[0-9]+/?$ ]]; then
        echo "**Visualiser URL**: $URL"
        exit 0
      fi
    fi
  fi
  # Stale or mismatched — tidy up so the greenfield path proceeds.
  rm -f "$INFO" "$PID_FILE"
fi
rm -f "$STOPPED"

# Platform detection.
OS_RAW="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_RAW="$(uname -m)"
case "$OS_RAW" in
  darwin|linux) OS="$OS_RAW" ;;
  *) die_json "$(jq -nc --arg error 'unsupported platform' --arg os "$OS_RAW" \
       '{error:$error,os:$os}')" ;;
esac
case "$ARCH_RAW" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64) ARCH="x64" ;;
  *) die_json "$(jq -nc --arg error 'unsupported architecture' --arg arch "$ARCH_RAW" \
       '{error:$error,arch:$arch}')" ;;
esac

if ! command -v jq >/dev/null 2>&1; then
  die_json '{"error":"jq is required but not found","hint":"brew install jq / apt install jq / apk add jq"}'
fi

PLUGIN_VERSION="$(jq -r .version "$PLUGIN_ROOT/.claude-plugin/plugin.json")"
MANIFEST="$SKILL_ROOT/bin/checksums.json"
BIN_CACHE="$SKILL_ROOT/bin/accelerator-visualiser-${OS}-${ARCH}"
RELEASES_URL_BASE="${ACCELERATOR_VISUALISER_RELEASES_URL:-https://github.com/atomic-innovation/accelerator/releases/download}"

# ─── tri-precedence binary resolution ────────────────────────

BIN=""
if [ -n "${ACCELERATOR_VISUALISER_BIN:-}" ]; then
  BIN="$ACCELERATOR_VISUALISER_BIN"
else
  CONFIG_BIN="$("$PLUGIN_ROOT/scripts/config-read-value.sh" visualiser.binary 2>/dev/null || true)"
  if [ -n "$CONFIG_BIN" ]; then
    case "$CONFIG_BIN" in
      /*) ;;                                        # absolute — use verbatim
      *)  CONFIG_BIN="$PROJECT_ROOT/$CONFIG_BIN" ;; # relative — resolve against project root
    esac
    if [ ! -x "$CONFIG_BIN" ]; then
      die_json "$(jq -nc --arg error 'configured visualiser.binary is not executable' \
        --arg path "$CONFIG_BIN" '{error:$error,path:$path}')"
    fi
    BIN="$CONFIG_BIN"
  fi
fi

if [ -z "$BIN" ]; then
  EXPECTED_SHA_RAW="$(jq -r ".binaries[\"${OS}-${ARCH}\"] // empty" "$MANIFEST")"
  EXPECTED_SHA="${EXPECTED_SHA_RAW#sha256:}"
  # Placeholder sentinel detection: a manifest with all-zero hashes is
  # a dev checkpoint that hasn't been through the release pipeline.
  # Refuse to proceed — the user must supply an override.
  if [ "$EXPECTED_SHA" = "0000000000000000000000000000000000000000000000000000000000000000" ]; then
    die_json "$(jq -nc \
      --arg error 'no released binary for this plugin version' \
      --arg version "$PLUGIN_VERSION" \
      --arg hint 'set ACCELERATOR_VISUALISER_BIN=<path> (one-shot) or add `visualiser:\n  binary: <path>` to .claude/accelerator.local.md (persistent)' \
      '{error:$error,plugin_version:$version,hint:$hint}')"
  fi
  # Manifest drift: Phase 12 commits real hashes alongside every
  # plugin-version bump. A mismatch here means someone bumped one
  # without the other.
  MANIFEST_VERSION="$(jq -r '.version // empty' "$MANIFEST")"
  if [ -n "$MANIFEST_VERSION" ] && [ "$MANIFEST_VERSION" != "$PLUGIN_VERSION" ]; then
    die_json "$(jq -nc \
      --arg error 'checksum manifest version drift' \
      --arg plugin "$PLUGIN_VERSION" --arg manifest "$MANIFEST_VERSION" \
      '{error:$error,plugin_version:$plugin,manifest_version:$manifest}')"
  fi
  if [ -x "$BIN_CACHE" ] && [ ! -L "$BIN_CACHE" ]; then
    ACTUAL_SHA="$(sha256_of "$BIN_CACHE")"
  else
    # Refuse symlinks at the cache path — avoids `mv` redirecting
    # the downloaded binary to an attacker-placed destination.
    [ -L "$BIN_CACHE" ] && rm -f "$BIN_CACHE"
    ACTUAL_SHA=""
  fi
  if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
    # First-run notice on stdout (not stderr) so Claude Code's
    # slash-command preamble relays it to the user visibly.
    echo "Downloading visualiser server (first run, ~8 MB)…"
    ASSET_URL="${RELEASES_URL_BASE}/v${PLUGIN_VERSION}/accelerator-visualiser-${OS}-${ARCH}"
    # Stage inside $SKILL_ROOT/bin so the final `mv` is a same-FS
    # atomic rename (mktemp default /tmp often crosses filesystems
    # into the plugin tree).
    TMP_PART="$(mktemp "$SKILL_ROOT/bin/accelerator-visualiser.XXXXXX")"
    if ! download_to "$ASSET_URL" "$TMP_PART"; then
      rm -f "$TMP_PART"
      die_json "$(jq -nc --arg error 'download failed' --arg url "$ASSET_URL" \
        --arg hint 'set ACCELERATOR_VISUALISER_BIN=<path> (one-shot) or add `visualiser:\n  binary: <path>` to .claude/accelerator.local.md (persistent); set ACCELERATOR_VISUALISER_RELEASES_URL for a mirror' \
        '{error:$error,url:$url,hint:$hint}')"
    fi
    DOWNLOADED_SHA="$(sha256_of "$TMP_PART")"
    if [ "$DOWNLOADED_SHA" != "$EXPECTED_SHA" ]; then
      rm -f "$TMP_PART"
      die_json "$(jq -nc --arg error 'checksum mismatch' \
        --arg expected "$EXPECTED_SHA" --arg actual "$DOWNLOADED_SHA" \
        '{error:$error,expected:$expected,actual:$actual}')"
    fi
    install -m 0755 "$TMP_PART" "$BIN_CACHE"
    rm -f "$TMP_PART"
  fi
  BIN="$BIN_CACHE"
fi

# ─── owner PID + start_time for the lifecycle handshake ──────

OWNER_PID="$(ppid_of "$PPID" 2>/dev/null || echo '')"
if [ -z "$OWNER_PID" ] || [ "$OWNER_PID" = "1" ]; then OWNER_PID="$PPID"; fi

# If OWNER_PID resolves to 1 under init-reparenting (containers,
# systemd user scopes), disable the watchdog by passing 0 — the
# lifecycle loop's `if owner_pid > 0` guard skips the check.
if [ "$OWNER_PID" = "1" ]; then OWNER_PID=0; fi

OWNER_START_TIME=""
if [ "$OWNER_PID" -gt 0 ]; then
  OWNER_START_TIME="$(start_time_of "$OWNER_PID" 2>/dev/null || echo '')"
fi

# ─── write config.json (delegated to sibling script) ─────────

CONFIG_ARGS=(
  --plugin-root "$PLUGIN_ROOT"
  --plugin-version "$PLUGIN_VERSION"
  --project-root "$PROJECT_ROOT"
  --tmp-dir "$TMP_DIR"
  --log-file "$LOG_FILE"
  --owner-pid "$OWNER_PID"
)
if [ -n "$OWNER_START_TIME" ]; then
  CONFIG_ARGS+=(--owner-start-time "$OWNER_START_TIME")
fi
"$SCRIPT_DIR/write-visualiser-config.sh" "${CONFIG_ARGS[@]}" > "$CFG"

# ─── background launch; server writes its own pid file ───────

nohup "$BIN" --config "$CFG" >> "$LOG_FILE" 2>&1 &
SERVER_PID=$!
disown "$SERVER_PID" 2>/dev/null || true

# Poll for server-info.json AND server.pid (both are written by
# the Rust server atomically). 5s cap matches Phase 1 precedent.
for _ in $(seq 1 50); do
  [ -f "$INFO" ] && [ -f "$PID_FILE" ] && break
  sleep 0.1
done
if [ ! -f "$INFO" ]; then
  die_json "$(jq -nc --arg error 'server-info.json did not appear within 5s' \
    --arg log "$LOG_FILE" '{error:$error,log:$log}')"
fi

URL="$(jq -r '.url // empty' "$INFO")"
if ! [[ "$URL" =~ ^http://127\.0\.0\.1:[0-9]+/?$ ]]; then
  die_json "$(jq -nc --arg error 'server-info.json contained an invalid url' \
    --arg url "$URL" '{error:$error,url:$url}')"
fi
echo "**Visualiser URL**: $URL"
```

#### 2. `write-visualiser-config.sh` (extracted sibling script)

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
**Changes**: New file, executable. Constructs the `config.json` body via `jq -n`
and writes to stdout. Unit-testable on its own (takes all inputs via flags;
emits JSON; zero filesystem side effects outside of the `jq` process itself).

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT_DEFAULT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

PLUGIN_ROOT="$PLUGIN_ROOT_DEFAULT"
PLUGIN_VERSION=""
PROJECT_ROOT=""
TMP_DIR=""
LOG_FILE=""
OWNER_PID=0
OWNER_START_TIME=""

while [ $# -gt 0 ]; do
  case "$1" in
    --plugin-root)       PLUGIN_ROOT="$2"; shift 2 ;;
    --plugin-version)    PLUGIN_VERSION="$2"; shift 2 ;;
    --project-root)      PROJECT_ROOT="$2"; shift 2 ;;
    --tmp-dir)           TMP_DIR="$2"; shift 2 ;;
    --log-file)          LOG_FILE="$2"; shift 2 ;;
    --owner-pid)         OWNER_PID="$2"; shift 2 ;;
    --owner-start-time)  OWNER_START_TIME="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

for required in PLUGIN_VERSION PROJECT_ROOT TMP_DIR LOG_FILE; do
  if [ -z "${!required}" ]; then
    echo "missing required arg: --${required,,}" >&2
    exit 2
  fi
done

resolve_path() { "$PLUGIN_ROOT/scripts/config-read-path.sh" "$1" "$2"; }
abs_path() {
  # Absolute path for a path key. $1=key, $2=default (relative).
  echo "$PROJECT_ROOT/$(resolve_path "$1" "$2")"
}

# Resolve the nine doc-path keys.
DECISIONS="$(abs_path decisions meta/decisions)"
TICKETS="$(abs_path tickets meta/tickets)"
PLANS="$(abs_path plans meta/plans)"
RESEARCH="$(abs_path research meta/research)"
REVIEW_PLANS="$(abs_path review_plans meta/reviews/plans)"
REVIEW_PRS="$(abs_path review_prs meta/reviews/prs)"
VALIDATIONS="$(abs_path validations meta/validations)"
NOTES="$(abs_path notes meta/notes)"
PRS="$(abs_path prs meta/prs)"

TEMPLATES_USER_ROOT="$(abs_path templates meta/templates)"
TEMPLATES_PLUGIN_ROOT="$PLUGIN_ROOT/templates"

# For each template name, emit a three-tier object. config_override
# is read via config-read-value.sh; emit null when unset.
template_tier() {
  local name="$1"
  local override
  override="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "templates.$name" 2>/dev/null || true)"
  local override_json
  if [ -z "$override" ]; then
    override_json="null"
  else
    override_json="$(jq -nc --arg p "$override" '$p')"
  fi
  jq -nc \
    --argjson config_override "$override_json" \
    --arg user_override "$TEMPLATES_USER_ROOT/$name.md" \
    --arg plugin_default "$TEMPLATES_PLUGIN_ROOT/$name.md" \
    '{config_override:$config_override, user_override:$user_override, plugin_default:$plugin_default}'
}

ADR="$(template_tier adr)"
PLAN="$(template_tier plan)"
RES="$(template_tier research)"
VAL="$(template_tier validation)"
PRD="$(template_tier pr-description)"

if [ -z "$OWNER_START_TIME" ]; then
  OWNER_START_TIME_JSON="null"
else
  OWNER_START_TIME_JSON="$OWNER_START_TIME"
fi

jq -n \
  --arg plugin_root "$PLUGIN_ROOT" \
  --arg plugin_version "$PLUGIN_VERSION" \
  --arg tmp_path "$TMP_DIR" \
  --arg host "127.0.0.1" \
  --argjson owner_pid "$OWNER_PID" \
  --argjson owner_start_time "$OWNER_START_TIME_JSON" \
  --arg log_path "$LOG_FILE" \
  --arg decisions "$DECISIONS" --arg tickets "$TICKETS" \
  --arg plans "$PLANS" --arg research "$RESEARCH" \
  --arg review_plans "$REVIEW_PLANS" --arg review_prs "$REVIEW_PRS" \
  --arg validations "$VALIDATIONS" --arg notes "$NOTES" --arg prs "$PRS" \
  --argjson adr "$ADR" --argjson plan "$PLAN" --argjson research_t "$RES" \
  --argjson validation "$VAL" --argjson pr_description "$PRD" \
  '{
    plugin_root: $plugin_root,
    plugin_version: $plugin_version,
    tmp_path: $tmp_path,
    host: $host,
    owner_pid: $owner_pid,
    owner_start_time: $owner_start_time,
    log_path: $log_path,
    doc_paths: {
      decisions: $decisions, tickets: $tickets, plans: $plans,
      research: $research, review_plans: $review_plans,
      review_prs: $review_prs, validations: $validations,
      notes: $notes, prs: $prs
    },
    templates: {
      adr: $adr, plan: $plan, research: $research_t,
      validation: $validation, "pr-description": $pr_description
    }
  }'
```

Chmod +x.

The extraction gives three wins: (a) the ~60-line `jq -n` construction is in one
focused file, (b) a cargo test can now shell out to this script directly and
feed its output into `Config::from_path` (Rec 11 — see Phase 2.7 testing
additions and the `config_contract.rs` integration test), (c) if Phase 3's
indexer adds new doc types or templates, the change is localised to this script
and the `Config` struct — the launcher proper doesn't change.

#### 3. Replace Phase 1 test harness

**File**: `skills/visualisation/visualise/scripts/test-launch-server.sh`
**Changes**: Full rewrite.

Uses `ACCELERATOR_VISUALISER_BIN` pointing at a bash script that emulates the
real server: it **binds a real ephemeral port** via `python3 -m http.server 0`
(fallback to `nc -l` if Python isn't available), writes `server-info.json` with
the bound port, and parks until SIGTERM. The test harness then reads the URL
from the launcher's stdout and `curl`s the fake server's `/` — closing the
end-to-end reachability gap flagged in the review (Rec 6a).

The fake-server setup is shared with `test-stop-server.sh` via a
`make_fake_visualiser()` helper in `scripts/test-helpers.sh` (see Testing
Strategy § Unit Tests (bash)).

Config-override sub-cases also `curl` the URL to prove the fake is actually
serving. Value-level assertions on `config.json` (each `doc_paths.<key>`
mapping, each `templates.<name>` tier) replace the Phase-1 cardinality checks
(Rec 6d).

Outline (the full script is ~250 lines — the implementation should closely
mirror the existing Phase 1 harness style with the following load-bearing
additions):

1. **Source `scripts/test-helpers.sh`** and use its `make_fake_visualiser
   <out-path>` helper (added in Rec 6a — see Testing Strategy § Unit Tests
   (bash)) to produce the fake binary. Remove inline heredoc copies.

2. **Fake binary binds a real port**. `make_fake_visualiser` uses `python3 -m
   http.server 0` (or `nc -l 0` fallback) to bind an ephemeral port, captures
   the bound port, writes `server-info.json` with the real port + the fake's PID
   + its start-time, and parks until SIGTERM. Every test case that exercises the
   launcher then does `curl -fsS "$URL"` after reading the URL from stdout,
   asserting a 200 response — closing the reachability gap (Rec 6a).

3. **Value-level `config.json` assertions** (Rec 6d). Replace the cardinality
   checks (`.doc_paths | length == 9`, `.templates | length == 5`) with per-key
   mappings:

   ```bash
   assert_eq "config.doc_paths.decisions" \
     "$PROJECT_COPY/meta/decisions" \
     "$(jq -r '.doc_paths.decisions' "$CFG_FILE")"
   # …one assertion per doc_paths key (9) and per templates key (5)
   ```

4. **Placeholder-sentinel refusal test** (Rec 6e, companion). With
   `bin/checksums.json` at the Phase 2.6 placeholder and no
   `ACCELERATOR_VISUALISER_BIN` / `visualiser.binary` set:

   ```bash
   unset ACCELERATOR_VISUALISER_BIN
   RC=0; ERR="$TMPDIR_BASE/sentinel.err"
   bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
   assert_eq "sentinel refusal exit" "1" "$RC"
   assert_eq "sentinel refusal error" "no released binary for this plugin version" \
     "$(jq -r .error "$ERR")"
   ```

5. **Checksum-mismatch automated test** (Rec 6e). Stand up a tiny HTTP fixture
   that serves a fixed byte payload whose SHA-256 is known-wrong for the
   manifest entry. Override `ACCELERATOR_VISUALISER_RELEASES_URL` to point at
   `http://127.0.0.1:<fixture-port>/releases/download` (the launcher appends
   `/v<version>/accelerator-visualiser-...`). Pre-populate `checksums.json` with
   a non-placeholder hash that will not match what the fixture serves:

   ```bash
   python3 -m http.server <port> --directory "$TMPDIR_BASE/fixture" &
   FIXTURE_PID=$!
   # ... inject a non-placeholder expected hash into a copy of checksums.json
   # (the test sets SKILL_ROOT to the tempdir so the launcher reads the copy).
   RC=0; ERR="$TMPDIR_BASE/mismatch.err"
   bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
   assert_eq "mismatch exit" "1" "$RC"
   assert_eq "mismatch error" "checksum mismatch" "$(jq -r .error "$ERR")"
   kill "$FIXTURE_PID" 2>/dev/null || true
   ```

6. **Concurrent-launch race refusal**. Hold a background `flock` on the lock
   file, then invoke the launcher and assert a `"another launcher is running"`
   error exit.

7. **PID-identity mismatch → fresh launch**. Write a fabricated
   `server-info.json` pointing at the test harness's own PID but with a
   deliberately wrong `start_time` (e.g. recorded start_time + 1). Confirm the
   launcher removes the stale files and starts a fresh fake server (new PID)
   rather than hand out the stale URL.

8. **Config-override happy paths** (kept from earlier revision, but now each
   `curl`s the URL after launch to prove reachability).

9. **Platform-detection error path** (kept). The stub `uname` now also errors on
   unexpected flags so a future launcher that calls `uname -r` surfaces loudly
   rather than silently falling through.

10. **All tests wrapped in subshells `( ... )`** so `pushd`/`popd` and env-var
    state don't leak between cases. `test-launch-server.sh` uses subshells
    instead of bare `pushd`/`popd` to keep `set -e` safe (the review flagged
    this in Phase 1's harness too).

Every test block stops the fake server via `stop-server.sh` before the next
case; the trailing EXIT trap reaps any survivors by walking the
`*/meta/tmp/visualiser/server.pid` files under `$TMPDIR_BASE` and `kill`-ing
them by PID (not by ppid, which doesn't catch disowned processes).

The "fake server binds a real port" pattern is load-bearing: it lets the harness
exercise the entire launcher flow — config writing, exec, backgrounding,
info-file polling, URL parsing, **and actual HTTP reachability** — without any
Rust build step. Real binary wiring is still additionally covered by the cargo
integration tests (Phases 2.3–2.5).

#### 4. `config_contract.rs` — Rust / shell schema round-trip (Rec 11)

The `config.json` schema is written by `write-visualiser-config.sh` (shell, `jq
-n`) and consumed by the Rust `Config` struct. Two producers, one consumer; a
schema drift between them is silent (extra field → `deny_unknown_fields` rejects
it loudly; missing field → `#[serde(default)]` or parse error). To guard against
drift, a dedicated cargo test shells out to the script and round-trips the
output through `Config`.

**File**: `skills/visualisation/visualise/server/tests/config_contract.rs`
**Changes**: New file.

```rust
use std::process::Command;

use accelerator_visualiser::config::Config;

#[test]
fn write_visualiser_config_produces_valid_config_json() {
    // Locate the shell script. Tests run from the server/ dir
    // (CARGO_MANIFEST_DIR), so the script is at ../scripts/.
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts/write-visualiser-config.sh");
    assert!(script.exists(), "write-visualiser-config.sh not found at {}", script.display());

    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("project");
    std::fs::create_dir_all(&project_root).unwrap();

    let output = Command::new("bash")
        .arg(&script)
        .args(["--plugin-root", tmp.path().to_str().unwrap()])
        .args(["--plugin-version", "0.0.0-contract-test"])
        .args(["--project-root", project_root.to_str().unwrap()])
        .args(["--tmp-dir", tmp.path().join("visualiser").to_str().unwrap()])
        .args(["--log-file", tmp.path().join("server.log").to_str().unwrap()])
        .args(["--owner-pid", "0"])
        .output()
        .expect("spawn write-visualiser-config.sh");
    assert!(output.status.success(),
        "script failed: {}", String::from_utf8_lossy(&output.stderr));

    // Round-trip: write stdout to a temp file and parse as Config.
    let cfg_path = tmp.path().join("config.json");
    std::fs::write(&cfg_path, &output.stdout).unwrap();
    let cfg = Config::from_path(&cfg_path)
        .expect("config.json produced by script must deserialise as Config");

    assert_eq!(cfg.plugin_version, "0.0.0-contract-test");
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.owner_pid, 0);
    assert_eq!(cfg.doc_paths.len(), 9);
    assert!(cfg.doc_paths.contains_key("decisions"));
    assert!(cfg.doc_paths.contains_key("review_plans"));
    assert!(cfg.doc_paths.contains_key("review_prs"));
    assert_eq!(cfg.templates.len(), 5);
    for name in ["adr", "plan", "research", "validation", "pr-description"] {
        assert!(cfg.templates.contains_key(name), "template {name} missing");
        let tiers = cfg.templates.get(name).unwrap();
        assert!(tiers.plugin_default.to_string_lossy().ends_with(&format!("{name}.md")));
    }
}
```

This test fails the cargo build if either (a) the shell script emits a key the
`Config` struct doesn't recognise — caught by `deny_unknown_fields`; or (b) the
struct adds a required field the script doesn't populate — caught by serde's
missing-field error. Adding a new doc type or template name to later phases
requires coordinated edits on both sides, enforced by this test.

Because it shells out to `bash`, the test requires `bash`, `jq`, and
`config-read-path.sh` / `config-read-value.sh` to be invocable from the script's
working directory. In CI the cargo test inherits `PATH` from the environment
where `mise` has already provisioned these tools.

#### 5. Remove the Phase 1 sentinel

The old sentinel string (`placeholder://phase-1-scaffold-not-yet-running`) is
gone. Anything that grepped for it breaks in this sub-phase. The Phase 1
`test-launch-server.sh` is replaced wholesale; no searches remain. The
`test-cli-wrapper.sh` referenced the launcher's output *indirectly* (via the
delegation sentinel, which still works as-is because it mutates a tempdir copy
of the launcher, not the real one). No change needed.

### Success Criteria

#### Automated Verification

- [ ] `bash skills/visualisation/visualise/scripts/test-launch-server.sh` exits
  0.
- [ ]  `[ -x skills/visualisation/visualise/scripts/write-visualiser-config.sh
  ]`.
- [ ]  `skills/visualisation/visualise/scripts/write-visualiser-config.sh
  --plugin-root <root> --plugin-version 0.0.0-test --project-root <proj>
  --tmp-dir <tmp> --log-file <log> --owner-pid 0 | jq -e '.doc_paths | length ==
  9'` exits 0 (standalone invocation produces a valid config.json body).
- [ ] `mise run test:integration` exits 0 (glob picks up the new suite without
  edits).
- [ ]  `ACCELERATOR_VISUALISER_BIN=…/target/release/accelerator-visualiser bash
  skills/visualisation/visualise/scripts/launch-server.sh` prints a
  `**Visualiser URL**: http://127.0.0.1:<port>` line and `curl -fsS "$URL"`
  returns 200.
- [ ] In a project whose `.claude/accelerator.local.md` sets `visualiser.binary:
  <abs path to built binary>` and with no env var set, the launcher execs the
  configured binary (provable via the config-override test case in
  `test-launch-server.sh`).
- [ ] With both env var and config key set, the env var wins (provable via the
  precedence test case).
- [ ] With `visualiser.binary` pointing at a non-existent path, the launcher
  exits 1 with `{"error":"configured visualiser.binary is not executable",...}`
  on stderr.
- [ ] With placeholder checksums (`sha256:0…0`) in `checksums.json` and no
  override, the launcher fails with `{"error":"no released binary for this
  plugin version",...,"hint":"set ACCELERATOR_VISUALISER_BIN=<path> ..."}` — the
  sentinel is not a usable verification path.
- [ ] With `ACCELERATOR_VISUALISER_RELEASES_URL` pointing at a local HTTP
  fixture serving a binary whose hash doesn't match the manifest, the launcher
  exits with `{"error":"checksum mismatch",...}` (automated via the SHA-256
  mismatch test in Phase 2.7 harness, see Testing Strategy).
- [ ] Two concurrent launcher invocations in the same project: the second prints
  `{"error":"another launcher is running"...}` rather than racing to spawn a
  duplicate server (provable via a concurrent-launch test case).
- [ ] With a stale `server-info.json` recording a PID that is now recycled to a
  *different* process (spawn a child, record its PID, wait, start an unrelated
  long-running process hopefully with that PID — or synthesise via the test
  harness by writing a `server-info.json` with our own PID but a mismatched
  `start_time`), the launcher detects the identity mismatch, removes the stale
  files, and starts a fresh server rather than hand out the stale URL.

#### Manual Verification

- [ ] Run the launcher with placeholder checksums and no override — it exits 1
  with the "no released binary" error rather than attempting a download. This is
  the expected state until Phase 12 ships real hashes.
- [ ] With a dev override against a binary built by Phase 2.1-2.5, open the
  printed URL in a browser and confirm the placeholder body is visible. Confirm
  `server.pid` in `<tmp>/visualiser/` matches the `accelerator-visualiser`
  process PID (written by the server itself, not the launcher).
- [ ] In a test project, set `visualiser.binary: <abs path>` in
  `.claude/accelerator.local.md`; run `/accelerator:visualise`; confirm the
  configured binary is used (inspect `ps` for the PID and its argv).
- [ ] Start the launcher from a terminal that is not Claude Code; confirm
  `owner_pid` in `config.json` is the terminal shell's PID (or 0 in a container
  under init); close the terminal and observe the server exiting within ~60s via
  the owner-PID watch (or not exiting, if `owner_pid == 0` was written).

---

## Phase 2.8: `stop-server.sh` and PID reuse

### Overview

Add the graceful-shutdown CLI that hooks into the lifecycle files, and teach
`launch-server.sh` to detect an already-live instance and return its URL instead
of starting a duplicate.

### Changes Required

#### 1. `stop-server.sh`

**File**: `skills/visualisation/visualise/scripts/stop-server.sh` **Changes**:
New file, executable. Sources the same cross-platform shims (`start_time_of`)
that `launch-server.sh` uses so PID-identity checks behave identically. On
SIGKILL escalation, synthesises a `server-stopped.json` with `reason:
"forced-sigkill"` so the post-shutdown lifecycle-file invariant holds (the
kernel kills the server before it can write the file itself).

```bash
#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"

# Share the start-time helper with launch-server.sh by sourcing
# a thin helpers file. Both scripts consume the same function.
source "$SCRIPT_DIR/_launcher-helpers.sh"

PROJECT_ROOT="$(find_repo_root)"
cd "$PROJECT_ROOT"

TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp meta/tmp)"
TMP_DIR="$PROJECT_ROOT/$TMP_REL/visualiser"
INFO="$TMP_DIR/server-info.json"
PID_FILE="$TMP_DIR/server.pid"
STOPPED="$TMP_DIR/server-stopped.json"

subcommand="${1:-stop}"
case "$subcommand" in
  status) stop_server_status; exit $? ;;
  stop)   stop_server_stop;   exit $? ;;
  *)
    echo '{"error":"unknown subcommand","hint":"use stop (default) or status"}' >&2
    exit 2
    ;;
esac
```

The `stop` and `status` entry points, and the helpers file, are defined below.

#### 2. `_launcher-helpers.sh` (shared by launch / stop / status)

**File**: `skills/visualisation/visualise/scripts/_launcher-helpers.sh`
**Changes**: New file, **not executable** (sourced only — the leading underscore
and missing exec bit together exclude it from Phase 1.6's test-runner glob).

```bash
#!/usr/bin/env bash
# Shared helpers for launch-server.sh, stop-server.sh. Never
# executed directly — sourced only.

# (Move sha256_of, download_to, ppid_of, start_time_of, die_json
# from launch-server.sh into this file; launch-server.sh also
# sources it. Keeps the two scripts byte-identical on the
# cross-platform shims.)

sha256_of() { … }       # as in launch-server.sh
download_to() { … }     # as in launch-server.sh
ppid_of() { … }         # as in launch-server.sh
start_time_of() { … }   # as in launch-server.sh
die_json() { … }        # as in launch-server.sh

# Written atomically: tempfile + mv within the same dir.
write_server_stopped() {
  local path="$1" reason="$2"
  local dir
  dir="$(dirname "$path")"
  mkdir -p "$dir"
  local tmp
  tmp="$(mktemp "$dir/.server-stopped.XXXXXX")"
  local ts
  ts="$(date +%s 2>/dev/null || echo null)"
  jq -nc --arg reason "$reason" --argjson timestamp "$ts" \
    '{reason:$reason, timestamp:$timestamp, written_by:"stop-server.sh"}' > "$tmp"
  chmod 0600 "$tmp" 2>/dev/null || true
  mv "$tmp" "$path"
}

# Common impls used by stop-server.sh:

stop_server_status() {
  if [ ! -f "$INFO" ]; then
    jq -nc '{status:"not_running"}'
    return 0
  fi
  local pid url start_time alive
  pid="$(tr -cd '0-9' < "$PID_FILE" 2>/dev/null || echo '')"
  url="$(jq -r '.url // empty' "$INFO" 2>/dev/null || true)"
  start_time="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  alive=false
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    if [ -z "$start_time" ] || [ "$(start_time_of "$pid" 2>/dev/null || echo '')" = "$start_time" ]; then
      alive=true
    fi
  fi
  jq -nc --arg status "$([ "$alive" = true ] && echo running || echo stale)" \
    --arg url "$url" --argjson pid "${pid:-null}" \
    '{status:$status, url:$url, pid:$pid}'
}

stop_server_stop() {
  if [ ! -f "$PID_FILE" ]; then
    jq -nc '{status:"not_running"}'
    return 0
  fi

  local pid expected_start current_start
  pid="$(tr -cd '0-9' < "$PID_FILE")"
  expected_start="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"

  if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
    # PID file exists but process is gone — clean up.
    rm -f "$PID_FILE" "$INFO"
    jq -nc '{status:"stopped",note:"pid was already dead"}'
    return 0
  fi

  # PID identity check: refuse to kill an unrelated process that
  # happens to hold a recycled PID.
  if [ -n "$expected_start" ]; then
    current_start="$(start_time_of "$pid" 2>/dev/null || echo '')"
    if [ "$current_start" != "$expected_start" ]; then
      jq -nc --argjson pid "$pid" --arg expected "$expected_start" --arg actual "${current_start:-unknown}" \
        '{status:"refused",reason:"pid identity mismatch — not killing an unrelated process",pid:$pid,expected_start_time:$expected,actual_start_time:$actual}'
      rm -f "$PID_FILE" "$INFO"  # stale lifecycle files, but leave the unrelated process alone.
      return 1
    fi
  fi

  # SIGTERM, 2s grace, then SIGKILL escalation.
  kill "$pid" 2>/dev/null || true
  local i
  for i in $(seq 1 20); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.1
  done

  local forced=false
  if kill -0 "$pid" 2>/dev/null; then
    forced=true
    kill -9 "$pid" 2>/dev/null || true
    sleep 0.1
  fi
  if kill -0 "$pid" 2>/dev/null; then
    jq -nc --argjson pid "$pid" '{status:"failed",error:"process still running after SIGKILL",pid:$pid}'
    return 1
  fi

  # Post-shutdown invariant: server-stopped.json must exist after a
  # clean shutdown. If the server was SIGTERM'd, the Rust shutdown
  # handler wrote it. If it was SIGKILL'd, we synthesise it from
  # here — the graceful code path never ran.
  if [ "$forced" = true ] || [ ! -f "$STOPPED" ]; then
    write_server_stopped "$STOPPED" "forced-sigkill"
  fi

  rm -f "$PID_FILE" "$INFO"
  if [ "$forced" = true ]; then
    jq -nc '{status:"stopped",forced:true}'
  else
    jq -nc '{status:"stopped"}'
  fi
}
```

#### 3. Refactor `launch-server.sh` to source the shared helpers

**File**: `skills/visualisation/visualise/scripts/launch-server.sh` **Changes**:
Move the `sha256_of`, `download_to`, `ppid_of`, `start_time_of`, `die_json`
function bodies out of the launcher and into `_launcher-helpers.sh`. The
launcher sources them:

```bash
source "$SCRIPT_DIR/_launcher-helpers.sh"
```

— immediately after the `source "$PLUGIN_ROOT/scripts/vcs-common.sh"` line.
Everything else in the launcher is unchanged.

#### 4. Test harness for stop-server + reuse

**File**: `skills/visualisation/visualise/scripts/test-stop-server.sh`
**Changes**: New file, executable.

Outline — the full script follows the same ~130-line shape as the Phase 1
`test-stop-server.sh` precedent with the following discipline:

1. **Source `scripts/test-helpers.sh`** and use its `make_fake_visualiser
   "$FAKE_BIN"` helper instead of an inline heredoc. The shared helper binds a
   real ephemeral port, so the reuse test can curl the URL to prove reachability
   — closing the pass-2 gap where the inline fake wrote port 9 verbatim.
2. **Use `spawn_and_reap_pid` for every "deterministically-dead PID" case**.
   Hardcoded `999999` is forbidden. The stale-file test writes
   `$(spawn_and_reap_pid)` into `server.pid` + the fabricated
   `server-info.json`, and asserts the post-launch `server.pid` is *not* equal
   to that stale value.
3. **EXIT trap uses the shared `reap_visualiser_fakes <base>` helper** (added to
   `scripts/test-helpers.sh` alongside `make_fake_visualiser`), which walks
   `$TMPDIR_BASE/**/meta/tmp/visualiser/server.pid` and `kill -9`s each PID it
   finds. Replaces `pkill -P $$`, which doesn't catch disowned/reparented fakes.
4. **Every `curl` assertion against the reuse URL** — the whole point of binding
   a real port is to prove reachability round-trips through the launcher's
   stdout. `assert_exit_code "URL reachable" 0 curl -fsS "$URL"`.
5. **Subshell per test block** (`( ... )` rather than bare `pushd`/`popd`) so
   cwd and env-var mutations don't leak.
6. **Identity-mismatch refusal test** (new): fabricate a `server-info.json`
   pointing at the harness's own PID but with `start_time` = real_start + 1.
   Invoke `stop-server.sh` and assert output contains `"status":"refused"` and
   `"reason":"pid identity mismatch"`. Confirm the harness process is **still
   alive** after the call.
7. **Forced-SIGKILL stopped.json synthesis test** (new): use
   `make_unkillable_fake_visualiser` (a variant that traps SIGTERM to a no-op
   and ignores it, forcing escalation) to prove `stop-server.sh` writes
   `{reason:"forced-sigkill", written_by:"stop-server.sh"}` after the kill-9
   path runs.
8. **Status subverb tests** (new): assert `stop-server.sh status` returns the
   three documented states (`running` / `stale` / `not_running`) in the three
   matching scenarios, with a matching `url` field when running.

Test cases (each in a subshell):

- Fresh project → `stop-server.sh` prints `{"status":"not_running"}`.
- Launch → PID/info files land; `curl $URL` returns 200; `stop-server.sh` prints
  `{"status":"stopped"}`; post-stop the URL is unreachable (`curl` fails);
  `server-stopped.json` exists and records `"reason":"sigterm"`.
- Double launch (reuse path) — second invocation prints the same URL, same PID;
  `curl` against the reused URL still returns 200.
- Stale-PID cleanup — uses `$(spawn_and_reap_pid)` for the stale value;
  post-launch `$NEW_PID != $STALE_PID`, new PID is alive.
- Identity-mismatch refusal — `stop-server.sh` refuses and emits
  `{"status":"refused",...}`; harness PID still alive.
- Forced SIGKILL — unkillable-fake triggers escalation path;
  `server-stopped.json` present with `"reason":"forced-sigkill"`.
- Status subverb — `running` when alive,  `stale` when info exists but PID is
  dead/mismatched, `not_running` when info is absent.

Chmod +x the test script.

### Success Criteria

#### Automated Verification

- [ ] `bash skills/visualisation/visualise/scripts/test-stop-server.sh` exits 0.
- [ ] `[ ! -x skills/visualisation/visualise/scripts/_launcher-helpers.sh ]` —
  shared helpers file is not executable (sourced only).
- [ ] `mise run test:integration` still exits 0 after the harness lands (glob
  auto-enrols test files and excludes `_launcher-helpers.sh` both by
  leading-underscore name and by non-exec bit).
- [ ] `stop-server.sh` with no `server.pid` present prints
  `{"status":"not_running"}` and exits 0.
- [ ] `stop-server.sh status` with no `server-info.json` prints
  `{"status":"not_running"}`; with a live recorded instance prints
  `{"status":"running","url":"...","pid":...}`; with a recorded instance whose
  PID has been recycled prints `{"status":"stale",...}` (provable via
  `test-stop-server.sh`'s identity-mismatch case).
- [ ] `stop-server.sh` refuses to kill a PID whose start-time doesn't match the
  recorded one — outputs `{"status":"refused","reason":"pid identity mismatch
  ..."}` and removes the stale lifecycle files (proven by a test that writes a
  fabricated `server-info.json` pointing at the test harness's own PID with a
  mismatched `start_time`).
- [ ] After `stop-server.sh` escalates to SIGKILL, `server-stopped.json` exists
  with `{"reason":"forced-sigkill","written_by":"stop-server.sh",...}` (proven
  by a test that spawns a fake binary ignoring SIGTERM).

#### Manual Verification

- [ ] Manual end-to-end: launch the real binary via dev-override; confirm the
  URL in a browser; run `stop-server.sh`; confirm the URL no longer responds;
  `server-stopped.json` is present with `"reason": "sigterm"`;`server-info.json`
  and `server.pid` are gone.
- [ ] Relaunch after stopping produces a different PID (new process) — proves
  the stop actually terminated the prior instance, not just the lifecycle files.
- [ ] `stop-server.sh status` reports read-only state without touching the
  server — subsequent `curl $URL` still succeeds.

---

## Phase 2.9: SKILL.md update and level/component test hierarchy

### Overview

Replace the Phase 1 "Availability" block with real operational guidance, and
restructure the invoke + mise test tasks into a level/component hierarchy:

- **Invoke layer**: one task per `<level>.<component>` intersection. Phase 2
  introduces `test.unit.visualiser` (cargo `--lib`),
  `test.integration.visualiser` (cargo `--tests` + shell suites under
  `skills/visualisation/visualise/`), `test.integration.config` (shell suites
  under `scripts/`), and `test.integration.decisions` (shell suites under
  `skills/decisions/`). These land inside a new `tasks/test/` package, replacing
  the flat `tasks/test.py`.
- **Mise layer**: composes levels only. `mise run test:unit` runs every
  `test.unit.*` task; `mise run test:integration` runs every
  `test.integration.*` task; `mise run test` runs `test:unit` then
  `test:integration` in order (serial via `run` array, not `depends`, so running
  either level alone stays pure to that level).

The split of the old single `tasks/test.py` `integration` task into three
component tasks (`visualiser`, `config`, `decisions`) is a **bundled refactor**
— not new behaviour, but necessary because the flat global task cannot coexist
with the component-scoped ones. All existing shell suites continue to run; they
just run under the component whose subtree they live in.

Each component task keeps the Phase 1.6 glob-discovery idiom (executable
`test-*.sh` files, with `test-helpers.sh` name and exec-bit exclusions) scoped
to its subtree, so adding a new shell suite inside a component auto-enrols with
no task-file edit.

### Changes Required

#### 1. SKILL.md body

**File**: `skills/visualisation/visualise/SKILL.md` **Changes**: Replace the
`Visualiser URL (not yet running)` line and the entire `## Availability` block.

The 11-path preamble (intentional per the Phase 1 plan review) is retained —
it's the source for the `config.json` that `launch-server.sh` now writes. The
preamble shape does not change in this sub-phase.

Diff (conceptual):

```diff
-**Visualiser URL (not yet running)**: !`bash ${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/launch-server.sh`
+**Visualiser**: !`bash ${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/launch-server.sh`

-## Availability
-
-<!--
-Context for Claude only — do not relay to the user:
-This is a scaffold release. The Rust server that will eventually
-back the URL line is added in a later phase. No server is running and no
-port is listening.
--->
-
-Tell the user, without referring to phases, sub-phases, or release
-numbers: the visualiser UI isn't ready yet — this is a scaffold
-release. There's no server to connect to; the `placeholder://` line
-above will be replaced by a real URL in a future release. Do not
-attempt to open the placeholder in a browser.
-
-To use the same entry point from a terminal (also a placeholder
-today), symlink the wrapper onto `$PATH`. Copy the full command
-below — the path is pre-resolved for you:
-
-**Install command**: !`printf 'ln -s "%s" "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser" "$HOME/.local/bin/accelerator-visualiser"`
-
-If `accelerator-visualiser` is not found after running that command,
-make sure `$HOME/.local/bin` is on your `$PATH` (on macOS you may
-need to add `export PATH="$HOME/.local/bin:$PATH"` to your shell rc).
+## Server lifecycle
+
+<!--
+Context for Claude only — do not relay to the user verbatim:
+The visualiser server is a locally-backgrounded Rust process. It
+binds a random high port on 127.0.0.1 and exits automatically when
+idle for 30 minutes, when the process that launched the server
+exits, or when `stop-server.sh` is invoked. Re-running
+`/accelerator:visualise` while the server is up reuses the
+existing instance. Note: "the process that launched the server"
+means different things by invocation mode — for the slash command
+it's the Claude Code harness; for the CLI wrapper it's the
+terminal shell. Don't assume Claude Code specifically.
+-->
+
+The server runs in the background on your local machine. The
+`**Visualiser**:` line above renders the URL on success, or a
+JSON error line on failure. Tell the user:
+- Open the URL in a browser — no HTML UI is served yet; only a
+  plain-text placeholder response that confirms the server is up.
+- Re-running this command returns the same URL if the server is
+  already running.
+- The server exits on its own after 30 minutes idle, or when the
+  process that launched it exits. To stop it explicitly, run the
+  command below.
+- If the line above contains a JSON `{"error":...}` object, the
+  server isn't running; read the `hint` field in the JSON for
+  remediation.
+
+**Stop command**: !`printf 'bash "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/stop-server.sh"`
+**Status command**: !`printf 'bash "%s" status' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/stop-server.sh"`
+
+### Overrides
+
+By default the plugin downloads a verified per-arch binary from
+GitHub Releases on first use. Two overrides exist for dev,
+air-gapped, or pinned-binary workflows:
+
+1. **Environment variable** (one-shot, shell-scoped):
+   `ACCELERATOR_VISUALISER_BIN=<path>`. Bypasses SHA-256
+   verification; use for local dev builds.
+2. **Config key** (persistent, per-project):
+
+   ```yaml
+   ---
+   visualiser:
+     binary: <absolute or project-relative path>
+   ---
+   ```

+
+ in `.claude/accelerator.md` (team-committed) or
+ `.claude/accelerator.local.md` (personal, gitignored).
+ Relative paths resolve against the project root.
+
+ The team-committed form is trusted on par with the rest of
+ the repo — anyone approving a PR that changes it should
+ treat the value as code, not data.
+

+The release-binary mirror URL can be overridden via
+`ACCELERATOR_VISUALISER_RELEASES_URL=<base-url>` (air-gapped
+or proxy-hosted mirrors).

+

+To run the visualiser from a terminal, symlink the CLI wrapper:

+

+**Install command**: !
`printf 'ln -s "%s" "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser" "$HOME/.local/bin/accelerator-visualiser"`

```

The `**Visualiser**` bold label (now without the "URL" suffix) is emitted by the
SKILL.md preamble, not the launcher. The launcher emits `**Visualiser URL**:
<url>` on stdout inside that line on the happy path, and a JSON error on stderr
on failure. The double-labelling (`Visualiser` outer, `Visualiser URL` inner) is
deliberate — if the launcher errors and emits nothing on stdout, the outer label
still renders and tells the reader this is where the URL would go.

#### 2. Convert `tasks/test.py` into a `tasks/test/` package

**File**: `tasks/test.py` **Changes**: **Deleted.** Replaced by the package
below. Invoke resolves `tasks/test/` as a sub-collection so `test.unit.*` and
`test.integration.*` dotted names work without explicit `Collection.from_module`
scaffolding in the parent `tasks/__init__.py` (existing auto-loading continues
to apply).

**File**: `tasks/test/__init__.py` **Changes**: New file. Empty — invoke
auto-discovers sub-modules.

```python
"""Test tasks, grouped by level (unit, integration, e2e).

Each sub-module exposes one task per component so we can run a
single component in isolation (`invoke test.integration.visualiser`)
or compose levels at the mise layer
(`mise run test:integration` → all integration components).
"""
```

**File**: `tasks/test/_helpers.py` **Changes**: New file. Shared shell-suite
discovery used by every `test.integration.<component>` task.

```python
"""Shared helpers for component integration tasks."""

import os
from pathlib import Path

from invoke import Context

EXCLUDED_HELPER_NAMES = {"test-helpers.sh"}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def run_shell_suites(context: Context, subtree: str) -> None:
    """Glob-discover and run every executable test-*.sh inside a subtree.

    The exec-bit filter excludes `scripts/test-helpers.sh` (sourced,
    not run); the name-level filter is belt-and-braces for
    filesystems that synthesise exec bits uniformly.
    """
    repo = repo_root()
    root = repo / subtree
    if not root.exists():
        return
    suites = sorted(
        p.relative_to(repo).as_posix()
        for p in root.glob("**/test-*.sh")
        if p.is_file()
        and p.name not in EXCLUDED_HELPER_NAMES
        and os.access(p, os.X_OK)
    )
    for suite in suites:
        print(f"Running {suite}...")
        context.run(suite)
        print()
```

**File**: `tasks/test/unit.py` **Changes**: New file. One task per component
with unit-level coverage.

```python
"""Unit-level tests, grouped by component."""

from invoke import Context, task

from ._helpers import repo_root


@task
def visualiser(context: Context):
    """Unit tests for the visualiser server (cargo --lib)."""
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo test --manifest-path {manifest} --lib")
```

**File**: `tasks/test/integration.py` **Changes**: New file. One task per
component with integration-level coverage.

```python
"""Integration-level tests, grouped by component."""

from invoke import Context, task

from ._helpers import repo_root, run_shell_suites


@task
def visualiser(context: Context):
    """Integration tests for the visualiser (cargo --tests + shell suites)."""
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo test --manifest-path {manifest} --tests")
    run_shell_suites(context, "skills/visualisation/visualise")


@task
def config(context: Context):
    """Integration tests for the plugin-wide config scripts."""
    run_shell_suites(context, "scripts")


@task
def decisions(context: Context):
    """Integration tests for the decisions skill scripts."""
    run_shell_suites(context, "skills/decisions")
```

#### 3. `mise.toml` — level-composed test tasks

**File**: `mise.toml` **Changes**: Replace the existing `test:integration` and
`test` definitions.

```toml
[tasks."test:unit"]
description = "Run all unit tests (every test.unit.<component> invoke task)"
run = [
    "invoke test.unit.visualiser",
]

[tasks."test:integration"]
description = "Run all integration tests (every test.integration.<component> invoke task)"
run = [
    "invoke test.integration.visualiser",
    "invoke test.integration.config",
    "invoke test.integration.decisions",
]

[tasks.test]
description = "Run unit tests, then integration tests, in order"
run = [
    "mise run test:unit",
    "mise run test:integration",
]
```

Ordering notes:

- `test` uses sequential `run` rather than `depends` because `depends` entries
  can execute in parallel at mise's discretion, and we want unit tests to
  precede integration tests deterministically. `run` runs the listed commands
  serially, failing fast on the first non-zero exit.
- Inside each level task, components are also listed sequentially in `run`.
  Parallelising components is a future option if the test wall-time warrants it;
  Phase 2 stays serial so test output remains easy to read.
- Neither level task depends on the other, so `mise run test:integration` in
  isolation runs the integration level only — not the unit level too —
  preserving the invariant "running a level alone means that level alone".

#### 4. Verify no stragglers reference the removed `test.integration` task

Grep the repo for the old task name to catch any stale documentation. Phase 1's
plan and review files contain the name in reference form (historical record);
those are fine. Any `mise` or `invoke` *invocation* of `test.integration` must
be updated, since the flat task is gone.

### Success Criteria

#### Automated Verification

- [ ]  `yq --front-matter=extract '.name'
  skills/visualisation/visualise/SKILL.md` prints `visualise` (frontmatter
  intact).
- [ ]  `grep -F 'Visualiser URL (not yet running)'
  skills/visualisation/visualise/SKILL.md` **does not match** (the Phase 1
  wording is gone).
- [ ] `grep -F 'Server lifecycle' skills/visualisation/visualise/SKILL.md`
  matches.
- [ ] `grep -F 'Stop command' skills/visualisation/visualise/SKILL.md` matches.
- [ ] `[ ! -f tasks/test.py ]` (flat file removed).
- [ ]  `[ -f tasks/test/__init__.py ] && [ -f tasks/test/unit.py ] && [ -f
  tasks/test/integration.py ] && [ -f tasks/test/_helpers.py ]`.
- [ ] `invoke test.unit.visualiser` exits 0.
- [ ] `invoke test.integration.visualiser` exits 0.
- [ ] `invoke test.integration.config` exits 0.
- [ ] `invoke test.integration.decisions` exits 0.
- [ ] `invoke --list | grep -E '^  test\.(unit|integration)\.'` lists exactly
  four component tasks.
- [ ] `mise run test:unit` exits 0.
- [ ] `mise run test:integration` exits 0.
- [ ] `mise run test` exits 0 (runs level tasks in order; fails fast on the
  first non-zero sub-task).
- [ ] `invoke test.integration` **does not resolve** — the old flat task is gone
  (`invoke test.integration 2>&1 | grep -F 'No idea'` matches).

#### Manual Verification

- [ ] In a Claude Code session, `/accelerator:visualise` produces a preamble
  that includes (a) the 11 resolved paths, (b) a real `**Visualiser URL**:
  http://127.0.0.1:<port>` line, and (c) the new Server lifecycle section with a
  copy-pasteable stop command.
- [ ] Opening the URL in a browser returns 200 with the Phase 2 placeholder
  body.
- [ ] Claude's paraphrase of the Server lifecycle block relays operational info
  (reuse, idle timeout, stop command) in user-facing language; no phase numbers
  leak.
- [ ] Running `mise run test:integration` prints `Running …` banners for suites
  from every component (at minimum, the visualiser cargo `--tests` output, the
  visualiser shell suites, `scripts/test-config.sh`, and
  `skills/decisions/scripts/test-adr-scripts.sh`).
- [ ] Deliberately break one unit test (e.g., flip an `assert_eq` in
  `config::tests::parses_valid_config`), run `mise run test`, observe it fails
  at the `test:unit` stage and never reaches `test:integration` (proves the
  level-ordering contract).

---

## Testing Strategy

### Assertion discipline

Every `matches!` expression used as an assertion **must** be wrapped in
`assert!(...)`. A bare `matches!` at statement position evaluates and discards
the bool — a silent no-op assertion. The Phase 2 `lifecycle_idle.rs` and
`lifecycle_owner.rs` both apply this rule; reviewers should grep for
`^\s*matches!` in every new test module.

### Shared test helpers

**`scripts/test-helpers.sh`** is extended with:

- **`make_fake_visualiser <out-path>`** — writes an executable shell script to
  `<out-path>` that, when invoked with `--config <cfg>`, reads `tmp_path` from
  the config, binds an ephemeral port (`python3 -m http.server 0` preferred; `nc
  -l 0` fallback), serves a static body from a tempdir, writes
  `server-info.json` with the bound port + its PID + its start-time, and parks
  until SIGTERM. Used by `test-launch-server.sh`, `test-stop-server.sh`, and any
  future harness that needs a drop-in visualiser stand-in.
- **`make_unkillable_fake_visualiser <out-path>`** — variant of the above that
  traps SIGTERM to a no-op and only exits on SIGKILL. Used by
  `test-stop-server.sh` to exercise the `kill -9` escalation path in
  `stop-server.sh` and assert the `{reason:"forced-sigkill",
  written_by:"stop-server.sh"}` synthesis.
- **`reap_visualiser_fakes <base-dir>`** — EXIT-trap reaper. Walks
  `<base-dir>/**/meta/tmp/visualiser/server.pid` and `kill -9`s each PID. Unlike
  `pkill -P $$` this catches disowned + reparented fakes (which are the common
  case since the launcher's `nohup … & disown` chain reparents to init).
  Harnesses install it via `trap 'reap_visualiser_fakes "$TMPDIR_BASE"; rm -rf
  "$TMPDIR_BASE"' EXIT`.
- **`assert_json_eq <name> <jq-filter> <expected> <path>`** — `jq`-based field
  assertion used for value-level `config.json` checks.
- **`assert_stderr_contains <name> <substr> <cmd…>`** — one-shot grep-in-stderr
  helper replacing the bespoke `grep -qF | echo PASS` pattern from the Phase 1
  plan's config-override sub-case.
- **`spawn_and_reap_pid`** — spawn a `sh -c 'exit 0'`, `wait` for it, echo the
  reaped PID. Used by bash tests that need a deterministically-dead PID (mirrors
  the Rust `lifecycle_owner` pattern from Rec 6b).

### Unit Tests (cargo)

Colocated `#[cfg(test)] mod tests` per module:

- **`config`** — parses the valid fixture, rejects missing required fields,
  rejects missing files, accepts populated `config_override` values, rejects
  unknown top-level fields (Rec 6 minor; `#[serde(deny_unknown_fields)]` on
  `Config`).
- **`server`** — `write_server_info`/`write_server_stopped` round-trip with 0600
  permissions, `write_pid_file` round-trip, `process_start_time` is stable for
  the same PID, non-loopback host is rejected, `host_header_guard` 403s
  mismatched Host headers, middleware updates `Activity` on real requests.
- **`activity`** — `touch` updates the atomic; `last_millis` returns monotonic
  values.
- **`lifecycle`** — `owner_alive(self_pid, None) == true`; reaped child PID
  returns `false`; `start_time` mismatch also returns `false`.

### Integration Tests (cargo)

Separate `server/tests/*.rs` files driving the real binary via `assert_cmd` and
subprocess spawning:

- **`config_cli.rs`** — missing config → exit code 2; valid fixture → exit code
  0.
- **`shutdown.rs`** — spawn binary, send SIGTERM, assert clean shutdown with
  lifecycle files in the expected state; check that `server.pid` is removed
  alongside `server-info.json`; verify that an in-flight request on a
  deliberately-slow test route completes successfully across SIGTERM (Rec 6f,
  see `graceful_draining.rs` below).
- **`graceful_draining.rs`** — add a `#[cfg(test)]`-gated slow route inside the
  library (e.g. a handler that `tokio::time::sleep(500ms).await`s then returns
  200), start a request, send SIGTERM after 100ms, assert the client receives a
  full 200 before the process exits. This is the only way to verify
  `with_graceful_shutdown` actually drains.
- **`lifecycle_idle.rs`** — fast-clock idle timeout delivers
  `ShutdownReason::IdleTimeout` on the channel; the assertion is wrapped in
  `assert!(matches!(...))`.
- **`lifecycle_owner.rs`** — owner-PID death triggers
  `ShutdownReason::OwnerPidExited`; spawn-and-reap pattern.
- **`config_contract.rs`** — Rec 11: shells out to
  `scripts/write-visualiser-config.sh` with a deterministic set of flags, pipes
  the output into `Config::from_path`-via-tempfile, and asserts successful
  round-trip. Fails the cargo build if the shell script emits a shape the Rust
  struct can't deserialise (or vice versa) — single-source-of-truth guard
  against schema drift.

### Integration Tests (bash)

Colocated with the scripts they test; auto-enrolled by the glob runner from
Phase 1.6:

- **`test-launch-server.sh`** (Phase 2 rewrite) — full launcher flow against
  `make_fake_visualiser` (binds a real ephemeral port); config-writing,
  backgrounding, server-info polling, URL parsing + reachability via `curl`,
  value-level `config.json` assertions on all 14 key→path mappings,
  placeholder-sentinel refusal, checksum-mismatch against a local HTTP fixture,
  concurrent-launch refusal via a held `flock`, PID-identity mismatch → fresh
  launch, config-override (env/abs/relative/broken/env-beats-config),
  unsupported-platform error.
- **`test-stop-server.sh`** (new) — stop-server on fresh project
  (`not_running`), stop-server after launch (`stopped`, server-stopped.json
  present), double-launch reuse, stale-file cleanup (using `spawn_and_reap_pid`
  for a deterministically-dead PID rather than `999999`), PID-identity mismatch
  refuses kill, SIGKILL escalation synthesises `server-stopped.json`,
  `stop-server.sh status` in each state (not_running / running / stale).
- **`test-cli-wrapper.sh`** (unchanged from Phase 1) — wrapper delegation and
  argument forwarding stay correct because the wrapper contract is unchanged.

### Manual Testing Steps

1. Build the server locally: `cargo build --release --manifest-path
   skills/visualisation/visualise/server/Cargo.toml`.
2. Export
   `ACCELERATOR_VISUALISER_BIN=$(pwd)/skills/visualisation/visualise/server/target/release/accelerator-visualiser`.
3. In a Claude Code session rooted at the workspace, type
   `/accelerator:visualise`. Confirm the expanded skill prompt contains the 11
   path values, a `**Visualiser URL**: http://127.0.0.1:<port>` line, and the
   Server lifecycle block.
4. `curl -fsS <url>/` returns 200 with the placeholder body.
5. Re-run `/accelerator:visualise`; confirm the URL is identical (reuse), the
   PID in `server.pid` is unchanged.
6. Run `bash skills/visualisation/visualise/scripts/stop-server.sh`; confirm
   `{"status":"stopped"}`, browser now gets connection refused,
   `server-stopped.json` contains `"reason":"sigterm"`.
7. Relaunch; confirm a new PID is assigned and a new URL.
8. Relaunch, then `kill -9` the Claude Code process (or close the tab); within
   ~60s the server exits on its own with `"reason": "owner-pid-exited"` in
   `server-stopped.json`.
9. Without `ACCELERATOR_VISUALISER_BIN` set, launch. The preprocessor attempts
   to download the release asset, fails (no real release until Phase 12), and
   exits 1 with a clear error. Expected until Phase 12.

## Performance Considerations

- **Startup latency**: binary start → listen → `server-info.json` written is
  dominated by axum's TCP bind and the first `tracing_subscriber::fmt()` call.
  Target: <150 ms on a warm cache. If profiling shows otherwise, review tracing
  init and `tokio` runtime flavour.
- **Idle overhead**: 60s tick of the lifecycle watch is negligible;
  `nix::sys::signal::kill(pid, None)` is a cheap syscall and the activity atomic
  is lock-free.
- **Binary size**: with the pinned dep set, `cargo bloat` should report <8 MB on
  the linux-x64 release profile (axum + tokio + serde + clap + tempfile + nix +
  tracing). If it exceeds 12 MB, revisit feature flags on the dependencies —
  this is a pre-gate for Phase 12's stated 6–10 MB target and for the research's
  "end-user downloads ~8 MB" UX line.

## Migration Notes

Phase 2 **removes** the following Phase 1 artefacts:

- The `placeholder://phase-1-scaffold-not-yet-running` sentinel. Anything that
  grepped for it no longer matches.
- The Phase 1 `test-launch-server.sh`. Replaced wholesale in Phase 2.7; the new
  harness asserts the new stdout shape.
- The Phase 1 "Availability" wording in SKILL.md.

Phase 2 **adds** the following files that did not exist in Phase 1:

- `skills/visualisation/visualise/server/**/*` (Cargo project).
- `skills/visualisation/visualise/bin/checksums.json` (placeholder manifest).
- `skills/visualisation/visualise/scripts/stop-server.sh`.
- `skills/visualisation/visualise/scripts/test-stop-server.sh`.
- `tasks/test/__init__.py`, `tasks/test/_helpers.py`, `tasks/test/unit.py`,
  `tasks/test/integration.py` (replaces the flat `tasks/test.py`).

Phase 2 **modifies** the following files:

- `.gitignore` — adds `server/target/`, `bin/accelerator-visualiser-*`,
  `frontend/dist/`, `frontend/node_modules/`.
- `mise.toml` — replaces `test:integration`/`test` with the level-composed
  `test:unit`, `test:integration`, and `test` tasks (see Phase 2.9).
- `skills/visualisation/visualise/SKILL.md` — replaces URL label and
  Availability block.
- `skills/visualisation/visualise/scripts/launch-server.sh` — full rewrite (not
  a migration; the old stub is deleted).
- `skills/visualisation/visualise/scripts/test-launch-server.sh` — full rewrite.

Phase 2 **removes** the following files:

- `tasks/test.py` — replaced by the `tasks/test/` package. The `invoke
  test.integration` dotted name is gone; callers must use the component-scoped
  tasks (`invoke test.integration.visualiser|config|decisions`) or the
  mise-level aggregate (`mise run test:integration`).

**Bundled test-task refactor.** Splitting the flat `test.integration` into
`test.integration.visualiser`, `test.integration.config`, and
`test.integration.decisions` lands in this phase because the new
component-scoped tasks cannot coexist with the flat one (same dotted-name prefix
with incompatible semantics). This is a pure mechanical refactor: every shell
suite that ran under the old task still runs, just under whichever component
owns its subtree. If Phase 12 or later introduces more components (e.g.,
`frontend`), adding `test.integration.frontend` is additive.

Known Phase 3+ churn points introduced by this phase:

- **`visualiser.binary` config key.** This is the first key under a
  `visualiser:` section in the accelerator config schema. If later phases add
  more visualiser-scoped config (e.g. `visualiser.port`,
  `visualiser.idle_timeout_minutes`), they land as sibling keys and — per
  ADR-0017's extension-point pattern — are read via `config-read-value.sh
  visualiser.<key>` in the launcher or server as needed. The key is not
  enumerated in `config-summary.sh` in Phase 2; if it becomes a common override,
  consider promoting its visibility there later.
- **`config.json` schema.** The `Config` struct is the binary contract. Any new
  field added by later phases requires a coordinated preprocessor change. Always
  add fields as `Option<T>` where backward compatibility is desired, or treat
  the schema bump as a deliberate breaking change.
- **`GET /` handler.** Phase 5 replaces the placeholder response with the SPA's
  embedded `index.html`. The route path stays `/`.
- **`AppState` shape.** Phase 3 extends it with indexer state; Phase 4 extends
  it with SSE hub state. The current `AppState { cfg }` is minimal so later
  phases can grow it without renaming.
- **`checksums.json` `version` field.** Gated to the plugin version; every
  plugin-version bump after Phase 12's real release must also regenerate the
  four hash entries. The `jq -r .version` drift check in this phase's automated
  verification catches a version mismatch; Phase 12's release script also
  enforces it.
- **Idle timeout constant.** Phase 2 hardcodes `30 * 60 * 1000` ms. If
  real-world use shows this is wrong, Phase 10 can promote it to a config field
  without changing the channel or watch logic.
- **Test-task scheme for future components.** Later phases that introduce new
  components (Phase 5 adds `frontend/`; Phase 11 adds Playwright E2E) register
  their test tasks as new sub-tasks: `test.unit.frontend` in
  `tasks/test/unit.py`, `test.integration.frontend` in
  `tasks/test/integration.py`, and — when E2E lands — a new `tasks/test/e2e.py`
  module plus a `mise run test:e2e` level task. `mise run test` gains a third
  entry in its sequential `run` array at that point. The component is always the
  innermost dotted segment; the level is the mise prefix. No restructuring is
  needed beyond additions.

## References

- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  (Phase 2 ownership: Gaps 2, 5, 6; resolved decisions D3, D4, D8)
- Design spec: `meta/specs/2026-04-17-meta-visualisation-design.md` (
  Architecture § Runtime, Launch and lifecycle, Preprocessor responsibilities)
- Phase 1 plan:
  `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md`
- Plan review for Phase 1 (style precedent):
  `meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md`
- Superpowers launcher precedent:
  `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/scripts/start-server.sh`
- Superpowers server precedent (owner-PID + idle-timeout patterns):
  `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/scripts/server.cjs:270-317`
- Superpowers stop script precedent:
  `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/brainstorming/scripts/stop-server.sh`
- Shared test helpers: `scripts/test-helpers.sh`
- Glob-discovered test runner: `tasks/test.py`
- Config path wrapper: `scripts/config-read-path.sh`
- Repo-root resolver: `scripts/vcs-common.sh:8-18`
- Existing SKILL.md preamble precedent: `skills/github/review-pr/SKILL.md:1-36`
- Plugin manifest (registered in Phase 1.5): `.claude-plugin/plugin.json`
