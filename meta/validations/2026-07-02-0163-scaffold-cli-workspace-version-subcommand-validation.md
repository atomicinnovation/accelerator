---
type: plan-validation
id: "2026-07-02-0163-scaffold-cli-workspace-version-subcommand-validation"
title: "Validation Report: Scaffold the cli/ Hexagonal Workspace with a version Subcommand"
date: "2026-07-03T10:53:41+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-07-02-0163-scaffold-cli-workspace-version-subcommand"
target: "plan:2026-07-02-0163-scaffold-cli-workspace-version-subcommand"
tags: [rust, cli, hexagonal, scaffold, workspace, version, kernel]
last_updated: "2026-07-03T10:53:41+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Scaffold the cli/ Hexagonal Workspace with a version Subcommand

### Implementation Status

✓ Phase 1: kernel crate — error taxonomy and logging facility — **Fully implemented**
✓ Phase 2: version hexagon and build-metadata injection — **Fully implemented**
✓ Phase 3: launcher composition root, integration tests, and cargo-pup rule — **Fully implemented**

All three phases map cleanly to their commits:

- `6f445c537153` Add kernel crate with error taxonomy and logging facility (Phase 1)
- `3205bce21ad1` Add version hexagon and vergen build-metadata injection (Phase 2)
- `f60df2765163` Wire the version composition root, integration tests, and pup rule (Phase 3)

Every file the plan specified exists and matches the intended shape; no
planned change is missing and no unplanned file crept in.

### Automated Verification Results

All commands run from a clean working tree at `e25f077efdf1`:

✓ Workspace builds: `cd cli && cargo build`
✓ Full unit + integration suite (17 tests, 4 binaries): `cd cli && cargo nextest run` — 17 passed
✓ Format + lint clean: `mise run cli:check`
✓ Dependency graph clean: `mise run deny:check` — advisories/bans/licenses/sources all ok
✓ cargo-pup architecture lane: `mise run pup:check`
✓ pup probe fixture regression: `uv run pytest tests/integration/pup/test_import_rule.py` — 5 passed
✓ Full read-only gate: `mise run check` — exit 0 (all four components)

The bare default task (`mise run`) was not re-run in this session (it
reformats in place and rebuilds every component); its read-only mirror
`mise run check` passing, together with all cli-specific test suites
passing, gives equivalent confidence for a cli-only change.

### Code Review Findings

#### Matches Plan:

- **kernel error taxonomy** (`cli/kernel/src/lib.rs`) — `Error::LogFilter`
  wraps `tracing_subscriber::filter::ParseError` via `#[from]`, Display
  renders `invalid log filter: {0}`, preserving the source chain exactly as
  specified.
- **logging facility** (`cli/kernel/src/logging.rs`) — pure `filter_from_env`
  seam + thin `init()` wrapper reading `ACCELERATOR_LOG` (namespaced, not
  `RUST_LOG`); the three unit tests pin unset/valid/malformed paths.
- **version domain** (`core.rs`) — the two ports, `VersionReport` value
  object, and `VersionReporter` service; `core` imports no adapter/I/O.
- **outbound adapter** (`build_metadata.rs`) — `or_unknown` degradation helper
  plus per-accessor tests that route each real `option_env!` key through it
  (catching a bypass or wrong key), and `crate_version` reconciled against
  `env!("CARGO_PKG_VERSION")`.
- **inbound clap adapter** (`cli.rs`) — `disable_version_flag`, single
  `Version` subcommand, exact four-line `render` with the position-sensitive
  `built:  ` two-space prefix reproduced verbatim; `dispatch` emits the
  `reporting version` debug line then prints.
- **build.rs** — replicates the luminosity blueprint, omits `fail_on_error()`,
  keeps only the non-obvious rationale comment.
- **composition root** (`main.rs`) — parse → init logging → dispatch, mapping
  `kernel::Error` to a non-zero `ExitCode`.
- **integration test** (`tests/version.rs`) — keeps the `assert_eq!`
  reconciliation against `option_env!`/`env!` (plumbing proven by
  construction), version-line reconciliation for AC-6, the git-scoped vs
  unconditional `assert_ne!` split, the RFC-3339 not-in-future guard, and the
  four accelerator-specific cases (debug log line, quiet-by-default,
  malformed filter, unknown subcommand).
- **pup rule** (`cli/pup.ron`) — retargeted to `^launcher::version::core`,
  renamed `version_core_imports_only_permitted`, with the narrowed
  `^kernel::Error(::|$)` allowance (not the whole crate).
- **pup fixture** (`test_import_rule.py`) — mirrors the shipped rule shape and
  adds the kernel-axis pair proving the narrowed allowance discriminates
  `kernel::Error` (passes) from `kernel::logging` (rejected).

#### Deviations from Plan (all minor, all improvements):

- `cli/kernel/src/logging.rs:54-59` — the malformed-directive test uses
  `matches!(result, Err(...))` + `if let Err` rather than the plan's
  `unwrap_err()`, avoiding the `unwrap_used` clippy opt-in. Same two
  assertions (variant + `invalid log filter` substring); intent preserved.
- `cli/launcher/tests/version.rs:20` — the test harness calls
  `command.env_remove("ACCELERATOR_LOG")` before applying per-case env, making
  the quiet-by-default and metadata cases robust against an ambient
  `ACCELERATOR_LOG` in the runner's shell. A robustness addition beyond the
  plan text.

No behavioural deviations; the two above are test-hardening.

#### Potential Issues (non-blocking observations):

- **Duplicate `thiserror-impl`** — `cargo deny` emits a `duplicate` *warning*:
  `thiserror 1.0.69` (kernel's runtime dep) and `thiserror 2.0.18` (pulled
  transitively through vergen's build-dep `cargo_metadata`). `deny.toml` is
  configured to warn, not fail, so the gate stays green. It is a build-dep vs
  runtime-dep split with no runtime impact, but it is not called out in the
  plan's Migration Notes; worth a one-line note there if the closure is
  revisited.
- **nextest LEAK flag** — `a_missing_fact_degrades_to_unknown` is reported
  `1 leaky` (passed). This is a nextest handle/subprocess-leak heuristic on a
  pure-function test; benign and not reproducible as a failure, but noted.

### Manual Testing Required:

Performed during validation (binary at `cli/target/debug/accelerator`):

1. `accelerator version` → four fields, one per line, exit 0:
  - [x] `accelerator 1.24.0-pre.2` (line 1 matches `[workspace.package].version` — AC-6)
  - [x] `commit:`, `built:` (RFC-3339), `target: aarch64-apple-darwin`
2. Logging:
  - [x] `ACCELERATOR_LOG=debug accelerator version` → `reporting version` on stderr, four lines on stdout
  - [x] bare `accelerator version` → stderr free of `reporting version`
3. Error paths:
  - [x] `ACCELERATOR_LOG=bad=notalevel accelerator version` → exit 1, `invalid log filter` on stderr
  - [x] `accelerator nope` → exit 2, `unrecognized subcommand` on stderr
4. Architecture guard (AC-4 defence-in-depth):
  - [x] Injecting a `use crate::version::outbound::...` into `core.rs` makes
    `mise run pup:check` fail with a module-import rule violation (rule-name
    emission proven by the fixture's `test_core_importing_adapter_is_rejected`
    assertion). Working tree restored clean afterward.

No outstanding manual steps.

### Recommendations:

- Optionally add a one-line note to the plan/epic Migration Notes recording
  the benign `thiserror 1 vs 2` build-dep duplicate, so a future closure edit
  is not surprised by the `deny` warning.
- No code changes required before merge. The slice is complete, test-first,
  and the architecture guard is proven to bite on both the adapter and the
  kernel-infra axes.
