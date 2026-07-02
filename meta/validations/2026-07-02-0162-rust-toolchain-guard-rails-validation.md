---
type: plan-validation
id: "2026-07-02-0162-rust-toolchain-guard-rails-validation"
title: "Validation Report: Rust Toolchain Guard Rails in mise + CI"
date: "2026-07-02T20:52:50+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-07-02-0162-rust-toolchain-guard-rails"
target: "plan:2026-07-02-0162-rust-toolchain-guard-rails"
relates_to: []
tags: [rust, tooling, ci, guard-rails, architecture-enforcement]
last_updated: "2026-07-02T20:52:50+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Rust Toolchain Guard Rails in mise + CI

### Implementation Status

✓ Phase 1: Minimal `cli/` workspace + version-coherence generalisation — Fully implemented
✓ Phase 2: Workspace-wide format + clippy (`cli:check`) into `check` + CI — Fully implemented
✓ Phase 3: cargo-deny (bans) into `check` + CI + regression — Fully implemented
✓ Phase 4: cargo-pup nightly lane into `check` + CI + regression (isolated) — Fully implemented
✓ Phase 5: Tests + coverage (`cargo llvm-cov nextest`) into `test` — Fully implemented

Each phase maps to one atomic commit, in order:

- `klpsyznr` — Add minimal cli/ Rust workspace and generalise version coherence
- `xzsqlmlx` — Add workspace-wide cli format + clippy checks to check and CI
- `yqkkpznr` — Add cargo-deny supply-chain check to check and CI
- `ruznvtyw` — Add cargo-pup architecture check on an isolated nightly lane
- `xsoqqkrn` — Add cli test + coverage run via cargo llvm-cov nextest

Working tree is clean; no uncommitted drift.

### Automated Verification Results

✓ Full read-only gate passes: `mise run check` (exit 0; `cli:check`, `deny:check`, and `pup:check` all observed running within it)
✓ Task unit tests pass: `test_build.py test_version.py test_rust.py test_mise.py test_workflows.py` — 78 passed
✓ cargo-deny ban regression passes: `mise run test:integration:deny` — 2 passed (violation fires + clean passes, offline against real `cli/deny.toml`)
✓ cargo-pup architecture regression passes: `mise run test:integration:pup` — 3 passed (rejection + positive control + real-config-loads)
✓ `pup:check` clean on the workspace: `mise run pup:check` (exit 0, nightly-2026-01-22 / cargo-pup 0.1.8)
✓ Coverage leaf emits a summary, not gated: `mise run test:unit:cli` — 1 test via nextest, TOTAL 77.78% region coverage, exit 0
✓ Version coherence holds: `version:read` = `1.24.0-pre.2`, equal across `plugin.json`, server `Cargo.toml`, and `cli/Cargo.toml`

### Code Review Findings

#### Matches Plan:

- `cli/Cargo.toml` virtual workspace with the exact resolved clippy lint set from the plan; version seeded to the live plugin version (`1.24.0-pre.2`) as instructed, not the draft's `-pre.7` literal.
- `tasks/shared/rust.py` holds `LAUNCHER_CRATE`, the `PUP_NIGHTLY`/`PUP_VERSION` matched pair (`nightly-2026-01-22` / `0.1.8`), `pup_mode()` (default `deny`, fail-closed with a visible WARNING on unrecognised values), and `coverage_enabled()` (default on) — all read at call time as required.
- `cli/deny.toml` bans `native-tls`/`openssl`/`openssl-sys`, opts both `[advisories]` and `[licenses]` into `version = 2`, names the five explicit `[graph].targets`, and scaffolds the infra-out-of-domain ban with empty `skip`/`skip-tree` plus the documented wrapper mechanism.
- `cli/pup.ron` encodes the inward-dependency `RestrictImports` rule with `severity: Error` against `launcher::domain`, with the near-vacuous-until-scaffold caveat documented inline.
- mise wiring exactly as specified: `cli:check` = `format:cli:check` + `lint:cli:check`; `check.depends` carries `cli:check`, `deny:check`, `pup:check`; bare `default` adds them plus `test`. `test:integration:deny` is in the `test:integration` roll-up; `test:integration:pup` is deliberately excluded (grep count 0). `test:unit:cli` is in `test:unit` and not reachable from `check`.
- CI: `check-cli`, `check-supply-chain`, `check-architecture` jobs all present and added to `prerelease.needs`.
- Tool pins match the plan: cargo-deny 0.19.8, cargo-nextest 0.9.138, cargo-llvm-cov 0.8.7, each with a why-pinned inline comment; committed `mise.lock` carries entries for linux-x64, macos-arm64, macos-x64.
- Docs: `tasks/README.md` and `CLAUDE.md` updated; the CI-job → local-command mapping is present (5 references).
- Hand-duplicated 80-col width holds across `cli/deny.toml`, `cli/pup.ron`, `cli/rustfmt.toml`, `cli/Cargo.toml` (no line exceeds 80).

#### Deviations from Plan:

- None material. The version literal differs from the plan draft (`1.24.0-pre.2` vs the draft's `1.24.0-pre.7`), which is the plan's own explicit instruction (read from `plugin.json` at implementation time), not a deviation.

#### Potential Issues:

- The `cli/` workspace is single-crate at this milestone, so cargo-deny's cross-crate bans and cargo-pup's domain/adapter rule are proven only by the fixture regressions, not by shipped code — an accepted, documented co-verification boundary that first bites at 0163/0166. Tracked in the plan's Desired End State; no action here.
- `check-visualiser-server` is knowingly left without a `Swatinem/rust-cache` target cache (its fold-in is 0168). Documented; no action.

### Manual Testing Required:

These items are inherently CI-only or require an open PR and cannot be exercised on the local darwin host. They remain unchecked in the plan by design:

1. Pull-request CI:
  - [ ] `check-cli` runs on a PR and is green
  - [ ] `check-supply-chain` runs on a PR and is green
  - [ ] `check-architecture` runs on a PR and is green; confirmed the only nightly consumer (structurally guarded by `test_workflows.py`)

2. Cross-runner / matrix:
  - [ ] `mise install` leaves `mise.lock` unmodified on both the ubuntu and macos runners
  - [ ] `test-unit` passes on both ubuntu and macos with cli coverage folded in, and the coverage summary appears in the job log
  - [ ] With the nightly made unavailable, only `check-architecture` fails while every stable job and the product build stay green

### Recommendations:

- Open the tracking PR and confirm the six CI-only items above go green before merge; the local evidence predicts all pass.
- No code changes required — the implementation is complete and faithful to the plan.
