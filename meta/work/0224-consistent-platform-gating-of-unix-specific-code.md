---
type: "work-item"
id: "0224"
title: "Consistent Platform-Gating of Unix-Specific Code Across the CLI Crates"
date: "2026-08-23T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0276"
relates_to: ["work-item:0196"]
tags: ["rust", "cli", "portability", "consistency", "tech-debt"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-809"
---

# 0224: Consistent Platform-Gating of Unix-Specific Code Across the CLI Crates

**Kind**: Task
**Status**: Draft
**Priority**: Low

## Summary

Choose one treatment for production code that uses the Unix-only surface
(`std::os::unix`, `std::os::fd`, rustix's Unix `fs`) and apply it uniformly
across the `cli/` workspace. The same kind of code is gated three different ways
today, and the three appear side by side in the same module directory, so a
contributor cannot tell whether a bare `std::os::unix` import is a decision or an
oversight.

## Context

Of the 34 production files under `cli/*/src` that touch the Unix-only surface,
three treatments coexist:

- **Full defence** (11 files) — a `#[cfg(unix)]` implementation paired with a
  `#[cfg(not(unix))]` stub that `unimplemented!()`s. The launcher's
  `outbound/resolve/{cache, cache_root}` and `tree/{claims, extract, layout,
  lease, seal}`, `tracker-support/src/credentials`, `vcs-test-support/src/stubs`,
  and the visualiser server's `log`/`main`.
- **Gated, no fallback** (2 files) — `#[cfg(unix)]` only, compiling to nothing
  off-Unix: `launch/core/tree_entry`, visualiser server `file_driver`.
- **Ungated** (21 files) — the Unix surface imported with no `cfg` at all across
  design-adapters, vcs-adapters, corpus-adapters, config-adapters, work-adapters,
  `store`, `work-cli`, design-cli, and the visualiser orchestration.

The split is arbitrary at module granularity. `tree/resolver.rs` imports
`std::os::unix::fs::MetadataExt` ungated at module top level, sitting beside its
five fully-defended siblings in the same directory; `outbound/exec.rs` imports
`std::os::unix::process::CommandExt` ungated next to the fully-defended
`cache.rs`.

Nothing enforces the fallback. cargo-pup (`cli/pup.ron`) only restricts
module-import paths and carries no platform rule; clippy compiles only the active
host cfg, so the `not(unix)` arms never build on the Unix runners CI uses. The
shipped matrix is five Unix targets — `aarch64/x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `aarch64/x86_64-unknown-linux-musl` — with no Windows
job, so `cfg(unix)` is always true and the stubs are dead code. The difference is
therefore cosmetic at build time, but it reads as intent and misleads.

The decision the item must settle is which single treatment the workspace adopts:
drop the `not(unix)` stubs and let Unix code sit ungated uniformly (matching the
de-facto majority, and the fact that off-Unix is never built), or adopt the
portable-stub idiom everywhere (which buys off-Unix `cargo check`/`cargo doc`/
rust-analyzer and an explicit, greppable platform boundary). Either is
defensible; the current mix is not.

## Requirements

- Decide the single treatment for Unix-specific production code and record the
  rationale (a short ADR is the natural home, given the repo's decision culture).
- Apply the chosen treatment to all production `cli/*/src` files that use
  `std::os::unix` or `std::os::fd`, removing the two-of-three that do not match.
- Decide whether to back the rule with enforcement so it cannot drift again — a
  cargo-pup or clippy/script check that fails on a deviating file — or to leave
  it to review, documenting which and why.

## Acceptance Criteria

- [ ] Given the decision, then a record (ADR or equivalent note) states the
      single chosen treatment and the rationale for it over the alternative.
- [ ] Given the decision applied, then every production `cli/*/src` file using
      `std::os::unix` or `std::os::fd` follows that treatment, with zero
      exceptions across the 34 files surveyed.
- [ ] Given enforcement is chosen, when `mise run check` runs against a file that
      deviates from the treatment, then it fails naming the file; given
      enforcement is declined, then the item records why review is sufficient.

## Dependencies

- Relates to: 0196 — the launcher tree module (`extract`/`seal`/`lease`) whose
  full-defence pattern surfaced this inconsistency.

## References

- Surfaced while reviewing the change "Extract, seal and lease a materialised
  tree" in `plan:2026-08-11-0196-design-vendored-runtime-distribution`.
- Ruleset that does *not* enforce the fallback: `cli/pup.ron`.
