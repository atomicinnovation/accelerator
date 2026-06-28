---
type: work-item
id: "0162"
title: "Rust Toolchain Guard Rails in mise + CI"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: high
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0163"]
tags: [rust, tooling, ci, guard-rails, architecture-enforcement]
last_updated: "2026-06-28T22:52:20+00:00"
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
supply-chain, and architecture enforcement — into `mise run` tasks and CI, so the
new `cli/` workspace is held to the same automated bar as the Python and shell
components and the inward-dependency rule (ADR-0053) is enforced mechanically.

## Context

The repo already provisions rust + rustfmt + clippy in `mise.toml` and has
Rust-aware release scaffolding (`tasks/deps.py`, `tasks/shared/targets.py`,
`tasks/shared/paths.py`) for the visualiser. This story is therefore an
**extension**, not a stand-up: it adds the missing quality and
architecture-enforcement tooling for a multi-crate workspace. Mirrors luminosity
work item 0006, rewritten for Accelerator's brownfield state and ADR set
(ADR-0048/0053/0054, not luminosity's 0004/0009/0010).

## Requirements

- Add per-crate `<crate>:check` components (rustfmt `--check` + clippy
  `-D warnings`, pedantic + nursery + cherry-picked restriction lints) scoped via
  `cargo … -p <crate>` for every workspace member.
- Add testing via cargo-nextest with coverage folded into the test run
  (`cargo llvm-cov nextest`), wired into the `test` roll-up (not `check`), running
  on every test OS.
- Add cargo-deny (advisories, licenses, bans, sources). The bans section is
  load-bearing for architecture: (a) keep infrastructure crates out of the
  light/domain crates' dependency closures (ADR-0053); (b) ban native-tls/OpenSSL
  workspace-wide so nothing re-enables `default-tls` and breaks the musl-static
  build (ADR-0046).
- Add cargo-pup as a blocking check on a **pinned-nightly lane** (nightly pinned in
  `mise.toml`) enforcing intra-crate module-import (inward-dependency) rules, while
  the product build and all other checks stay on stable.
- Pin cargo-nextest / cargo-llvm-cov / cargo-deny / cargo-pup and the nightly in
  `mise.toml`; create `rustfmt.toml`, `clippy.toml`, `deny.toml` (80-col width
  duplicated by hand into `rustfmt.toml`, consistent with the other components).
- Revisit `tasks/shared/paths.py`, which currently hard-codes a single
  `Cargo.toml`, for the multi-crate workspace.

## Acceptance Criteria

- [ ] `mise run check` includes per-crate format-check + clippy `-D warnings` for
      every workspace crate plus workspace-scope `cargo deny check` and the
      cargo-pup architecture check, and exits 0; `check` stays read-only and
      test-free.
- [ ] The bare `mise run` default runs format + clippy + tests-with-coverage +
      deny + pup and exits 0 end-to-end with the Rust component included.
- [ ] A change that fails format-check, clippy, cargo-deny, or cargo-pup fails CI
      and is non-mergeable.
- [ ] cargo-deny's bans encode both the infra-out-of-domain rule and the
      workspace-wide native-tls/OpenSSL ban (rustls only).
- [ ] cargo-pup runs on a pinned-nightly lane in `mise run check` and CI while the
      product build stays on stable; a module-import violation fails the build.

## Open Questions

- Which restriction lints are cherry-picked, and which nightly version pins the
  cargo-pup lane? (Both decided at implementation, per the source.)

## Dependencies

- Paired with: 0163 (scaffold) — there is nothing to lint or test until Rust code
  exists; both land before the green-build criteria can pass.
- Parent: epic 0136.

## Assumptions

- The quality-tool stack (nextest, llvm-cov, cargo-deny) is taken from the
  established direction; no dedicated ADR ratifies it. Architecture enforcement
  (cargo-pup + cargo-deny ban-lists) derives from ADR-0053; a cheap grep-based
  dependency tripwire (proposed in the source research) is added as a floor
  beneath cargo-pup — no such check exists in the repo today.

## Technical Notes

- The cross-crate ban-lists are largely inert until the workspace splits into
  multiple crates; they first bite at the `config`/`config-adapters` split (0166).
- Component task naming follows the existing `<component>:check` convention.

## Drafting Notes

- Treated as the Phase 0 enforcement story mirroring luminosity 0006, scoped as an
  extension of the existing Rust wiring rather than a green-field stand-up.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0046, ADR-0048, ADR-0053, ADR-0054
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0006-establish-rust-toolchain-guard-rails-in-mise-and-ci.md
