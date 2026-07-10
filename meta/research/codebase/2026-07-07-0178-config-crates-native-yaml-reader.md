---
type: codebase-research
id: "2026-07-07-0178-config-crates-native-yaml-reader"
title: "Research: config and config-adapters Crates with Native YAML Reader"
date: "2026-07-07T00:49:01+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0178"
parent: "work-item:0178"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
topic: "config and config-adapters Crates with Native YAML Reader"
tags: [research, codebase, config, config-adapters, rust, hexagonal, serde, yaml, cargo-deny, cargo-pup, luminosity]
revision: "f38f8ca773496ec3e30561d6f1234166c9d5f38a"
repository: "accelerator"
last_updated: "2026-07-07T00:49:01+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: config and config-adapters Crates with Native YAML Reader

**Date**: 2026-07-07 00:49 UTC
**Author**: Toby Clemson
**Git Commit**: f38f8ca773496ec3e30561d6f1234166c9d5f38a
**Branch**: HEAD (detached)
**Repository**: accelerator

## Research Question

For work item `0178 — config and config-adapters Crates with Native YAML
Reader`: research the existing config scripts (the bash parity target), anything
relevant in the visualiser codebase, and the luminosity codebase at
`../luminosity` (which has already implemented config deserialisation and
serialisation), to ground the implementation of the two new Rust crates.

## Summary

0178 delivers a two-crate hexagon — `config` (serde-free domain + application +
ports) and `config-adapters` (native serde/YAML frontmatter reader + filesystem
adapter) — that replaces the bash 2-level awk reader, and is the **first-mover
activation** of the workspace's cargo-deny infra-out-of-domain ban and cargo-pup
domain-import rule.

The single most important finding: **luminosity has already built exactly this
crate pair**, and it is a near-complete reference implementation to mirror.
`../luminosity/cli/config` + `cli/config-adapters` implement the same
domain/adapter split, the same team/personal frontmatter-in-Markdown scheme, the
same per-key precedence, arbitrary nesting, inline-array handling, and the same
`From<ConfigError> for kernel::Error` boundary mapping this task needs. **It also
resolves the task's Open Question in an unexpected direction**: luminosity uses
neither `serde_yaml` nor `serde_yml` — it uses **`serde-saphyr 0.0.29`**, a
pure-Rust, order-preserving YAML crate, sidestepping the maintained-fork
question entirely and avoiding the libyml C-panic hazard.

Three secondary findings shape the build:

1. **The bash reader is fully characterised** — exact 42-key catalogue across
   five groups + two doc-type parallel arrays, the last-writer-wins precedence
   loop, the *presence-not-value* shadowing semantic, the verbatim legacy guard,
   and `config_parse_array`. There are subtle behaviours (presence shadows even
   with an empty value; string-prefix matching not regex; one-pair quote
   stripping on scalars but not array elements) a parity-seeking Rust port must
   preserve.

2. **The visualiser's `config.rs` is confirmed non-reusable** (JSON-only,
   single-file, schema-specific) — but the repo is *not* YAML-free: the
   visualiser server crate already depends on **`serde_yml 0.0.12`**, used in
   `frontmatter.rs` via an *untyped `Value` walk wrapped in `catch_unwind`
   because libyml panics on adversarial input**. That caveat is a direct
   argument for luminosity's `serde-saphyr` choice.

3. **The hexagon template and enforcement scaffolding are ready and waiting.**
   The version slice (`cli/launcher/src/version/`) is the cleanest shape to
   copy; `cli/deny.toml:67-73` and `cli/pup.ron:10-39` are scaffolded with the
   comment that they stay inert "until the config/config-adapters split makes the
   rule bite". A structural wrinkle: because `config`/`config-adapters` are
   *separate crates* (not modules inside `accelerator`), the pup regex shifts
   from module-rooted (`^accelerator::version::core`) to **crate-rooted**
   (`^config::…::core`), and the deny `wrappers = ["config-adapters"]` mechanism
   bites at crate granularity — which is precisely why the ban was scaffolded to
   wait for a real crate boundary.

---

## Detailed Findings

### Area 1 — The bash config reader (the parity target)

The system reads YAML frontmatter from `.accelerator/config.md` (team) and
`.accelerator/config.local.md` (local). All value reads funnel through
`scripts/config-read-value.sh`.

#### Value resolution and the 2-level cap

- **Key split** (`scripts/config-read-value.sh:24-39`): key split on the *first*
  dot into `SECTION` / `SUBKEY`. `SUBKEY="${KEY#*.}"` strips only the first dot,
  but the awk then matches the remainder as a literal subkey prefix inside the
  section — so genuine 3-level nesting is structurally impossible.
- **The cap is the absence of recursion** (`config-read-value.sh:56-110`): there
  are exactly two awk paths — a nested-section reader (`:60-91`) and a top-level
  reader (`:94-110`). The nested reader detects a `section:` header, then reads
  indented `subkey:` lines; it never descends into a further nested section. A
  deeper-indented key is only ever matched as a literal prefix on the stripped
  line. This is the cap 0178 removes.
- **Matching is string-prefix, not regex** (`config-read-value.sh:42-43`,
  `:64-66`), deliberately so dots/brackets in keys aren't interpreted, and so
  `review:` doesn't match `reviewer:` (the char after `section:` must be
  end-of-line or whitespace).
- **Quote stripping** (`config-read-value.sh:82-84`): one surrounding pair of
  matching single or double quotes is stripped from scalar values — but **not**
  from array elements.

#### Precedence — the load-bearing subtlety

- **Last-writer-wins loop** (`config-read-value.sh:114-130`): it does *not* break
  on first match; it iterates all files (team then local) and keeps the *last*
  successful read. Order is set by `config_find_files()`
  (`config-common.sh:27-49`): team first, local second. Effective precedence,
  highest to lowest: `config.local.md` → `config.md` → the caller's `[default]`.
- **Presence, not value** (`config-read-value.sh:73-89`): `_read_from_file`
  returns success whenever the key prefix matches, *regardless of whether the
  value is empty*. So a present-but-blank key in `config.local.md` shadows both
  the team value and the default. A Rust port must replicate "found = present,
  not non-empty" — luminosity does exactly this (a present `null`/empty string
  resolves to `Found`).

#### The legacy guard (verbatim, `config-common.sh:55-67`)

```bash
config_assert_no_legacy_layout() {
  [ "${ACCELERATOR_MIGRATION_MODE:-}" = "1" ] && return 0
  local root
  root=$(config_project_root)
  local team="$root/.accelerator/config.md"
  local legacy_team="$root/.claude/accelerator.md"
  if [ ! -f "$team" ] && [ -f "$legacy_team" ]; then
    printf '%s\n' \
      "Accelerator: legacy config detected at .claude/accelerator.md." \
      "Run /accelerator:migrate to update the layout, then retry." >&2
    exit 1
  fi
}
```

- Trigger: new-layout team file **absent** AND legacy `.claude/accelerator.md`
  **present**. The `.local` variants are *not* part of the trigger.
- Output: two stderr lines, then `exit 1`.
- 0178 reimplements this **without** the `ACCELERATOR_MIGRATION_MODE` early
  return (AC: even with `ACCELERATOR_MIGRATION_MODE=1` + legacy layout, the Rust
  reader still exits 1 — fails closed). The migration-internal read fallback in
  `config_find_files` (`config-common.sh:41-47`) is likewise not ported; parent
  0166 assigns any future mid-migration fallback to work item 0172.

#### The recognised-key catalogue (42 keys, 5 groups + 2 doc-type arrays)

Split across two files — this is the exact set the domain must model:

**`scripts/config-defaults.sh`**
- **`PATH_KEYS` (17)** `:26-44` with index-aligned `PATH_DEFAULTS` `:46-64` —
  e.g. `paths.plans`→`meta/plans`, `paths.work`→`meta/work`,
  `paths.templates`→`.accelerator/templates`,
  `paths.integrations`→`.accelerator/state/integrations`, etc. (full 17-row table
  in Code References).
- **`DOC_TYPE_NAMES` (13)** `:74-78` and **`DOC_TYPE_PATH_KEYS` (13)** `:79-83` —
  index-aligned parallel arrays mapping each doc-type (`work-item`, `plan`,
  `adr`, `codebase-research`, `note`, …) to a bare `PATH_KEYS` suffix (`work`,
  `plans`, `decisions`, `research_codebase`, `notes`, …).
- **`TEMPLATE_KEYS` (6)** `:85-92`, **no defaults** (template reads default to
  empty, resolved via the 3-tier `config_resolve_template`,
  `config-common.sh:399-439`).
- **`WORK_KEYS` (3)** `:94-98` with `WORK_DEFAULTS` `:100-104`:
  `work.integration`→`""`, `work.id_pattern`→`{number:04d}`,
  `work.default_project_code`→`""`.
- **`WORK_INTEGRATION_VALUES`** `:110-115` — a *value-domain constraint* on
  `work.integration` (`jira`, `linear`, `trello`, `github-issues`), **not** a
  separate key; excluded from the 42 count.

**`scripts/config-dump.sh`** (the review/agent keys live here, *not* in
config-defaults.sh)
- **`REVIEW_KEYS` (9)** `:109-119` with `REVIEW_DEFAULTS` `:121-131`. Two
  defaults are **inline YAML arrays**:
  `review.core_lenses`→`[architecture, code-quality, test-coverage,
  correctness]`, `review.disabled_lenses`→`[]`. The rest are scalars
  (`max_inline_comments`→`10`, `min_lenses`→`4`, etc.).
- **`AGENT_KEYS` (7)** `:134-142` with `AGENT_DEFAULTS` `:144-152`, each default
  `accelerator:<name>` (prefix from `config-common.sh:13`).

**Catalogue caveat the parity fixtures must not miss**: the dump arrays are *not*
the complete recognised set. `scripts/config-read-review.sh` reads three further
keys with hard-coded defaults that never appear in `REVIEW_KEYS`:
`review.work_item_revise_severity` (`critical`),
`review.work_item_revise_major_count` (`2`), and a **mode-dependent**
`review.min_lenses` default (4 for pr/plan, 3 for work-item;
`config-read-review.sh:29-32,73-77,85-86`). If "every recognised key" is to mean
what skills actually read, these belong in the fixture suite.

#### Inline-array parsing (`config_parse_array`, `config-common.sh:318-331`)

A *second stage* applied to a normal scalar read (the value reader treats
`[a, b]` as an opaque string): strip one leading `[` / trailing `]`, split on
commas via `tr`, trim each element with `sed`, drop empties. `[]`→zero elements.
It does **not** strip quotes from elements and does **not** handle commas inside
quoted elements. 0178's AC replaces this with serde typed sequences, so a Rust
port need not reproduce the string-splitting — but must reproduce the *result*
(ordered element list, empty `[]`→empty sequence).

#### Frontmatter extraction contract (`config-common.sh:74-101`)

Recognised only if line 1 is exactly `---` (`/^---[[:space:]]*$/`). Opening `---`
with no closing `---` → treated as malformed: `_read_from_file` warns to stderr
and skips the file (`config-read-value.sh:44-54`). A file with no frontmatter is
all-body. Luminosity's `frontmatter::split` mirrors this contract (with CRLF
tolerance and body-preservation for round-tripping).

### Area 2 — The visualiser codebase

#### `config.rs` is confirmed non-reusable

`skills/visualisation/visualise/server/src/config.rs` is a **JSON-only**,
single-file, hard-coded-schema reader: `Config::from_path`
(`config.rs:280-292`) does `serde_json::from_slice` into a flat `Config` struct
with `#[serde(deny_unknown_fields)]` (`config.rs:13-55`). It does **no** file
location or team/local layering — `scripts/launch-server.sh` produces one
flattened `config.json` upstream and the server reads that single path from a
`--config` flag (`main.rs:10-18`). All substantive logic is visualiser-specific
(work-item regex, kanban defaults, humantime idle-timeout). Parent 0166 already
records that only `WorkItemConfig` (`config.rs:83-251`) is reusable *corpus*
logic — and that belongs to sibling 0179, not this task.

#### But the repo already ships a YAML crate — with a panic caveat

The visualiser server crate depends on **`serde_yml = "0.0.12"`**
(`server/Cargo.toml:46`) plus `gray_matter 0.3` (`:45`). The only real YAML
parsing in the repo is `server/src/frontmatter.rs`, and it is **untyped, not
derive-based**: `serde_yml::from_str::<serde_yml::Value>` into an untyped
`Value` (`frontmatter.rs:143-145`), **wrapped in `catch_unwind` because
`serde_yml`/libyml can panic on adversarial input** (`frontmatter.rs:134-154`),
then a manual `Value`→`serde_json::Value` walk (`yml_to_json`, `:196-235`).

This is directly relevant to the Open Question: the repo's own experience with
`serde_yml` is that its C backend (libyml) panics and had to be defensively
sandboxed. That is a concrete argument against adopting `serde_yml` for the new
reader and in favour of luminosity's pure-Rust `serde-saphyr`.

The `cli/` workspace itself has **no YAML crate at all** yet — the
`[workspace.dependencies]` closure is JSON-only (`cli/Cargo.toml:38-39`: `serde`
+ `serde_json`). 0178 adds the first YAML dependency there.

### Area 3 — The hexagon template + enforcement scaffolding to mirror/activate

#### The version slice is the cleanest hexagon to copy

`cli/launcher/src/version/` is the exemplar (parent 0166 and the work item both
point here):

- **`core.rs`** — the domain owns the ports as traits and depends only on
  std + `kernel::Error` (enforced by cargo-pup). Outbound/driven port
  `BuildMetadata` (`core.rs:5-11`); the value object `VersionReport`
  (`:13-20`); inbound/driving port `ReportVersion` (`:22-25`); the application
  service `VersionReporter<M: BuildMetadata>` generic over the outbound port,
  holding it by value and implementing the inbound port (`:27-47`). Tests inject
  a `FakeBuildMetadata` (`:53-68`).
- **`outbound/build_metadata.rs`** — `VergenBuildMetadata` implements the core's
  `BuildMetadata` against real infrastructure; it imports the port *inward*
  (`use crate::version::core::BuildMetadata`, `:4`).
- **`inbound/cli.rs`** — takes `&impl ReportVersion` and prints; thin.
- **Composition root** (`main.rs:86-92`): `VersionReporter::new(VergenBuildMetadata)`
  — concrete adapter injected into the service, then dispatched. This is the
  Model 1 shape AC-7 requires.
- **Error mapping** (`launch/core.rs:167-171`): `impl From<ResolutionError> for
  kernel::Error { … Self::Failed(error.to_string()) }`. The `config` crate
  defines its own `ConfigError` enum and adds the same `From` impl; `kernel` is
  the lower crate and cannot name subdomain error types
  (`kernel/src/lib.rs:9-15`, `Error::Failed(String)` is the catch-all).

#### Workspace + new-crate mechanics

- Add `config` and `config-adapters` to `members` (`cli/Cargo.toml:4`).
- Per-crate `Cargo.toml` inherits everything via `.workspace = true` and opts
  into shared lints with `[lints] workspace = true` (see `cli/verify/Cargo.toml`
  for the minimal shape). Intra-workspace deps use `{ path = "../kernel" }`;
  external deps use `{ workspace = true }`.
- `config` depends on `kernel` + a YAML crate (serde-free domain — but note:
  luminosity keeps serde *entirely* out of the domain crate and puts it in
  `config-adapters`; see Architecture Insights). `config-adapters` depends on
  `config` + `serde` + the YAML crate.
- Add the YAML crate to `[workspace.dependencies]` (pinned) and its licence to
  `deny.toml`'s allow-list (`:41-52`) if it introduces a new one — `deny.toml`
  warns on *unused* allowances.

#### cargo-deny — the infra-out-of-domain ban (`cli/deny.toml:55-73`)

Currently `[[bans.deny]]` forbids only the three TLS crates; `skip`/`skip-tree`
are empty with the comment (`:67-71`) that the mechanism stays inert "until the
config/config-adapters split makes the rule bite". To activate: add a deny entry
for the YAML crate with `wrappers = ["config-adapters"]`, so any *other*
dependent — notably the `config` domain crate importing the YAML library
directly — violates. AC-8 requires a **committed violating canary** (a
`config`-domain import of `serde_yml`/the chosen crate) confirmed to make
cargo-deny exit non-zero; presence of the rule alone does not satisfy the AC.
Runs via `mise run deny:check` from `cli/`.

#### cargo-pup — the domain-import rule (`cli/pup.ron:8-41`)

Existing rules (`:10-24` version, `:25-39` launch) use `RestrictImports` with
`allowed_only`: a module matched by `matches: Module("^accelerator::version::core($|::)")`
may import only std/core/alloc, `^kernel::Error(::|$)`, and its own subtree
`^crate::version::core(::|$)`. **Structural difference for 0178**: those regexes
are rooted at `accelerator` (the launcher package name) because the hexagons are
*modules inside one crate*. `config` is a *separate crate*, so:
- `matches` becomes crate-rooted: `^config::…::core($|::)`.
- The self-reference allowance stays `^crate::…::core` (crate-relative from
  inside `config`), plus `^kernel::Error`.
This crate-boundary shift is exactly why the deny `wrappers` ban was scaffolded
to wait — a crate boundary, not a module boundary, is what makes it enforceable.
Runs via `mise run pup:check` on a pinned-nightly lane. Both gates are already
wired into `mise run check` and the default task, so activation is edits to
`deny.toml`/`pup.ron` only — no task changes.

### Area 4 — Luminosity: the near-complete reference implementation

`../luminosity/cli/` implements the identical two-crate pair. This is the
strongest asset for 0178 — it can be mirrored almost directly.

#### The Open Question, resolved a third way: `serde-saphyr`

Luminosity uses **neither `serde_yaml` nor `serde_yml`**. From
`../luminosity/cli/Cargo.toml:29-33`:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# Pure-Rust YAML with typed, order-preserving round-trip (config-adapters uses
# it to map frontmatter into the serde-free config::Node).
serde-saphyr = "0.0.29"
```

`serde-saphyr 0.0.29` is a **pure-Rust** YAML implementation (no libyml C
backend → no `catch_unwind` panic-sandbox needed, unlike the visualiser's
`serde_yml`), with typed order-preserving round-trip. Consumed **only** by
`config-adapters` (`../luminosity/cli/config-adapters/Cargo.toml:12-15`); the
`config` domain crate depends on `kernel` alone. Caveat: it is a `0.0.x`
early-stage crate. This gives 0178 three candidates for the Open Question, with
a clear repo-experience signal: `serde_yml` (already vendored but libyml-panics),
`serde_yaml` (unmaintained), or `serde-saphyr` (what the reference impl chose).

#### The domain model — a dynamic, order-preserving tree (no fixed schema)

`../luminosity/cli/config/src/node.rs:8-29`: a recursive `Node` enum
(`Scalar | Sequence(Vec<Node>) | Mapping`) with a typed `Scalar`
(`String|Bool|Int|Float|Null`) and `Mapping(Vec<(String, Node)>)` — a `Vec`, not
a map, deliberately to preserve YAML key insertion order across a round-trip and
keep no ordered-map crate in the core's closure. The domain has **no serde
derives and no `#[serde(...)]` attributes** — it is schema-*less*. The serde-side
mirror type `Parsed` lives in the adapter (`config-adapters/src/document.rs:14-22`)
with a **hand-written `Visitor`** via `deserialize_any` (`:115-182`) and a manual
`Serialize` (`:184-211`). Other domain types: `Level` (Team/Personal,
`level.rs`), `Key` (dotted path as `Vec<String>` with validating `parse`,
`key.rs:8-27`).

**Design choice 0178 must make**: luminosity's dynamic `Node` tree is
schema-agnostic — it does not model the 42-key catalogue as typed fields. 0178's
requirements say to "model the recognised-key catalogue … as domain concepts",
which is a *stronger* typing than luminosity chose. Two viable shapes: (a) mirror
luminosity's dynamic tree + a separate recognised-key/defaults catalogue as
domain constants layered on top, or (b) a typed struct schema with serde derives
in the adapter. Luminosity is a model for (a); the visualiser's `Config` struct
is the (b) shape (but JSON/schema-specific). See Open Questions.

#### Frontmatter split (`config-adapters/src/frontmatter.rs:21-55`)

`split()` uses `split_inclusive('\n')` so newlines are retained and the body
round-trips byte-for-byte; only the first two `---` fences delimit frontmatter
(a `---` thematic break in the body survives); `is_fence` is CRLF-tolerant; a
file not starting with `---` is all-body; unclosed frontmatter is an error. This
is a faithful, better-tested version of the bash `config_extract_frontmatter`
contract.

#### Precedence — per-key, personal-over-team (`config/src/service.rs:83-99`)

`ConfigService::get` reads Personal then Team and resolves **per key at read
time** (not a document merge): if the key resolves `Found` in Personal it wins,
else fall through to Team. Crucially, a present `null`/empty string counts as
`Found` and wins (tests `present_null_resolves_to_found` `:328`,
`present_empty_string_resolves_to_found` `:344`) — the exact "presence, not
value" semantic the bash reader has. If a Personal path lands on a *mapping*
where a scalar was sought, it resolves `Absent` and falls through. It fails loud:
a full `get` reads both levels up front and errors if either is malformed. This
maps directly onto the bash team→local last-writer-wins model (luminosity's
"Personal over Team" == bash "local over team").

#### Nesting, arrays, writes, errors

- **Arbitrary nesting** via the recursive `Node`; `resolve`/`insert` walk and
  auto-create intermediate mappings (`service.rs:116-187`). No depth cap — this
  is the headline capability over the bash reader.
- **Inline arrays**: `visit_seq` (`document.rs:153-162`) handles both block and
  flow YAML; `Parsed::Seq`→`Node::Sequence`. Note luminosity treats a sequence
  as an *opaque leaf* for get/set (a dotted key landing on a `Sequence` resolves
  `Absent`); 0178's AC-4 instead wants inline arrays resolved to a *typed
  sequence with the expected element list* — so 0178 needs sequence values to be
  addressable/returnable, a small extension over luminosity's get semantics.
- **Writes**: `FileConfigStore` (`config-adapters/src/store.rs`) does atomic
  temp-file-then-rename (`:50-72`), roots via upward walk for `.luminosity/` or
  `.git` (`:112-121`), and rewrites the whole document for a single level;
  `render` preserves the existing body verbatim (`document.rs:41-50`). 0178 is
  read-focused, but this is the write side 0167's `config set` will want.
- **Error taxonomy** (`config/src/error.rs:39-61`): `ConfigError { NotFound,
  PathConflict, MalformedFrontmatter, Io, InvalidKey }`; serde errors are
  stringified at the adapter boundary (`document.rs:62-63`) then wrapped with
  file context into `MalformedFrontmatter { path, detail }`
  (`store.rs:85-90`); finally `impl From<ConfigError> for kernel::Error`
  → `Failed(error.to_string())` (`error.rs:99-103`). This is the exact boundary
  pattern to copy.

#### Hexagon layout (mirror this directory shape)

- Domain core `config/` depends only on `kernel` (no serde/YAML/fs): `node.rs`,
  `key.rs`, `level.rs`, `error.rs`, `service.rs` (with driven ports
  `ReadConfigLevel`/`WriteConfigLevel`, driving port `ConfigAccess`, service
  `ConfigService<R, W>`).
- Adapters `config-adapters/`: `frontmatter.rs`, `document.rs` (the serde
  boundary), `store.rs` (fs + I/O), exporting only `FileConfigStore`.
- Composition root: launcher `discover_config_service`
  (`../luminosity/cli/launcher/src/main.rs:84-92`) builds
  `ConfigService::new(store.clone(), store)` — one `FileConfigStore` backs both
  ports; `LazyConfigAccess` defers the fs walk until first `get`/`set`.

---

## Code References

### Bash reader (accelerator)
- `scripts/config-read-value.sh:24-39` — key split (first-dot only)
- `scripts/config-read-value.sh:56-110` — the two awk paths = the 2-level cap
- `scripts/config-read-value.sh:114-130` — last-writer-wins precedence loop
- `scripts/config-read-value.sh:73-89` — presence-not-value success semantic
- `scripts/config-common.sh:27-49` — `config_find_files` (team-then-local order)
- `scripts/config-common.sh:55-67` — `config_assert_no_legacy_layout` (verbatim above)
- `scripts/config-common.sh:74-101` — frontmatter extraction contract
- `scripts/config-common.sh:318-331` — `config_parse_array`
- `scripts/config-defaults.sh:26-64` — `PATH_KEYS`/`PATH_DEFAULTS` (17)
- `scripts/config-defaults.sh:74-83` — `DOC_TYPE_NAMES`/`DOC_TYPE_PATH_KEYS` (13 each)
- `scripts/config-defaults.sh:85-104` — `TEMPLATE_KEYS` (6), `WORK_KEYS`/`WORK_DEFAULTS` (3)
- `scripts/config-defaults.sh:110-115` — `WORK_INTEGRATION_VALUES` (value constraint)
- `scripts/config-dump.sh:109-152` — `REVIEW_KEYS` (9) + `AGENT_KEYS` (7)
- `scripts/config-read-review.sh:29-32,85-86` — three extra review keys not in `REVIEW_KEYS`

### Visualiser (accelerator)
- `skills/visualisation/visualise/server/src/config.rs:280-292` — JSON `from_path`
- `.../server/src/config.rs:13-55` — flat `Config` struct (`deny_unknown_fields`)
- `.../server/src/frontmatter.rs:134-154` — `serde_yml` untyped `Value` + `catch_unwind`
- `.../server/Cargo.toml:45-46` — `gray_matter 0.3`, `serde_yml 0.0.12`

### Hexagon template + enforcement (accelerator)
- `cli/launcher/src/version/core.rs:5-47` — ports + service exemplar
- `cli/launcher/src/version/outbound/build_metadata.rs:4-31` — driven adapter
- `cli/launcher/src/main.rs:86-92` — composition root (Model 1)
- `cli/launcher/src/launch/core.rs:167-171` — `From<DomainError> for kernel::Error`
- `cli/kernel/src/lib.rs:9-15` — `Error::Failed(String)` catch-all
- `cli/Cargo.toml:4,38-39` — members + serde workspace deps (no YAML crate)
- `cli/verify/Cargo.toml` — minimal per-crate manifest shape
- `cli/deny.toml:55-73` — bans structure + scaffolded `wrappers` ban
- `cli/pup.ron:10-39` — `RestrictImports` rules (module-rooted; 0178 goes crate-rooted)

### Luminosity reference (`../luminosity`)
- `cli/Cargo.toml:29-33` — `serde-saphyr 0.0.29` (resolves the Open Question)
- `cli/config/src/node.rs:8-29` — order-preserving dynamic `Node`/`Scalar`/`Mapping`
- `cli/config/src/service.rs:83-99` — per-key personal-over-team precedence
- `cli/config/src/service.rs:116-187` — nested resolve/insert (arbitrary depth)
- `cli/config/src/error.rs:39-61,99-103` — `ConfigError` + `From … for kernel::Error`
- `cli/config-adapters/src/frontmatter.rs:21-55` — CRLF-aware body-preserving split
- `cli/config-adapters/src/document.rs:14-22,115-211` — `Parsed` + hand-written serde
- `cli/config-adapters/src/store.rs:50-121` — atomic write + root discovery
- `cli/launcher/src/main.rs:84-92` — composition root + `LazyConfigAccess`

---

## Architecture Insights

- **Mirror, don't invent.** Luminosity is the direct ancestor (ADR-0047 and
  ADR-0053 are both "ported from luminosity"; parent 0166 explicitly "mirrors
  luminosity 0009"). The `config`/`config-adapters` split, the `Node` tree, the
  frontmatter split, the precedence resolver, the error boundary, and the lazy
  composition root can be lifted with accelerator-specific naming
  (`.accelerator/` paths, the migrate directive, the 42-key catalogue).

- **Serde belongs in the adapter, not the domain.** Luminosity keeps the `config`
  core serde-free (depends on `kernel` only) and puts *all* serde/YAML/fs in
  `config-adapters`. This is what makes the cargo-pup rule and the cargo-deny
  `wrappers` ban meaningful: the domain crate literally must not have the YAML
  crate in its dependency closure. The work item's phrase "serde-free domain" and
  the AC-8 canary both assume this arrangement. (Note the work item's Requirements
  say `config` is "domain + application + ports, no outbound dependency" — matching
  luminosity's serde-free core, so lean toward the dynamic-tree shape (a) over a
  serde-derived typed schema in the domain.)

- **The YAML-crate decision has a clear signal.** Repo experience with
  `serde_yml` (libyml panics, needed `catch_unwind`) argues against it;
  `serde_yaml` is unmaintained; luminosity's `serde-saphyr` is pure-Rust and
  order-preserving but early-stage (`0.0.x`). The safest mirror is
  `serde-saphyr`, matching the reference impl and avoiding the panic hazard — but
  its maturity should be sanity-checked (it is the cargo-deny `[[bans.deny]]`
  target either way).

- **Crate boundary is why the enforcement was scaffolded, not activated.**
  cargo-deny `wrappers` and the crate-rooted cargo-pup regex both require a real
  crate split. 0178 is the first task to introduce one in the shared layer,
  hence "first-mover activation". Siblings 0179/0180 extend the same files.

- **Parity is bounded to depth ≤2.** The bash reader is the oracle only where it
  can represent the data (≤2 levels); depth ≥3 and typed inline-arrays are
  verified against declared expected values, not bash parity. The 0178 review
  (three passes to APPROVE) drove exactly this split into the ACs.

## Historical Context

- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md` — the
  config *model* (not the reader): dedicated `.accelerator/` dir, team `config.md`
  + personal `config.local.md`, last-writer-wins personal-last, **CLI-native
  reader dropping the 2-level cap**, arbitrary YAML. Names the bash-3.2 floor
  (ADR-0049) as the sole reason for the cap and the no-YAML-dependency ban that
  0178 removes. Ported from luminosity ADR-0003.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  — the hexagon: ports-as-traits, inward dependency enforced by cargo-deny
  (between crates) + cargo-pup (within a crate). Explicitly notes crate
  boundaries are "initially inert" and cargo-pup is sole enforcer until a
  subdomain splits into crates — 0178 is that split for config. Ported from
  luminosity ADR-0009.
- `meta/work/0166-shared-config-corpus-store-crates.md` — parent story. Confirms:
  visualiser `config.rs` is *not* the native reader (only `WorkItemConfig` is
  reusable, and that's corpus/0179); `ACCELERATOR_MIGRATION_MODE` fallback
  deliberately not ported (deferred to 0172); Model 1 = each sub-binary wires its
  own `config-adapters` at its composition root; the deny/pup ban-lists "first
  bite at the config/config-adapters split here".
- `meta/reviews/work/0178-config-crates-native-yaml-reader-review-1.md` — the
  work item reached APPROVE over three passes. The ACs were hardened to: an
  explicit shared fixture suite, ≤2 parity + ≥3 declared-value verification,
  typed inline-array criterion, the `ACCELERATOR_MIGRATION_MODE` negative case, a
  single named "config-reader entry point" (the composition-root example), and a
  committed cargo-deny **canary** naming `serde_yml` as the ban target.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — epic-wide architecture (parent 0166 derived from it).

## Related Research

- `meta/research/codebase/2026-06-27-0157-porting-luminosity-adrs-and-feeding-spikes.md`
  — luminosity reference-impl research.
- `meta/research/codebase/2026-07-02-0163-cli-workspace-version-subcommand-scaffold.md`
  — the version hexagon scaffold this task mirrors.
- `meta/research/codebase/2026-06-29-0162-rust-toolchain-guard-rails-wiring.md`
  — the cargo-deny/cargo-pup wiring 0178 activates.

## Open Questions

1. **YAML crate choice** (the work item's stated Open Question). Three
   candidates now, with a signal: `serde-saphyr 0.0.29` (luminosity's choice,
   pure-Rust, order-preserving, but `0.0.x`), `serde_yml 0.0.12` (already in the
   repo but libyml-panics — needed `catch_unwind`), `serde_yaml` (unmaintained).
   Recommendation to validate: mirror luminosity with `serde-saphyr`, pending a
   maturity/maintenance sanity check. Whichever is chosen is the cargo-deny
   `[[bans.deny]]` target and the AC-8 canary import.
2. **Domain shape: dynamic tree vs typed schema.** Luminosity models config as a
   schema-less order-preserving `Node` tree with the recognised keys enforced
   elsewhere; the visualiser uses a typed serde struct. 0178's "no outbound
   dependency in the domain" + "model the recognised-key catalogue as domain
   concepts" points to a dynamic tree *plus* a domain-side catalogue/defaults
   table. Confirm during planning which the catalogue+defaults live as (constants
   in the domain vs a typed struct with serde defaults in the adapter).
3. **Sequence addressability.** Luminosity treats sequences as opaque leaves for
   get/set; 0178 AC-4 wants inline arrays *returned* as typed element lists — a
   small extension to the resolve semantics to design in.
4. **Catalogue completeness for the fixture suite.** Should the parity fixtures
   include the three review keys read only by `config-read-review.sh` (outside
   `REVIEW_KEYS`) and the mode-dependent `min_lenses` default? They are keys
   skills actually read, so arguably yes for true behavioural parity.
