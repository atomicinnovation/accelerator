---
type: plan
id: "2026-07-11-0179-corpus-crates-parsing-conventions"
title: "corpus and corpus-adapters Crates for Parsing and Conventions Implementation Plan"
date: "2026-07-11T12:40:21+00:00"
author: Toby Clemson
producer: create-plan
status: in-progress
reviewer: Toby Clemson
work_item_id: "work-item:0179"
parent: "work-item:0179"
derived_from: ["codebase-research:2026-07-11-0179-corpus-crates-parsing-conventions"]
tags: [rust, corpus, config, crates, document, frontmatter, serde-saphyr, doc-type, typed-linkage, slug, work-item-id, vcs, artifact-metadata]
revision: "83707a5ed6c23eaf40f7d1f39f5847dc1801c43f"
repository: "accelerator"
last_updated: "2026-07-12T00:05:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# corpus and corpus-adapters Crates for Parsing and Conventions Implementation Plan

## Overview

Build the parsing- and convention-layer crates for the meta corpus as a
consolidating rewrite onto the 0178 hexagonal pattern. Five new `cli/` crates
plus one retrofit:

- **`document`** — the shared markdown+frontmatter *protocol* crate (fence split,
  serde-saphyr parse, round-trip render), consumed by both `config-adapters` and
  `corpus-adapters`. `config-adapters` is retrofitted onto it so YAML is parsed
  in exactly one place.
- **`corpus`** — the domain crate (`kernel`-only): a serde-free frontmatter value
  model, `DocTypeKey`, the typed-linkage value parser **and single-document linkage
  resolver**, the **doc-type inference matcher**, the work-item-ID runtime 
  predicate + an injected-scanner port, the slug conventions, a `Clock` port, and 
  the artifact-metadata output type. The pure convention *algorithms* live here —
  mirroring how `config` keeps its precedence-resolution and path-walk domain-side —
  taking infra-sourced data (the compiled scan regex, the doc-type table, a parsed
  `FrontmatterValue`) by injection.
- **`corpus-adapters`** — the outbound infra + imperative shell: the
  `document::Yaml → FrontmatterValue` translation and `FrontmatterState`, the
  regex-backed `IdScanner`, sourcing the config doc-type table, the per-document
  assembler that reads a file → parses → **invokes the `corpus` conventions**, the
  frontmatter write-convention, and artifact-metadata derivation.
- **`vcs` / `vcs-adapters`** — a dedicated domain+adapters pair for the
  cross-cutting VCS/repo probe (repo-root, VCS-kind, revision, repo-name), which
  the artifact-metadata service consumes rather than owning.

The rewrite consolidates duplications that have accreted across the bash library,
the visualiser, and the migration framework, and improves on the visualiser's
implementation (serde-free domain value model, pure-Rust parser, no `catch_unwind`
sandbox) rather than lifting it verbatim.

## Current State Analysis

- The 0178 crates prove the pattern: `config` (domain, `kernel`-only) →
  `config-adapters` (serde + serde-saphyr). The serde-free `config::Node`
  (`cli/config/src/node.rs:4-24`), the hand-written serde boundary
  (`cli/config-adapters/src/document.rs:115-211`), the owned-halves `split`
  (`cli/config-adapters/src/frontmatter.rs:18-44`), the marker-walk
  `discover_root` (`cli/config-adapters/src/store.rs:33-45`), and the three
  enforcement mechanisms (serde-saphyr pin `cli/Cargo.toml:42`; cargo-deny
  wrapper ban `cli/deny.toml:64-69`; cargo-pup kernel-only import rule
  `cli/pup.ron:42-56`) are all in place and green.
- The parse/convention logic to consolidate lives in the visualiser server
  (`skills/visualisation/visualise/server/src/`): `typed_ref.rs` (pure ADR-0034
  value parser), `frontmatter.rs` (byte-offset fence split + serde_yml parse
  behind `catch_unwind`), `slug.rs` (per-doc-type slug dispatch), `docs.rs`
  (`DocTypeKey`), `config.rs` (`WorkItemConfig` runtime predicate), `patcher.rs`
  (in-place `status:` write-convention). All are pure over `WorkItemConfig`,
  `DocTypeKey`, and (`typed_ref`) `std::path`; none imports tokio/axum/`Arc`.
- The bash convention library is the parity oracle: `doc-type-inference.sh`
  (longest-configured-dir-wins matcher over the `config-defaults.sh` registry),
  `scripts/linkage-parser.sh` (ADR-0034 document extractor),
  `skills/work/scripts/work-item-pattern.sh` (the pattern DSL compiler + runtime
  predicate).
- Artifact-metadata is three byte-identical helpers
  (`scripts/artifact-derive-metadata.sh`,
  `skills/design/analyse-design-gaps/scripts/gap-metadata.sh`,
  `skills/design/inventory-design/scripts/inventory-metadata.sh`) differing only
  in filename-timestamp format, yoked by the output-contract test
  `scripts/test-metadata-helpers.sh`. VCS detection is triplicated across
  `scripts/vcs-common.sh`, `hooks/vcs-detect.sh`, and the 0007 migration; the only
  Rust twin today is `config-adapters`' `discover_root` marker-walk.

### Desired End State

Five new crates exist in the `cli/` workspace, each `cli:check` / `deny:check` /
`pup:check` clean and unit-tested; `config-adapters` obtains all frontmatter
split/parse/render through `document` and no longer names serde-saphyr; the
serde-saphyr deny-wrapper is re-homed to `document` and its ban regression still
passes; and `corpus-adapters` reproduces the bash convention behaviour at parity
over a shared fixture corpus, with artifact-metadata derived behind faked clock
and VCS ports. Verify by: `mise run check` green end-to-end; the new Rust unit
suites green under `mise run test:unit:cli`; the deny/pup regressions
(`mise run test:integration:deny`, `mise run test:integration:pup`) green; and the
bash parity suites unchanged and still green.

### Key Discoveries

- **The frontmatter value is opaque JSON at every production site** in the
  visualiser (`.get()/.as_str()/.as_array()` only, `indexer.rs:80-88`,
  `cluster_key.rs:86,133`) — never typed deserialisation. A `config::Node`-style
  serde-free enum supplies every need; only the API boundary wants `Serialize`,
  satisfied hand-written (config's `Parsed` proves it).
- **AC-1 (`kernel`-only) forces `regex` out of the domain.** The pure predicates
  (`is_canonical_id_token`, `normalise_id`, `canonical_digit_width`) are plain
  string logic; only `extract_id` and the work-item slug need the compiled scan
  regex, which is *injected* via a domain-defined scanner port implemented in
  `corpus-adapters`. This matches "scan regex is injected" (0179 AC-7).
- **AC-1 also keeps `corpus` from referencing `vcs` value types**, so the
  artifact-metadata composition lives in `corpus-adapters` (which may depend on
  `vcs-adapters`), not the `corpus` domain.
- **The title-caser collapse is only partly reachable here.** `humanise_slug`
  moves to `corpus` and becomes canonical; `config::label_from_key`
  (`config.rs:307`) and `api::library::humanise_status` live in server-runtime
  files that stay server-side until 0168 folds the server into `cli/`, so their
  retirement onto `corpus` is deferred to 0168. Likewise the two `{0Nd}` width
  parsers: the `WorkItemConfig` one extracts into `corpus` now, but its twin
  `indexer::number_width_from_id_pattern` is server-side, so the collapse to a
  single `canonical_digit_width` completes only when 0168 folds the server in —
  0179 lands the canonical copy and pins its default.
- **The doc-type "triplication" is a matcher duplication, not a data one.** The
  dir→type default has a single authoring site (`config-defaults.sh`); what three
  surfaces re-implement is the longest-dir-wins match. AC-6 converges the
  *matcher* onto `corpus` and makes the 0007 migration snapshot + the rewrite awk
  table derive from `DocTypeKey`.
- **The metadata helpers return the full working-copy revision** (per parity), not
  the 0007 migration's short/file-scoped id — file-scoping is 0173's command
  surface, out of scope.

## What We're NOT Doing

- **Not modifying the visualiser server.** 0179 writes new crates; 0168 later
  folds the server into `cli/` and deletes its now-duplicate copies. Duplication
  temporarily increases; the server is untouched here, so there is no server
  regression surface. The corpus fixture corpus reuses the visualiser's own
  frontmatter/slug test inputs so any drift beyond the intended number-policy fork
  (visualiser widens big ints to `f64`/`Null`; corpus keeps `String`) surfaces;
  that fork is a known behaviour delta 0168 must land.
- **Not implementing the work-item pattern DSL compiler** (`work.id_pattern` →
  `scan_regex`). It is a work/config concern (0167/0170); the compiled regex is
  injected. No dependency on 0167/0170 is created.
- **Not corpus-frontmatter validation** (0173) — this ships the parse/convention
  primitives validation is later built on.
- **Not the store-side cluster/related machinery** — `IndexEntry`, the six
  `Arc<RwLock<HashMap>>` indexes, `resolve_cluster_key`/`walk`, `clusters.rs`,
  `related.rs`, `file_driver.rs` stay server-side.
- **Not the full `classify_checkout` taxonomy.** `vcs`/`vcs-adapters` implement the
  helpers' contract (repo-root, VCS-kind, revision, repo-name); the seven-kind
  checkout classifier is a store/0168 concern.
- **Not retiring the two server-side title-casers** (`label_from_key`,
  `humanise_status`) — deferred to 0168.
- **Not the atomic-store primitives** (0180) or a CLI surface (0173).

## Implementation Approach

Follow, don't copy, the 0178 pattern. Each phase is an independently mergeable
unit that leaves `mise run check`, `deny:check`, and `pup:check` green. Work
test-first: the bash parity suites and the visualiser's own test tables are the
behavioural oracle — port the assertions before/with the code and hold the new
crates to them.

**Supported OS matrix**: macOS + Linux (the four release triples plus the ubuntu-gnu
dev graph), matching the bash 3.2 floor and the musl build. The crates use portable
`std` APIs, but retain Unix couplings (VCS tool flags, `time` `local-offset`/tzdata,
bash/awk-coupled parity tests) — Windows is out of scope, stated so the couplings are
an acknowledged boundary, not an accident.

Dependency order: **Phase 1 ∥ Phase 2 ∥ Phase 4** are independent; **Phase 3**
needs 1 + 2; **Phase 5** needs 2 + 4.

Value-model layering (three structurally-identical enums, one per architectural
role, mirroring how `config` already separates domain `Node` from adapter
`Parsed`):

- `document::Yaml` — serde-ful, carries the big-int-as-`String` policy and the
  hand-written `Serialize`/`Deserialize`. Lives in the format layer. Named `Yaml`
  (not `Value`) to avoid colliding with the existing `config::Value` re-export, and
  given the **same nested `Scalar(Scalar)` / `Sequence` / `Mapping` shape** as the
  two domain enums (not the old flat `Parsed` shape) so the three are genuinely 1:1
  and every mapping arm is mechanical.
- `config::Node` — domain (unchanged).
- `corpus::FrontmatterValue` — domain, serde-free, `kernel`-only.

Adapters map `document::Yaml ↔ {Node, FrontmatterValue}` with explicit per-variant
arms — **no `_` wildcard**. For that wildcard-free match to be legal across crate
boundaries, the inner `Scalar` enums (`document::Scalar`, `corpus::Scalar`, and — as
part of the Phase 1 retrofit — `config::Scalar`) **drop `#[non_exhaustive]`**: these
crates are all `publish = false`, so `#[non_exhaustive]` (an external-consumer
forward-compat affordance) buys nothing here and would force the very `_` arm that
silently downgrades a new variant to `Null` (as shipped
`config-adapters/document.rs:101` does today). With it dropped, a future `Scalar`
variant fails to compile at every mapping site. A `document::Yaml ↔ FrontmatterValue
↔ Node` round-trip conformance test additionally pins every scalar kind, including
the three number boundaries (`i64`-range → `Int`; beyond-`i64`-within-`u64` →
`String`; beyond-`u64` → `Float`).

---

## Phase 1: `document` crate + `config-adapters` retrofit

### Overview

Create `cli/document/`, the single implementation of the markdown-frontmatter
protocol, and retrofit `config-adapters` onto it so YAML is parsed in one place
and the serde-saphyr ban re-homes cleanly.

### Changes Required

#### 1. New `document` crate

**File**: `cli/document/Cargo.toml`

```toml
[package]
name = "document"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[[bin]]
name = "document-fixture"
path = "tests/fixtures/document_fixture.rs"

[dependencies]
serde = { workspace = true }
serde-saphyr = { workspace = true }
```

**File**: `cli/document/src/lib.rs` — modules `fence`, `value`, `error`, `parse`,
`render`, with `DocumentError` re-exported at the crate root (mirroring
`config::error::ConfigError`).

- `fence`: the byte-offset primitive and the owned-halves form derived from it.

```rust
pub struct Split {
    pub frontmatter: String,
    pub body: String,
}

pub fn fence_offsets(raw: &[u8]) -> Result<Option<(usize, usize)>, DocumentError>;

pub fn split(content: &str) -> Result<Split, DocumentError>;
```

  `fence_offsets` is the visualiser's `frontmatter.rs:21-70` verbatim in behaviour
  (1 MiB `MAX_SCAN`, CRLF-tolerant, `Ok(None)` absent vs `Err` unclosed). `split`
  reproduces `config-adapters/frontmatter.rs:18-44` (owned halves, body never
  re-scanned) and, because it is expressed in terms of `fence_offsets`, **inherits
  the 1 MiB cap** — a deliberate, documented change: the retrofitted config read
  path gains a 1 MiB ceiling (a negligible-risk DoS bound; configs are KB-sized;
  an over-cap file fails closed as malformed). A boundary test at the `MAX_SCAN`
  edge pins that the offset and owned forms agree. `split` slices the body from
  `body_start` **verbatim** — it does *not* copy the visualiser's `parse` body trim
  (`s[body_start..].trim_start_matches('\n')`, `frontmatter.rs:132`), which lives in
  the visualiser's parse layer, not the fence primitive — so config's byte-for-byte
  body preservation (incl. a body that opens with a blank line) is retained.
  `fence_offsets` also accepts a closing `---` that is the final line with **no
  trailing newline** (`---\n…\n---`), matching config's current `split` — a
  deliberate relaxation of the visualiser's stricter `frontmatter.rs` (which rejects
  it) so the retrofit does not regress a hand-edited config whose last line is the
  closing fence. A `document` test pins this input.

- `value`: `document::Yaml`, a public serde-ful value tree adapted from the private
  `config-adapters::document::Parsed` but **reshaped to the nested `Scalar` shape
  with a `Mapping` newtype** (so it is genuinely 1:1 with `config::Node` /
  `corpus::FrontmatterValue`, incl. the shared insertion-ordered mapping API), with
  its hand-written `Serialize`/`Deserialize` (YAML round-trip) and the
  `u64`-beyond-`i64` → `String` policy (`document.rs:132-135`). An integer beyond
  `u64` arrives via `visit_f64` and lands as `Float` (out of the domain's id
  range — accepted, not a silent bug; pinned by the number-boundary conformance
  test). `Scalar` is **not** `#[non_exhaustive]` (see Value-model layering) so the
  cross-crate mapping arms stay wildcard-free.

```rust
pub enum Yaml {
    Scalar(Scalar),
    Sequence(Vec<Yaml>),
    Mapping(Mapping),
}

pub enum Scalar { String(String), Bool(bool), Int(i64), Float(f64), Null }

pub struct Mapping(Vec<(String, Yaml)>);
```

- `error`: `DocumentError` replaces the `Result<_, String>` façade so consumers
  (and the eventual CLI mapping into `kernel::Error::Failed`) can branch on error
  category; `render`'s three failure modes map to distinct variants.

```rust
pub enum DocumentError {
    Unterminated,
    InvalidYaml(String),
    Emit(String),
}
```

  `Unterminated` / `InvalidYaml` cover a malformed existing file, `Emit` a
  `serde_saphyr::to_string` failure. A non-mapping root is *not* a `DocumentError`
  variant — it is the consumer's policy (see Phase 3's `FrontmatterState`).

- `parse`: `split` + `serde_saphyr::from_str::<Yaml>`; empty frontmatter → an
  empty `Yaml::Map`.
- `render`: `render(existing: Option<&str>, frontmatter: &Yaml) ->
  Result<String, DocumentError>` — the `---\n{yaml}---\n{body}` framing and
  `preserved_body` (`document.rs:41-56`), body preserved byte-for-byte.
  **`preserved_body` must re-parse (not merely fence-split) the existing
  frontmatter**, so a fence-valid-but-invalid-YAML file makes `render` error and
  the caller's write never fires (the shipped fail-closed guarantee). This
  re-parse is part of `render`'s contract and is pinned by a `document`-level test.

#### 2. Retrofit `config-adapters`

**File**: `cli/config-adapters/src/document.rs`
**Changes**: replace the private `Parsed` + its serde impls + the serde-saphyr
call sites with a mechanical nested↔nested mapping between `document::Yaml` and
`config::Node` (both now share the `Scalar(Scalar)` / `Sequence` / `Mapping`
shape); `parse`/`render` keep their signatures and delegate to `document`. Drop
`#[non_exhaustive]` from `config::Scalar` (in `cli/config`) so the `Node ↔ Yaml`
`Scalar` arms are wildcard-free — a safe change on a `publish = false` crate.

```rust
fn to_node(value: document::Yaml) -> Node { /* explicit per-variant arms */ }
fn to_yaml(node: &Node) -> document::Yaml { /* explicit per-variant arms */ }

pub fn parse(content: &str) -> Result<Node, String> {
    document::parse(content).map(to_node).map_err(|e| e.to_string())
}

pub fn render(existing: Option<&str>, document_node: &Node) -> Result<String, String> {
    document::render(existing, &to_yaml(document_node)).map_err(|e| e.to_string())
}
```

**File**: `cli/config-adapters/src/frontmatter.rs` — deleted; `split` now lives in
`document`. Update the one internal caller (`document.rs`) accordingly.

**File**: `cli/config-adapters/Cargo.toml`
**Changes**: add `document = { path = "../document" }`; remove
`serde-saphyr`; remove `serde` if no remaining use.

#### 3. Move the enforcement

**File**: `cli/Cargo.toml` — add `document` to `members`. Final members ordering
keeps each domain crate before its adapters, shared utilities grouped:
`["launcher", "kernel", "verify", "document", "config", "config-adapters",
"corpus", "corpus-adapters", "vcs", "vcs-adapters"]`.
**File**: `cli/deny.toml`
**Changes**: `{ crate = "serde-saphyr", wrappers = ["document"] }`.
**File**: `tests/integration/deny/fixtures/` + `test_serde_saphyr_ban.py`
**Changes**: rename the **clean** (permitted-wrapper) fixture package — its
`[package] name`, its `[[bin]]`, **and its `Cargo.lock` `[[package]] name`** — from
`config-adapters` to `document` so the positive control matches
`wrappers = ["document"]`. The `Cargo.lock` rename is load-bearing: the ban test
runs `cargo deny --frozen`, so a manifest-only rename makes `cargo metadata
--frozen` error before bans evaluate. Leave the banned fixture (named `config`, a
non-wrapper) unchanged. `cargo-deny` `wrappers` matches only *direct* dependents, so
the crate the clean fixture depends on must be the one now listed.

#### 4. Adversarial hardening (replaces the panic guard)

**File**: `cli/document/tests/adversarial.rs` and
`cli/document/tests/fixtures/document_fixture.rs`.

Port the 0178 bounded-time adversarial pattern
(`plan:2026-07-07-0178:785-793`): each adversarial parse runs under a worker-thread
`recv_timeout` (or the `document-fixture` subprocess with a wall-clock kill),
asserting no panic / no abort / no hang. Fixtures: deeply-nested YAML, an
**anchor/alias-expansion (billion-laughs) input** that actually exercises the
timeout guard, structurally-malformed YAML (characterise), **plus** the
visualiser's trailing-whitespace quoted-flow-scalar input (`frontmatter.rs:416-429`)
— which now **parses cleanly** (it is valid YAML; only libyml's memory-safety bug
made it crash, which `catch_unwind` masked as `Malformed`). The real guarantee is
bounded, no-panic/no-abort/no-hang; the deeply-nested / alias-bomb /
structurally-malformed inputs are bounded *rejections*.

Removing `catch_unwind` rests per-document fault isolation on serde-saphyr's
panic-freedom (a `catch_unwind` cannot catch a stack-overflow abort anyway). The
blast radius is nil until 0168 wires a corpus-*walking* consumer; before that
consumer ships, the 0168 hand-off requires **either** proven panic-freedom (fuzz
coverage) **or** a re-established per-file isolation boundary — a thread/subprocess
boundary that maps a single file's panic/abort to `Malformed` and continues the
walk — so one poison file cannot abort the scan. Not re-introduced speculatively in
0179.

### Success Criteria

#### Automated Verification

- [x] Workspace builds and lints: `mise run cli:check`
- [x] All cli unit tests pass, including existing `config`/`config-adapters`
      suites unchanged: `mise run test:unit:cli` (187 passed)
- [x] `document` is the only serde-saphyr wrapper and the ban still evaluates:
      `mise run deny:check` and `mise run test:integration:deny`
- [x] Adversarial fixtures (incl. the alias-expansion input) return bounded,
      no-panic/no-abort/no-hang outcomes — the trailing-whitespace input now parses
      cleanly, the others are bounded rejections:
      `cd cli && cargo test -p document adversarial`
- [x] Import rules hold: `mise run pup:check`
- [x] `document::render` returns `Err` and emits no output on a
      fence-valid-but-invalid-YAML existing input: `cd cli && cargo test -p document`
- [x] The relocated `split` tests are ported into `document` (not dropped), and
      `document::render` preserves an arbitrary body — CRLF, trailing-newline, and a
      body opening with a blank line — byte-for-byte: `cd cli && cargo test -p document`
- [x] End-to-end fail-closed pins: `config-adapters`'
      `a_write_against_a_malformed_file_fails_closed` (unterminated fence) stays green;
      a new store-level test seeds a fence-valid-but-invalid-YAML config and asserts
      `MalformedFrontmatter` with the target byte-identical (the re-parse branch); and
      an over-cap (`> MAX_SCAN`) config resolves as `MalformedFrontmatter` not an I/O
      error: `cd cli && cargo test -p config-adapters`

#### Manual Verification

- [x] `config-adapters/Cargo.toml` names no serde-saphyr; `document` is the sole
      importer.
- [x] A round-trip through the retrofitted `config-adapters` preserves an existing
      config file's body byte-for-byte (covered by `render_preserves_the_body_byte_for_byte`).

---

## Phase 2: `corpus` domain crate

### Overview

The `kernel`-only domain: the serde-free value model, `DocTypeKey` as the single
dir→type source **and its inference matcher**, the typed-linkage value parser **and
single-document resolver**, the work-item-ID predicate + a scanner port, the slug
conventions, a `Clock` port, and the artifact-metadata output type. The pure
convention algorithms live here; the adapter feeds them infra-sourced data. No
consumers yet, exactly as `config` began.

### Changes Required

#### 1. Crate + enforcement

**File**: `cli/corpus/Cargo.toml` — `[dependencies] kernel = { path = "../kernel" }`
only.
**File**: `cli/Cargo.toml` — add `corpus` to `members`.
**File**: `cli/pup.ron` — add a `corpus`-domain rule mirroring the `config` one:

```ron
Module((
  name: "corpus_domain_imports_only_permitted",
  matches: Module("^corpus($|::)"),
  rules: [ RestrictImports(
    allowed_only: Some([
      "^(std|core|alloc)(::|$)",
      "^kernel::Error(::|$)",
      "^crate(::|$)",
    ]),
    denied: None,
    severity: Error,
  ) ],
)),
```

Production (non-test) domain modules must use `crate::`-prefixed `use` paths to
satisfy the whole-crate `^crate(::|$)` rule (single-item `use crate::...;`, as
`config` writes them). Test modules are pup-exempt — cargo-pup does not analyse
`#[cfg(test)]` code — so they may use `super::`/grouped imports, as `config`'s own
tests do.

#### 2. Value model

**File**: `cli/corpus/src/value.rs`

```rust
pub enum FrontmatterValue {
    Scalar(Scalar),
    Sequence(Vec<FrontmatterValue>),
    Mapping(Mapping),
}

pub enum Scalar { String(String), Bool(bool), Int(i64), Float(f64), Null }

pub struct Mapping(Vec<(String, FrontmatterValue)>);
```

Mirror `config::Node`'s accessors (`get`, `entries`, `FromIterator`,
order-preserving). No serde. The big-int-as-`String` policy is already applied at
the `document::Yaml` boundary, so the adapter mapping in Phase 3 carries it
through without a domain tag variant. (Module named `value.rs`, not mirroring
`config`'s `node.rs`; the type name `FrontmatterValue` is domain-specific by design
and `value.rs` reads as the generic value home.)

#### 3. `DocTypeKey` (single source)

**File**: `cli/corpus/src/doc_type.rs` — extract the pure `DocTypeKey` (14
variants) and all `match`-based self-methods from `docs.rs:6-178`
(`config_path_key`, `label`, `wire_str`/`from_wire_str`, `in_lifecycle`,
`participates_in_lifecycle`, `carries_target_frontmatter`, `is_virtual`,
`in_kanban`, `nested_manifest_filename`, `all`). Leave the `Config`-coupled
`DocType`/`describe_types` (`docs.rs:180-222`) server-side. The kebab-case wire
form is preserved (`serde` is not available here, so `from_wire_str`/`wire_str`
provide the round-trip that `serde(rename_all = "kebab-case")` provided; the
wire-string table becomes the single source and Phase 3 asserts the migration
snapshot + awk table agree with it).

The **doc-type inference matcher** is a pure domain function in the same module:

```rust
pub fn infer(path: &Path, table: &[(DocTypeKey, PathBuf)]) -> Option<DocTypeKey>;
```

Longest-configured-dir-wins, segment-anchored (`doc-type-inference.sh:45-65`), over
an **injected** `type→dir` table — no regex, no config dependency, so it stays
`kernel`-only. `corpus-adapters` sources the table from config and calls it (Phase 3
§3). This mirrors `config` keeping its precedence resolution / path-walk domain-side
rather than in `config-adapters`.

#### 4. Typed-linkage value parser + document resolver

**File**: `cli/corpus/src/typed_ref.rs` — `typed_ref.rs:12-72` verbatim (sole dep
`std::path`). The *value* parser (`"plan:0042"` → `TypedRef`).

**File**: `cli/corpus/src/linkage.rs` — the single-document ADR-0034 *resolver*
(the pure core of bash `linkage-parser.sh`): given a document's `&FrontmatterValue`
and the qualifying H2 sections, extract its linkage keys, build the `<type>:<id>`
target refs via `typed_ref`, and classify each band (`resolved`/`ambiguous`) against
the fixed ADR-0034 type-pairs vocabulary (a domain constant, not the bash TSV file).
Pure over domain types — no I/O — so it belongs in the domain, not the adapter. The
*cross-document* cluster walker (store-coupled, `IndexEntry`-bound) stays
server-side and out of scope.

**Implementation timing (deviation)**: `linkage.rs` is a `corpus`-domain module,
but bash `linkage-parser.sh` is a ~300-line body-section prose extractor (keyword
detection, template-path blocklist, ADR-0038 two-band model, per-reference anchors)
whose correctness is only validated against the oracle in Phase 3. It is therefore
*implemented* in Phase 3 alongside its parity suite — its domain placement is
unchanged; only the build timing co-locates with the oracle. Phase 2 ships the
other six domain modules.

#### 5. Work-item-ID predicate + scanner port

**File**: `cli/corpus/src/work_item_id.rs`

```rust
pub struct WorkItemIdScheme {
    pub id_pattern: String,
    pub default_project_code: Option<String>,
}

impl WorkItemIdScheme {
    pub fn is_canonical_id_token(&self, token: &str) -> bool;
    pub fn canonical_digit_width(&self) -> usize;
    pub fn normalise_id(&self, raw: &str) -> Option<String>;
    pub fn extract_id(&self, filename: &str, scanner: &dyn IdScanner) -> Option<String>;
}

pub trait IdScanner {
    fn scan(&self, text: &str) -> Option<IdScan>;
}

pub struct IdScan { pub digits: String, pub match_end: usize }
```

Port `config.rs:129-229` (the pure predicates verbatim). `extract_id` uses the
injected `scanner` for the primary regex pass (digits from the capture) and the
existing pure bare-numeric fallback (`config.rs:186-193`). The `regex` type never
enters this crate.

#### 6. Slug conventions

**File**: `cli/corpus/src/slug.rs` — port `slug.rs` with the work-item arm taking a
`&dyn IdScanner`:

```rust
pub fn derive(
    kind: DocTypeKey,
    filename: &str,
    scheme: &WorkItemIdScheme,
    scanner: &dyn IdScanner,
) -> Option<String>;

pub fn derive_work_item(filename: &str, scanner: &dyn IdScanner) -> Option<String>;
```

Dated/ADR arms use `scheme.is_canonical_id_token` (pure). The work-item arm
composes the scanner path with the pure fallback exactly as the visualiser's
`build_entry` does (`indexer.rs:1346-1355`):
`derive_work_item(scanner).or_else(|| strip_prefix_work_item_id(stem))` — so a
legacy bare-numeric `0042-foo.md` under a `{project}-{number}` pattern (which the
scan regex rejects) still yields `foo` rather than `None`. `derive_work_item`
replaces `derive_work_item_with_regex` (`slug.rs:8-20`) with the scanner's
`match_end`. `humanise_slug`/`strip_humanise_prefix`/`title_case_segment` become
the **canonical** title-caser here.

#### 7. Consolidations

- **Width parser**: `WorkItemIdScheme::canonical_digit_width` is the canonical
  copy. The two former parsers disagree on the missing-width default —
  `number_width_from_id_pattern` returns `4` (`unwrap_or(4)`, `:1200`), the
  `WorkItemConfig` one returns `0` ("admit any digit count", `config.rs:148`). The
  merged function **keeps the `0`/"admit-any" default** that the runtime predicates
  `is_canonical_id_token`/`normalise_id` depend on, and unit cases pin both the
  missing-width and `{number:0d}` inputs. Because `number_width_from_id_pattern`'s
  call site is server-side, its retirement onto this canonical copy completes in
  0168 (like the title-casers) — 0179 lands and pins the canonical copy.
- **Title-caser**: `corpus::slug::title_case_segment` is canonical;
  `config::label_from_key` and `api::library::humanise_status` are byte-identical
  copies that stay server-side and retire onto the canonical copy in 0168 (the
  Success Criteria reflect this partial convergence).
- **Drift during the window**: because the server is not a `cli/` crate until 0168,
  no automated test can bind the server-side twins (the title-caser and
  `number_width_from_id_pattern`) to the canonical `corpus` copies in the 0179→0168
  window, so the copies could silently diverge. Tracked as a 0168 hand-off: the fold
  must add a conformance test binding the twins to the canonical copies (as the
  doc-type fact has) when it retires them.

#### 8. Clock port + artifact-metadata output type

**File**: `cli/corpus/src/metadata.rs`

```rust
pub trait Clock {
    fn now_utc_iso(&self) -> String;
    fn filename_timestamp(&self, format: FilenameTimestampFormat) -> String;
}

pub enum FilenameTimestampFormat { DateTimeUnderscored, DateOnly, CompactTime }

pub struct ArtifactMetadata {
    pub datetime_utc: String,
    pub filename_timestamp: String,
    pub repository_name: Option<String>,
    pub revision: Option<String>,
}
```

The three `FilenameTimestampFormat` variants correspond to the three helpers'
formats (`_H-M-S`, date-only, `-HMMSS`). Matching the bash oracle exactly:
`now_utc_iso` is **UTC** (`date -u`, literal `+00:00`), while `filename_timestamp`
is **host-local** (plain `date`) — the two zones differ and the port makes that
explicit. The `Clock` methods return rendered strings, but the rendering is
delegated to **pure functions** over an instant + resolved offset (unit-tested
directly, with a golden per format), so only acquiring the instant/offset is I/O;
the fallibility of offset resolution lives on `SystemClock`'s fallible constructor,
not the trait (Phase 5). `Clock` lives in `corpus` because artifact-metadata is its
only consumer; the decision is recorded: **if a second time-consumer appears (e.g.
0173's CLI or 0180's store), relocate the port to `kernel`** rather than have a
lower crate depend on `corpus`. The composition that fills `ArtifactMetadata` from a
`Clock` + VCS facts lands in Phase 5.

### Success Criteria

#### Automated Verification

- [x] `corpus` compiles `kernel`-only and imports no serde/serde_json/serde-saphyr
      symbol: `mise run cli:check` and `mise run deny:check`
- [x] The new `corpus` pup rule passes and forbids adapter imports:
      `mise run pup:check` and `mise run test:integration:pup`
- [x] Ported unit tables pass (value model, `DocTypeKey` + doc-type inference
      matcher, typed-ref, work-item predicate, slug): `mise run test:unit:cli`
      (219 passed). The single-doc **linkage resolver** is deferred to Phase 3 with
      its parity oracle (see §4).
- [x] A single canonical `canonical_digit_width` exists in `corpus`, with unit
      cases pinning the missing-width and `{number:0d}` defaults (the server-side
      twin retires in 0168); one canonical title-caser in `corpus` (the two
      server-side copies retire in 0168).

#### Manual Verification

- [x] `corpus` names no `regex` dependency; `extract_id`/`derive_work_item` take a
      scanner.
- [x] `DocTypeKey` wire round-trip (`wire_str`/`from_wire_str`) covers all 14
      variants including `Templates` (virtual, no config key).

---

## Phase 3: `corpus-adapters` — parse and conventions

### Overview

The outbound infra + imperative shell over `corpus` + `document`: translate
`document::Yaml → FrontmatterValue` (with `FrontmatterState`), supply the
regex-backed `IdScanner` and the config-sourced doc-type table, and run the
per-document **assembler** that reads a file → parses → **invokes the `corpus`
conventions** (doc-type inference, linkage resolution, slug derivation). Plus the
frontmatter write-convention. The convention *algorithms* live in `corpus`
(Phase 2); this crate owns only the infra boundary and the orchestration. Parity is
proven against the bash sources.

### Changes Required

#### 1. Crate

**File**: `cli/Cargo.toml` — add `regex = "1"` to `[workspace.dependencies]`
(single source of truth; a caret `"1"` is defensible given regex's stability — the
exact-pin discipline that fences behaviour-sensitive crates like serde-saphyr is not
warranted here) and add `corpus-adapters` to `members`.

**File**: `cli/corpus-adapters/Cargo.toml`

```toml
[dependencies]
corpus = { path = "../corpus" }
document = { path = "../document" }
regex = { workspace = true }
```

(No `serde` dependency: the JSON wire `Serialize` is deferred to 0168/0173 — see
§2 — so nothing in `corpus-adapters` serialises in 0179.)

#### 2. Parse + `FrontmatterState`

**File**: `cli/corpus-adapters/src/document.rs` — map `document::Yaml →
FrontmatterValue` (explicit per-variant arms; nested↔nested, mechanical).
`FrontmatterState` (Parsed/Absent/Malformed, `frontmatter.rs:72-87`) is reproduced
over `document::fence_offsets` + `document::parse` with the visualiser's exact
root-shape rule, **arm-ordered so `Null` (which is a `Scalar` variant) is caught
first**: a **`Null` or empty** root → `Parsed` with an empty mapping; a
**non-`Null` `Scalar` or a `Sequence`** root → `Malformed`; a `document::parse`
error → `Malformed`.

**YAML tags — one unambiguous rule, matching the visualiser's fail-closed
behaviour**: any explicit YAML tag in frontmatter → `Malformed` (the visualiser
returns `None` for any `serde_yml::Value::Tagged`, `frontmatter.rs:233`). A spike in
Phase 3 characterises serde-saphyr's `deserialize_any` tag handling; the expectation
is that a `deserialize_any` visitor with no tag path errors. **If serde-saphyr
instead resolves a tag class to a base value, the guard is a *structural* one at the
parse-model boundary — reject when a tag is encountered during deserialization —
not a raw-text token scan** (which would false-positive on a quoted string like
`note: "see !!important"` and miss a tag on a nested value). Fixtures pin a custom
tag (`!custom`), a standard tag (`!!str`), a nested-value tag, a root-sequence, and
a bare-scalar all as `Malformed`, plus the quoted-tag-substring case as `Parsed`.

The hand-written JSON `Serialize` for the SPA/API wire boundary is **deferred to
0168/0173** — no consumer in 0179 serialises `FrontmatterValue` (the SPA lives in
the untouched server). When it lands it must be a conscious contract decision: the
order-preserving `Vec<(String, _)>` model and the big-int-as-`String` policy
**diverge** from the shipped SPA shape (`BTreeMap`, sorted keys, numeric
big-ints), so 0168 either preserves the old shape or accepts the change
deliberately. Recorded here as a 0168 hand-off.

#### 3. Doc-type table sourcing

**File**: `cli/corpus-adapters/src/doc_type.rs` — build the `type→dir` table from
config's `doc_paths` (keyed to `corpus::DocTypeKey`) and pass it to the pure
`corpus::doc_type::infer` matcher (Phase 2 §3). The matcher is domain; the adapter
only sources the table and calls it.

#### 4. Scanner + assembler (invoking the domain conventions)

**File**: `cli/corpus-adapters/src/scanner.rs`, `.../assemble.rs`

- `RegexScanner { re: regex::Regex }` implements `corpus::IdScanner`, built from an
  injected compiled scan regex. `IdScan.match_end` is the **full-match end**
  (delimiter consumed), not the digit capture-group end; a direct assertion pins
  this for a `{project}-{number:04d}` input so the slug tail can't silently gain a
  leading delimiter.
- The per-document assembler reads a file, parses it (§2), then **invokes the
  `corpus` conventions**: `corpus::doc_type::infer`, `corpus::slug::derive` (with the
  `RegexScanner`), and `corpus::linkage` over the parsed `FrontmatterValue`. It
  *orchestrates*; it holds none of the convention logic itself.
- The bash-parity harness re-paths: the shell-out to `work-item-pattern.sh`
  (`slug.rs:572-602`) moves from `../../../../skills/...` to
  `../../skills/work/scripts/work-item-pattern.sh` (relative to
  `cli/corpus-adapters`). The harness **asserts the script exists and is
  executable and hard-fails** if not (Rust's test harness has no skip primitive —
  a silent early-return registers as a green PASS), turning the "script moved"
  check into an automated assertion rather than a manual one.

#### 5. Frontmatter write-convention

**File**: `cli/corpus-adapters/src/patcher.rs` — `patcher.rs:8-260` over
`document::fence_offsets` (quote-style, inline-comment, CRLF, and body all
preserved; unsupported value shapes rejected). The patcher does surgical line
replacement over the fence offsets **without** re-parsing the whole frontmatter, so
its fail-closed envelope covers only a malformed fence and unsupported value shapes —
narrower than `document::render`'s full re-parse. It is not wired to any writer in
0179 (no CLI; 0173). 0168/0173 hand-off: any live patcher writer must pair it with a
store-style atomic write and accept that it validates fence + value-shape only, not
whole-frontmatter YAML validity. **Placement note**: the patcher is pure over bytes
(no I/O) and is about the markdown-frontmatter *format*, not corpus semantics, so it
arguably belongs in the `document` crate rather than `corpus-adapters`; kept here for
now (its only near-term consumer is corpus-side status transitions), revisit when a
second format-level mutator appears.

#### 6. Doc-type single-sourcing (AC-6)

**File**: `cli/corpus-adapters/tests/doc_type_single_source.rs` — because the 0007
migration snapshot re-serialises a config-injected table at runtime and
`0007-frontmatter-rewrite.awk`'s `path_to_typed` is a *matcher* (not a static
table), the test does not scrape source text. It **executes** each surface against
a known path set (the migration's table-emit step; the awk matcher over one path
per `DocTypeKey` dir) and asserts the resulting dir→type mapping equals the one
derived from the crate's `DocTypeKey`. It asserts the extracted set is **non-empty
and covers every non-virtual variant**, so an empty or partial read fails rather
than passing vacuously. Like the parity suite, it **asserts `bash`/`awk` and the
migration script exist and are executable and hard-fails** with a tool-naming
diagnostic when absent, rather than surfacing an absent tool as a confusing mapping
mismatch.

#### 7. Parity suite

**File**: `cli/corpus-adapters/tests/parity.rs` + a shared fixture corpus under
`cli/corpus-adapters/tests/fixtures/`. The corpus splits into two subsets, since
bash emits only **13** rows:

- **Diff-tested (13 types)**: crate output compared against live
  `doc-type-inference.sh`, `linkage-parser.sh`, and `work-item-pattern.sh` over the
  three identity schemes (ADR-`N`, bare-`N`, date-prefixed) and the
  optional-embedded-work-item-id cases — including the pinned edges
  (`2026-04-17-100-day-plan.md`, `2026-05-31-0040.md`, `ADR-0001.md`, and a
  bare-numeric filename under a `{project}` pattern exercising the slug `or_else`
  fallback). The corpus **must include a path nested under a configured doc-type
  directory that is itself a prefix of another** (plus an exact-length-tie case), so
  the matcher's longest-configured-dir-wins / first-entry-tie logic
  (`doc-type-inference.sh:40-64`) is exercised on both sides rather than passing
  vacuously. Each external-tool call asserts tool/script presence and hard-fails when
  absent (no silent skip).
- **Declared-value (no bash oracle)**: `Templates` (virtual → slug `None`), the
  `PrDescriptions` wire/config-key mismatch (`pr-descriptions` vs `prs`), and the
  **review-suffix slug edges** — trailing `-review-N` stripped, internal `-review-`
  preserved, non-numeric suffix → `None` (`work-item-pattern.sh` has no oracle for
  these) — asserted directly against expected literals.

Fold the trailing-whitespace quoted-flow-scalar input into the malformed fixtures.

**Toolchain gating**: the differential/parity, single-source, and detection suites
live behind a `bash-parity` cargo feature (enabled in CI). With the feature on, an
absent `bash`/`awk`/`jj`/`git` **hard-fails** (no silent skip); with it off, only the
pure-Rust unit tables run, so `cargo test` stays runnable on a bare machine.

**Ported-assertion note**: porting the visualiser's test tables is *not* verbatim
where the rewrite changes behaviour — the number-widening cases become
String-preservation assertions; the `Tagged` cases become `Malformed` assertions
(per the YAML-tag rule); and the trailing-whitespace quoted-scalar case, which the
visualiser reported `Malformed` only because libyml *crashed*, now asserts a **clean
parse** (it is valid YAML — confirmed against serde-saphyr in Phase 1). Those are
deliberate rewrites, called out so the oracle stays honest.

**Design-inventory id divergence (found by AC-6, deviation)**: the single-source
suite surfaced a *pre-existing* disagreement between the two bash surfaces.
`linkage-parser.sh` derives a design-inventory id from the **parent directory**
(its comment: "not the manifest basename `inventory`"), but the 0007 rewrite awk's
`path_to_typed` has no nested-manifest arm and falls through to the whole-stem
default, yielding `design-inventory:inventory`. The crate follows
`linkage-parser.sh`, which is what the parity suite pins it to. AC-6 asks for the
**dir→type** mapping, and the awk agrees with the crate on that for all 13
directories; the id-derivation test therefore covers the three arms the awk does
encode (work-item, ADR, whole-stem default) and names the design-inventory
exclusion. Fixing the awk is a migration-side change, out of scope here.

### Success Criteria

#### Automated Verification

- [x] `corpus-adapters` reaches serde-saphyr only through `document`:
      `mise run deny:check`
- [x] Import rules hold: `mise run pup:check`
- [x] Parse/infer/linkage/slug/patcher suites pass: `mise run test:unit:cli`
- [x] The single-source test passes (migration snapshot + awk derive from
      `DocTypeKey`): `cd cli && cargo test -p corpus-adapters doc_type_single_source`
- [x] The parity suite passes against the live bash scripts:
      `cd cli && cargo test -p corpus-adapters parity`
- [x] The bash parity suites are unchanged and still green:
      `bash skills/work/scripts/test-work-item-pattern.sh`,
      `bash scripts/test-linkage-parser.sh`

#### Manual Verification

- [x] The re-pathed harness locates `work-item-pattern.sh` from the new crate
      location (fails loudly, not silently, if the script moves).
- [x] A `status:` patch preserves quote style, inline comment, and CRLF against a
      hand-built fixture.

---

## Phase 4: `vcs` + `vcs-adapters` pair

### Overview

A dedicated domain+adapters pair for the cross-cutting VCS/repo probe, at the
helpers' contract level (repo-root, VCS-kind, revision, repo-name) — not the
seven-kind `classify_checkout` taxonomy. Independent of the parse/convention
crates; `kernel`-only domain.

### Design decision (records the in-scope VCS spike)

Per the "helpers' contract only" scope, each concern uses the technique its bash
oracle uses:

- **Repo-root**: filesystem **marker-walk** — walk ancestors for the first `.jj`
  or `.git` (bash `find_repo_root`, `vcs-common.sh:8-18`). This is **not**
  identical to `config-adapters::discover_root`, which also stops at `.accelerator`
  and returns `start` when no marker is found; `RepoRoot::discover` returns `None`
  when no `.jj`/`.git` marker exists. (A jj-secondary workspace roots at its own
  `.jj`, matching the helpers.)
- **VCS-kind**: **marker inspection** — `.jj` present wins over `.git` (jj-idiom,
  including colocated), else `.git` → git, else none (bash `vcs_mode`,
  `vcs-common.sh:27-36`). Markers are tested by **existence** (`-e`), matching bash,
  so a `.git`-*file* worktree/submodule is recognised (not only a `.git`
  directory).
- **Revision**: **command-probe** returning the full working-copy revision
  (`jj log -r @ --no-graph -T commit_id` / `git rev-parse HEAD`). A missing binary,
  a non-zero exit (incl. a no-commit *git* repo), or empty stdout all map to
  **`None`** — never `Some("")` or `Some(stderr)`. The probe runs with a sanitised
  environment and deterministic-output flags (see §3). Genuinely-bare / non-VCS →
  `None` matches the fall-through of `artifact-derive-metadata.sh:8-20`.
- **Repo-name**: `basename` of the repo-root.

The full `classify_checkout` taxonomy (secondary/colocated/worktree/bare/`GIT_DIR`)
stays out of scope for 0169 to add; 0179's marker-walk deliberately inherits the
helpers' behaviour, and the `.git`-as-file case is covered by existence-testing the
marker rather than classifying the checkout. **Convergence ledger**:
`config-adapters::discover_root` (which additionally stops at `.accelerator` and
returns `start`) is a second marker-walk left distinct in 0179; the 0168 hand-off is
to fold it onto a parameterised `vcs` marker-walk (the `.accelerator` stop and
start-fallback expressed as options) or record it as a deliberate permanent fork.

The richer `classify_checkout` (secondary/colocated/worktree taxonomy,
`vcs-common.sh:177-280`) is explicitly out of scope; fixtures assert only correct
root/kind/revision resolution for the three divergent shapes.

### Changes Required

#### 1. Crates + enforcement

**File**: `cli/vcs/Cargo.toml` — `kernel`-only.
**File**: `cli/vcs-adapters/Cargo.toml` — `vcs = { path = "../vcs" }` +
`tracing = { workspace = true }` (for the probe `warn` logs; command-probe via
`std::process`, marker-walk via `std::fs`).
**File**: `cli/Cargo.toml` — add `vcs`, `vcs-adapters` to `members`.
**File**: `cli/pup.ron` — add a `vcs`-domain kernel-only rule (mirror `corpus`).

The `vcs`/`vcs-adapters` types are small enough (one enum, one struct, two traits,
two impls) to live in each crate's `lib.rs`; splitting into `facts`/`ports` modules
is optional and noted rather than required.

#### 2. Domain

**File**: `cli/vcs/src/lib.rs`

```rust
pub enum VcsKind { Jj, Git, None }

pub struct RepoFacts {
    pub root: std::path::PathBuf,
    pub name: String,
    pub kind: VcsKind,
    pub revision: Option<String>,
}

pub trait RepoRoot {
    fn discover(&self, start: &std::path::Path) -> Option<std::path::PathBuf>;
}
pub trait VcsProbe {
    fn kind(&self, root: &std::path::Path) -> VcsKind;
    fn revision(&self, root: &std::path::Path, kind: VcsKind) -> Option<String>;
}
```

`facts(start) -> Option<RepoFacts>` returns `None` when `discover` finds no
`.jj`/`.git` marker (bare/non-VCS), so the no-repo state is representable rather
than fabricated as an empty `PathBuf`/`String`. Phase 5 maps `None` → an all-blank
`ArtifactMetadata`.

#### 3. Adapters

**File**: `cli/vcs-adapters/src/lib.rs` — `MarkerWalkRoot` implements `RepoRoot`
(the ancestor walk, `.jj`/`.git` by existence); `CommandProbe` implements
`VcsProbe` (marker inspection for kind; the jj/git revision command-probe). A
`facts(start)` free function composes them into `Option<RepoFacts>`.

`CommandProbe` runs each subprocess with **sanitised environment and
deterministic-output flags**, matching the bash oracle's hygiene
(`test-metadata-helpers.sh:34-36`, `vcs-common.sh:130-135`): scrub `GIT_DIR` /
`GIT_WORK_TREE` / `JJ_CONFIG` (and neutralise `GIT_CONFIG_*`); pass
`git -c color.ui=false rev-parse HEAD` and `jj --color=never --no-pager log ...` so
ambient user config can't inject ANSI or redirect the root. Each probe carries a
wall-clock cap with **headroom for a legitimately slow / lock-contended repo**
(reusing the adversarial subprocess-kill mechanism) so it can't hang metadata
derivation. Every failure mode — spawn failure, non-zero exit, empty stdout, **and a
wall-clock timeout (killed probe)** — maps to `revision: None` and is **`warn`-logged
via `tracing` against the `kernel::logging`-installed subscriber**, so a genuine
failure leaves a trace and is not silently indistinguishable from a legitimately bare
repo. (A `jj` repo always has a working-copy commit at `@`, so a fresh/no-commit
*jj* repo legitimately returns a revision; the no-commit → `None` case is a *git*
repo with no commits, where `git rev-parse HEAD` exits non-zero.)

#### 4. Fixtures

**File**: `cli/vcs-adapters/tests/detection.rs` — build temp repos and assert:
secondary jj workspace (root at the workspace `.jj`, kind `Jj`), colocated (kind
`Jj`, both markers present), a `.git`-*file* worktree (kind `Git`, recognised via
existence-test), a no-commit **git** repo (revision `None`, not `Some("")`), and a
bare repo (`facts → None`). At least one fixture asserts **`RepoFacts.name ==
basename(root)`** (a temp repo under a known directory name), since a wrong
path-component slip on a `PathBuf` would otherwise go uncaught. Tests that require a
real `jj`/`git` binary assert its presence and **hard-fail when absent** (Rust has no
skip primitive — an early return registers as a green PASS), mirroring the bash probe
rather than the metadata-helper's `skip_test`. A **fake-probe unit test** drives the
timeout branch (a probe that exceeds the wall-clock cap) and asserts it resolves to
`revision: None` with a warn log, without depending on a real slow repo.

### Success Criteria

#### Automated Verification

- [x] `vcs` compiles `kernel`-only; `vcs-adapters` depends only on `vcs` + std +
      `tracing`: `mise run cli:check`
- [x] Import rules hold for the new `vcs` rule: `mise run pup:check`
- [x] Detection fixtures pass for jj-secondary, colocated, and bare shapes:
      `cd cli && cargo test -p vcs-adapters detection`
- [x] Deny clean: `mise run deny:check`

#### Manual Verification

- [x] Root resolution matches `config-adapters::discover_root` **on an input where
      a `.jj`/`.git` marker exists with no shallower `.accelerator`** (the two
      diverge on `.accelerator`-only and marker-less trees by design).
- [x] Revision is the full working-copy id in both a jj-colocated and a plain git
      temp repo, and blanks (not errors) on a no-commit and a bare repo.

**Warn-log assertion (deviation)**: the failure branches (spawn failure, non-zero
exit, empty stdout, killed-at-cap) are each `warn`-logged as specified and each is
unit-tested to resolve to `revision: None` — but the tests assert the `None`, not
the presence of the log line. Capturing `tracing` output would need a custom
subscriber and a `tracing-subscriber` dev-dependency in `vcs-adapters`; the log is
an operator aid rather than behaviour a consumer branches on, so the assertion was
left at the return value.

**Root-equivalence check (method)**: verified by construction and by the two suites
asserting the same shape from each side — `vcs-adapters`'s
`the_walk_finds_the_root_from_a_nested_directory` and `config-adapters`'s
`discover_roots_at_a_jj_only_checkout` both pin a nested start to the marker
directory. The crates are not coupled to cross-check each other directly. The two
walks differ only where the plan says they should: the `.accelerator` stop, the
start-fallback, and whether the filesystem root itself is tested.

---

## Phase 5: `corpus-adapters` — artifact-metadata derivation

### Overview

The authoring composition: fill `corpus::ArtifactMetadata` from a `Clock` port + a
`vcs-adapters` probe, subsuming all three metadata helpers, with every field
asserted deterministically behind faked ports.

### Changes Required

#### 1. Dependency

**File**: `cli/Cargo.toml` — extend the workspace `time` pin to
`features = ["parsing", "formatting", "local-offset"]` (`formatting` renders the
timestamps; `local-offset` resolves the host offset for the filename timestamp; the
existing `parsing` feature stays).
**File**: `cli/corpus-adapters/Cargo.toml` — add
`vcs-adapters = { path = "../vcs-adapters" }` and `time = { workspace = true }`,
used by `SystemClock` so the produced binary stays self-contained (no shell-out to
`date`).

#### 2. Clock impl + composition

**File**: `cli/corpus-adapters/src/metadata.rs`

```rust
pub struct SystemClock { local_offset: UtcOffset }

impl SystemClock {
    pub fn try_new() -> Result<Self, ClockError>;
}
impl corpus::Clock for SystemClock { /* pure format fns over instant + local_offset */ }

pub fn derive(
    clock: &dyn corpus::Clock,
    facts: Option<&vcs::RepoFacts>,
    format: corpus::FilenameTimestampFormat,
) -> corpus::ArtifactMetadata;
```

`SystemClock` renders `now_utc_iso` in **UTC** (literal `+00:00`, not `%Z`) and
`filename_timestamp` in **host-local** time — matching bash's `date -u` vs plain
`date` — via **pure format functions over an instant + resolved offset**, so the
formatting is unit-tested without a clock (see §3). The host offset is the only I/O:
because `time`'s `now_local()` refuses to resolve in a multithreaded process (how
`cargo test`, CI, and 0168's tokio server all run), `SystemClock::try_new()` resolves
it **once via a short-lived single-threaded subprocess** — robust regardless of the
caller's threading, unlike a global "resolve before any thread spawns" assumption —
and caches it on the struct (no process-global static). If the offset cannot resolve
(no tzdata, no `TZ`), `try_new()` **errors rather than silently producing a
wrong-zone provenance timestamp** — a deliberate, documented divergence from bash's
silent degrade; `tzdata`/`TZ` is recorded as a runtime prerequisite for the
0168/0173 consumers. `derive` composes the two timestamps with `facts.map(|f|
&f.name)` / `facts.and_then(|f| f.revision)`; **`facts == None` (bare/no-VCS) yields
blank `repository_name`/`revision`**, and a transient probe failure blanks the same
fields — so the 0168/0173 hand-off requires the consuming writer to **surface (not
swallow) the probe `warn`** rather than persist silently-blank provenance. A
`derive_at(start_path, format) -> Result<ArtifactMetadata, _>` convenience wires
`SystemClock::try_new()` + `vcs_adapters::facts(start_path)`.

#### 3. Deterministic tests + parity

**File**: `cli/corpus-adapters/tests/metadata.rs` — a `FakeClock` and fake
`RepoFacts` assert each field exactly (including `facts == None` → blank name and
revision); a parity test asserts the rendered block satisfies the same contract as
`scripts/test-metadata-helpers.sh` (Current Revision present + non-empty; Current
Date/Time ISO `+00:00`; no legacy `Current Git Commit Hash:` / `Current Branch
Name:` labels; no `%Z` timestamp) for each of the three `FilenameTimestampFormat`
variants. **Golden per format**: each rendering is pinned by an exact-string golden
(the pure format function over a fixed instant + offset) against its bash helper's
literal shape (`%Y-%m-%d_%H-%M-%S` / date-only / `-HMMSS`) — the one field that
distinguishes the three subsumed helpers and that the reused contract test does not
inspect. A **controlled-`TZ` assertion** (a fixed non-UTC `TZ`, run single-threaded
or via the subprocess-kill harness to sidestep `time`'s multithread offset
limitation) pins that the filename timestamp is host-local and genuinely differs
from the UTC ISO line — deterministic even under a `TZ=UTC` CI, where an
ambient-host assertion would be vacuous. The test also asserts the offset resolves
(does not degrade to UTC).

### Success Criteria

#### Automated Verification

- [ ] Every derived field is asserted deterministically behind faked clock + VCS
      ports: `cd cli && cargo test -p corpus-adapters metadata`
- [ ] Each of the three filename formats is pinned by an exact-string golden and
      satisfies the metadata output contract.
- [ ] Full workspace green: `mise run cli:check`, `mise run test:unit:cli`,
      `mise run deny:check`, `mise run pup:check`
- [ ] The bash metadata-helper contract test is unchanged and still green:
      `mise run test:unit:templates`

#### Manual Verification

- [ ] `derive_at` in a real jj-colocated checkout produces a block matching the
      live `artifact-derive-metadata.sh` output shape (revision, repo name, ISO
      timestamp).
- [ ] A bare-repo invocation blanks revision/name rather than erroring, matching
      the helpers.

---

## Testing Strategy

### Unit Tests

- Port the visualiser's own test tables as the starting oracle (`typed_ref`,
  `slug`, `docs`/`DocTypeKey`, `WorkItemConfig`, `patcher`, `frontmatter`) into the
  new crates and hold the rewrite to them — *except* the cases the rewrite
  deliberately changes (number-widening → String-preservation; `Tagged` →
  `Malformed`; trailing-whitespace → clean parse, since serde-saphyr accepts the
  valid YAML libyml crashed on), which are rewritten rather than copied (see
  Phase 3 §7).
- Value-model round-trip: `document::Yaml` → `FrontmatterValue` → `Node` and back,
  asserting equality per variant and the three number boundaries (`i64`-range →
  `Int`; beyond-`i64`-within-`u64` → `String`; beyond-`u64` → `Float`).
- Consolidation: the canonical `canonical_digit_width` in `corpus` is unit-tested
  for the missing-width and `{number:0d}` defaults; the canonical title-caser
  covers `humanise_slug`'s pinned cases. (Retiring the server-side twins is 0168.)

### Integration / Parity Tests

- `corpus-adapters` output vs `doc-type-inference.sh`, `linkage-parser.sh`, and
  `work-item-pattern.sh` over the shared fixture corpus — the 13 bash-emitted types
  diff-tested, `Templates`/`PrDescriptions` asserted as declared values (3 identity
  schemes, embedded-id edges). These differential suites shell to bash/awk/jj/git and
  build temp repos, so they live in cargo `tests/` (integration-flavoured, run under
  `test:unit:cli` via nextest) and assert tool presence + hard-fail when absent.
- Doc-type single-source: the bash doc-type registry and the rewrite awk matcher are
  executed and asserted to agree with `DocTypeKey` on dir→type, and on id derivation
  for the arms the awk encodes (see the design-inventory divergence in Phase 3).
- Adversarial frontmatter under the bounded-time guard (no panic/abort/hang),
  including an anchor/alias-expansion (billion-laughs) input that exercises the
  guard and the trailing-whitespace regression.
- Enforcement regressions: `test:integration:deny` (serde-saphyr wrapper now
  `document`) and `test:integration:pup` (new `corpus`/`vcs` rules).

### Manual Testing Steps

1. Run `mise run check` from a clean tree; confirm green end-to-end.
2. From a jj-secondary workspace and from a plain git clone, drive `derive_at` and
   compare against the live `artifact-derive-metadata.sh`.
3. Grep the workspace to confirm one `canonical_digit_width` and one canonical
   title-caser in `corpus`.

## Performance Considerations

None material — pure string/parse logic, no hot paths. The 1 MiB `MAX_SCAN` fence
cap and serde-saphyr's built-in adversarial budgets bound worst-case parse cost;
the bounded-time test guard converts any regression into a deterministic failure.

## Migration Notes

The `config-adapters` retrofit (Phase 1) is the only change to shipped 0178 code;
its safety net is the existing `config`/`config-adapters` test suite staying green
across the swap, plus `document`'s own fail-closed and byte-for-byte render tests.
The one intentional behavioural change is the 1 MiB `MAX_SCAN` ceiling the shared
`split` inherits (a negligible-risk DoS bound; over-cap configs fail closed). The
SPA/API JSON `Serialize` divergence (order-preserving keys, big-int-as-`String` vs
the shipped `BTreeMap`) is deferred with the wire boundary to 0168 as a conscious
contract decision. No data migration. The 0166 epic is already updated to reflect
`vcs`/`vcs-adapters` alongside `document`.

## References

- Original work item: `meta/work/0179-corpus-crates-parsing-conventions.md`
- Research: `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`
- Parent epic: `meta/work/0166-shared-config-corpus-store-crates.md`
- 0178 pattern + adversarial guard:
  `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md:544-556,785-793`
- Value model: `cli/config/src/node.rs:4-24`
- Serde boundary: `cli/config-adapters/src/document.rs:115-211`
- Enforcement: `cli/deny.toml:64-69`, `cli/pup.ron:42-56`, `cli/Cargo.toml:42`
- Extraction sources: `skills/visualisation/visualise/server/src/{typed_ref,frontmatter,slug,config,docs,patcher}.rs`
- VCS oracle: `scripts/vcs-common.sh:8-36,177-280`, `scripts/artifact-derive-metadata.sh`
- Metadata parity: `scripts/test-metadata-helpers.sh`
- Conventions: ADR-0034 (typed-linkage), ADR-0045 (bash/Rust duplication), ADR-0053 (hexagonal)
