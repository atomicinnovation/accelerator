---
type: work-item
id: "0179"
title: "corpus and corpus-adapters Crates for Parsing and Conventions"
date: "2026-07-06T22:27:35+00:00"
author: Toby Clemson
producer: refine-work-item
status: draft
kind: task
priority: high
parent: "work-item:0166"
blocks: ["work-item:0180"]
tags: [rust, config, corpus, store, crates, dedup]
last_updated: "2026-07-06T23:08:57+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0179: corpus and corpus-adapters Crates for Parsing and Conventions

**Kind**: Task
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Build the `corpus` (domain + ports) and `corpus-adapters` crates by extracting
the parsing and convention logic that the visualiser already implements in Rust
— frontmatter, doc-type inference, typed-linkage, slug/path conventions,
work-item-ID logic, and artifact-metadata derivation — into shared crates.

## Context

Child of 0166 — Shared config, corpus, and store Crates. The visualiser
duplicates the bash library exactly (the duplication ADR-0045 exists to remove).
This task consolidates that logic once, so the visualiser refactor (0168) and
every subdomain reuse a single implementation. The directory→type fact is
currently triplicated (`doc-type-inference.sh`, the 0007 migration, and the
0007 rewrite awk); this task defines it once.

## Requirements

- `corpus` crate: domain + ports, no outbound dependency. Model frontmatter,
  the doc-type fact (single-sourced), typed-linkage (ADR-0034), slug/path
  conventions, work-item-ID identity, and artifact-metadata as domain concepts.
- `corpus-adapters` crate: real-YAML frontmatter parse, doc-type inference,
  typed-linkage resolution, slug/path derivation, work-item-ID
  scan/extract/normalise, and artifact-metadata derivation at parity with
  `artifact-derive-metadata.sh` (current UTC date/time, filename timestamp,
  repository name, and current revision via the jj → git → empty fallback), with
  the clock and VCS seams behind faked ports.
- Extract from the visualiser twins rather than re-deriving. Decide a clean
  domain frontmatter representation rather than propagating the visualiser's
  `serde_json::Value` map. Keep the `cluster_key` walk's `indexer`/`IndexEntry`
  coupling out of scope — extract the typed-linkage *parse* primitive, not the
  server-entangled walker.
- Add the cargo-pup import-restriction rule for the `corpus` domain module.

## Acceptance Criteria

- [ ] The `corpus` crate parses frontmatter, infers doc types, and resolves
      typed-linkage and slugs at parity with the bash sources
      (`doc-type-inference.sh`, `linkage-parser.sh`, and `work-item-pattern.sh`'s
      slug/path conventions), with the doc-type fact defined once (no
      triplication) and unit tests asserting each against a shared corpus of
      fixtures.
- [ ] Artifact-metadata derivation reaches parity with
      `artifact-derive-metadata.sh` — current UTC date/time, filename timestamp,
      repository name, and current revision resolved via the jj → git → empty
      fallback — with the clock and VCS seams behind faked ports so each field is
      asserted deterministically.
- [ ] Work-item-ID scan/extract/normalise matches the bash
      `work-item-pattern.sh` behaviour on a shared corpus of fixtures.
- [ ] The `corpus` domain layer carries no inbound/outbound dependency
      (enforced by cargo-deny + cargo-pup).

## Dependencies

- Blocked by: 0166 crate-layer conventions (parent).
- Blocks: 0180 (atomic-store primitives land in `corpus-adapters`, which this
  task creates).
- Parent: 0166.

## Assumptions

- The visualiser's corpus code is extractable with bounded effort; entanglement
  grades run from `typed_ref.rs` (trivial) to `cluster_key.rs` (highest, and
  partly out of scope via its `indexer`/`IndexEntry` coupling).

## Technical Notes

**Size**: L — the heaviest of the three siblings: six visualiser twins of
varying entanglement (trivial `typed_ref` → high-effort `cluster_key`), a
genuinely greenfield artifact-metadata piece (clock + VCS jj→git→empty ports,
no Rust twin to extract), the `serde_json`-in-domain representation decision,
and the slug bash-parity harness needing re-pathing when the crate moves into
`cli/`.

- Visualiser twins to extract from
  (`skills/visualisation/visualise/server/src/`): `typed_ref.rs` (cleanest —
  pure ADR-0034 parser, ~72 logic lines), `slug.rs`, `docs.rs` (`DocTypeKey`,
  14 variants — the doc-type fact), `config.rs:83-251` (`WorkItemConfig` only:
  `extract_id`/`normalise_id`/`is_canonical_id_token`/`canonical_digit_width`),
  `frontmatter.rs` (moderate-high — `serde_yml` + `serde_json::Value`
  representation decision), `cluster_key.rs` (highest — `indexer`/`IndexEntry`
  coupling, walker out of scope).
- Source bash: `scripts/doc-type-inference.sh:1-29` (the documented
  triplication), `scripts/linkage-parser.sh`,
  `skills/work/scripts/work-item-pattern.sh`.
- Artifact-metadata source bash: `scripts/artifact-derive-metadata.sh` — derives
  current UTC date/time, filename timestamp, repository name, and current
  revision, resolving the revision/repo-name via a jj → git → empty fallback
  (the clock and VCS seams become faked ports in the Rust port).
- Slug tests in the visualiser shell out to `work-item-pattern.sh` for bash↔Rust
  parity (`slug.rs:572-602`) — a parity harness to preserve.
- Triplication is subtler than "one fact in three files": `doc-type-inference.sh`
  already loads its table from `doc-type-table.sh` (single-sourced by the 0007
  schema via `config-read-doc-type-paths.sh`). The three *materialisations* of
  the dir→type fact to converge are the runtime table (`doc-type-table.sh`), the
  0007 migration snapshot (`0007-unify-meta-corpus-frontmatter.sh:49-66`), and
  `0007-frontmatter-rewrite.awk` — the corpus crate becomes their single source.
- `serde_json`-in-domain decision: `frontmatter.rs:72-77` represents parsed
  frontmatter as `BTreeMap<String, serde_json::Value>`. Keeping that map in the
  corpus *domain* core collides with the infra-out-of-domain pup.ron/deny.toml
  intent (serde_json is arguably infra) — decide a clean domain type vs. allowing
  serde_json inside the corpus-domain crate.
- Preserve the libyml panic guard: `parse` wraps `serde_yml` in `catch_unwind`
  (`frontmatter.rs:143-154`) because libyml *panics* (not errors) on adversarial
  input; port the regression test (`frontmatter.rs:416-429`).
- `docs.rs` coupling is isolated: the pure `DocTypeKey` enum (14 variants,
  `docs.rs:6-21`, all-`match` self-methods) extracts clean; the `Config`-coupled
  `DocType` struct + `describe_types` (`docs.rs:180-222`) are the
  visualiser-runtime projection and stay server-side.
- `cluster_key` split: extract only the pure linkage primitives — `id_from_value`
  (`cluster_key.rs:147-161`), `canonicalise_one_id` (`indexer.rs:1207-1243`), and
  `number_width_from_id_pattern` (`indexer.rs:1193-1201`). The recursive walker
  (`resolve_cluster_key`/`walk`, `cluster_key.rs:32-121`) with its `IndexEntry` /
  `target_path_from_entry` / `normalize_target_key` deps is store-side and out of
  scope; its tests lean on the server-internal `test_support::entry_for_test`.
- slug parity-harness re-pathing: `slug.rs:572-602` shells out to
  `work-item-pattern.sh --compile-scan` via a `CARGO_MANIFEST_DIR`-relative path
  (`../../../../skills/work/...`) that breaks when the crate moves into `cli/`
  — the bash script stays put, so the harness needs re-pathing.
- Extraction order (leaves first): `typed_ref.rs` (sole dep `std::path::PathBuf`;
  extract first — frontmatter, cluster_key, and the indexer target-resolver all
  depend on it), `WorkItemConfig` (`config.rs:66-251`, leaf; only `ConfigError`
  is shared with the runtime `Config`), and `DocTypeKey` (leaf) → `slug` +
  `frontmatter` (moderate) → the `cluster_key` linkage primitives.
- Artifact-metadata is genuinely greenfield (no Rust twin): the four
  `artifact-derive-metadata.sh` outputs need net-new clock and VCS (jj→git→empty)
  ports; `REPO_NAME` derives from the VCS-root port.

## References

- Parent: `meta/work/0166-shared-config-corpus-store-crates.md`
- ADRs: ADR-0034, ADR-0045, ADR-0053
