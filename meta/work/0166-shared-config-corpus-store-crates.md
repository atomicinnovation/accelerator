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
blocked_by: ["work-item:0163"]
relates_to: ["work-item:0162"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, config, corpus, store, crates, dedup]
last_updated: "2026-07-06T22:16:29+00:00"
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
artifact-metadata, plus the atomic-store JSONL/locking primitives) — collapsing the
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
  `agents.*`). The reader **fails closed on a legacy `.claude/accelerator.md`
  layout** (porting `config_assert_no_legacy_layout`), directing the user to
  `/accelerator:migrate` rather than reading legacy config — it does not carry the
  migration-internal `ACCELERATOR_MIGRATION_MODE` read fallback. Each sub-binary
  wires its own `config-adapters` at its composition root (Model 1).
- `corpus` (domain + ports) and `corpus-adapters`: frontmatter parse (real YAML),
  doc-type inference (consolidating the triplicated directory→type fact),
  typed-linkage (ADR-0034), slug/path conventions, work-item-ID
  scan/extract/normalise, artifact-metadata derivation. `corpus-adapters` also
  houses the atomic-store primitives — `atomic_write` (temp + rename), the
  `mkdir`-based lock with PID-owner reclaim and jittered back-off, and
  canonical-order JSONL compose/remove — porting the load-bearing concurrency
  semantics from `atomic-common.sh`/`jsonl-common.sh`. No standalone `store` crate
  for now (see Drafting Notes). Extract from the visualiser where it already exists
  rather than re-deriving.
- All crates unit-tested in isolation with faked ports (ADR-0053); the cargo-deny
  ban-lists from 0162 first bite at the `config`/`config-adapters` split here.

### Child work items

- 0178 — config and config-adapters Crates with Native YAML Reader
- 0179 — corpus and corpus-adapters Crates for Parsing and Conventions
- 0180 — Atomic-Store Primitives in corpus-adapters

## Acceptance Criteria

- [ ] The native config reader resolves team→local precedence, arbitrary nesting,
      and every recognised key (`paths.*`, `templates.*`, `work.*`, `review.*`,
      `agents.*`), with unit tests against in-memory fakes; behaviour matches the
      bash reader (`config-read-value.sh` over `config-common.sh` /
      `config-defaults.sh`) on a shared corpus of fixtures.
- [ ] Given a repo still on the legacy `.claude/accelerator.md` layout (no
      `.accelerator/config.md`), when any Rust config-reader entry point runs, then
      it exits non-zero with a "run /accelerator:migrate" message rather than
      reading legacy config — at parity with `config_assert_no_legacy_layout`
      (`config-common.sh`).
- [ ] Each sub-binary wires its own `config-adapters` at its composition root
      (Model 1): a composition-root test/example constructs the reader from
      concrete adapters, and the `config` domain layer carries no outbound
      dependency (enforced by cargo-pup).
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
- [ ] `corpus-adapters` provides atomic writes (temp + rename) and the mkdir-based
      lock with PID-owner reclaim and jittered back-off, plus canonical-order JSONL
      compose/remove — at parity with `atomic-common.sh` / `jsonl-common.sh` —
      with tests covering contended-lock reclaim of a dead holder.
- [ ] The crates carry no inbound/outbound dependency into their domain layer
      (enforced by cargo-deny + cargo-pup).

## Open Questions

- None outstanding. (The earlier question of whether atomic JSONL + locking is its
  own `store` crate or folded into `corpus-adapters` is resolved: folded into
  `corpus-adapters` for now — see Drafting Notes.)

## Dependencies

- Blocked by: 0163 (workspace skeleton) — **complete**. 0162 (Rust toolchain guard
  rails, `relates_to`) is also **complete**, so both prerequisites are satisfied
  and this story is unblocked.
- Blocks: 0167 (built-in config command), 0168 (visualiser refactor), 0169 (vcs),
  0170 (work), 0171 (integrations), 0172 (migrate), 0173 (remaining subdomains) —
  all consume these crates.
- Parent: epic 0136.

## Assumptions

- The visualiser's existing corpus code is extractable into shared crates with
  bounded effort (it is currently entangled with `serde_json::Value` representations
  but is axum-free).

## Technical Notes

**Size**: L — four new crates (`config`, `config-adapters`, `corpus`,
`corpus-adapters`) plus their cargo-deny/cargo-pup rule activation; a
from-scratch native YAML reader (the bash 2-level reader and the visualiser's
JSON-reading `config.rs` are both non-reusable); extraction of six visualiser
twins spanning trivial (`typed_ref.rs`) to highest-effort (`cluster_key.rs`,
entangled with the out-of-scope `indexer`/`IndexEntry`); and a careful port of
subtle concurrency primitives (mkdir-lock + PID reclaim + jittered back-off +
canonical JSONL) whose bash semantics must be reasoned through for Rust.

- Source bash: `scripts/config-common.sh` (including `config_assert_no_legacy_layout`,
  the fail-closed guard to port), `config-defaults.sh`, `config-read-value.sh`
  (2-level awk reader), `doc-type-inference.sh`, `linkage-parser.sh`,
  `atomic-common.sh`, `jsonl-common.sh`, `skills/work/scripts/work-item-pattern.sh`.
- Visualiser twins to extract from: `frontmatter.rs`, `docs.rs`, `slug.rs`,
  `config.rs` (`WorkItemConfig`), `typed_ref.rs`, `cluster_key.rs`.
- The `ACCELERATOR_MIGRATION_MODE` legacy-read fallback in `config-common.sh` is
  migration-internal plumbing (it lets migrations 0001/0002 read config before
  migration 0003 relocates it) and is set only by the bash migration scripts. It is
  deliberately **not** ported here. If a future Rust migration engine (0172) invokes
  the Rust config reader mid-migration, re-introducing that fallback is 0172's
  concern, not this story's.
- Hexagon/composition-root template to mirror: `cli/launcher/src/main.rs:86-92`
  (the `run` composition root that wires concrete adapters to port traits — the
  Model 1 pattern AC-3 requires), `cli/launcher/src/launch/core.rs:174-202`
  (driven-port traits + the `FixedResolver`/`RecordingExec` faked ports for
  isolation tests, ADR-0053), `cli/kernel/src/lib.rs:9-15` (the shared `Error`
  taxonomy each subdomain maps into at the dispatch boundary).
- Enforcement scaffolding to activate (currently inert, waiting on this split):
  `cli/deny.toml:67-73` — the infra-out-of-domain ban has empty `skip`/`wrappers`
  lists, commented "until the config/config-adapters split makes the rule bite";
  `cli/pup.ron:10-39` — the per-domain `RestrictImports { allowed_only: [...] }`
  rules for `version::core`/`launch::core`, extended one-per new `config`/`corpus`
  domain module.
- Per-twin extractability (informs sequencing, cheapest→dearest): `typed_ref.rs`
  trivial (~72 logic lines, pure ADR-0034 parser, no deps); `slug.rs`/`docs.rs`/
  `WorkItemConfig` moderate (share `DocTypeKey`/`WorkItemConfig`, travel together);
  `frontmatter.rs` moderate-high (needs a domain frontmatter type vs. propagating
  the `serde_yml`→`serde_json::Value` map, plus a `catch_unwind` around libyml
  panics); `cluster_key.rs` highest — coupled to `indexer::{canonicalise_one_id,
  target_path_from_entry, IndexEntry}`, out of the twin list, so extract the
  linkage *parse* primitive not the server-coupled walker.
- Reader-scope caveat: the visualiser's `config.rs` deserialises a pre-baked
  `config.json` (bash `launch-server.sh` produces it) — it is **not** the native
  YAML reader. Only `WorkItemConfig` (`config.rs:83-251`) is reusable corpus
  logic; the new `config` crate ports the bash YAML reader from scratch.

## Drafting Notes

- Treated as the Phase 3 shared-crate story; it is the dedup linchpin the visualiser
  refactor (0168) and every subdomain build on.
- Atomic-store JSONL + locking is folded into `corpus-adapters` for now rather than
  carved into a standalone `store` crate; the title retains "store" because the
  capability is still delivered, and a later split stays open if a second consumer
  needs it independently.
- Dropped the `ACCELERATOR_MIGRATION_MODE` legacy-read path: it is not user-facing
  backward-compat but migration-internal plumbing, and normal readers already
  fail closed on a legacy layout. The Rust reader ports that fail-closed assertion
  (safe: no silent config loss) and leaves any Rust-migration-engine fallback to 0172.
- Boundary with 0173: the frontmatter/doc-type/typed-linkage *parsing* logic lands
  only in this story's `corpus`/`corpus-adapters` crates; 0173's `accelerator-corpus`
  binary merely *calls* them, so the two must not re-implement or diverge.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: 0162 (Rust toolchain guard rails), 0163 (cli/ workspace skeleton),
  0167–0173 (consumers of these crates)
- ADRs: ADR-0045, ADR-0047, ADR-0053
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0009-multi-level-configuration-system.md
