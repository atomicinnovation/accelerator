---
type: work-item
id: "0162"
title: "Rust Toolchain Guard Rails in mise + CI"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: done
kind: story
priority: high
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0163"]
tags: [rust, tooling, ci, guard-rails, architecture-enforcement]
last_updated: "2026-07-02T15:05:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-183"
---

# 0162: Rust Toolchain Guard Rails in mise + CI

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Extend the repo's Rust toolchain wiring — format, lint, test, coverage,
supply-chain, and architecture enforcement — into `mise run` tasks and CI, so
that contributors and CI hold the new `cli/` workspace to the same automated bar
as the Python and shell components and the inward-dependency rule (ADR-0053) is
enforced mechanically rather than by review.

## Context

The repo already provisions rust + rustfmt + clippy in `mise.toml` and has
Rust-aware release scaffolding (`tasks/deps.py`, `tasks/shared/targets.py`,
`tasks/shared/paths.py`) for the visualiser. This story is therefore an
**extension**, not a stand-up: it adds the missing quality and
architecture-enforcement tooling for a multi-crate workspace. Mirrors luminosity
work item 0006, rewritten for Accelerator's brownfield state and ADR set
(ADR-0048/0053/0054, not luminosity's 0004/0009/0010).

Without these guard rails the incoming `cli/` workspace would have no mechanical
enforcement of the architecture rules (ADR-0053) or the supply-chain constraints
(ADR-0046) the foundation ADRs (ADR-0046/0048/0053/0054) depend on — they would
be held only by review, unlike the other components, and could be violated
silently as the workspace grows.

## Requirements

- Add a workspace-wide `cli:check` (rustfmt `--check` + clippy `-D warnings`,
  pedantic + nursery + cherry-picked restriction lints) running one
  `cargo clippy --workspace` pass that covers every member; per-crate
  `<crate>:check` tasks (scoped `-p <crate>`) may be added for ad-hoc use but
  stay out of the aggregate. The lint set is configured at the workspace via
  `[workspace.lints.clippy]` (members opt in with `[lints] workspace = true`) —
  see the concrete block in Technical Notes.
- Add testing via cargo-nextest with coverage folded into the test run
  (`cargo llvm-cov nextest`), wired into the `test` roll-up (not `check`), running
  on every test OS.
- Add cargo-deny (advisories, licenses, bans, sources). The bans section is
  load-bearing for architecture: (a) enforce the infra-out-of-domain dependency
  rule — keep infrastructure crates out of the dependency closures of the
  *light* (infra-free) crates, i.e. the domain subdomain crates plus the shared
  pure crates such as `kernel`/`config` (ADR-0053); (b) ban native-tls/OpenSSL
  workspace-wide so nothing re-enables `default-tls` and breaks the musl-static
  build (ADR-0046).
- Add cargo-pup as a blocking check on a **pinned-nightly lane** (nightly pinned in
  `mise.toml`) enforcing intra-crate module-import (inward-dependency) rules, while
  the product build and all other checks stay on stable. Pins: cargo-pup `0.1.8`
  on `nightly-2026-01-22`.
- Pin cargo-nextest / cargo-llvm-cov / cargo-deny / cargo-pup and the nightly in
  `mise.toml`; create `rustfmt.toml`, `deny.toml` (80-col width duplicated by
  hand into `rustfmt.toml`, consistent with the other components). Clippy lint
  *levels* live in the workspace manifest (`[workspace.lints.clippy]`); a
  `clippy.toml` is only needed if a configurable-lint *threshold* is later
  required, so it is optional at this stage.
- Revisit `tasks/shared/paths.py`, which currently hard-codes a single
  `Cargo.toml`, for the multi-crate workspace. The version is kept at workspace
  level (`[workspace.package].version` in the root `cli/Cargo.toml`), members
  inheriting via `version.workspace = true`; `paths.py` must still enumerate
  every member `Cargo.toml` so a member that hardcodes its own
  `[package].version` instead of inheriting is detected.

## Acceptance Criteria

The green-build criteria below presuppose that Rust code and the
`cli/` workspace exist; they are **co-verified with the paired scaffold story
0163** (this story wires the gates, 0163 supplies the code they run against). At
0162's own close the workspace may be a single crate — the workspace-wide
`cli:check` covers whatever members `cli/Cargo.toml` declares at that point and
extends to further members with no per-member wiring. See Dependencies.

- [ ] `mise run check` includes a workspace-wide format-check + clippy
      `-D warnings` pass (a single `cli:check` covering every member listed in
      `cli/Cargo.toml` `[workspace].members`), plus workspace-scope `cargo deny
      check` and the cargo-pup architecture check, and exits 0; `check` stays
      read-only and test-free. Per-crate `<crate>:check` tasks may be provided
      for ad-hoc use but are deliberately excluded from the aggregate, which
      covers all members in the single workspace-wide pass.
- [ ] The bare `mise run` default runs the workspace-wide `cli:check` (format +
      clippy) plus the workspace-scope deny/pup checks and the tests-with-coverage
      run, and exits 0 end-to-end.
- [ ] The `test` roll-up emits a `cargo llvm-cov` coverage report; coverage is
      collected but not gated at this Phase 0 stage (no minimum threshold fails
      the run).
- [ ] A change that fails format-check, clippy, cargo-deny, or cargo-pup fails CI
      and is non-mergeable.
- [ ] cargo-deny's bans encode both the infra-out-of-domain dependency rule and
      the workspace-wide native-tls/OpenSSL ban (rustls only). The
      native-tls/OpenSSL ban is demonstrable within this story: adding a
      dependency on a native-tls/OpenSSL crate makes `cargo deny check` exit
      non-zero.
- [ ] The infra-out-of-domain ban's cross-crate fixture — a domain crate
      depending on an infrastructure crate makes `cargo deny check` exit non-zero
      — is **co-verified with work item 0166** (the config/config-adapters
      split), where the split first makes the rule bite. This story encodes the
      ban; 0166 demonstrates it firing.
- [ ] cargo-pup runs on a pinned-nightly lane in `mise run check` and CI while the
      product build and all other checks stay on stable; a deliberately-failing
      fixture — a domain module importing an adapter/infrastructure module — fails
      the build.
- [ ] The nightly lane is isolated: with the pinned nightly unavailable, the
      product build and every stable-lane check still pass and only the cargo-pup
      lane fails — a nightly/cargo-pup break gates the architecture check alone,
      not the product.
- [ ] `tasks/shared/paths.py` resolves every workspace member `Cargo.toml` (not a
      single hard-coded path). With the version held at workspace level, members
      normally inherit (`version.workspace = true`) and carry no version of their
      own; the coherence check therefore asserts each member either inherits or,
      if it pins its own `[package].version`, matches the workspace version — so a
      member that opts out of inheritance and drifts is detected.

## Open Questions

- None outstanding. The restriction-lint set, the cargo-pup nightly/version
  pins, and the workspace-level version decision are now resolved (recorded in
  Requirements and Technical Notes).

## Dependencies

- Paired with: 0163 (scaffold) — there is nothing to lint or test until Rust code
  exists; both land before the green-build criteria can pass.
- Parent: epic 0136.
- Blocks: this story establishes the `mise run check`/CI enforcement floor that
  the downstream migration phases (0163–0174) inherit; the cross-crate ban-lists
  in particular first bite at the config/config-adapters split (0166).
- Consumes the `cli/Cargo.toml` workspace-manifest contract authored by 0163: the
  per-crate `<crate>:check` enumeration and the `paths.py` member resolution both
  read `[workspace].members`, so the manifest format and location must be agreed
  across the 0162/0163 pair.
- Split-acceptance timeline: the native-tls/OpenSSL ban and all stable-lane gates
  are verifiable at this story's close (jointly with 0163); the infra-out-of-domain
  ban is *encoded* here but its cross-crate fixture is co-verified with 0166, where
  the multi-crate split makes it bite. The story is not independently demonstrable
  in isolation from 0163.
- External tooling: cargo-nextest, cargo-llvm-cov, and cargo-deny are new pinned
  toolchain dependencies provisioned via `mise.toml` that must be fetchable and
  installable in CI on every test OS — a net-new toolchain-surface coupling.
  Additionally, the cargo-pup pinned-nightly lane couples to a specific nightly
  compiler ABI, so nightly-toolchain availability/compatibility is a maintenance
  dependency. The lane is isolated from the stable product build, so a
  nightly/cargo-pup break gates only the architecture check, not the product.

## Assumptions

- The quality-tool stack (nextest, llvm-cov, cargo-deny) is taken from the
  direction set in the 0158 architecture spike; no dedicated ADR ratifies it, and
  ADR ratification of the stack is a possible follow-up. Architecture enforcement
  (cargo-pup + cargo-deny ban-lists) derives from ADR-0053. The source research
  proposed a cheap grep-based dependency tripwire as a floor beneath cargo-pup;
  this story deliberately omits it and relies wholly on cargo-pup for
  intra-crate module-import enforcement. Trade-off: with no stable-lane floor, a
  nightly/cargo-pup outage (or an unavailable pinned nightly) leaves the
  inward-dependency rule unenforced until the lane is restored — module-import
  violations would not be caught on the stable lane in the interim.

## Technical Notes

- The cross-crate ban-lists are largely inert until the workspace splits into
  multiple crates; they first bite at the `config`/`config-adapters` split (0166).
- Component task naming follows the existing `<component>:check` convention.
- Clippy lint set, configured at the workspace and inherited by each member via
  `[lints] workspace = true`. `pedantic`/`nursery` carry `priority = -1` so the
  individual restriction opt-ins and allows below override them; the workspace-wide
  `cli:check` clippy pass retains `-D warnings`, promoting every `warn` to a hard
  CI failure:

  ```toml
  [workspace.lints.clippy]
  pedantic = { level = "warn", priority = -1 }
  nursery  = { level = "warn", priority = -1 }
  # restriction is allow-by-default; these are the cherry-picked opt-ins.
  unwrap_used   = "warn"
  expect_used   = "warn"
  panic         = "warn"
  dbg_macro     = "warn"
  todo          = "warn"
  unimplemented = "warn"
  module_name_repetitions = "allow"
  must_use_candidate      = "allow"
  ```
- cargo-pup is pinned to `0.1.8` and runs on `nightly-2026-01-22` (both in
  `mise.toml`); the nightly drives only the isolated cargo-pup lane.
- Version coherence: the workspace version lives in `[workspace.package].version`
  (single write site); members inherit via `version.workspace = true`.

## Drafting Notes

- Treated as the Phase 0 enforcement story mirroring luminosity 0006, scoped as an
  extension of the existing Rust wiring rather than a green-field stand-up.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Tool-stack direction: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
- ADRs: ADR-0046, ADR-0048, ADR-0053, ADR-0054
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0006-establish-rust-toolchain-guard-rails-in-mise-and-ci.md
