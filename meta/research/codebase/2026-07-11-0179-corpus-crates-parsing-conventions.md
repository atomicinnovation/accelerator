---
type: codebase-research
id: "2026-07-11-0179-corpus-crates-parsing-conventions"
title: "Research: corpus and corpus-adapters Crates for Parsing and Conventions"
date: "2026-07-11T11:27:14+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0179"
parent: "work-item:0179"
topic: "corpus and corpus-adapters crate extraction — parsing, conventions, and artifact-metadata"
tags: [research, codebase, corpus, config-crates, frontmatter, serde-saphyr, doc-type, typed-linkage, slug, work-item-id, vcs-detection, cargo-pup]
revision: "aa8adbc7216fad416d6d3531cfc74026c7b4c685"
repository: "accelerator"
last_updated: "2026-07-11T11:27:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: corpus and corpus-adapters Crates for Parsing and Conventions

**Date**: 2026-07-11 11:27 UTC
**Author**: Toby Clemson
**Git Commit**: aa8adbc7216fad416d6d3531cfc74026c7b4c685
**Branch**: (anonymous change atop `deeedf2c`, not pushed)
**Repository**: accelerator

## Research Question

Support planning for work item **0179 — corpus and corpus-adapters Crates for
Parsing and Conventions**. The work item is a dense, self-referential spec built
on `file:line` claims about the visualiser server, the 0178 config crates, the
bash convention library, and the artifact-metadata scripts. This research
verifies those claims against the live codebase and surfaces the concrete
structural detail the plan needs: what extracts cleanly, where the coupling
boundaries actually fall, and what the parity baselines and enforcement patterns
are.

## Summary

The work item's claims are **accurate almost everywhere** — the extraction
targets, the entanglement grades, and the 0178 pattern all hold up. Six
corrections/clarifications matter for planning:

1. **`config-adapters` has no `infra` dependency.** Its deps are `config` +
   `serde` + `serde-saphyr` only; there is no `infra` crate in the workspace.
   The 0179 requirement text says "corpus-adapters — deps: corpus + shared
   document-format crate + infra"; there is no infra crate to depend on.
2. **The parse leaves are pure but not independent leaves.** `frontmatter.rs`
   depends on `slug::humanise_slug` and `typed_ref`; `slug.rs` depends on
   `WorkItemConfig` + `DocTypeKey`; `patcher.rs` depends on `frontmatter`. They
   form one connected cluster that must move together — only `typed_ref.rs` is a
   truly standalone leaf.
3. **The "parse-time core" of `IndexEntry` is 10 fields, not 8** — `rel_path`
   and `body_preview` were omitted from the work item's enumeration but are
   purely derived in `build_entry`. The genuinely store-mutated fields are only
   three: `completeness`, `linked_count`, `cluster_key`.
4. **The doc-type fact is single-sourced already** — the dir→type default table
   has exactly one authoring site (`config-defaults.sh`). What is "triplicated"
   is the *matching algorithm* (three runtime surfaces re-implement
   longest-configured-dir-wins over the same injected data), not the data. This
   reframes AC-6.
5. **"14 doc types" = 13 content types + `Templates`** (virtual, no config path
   key). The bash registry emits 13 rows; the Rust `DocTypeKey` enum adds
   `Templates`. Any shared fixture corpus must reconcile these two vocabularies.
6. **The VCS story has three techniques, not two.** The metadata scripts use a
   **command-probe** returning the *full* working-copy revision; `vcs-common.sh`
   uses a **marker-walk** (with a richer command-probe classifier layer on top);
   the 0007 migration uses a **hybrid** (marker-gate + file-scoped command-probe
   returning the *short* id). They diverge on secondary jj workspaces, bare
   repos, and git-worktree/`.git`-as-file cases.

0166 **has** been updated for the fifth (document-format) crate, confirmed in
three places. serde-saphyr is proven, pinned `=0.0.29`, and fenced by a
cargo-deny `wrappers` ban plus a real-config regression test — a known-good
pattern to mirror, not a bet.

## Detailed Findings

### 1. Visualiser parse/convention leaves (extraction targets)

All under `skills/visualisation/visualise/server/src/`. **None** of the seven
target files imports `tokio`, `Arc`, `RwLock`, `axum`, or `IndexEntry` in
production code. The real extraction coupling is entirely internal:
`WorkItemConfig`, `Config`, and `DocTypeKey`.

Purity ranking (verified line numbers; the work item's numbers had drifted in a
few spots, corrected here):

| File / item | Purity | Blocking deps |
|---|---|---|
| `typed_ref.rs` (whole, 151 lines) | **Fully pure leaf** | only `std::path` |
| `patcher.rs` (whole, 448 lines) | Pure | `frontmatter::fence_offsets` + `FenceError` |
| `frontmatter.rs`: `fence_offsets`, `parse`, `yml_to_json`, `body_preview_from` | Pure | serde_yml / serde_json only |
| `frontmatter.rs`: `title_from`, `read_ref_keys` | Pure logic | `slug::humanise_slug`, `typed_ref` |
| `docs.rs`: `DocTypeKey` + methods | Pure leaf | serde + `PathBuf` |
| `config.rs`: `WorkItemConfig` + `RawWorkItemConfig` | Pure convention engine | `regex`, `ConfigError::InvalidScanRegex` |
| `slug.rs` (whole, 651 lines) | Pure logic | `WorkItemConfig`, `DocTypeKey` |
| `api/library.rs`: `humanise_status` | Pure fn in impure file | none (embedded in axum handler module) |
| `docs.rs`: `DocType` / `describe_types` | **Config-coupled** | `crate::config::Config` (server schema) |
| `config.rs`: `Config`, kanban/idle/editor, `from_path` | **Server-runtime-bound** | launcher JSON schema, humantime, lifecycle |

Key specifics:

- **`frontmatter.rs`** — `fence_offsets(raw: &[u8]) -> Result<Option<(usize,
  usize)>, FenceError>` at `frontmatter.rs:21-70`, `MAX_SCAN = 1 << 20` (1 MiB)
  at `:22`. Byte-level, CRLF-tolerant, distinguishes absent (`Ok(None)`) from
  unclosed (`Err(Malformed)`). Frontmatter modelled as
  `BTreeMap<String, serde_json::Value>` at `:74`. The `catch_unwind` around
  `serde_yml::from_str` is at `:143-145`. `yml_to_json` number widening
  (i64→u64→f64, else `Null`) at `:201-211`; **YAML `Tagged` → `return None`** at
  `:233`, which propagates to `Malformed`. Regression test
  `malformed_when_quoted_scalar_has_trailing_whitespace` at `:415-429`.
- **`typed_ref.rs`** — the cleanest leaf. `parse_typed_ref(raw: &str) ->
  Option<TypedRef>` at `:32-72`; `enum TypedRef { WorkItem, Plan, Adr, Pr, Path }`
  at `:12-19`. Strips `work-item:` / `plan:` / `adr:` / `pr:` prefixes; only
  `work-item:` and `plan:` route a path-shaped payload to `TypedRef::Path`.
  Sole dep `std::path`.
- **`slug.rs`** — `derive(kind: DocTypeKey, filename, cfg: &WorkItemConfig)` at
  `:22-72` is the per-doc-type dispatch. Bash-parity harness `compile_scan` at
  `:572-588` shells out to `work-item-pattern.sh --compile-scan` via a
  `CARGO_MANIFEST_DIR`-relative path (`../../../../skills/work/scripts/...`) that
  **breaks when the crate moves into `cli/`** — a known re-pathing chore.
- **`config.rs`** — split file. `WorkItemConfig` (struct `:82-88`, impl
  `:90-230`) is the pure identity-scheme engine: `is_canonical_id_token`
  (`:129-144`), `canonical_digit_width` (`:146-166`, private), `extract_id`
  (`:178-194`), `normalise_id` (`:207-229`). It **receives the compiled scan
  regex** via `from_raw` (`:91-105`) — compiled once at boot, never recompiled.
  The `Config` struct (`:13-55`, `deny_unknown_fields`, carries `owner_pid`,
  `log_path`, `editor`, `idle_timeout`) is launcher-runtime-bound and stays
  server-side. `ConfigError` is shared by both halves — its `InvalidScanRegex`
  variant must travel with `WorkItemConfig` or be split.
- **`humanise_status`** actually lives in `api/library.rs:253-261` (there is no
  top-level `library.rs`), a byte-for-byte mirror of `slug::title_case_segment`.
  The slug.rs code comment at `:229-231` explicitly anticipates unifying these
  "when a third humaniser appears" — that third caser is `config::label_from_key`
  (`config.rs:307-314`). This is the title-caser consolidation AC.

### 2. The `IndexEntry` entanglement and the parse-time core

`IndexEntry` (`indexer.rs:160-196`) fuses three concerns under one
`#[derive(Serialize)] #[serde(rename_all = "camelCase")]` — every field crosses
the `/api` SPA boundary. Full 16-field inventory, classified:

- **Parse-time core (10):** `r#type`, `path`, `rel_path`, `slug`,
  `work_item_id`, `title`, `frontmatter` (`serde_json::Value`, `:172`),
  `frontmatter_state`, `work_item_refs`, `body_preview`. *(The work item's
  enumeration omitted `rel_path` and `body_preview` — both are derived purely in
  `build_entry`: `rel_path` via `strip_prefix(project_root)` at `:1414-1416`,
  `body_preview` via `frontmatter::body_preview_from` at `:1395`.)*
- **File I/O metadata (3):** `mtime_ms`, `size`, `etag` — straight off
  `FileContent` (`file_driver.rs:12-19`), not a later store pass.
- **Store back-fill (3):** `completeness`, `linked_count`, `cluster_key` —
  default to `None`/`0` in `build_entry` (`:1432-1434`), overwritten by later
  cluster/related passes. **These are the only genuinely store-mutated fields.**

So a `ParsedEntry` core = everything except those last three. It is produced
entirely by `build_entry(kind, path, &FileContent, project_root,
&WorkItemConfig) -> IndexEntry` (`:1324-1436`) with **zero** `Indexer`/lock/async
dependency — `build_entry` moves wholesale into the core crate and returns the
core struct.

**Every consumer treats `frontmatter` as opaque JSON** — only `.get()/.as_str()/
.as_array()` navigation, never typed deserialisation:
`extract_facet_value` (`:80-88`), `target_path_from_entry` (`:985`),
`parse_adr_id` (`:1465`), `plan_id_from_entry` (`:1457`), `cluster_key.rs:86,133`.
This is exactly why a `config::Node`-style serde-free enum supplies all
production needs and the single JSON dependency is the API serialisation
boundary (satisfied by a hand-written `Serialize`).

Cleanly extractable id/convention free-functions (pure over
`(WorkItemConfig, DocTypeKey, serde_json::Value)`): `canonicalise_one_id`
(`:1207`), `canonicalise_refs` (`:1254`), `number_width_from_id_pattern`
(`:1193`), `parse_adr_id` (`:1464`), `normalize_absolute` (`:903`),
`normalize_target_key` (`:933`), `cluster_key::id_from_value` (`:147-161`).
`target_path_from_entry` (`:974`) and `build_entry` reference `IndexEntry` only
by generic accessor / return type and follow the core cleanly.

**Stays server-side:** the `Indexer` store (six `Arc<RwLock<HashMap>>` indexes,
`:220-238`, with a documented lock-ordering invariant), the recursive cluster
walker `resolve_cluster_key`/`walk` (`cluster_key.rs:32-121`, recurses through
the entry graph), `clusters.rs`, `related.rs` (takes `&Indexer`, acquires read
locks, async), and the `FileDriver` trait in `file_driver.rs`.

**Circular-dependency smell for the split:** `indexer` imports
`clusters::Completeness` while `clusters` imports `indexer::IndexEntry`. A clean
core split must break this — move `Completeness` out of the core, or keep it
server-side with the `completeness` field as the coupling point.

Correction to the module count: `lib.rs` declares **23** `pub mod` (`:9-31`), not
20; `api` is the only nested module (15 submodules).

### 3. The 0178 config/config-adapters pattern to mirror

The pattern is clean hexagonal (ADR-0053), enforced by three independent
mechanisms.

**Workspace** (`cli/Cargo.toml:4`): members `launcher`, `kernel`, `verify`,
`config`, `config-adapters`. Layering: `kernel` (thiserror/tracing, no
workspace-internal deps) → `config` (**kernel only**) → `config-adapters`
(`config` + `serde` + `serde-saphyr`).

**The `Node` value model** (`cli/config/src/node.rs:4-24`) — the exact shape
0179's frontmatter domain value type must mirror:

```rust
pub enum Node { Scalar(Scalar), Sequence(Vec<Node>), Mapping(Mapping) }
#[non_exhaustive]
pub enum Scalar { String(String), Bool(bool), Int(i64), Float(f64), Null }
pub struct Mapping(Vec<(String, Node)>);  // insertion-ordered
```

Order preservation is structural (`Vec<(String, Node)>`, not a hash map). No
serde derives. `Mapping` accessors: `get`, `get_mut`, `upsert` (position-
preserving replace), `push`, `entries` (ordered slice), `FromIterator`
(`node.rs:26-70`).

**The serde boundary** (`config-adapters/document.rs`): a private `enum Parsed`
mirrors `Node` on the serde side (`:14-22`), with **hand-written**
`Deserialize` (`:176-182` delegating to `ParsedVisitor` at `:115-174`) and
`Serialize` (`:184-211`). **Number-as-String policy** at `:132-135` — a `u64`
beyond `i64` range becomes `Parsed::Str(value.to_string())` rather than losing
precision (this is the policy 0179 adopts over the visualiser's `f64`/`Null`
widening). `preserved_body` (`:52-56`) re-splits + re-parses the existing file
(so a malformed existing file errors before overwrite) and returns the body for
round-trip render. `serde_saphyr::from_str` / `to_string` at `:62`/`:68` — the
only two call sites in the workspace.

**`frontmatter::split`** (`config-adapters/frontmatter.rs:18-44`): `&str` in,
owned `String` halves out (a `Split` struct), `split_inclusive('\n')`,
CRLF-tolerant, no scan cap, body never re-scanned, `Err` **only** on an
unterminated block. This is the owned-halves form 0179's shared crate must expose
alongside the visualiser's byte-offset `fence_offsets` form (the offset form is
the primitive; the owned form derives from it).

**`discover_root`** (`config-adapters/store.rs:33-45`): marker-walk up ancestors
for the first dir containing `.accelerator/`, `.git`, or `.jj`. Note the
`.accelerator/` extra stop marker is Rust-only (bash `find_repo_root` looks only
for `.jj`/`.git`).

**Enforcement (the three mechanisms):**

1. **serde-saphyr pin** `= 0.0.29` (`cli/Cargo.toml:42`); every 0.0.x patch is
   semver-breaking, reviewed at the adapters boundary.
2. **cargo-deny wrapper ban** (`cli/deny.toml:64-69`):
   `{ crate = "serde-saphyr", wrappers = ["config-adapters"] }` — reachable
   *only* through `config-adapters`. Backed by a real regression test
   (`tests/integration/deny/test_serde_saphyr_ban.py`) that runs the **actual**
   `deny.toml` against a banned fixture (must fail, naming serde-saphyr) and a
   clean fixture (must pass) — proving the check evaluated-and-allowed, not
   evaluated-nothing. **For 0179 this wrapper moves to the new document-format
   crate.**
3. **cargo-pup import restriction** (`cli/pup.ron:42-56`):

   ```ron
   Module((
     name: "config_domain_imports_only_permitted",
     matches: Module("^config($|::)"),
     rules: [ RestrictImports(
       allowed_only: Some([ "^(std|core|alloc)(::|$)", "^kernel::Error(::|$)", "^crate(::|$)" ]),
       severity: Error,
     ) ],
   )),
   ```

   `matches` is the **resolved** module path — `^config($|::)` covers the whole
   crate because the entire `config` crate is domain (no adapter modules live in
   it). `allowed_only` matches the **literal** use-path, so in-crate imports must
   be written `crate::`-qualified (`use crate::node::Node;`). The one permitted
   cross-crate import is `^kernel::Error` — because `kernel::Error::Failed(String)`
   (`cli/kernel/src/lib.rs:14`) is the taxonomy each subdomain maps its richer
   error enum into at the dispatch boundary.

`corpus` mirrors this exactly: kernel-only domain, faked ports,
`corpus-adapters` as the infra side, a new pup rule per new domain crate, and the
serde-saphyr ban re-homed to the document-format crate.

### 4. Bash convention library (parity baselines)

**Doc-type inference** — a four-file chain, *not* a hardcoded table:
`config-defaults.sh:74-83` (registry: `DOC_TYPE_NAMES` × `DOC_TYPE_PATH_KEYS`) →
`config-read-doc-type-paths.sh` (resolver, emits `type<TAB>dir` TSV through the
config override chain) → `doc-type-table.sh:load_doc_type_table` (loader,
populates injected arrays) → `doc-type-inference.sh:infer_type_from_path`
(`:45-65`, matcher). The matcher is **longest-configured-dir-wins**, segment-
anchored:

```bash
case "$path" in */"$d"/* | "$d"/*) ;; *) continue ;; esac
```

Fail-closed when the table is not injected. Most-specific wins by dir length;
exact-length tie → first array entry.

The **13 emitted rows** (bash) vs the **14-variant** Rust `DocTypeKey`: the 14th
is `Templates` (virtual, `config_path_key() == None`, `is_virtual() == true`).
`PrDescriptions` has a wire-token/config-key mismatch (`pr-descriptions` on the
wire, `prs` on disk). The full mapping table is in the report body below.

**Typed-linkage** (`scripts/linkage-parser.sh`) — the ADR-0034 parser. CLI:
`linkage-parser.sh <file> [source_type]` emits
`source_type<TAB>key<TAB>target_ref<TAB>anchor<TAB>band`. Only five H2 sections
qualify (`## References`, `## Dependencies`, `## Historical Context`,
`## Related Research`, `## Source References`). Target refs built as
`<type>:<id>` (e.g. `work-item:0042`, `adr:ADR-0001`). Backed by
`scripts/linkage-type-pairs.tsv` (14 rows of valid ADR-0034 pairings). Band is
`resolved` (explicit hint + table-backed pair or universally-valid key) or
`ambiguous`. Deliberately avoids `\b` (BSD awk/grep) — the test suite
(`scripts/test-linkage-parser.sh`) must replay under macOS bash 3.2.

**Note the two-parser reality:** the visualiser's `typed_ref.rs` parses a
*single reference value* (`"plan:0042"` → `TypedRef`); the bash
`linkage-parser.sh` is a whole-document *extractor* (walks sections, infers keys,
classifies bands). The work item's parity target for `corpus`'s `typed_ref` is
the *value* parser; the document-walking extractor is a higher layer.

**Work-item-ID — two layers:**

- *Runtime predicate* (what `corpus` keeps): the Rust `WorkItemConfig` methods
  above, bash-mirrored by `work-item-common.sh` (`wip_extract_id_from_filename`
  `:430`, `wip_canonicalise_id` `:356`, `wip_is_legacy_id` `:271`,
  `wip_parse_full_id` `:292`). Parity suite:
  `skills/work/scripts/test-work-item-scripts.sh`.
- *Pattern DSL compiler* (**out of scope**, 0170/0167's concern):
  `work-item-common.sh:_wip_compile` (`:43-207`), wrapped by
  `work-item-pattern.sh`. DSL tokens: `{number}`, `{number:0Nd}` (width
  `^0[1-9][0-9]*d$`), `{project}` (`^[A-Za-z][A-Za-z0-9]*$`), `{{`/`}}` escapes.
  Flags: `--validate` / `--compile-scan` / `--compile-format`. Compiled scan is
  **width-agnostic and anchored**: `--compile-scan "{number:04d}" ""` →
  `^([0-9]+)-`; `--compile-scan "{project}-{number:04d}" "PROJ"` → `^PROJ-([0-9]+)-`.
  Stable error prefixes `E_PATTERN_*`. **0179 injects the compiled regex** from
  this script at test time — it does not implement the compiler, so it does not
  depend on 0167/0170. Parity suite:
  `skills/work/scripts/test-work-item-pattern.sh`.

**Slug — the three identity schemes** (confirmed in `slug.rs`):

1. **ADR-N** (`Decisions`): literal `ADR-` prefix → `strip_prefix_numbered`.
2. **Bare-N / project-N** (`WorkItems`): leading digit-run (or the tail after the
   compiled scan-regex full match for project patterns).
3. **Date-prefixed with optional embedded work-item-id** (`Plans`, `Research`,
   `Validations`, `Notes`, `PrDescriptions`, `DesignGaps`, `DesignInventories`,
   `RootCauseAnalyses`): strip strict `YYYY-MM-DD-`, then *optionally* strip a
   leading canonical id token via `cfg.is_canonical_id_token`. Review types layer
   a `-review-N` suffix strip; `WorkItemReviews` has a no-date
   `NNNN-slug-review-N` fallback. `Templates` → always `None`.

   Edge cases the baselines pin (fixture corpus must cover): `meta/prs-archive`
   (segment boundary, out of scope), `2026-05-31-0040.md` (id-only → `None`),
   `ADR-0001.md` (no tail → `None`), `2026-04-17-100-day-plan.md` (100 not a
   canonical 4-digit token → preserved), bare-vs-project id under a `{project}`
   pattern (bare id preserved).

### 5. Artifact-metadata family and VCS/repo-root detection

**Three near-identical helpers**, yoked by
`scripts/test-metadata-helpers.sh:21-25`:
`scripts/artifact-derive-metadata.sh` (filename ts `_H-M-S`, `:6`),
`skills/design/analyse-design-gaps/scripts/gap-metadata.sh` (date-only, `:11`),
`skills/design/inventory-design/scripts/inventory-metadata.sh` (`-HMMSS`, `:11`).
They share a **byte-identical VCS block** and derive four facts: current UTC
date/time (`date -u +%Y-%m-%dT%H:%M:%S+00:00` — literal `+00:00`, not `%Z`),
parameterised filename timestamp, repository name (`basename` of root), and
current revision. The test asserts the *common output contract* (revision
present, ISO timestamp, no legacy `Current Git Commit Hash:` etc.) — **not** the
per-helper filename format, so the port parameterises the format and subsumes all
three.

**Three VCS techniques (this is subtler than the work item's "two ports"
framing):**

1. **Command-probe (metadata helpers):** `jj root` / `git rev-parse
   --show-toplevel` for the root, `jj log -r @ ... commit_id` / `git rev-parse
   HEAD` for the revision. Returns the **full** working-copy revision. Falls to
   all-blank in a bare repo (`--is-inside-work-tree` is false).
2. **Marker-walk (`scripts/vcs-common.sh`):** `find_repo_root` (`:8-18`) walks
   `$PWD` up for the first `.jj`/`.git`; `vcs_mode` (`:27-36`) inspects markers
   (`.jj` wins over `.git` for colocated). **Plus** a richer command-probe
   classifier layer — `classify_checkout` (`:177-280`) using `jj workspace root`,
   `git rev-parse --git-common-dir/--is-bare-repository/...` to distinguish
   `main` / `jj-secondary` / `git-worktree` / `colocated` / `nested-*` / `none`.
   **This is the real reference if `corpus-adapters` needs workspace-boundary
   awareness**, not the metadata helpers.
3. **Hybrid, file-scoped (0007 migration `resolve_revision`,
   `0007-...sh:252-261`):** marker-gate (`[ -d "$PROJECT_ROOT/.jj" ]`) picks the
   VCS, then a **file-scoped** command-probe for the last commit touching one
   file (`latest(::@ & files(...))` / `git log -1 -- <path>`), returning the
   **short** id. Root is pre-established, not discovered.

**Where they diverge** (the in-scope investigation surface): secondary jj
workspaces (`jj root` and `find_repo_root` both stop at the workspace `.jj`; only
`classify_checkout` resolves the main workspace root); colocated repos (all let
jj win); bare repos (helpers blank out, `find_repo_root` still matches, migration
gate can misfire); **git-worktrees/submodules where `.git` is a file** — the
migration's `-d` directory gate and `vcs-detect.sh`'s inline `-d` test both have
a blind spot here, mitigated only by the separate `classify_checkout` probe.
`config-adapters/store.rs:discover_root` is the existing Rust marker-walk twin
(uses `.accelerator/`/`.git`/`.jj`).

### 6. Doc-type "triplication" — reframed

The work item calls the dir→type fact "triplicated." Precisely: the **default
table has a single authoring site** (`config-defaults.sh`). What the three
runtime surfaces duplicate is the **matching algorithm** over the same
config-injected data:

- `doc-type-inference.sh:infer_type_from_path` (the file classifier).
- `0007-unify-meta-corpus-frontmatter.sh:49-66` — **re-serialises** the
  already-injected table into an RS-delimited (`0x1E`) string for awk (not a
  second hardcoded copy).
- `0007-frontmatter-rewrite.awk:path_to_typed` (`:81-98`) — a second *matcher*
  (longest-literal-prefix over the injected `DT_DIR[]`), needed because it
  classifies a meta-path *inside a linkage value*, not the current file.

So AC-6 ("single-sourced, no triplication") is really about converging the
**matcher** onto `corpus`, and having the migration snapshot + awk table *derive
from* the crate's `DocTypeKey` source. The awk additionally hardcodes adjacent
*type-relationship* tables (`canonical_type`, `is_linkage_key`,
`bare_target_type` per ADR-0034) that are distinct from the dir→type fact.

### 7. Historical context and binding constraints

- **0166 is updated for the fifth crate** — confirmed in Requirements
  (`0166:53-59`), Size (`:146-147`), and Drafting Notes (`:208-212`). The fifth
  crate is the shared markdown+frontmatter document-format crate, created under
  0179, with the 0178 `config-adapters` retrofit in scope and the serde-saphyr
  deny.toml wrapper moving into it.
- **Adversarial-fixture + bounded-time-guard pattern (0178 plan)** — the exact
  mechanism 0179's document-format crate reuses. serde-saphyr is pure-Rust, so
  **no `catch_unwind`, no adapter-side recursion cap** by default
  (`plan:544-556`). The revisit trigger is widened to *four* failure modes —
  "a catchable panic, a stack-overflow abort, an OOM, or a hang (billion-laughs
  alias expansion)" (`:551-553`) — and `catch_unwind` **cannot** stop an abort;
  the effective control is a depth/input-size bound in the adapter. The
  **bounded-time guard** (`:785-793`): each adversarial parse runs on a worker
  thread joined with `recv_timeout`, *or* via a `config-adapters-fixture`
  subprocess with a wall-clock kill (which also catches abort/OOM in the child) —
  because `cargo test` has no default per-test timeout, a hang must fail the test
  deterministically. Enumerated adversarial fixtures: deeply-nested YAML +
  structurally-malformed YAML (characterise, don't hard-assert), **plus** 0179
  folds in the visualiser's trailing-whitespace quoted-flow-scalar regression.
- **ADR-0034** (typed-linkage): two reference forms — `doc-type:id` (canonical,
  e.g. `"plan:0042"`, `"adr:ADR-0033"`) and project-root-relative path — each a
  single quoted YAML string (`"plan:0042"`, never `plan:"0042"`). Nine flat keys
  (`parent`, `supersedes`, `superseded_by`, `blocks`, `blocked_by`, `target`,
  `derived_from`, `relates_to`, `source`). **Consumers must accept both forms on
  every key** and **derive the inverse** by traversing the corpus (single side
  suffices). `supersedes` is canonical for ADRs.
- **ADR-0045** — the duplication 0179 removes ("duplicate deterministic logic and
  data definitions across Bash and Rust ... a shared compiled core lets both
  surfaces reuse one implementation").
- **ADR-0053** — the hexagonal structure 0179's crates instantiate; inward
  dependency direction enforced mechanically (crate boundaries + cargo-deny for
  cross-crate, cargo-pup for intra-crate on a pinned-nightly lane); ports are
  traits in the domain.
- **Sibling boundaries**: 0180 lands atomic-store primitives *in* `corpus-adapters`
  (blocked by 0179); 0170 (`accelerator-work`) consumes corpus and owns the DSL
  compiler; 0173 (`accelerator-corpus` CLI) consumes these libs and owns
  frontmatter *validation* (out of scope here); 0168 folds the visualiser into
  `cli/` and is **sequenced after** 0179's extraction.

## Code References

- `skills/visualisation/visualise/server/src/frontmatter.rs:21-70` — `fence_offsets`, 1 MiB `MAX_SCAN`, byte-level fence split
- `.../frontmatter.rs:143-145` — `catch_unwind` around `serde_yml::from_str` (obsolete under serde-saphyr)
- `.../frontmatter.rs:201-211,233` — number widening + `Tagged` → `None`
- `.../frontmatter.rs:415-429` — trailing-whitespace quoted-flow-scalar regression test
- `.../typed_ref.rs:32-72` — ADR-0034 value parser (cleanest leaf; sole dep `std::path`)
- `.../slug.rs:22-72` — per-doc-type slug dispatch; `.../slug.rs:572-588` — bash-parity shell-out (breaks on crate move)
- `.../docs.rs:6-21` — `DocTypeKey` (14 variants); `:44-64` — `config_path_key` dir→type
- `.../config.rs:82-230` — `WorkItemConfig` runtime predicate; `:91-105` — receives compiled scan regex
- `.../patcher.rs:8-94` — in-place `status:` mutator over `fence_offsets`
- `.../api/library.rs:253-261` — `humanise_status` (title-caser #3)
- `.../indexer.rs:160-196` — `IndexEntry` (16 fields; parse-time core = 10); `:1324-1436` — `build_entry`
- `.../indexer.rs:220-238` — the six `Arc<RwLock<HashMap>>` store indexes (server-side)
- `.../cluster_key.rs:147-161` — pure `id_from_value`; `:32-121` — recursive walker (server-side)
- `cli/config/src/node.rs:4-24` — the `Node`/`Scalar`/`Mapping` serde-free value model
- `cli/config-adapters/src/document.rs:115-211` — hand-written `Serialize`/`Deserialize`; `:132-135` — number-as-String
- `cli/config-adapters/src/frontmatter.rs:18-44` — owned-halves `split`
- `cli/config-adapters/src/store.rs:33-45` — marker-walk `discover_root`
- `cli/Cargo.toml:42` — serde-saphyr `= 0.0.29` pin
- `cli/deny.toml:64-69` — serde-saphyr `wrappers = ["config-adapters"]` ban
- `cli/pup.ron:42-56` — `config` domain import restriction
- `cli/kernel/src/lib.rs:14` — `Error::Failed(String)`
- `tests/integration/deny/test_serde_saphyr_ban.py` — real-config ban regression test
- `scripts/config-defaults.sh:74-83` — dir→type registry (single authoring site)
- `scripts/doc-type-inference.sh:45-65` — longest-configured-dir-wins matcher
- `scripts/linkage-parser.sh` + `scripts/linkage-type-pairs.tsv` — ADR-0034 document extractor
- `skills/work/scripts/work-item-common.sh:43-207` — pattern DSL compiler (`_wip_compile`); `:356-446` — runtime predicates
- `skills/work/scripts/work-item-pattern.sh` — DSL CLI (`--validate`/`--compile-scan`/`--compile-format`)
- `scripts/artifact-derive-metadata.sh` + `.../gap-metadata.sh` + `.../inventory-metadata.sh` — the metadata family
- `scripts/vcs-common.sh:8-36,177-280` — marker-walk root + `classify_checkout` command-probe classifier
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:49-66,252-261` — doc-type snapshot + `resolve_revision`
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:81-98` — the linkage-path matcher (third dir→type surface)

## Architecture Insights

- **The frontmatter value is opaque JSON at every production site.** Not one
  consumer deserialises it into a typed struct. This is the load-bearing fact
  that makes the serde-free domain enum viable: the domain carries an opaque
  order-preserving value; only the API boundary needs `Serialize` (hand-written,
  proven by config's `Parsed`).
- **The extraction cluster is connected, not a set of independent leaves.** Only
  `typed_ref.rs` stands alone. `frontmatter → {slug, typed_ref}`,
  `slug → {WorkItemConfig, DocTypeKey}`, `patcher → frontmatter`. The natural
  `corpus` domain is this whole cluster plus `WorkItemConfig` (lifted out of
  `config.rs`) and `DocTypeKey` (lifted out of `docs.rs`), leaving the
  `Config`-coupled tails (`describe_types`, the runtime `Config` struct)
  server-side.
- **The `IndexEntry`/`Completeness` circular import is the one structural knot.**
  Any core split must break it — this is the single non-mechanical refactor in
  the visualiser extraction.
- **"Single-sourcing" is about the matcher, not the data.** The dir→type default
  already has one home; the work is converging three *algorithm* copies and
  making the migration/awk surfaces derive from the crate. Framing AC-6 as
  "de-duplicate the data" would miss the actual duplication.
- **The runtime-predicate-in-corpus decision is a layering call, not a semantic
  one.** corpus's own `slug::derive` depends on `is_canonical_id_token`, so
  putting the predicate in a `work` crate would invert the dependency direction.
  The DSL compiler is the clean carve-out (work/config concern, injected regex).
- **Enforcement is mechanical and testable.** cargo-deny `wrappers` + a
  real-config regression test + cargo-pup literal-path import rules. 0179 adds
  one pup rule per new domain crate and re-homes the serde-saphyr ban — no new
  enforcement mechanism to invent.

## Historical Context

- `meta/work/0166-shared-config-corpus-store-crates.md` — parent epic; updated
  for the fifth (document-format) crate (`:53-59,146-147,208-212`).
- `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md:544-556,785-793`
  — parser-choice rationale + bounded-time adversarial guard (the pattern to
  reuse).
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — the linkage format spec.
- `meta/decisions/ADR-0045-*.md` — the bash/Rust duplication 0179 removes.
- `meta/decisions/ADR-0053-*.md` — the hexagonal thin-CLI structure.
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md` —
  the 0178 investigation (config-reader parity oracle, value-encoding divergence).
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`,
  `.../2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md` — the migration
  architecture the epic derives from.
- `meta/research/codebase/2026-06-29-0162-rust-toolchain-guard-rails-wiring.md` —
  cargo-deny/cargo-pup wiring context.

## Related Research

- 0178 config-crates research (above) — the direct predecessor pattern.
- 0136 migration scope/surface research — the epic's origin.
- 0162 toolchain guard-rails research — the enforcement wiring.

## Open Questions

- **VCS-kind vs repo-root technique per port** (the work item's flagged in-scope
  spike): research confirms *three* live techniques diverging on secondary jj
  workspaces, bare repos, and `.git`-as-file worktrees/submodules. The richest
  reference is `vcs-common.sh:classify_checkout` (command-probe), not the
  metadata helpers' simpler probe. The spike must decide whether `corpus-adapters`
  needs full workspace-boundary classification or only the helpers' root+revision
  contract — the two imply very different port surfaces.
- **Document-format crate naming/placement** (deferred to the plan) — a utility
  crate beside `kernel`, consumed by both adapter crates.
- **`Completeness` relocation** — where does it live once `IndexEntry` splits, to
  break the `indexer`↔`clusters` circular import? (Only relevant to 0168's fold,
  but the core boundary 0179 draws determines the answer.)
- **Full vs short revision** — the metadata helpers return the full working-copy
  revision; the 0007 migration returns a short, file-scoped id. Which contract
  does the artifact-metadata port expose (the helpers' full/working-copy, per
  parity), and is file-scoping a separate concern left to 0173's command surface?
