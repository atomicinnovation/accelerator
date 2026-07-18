---
type: work-item
id: "0179"
title: "corpus and corpus-adapters Crates for Parsing and Conventions"
date: "2026-07-06T22:27:35+00:00"
author: Toby Clemson
producer: refine-work-item
status: done
kind: task
priority: high
parent: "work-item:0166"
external_id: PP-703
blocks: ["work-item:0180", "work-item:0170", "work-item:0173", "work-item:0168"]
tags: [rust, config, corpus, store, crates, dedup, frontmatter, serde, vcs, metadata]
last_updated: "2026-07-11T11:10:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0179: corpus and corpus-adapters Crates for Parsing and Conventions

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Build the `corpus` (domain + ports) and `corpus-adapters` crates for parsing and
convention logic over the meta corpus — frontmatter, doc-type inference,
typed-linkage, slug/path conventions, the work-item-ID runtime predicate, the
frontmatter write-convention, and artifact-metadata derivation. This is a
**rewrite that consolidates**, not a mechanical lift: the visualiser already
implements most of this in Rust (the duplication ADR-0045 exists to remove) and
the 0178 config crates established the domain/adapters pattern to follow. The
rewrite is licensed to improve on the visualiser's implementation while
following (not merely copying) the 0178 pattern — introducing a shared
document-format utility crate, adopting a serde-free domain value model, and
collapsing the duplications that have accreted across the bash library, the
visualiser, and the migration framework into one implementation.

## Context

Child of 0166 — Shared config, corpus, and store Crates. The visualiser
duplicates the bash library, and several facts are triplicated or worse across
the codebase (see Technical Notes). 0178 (config crates, **done**) proved the
crate conventions — `X` domain crate (deps: `kernel` only) + `X-adapters`
(outbound infra), enforced by cargo-deny + cargo-pup — and produced the
serde-free `config::Node` value model and the serde-saphyr YAML boundary this
task reuses.

This task deliberately reaches **beyond a like-for-like extraction** in three
ways the investigation surfaced:

1. It introduces a **shared markdown+frontmatter document-format crate** consumed
   by both `config-adapters` and `corpus-adapters`, rather than letting `corpus`
   become a third independent implementation of frontmatter splitting/parsing.
2. It **subsumes the whole three-script artifact-metadata family**, not just
   `artifact-derive-metadata.sh`, and separates VCS-kind detection from
   repo-root detection as distinct concerns.
3. It **consolidates duplications** that exist even inside the visualiser (two
   `{number:0Nd}` width parsers, three title-casers).

Boundary with siblings: 0179 builds the **libraries**; 0173 (`accelerator-corpus`)
is the inbound CLI that consumes them and owns corpus-frontmatter *validation*
(out of scope here); 0168 folds the visualiser into `cli/` and touches the same
code this task extracts; 0170 (`accelerator-work`) is built *on top of* corpus
and owns the work-item lifecycle and the ID pattern DSL compiler; 0180 lands the
atomic-store primitives in `corpus-adapters`.

## Requirements

### Crate topology

- **`corpus`** (domain + ports) — deps: `kernel` only. No serde, no YAML, no
  outbound infra. Models frontmatter as a domain value type, the doc-type fact
  (single-sourced), typed-linkage (ADR-0034), slug/path conventions, the
  work-item-ID runtime predicate, and artifact-metadata as domain concepts +
  ports. Import restrictions enforced by cargo-pup and cargo-deny (mirror the
  `config` domain rules).
- **`corpus-adapters`** — deps: `corpus` + the shared document-format crate +
  infra. Real frontmatter parse, doc-type inference, typed-linkage resolution,
  slug/path derivation, work-item-ID scan/extract/normalise, the frontmatter
  write-convention, and artifact-metadata derivation. Clock and VCS seams behind
  faked ports.
- **New shared document-format crate** (naming TBD — e.g. `document`) — a utility
  crate purely about the markdown-with-frontmatter *protocol/format*, consumed by
  **both** `config-adapters` and `corpus-adapters`. Owns: fence splitting,
  frontmatter parsing, and round-trip rendering. serde-saphyr is confined here
  (the `deny.toml` wrapper moves from `config-adapters` to this crate). See the
  document-format requirement below.

### Domain value model

- Model parsed frontmatter with a **serde-free domain value enum** mirroring
  `config::Node` (`Scalar`/`Sequence`/`Mapping`; `Scalar` = `String`/`Bool`/
  `Int`/`Float`/`Null`), order-preserving. **Do not** propagate the visualiser's
  `BTreeMap<String, serde_json::Value>` into the domain — `serde_json::Value` is
  load-bearing there only because `IndexEntry` serialises to the SPA over HTTP;
  the domain gets a serde-free type and the adapter/API boundary implements
  `Serialize` (config's `Parsed` proves a hand-written `Serialize`/`Deserialize`
  round-trips cleanly).
- **Big integers beyond `i64` are preserved as `String`** (config's policy),
  not widened to `f64`/`Null` (the visualiser's policy).
- YAML tags in frontmatter are treated as a clean parse error / malformed state —
  never a panic or silent drop. This matches the effective behaviour of both
  existing implementations (visualiser → `Malformed`, config → serde error); no
  corpus document uses YAML tags. See Drafting Notes.

### Shared document-format crate

- Expose the fence split in **both** forms the two existing consumers need: a
  byte-offset form with a scan cap (the visualiser's `fence_offsets`, needed by
  the frontmatter write-convention for in-place edits) and an owned-string-halves
  form (config's `split`, needed for round-trip render). The owned form derives
  from the offset form.
- Parse the frontmatter block via **serde-saphyr** (pure-Rust). Because
  serde-saphyr has no C backend, **drop the `catch_unwind` libyml sandbox** the
  visualiser's `serde_yml` forces today; replace the panic-regression test with
  adversarial-input fixtures asserting a bounded, no-panic/no-abort/no-hang floor
  (the 0178 approach).
- Provide round-trip rendering that preserves the existing body (config's
  `preserved_body`).
- **Retrofit `config-adapters` (0178) onto this crate** so there is a single
  implementation of the document protocol — `config-adapters` no longer parses
  YAML independently. This is in scope for 0179 (see Drafting Notes); the 0166
  epic is updated to reflect the fifth crate.

### Conventions to extract (rewrite, not verbatim)

- **Doc-type fact, single-sourced**: extract the pure `DocTypeKey` enum (14
  variants) and its `match`-based predicates. Converge the three
  *materialisations* of the dir→type fact — the runtime table
  (`doc-type-table.sh`), the 0007 migration snapshot, and
  `0007-frontmatter-rewrite.awk` — onto `corpus` as the single source. Leave the
  `Config`-coupled `DocType`/`describe_types` projection server-side.
- **Typed-linkage parse primitive**: extract the pure ADR-0034 parser
  (`typed_ref`), at parity with the bash typed-linkage parser
  (`scripts/linkage-parser.sh`). Keep the server-entangled cluster walker
  (`indexer`/`IndexEntry` coupling) out of scope.
- **Slug/path conventions**: extract the per-doc-type slug dispatch (ADR-`N`,
  bare-`N`, date-prefixed, optional embedded work-item-id). Re-path the
  bash-parity harness (see Technical Notes).
- **Work-item-ID runtime predicate**: `extract_id` / `normalise_id` /
  `is_canonical_id_token` / `canonical_digit_width` land in `corpus`. The
  compiled scan regex is **injected** — the `work.id_pattern` → regex DSL
  compiler (`work-item-common.sh`) is **out of scope** (it is a work/config
  concern, home to 0170/0167). Corpus recognises work-item references embedded
  across every dated doc-type and in cross-doc linkage, which is why the runtime
  predicate belongs at the corpus layer rather than in a `work` crate (moving it
  would invert the layering).
- **Frontmatter write-convention**: extract the in-place frontmatter mutator
  (`patcher` — line-preserving `status:` replacement; quote-style, inline-comment,
  and CRLF preservation) into `corpus-adapters`. Built on the shared crate's
  offset-form split.

### Artifact-metadata derivation

- Reach parity with **all three** members of the metadata-helper family —
  `artifact-derive-metadata.sh`, `gap-metadata.sh`, `inventory-metadata.sh` —
  which are byte-identical in their VCS block and differ only in filename-timestamp
  format. Subsume the family, not one script. The four facts: current UTC
  date/time, filename timestamp (parameterised format), repository name, and
  current revision.
- Model **VCS-kind detection** ("which VCS is in use") and **repo-root
  detection** ("where is the root repository") as **two separate concerns/ports**
  — they serve different purposes and need not share a technique. The current
  approaches disagree (the metadata scripts use a VCS-command probe; `vcs-common.sh`
  and the Rust `config-adapters discover_root` use a filesystem marker-walk).
  Choosing each port's technique is an **in-scope investigation** (spike-sized —
  reconcile secondary jj workspaces, colocated repos, and bare repos) that
  precedes the VCS-port implementation; see Open Questions.
- The clock, VCS-kind, and repo-root seams are ports with faked implementations
  so every derived field is asserted deterministically.

### Consolidation (licensed by the rewrite)

- Collapse the two `{number:0Nd}` width parsers
  (`indexer::number_width_from_id_pattern` and
  `WorkItemConfig::canonical_digit_width`) into one.
- Collapse the three independent title-casers (`slug::humanise_slug`,
  `config::label_from_key`, `library::humanise_status`) into a single
  convention helper where they share semantics.

## Acceptance Criteria

- [x] The `corpus` domain crate depends on `kernel` only and imports no serde,
      serde_json, serde_yml, or serde-saphyr symbol; cargo-deny + cargo-pup fail
      the build if it does.
- [x] Parsed frontmatter is represented by a serde-free domain value enum; a
      round-trip (parse → render) through `corpus-adapters` preserves the body
      byte-for-byte, and big integers beyond `i64` survive as `String`.
- [x] The shared document-format crate is the **only** crate importing
      serde-saphyr (enforced by the `deny.toml` wrapper), and both
      `config-adapters` and `corpus-adapters` obtain frontmatter split/parse
      through it — `config-adapters` no longer parses YAML independently.
- [x] Frontmatter parsing of an enumerated adversarial-input fixture set — the
      0178 plan's adversarial fixtures plus the visualiser's trailing-whitespace
      quoted-flow-scalar regression — returns a clean malformed/error result
      under the 0178 plan's bounded-time guard (cited in References): each parse
      must not panic, abort, or hang, without a `catch_unwind` guard.
- [x] `corpus-adapters` parses frontmatter, infers doc types, and resolves
      typed-linkage and slugs over the `corpus` domain types at parity with the
      bash sources — crate output compared against `doc-type-inference.sh`,
      `linkage-parser.sh`, and `work-item-pattern.sh`'s slug/path conventions over
      a shared fixture corpus that spans each of the 14 `DocTypeKey` variants, all
      three identity schemes (ADR-`N`, bare-`N`, date-prefixed), and the
      optional-embedded-work-item-id cases.
- [x] The dir→type fact is single-sourced in `corpus` — no triplication —
      verified by a test asserting the 0007 migration snapshot and the
      `0007-frontmatter-rewrite.awk` table derive from the crate's `DocTypeKey`
      source rather than re-declaring it.
- [x] Work-item-ID `extract`/`normalise`/`is-canonical` match the bash
      `work-item-pattern.sh` behaviour, reusing that script's existing test suite
      as the parity baseline, with the scan regex injected (the pattern DSL
      compiler is not implemented here).
- [x] The frontmatter write-convention performs a `status:` value replacement
      that preserves surrounding quote style, inline comments, CRLF line endings,
      and the untouched body, matched against fixtures.
- [x] Artifact-metadata derivation reaches parity with all three helper scripts —
      current UTC date/time, a parameterised filename timestamp, repository name,
      and current revision — with VCS-kind detection, repo-root detection, and the
      clock behind faked ports so each field is asserted deterministically.
- [x] Each VCS port's chosen technique (marker-walk vs command-probe) is recorded
      (in the plan or a decision note), and fixtures assert correct repo-root and
      VCS-kind resolution for a secondary jj workspace, a colocated repo, and a
      bare repo.
- [x] The two `{number:0Nd}` width parsers are collapsed into one; the
      title-casers that share semantics are collapsed into a single helper and
      reused, with any intentional divergence (e.g. `humanise_status` vs
      `humanise_slug`) documented.

## Open Questions

- **New crate naming and placement**: name for the document-format utility crate
  (e.g. `document`, `frontmatter`, `docformat`), and where it sits in the
  workspace layering (a utility beside `kernel`, consumed by both adapter
  crates). Deferred to the plan; does not block the scope of this work item.
- **VCS-kind vs repo-root techniques** (in-scope investigation): which technique
  does each port use — filesystem marker-walk (unifying with `config-adapters
  discover_root` and `vcs-common.sh`) or VCS-command probe (the metadata scripts'
  current approach)? Marker-walk and command-probe can diverge on secondary jj
  workspaces, colocated repos, and bare repos. This spike-sized investigation is
  part of 0179 and precedes the VCS-port implementation.

**Resolved during review 1** (moved into Requirements / Drafting Notes):
YAML-tag handling (clean parse error / malformed), the config-adapters retrofit
(in scope for 0179), and the big-int/number policy (preserved as `String`, no
tag variant).

## Dependencies

- Blocked by: 0166 crate-layer conventions (parent); 0178 config crates (done —
  supplies the `config::Node` pattern, the serde-saphyr boundary, and the
  `config-adapters` code this task retrofits onto the shared document-format
  crate).
- Blocks:
  - 0180 — atomic-store primitives land in `corpus-adapters`, which this task
    creates.
  - 0170 (`accelerator-work`) — consumes `corpus`; cannot land its
    library-consuming work until these crates exist.
  - 0173 (`accelerator-corpus` CLI) — consumes these libraries; owns
    corpus-frontmatter validation (out of scope here).
  - 0168 — folds the visualiser into `cli/` over the same code this task
    extracts; **0179 extracts first**, then 0168 folds the remaining server. The
    two are sequenced, not independent.
- No dependency on 0167: the work-item-ID scan regex is injected from the
  existing bash `work-item-pattern.sh` compiler at test time, so 0179 does not
  depend on 0167/0170 delivering the Rust DSL compiler.
- Parent: 0166.

## Assumptions

- The visualiser's corpus code is extractable with bounded effort; entanglement
  grades run from `typed_ref.rs` (trivial) to `cluster_key.rs` (highest, and
  partly out of scope via its `indexer`/`IndexEntry` coupling). The pure
  convention leaves (`typed_ref`, `frontmatter`, `slug`, `docs`/`DocTypeKey`,
  `WorkItemConfig`, `patcher`) form a natural `corpus` domain and extract with
  near-zero surgery; `cluster_key`/`clusters`/`build_entry` are gated on
  moving/redefining a parse-time core out of `IndexEntry`.
- 0178 being **done** de-risks the parser choice: serde-saphyr is already proven,
  pinned (`=0.0.29`), and fenced by a cargo-deny wrapper in this repo, so the
  serde-free-domain + serde-saphyr-in-adapters direction is a known-good pattern
  rather than a bet.
- Corpus-frontmatter *validation* is 0173's concern, not 0179's; this task ships
  the parse/convention primitives validation is later built on.

## Technical Notes

**Size**: L — the heaviest of the three 0166 siblings: six-plus visualiser twins
of varying entanglement, a new shared document-format crate (plus an 0178
retrofit if in scope), a greenfield-ish artifact-metadata piece spanning a
three-script family and two VCS concerns, the serde-free-value-model rewrite, and
the slug bash-parity harness needing re-pathing when the crate moves into `cli/`.

### serde_json → serde-free domain value model

- The visualiser represents parsed frontmatter as
  `BTreeMap<String, serde_json::Value>` (`frontmatter.rs:74`), built by
  `yml_to_json` (`frontmatter.rs:196-235`). `serde_json` is load-bearing for
  exactly one reason: `IndexEntry.frontmatter: serde_json::Value`
  (`indexer.rs:172`) rides onto the SPA's JSON wire via `IndexEntry`'s
  `Serialize` derive (`indexer.rs:160-161`).
- Every production consumer of the value bag uses only generic accessors —
  `.get(key)`, `.as_str()`, `.as_array()`, number-stringify — at
  `indexer.rs:80-88,985,1443-1457` and `cluster_key.rs:86,133`. None needs JSON
  semantics; a `config::Node`-style enum supplies all of them. The single JSON
  dependency is the API serialisation, which the adapter/API boundary satisfies
  with a hand-written `Serialize` (config's `Parsed`,
  `config-adapters/document.rs:115-211`, is the proof).
- `serde-saphyr` lives only in `config-adapters` today
  (`config-adapters/Cargo.toml:22`, `document.rs:62,68`), pinned `=0.0.29`
  (`cli/Cargo.toml:42`), fenced by `cli/deny.toml:64-69`
  (`wrappers = ["config-adapters"]`) with a dedicated ban test. The wrapper moves
  to the new document-format crate.

### libyml panic guard becomes obsolete

- The visualiser wraps `serde_yml::from_str` in `catch_unwind`
  (`frontmatter.rs:143-154`) because libyml (a C port) *panics* rather than
  erroring on adversarial input (e.g. a quoted flow scalar with trailing
  whitespace → "String join would overflow memory bounds"). Regression test:
  `malformed_when_quoted_scalar_has_trailing_whitespace`
  (`frontmatter.rs:415-429`). serde-saphyr is pure-Rust with no such hazard
  (0178 plan `:544-554`), so the guard is deleted and replaced by
  adversarial-input fixtures under a bounded-time floor (0178 plan `:785-792`).
- Behavioural deltas between the two existing parsers to reconcile in the
  rewrite: number widening (visualiser → `f64`/`Null`, `frontmatter.rs:201-211`;
  config → `String`, `document.rs:132-135` — **rewrite keeps config's String
  policy**) and YAML tags (visualiser → `Malformed`, `frontmatter.rs:233,181-186`;
  config → serde error — **rewrite: clean parse error, per Open Question**).

### Shared document-format crate

- Two independent fence-splitters exist today with divergent shapes a shared
  crate must reconcile:
  - Visualiser `fence_offsets` (`frontmatter.rs:21-70`) — operates on raw
    `&[u8]`, returns byte-offset ranges, enforces a 1 MiB `MAX_SCAN` cap, is
    CRLF-tolerant, distinguishes absent (`None`) from unclosed (`Err`). Reused by
    the write-convention (`patcher.rs:38`).
  - Config `split` (`config-adapters/frontmatter.rs:18-52`) — operates on `&str`,
    returns owned `String` halves, CRLF-tolerant, no scan cap, does not rescan the
    body, `Err` only on an unterminated block. Consumed by `document.rs:31,53`
    including `preserved_body` for round-trip render.
- The offset form is the primitive; the owned-halves form derives from it. A
  single crate exposing both, plus serde-saphyr parse and round-trip render, lets
  `config-adapters` and `corpus-adapters` share one document protocol.

### Visualiser structure and the IndexEntry bottleneck

- The server is one flat library crate (`lib.rs:9-31`, 20 peer modules; only
  `api/` is nested). There is no domain/adapters boundary, but an **implicit
  dependency-ordered layering** already exists: `typed_ref` → (nothing);
  `slug`/`docs` → `WorkItemConfig`; `frontmatter` → `slug` + `typed_ref`;
  `indexer::build_entry` → `frontmatter` + `slug` + `WorkItemConfig`; `cluster_key`
  → `indexer` helpers + `IndexEntry`; `clusters` → `cluster_key`; `related` →
  `Indexer`.
- Extraction entanglement grades:
  - **Trivial** (pure over primitives + `&WorkItemConfig`): `typed_ref.rs`
    (~72 logic lines, ADR-0034 parser, sole dep `std::path`), `frontmatter.rs`,
    `slug.rs`, `WorkItemConfig` (`config.rs:64-251`), `docs.rs` `DocTypeKey`
    (14 variants, `docs.rs:6-21`, all-`match` self-methods), `patcher.rs`, and
    the `indexer.rs` id helpers — `canonicalise_one_id` (`:1207`),
    `canonicalise_refs` (`:1254`), `number_width_from_id_pattern` (`:1193`),
    `parse_adr_id` (`:1464`), `normalize_absolute` (`:903`), `normalize_target_key`
    (`:933`).
  - **Moderate** (gated on a parse-time core moving out of `IndexEntry`):
    `cluster_key.rs`, `clusters.rs`, `indexer::build_entry` (`:1324`, the central
    per-document assembler), `target_path_from_entry` (`:974`).
  - **High** (stays server-side): `related.rs` (calls `&Indexer` secondary-index
    methods under async locks), the `Indexer` store itself (six
    `Arc<RwLock<HashMap>>` indexes, tokio-fs), and `file_driver.rs` listing.
- `IndexEntry` (`indexer.rs:160-196`) is the coupling hub — `cluster_key`,
  `clusters`, `related`, `build_entry`, and the facet functions all speak it. A
  clean split extracts a parse-time core (`type`, `path`, `slug`, `work_item_id`,
  `title`, `frontmatter`, `frontmatter_state`, `work_item_refs`) and leaves the
  store/back-fill fields (`etag`, `mtime_ms`, `size`, `completeness`,
  `linked_count`) server-side.
- `docs.rs` coupling is isolated: the pure `DocTypeKey` enum extracts clean; the
  `Config`-coupled `DocType` struct + `describe_types` (`docs.rs:180-222`) are the
  visualiser-runtime projection and stay server-side.
- `cluster_key` split: extract only the pure linkage primitives (`id_from_value`
  `cluster_key.rs:147-161`, plus the `indexer.rs` id helpers above). The
  recursive walker (`resolve_cluster_key`/`walk`, `cluster_key.rs:32-121`) with
  its `IndexEntry`/`target_path_from_entry`/`normalize_target_key` deps is
  store-side and out of scope.

### Doc-type triplication (subtler than one fact in three files)

- `doc-type-inference.sh` already loads its table from `doc-type-table.sh`
  (single-sourced by the 0007 schema via `config-read-doc-type-paths.sh`). The
  three *materialisations* of the dir→type fact to converge onto `corpus` are
  the runtime table (`doc-type-table.sh`), the 0007 migration snapshot
  (`0007-unify-meta-corpus-frontmatter.sh:49-66`), and
  `0007-frontmatter-rewrite.awk`.

### Work-item-ID: runtime predicate vs DSL compiler

- The runtime predicate (`config.rs:129-251`: `is_canonical_id_token`,
  `canonical_digit_width`, `extract_id`, `normalise_id`) is work-item-specific in
  naming/config/keying (`work.*`, the `work_item` config.json key), but it is a
  **cross-cutting dependency of the corpus layer**: `slug::derive`
  (`slug.rs:22-72`) dispatches per doc-type across three *different* identity
  schemes — ADR-`N` (literal `"ADR-"` prefix), bare-`N` work items, and
  date-prefixed plans/research/etc. (no NNNN id of their own) — and uses the
  predicate to strip an *optional embedded work-item id* on the dated types
  (`strip_optional_work_item_id_prefix`, `slug.rs:139-170`) and to resolve
  cross-doc linkage. So corpus depends on the predicate; a `work` crate owning it
  would invert the layering (0170's `accelerator-work` depends on corpus, not the
  reverse).
- The genuinely work-specific seam is the **pattern DSL compiler**
  (`work-item-common.sh` `wip_validate_pattern`/`wip_compile_scan`/
  `wip_compile_format`, wrapped by `work-item-pattern.sh`), which validates and
  compiles `work.id_pattern` into a `scan_regex` at the config/launcher boundary.
  The visualiser's `WorkItemConfig` receives the *already-compiled* regex
  (`config.rs:79-105`). The rewrite keeps this seam: corpus takes the compiled
  regex by injection; the compiler is out of scope (work/config — 0170/0167).

### Artifact-metadata: a family, not a script; two VCS concerns

- Three near-identical helpers share the metadata contract, yoked by
  `scripts/test-metadata-helpers.sh:21-25`: `artifact-derive-metadata.sh`,
  `skills/design/analyse-design-gaps/scripts/gap-metadata.sh`, and
  `skills/design/inventory-design/scripts/inventory-metadata.sh`. Their VCS
  blocks are byte-identical; they differ only in filename-timestamp format
  (`_H-M-S` vs date-only vs `-HMMSS`). The port parameterises that format and
  subsumes all three.
- The jj→git→empty logic is duplicated well beyond the helpers: the canonical
  marker-walk root detection in `scripts/vcs-common.sh:8-36`
  (`find_repo_root`/`vcs_mode`), `hooks/vcs-detect.sh`, and the 0007 migration's
  `resolve_revision` (`:250-261`, short-id, file-scoped). Rust already has a
  root-walk twin — `cli/config-adapters/src/store.rs:33-45 discover_root` — but it
  uses a **marker-walk**, whereas the metadata scripts use a **VCS-command probe**
  (`jj root` / `git rev-parse --show-toplevel`). No Rust code derives a revision.
- Per the design decision, VCS-kind detection ("which VCS") and repo-root
  detection ("where is root") are **separate ports** with distinct purposes;
  which technique each uses (marker-walk vs command-probe) is open and the two
  can diverge on secondary jj workspaces, colocated repos, and bare repos
  (handled only by `vcs-common.sh` today). Repo-name derives from the repo-root
  port; revision derivation composes both.

### Slug parity harness re-pathing

- `slug.rs:572-602` shells out to `work-item-pattern.sh --compile-scan` via a
  `CARGO_MANIFEST_DIR`-relative path (`../../../../skills/work/...`) that breaks
  when the crate moves into `cli/`. The bash script stays put; the harness needs
  re-pathing.

### Intra-visualiser duplications to collapse in the rewrite

- Two `{number:0Nd}` width parsers: `indexer::number_width_from_id_pattern`
  (`:1193`) and `WorkItemConfig::canonical_digit_width` (`config.rs:146`).
- Three title-casers: `slug::humanise_slug` (`slug.rs:196`, which explicitly
  notes the duplication at `:230-231`), `config::label_from_key` (`:307`), and
  `library::humanise_status` (`:253`).

## Drafting Notes

- Framed this work as a **consolidating rewrite** rather than a mechanical
  extraction, per the direction to improve/consolidate the previous
  implementation. Several requirements (the shared document-format crate,
  subsuming all three metadata helpers, the intra-visualiser dedup) go beyond
  "port the twins" — a reviewer who expected a straight lift should note the
  widened scope.
- **Corpus-vs-work placement**: kept the runtime work-item-ID predicate in
  `corpus` (not a `work` crate) on a layering argument — corpus's own slug and
  linkage code depends on it, and relocating it would invert the crate
  dependency direction. If a reviewer prefers strict semantic placement over
  layering, this is the call to challenge; the DSL-compiler carve-out is the
  release valve.
- **New crate + retrofit are in scope (review-1 decision)**: the shared
  document-format crate is a fifth crate the parent epic did not plan, and the
  0178 `config-adapters` retrofit onto it is included in 0179 (not deferred) so
  there is a single document-protocol implementation. The 0166 epic must be
  updated to reflect five crates. This widens 0179's blast radius into shipped
  0178 code deliberately.
- **Multi-thread L scope retained (review-1 decision)**: 0179 keeps the
  artifact-metadata + clock/VCS-ports thread alongside the parse/convention
  crates rather than splitting it out, despite the threads being independently
  deliverable. Confirmed as one `task` (review-1 pass 2): the sizing risk is
  accepted deliberately rather than split into a `story` with child tasks.
- **YAML-tag handling (review-1 decision)**: chosen as a clean parse error /
  malformed state, with no tag variant in the domain value model — matching both
  existing implementations and the fact that no corpus document uses YAML tags.
  Revisit only if a document legitimately needs a tag.
- **Library-vs-CLI boundary**: interpreted 0179 as strictly the libraries and
  assigned artifact-metadata's *command* surface and corpus-frontmatter
  *validation* to 0173. If the intent was for 0179 to also ship a CLI or the
  validator, that boundary needs revisiting.
- **Value-model policy**: chose config's big-int-as-`String` policy over the
  visualiser's `f64`/`Null` widening, and a no-tag-variant value model, to
  converge corpus with the already-shipped config semantics. These are
  behaviour-visible choices a reviewer should confirm.

## References

- Parent: `meta/work/0166-shared-config-corpus-store-crates.md`
- Siblings/consumers: `meta/work/0178-config-crates-native-yaml-reader.md`,
  `meta/work/0180-atomic-store-primitives-corpus-adapters.md`,
  `meta/work/0168-fold-visualiser-into-cli-workspace.md`,
  `meta/work/0170-work-item-subdomain-and-sync-engine.md`,
  `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
- Convention specs: ADR-0034 (typed-linkage), ADR-0045 (bash/Rust duplication),
  ADR-0053; `meta/work/0060` (unified frontmatter schema), `meta/work/0061`
  (typed-linkage vocabulary), `meta/work/0064` (canonical work-item-id/author)
- 0178 parser rationale: `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md:544-554,785-792`
