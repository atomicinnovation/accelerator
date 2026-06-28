---
type: work-item
id: "0168"
title: "Fold the Visualiser into the cli/ Workspace"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0165"]
tags: [rust, visualiser, frontend, workspace]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-189"
---

# 0168: Fold the Visualiser into the cli/ Workspace

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Relocate the existing `accelerator-visualiser` server and its frontend into the
`cli/` workspace, refactor it onto the shared `config`/`corpus` crates, and move its
start/stop/status orchestration into `accelerator visualiser …` so it becomes the
first concrete on-demand sub-binary dispatched by the unified launcher (ADR-0054).

## Context

The visualiser is already the crate `accelerator-visualiser` with a `[lib]`/`[[bin]]`
split and ~15.4k lines of Rust, much of it corpus logic that duplicates the bash
library (the dedup collapsed into the shared crates by 0166). The `server`+`frontend`
pair moves as a unit to `cli/visualiser/{server,frontend}` (resolved Q1), preserving
their relative layout so the three `../frontend/dist` embed literals are unchanged.
The bespoke `launch-server.sh` daemon launcher is retired in favour of the unified
launcher (0164).

## Requirements

- Move `skills/visualisation/visualise/{server,frontend}` to
  `cli/visualiser/{server,frontend}` as a unit; add `visualiser/server` as a
  workspace member; keep `skills/visualisation/visualise/` holding only the skill.
- Refactor the server to consume the shared `corpus`/`config` crates instead of its
  own duplicated frontmatter/doc-type/slug/work-item-ID/typed-linkage code; keep
  axum/tokio/notify isolated in this crate.
- Move start/stop/status orchestration (`visualiser.sh`, `launch-server.sh`,
  `stop-server.sh`, `status-server.sh`, `write-visualiser-config.sh`) into
  `accelerator visualiser start|stop|status`, dispatched by the launcher; preserve
  the owner-PID/`start_time`/idle-shutdown lifecycle and the loopback-binding +
  Host/Origin security model.
- Retire `launch-server.sh` and the visualiser's separate `bin/checksums.json` in
  favour of the unified launcher + release manifest (0164/0165).
- Update the visualiser build wiring (the `../frontend/dist` literals stay valid
  given the unit move; `build.rs` still requires the prebuilt dist under
  `embed-dist`).

## Acceptance Criteria

- [ ] `accelerator visualiser start` resolves config, launches the server, and
      returns its loopback URL; `stop`/`status` behave as the shell commands did,
      preserving the PID/`start_time` recycle guard and idle shutdown.
- [ ] The server consumes the shared `corpus`/`config` crates; the duplicated
      corpus logic is removed from the visualiser crate.
- [ ] The visualiser builds and embeds the frontend from `cli/visualiser/frontend`
      with the embed literals unchanged.
- [ ] `launch-server.sh` and the standalone visualiser checksum manifest are
      removed; the visualiser is fetched/verified/dispatched by the unified
      launcher.
- [ ] The visualiser E2E/integration suites pass against the relocated, refactored
      crate.

## Open Questions

- Whether the frontend's own toolchain checks (Biome/vitest/Playwright) move under
  a `cli/visualiser/frontend` task path or stay as-is — decided during
  implementation.

## Dependencies

- Blocked by: 0166 (shared crates the server refactors onto), 0164 (launcher
  dispatch).
- Relates to: 0165 (the visualiser joins the multi-binary release).
- Parent: epic 0136.

## Assumptions

- Moving the `server`+`frontend` pair as a unit preserves the relative embed path,
  so no literal surgery is needed (resolved Q1).

## Technical Notes

- Existing seam: `file_driver.rs` already defines a `FileDriver` port trait.
- The server has no production TLS (loopback + guards); the launcher's rustls is the
  only production TLS.

## Drafting Notes

- Treated as the Phase 5 story; it is both a fold-in and the validation that the
  shared crates (0166) actually absorb the visualiser's duplicated logic.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0053, ADR-0054
