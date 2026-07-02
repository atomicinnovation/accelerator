---
type: codebase-research
id: "2026-07-02-0163-cli-workspace-version-subcommand-scaffold"
title: "Research: Scaffolding the cli/ Hexagonal Workspace with a version Subcommand (0163)"
date: "2026-07-02T22:44:46+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0163"
parent: "work-item:0163"
topic: "Scaffolding the cli/ hexagonal workspace with a version subcommand"
tags: [research, codebase, rust, cli, hexagonal, version, vergen, cargo-pup, kernel]
revision: "66168546461febe7a502fb44117ff16d4837c03a"
repository: "accelerator"
last_updated: "2026-07-02T22:44:46+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Scaffolding the cli/ Hexagonal Workspace with a version Subcommand (0163)

**Date**: 2026-07-02T22:44:46+00:00
**Author**: Toby Clemson
**Git Commit**: 66168546461febe7a502fb44117ff16d4837c03a
**Branch**: HEAD (main)
**Repository**: accelerator

## Research Question

What does the codebase currently provide — and what reference patterns exist —
to implement work item 0163: scaffolding the `cli/` hexagonal workspace with a
`version` subcommand? Specifically: the post-0162 state of `cli/`, how the mise
task tree picks up a new crate, the binding ADR/spike constraints, the luminosity
0007 replication blueprint (workspace, `build.rs`, vergen pins, kernel, the
`version` hexagon and its tests), and the in-repo Rust conventions the new code
should match.

## Summary

0162 landed a **minimal one-member `cli/` workspace**: a workspace root
(`Cargo.toml`, `deny.toml`, `pup.ron`, `rustfmt.toml`) plus a single `launcher`
crate whose `main.rs` is `const fn main() {}` and whose `lib.rs` only exposes a
self-test `crate_name()`. There is **no `kernel` crate, no `build.rs`, and no
`version` module tree** yet — those are exactly what 0163 adds.

The luminosity repo at `../luminosity` is a near-complete replication blueprint
for the source layout, `build.rs`, vergen pins, the `version` hexagon
(`version/{core, inbound/cli, outbound/build_metadata}`), and the black-box
integration test. Copying its structure gets 0163 most of the way.

Three findings need a decision **before** implementation (details in Open
Questions):

1. **The logging acceptance criterion has no luminosity precedent.** WI-0163 AC-5
   requires the version slice to *"initialise logging through the kernel logging
   facility ... verifiable by ... an emitted log line at a defined level."*
   Luminosity's 0007 has **no logging facility at all** — no `tracing`, no init
   function, and its `kernel` crate has zero dependencies. Either 0163 goes
   beyond the mirror and adds a minimal logging facility (the repo's blessed
   stack is `tracing` + `tracing-subscriber`, per the visualiser server), or AC-5
   is softened to match luminosity.
2. **The `kernel` error taxonomy in luminosity is an uninhabited `pub enum
   Error {}`** consumed only as a `Result<(), kernel::Error>` seam — no
   `thiserror`, no variants. WI-0163 AC-5 ("errors expressed via the kernel error
   taxonomy") is satisfiable by that seam, but it is a placeholder, not a real
   taxonomy.
3. **The existing `cli/pup.ron` rule targets the wrong module path.** It matches
   `launcher::domain` / `crate::domain`, but the hexagon layout (both WI-0163 and
   luminosity) uses `version::core`. 0163 must rewrite the rule to target
   `launcher::version::core` and additionally permit `^kernel(::|$)`.

## Detailed Findings

### 1. Current state of `cli/` (post-0162)

The entire non-artifact source surface is **8 files** (ignore everything under
`cli/target/` and `cli/.pup/`):

- `cli/Cargo.toml` — workspace root: `resolver = "2"`, `members = ["launcher"]`;
  `[workspace.package]` sets `version = "1.24.0-pre.2"`, `edition = "2021"`;
  `[workspace.lints.rust] warnings = "deny"`; `[workspace.lints.clippy]` =
  pedantic+nursery at `warn` (priority -1) plus restriction opt-ins
  (`unwrap_used`, `expect_used`, `panic`, `dbg_macro`, `todo`, `unimplemented`)
  and two allows (`module_name_repetitions`, `must_use_candidate`).
  **No `[workspace.dependencies]` table**, **no `[profile.*]`**.
- `cli/launcher/Cargo.toml` — `name = "launcher"`, `version.workspace = true`,
  `edition.workspace = true`, `license = "MIT"`, `[lints] workspace = true`;
  `[[bin]] name = "accelerator", path = "src/main.rs"`. **No `[dependencies]`,
  no `[build-dependencies]`, no `build = "build.rs"`.**
- `cli/launcher/src/main.rs` — `const fn main() {}` (does nothing).
- `cli/launcher/src/lib.rs` — `pub const fn crate_name() -> &'static str`
  returning `"launcher"` + a `#[cfg(test)]` self-test.
- `cli/rustfmt.toml` — `max_width = 80`, `edition = "2021"` (a Python guard in
  `tests/unit/tasks/test_version.py` asserts this literal matches `Cargo.toml`).
- `cli/deny.toml` — targets = 4 shipped triples + `x86_64-unknown-linux-gnu`;
  advisories `unmaintained="all"`/`yanked="deny"`; permissive license allow-list;
  bans block `native-tls`/`openssl`/`openssl-sys` (rustls-only, protects the
  musl-static build); `skip`/`skip-tree` empty.
- `cli/pup.ron` — one `Module` rule `domain_imports_only_permitted` matching
  `^launcher::domain($|::)`, `allowed_only = ["^(std|core|alloc)(::|$)",
  "^crate::domain(::|$)"]`, severity Error. **Near-vacuous** until a hexagon
  lands — and it targets `domain`, not `version::core` (see §5).

**Gap summary — what 0163 must add:** a `kernel` crate + `members` entry; a
`[workspace.dependencies]` table (clap, vergen, vergen-gitcl); the `version`
hexagon module tree; a `build.rs` in `launcher`; real `main.rs` (clap dispatch);
and a rewritten `pup.ron` rule.

### 2. mise / invoke task-tree wiring — new crate is picked up automatically

Every `cli/` quality gate runs **workspace-wide** against `cli/Cargo.toml`; none
enumerate crates, so a new member is enforced the moment it is added to
`[workspace].members`:

- `format:cli:check/fix` → `tasks/format/cli.py` → `cargo fmt --all`.
- `lint:cli:check/fix` → `tasks/lint/cli.py` → `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.
- `deny:check` → `tasks/deny.py` → `cargo deny check ...` (whole graph).
- `pup:check` → `tasks/pup.py` → `cargo +<nightly> pup` (reads `cli/pup.ron`).
- `test:unit:cli` → `tasks/test/cli.py` → `cargo llvm-cov nextest --workspace
  --summary-only` (coverage folded in, report-only; `ACCELERATOR_COVERAGE=off`
  drops to plain `cargo nextest run`).

Aggregation:
- `cli:check` = `format:cli:check` + `lint:cli:check` (no tests).
- top-level `check` = frontend/server/**cli:check** + **deny:check** + **pup:check**
  + build-system + scripts.
- top-level `default` covers cli via `lint:check`, `format:fix`, `deny:check`,
  `pup:check`, and `test` (→ `test:unit:cli`).

**cargo-pup nightly lane:** matched pair `PUP_NIGHTLY = "nightly-2026-01-22"` /
`PUP_VERSION = "0.1.8"` in `tasks/shared/rust.py`; provisioned only by
`deps:install:pup`; product build + all other checks stay on stable `1.90.0`.
`ACCELERATOR_PUP_MODE=warn` downgrades findings to advisory (default fail-closed).

**Version-coherence caveat:** `tasks/build.py` (`_pinned_member_versions`) reads
`[workspace].members` dynamically and asserts each member either inherits
`version.workspace = true` or agrees with the workspace version. **Have `kernel`
use `version.workspace = true`** (as `launcher` does) to stay clean. Tool pins:
cargo-deny `0.19.8`, cargo-nextest `0.9.138`, cargo-llvm-cov `0.8.7`.

### 3. Binding constraints from ADR-0053, ADR-0054, spike 0158

- **ADR-0053 (hexagon):** domain/application core depends on no infrastructure;
  ports = traits (both inbound/driving and outbound/driven), expressed in the
  domain's own terms. The CLI is the *primary inbound adapter* — thin, parse +
  present only. Adapters depend on the core; the core depends on neither. A
  hexagon **begins as one crate with layers as modules**; the concrete module
  layout is settled by *this* scaffold. In the single-crate phase, **cargo-pup is
  the sole enforcer** of the inward rule; the ADR **deliberately omits** the grep
  tripwire the spike proposed (follow the ADR).
- **ADR-0054 (composition):** `kernel` is deliberately dependency-light (error
  taxonomy, config-access + dispatch/launcher contracts, logging — "everything
  links it"). `version` and `config` are **built-in** subcommands compiled into
  the launcher; external-subcommand dispatch is the *growth* mechanism and is
  **deferred**. clap 4.x derive `#[command(external_subcommand)] Vec<OsString>`
  and Unix `exec` dispatch are 0164's concern. Workspace-wide HTTP stack is
  `reqwest` + rustls (also deferred).
- **Spike 0158 (layout):** two axes — binary axis (one crate per shippable
  sub-binary) + layering axis (hexagonal layers as modules). `kernel` holds the
  cross-cutting contracts; each subdomain starts single-crate with
  `domain`/`application` (ports) + `inbound/` + `outbound/` modules; reference is
  howtocodeit/hexarch. Splitting layers into crates is deferred until pressure.
- **Gaps in the ADR/spike set (resolved elsewhere):** the `launcher`-crate rename
  + top-level `cli/` directory decision, and the vergen build-metadata mechanism,
  are **not** in these three docs — they come from WI-0163's own body and the
  luminosity reference. The grep tripwire is a spike↔ADR divergence; the ADR wins
  (no tripwire).

### 4. Luminosity 0007 replication blueprint (`../luminosity/cli/`)

**Workspace root** (`cli/Cargo.toml`): `resolver = "2"`, `members =
["launcher", "kernel"]`. Note luminosity has **no `[workspace.package]`** (crates
pin `version`/`edition` inline) — accelerator already uses `[workspace.package]`
and can keep its own style. `[workspace.dependencies]`:

```toml
clap = { version = "4.6", features = ["derive"] }
# vergen pinned EXACTLY: 9.1.0's vergen-lib bump is incompatible with
# vergen-gitcl 1.0.8; 10.x needs a newer toolchain.
vergen = { version = "=9.0.6", default-features = false }
vergen-gitcl = { version = "1", features = ["build", "cargo"] }
time = { version = "0.3", features = ["parsing"] }   # dev-only
```

`[workspace.lints]` matches accelerator's exactly (rust `warnings = "deny"`;
clippy pedantic+nursery + the same restriction opt-ins/allows).

**launcher crate:** binary name matches package name; `[dependencies]` = `kernel =
{ path = "../kernel" }`, `clap = { workspace = true }`; `[build-dependencies]` =
`vergen` + `vergen-gitcl` (both `workspace = true`); `[dev-dependencies]` = `time`.

**kernel crate:** `publish = false`, `[lints] workspace = true`, **zero
dependencies** (no thiserror, no tracing).

**`launcher/build.rs` (verbatim — replicate this):**

```rust
//! Emits the `version` subcommand's build metadata via vergen.
//!
//! `fail_on_error()` is intentionally not called: vergen degrades a git-less or
//! shallow build to a placeholder rather than failing to compile.

use std::error::Error;

use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder};

const SHORT_SHA: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default().target_triple(true).build()?;
    let gitcl = GitclBuilder::default().sha(SHORT_SHA).build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&gitcl)?
        .emit()?;
    Ok(())
}
```

Emits `VERGEN_BUILD_TIMESTAMP`, `VERGEN_CARGO_TARGET_TRIPLE`, `VERGEN_GIT_SHA`
(full SHA). No `fail_on_error()` → git-less/shallow builds degrade to a
placeholder.

**luminosity `pup.ron`** — the rule 0163 should mirror (see §5):
`Module("^luminosity::version::core($|::)")`, `RestrictImports { allowed_only:
["^(std|core|alloc)(::|$)", "^kernel(::|$)", "^crate::version::core(::|$)"],
severity: Error }`.

**luminosity mise/tasks:** same shape as accelerator. Env prefix is
`LUMINOSITY_*` there vs `ACCELERATOR_*` here (`_PUP_MODE`, `_COVERAGE`).

### 5. The `version` hexagon (source blueprint)

Module tree inside `launcher/src/version/`:

```
version/mod.rs            -> pub mod core; pub mod inbound; pub mod outbound;
version/core.rs           -> domain (no `use` at all)
version/inbound/mod.rs    -> pub mod cli;
version/inbound/cli.rs    -> clap driving adapter
version/outbound/mod.rs   -> pub mod build_metadata;
version/outbound/build_metadata.rs -> vergen driven adapter
```

**core.rs (domain — imports nothing):** an outbound port `trait BuildMetadata`
(`crate_version`/`commit_sha`/`build_date`/`target_triple`, all `-> &'static
str`), a `VersionReport` value object (four owned `String`s), an inbound port
`trait ReportVersion { fn report(&self) -> VersionReport; }`, and the application
service `VersionReporter<M: BuildMetadata>` (with `const fn new`) implementing
`ReportVersion`. A `#[cfg(test)]` unit test drives it against a
`FakeBuildMetadata`.

**inbound/cli.rs (thin adapter):** clap derive —
`#[command(name = "...", disable_version_flag = true)]` on `struct Cli` (the
`disable_version_flag` frees the explicit `version` subcommand to own that
surface), a `Command` enum with a single `Version` variant, a `render(&report)
-> String` producing exactly four prefixed lines, and `pub fn dispatch(cli, &impl
ReportVersion) -> Result<(), kernel::Error>` that matches the subcommand, prints,
returns `Ok(())`.

**outbound/build_metadata.rs (driven adapter):** unit struct implementing
`BuildMetadata`, reading build-time env: `env!("CARGO_PKG_VERSION")` for the
version (always present) and `option_env!("VERGEN_*").unwrap_or("unknown")` for
the three git/build facts — this is the git-less `"unknown"` degradation.

**kernel wiring:** the only kernel touchpoint is `dispatch`'s `Result<(),
kernel::Error>` return + `main.rs`'s `match error {}` never-arm. `kernel::Error`
is an **uninhabited `pub enum Error {}`** with a manual `Display`
(`match *self {}` behind `#[expect(clippy::uninhabited_references)]`) and manual
`impl std::error::Error`. **No logging init exists anywhere.**

**main.rs (composition root):** `Cli::parse()` → build `VergenBuildMetadata` →
`VersionReporter::new(...)` → `dispatch(&cli, &reporter)` → map `Ok` to
`ExitCode::SUCCESS`, `Err` to `match error {}`. No `#[command(external_subcommand)]`
anywhere — external dispatch is cleanly deferred, and an unknown subcommand exits
non-zero via clap (satisfies WI-0163 AC-7).

**Tests (two layers):**
- Unit (`core.rs` `#[cfg(test)]`): `FakeBuildMetadata` returns four distinct
  constants; asserts each `VersionReport` field.
- Integration (`launcher/tests/version.rs`): shells out via
  `Command::new(env!("CARGO_BIN_EXE_luminosity")).arg("version")`, splits stdout,
  and asserts the **four fields one-per-line** by stripping prefixes at indices
  0-3 (this is the "one field per line" AC). Extra guards worth mirroring:
  non-empty fields, a `HashSet` distinctness check (guards a dropped/swapped
  field), an RFC-3339 parse of the build date with a not-in-the-future bound
  (uses the `time` dev-dep), and `assert_ne!(field, "unknown")` for the three git
  facts (valid because CI/local always builds inside a git tree, keeping the
  equality checks discriminating). Note: there is **no test that forces the
  git-less path** to observe `"unknown"` at runtime — the placeholder is only
  proven by construction (see Open Questions re AC-2).

### 6. In-repo Rust conventions (visualiser server)

The server (`skills/visualisation/visualise/server/`) is the existing Rust
precedent. Blessed patterns/versions to match for consistency:

- **thiserror `1`** (not 2.x) — per-module error enums, named fields, `#[from]`,
  `#[source]`, `#[error(transparent)]`. (`anyhow` is declared but has **zero
  usages** — not a blessed pattern.)
- **tracing `0.1` + tracing-subscriber `0.3`** (features `json`, `env-filter`) —
  init in `src/log.rs::init()` builds a JSON subscriber, `RUST_LOG` env filter,
  non-blocking rotating **file** writer, `try_init()` mapping the
  already-init error. The binary holds the returned `WorkerGuard`. **Note:** the
  server logs to a file with stdout/stderr sent to `/dev/null`; a CLI tool almost
  certainly wants a **stderr `fmt` subscriber** instead.
- **clap `4` derive** — `#[derive(Parser)]`, `#[command(version, about)]`,
  doc-comment help, `PathBuf` args.
- Version via `env!("CARGO_PKG_VERSION")`; edition 2021; **caret major-only**
  pinning (server style — differs from the `cli/` workspace's exact vergen pin);
  in-manifest `[lints.clippy]`; crate-local `rustfmt.toml` (`max_width = 80`).
- Tests: both `#[cfg(test)]` modules and `tests/*.rs`; plain `assert!`/`assert_eq!`;
  CLI integration via `assert_cmd = "2"` + `predicates = "3"`. **The server runs
  under plain `cargo test`; nextest + llvm-cov is the `cli/` workspace convention.**
- `[profile.release]` in the server: `lto = "thin"`, `codegen-units = 1`,
  `strip = true`, `opt-level = 3` — a precedent if the cli workspace later wants a
  release profile (currently absent).

## Code References

- `cli/Cargo.toml` — workspace root; add `kernel` to `members`, add
  `[workspace.dependencies]`.
- `cli/launcher/Cargo.toml` — add `[dependencies]` (kernel, clap) +
  `[build-dependencies]` (vergen, vergen-gitcl).
- `cli/launcher/src/main.rs:1` — `const fn main() {}` to be replaced by the
  composition root.
- `cli/pup.ron:12-29` — rule targets `launcher::domain`; must become
  `launcher::version::core` + allow `^kernel(::|$)`.
- `tasks/shared/paths.py:27-36` — `cli_member_manifests()` reads members
  dynamically.
- `tasks/build.py:74-101` — version-coherence over members (kernel should use
  `version.workspace = true`).
- `tasks/shared/rust.py:6-7` — `PUP_NIGHTLY`/`PUP_VERSION` matched pair.
- `../luminosity/cli/launcher/build.rs:1-22` — the `build.rs` to replicate.
- `../luminosity/cli/launcher/src/version/{core,inbound/cli,outbound/build_metadata}.rs`
  — the hexagon to replicate.
- `../luminosity/cli/launcher/tests/version.rs` — the black-box test to replicate.
- `../luminosity/cli/pup.ron` — the correct `version::core` pup rule.
- `../luminosity/cli/kernel/src/lib.rs:1-19` — the uninhabited `Error` seam.
- `skills/visualisation/visualise/server/src/log.rs:45-55` — the repo's blessed
  tracing init (reference if 0163 adds logging).

## Architecture Insights

- **Module-tree hexagon, not crate-per-layer.** `version` lives inside `launcher`
  as `version/{core, inbound, outbound}`; the inward-dependency rule is enforced
  at module granularity by cargo-pup (the sole enforcer while single-crate).
- **Single source of truth for version** = `env!("CARGO_PKG_VERSION")` (the
  launcher crate's Cargo version); build metadata is injected by vergen at build
  time, never hard-coded — satisfying AC-6 (bump the crate version, output
  changes with no other edit).
- **Graceful degradation over build failure** is a deliberate design choice: no
  `fail_on_error()`, `option_env!(...).unwrap_or("unknown")` — a git-less/shallow
  clone still compiles and runs.
- **Exact vergen pin (`=9.0.6`)** is load-bearing: transitive `vergen-lib` bumps
  break the `Emitter` across minor versions. This is the one place the `cli/`
  workspace deviates from the server's caret-pinning convention.
- **Task tree is member-agnostic** — adding a crate needs no task edits, only a
  `members` entry (+ `version.workspace = true` for coherence).

## Historical Context

- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  — the hexagon structure and inward-dependency rule; deliberately omits the grep
  tripwire.
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  — built-in vs external subcommands; `version` is built-in; external dispatch
  deferred to 0164.
- `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
  — the spike that landed the workspace layout (kernel + per-subdomain hexagons).
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — the parent research 0163 derives from.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`

## Open Questions

1. **Does 0163 add a kernel logging facility, or is AC-5's logging clause
   descoped?** Luminosity 0007 has none. If it stays, the blessed stack is
   `tracing` + `tracing-subscriber` (server precedent) — but a CLI wants a
   **stderr `fmt` subscriber**, not the server's JSON-to-file setup, so there is
   no drop-in mirror; a small new `kernel::logging::init()` would have to be
   designed. Recommend deciding this explicitly before planning — it is the
   largest divergence from the reference.
2. **How is AC-2 (git-less `unknown` placeholder) tested?** Luminosity proves the
   placeholder by construction only; its runtime integration test asserts the
   *non*-degraded path (`assert_ne!(field, "unknown")`). AC-2 asks for a test
   that the build "still succeeds and prints `unknown`" — satisfying that
   literally may need a unit test that injects a fake returning `"unknown"`
   through the `BuildMetadata` port (trivial with the port design), rather than
   forcing an actual git-less build.
3. **Is the `kernel::Error` uninhabited-enum seam sufficient for AC-5's "errors
   expressed via the kernel error taxonomy"?** Luminosity uses exactly that. If a
   real (thiserror-backed, `1.x` to match the server) taxonomy is wanted instead,
   that is a deliberate step beyond the mirror.
4. **`pup.ron` rewrite scope:** confirm the module path prefix — accelerator's
   binary crate is `launcher`, so the rule becomes
   `^launcher::version::core($|::)` with `allowed_only` including `^kernel(::|$)`
   and `^crate::version::core(::|$)` (vs luminosity's `luminosity::` prefix).
   Also confirm whether the pup fixture regression under `tests/integration/pup/`
   needs updating for the new rule.
