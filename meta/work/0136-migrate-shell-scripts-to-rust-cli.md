---
type: work-item
id: "0136"
title: "Migrate Shell Scripts into a Rust CLI"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: epic
priority: medium
source: "note:2026-06-22-ideas-backlog"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture", "codebase-research:2026-06-23-0136-shell-scripts-rust-cli-migration-surface"]
tags: [rust, cli, migration, epic]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-157
---

# 0136: Migrate Shell Scripts into a Rust CLI

**Kind**: Epic
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the shell-script library that backs the skills into a Rust CLI,
consolidating logic into a typed, testable, cross-platform binary.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). A large bash library (config
reading, VCS detection, frontmatter parsing, migrations) currently backs the
skills and is constrained by a bash 3.2 floor.

## Requirements

- Identify the shell scripts and shared library functions in scope for
  migration. *(Done — see the surface research and the scope/architecture
  research below.)*
- Define the Rust CLI surface that replaces them, preserving the
  `${CLAUDE_PLUGIN_ROOT}` invocation contract used by skills. *(Done — the
  `accelerator` launcher + `accelerator-<sub>` binaries, crate split, and
  subcommand surface are defined in the architecture research.)*
- Plan an incremental migration that keeps skills working throughout. *(Done —
  the 12-phase decomposition below, sequenced along the dependency spine.)*

## Decomposition

This epic is decomposed into the following child work items (status: draft),
ordered along the dependency spine; each keeps the plugin functional at its step:

**Foundations (Phases 0–2):**
- 0162 — Rust Toolchain Guard Rails in mise + CI
- 0163 — Scaffold the cli/ Hexagonal Workspace with a version Subcommand
- 0164 — Launcher and Git-Style Dispatch
- 0165 — Multi-Binary Static Distribution and Release Pipeline with minisign

**Shared core + the contract cutover (Phases 3–4):**
- 0166 — Shared config, corpus, and store Crates
- 0167 — Built-in config Command and Invocation-Contract Migration

**Subdomain migrations (Phases 5–10):**
- 0168 — Fold the Visualiser into the cli/ Workspace
- 0169 — VCS Subdomain and Hooks Migration
- 0170 — Work-Item Subdomain and Sync Engine
- 0171 — Jira and Linear Integrations
- 0172 — Migration Engine Subdomain
- 0173 — Remaining Subdomains: corpus, design, collaboration

**Cleanup (Phase 11):**
- 0174 — Retire Shell Tooling and CI Guards

The target architecture (git-style `accelerator` launcher dispatching to on-demand
`accelerator-<sub>` static binaries, each a hexagonal crate; the visualiser folded
in as the first sub-binary) is fixed by ADR-0045/0046/0047/0051/0052/0053/0054.

## Acceptance Criteria

- [ ] Shell-script responsibilities are migrated into a Rust CLI without
      regressing skill behaviour, with the migration sequenced so the plugin
      stays functional at each step.
- [ ] All child work items 0162–0174 are completed.

## Open Questions

*(All resolved — see the architecture research's decision log. The two original
questions are answered:)*

- Which scripts migrate first, and which remain as shell? — Leaf/shared crates and
  the launcher first, the migration engine last; a thin residual shell surface
  (launcher bootstrap, hook wrapper, Playwright executor) remains under the bash
  3.2 floor (ADR-0048/0049).
- How is the CLI distributed? — Zero-setup, fully static musl/darwin binaries
  fetched, sha256+minisign-verified, and exec'd on demand, reusing the existing
  visualiser release pipeline (ADR-0046).

## Dependencies

- Blocked by: None.
- Blocks: None directly (the children carry the internal dependency spine).
- Children: 0162–0174 (parented to this epic).

## Assumptions

- The Rust CLI is distributed similarly to the existing visualiser binary
  (release artefact + checksum verification), confirmed and extended with minisign
  by ADR-0046.

## Technical Notes

- Removes the bash 3.2 floor constraint for migrated functionality; a thin residual
  shell surface remains and stays under the floor (ADR-0048/0049).
- Must preserve bare-path invocation semantics expected by skill `allowed-tools`;
  the cache lives under `${CLAUDE_PLUGIN_ROOT}` to keep permission matches working
  (the contract cutover is 0167).

## Drafting Notes

- Treated as an epic per the user's instruction. Decomposed 2026-06-28 from the
  scope/architecture research into children 0162–0174, with all open questions
  resolved interactively; promoted from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
- Research: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- Spike: `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
- ADRs: ADR-0045, ADR-0046, ADR-0047, ADR-0048, ADR-0049, ADR-0051, ADR-0052, ADR-0053, ADR-0054
