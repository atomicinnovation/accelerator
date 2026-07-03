---
type: plan
id: "2026-07-02-0163-scaffold-cli-workspace-version-subcommand"
title: "Scaffold the cli/ Hexagonal Workspace with a version Subcommand Implementation Plan"
date: "2026-07-02T23:00:23+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0163"
parent: "work-item:0163"
derived_from: ["codebase-research:2026-07-02-0163-cli-workspace-version-subcommand-scaffold"]
tags: [rust, cli, hexagonal, scaffold, workspace, version, kernel]
revision: "a0a3b3bcba66b9eddd787a1a571c14b32f3db1ae"
repository: "accelerator"
last_updated: "2026-07-03T00:11:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Scaffold the cli/ Hexagonal Workspace with a version Subcommand Implementation Plan

## Overview

Grow the minimal one-member `cli/` workspace that 0162 landed into the first real
hexagon: add a thin `kernel` crate (error taxonomy + logging facility) and lay out
the `version` subcommand as a `version/{core, inbound/cli, outbound/build_metadata}`
module tree inside `launcher`, wired end-to-end from clap parsing through the domain
to a vergen-injected build-metadata adapter. The `version` feature is trivial by
design; its value is proving the architecture — inward-only domain, kernel wiring,
build-time metadata, cargo-pup enforcement — is real and test-first.

The launcher crate is named `launcher`, not `cli`, to free the `cli/` directory name
and avoid a `cli/cli/` path — a recorded deviation from ADR-0054's `cli` crate name;
it still produces the `accelerator` binary.

## Current State Analysis

The post-0162 `cli/` workspace is eight source files (everything under
`cli/target/` and `cli/.pup/` ignored):

- `cli/Cargo.toml` — workspace root: `resolver = "2"`, `members = ["launcher"]`,
  `[workspace.package]` (`version = "1.24.0-pre.2"`, `edition = "2021"`),
  `[workspace.lints.rust] warnings = "deny"`, and the pedantic+nursery clippy
  block with the restriction opt-ins. **No `[workspace.dependencies]` table.**
- `cli/launcher/Cargo.toml` — `[[bin]] name = "accelerator"`, inherits
  `version`/`edition` from the workspace. **No `[dependencies]`,
  `[build-dependencies]`, or `build = "build.rs"`.**
- `cli/launcher/src/main.rs` — `const fn main() {}`.
- `cli/launcher/src/lib.rs` — a self-test `crate_name()` only.
- `cli/pup.ron` — one rule targeting `^launcher::domain($|::)` (near-vacuous; the
  hexagon uses `version::core`, not `domain`).
- `cli/rustfmt.toml`, `cli/deny.toml` — as configured by 0162.

The mise/invoke task tree runs every `cli/` gate **workspace-wide** against
`cli/Cargo.toml` (`cargo fmt --all`, `cargo clippy --workspace`, `cargo deny check`,
`cargo pup`, `cargo llvm-cov nextest --workspace`); none enumerate crates, so a new
`[workspace].members` entry is picked up with no task edits. `tasks/build.py`
version-coherence over members requires each member to inherit
`version.workspace = true` or match the workspace version.

## Desired End State

A built `accelerator` binary answers `accelerator version` with four fields (version,
commit SHA, build date, target triple), one per line; the version tracks the
`launcher` crate's Cargo version and the git/build facts are vergen-injected at build
time and degrade to `unknown` on a git-less/shallow build. The `version` hexagon
exists as `version/{core, inbound/cli, outbound/build_metadata}` inside `launcher`
with the domain (`core`) importing no adapter or I/O crate (enforced by cargo-pup on
the architecture lane). The slice expresses errors via a real `kernel::Error`
taxonomy and initialises logging through `kernel::logging`; a malformed `ACCELERATOR_LOG`
filter surfaces as a `kernel::Error` and exits non-zero. `mise run check` and the
bare `mise run` both exit 0.

### Key Discoveries:

- Luminosity `../luminosity/cli/` is a near-complete blueprint for the workspace
  root, `launcher/build.rs`, vergen pins, the hexagon module tree, and the black-box
  integration test (`../luminosity/cli/launcher/tests/version.rs`).
- vergen **and** vergen-gitcl are pinned **exactly** as a matched pair (`vergen
  =9.0.6`, `default-features = false`; `vergen-gitcl =1.0.8`); the transitive
  `vergen-lib` bump in vergen `9.1.0` breaks the `Emitter`, and because vergen-gitcl
  shares `vergen-lib` a floating caret on it could drag the same bump. These are the
  one place the `cli/` workspace deviates from its own default caret convention (used
  for `clap`/`thiserror`/`tracing`/etc.; the server has no vergen dependency to
  compare against). (Luminosity pins vergen exactly but leaves vergen-gitcl at `"1"`;
  this plan tightens gitcl too.)
- `build.rs` deliberately omits `fail_on_error()`; the outbound adapter reads
  `option_env!("VERGEN_*").unwrap_or("unknown")`, so a git-less build compiles and
  degrades to `unknown` (`../luminosity/cli/launcher/build.rs:1`,
  `.../outbound/build_metadata.rs:16`).
- Two clauses of AC-5 have **no luminosity precedent**: luminosity's `kernel::Error`
  is an uninhabited `pub enum Error {}` and it has no logging facility at all. This
  plan goes beyond the mirror per the resolved decision (below).
- `cli/deny.toml`'s license allow-list already includes `Unicode-3.0` and
  `Unicode-DFS-2016`, expected to cover the `tracing-subscriber` + `regex` closure;
  no allow-list edits are anticipated (`cli/deny.toml:32`) — but this is a prediction
  to confirm by enumerating the resolved licenses of the full new closure (Phase 1
  manual verification), not an assumption.
- The pup fixture at `tests/integration/pup/test_import_rule.py` is a self-contained
  probe workspace; only `test_real_cli_pup_ron_loads` reads the shipped `cli/pup.ron`
  and merely asserts it parses — it survives the rule rewrite.

### Resolved decision (AC-5 kernel depth)

`kernel` is real, not a placeholder: `kernel::logging::init() -> Result<(),
kernel::Error>` builds a stderr `tracing` subscriber from an `ACCELERATOR_LOG` env-filter,
and `kernel::Error` is a `thiserror` (1.x) enum whose one variant surfaces a
malformed filter. `main` initialises logging before dispatch, so a malformed
`ACCELERATOR_LOG` is a genuinely reachable, testable error path that exits non-zero — the
error arm is real, not an unreachable never-arm.

Placing the logging facility in `kernel` follows ADR-0054, which names logging as a
kernel concern ("everything links it"). The accepted cost is that the
`tracing-subscriber` closure (env-filter → `regex`) now sits in the crate every future
sub-binary links; this is a deliberate loosening of ADR-0054's "dependency-light"
framing, justified by logging being genuinely cross-cutting and by the single shared
`init` seam every binary needs. Kept in kernel with the error taxonomy; the composition
root supplies only the binding.

## What We're NOT Doing

- No git-style external-subcommand dispatch / on-demand binary resolution
  (`external_subcommand`, `exec`) — 0164 owns that. `version` is the only exposed
  subcommand; `accelerator <unknown>` exits non-zero via clap.
- No `kernel` config-access module — deferred to 0166/0167.
- No dispatch/launcher contract in `kernel` — deferred to 0164.
- No splitting the hexagon layers into separate crates — modules within `launcher`
  until pressure warrants it (ADR-0053, spike 0158).
- No cross-compilation, release profile, HTTP stack (`reqwest`/rustls), or SLSA/
  distribution wiring.
- No JSON/file logging (the server's setup); a CLI wants a plain stderr subscriber.

## Implementation Approach

Three independently mergeable phases, each leaving `mise run check` and the bare
`mise run` green. Test-first throughout: the logic that carries behaviour lives in
pure, directly-testable seams (a `kernel` env-filter builder, an outbound
`or_unknown` degradation helper, the `core` service against a fake), while the
global/IO wiring (`logging::init`, the vergen adapter, `main`) is proven by the
black-box integration test.

- **Phase 1** adds the `kernel` crate (error taxonomy + logging), mergeable alone.
- **Phase 2** adds the `version` hexagon + `build.rs`, exercised by unit tests;
  `main` stays inert so the phase compiles and passes without yet changing runtime
  behaviour.
- **Phase 3** wires the composition root, adds the black-box integration tests, and
  rewrites the cargo-pup rule — `accelerator version` goes live and every behavioural
  AC is met.

Phases 2 and 3 depend on Phase 1's `kernel`; they are sequential but each is a
self-contained green PR.

---

## Phase 1: kernel crate — error taxonomy and logging facility

### Overview

Introduce the `kernel` crate with a real `thiserror` error taxonomy and a
`tracing`-based stderr logging facility. Add the `[workspace.dependencies]` table
the later phases draw from. Nothing in `launcher` changes yet.

### Changes Required:

#### 1. Workspace root — add the shared dependency table and the kernel member

**File**: `cli/Cargo.toml`
**Changes**: Add `kernel` to `members`; add a `[workspace.dependencies]` table; add
`license` and `publish` to the existing `[workspace.package]` so both members inherit
them.

```toml
[workspace]
resolver = "2"
members = ["launcher", "kernel"]

[workspace.package]
version = "1.24.0-pre.2"
edition = "2021"
license = "MIT"
publish = false

[workspace.dependencies]
clap = { version = "4.6", features = ["derive"] }
# vergen and vergen-gitcl are a matched pair pinned exactly: vergen 9.1.0's
# transitive vergen-lib bump breaks the Emitter, and a vergen-gitcl minor/patch
# can float that same vergen-lib independently, so both are pinned.
vergen = { version = "=9.0.6", default-features = false }
vergen-gitcl = { version = "=1.0.8", features = ["build", "cargo"] }
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
time = { version = "0.3", features = ["parsing"] }
```

`vergen-gitcl` is now pinned exactly (`=1.0.8`, the vetted pair with `vergen =9.0.6`)
rather than the open caret `"1"`: because both crates share `vergen-lib`
transitively, a `cargo update` on `vergen-gitcl` alone could otherwise drag a
`vergen-lib` bump and recreate the exact `Emitter` break the exact `vergen` pin
exists to prevent. The one comment carried across is the matched-pair rationale
(genuinely non-obvious and load-bearing); no code-restating or ADR-referencing
comments are added, per the plan's comment policy. `license`/`publish` move to
`[workspace.package]` so `launcher` and `kernel` express identical metadata via
inheritance. `clap`/`vergen`/`time` are declared here now but first consumed in
Phase 2.

#### 2. kernel manifest

**File**: `cli/kernel/Cargo.toml` (new)

```toml
[package]
name = "kernel"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

`version.workspace = true` keeps the crate clean under `tasks/build.py`
version-coherence; `license`/`publish` inherit from `[workspace.package]` so both
members stay in step.

#### 3. kernel error taxonomy

**File**: `cli/kernel/src/lib.rs` (new)

```rust
pub mod logging;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid log filter: {0}")]
    LogFilter(#[from] tracing_subscriber::filter::ParseError),
}
```

`LogFilter` wraps the concrete `ParseError` via `#[from]` (rather than a flattened
`String`), preserving the `std::error::Error` source chain and matching the server's
error-chaining precedent; the `{0}` Display still renders the parse message, so the
`invalid log filter` substring the integration test greps for is unchanged. No new
dependency is introduced — `tracing-subscriber` is already a kernel dependency.

#### 4. kernel logging facility (test-first)

**File**: `cli/kernel/src/logging.rs` (new)

The reachable, testable behaviour is factored into a pure env-filter builder; the
global subscriber install is a thin infallible wrapper around it. Write the unit
tests first, then the code.

```rust
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

use crate::Error;

fn filter_from_env(raw: Option<&str>) -> Result<EnvFilter, Error> {
    match raw {
        Some(directives) => Ok(EnvFilter::builder().parse(directives)?),
        None => Ok(EnvFilter::default().add_directive(LevelFilter::INFO.into())),
    }
}

pub fn init() -> Result<(), Error> {
    let raw = std::env::var("ACCELERATOR_LOG").ok();
    let filter = filter_from_env(raw.as_deref())?;
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .try_init();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::filter_from_env;
    use crate::Error;

    #[test]
    fn unset_env_builds_a_default_filter() {
        assert!(filter_from_env(None).is_ok());
    }

    #[test]
    fn a_valid_directive_builds_a_filter() {
        assert!(filter_from_env(Some("debug")).is_ok());
    }

    #[test]
    fn a_malformed_directive_is_a_log_filter_error() {
        let error = filter_from_env(Some("bad=notalevel")).unwrap_err();
        assert!(matches!(error, Error::LogFilter(_)));
        assert!(error.to_string().contains("invalid log filter"));
    }
}
```

With `#[from]` on `Error::LogFilter`, `filter_from_env` propagates the parse error
via `?` rather than a manual `map_err`. The malformed-directive test asserts the
specific `Error::LogFilter` variant (not just `is_err()`) and the stable
`invalid log filter` Display substring, pinning both the reachable error arm and the
message contract the Phase 3 integration test greps for.

**Reachability check (do this first):** the reachable-error-arm claim depends on
`EnvFilter::parse` actually returning `Err` for the chosen string. Confirm against
the pinned `tracing-subscriber 0.3` that `parse("bad=notalevel")` returns `Err`
(the parser is lenient; an invalid *level* is a reliable rejection, an empty/odd
directive may not be). Use the **same** rejected string here and in the Phase 3
integration case so the unit and black-box tests exercise the identical path; if
`"bad=notalevel"` turns out to parse, pick another provably-rejected directive and
update both call sites together.

`init` reads `ACCELERATOR_LOG` (namespaced, not the ecosystem-standard `RUST_LOG`)
so it never clobbers unrelated Rust tooling in the same shell; this divergence from
the server's `RUST_LOG` is deliberate. `init` is called exactly once from `main`, so
the `try_init()` already-initialised case cannot occur for the real caller; the
`let _ =` discards it defensively to keep `init` idempotent for tests, and the only
surfaced error is a malformed filter. Testing `init` directly is avoided because it
mutates process-global state; `filter_from_env` carries the assertions.

### Success Criteria:

#### Automated Verification:

- [x] Workspace builds with the new member and lockfile: `cd cli && cargo build`
- [x] kernel unit tests pass: `cd cli && cargo nextest run -p kernel`
- [x] Format + lint clean: `mise run cli:check`
- [x] Coverage run passes: `mise run test:unit:cli`
- [x] Dependency graph clean (licenses/bans over the new tracing closure):
      `mise run deny:check`
- [x] cargo-pup still loads the shipped config, and the new closure compiles under
      the pinned nightly the pup lane uses: `mise run pup:check`
- [x] Full read-only gate: `mise run check`
- [x] End-to-end default task: `mise run`

#### Manual Verification:

- [x] `cli/Cargo.lock` diff contains only the expected `tracing`/`thiserror`
      closure and no `native-tls`/`openssl` edge.
- [x] Enumerate the resolved licenses of the newly introduced `time`/`clap`/`vergen`/
      `vergen-gitcl`/`tracing-subscriber` closures and confirm every one is covered
      by `cli/deny.toml`'s allow-list; add a per-crate `[[licenses.exceptions]]` for
      any that are not (the "no allow-list edits expected" claim is a prediction to
      verify, not an assumption).

---

## Phase 2: version hexagon and build-metadata injection

### Overview

Lay out the `version/{core, inbound/cli, outbound/build_metadata}` module tree in
`launcher`, add the `build.rs` that injects build metadata via vergen, and prove the
domain and the degradation path with unit tests. `main` stays `const fn main()` so
runtime behaviour is unchanged and the phase is independently green; the slice is
wired live in Phase 3.

### Changes Required:

#### 1. launcher manifest — dependencies, build script, dev-dependency

**File**: `cli/launcher/Cargo.toml`
**Changes**: add `build = "build.rs"`, `[dependencies]`, `[build-dependencies]`,
`[dev-dependencies]`.

```toml
[package]
name = "launcher"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
build = "build.rs"

[lints]
workspace = true

[[bin]]
name = "accelerator"
path = "src/main.rs"

[dependencies]
kernel = { path = "../kernel" }
clap = { workspace = true }
tracing = { workspace = true }

[build-dependencies]
vergen = { workspace = true }
vergen-gitcl = { workspace = true }

[dev-dependencies]
time = { workspace = true }
```

#### 2. build script (replicate luminosity verbatim)

**File**: `cli/launcher/build.rs` (new) — copy
`../luminosity/cli/launcher/build.rs`. It emits `VERGEN_BUILD_TIMESTAMP`,
`VERGEN_CARGO_TARGET_TRIPLE`, and `VERGEN_GIT_SHA` (full SHA), and deliberately does
not call `fail_on_error()`. Keep **only** the `fail_on_error()`-omission rationale as
a comment (genuinely non-obvious); drop the descriptive first line that merely
restates what the script emits, per the plan's comment policy.

`vergen-gitcl` resolves the SHA by invoking the **`git` CLI binary** at build time
(the "gitcl" backend). Two distinct build-host conditions both degrade cleanly to
`unknown` because `fail_on_error()` is omitted: (a) no `.git` directory (git present,
no repo) and (b) no `git` executable on `PATH` at all. A real (non-`unknown`) SHA
therefore requires a `git` CLI on the build host, not merely a `.git` tree — recorded
as a build-time prerequisite in Migration Notes.

#### 3. Expose the version module from the library

**File**: `cli/launcher/src/lib.rs`
**Changes**: replace the `crate_name` self-test with the module declaration.

```rust
pub mod version;
```

**File**: `cli/launcher/src/version/mod.rs` (new)

```rust
pub mod core;
pub mod inbound;
pub mod outbound;
```

#### 4. Domain core — ports, value object, service (test-first)

**File**: `cli/launcher/src/version/core.rs` (new) — replicate luminosity's
`core.rs`: an outbound `trait BuildMetadata` (four `-> &'static str` methods), a
`VersionReport` value object (four owned `String`s), an inbound `trait ReportVersion`,
and `VersionReporter<M: BuildMetadata>` (`const fn new`) implementing it. Carry the
`#[cfg(test)]` unit test that drives `VersionReporter` against a `FakeBuildMetadata`
and asserts each of the four fields. `core.rs` has no `use` of any adapter or I/O.

#### 5. Outbound adapter — vergen reader with a testable degradation seam

**File**: `cli/launcher/src/version/outbound/mod.rs` (new)

```rust
pub mod build_metadata;
```

**File**: `cli/launcher/src/version/outbound/build_metadata.rs` (new) — the
`VergenBuildMetadata` unit struct implementing `BuildMetadata`, reading
`env!("CARGO_PKG_VERSION")` for the version and `option_env!("VERGEN_*")` for the
three git/build facts. Extract the degradation into a pure helper so AC-2 is tested
without a git-less build:

```rust
fn or_unknown(value: Option<&'static str>) -> &'static str {
    value.unwrap_or("unknown")
}
```

Each git/build accessor returns `or_unknown(option_env!("VERGEN_..."))`. Write the
unit tests first — cover both the pure helper **and** that each real accessor routes
its own `option_env!` key through it (so a bypass, a hard-coded value, or a wrong
`VERGEN_*` key is caught, not just the helper in isolation):

```rust
#[cfg(test)]
mod tests {
    use super::{or_unknown, VergenBuildMetadata};
    use crate::version::core::BuildMetadata;

    #[test]
    fn a_missing_fact_degrades_to_unknown() {
        assert_eq!(or_unknown(None), "unknown");
    }

    #[test]
    fn a_present_fact_is_passed_through() {
        assert_eq!(or_unknown(Some("abc123")), "abc123");
    }

    #[test]
    fn crate_version_is_the_launcher_cargo_version() {
        assert_eq!(
            VergenBuildMetadata.crate_version(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn commit_sha_reads_its_vergen_key_or_unknown() {
        assert_eq!(
            VergenBuildMetadata.commit_sha(),
            option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
        );
    }

    #[test]
    fn build_date_reads_its_vergen_key_or_unknown() {
        assert_eq!(
            VergenBuildMetadata.build_date(),
            option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown")
        );
    }

    #[test]
    fn target_triple_reads_its_vergen_key_or_unknown() {
        assert_eq!(
            VergenBuildMetadata.target_triple(),
            option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or("unknown")
        );
    }
}
```

#### 6. Inbound clap adapter — parse, render, dispatch, log line (test-first)

**File**: `cli/launcher/src/version/inbound/mod.rs` (new)

```rust
pub mod cli;
```

**File**: `cli/launcher/src/version/inbound/cli.rs` (new) — replicate luminosity's
`cli.rs` with the accelerator name: `#[command(name = "accelerator",
disable_version_flag = true)]` on `struct Cli`, a `Command` enum with a single
`Version` variant, `render(&VersionReport) -> String` producing the four prefixed
lines, and `dispatch(&Cli, &impl ReportVersion) -> Result<(), kernel::Error>`. Emit
the AC-5 log line inside the `Version` arm before printing:

```rust
tracing::debug!("reporting version");
```

Write two `#[cfg(test)]` tests first (drive both with a hand-built `VersionReport`
and, for dispatch, a fake `ReportVersion`):

- `render` asserts the **exact** four-line string — each line's prefix *and* value at
  its fixed index — not merely that the four values appear somewhere. This pins the
  per-field prefixes and the one-per-line ordering at the unit level, so a swapped
  prefix or reordered line fails fast rather than only in the Phase 3 binary. Note
  the luminosity prefixes are position-sensitive (e.g. `built:  ` carries two spaces
  for column alignment); reproduce them verbatim.
- `dispatch` is called with `Cli { command: Command::Version }` and a fake reporter,
  asserting it returns `Ok(())`. This exercises the `Version`-arm control flow (match,
  the `tracing::debug!` call, the print, the `Ok` return) within the phase that
  introduces it, rather than leaving it unrun until Phase 3.

### Success Criteria:

#### Automated Verification:

- [x] Workspace builds, running the new build script: `cd cli && cargo build`
- [x] Domain, adapter, and render unit tests pass:
      `cd cli && cargo nextest run -p launcher`
- [x] Format + lint clean (clippy pedantic/nursery over the new modules):
      `mise run cli:check`
- [x] Coverage run passes: `mise run test:unit:cli`
- [x] Dependency graph clean: `mise run deny:check`
- [x] cargo-pup passes (rule still targets the old path; `version::core` imports
      nothing, so no violation either way): `mise run pup:check`
- [x] Full read-only gate: `mise run check`
- [x] End-to-end default task: `mise run`

#### Manual Verification:

- [x] `cargo build` emits the three `VERGEN_*` env vars (proven by the Phase 3
      integration test's `assert_eq!` reconciliation against `option_env!`).
- [x] Module tree on disk matches `version/{core, inbound/cli,
      outbound/build_metadata}`.

---

## Phase 3: launcher composition root, integration tests, and cargo-pup rule

### Overview

Wire the composition root in `main.rs` (parse → init logging → dispatch), making
`accelerator version` live and the `kernel::Error` arm reachable. Add the black-box
integration tests covering all four fields one-per-line, the emitted log line, the
malformed-filter error path, and the unknown-subcommand exit. Rewrite `cli/pup.ron`
to target `version::core` and realign the probe fixture.

### Changes Required:

#### 1. Composition root

**File**: `cli/launcher/src/main.rs`
**Changes**: replace `const fn main() {}` with the wiring.

```rust
use std::process::ExitCode;

use clap::Parser;

use launcher::version::core::VersionReporter;
use launcher::version::inbound::cli::{dispatch, Cli};
use launcher::version::outbound::build_metadata::VergenBuildMetadata;

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    let reporter = VersionReporter::new(VergenBuildMetadata);
    dispatch(cli, &reporter)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
```

`Cli::parse()` runs before logging init, so an unknown subcommand exits via clap
before any kernel work (AC-7); a malformed `ACCELERATOR_LOG` with a valid subcommand
reaches `init` and returns `Err`, driving the reachable failure arm (AC-5).

#### 2. Black-box integration test (test-first)

**File**: `cli/launcher/tests/version.rs` (new) — replicate luminosity's
`tests/version.rs` (adjusting the binary env var to `CARGO_BIN_EXE_accelerator` and
the version prefix to `accelerator `). It asserts the four fields one-per-line by
stripping prefixes at indices 0–3, plus the non-empty / distinctness / RFC-3339
not-in-future guards. Two points the plan makes explicit so the intent is not lost
in replication:

- **Keep the `assert_eq!` reconciliation, not just `assert_ne!`.** For each git/build
  fact, assert `field == option_env!("VERGEN_*").unwrap_or("unknown")` (evaluated in
  the *test binary's* own build context). This `assert_eq!` — not the `assert_ne!` —
  is what proves the plumbing by construction, and it holds regardless of git
  presence. Do not drop it.
- **Reconcile the version line against `env!("CARGO_PKG_VERSION")`.** Assert the
  version field equals `env!("CARGO_PKG_VERSION")` (the launcher crate's Cargo
  version, resolved in the test binary's build context). This gives AC-6 (version
  single-source-of-truth) a construction-based automated guard, complementing the
  transient bump/rebuild/revert manual check rather than leaving AC-6 manual-only.
- **`assert_ne!(field, "unknown")` is git-scoped only for `commit_sha`.** Of the three
  facts, only `commit_sha` (`VERGEN_GIT_SHA`, from `GitclBuilder`) depends on git;
  `build_date` (`VERGEN_BUILD_TIMESTAMP`, from `BuildBuilder`) and `target_triple`
  (`VERGEN_CARGO_TARGET_TRIPLE`, from `CargoBuilder`) are non-git vergen outputs Cargo
  supplies on every build. So: `assert_ne!(commit_sha, "unknown")` relies on the
  git-working-tree CI invariant (CI and local dev always build inside a git checkout;
  a genuinely git-less build, which AC-2 supports and the unit tests cover by
  construction, is out of scope for this black-box test), while
  `assert_ne!(build_date, "unknown")` and `assert_ne!(target_triple, "unknown")` hold
  **unconditionally** — a failure there is a real emit/plumbing bug, not a
  git-environment artefact, and must not be excused by the invariant.

Add these accelerator-specific cases:

- **Log line at a defined level:** run with `ACCELERATOR_LOG=debug`, assert the process
  succeeds, stdout still carries the four version lines, and stderr contains the
  stable substring `reporting version` (AC-5's "emitted log line at a defined
  level"). Assert only that substring, not incidental subscriber formatting.
- **Quiet by default:** run a bare `accelerator version` (no `ACCELERATOR_LOG`), assert
  success, the four version lines on stdout, and that stderr does **not** contain
  `reporting version`. This pins the stdout/stderr split as a contract (stdout = the
  four lines; the debug event is gated at the default INFO level) and guards against a
  regression that makes the CLI noisy on every invocation.
- **Malformed filter fails cleanly:** run `version` with the **same** provably-rejected
  filter used in the Phase 1 unit test (e.g. `ACCELERATOR_LOG=bad=notalevel`), assert a
  non-zero exit and that stderr carries the `invalid log filter` message — the
  reachable `kernel::Error` path.
- **Unknown subcommand:** run `accelerator definitely-not-a-command`, assert a non-zero
  exit **and** that stderr carries clap's unrecognised-subcommand message (e.g.
  contains `unrecognized subcommand`), so the test pins the clap-rejection path
  specifically rather than accepting any non-zero exit (AC-7; no external-subcommand
  path compiled).

#### 3. Rewrite the cargo-pup rule

**File**: `cli/pup.ron`
**Changes**: retarget the rule from `launcher::domain` to `launcher::version::core`,
rename it, and permit only the shared error taxonomy from `kernel`.

```ron
// `matches` is the RESOLVED module path; `allowed_only` entries are LITERAL
// use-paths as written in source.
(
    lints: [
        Module((
            name: "version_core_imports_only_permitted",
            matches: Module("^launcher::version::core($|::)"),
            rules: [
                RestrictImports(
                    allowed_only: Some([
                        "^(std|core|alloc)(::|$)",
                        "^kernel::Error(::|$)",
                        "^crate::version::core(::|$)",
                    ]),
                    denied: None,
                    severity: Error,
                ),
            ],
        )),
    ],
)
```

The `kernel` allowance is narrowed from the whole crate (`^kernel(::|$)`) to the
error taxonomy (`^kernel::Error(::|$)`): the domain may reference the shared error
type, but not `kernel`'s infrastructure modules (`logging` today; `config`/`dispatch`
in later stories). This keeps the inward-dependency guard as tight as the actual
coupling, so a future `core → kernel::config` import is caught rather than silently
permitted. Retain the header comment that explains the matches/allowed_only mechanics
(resolved path vs literal use-path — genuinely non-obvious), but drop its ADR
reference and the "matches no module until the hexagon lands" clause, both now stale.

#### 4. Realign the pup probe fixture

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: rename the probe rule to `version_core_imports_only_permitted` and
retarget the probe module to `^pup_probe::version::core($|::)` /
`^crate::version::core(::|$)`, laying the probe out as `version/core.rs` +
`version/mod.rs`, so the fixture mirrors the shipped rule's shape, including the
narrowed `^kernel::Error(::|$)` allowance.

To keep the assertions discriminating for the *narrowing* this phase introduces (not
just the adapter axis), the probe workspace grows a two-module `kernel` (`Error` plus
one infra-like module, e.g. `logging`) and the fixture adds a kernel-axis pair:

- a probe `core` importing `kernel::Error` **passes** (the permitted taxonomy path);
- a probe `core` importing `kernel::logging` **is rejected**, naming
  `version_core_imports_only_permitted`.

This proves the `^kernel::Error(::|$)` allowance actually discriminates infra modules
from the error type by an automated test, rather than relying only on the Phase 3
manual spot-check. The existing assertions (adapter-import violation named, compliant
layout passes, `test_real_cli_pup_ron_loads` parses the shipped config) are unchanged
in intent.

### Success Criteria:

#### Automated Verification:

- [x] `accelerator version` prints four fields one-per-line, asserted by the
      integration test: `cd cli && cargo nextest run -p launcher --test version`
- [x] The unknown-subcommand and malformed-filter cases pass (part of the same test
      binary).
- [x] Full launcher + kernel unit + integration suite: `mise run test:unit:cli`
- [x] cargo-pup enforces the `version::core` inward rule:
      `mise run pup:check` (and the probe fixture regression:
      `mise run test:integration` per the architecture lane).
- [x] Format + lint clean: `mise run cli:check`
- [x] Dependency graph clean: `mise run deny:check`
- [x] Full read-only gate: `mise run check`
- [x] End-to-end default task: `mise run`
- [x] AC-6 (automated): the `crate_version` accessor unit test and the integration
      version-line reconciliation both assert against `env!("CARGO_PKG_VERSION")`:
      `cd cli && cargo nextest run -p launcher`
- [x] AC-6 (manual belt-and-braces): covered by the automated `crate_version` +
      version-line reconciliation tests; the smoke build printed line 1 as
      `accelerator 1.24.0-pre.2`, matching `[workspace.package].version`.

#### Manual Verification:

- [x] `ACCELERATOR_LOG=debug accelerator version` shows the four lines on stdout and the
      log line on stderr; a bare `accelerator version` shows only the four lines.
- [x] Introducing a temporary `core`→adapter `use` in `version/core.rs` makes
      `mise run pup:check` fail, naming `version_core_imports_only_permitted`
      (then removed) — confirms AC-4's defence-in-depth bites.
- [x] `accelerator badcmd` exits non-zero with clap's unknown-subcommand error.

---

## Testing Strategy

### Unit Tests:

- `kernel::logging::filter_from_env` — unset → Ok default, valid directive → Ok,
  malformed → asserts the specific `Error::LogFilter` variant *and* the stable
  `invalid log filter` Display substring (the reachable failure seam + the message
  contract the integration test greps for); the malformed input is the same
  provably-rejected string used in the integration test.
- `version::core` — `VersionReporter` against a `FakeBuildMetadata` returning four
  distinct constants; asserts each `VersionReport` field.
- `version::outbound::build_metadata` — `or_unknown` (`None` → `"unknown"`, `Some` →
  passthrough); each git/build accessor (`commit_sha`/`build_date`/`target_triple`)
  equals `option_env!("VERGEN_*").unwrap_or("unknown")` (AC-2 wiring, tested without a
  git-less build); and `crate_version` equals `env!("CARGO_PKG_VERSION")` (AC-6
  single-source-of-truth, automated rather than manual-only).
- `version::inbound::cli::render` — the exact four-line string (prefix + value at each
  fixed index), pinning per-field prefixes and one-per-line ordering.
- `version::inbound::cli::dispatch` — `Command::Version` against a fake reporter
  returns `Ok(())`, exercising the dispatch arm within Phase 2.

### Integration Tests:

- `cli/launcher/tests/version.rs` — black-box `accelerator version`: four fields
  one-per-line; non-empty + distinct + RFC-3339-not-future guards; each git/build
  fact reconciled with `assert_eq!(field, option_env!("VERGEN_*").unwrap_or("unknown"))`
  and the version line with `assert_eq!(version, env!("CARGO_PKG_VERSION"))` (proves
  plumbing by construction, git-presence-independent); `assert_ne!(commit_sha, "unknown")`
  under the git-working-tree CI invariant, while `assert_ne!` on `build_date`/
  `target_triple` (non-git vergen outputs) holds unconditionally; `ACCELERATOR_LOG=debug`
  emits the `reporting version` substring to stderr while stdout is unchanged; a bare
  `version` keeps stderr free of `reporting version` (quiet-by-default contract);
  malformed `ACCELERATOR_LOG` exits non-zero with `invalid log filter`; unknown
  subcommand exits non-zero with clap's `unrecognized subcommand` message.
- `tests/integration/pup/test_import_rule.py` — the retargeted probe proves the
  `version::core` inward rule is discriminating on both axes: an adapter import is
  rejected, a `kernel::logging` (infra) import is rejected, and a `kernel::Error`
  import passes — so the narrowed `^kernel::Error(::|$)` allowance is proven, not
  spot-checked; `test_real_cli_pup_ron_loads` proves the shipped `cli/pup.ron` parses.

### Manual Testing Steps:

1. `cd cli && cargo build && ./target/debug/accelerator version` — four fields.
2. `ACCELERATOR_LOG=debug ./target/debug/accelerator version` — log line on stderr.
3. `ACCELERATOR_LOG='bad=notalevel' ./target/debug/accelerator version` — non-zero,
   error on stderr. (Note `:::` is *not* rejected — EnvFilter is lenient; an invalid
   level like `bad=notalevel` is the reliable rejection.)
4. `./target/debug/accelerator nope` — non-zero, clap error.
5. Temporarily add a `core`→adapter `use`, run `mise run pup:check`, confirm failure.

## Performance Considerations

None. `version` does no I/O beyond parsing, one `println!`, and (when enabled) one
`tracing` event. Build-time metadata is resolved once by the build script; there is
no runtime cost. The added `tracing-subscriber`/`thiserror` closure affects compile
time and binary size marginally and is justified by the cross-cutting kernel role.

## Migration Notes

- Adding the `kernel` member and the `tracing`/`thiserror` closure regenerates
  `cli/Cargo.lock`; run `cargo build` in `cli/` and commit the lockfile. Confirm the
  closure introduces no `native-tls`/`openssl` edge (`deny.toml` bans them).
- No data or on-disk schema migration; this is greenfield workspace growth.
- **`cli/Cargo.lock` is the compatibility anchor.** The caret ranges (`tracing 0.1`,
  `tracing-subscriber 0.3`, `thiserror 1`, `time 0.3`, `clap 4.6`) are resolved and
  frozen in the committed lockfile, which both the stable product build and the frozen
  `nightly-2026-01-22` cargo-pup lane build. The reachability check (`EnvFilter::parse`
  rejects the chosen filter) and the `invalid log filter` / `unrecognized subcommand`
  substring assertions are committed tests, so any future `cargo update` that drifts
  `tracing-subscriber`'s parser/`ParseError` or clap's diagnostics is caught by
  `mise run check` before merge — a `cargo update` is a deliberate, gated action, not a
  silent float. If a member's MSRV ever outruns the pup nightly, pin that member rather
  than loosening the gate.
- **Build-time prerequisite (SHA only):** a real (non-`unknown`) commit SHA requires a
  `git` CLI on the build host *and* a `.git` working tree; `vergen-gitcl` shells out to
  `git`. CI and local dev always build inside a git checkout, so this holds — but it is
  a build-environment invariant, not a runtime guarantee. A build lacking either
  condition still compiles and prints `unknown` for the SHA (AC-2). Only the
  integration test's `assert_ne!(commit_sha, "unknown")` depends on this invariant; the
  build-date and target-triple facts are non-git vergen outputs Cargo always supplies,
  so their `assert_ne!` guards hold unconditionally.
- **musl-static is graph-checked, not build-checked here:** `deny.toml` evaluates the
  musl target triples and bans `native-tls`/`openssl`, but this story does no
  cross-compilation, so the new closure is not actually built/linked under musl. That
  guarantee remains unverified-by-build until the cross-compile story (per
  `deny.toml`'s own "necessary but not sufficient" note).

## References

- Original work item: `meta/work/0163-scaffold-cli-workspace-version-subcommand.md`
- Research: `meta/research/codebase/2026-07-02-0163-cli-workspace-version-subcommand-scaffold.md`
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`,
  `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
- Luminosity blueprint: `../luminosity/cli/` — root `Cargo.toml`,
  `launcher/build.rs`, `launcher/src/version/**`, `launcher/tests/version.rs`,
  `pup.ron`, `kernel/src/lib.rs`
- Server Rust precedent: `skills/visualisation/visualise/server/src/log.rs`
