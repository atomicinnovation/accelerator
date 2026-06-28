---
type: work-item
id: "0166"
title: "Shared config, corpus, and store Crates"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: high
parent: "work-item:0136"
blocks: ["work-item:0167", "work-item:0168", "work-item:0169", "work-item:0170", "work-item:0171", "work-item:0172", "work-item:0173"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, config, corpus, store, crates, dedup]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-187"
---

# 0166: Shared config, corpus, and store Crates

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Build the shared library crates the subdomains depend on — `config`/`config-adapters`
(native YAML configuration reader), `corpus`/`corpus-adapters` (frontmatter,
doc-type inference, typed-linkage, slug/path conventions, work-item-ID,
artifact-metadata), and `store` (atomic JSONL writes + locking) — collapsing the
bash↔Rust duplication ADR-0045 names.

## Context

The visualiser already implements corpus parsing, doc-type inference, slug/path
conventions, work-item-ID logic, and typed-linkage in Rust that **duplicates the
bash library exactly** — the duplication ADR-0045 exists to remove. This story
establishes those as shared crates so the visualiser, work, corpus, and config
subdomains reuse one implementation. ADR-0047 makes the CLI the native config
reader with arbitrary YAML nesting (dropping the bash 2-level cap). Mirrors
luminosity 0009 (configuration system), scoped here to the shared crate layer
beneath the built-in `config` command (0167).

## Requirements

- `config` (domain + application + ports) and `config-adapters` (outbound: native
  YAML/`serde` frontmatter reader, filesystem). Implement team→local
  last-writer-wins precedence; arbitrary YAML structure (no 2-level cap); the full
  recognised-key catalogue (`paths.*`, `templates.*`, `work.*`, `review.*`,
  `agents.*`); the legacy `.claude/accelerator.md` read path under
  `ACCELERATOR_MIGRATION_MODE`. Each sub-binary wires its own `config-adapters` at
  its composition root (Model 1).
- `corpus` (domain + ports) and `corpus-adapters`: frontmatter parse (real YAML),
  doc-type inference (consolidating the triplicated directory→type fact),
  typed-linkage (ADR-0034), slug/path conventions, work-item-ID
  scan/extract/normalise, artifact-metadata derivation. Extract from the visualiser
  where it already exists rather than re-deriving.
- `store`: `atomic_write` (temp + rename), the `mkdir`-based lock with PID-owner
  reclaim and jittered back-off, and canonical-order JSONL compose/remove — porting
  the load-bearing concurrency semantics from `atomic-common.sh`/`jsonl-common.sh`.
- All crates unit-tested in isolation with faked ports (ADR-0053); the cargo-deny
  ban-lists from 0162 first bite at the `config`/`config-adapters` split here.

## Acceptance Criteria

- [ ] The native config reader resolves team→local precedence, arbitrary nesting,
      and every recognised key, with unit tests against in-memory fakes; behaviour
      matches the bash reader on a shared corpus of fixtures.
- [ ] The `corpus` crate parses frontmatter, infers doc types, resolves
      typed-linkage and slugs, and derives artifact metadata, with the doc-type
      fact defined once (no triplication).
- [ ] The `store` crate provides atomic writes and the mkdir-lock with PID-owner
      reclaim, with tests covering contended-lock reclaim of a dead holder.
- [ ] The crates carry no inbound/outbound dependency into their domain layer
      (enforced by cargo-deny + cargo-pup).

## Open Questions

- Whether atomic JSONL + locking is its own `store` crate or folded into
  `corpus-adapters` — decided during implementation (the research notes both).

## Dependencies

- Blocked by: 0163 (workspace skeleton).
- Blocks: 0167 (built-in config command), 0168 (visualiser refactor), 0169 (vcs),
  0170 (work), 0171 (integrations), 0172 (migrate), 0173 (remaining subdomains) —
  all consume these crates.
- Parent: epic 0136.

## Assumptions

- The visualiser's existing corpus code is extractable into shared crates with
  bounded effort (it is currently entangled with `serde_json::Value` representations
  but is axum-free).

## Technical Notes

- Source bash: `scripts/config-common.sh`, `config-defaults.sh`,
  `config-read-value.sh` (2-level awk reader), `doc-type-inference.sh`,
  `linkage-parser.sh`, `atomic-common.sh`, `jsonl-common.sh`,
  `skills/work/scripts/work-item-pattern.sh`.
- Visualiser twins to extract from: `frontmatter.rs`, `docs.rs`, `slug.rs`,
  `config.rs` (`WorkItemConfig`), `typed_ref.rs`, `cluster_key.rs`.

## Drafting Notes

- Treated as the Phase 3 shared-crate story; it is the dedup linchpin the visualiser
  refactor (0168) and every subdomain build on.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0047, ADR-0053
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0009-multi-level-configuration-system.md
