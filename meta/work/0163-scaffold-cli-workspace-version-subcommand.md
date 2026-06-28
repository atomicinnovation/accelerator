---
type: work-item
id: "0163"
title: "Scaffold the cli/ Hexagonal Workspace with a version Subcommand"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: high
parent: "work-item:0136"
blocks: ["work-item:0164", "work-item:0165", "work-item:0166"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0162"]
tags: [rust, cli, hexagonal, scaffold, workspace]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-184"
---

# 0163: Scaffold the cli/ Hexagonal Workspace with a version Subcommand

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Scaffold the `cli/` Cargo workspace — subdomain-first hexagonal layout with a thin
`kernel` crate and the `launcher` binary crate — and prove it end-to-end with an
`accelerator version` subcommand, built test-first, so the architecture is real and
subsequent stories have a skeleton to build on.

## Context

This is the first real Rust code outside the visualiser and the proof of the
hexagonal architecture (ADR-0053) and git-style composition (ADR-0054). The
workspace lives in a single top-level `cli/` directory (resolved Q1) whose
`Cargo.toml` is the workspace root. The `version` subcommand is deliberately
trivial — its value is exercising the full path from CLI inbound adapter through the
domain, test-first, not the feature. Mirrors luminosity work item 0007.

**Deviation from ADR-0053/0054 to record:** the launcher crate is named
**`launcher`**, not `cli`, to free the `cli/` directory name and avoid a `cli/cli/`
path. The crate still produces the `accelerator` binary.

## Requirements

- Create the `cli/` workspace per ADR-0053/0054: subdomain-first, a thin `kernel`
  crate (cross-cutting error taxonomy, config-access + dispatch contracts,
  logging) and the `launcher` binary crate, each hexagon starting as a single crate
  with domain/application (ports as traits) and inbound/outbound adapters as
  modules; split into separate crates only under pressure.
- Implement `accelerator version` built test-first, printing the CLI version
  (single source of truth = crate version) plus build metadata (commit SHA, build
  date, target triple) injected at build time. `version` is a built-in subcommand
  compiled into the launcher (ADR-0054).
- Keep dispatch in-process (a plain clap subcommand); the git-style
  external-subcommand launcher is 0164's concern, not this story's.
- Wire the new crates into the component-based `mise` task tree established by 0162.

## Acceptance Criteria

- [ ] Given a built CLI, when `accelerator version` runs, it prints version, commit
      SHA, build date, and target triple; covered by a test written test-first.
- [ ] The workspace follows the subdomain-first hexagonal layout, with the domain
      layer depending on no adapter or I/O crate — a violation fails to compile
      and/or trips cargo-deny/cargo-pup.
- [ ] The version value has a single source of truth (crate version) and build
      metadata is injected at build time, not hard-coded.
- [ ] Only the in-process `version` subcommand is exposed; no external-subcommand
      dispatch is wired in this story.
- [ ] The crates' checks run under `mise run check` and the bare `mise run`, both
      exiting 0.

## Open Questions

- The concrete starting crate set for a version-only scaffold (likely just
  `launcher` + `kernel`) is settled during implementation.
- Build-metadata injection mechanism (build script vs a vergen-style crate) is an
  implementation choice.

## Dependencies

- Paired with: 0162 (guard rails) — provides the per-crate checks and architecture
  enforcement.
- Blocks: 0164 (launcher/dispatch), 0165 (distribution), 0166 (shared crates), and
  transitively the subdomain stories.
- Parent: epic 0136.

## Assumptions

- `version` is a vertical-slice proof, not a feature; no real domain logic beyond
  what proves the architecture.

## Technical Notes

- Git-style dispatch (clap `external_subcommand`) and the on-demand launcher are
  0164's concern; this story needs only the in-process `version` subcommand over
  the hexagonal skeleton.
- Test-first is non-negotiable per the project's TDD convention.

## Drafting Notes

- Treated as the Phase 0 scaffold story mirroring luminosity 0007, carrying the
  resolved `cli/`-directory + `launcher`-crate-rename decision from the research.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0053, ADR-0054
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0007-scaffold-hexagonal-rust-workspace-with-version-subcommand.md
