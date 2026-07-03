---
type: work-item
id: "0163"
title: "Scaffold the cli/ Hexagonal Workspace with a version Subcommand"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: done
kind: story
priority: high
parent: "work-item:0136"
blocks: ["work-item:0164", "work-item:0165", "work-item:0166"]
blocked_by: ["work-item:0162"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, cli, hexagonal, scaffold, workspace]
last_updated: "2026-07-02T22:27:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-184"
---

# 0163: Scaffold the cli/ Hexagonal Workspace with a version Subcommand

**Kind**: Story
**Status**: Done
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
  crate and the `launcher` binary crate. Per the ADRs a hexagon eventually becomes
  its own crate with domain/application (ports as traits) and inbound/outbound
  adapters as modules, but in this scaffold the only hexagon — `version` — starts
  as a module tree *within* `launcher` (not its own crate); crates split out only
  under pressure.
- Start with two crates — `launcher` (binary crate that hosts the `version`
  hexagon) and a thin `kernel`. In this scaffold only the cross-cutting
  error-taxonomy and logging pieces of `kernel` are populated (they are the
  cross-cutting concerns the `version` slice exercises); `kernel` config-access is
  deferred to the shared-config work (0166/0167), and the dispatch contract named
  in ADR-0053/0054 is deferred to 0164 alongside the rest of dispatch.
- Implement `accelerator version` built test-first, printing the CLI version
  (single source of truth = the `launcher` crate's version) plus build metadata (commit SHA, build
  date, target triple) injected at build time via a **vergen-gitcl build script**.
  The build script must **not** fail on error, so git-less or shallow builds degrade
  to a placeholder rather than failing to compile. `version` is a built-in
  subcommand compiled into the launcher (ADR-0054).
- Keep dispatch in-process (a plain clap subcommand); the git-style
  external-subcommand dispatch/resolution pipeline is 0164's concern, not this
  story's. ("launcher" elsewhere in this story means the `launcher` crate created
  here, not that deferred dispatch mechanism.)
- Wire the new crates into the component-based `mise` task tree established by 0162.

## Acceptance Criteria

- [ ] Given a built CLI, when `accelerator version` runs, it prints the four named
      fields — version, commit SHA, build date, and target triple — one per line;
      covered by an automated test (written test-first) that asserts each of the
      four fields is present, one per line.
- [ ] When built without git history (or from a shallow clone), the build still
      succeeds and `accelerator version` prints the literal placeholder `unknown`
      for the git-derived field(s) rather than failing to compile; covered by a
      test.
- [ ] The `version` hexagon exists as the module tree
      `version/{core, inbound/cli, outbound/build_metadata}` within `launcher`,
      with the inbound CLI adapter delegating to a `core` inbound port and the
      build-metadata outbound adapter behind an outbound port — verifiable by
      inspecting the module tree.
- [ ] The domain (`core`) layer depends on no adapter or I/O crate. This is
      enforced at compile time by the crate/module boundary graph and,
      defence-in-depth, by cargo-pup on the architecture lane; introducing a
      `core`→adapter dependency causes the cargo-pup check to fail.
- [ ] The `version` slice wires through `kernel`: its errors are expressed via the
      `kernel` error taxonomy and it initialises logging through the `kernel`
      logging facility — verifiable by the error type used in the slice and by an
      emitted log line at a defined level.
- [ ] The version value has a single source of truth — the `launcher` crate's
      version in its `Cargo.toml` — and build metadata is injected at build time,
      not hard-coded; verified by bumping the `launcher` crate version and
      observing `accelerator version` output change with no other edit.
- [ ] Only the in-process `version` subcommand is exposed; no external-subcommand
      dispatch path is compiled in this story — invoking `accelerator <unknown>`
      exits non-zero with clap's unknown-subcommand error (0164 owns any
      fetch/exec surface).
- [ ] The crates' checks run under `mise run check` and the bare `mise run`, both
      exiting 0.

## Dependencies

- Blocked by: 0162 (guard rails) — provides the per-crate checks, the cargo-deny /
  cargo-pup enforcement lanes, and the component-based `mise` task tree that this
  story's Acceptance Criteria (architecture enforcement, `mise run check` /
  `mise run` exiting 0) consume. Developed in tandem, but 0162's task tree and
  enforcement lanes must land before this story's checks can pass. (0162 is
  complete as of this writing.)
- Blocks: 0164 (launcher/dispatch), 0165 (distribution), 0166 (shared crates), and
  transitively the subdomain stories.
- Parent: epic 0136.
- External input: the luminosity 0007 reference implementation (see References) is
  the source of truth this story mirrors for the crate layout, the `build.rs`
  approach, and the `vergen` `=9.0.6` pin; its availability at implementation time
  is assumed.

## Assumptions

- `version` is a vertical-slice proof, not a feature; no real domain logic beyond
  what proves the architecture.

## Technical Notes

- Git-style dispatch (clap `external_subcommand`) and the on-demand external-binary
  resolution are 0164's concern; this story needs only the in-process `version`
  subcommand over the hexagonal skeleton.
- Test-first is non-negotiable per the project's TDD convention.
- Starting crate set is `launcher` + `kernel`; the `version` hexagon is laid out as
  `version/{core, inbound/cli, outbound/build_metadata}` within `launcher`
  (mirrors luminosity).
- Build metadata via `vergen` + `vergen-gitcl` in `launcher/build.rs` (mirror
  luminosity): emit build timestamp, target triple, and commit SHA through
  `Emitter`; deliberately omit `fail_on_error()` so git-less or shallow builds
  degrade to a placeholder rather than breaking the build. Pin `vergen` exactly
  (luminosity uses `=9.0.6`) — its transitive `vergen-lib` bumps are incompatible
  across minor versions.

## Drafting Notes

- Treated as the Phase 0 scaffold story mirroring luminosity 0007, carrying the
  resolved `cli/`-directory + `launcher`-crate-rename decision from the research.
- The two former Open Questions (starting crate set; build-metadata injection
  mechanism) were resolved during interactive enrichment against luminosity's
  implementation: crate set is `launcher` + `kernel`, and metadata is injected via
  a `vergen-gitcl` build script that does not fail on error. Both are recorded as
  Requirements / Technical Notes rather than open unknowns.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0053, ADR-0054
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0007-scaffold-hexagonal-rust-workspace-with-version-subcommand.md
