---
type: "work-item"
id: "0276"
title: "Rust CLI Consolidation and Hardening"
date: "2026-09-05T21:55:04+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "epic"
priority: "medium"
relates_to: ["work-item:0136"]
tags: ["rust", "cli", "consolidation", "hardening", "epic"]
last_updated: "2026-09-05T21:55:04+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Drafted as the successor epic to 0136 (renumbered 0275 to 0276 to avoid a cross-branch id conflict); gathering the post-migration Rust-CLI refactoring, ergonomics, and hardening backlog that accreted parentless after the shell-to-Rust migration completed."
schema_version: 1
external_id: "PP-860"
---

# 0276: Rust CLI Consolidation and Hardening

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Consolidate and harden the `cli/` Rust workspace now that the shell-to-Rust
migration (0136) has landed. Push accreted domain logic down into crates,
unify the command-layer surface, and close the ergonomics and diagnostics
gaps that the migration deferred.

## Context

Epic 0136 migrated the bash library into the `cli/` Rust workspace and is
effectively complete. In its wake a backlog of refactoring, naming,
ergonomics, and hardening items accumulated parentless — extracted in bulk
from the ideas-backlog notes (`meta/notes/2026-08-31-third-ideas-backlog.md`
and predecessors) and never linked to an owning epic.

These items are not the migration itself. They evolve the workspace the
migration produced: command-layer logic that belongs in domain crates,
overlapping abstractions, inconsistent help output, and missing structured
logging. Housing them under a "Migrate Shell Scripts" epic would overstate
that epic's scope, so they are gathered here as a distinct successor.

Migration remnants proper — residual bash, shell-outs, and bugs in the
shipped crates — remain under 0136.

## Requirements

High-level themes; each child work item carries its own acceptance criteria.

- Push domain logic out of the command layer into reusable, independently
  tested crates.
- Rationalise overlapping abstractions and crate names into one coherent
  domain vocabulary.
- Deliver a consistent, discoverable CLI surface (help, logging, subcommand
  structure).
- Federate configuration into the domain crates that own it, with
  environment-variable overrides.
- Harden the design-daemon lifecycle and get its automation onto every build.

## Decomposition

Proposed child work items, grouped by theme. These are the Tier B candidates
identified while auditing items numbered above 0136 and linked to this epic on
2026-09-05.

**Crate architecture — domain logic out of the command layer:**
- 0249 — Richer Document Model Crate
- 0250 — Push Config Lookups Into Config Crates
- 0251 — Env Var Overrides In Config Crates
- 0252 — Federate Config Into Domain Crates
- 0253 — Survey CLIs For Domain-Crate Logic
- 0256 — Push Work-List Logic Into Core Crate
- 0267 — Move VCS Kind Detection Into The VCS Crates
- 0270 — Move Filesystem Into A Shared Crate
- 0271 — Unify Surface, RemoteTracker And Client Interfaces
- 0273 — Domain Crate For Linear And Jira Subcommands

**Naming and dependency hygiene:**
- 0263 — Rename ForeignDirt
- 0266 — Remove thiserror From The Codebase
- 0268 — Rename The remote-projection Crate

**CLI ergonomics and diagnostics:**
- 0227 — accelerator config validate Command
- 0254 — Add CLI Logging
- 0258 — Help Should Show Subcommands
- 0259 — Unify Help Style
- 0260 — Move ADR Into Its Own Subcommand
- 0265 — Collapse Discoverability-Hook And Format-Hook Switches

**Design-daemon lifecycle and test coverage:**
- 0208 — Runtime Test Lane Absent From Every Build
- 0246 — Daemon Process Management Library
- 0247 — Retire server.pid In Design Commands
- 0248 — Design-Automation Integration Tests In CI

**Cross-cutting:**
- 0224 — Consistent Platform-Gating of Unix-Specific Code Across the CLI Crates

## Open Questions

- Should this epic subsume the whole Tier B set, or should the crate-architecture
  cluster (0249–0273) be its own sub-epic given its size and internal ordering?
- Does 0274 (isolate `gh` calls into a shared Python module) belong here, or does
  it conflict with the Rust direction and warrant rescoping to a Rust wrapper first?

## Dependencies

- Blocked by: 0136 (the migration must be complete for consolidation to be
  meaningful; it effectively is)
- Blocks: none recorded yet

## Drafting Notes

- Scope was derived from an audit of every work item numbered above 0136,
  judging each against 0136's boundary (the shell-to-Rust migration and its
  shipped crates). Tier A items — migration remnants, launcher/config-crate
  bugs, and the runtime-cache cluster — were reparented under 0136 directly and
  are not listed here.
- The child items above were linked to this epic (`parent: "work-item:0276"`) on
  2026-09-05, following review of the >0136 audit.
- 0274 is deliberately left out of the decomposition and raised as an open
  question — as written it proposes a Python wrapper, which runs counter to the
  Rust consolidation this epic pursues.

## References

- Related: 0136
- Source: `meta/notes/2026-08-31-third-ideas-backlog.md`
