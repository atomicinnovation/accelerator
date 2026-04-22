---
date: "2026-04-21T09:00:00+01:00"
type: plan
skill: create-plan
ticket: null
status: draft
---

# Meta Visualiser — Phase 3: FileDriver, Indexer, and read-only API

## Overview

Turn the Phase 2 placeholder server into a read-only API surface. After this
phase, `GET /api/types` enumerates the ten DocTypes, `GET /api/docs?type=…`
returns indexed entries per type, `GET /api/docs/{*path}` returns a single
document with a strong SHA-256 ETag and honours `If-None-Match` → `304`,
`GET /api/templates` lists the five template summaries,
`GET /api/templates/:name` returns all three resolution tiers per
template, and `GET /api/lifecycle` / `GET /api/lifecycle/{slug}` return
slug-clustered pipelines. No writes, no SSE, no file watcher — all deferred
to later phases.

Phase 3 lands as **ten sub-phases**. Every sub-phase follows Red → Green →
Refactor: a failing test (unit or integration) is committed in its own `jj`
revision before the implementation lands; where tests and implementation
bundle into a single commit (e.g. when the test needs a freshly-added public
API to compile), the sub-phase's Manual Verification prescribes a **mutation
smoke test** — temporarily break the implementation, confirm the suite
fails, restore. This mirrors Phase 2's TDD discipline.

This phase resolves the following handoffs flagged in Phase 2:

- **`AppState` extension** (Phase 2 Migration Notes): gains `Arc<Indexer>`,
  `Arc<TemplateResolver>`, and `Arc<RwLock<Vec<LifecycleCluster>>>` fields
  in Phase 3.8.
- **Path-safety guard** (Phase 2 § What We're NOT Doing): `LocalFileDriver`
  canonicalises every disk path and enforces a prefix check against the
  configured doc-type roots in Phase 3.4.
- **Router builder extraction** (implicit): `server.rs::run` inlines the
  `Router` construction today; Phase 3.8 extracts a `build_router(state)`
  helper so integration tests can drive the full stack via
  `tower::ServiceExt::oneshot` without binding a real port. `run` then
  delegates to the helper.
- **`gray_matter`, `serde_yml`, `sha2` crates** (Phase 2 Key Discoveries):
  added in Phase 3.1 — their first consumers are the frontmatter parser
  (3.3) and the indexer (3.5).

The phase also executes the design decisions locked in the research
document:

- **D5** (two distinct review DocTypes): `plan-reviews` reads from
  `review_plans`, `pr-reviews` reads from `review_prs`. Both are flat walks;
  the nested `meta/reviews/{plans,prs}/` structure is absorbed by the
  two-type split.
- **D9** (three-tier templates): `templates` is a **virtual** DocType that
  does not walk a directory; Phase 3.6 implements the resolver over the
  `config.templates` map.
- **G3** (review slug suffix strip): Phase 3.2 anchors the `-review-\d+\.md$`
  regex at end-of-string so a plan slug that itself contains `-review-`
  internally (e.g.
  `initialise-skill-and-review-pr-ephemeral-migration-review-1.md`) still
  collapses to the correct cluster slug.
- **G6** (three-state frontmatter): Phase 3.3 distinguishes `Parsed`,
  `Absent` (not an error), and `Malformed` (indexed raw-only; UI banner in
  Phase 10).

## Current State Analysis

Phase 2 shipped the Rust server bootstrap and is fully in place:

- **Crate layout** at `skills/visualisation/visualise/server/`:
  `Cargo.toml` with pinned deps (axum 0.7, tokio 1, tower-http 0.5, serde,
  clap, tracing, tempfile, nix, anyhow, thiserror, libc); `[lib] test =
  false` so no unit-test runner against the library crate; binary at
  `src/main.rs`.
- **Modules** declared in `src/lib.rs`: `activity`, `config`, `lifecycle`,
  `server`, `shutdown`. The indexer, file driver, slug, frontmatter,
  templates, clusters, and api modules do not exist.
- **`Config`** at `src/config.rs` already deserialises the full `config.json`
  shape from Gap 2: `doc_paths: HashMap<String, PathBuf>` (9 keys),
  `templates: HashMap<String, TemplateTiers>` (5 names). Fields are
  populated by `scripts/write-visualiser-config.sh:38-72` unconditionally —
  every doc_type key is always present, every template always has the
  three-tier tuple. `deny_unknown_fields` guards the contract. Phase 3.8
  adds a `project_root: PathBuf` field (the repository root, distinct from
  `plugin_root`); `write-visualiser-config.sh` is extended to emit it.
- **`AppState`** at `src/server.rs:40-42` is `{ cfg: Arc<Config> }`.
  Minimal by design — Phase 3 extends it.
- **Router** is constructed inline inside `server::run` at `src/server.rs:87-96`:
  single `GET /` route behind an activity-middleware `route_layer`, then
  `RequestBodyLimitLayer`, `TimeoutLayer`, `host_header_guard`. `State` is
  bound last via `.with_state(state.clone())`.
- **Tests** live under `server/tests/*.rs`:
  `config_cli.rs` (exit-code contract), `config_contract.rs` (shell ↔ Rust
  JSON round-trip), `shutdown.rs` (signal handling + lifecycle files),
  `lifecycle_idle.rs` + `lifecycle_owner.rs` (idle timeout + owner-PID
  watch), plus `graceful_draining.rs` (draining across SIGTERM) introduced
  by Phase 2's review. Fixtures under `server/tests/fixtures/`:
  `config.valid.json`, `config.missing-required.json`,
  `config.optional-override-null.json`. No `meta/` fixture tree exists yet.
- **Integration-test pattern**: Phase 2 always binds a real port via
  `env!("CARGO_BIN_EXE_accelerator-visualiser")` + child process. No
  `tower::ServiceExt::oneshot` pattern is in use; the `tower` crate is not
  a direct dependency.
- **Launcher** (`scripts/launch-server.sh` + `scripts/write-visualiser-config.sh`)
  writes absolute paths for every `doc_paths` entry (via `abs_path()` helper
  at `write-visualiser-config.sh:34-36`). The Rust server never shells out;
  all configuration arrives as JSON.
- **Bash test-helpers** at `scripts/test-helpers.sh` include `assert_eq`,
  `assert_exit_code`, `make_fake_visualiser`, `reap_visualiser_fakes`,
  `assert_json_eq`. New bash harnesses auto-enrol via the glob runner in
  `tasks/test.py`; new cargo integration tests auto-enrol via
  `cargo test --tests`.
- **`invoke` test tasks** are organised into levels × components:
  `test.unit.visualiser` (cargo unit tests) and
  `test.integration.visualiser` (cargo `tests/*.rs` + bash harnesses) are
  both registered; adding new test files to either surface doesn't require
  task-file edits.
- **`mise.toml`** exposes `mise run test:unit`, `mise run test:integration`,
  `mise run test` (runs the levels in order).

### Key Discoveries

- **Existing `lifecycle.rs` is the server-lifecycle watch** (owner-PID +
  idle timeout), not document lifecycle. The Phase 3 slug-cluster module
  must not shadow this name. Phase 3.7 calls it `clusters.rs`; the wire
  type stays `LifecycleCluster` (spec-defined) and the HTTP routes stay
  `/api/lifecycle/*`.
- **The plan-review suffix regex must anchor `-review-\d+\.md$` at
  end-of-string.** The plan filename
  `2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md`
  contains `-review-` internally, so its plan-review companion
  (`2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md`)
  would incorrectly slug to `initialise-skill` under a greedy match,
  scattering that plan's lifecycle cluster. This specific file exists on
  disk today.
- **`ticket:` frontmatter appears as `null`, `""`, or is absent entirely.**
  21 of 28 live plans omit the key; 2 emit `null`; 2 emit `""`; none have
  a populated value. The indexer treats all three forms as "no ticket"
  uniformly — `Option<String>` with both `null` and `""` collapsing to
  `None` via a custom serde deserializer.
- **`meta/notes/` files are inconsistent** — 2 of 3 have no frontmatter at
  all, 1 has a full `date/author/tags/status` block. The parser must
  distinguish `FrontmatterState::Absent` (first line is not `---`) from
  `FrontmatterState::Malformed` (opens with `---` but YAML fails to
  parse). Only the latter emits a UI banner (Phase 10); `Absent` is normal.
- **`meta/specs/` exists on disk** with
  `2026-04-17-meta-visualisation-design.md` (the visualisation spec
  itself). The spec calls it out of scope for v1. Phase 3 does not add
  a `specs` key to `doc_paths` or to `DocTypeKey`; if a consumer repo
  adds `specs`, it is silently ignored by the indexer (not walked, not
  exposed via `/api/types`).
- **`plan-reviews` `target:` frontmatter is a repo-relative path string** —
  every one of the 8 live plan-reviews has `target: "meta/plans/<file>.md"`
  at line 5. Rendering this is Phase 9's concern; the indexer in Phase 3
  stores the raw frontmatter under `frontmatter` on the IndexEntry so
  Phase 9 has direct access without a re-parse.
- **`doc_paths` values are always absolute**; the `LocalFileDriver` must
  still canonicalise them (via `std::fs::canonicalize`) to resolve symlinks
  before the prefix check, otherwise a `/meta/plans/symlink-to-etc-passwd`
  could read through to an unconfigured root.
- **`.gitkeep` sentinels live in every empty doc-type directory** (per
  `/accelerator:init`'s `SKILL.md:34-44`). `LocalFileDriver::list` must
  filter for `*.md` exactly — listing everything would leak sentinels
  into the API response.
- **Missing doc-type directories are normal** on consumer repos today —
  `meta/validations/`, `meta/prs/`, `meta/reviews/prs/` are all empty
  (only `.gitkeep`); on a freshly-`/accelerator:init`-ed repo they still
  exist but contain only the sentinel. If a key is configured but the
  directory is missing entirely, `list` returns `Ok(vec![])` rather than
  an error — the config contract is the source of truth for what
  DocTypes exist, and absent-on-disk is distinct from absent-in-config.
- **MSRV is `1.85`** (Phase 2 bumped it from the originally-planned
  `1.80`). Native async trait methods (stable since 1.75) are available
  for static dispatch. However, traits used as `dyn Trait` (e.g.
  `FileDriver`) require manually desugared return types
  (`Pin<Box<dyn Future + Send + '_>>`) because RPITIT (`-> impl Trait`)
  methods are not dyn-compatible on any stable Rust release. Phase 3
  uses the manual desugaring on `FileDriver`; other traits that are
  only used with static dispatch use native async fn.
- **`tower = "0.5"` with the `util` feature** is required to use
  `ServiceExt::oneshot` in integration tests. axum 0.7's own public API
  does not re-export it. Added as a **dev-dependency** in Phase 3.1; no
  production code path uses it.
- **`gray_matter` v0.3** is the current release and has a YAML engine
  backed by a swappable `serde_yaml`/`serde_yml` parser. We use
  `serde_yml` (actively maintained) rather than `serde_yaml` (archived)
  as the YAML engine, configured via `gray_matter`'s generic parameter.
- **`http_body_util = "0.1"`** is needed alongside `tower` to collect
  response bodies in integration tests (axum 0.7 uses `http-body 1.0`).
  Dev-dep only.
- **SHA-256 ETag format is strong and hex-encoded**: `"sha256-<hex>"` with
  no `W/` prefix. Computed over the full file bytes via `sha2::Sha256`.
  Phase 3.5 caches it on the IndexEntry; Phase 4 will recompute on watcher
  events. Read handlers pull from the cache; they never hash on the
  request path.

## Desired End State

After this phase ships:

1. "`curl http://127.0.0.1:<port>/api/types` returns a JSON object
   `{ "types": [...] }` containing 10 `DocType` entries:"
   - 9 ordinary types (`decisions`, `tickets`, `plans`, `research`,
     `plan-reviews`, `pr-reviews`, `validations`, `notes`, `prs`), each
     with `dirPath` populated (absolute), `inLifecycle: true`, and
     `inKanban: true` only for `tickets`.
   - 1 virtual type (`templates`) with `dirPath: null`, `virtual: true`,
     `inLifecycle: false`, `inKanban: false`.
2. `curl 'http://127.0.0.1:<port>/api/docs?type=decisions'` on the live
   workspace returns a JSON object `{ "docs": [...] }` containing 21
   `IndexEntry` objects, each with `type: "decisions"`, `slug` matching
   the pattern `three-layer-review-architecture` (prefix stripped),
   `etag: "sha256-<hex>"`, `frontmatter: {...}`, and a `title` derived
   from frontmatter / first H1 / filename cascade.
3. `curl 'http://127.0.0.1:<port>/api/docs?type=plan-reviews'` returns
   a JSON object `{ "docs": [...] }` containing 8 `IndexEntry` objects,
   all with slugs stripped of both `YYYY-MM-DD-` prefix and
   `-review-\d+` suffix. The entry for
   `2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md`
   has slug `initialise-skill-and-review-pr-ephemeral-migration` (embedded
   `-review-` preserved).
4. `curl 'http://127.0.0.1:<port>/api/templates'` returns a JSON object
   `{ "templates": [...] }` containing 5 `TemplateSummary` entries for
   names `adr`, `plan`, `research`, `validation`, `pr-description`,
   each with three `tiers` entries in priority order
   (`config-override`, `user-override`, `plugin-default`), per-tier
   `present` booleans, and an `activeTier` value.
5. `curl -i http://127.0.0.1:<port>/api/docs/meta/decisions/ADR-0001-context-isolation-principles.md`
   returns 200 with a strong `ETag: "sha256-<hex>"` header and the raw
   markdown body. Re-requesting with `If-None-Match: "sha256-<the-same-hex>"`
   returns 304 with no body.
6. `curl http://127.0.0.1:<port>/api/templates/adr` returns a
   `TemplateDetail` with three `tiers` entries. Present tiers have
   `content` and `etag`; absent tiers have neither. The `plugin-default`
   tier is always present (baseline).
7. `curl http://127.0.0.1:<port>/api/lifecycle` returns a JSON object
   `{ "clusters": [...] }` containing `LifecycleCluster` entries grouped
   by slug. The cluster for slug
   `meta-visualiser-phase-2-server-bootstrap` contains at minimum the
   Phase 2 plan and its review; `completeness.hasPlan` and
   `completeness.hasPlanReview` are `true`.
8. `curl http://127.0.0.1:<port>/api/lifecycle/meta-visualiser-phase-2-server-bootstrap`
   returns the same single cluster.
9. Absent `meta/prs/` returns `Ok(vec![])` for `type=prs` (no crash).
10. Malformed frontmatter in a file (e.g. unclosed YAML block) produces
    an IndexEntry with `frontmatterState: "malformed"` and raw body
    accessible via `/api/docs/:path`; `doc-invalid` events are NOT
    emitted yet (that lands in Phase 4 alongside SSE).
11. Attempting to `GET /api/docs/../../etc/passwd` returns 403
    (path-escape guard).
12. All existing Phase 2 behaviours preserved: `GET /` still returns
    the placeholder; signal handling, shutdown flushes, lifecycle
    watches, and activity-driven idle timeout all unchanged.

### Verification

- `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml
  --lib` exits 0 (colocated unit tests).
- `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml
  --tests` exits 0 (integration tests under `server/tests/`).
- `cargo check --manifest-path skills/visualisation/visualise/server/Cargo.toml`
  exits 0.
- `mise run test:unit` exits 0.
- `mise run test:integration` exits 0 (runs every visualiser integration
  test including the new api_smoke.rs end-to-end test).
- `mise run test` exits 0 (runs unit then integration level in order).
- `ACCELERATOR_VISUALISER_BIN=$(pwd)/skills/visualisation/visualise/server/target/release/accelerator-visualiser
  skills/visualisation/visualise/scripts/launch-server.sh` prints a
  `**Visualiser URL**: http://127.0.0.1:<port>` line; `curl -f
  <url>/api/types` returns a JSON object with a 10-entry `types` array;
  `curl -f '<url>/api/docs?type=decisions'` returns a JSON object with
  the live 21 ADRs in a `docs` array; `curl -f '<url>/api/lifecycle'`
  returns a JSON object with a non-empty `clusters` array.
- `skills/visualisation/visualise/scripts/stop-server.sh` shuts down
  cleanly as before (no Phase 3 change required).

## What We're NOT Doing

Explicitly out of scope for Phase 3.

- **No file watcher** — Phase 4. The `notify` crate is NOT added in this
  phase. The Indexer scans once at server startup; it exposes a manual
  `rescan()` method for tests but never calls it on its own after
  construction. As a result, the in-memory index is **stale with respect
  to on-disk changes** after startup — a known limitation that Phase 4
  fixes. The HTTP API never re-reads files on request path; ETags and
  entries reflect startup-time state until server restart.
- **No SSE** — Phase 4. `GET /api/events` does not exist. The frontend
  query layer (Phase 5) works without live updates by polling on focus
  (acceptable until Phase 4 lands).
- **No writes** — Phase 8. `PATCH /api/docs/.../frontmatter` is not
  routed; the `FileDriver` trait does not have a `write_frontmatter`
  method in Phase 3 (it gains one in Phase 8). The route table rejects
  PATCH with a default 405 via axum's method-router behaviour.
- **No cross-reference rendering, no `[[ADR-NNNN]]`, no declared-link
  bidirectional index** — Phase 9. The indexer in Phase 3 builds the ID
  lookup tables (`adr_id` → IndexEntry, ticket-number → IndexEntry)
  because the computation is a natural extension of the scan, but the
  markdown renderer that consumes them is not added yet.
- **No frontend, no `GET /*` SPA fallback, no `rust-embed`** — Phase 5.
  The placeholder `GET /` route from Phase 2 is preserved unchanged.
- **No `doc-invalid` event emission** — Phase 4. Malformed frontmatter
  is recorded on the IndexEntry (`frontmatterState: "malformed"`) and
  visible via `/api/docs/:path`, but nothing downstream consumes it yet.
- **No performance tuning beyond the 2000-file target** — the spec's
  non-functional requirement is "initial scan of ~2000 files in under
  1s". Phase 3 meets this without exotic techniques (serial scan with
  `tokio::fs` reads, cached ETag per file). A parallel scan via
  `tokio::task::JoinSet` or a `rayon` thread pool is deferred to Phase
  10 polish if profiling shows a need.
- **No `specs` DocType** — out of scope per spec; revisit in v2.
- **No recursion into sub-directories** — every configured doc-type
  path is a **flat walk**. `meta/reviews/plans/sub/foo.md` would be
  ignored. The two-type review split (D5) means the `review_plans`
  and `review_prs` keys each point at their own flat directory; the
  `meta/reviews/` parent directory is never walked as a unit.
- **No template orphan detection or "unregistered" badge** — the
  research's D9 calls this out as a UI affordance for files placed in
  tier 2 without a tier 3 peer. Phase 3 ignores orphans (the template
  name list is authoritative from `config.templates`, which
  `write-visualiser-config.sh` derives from tier 3); the "unregistered"
  badge lands alongside the library templates view in Phase 5.
- **No content compression on disk reads** — files are read and cached
  as uncompressed `Vec<u8>`. For a 2000-file corpus averaging 10 KB
  each, memory budget is ~20 MB — acceptable for a local companion
  process. Large-repo optimisations are deferred.
- **No cache invalidation beyond full rescan** — if `Indexer::rescan`
  is called, it rebuilds the entire index. No per-path invalidation
  API. Phase 4's watcher drives the fine-grained path.
- **No `X-Accelerator-Visualiser` response header** — Phase 10.
- **No init-sentinel check** (existence of `<meta/tmp>/.gitignore`) —
  Phase 10.
- **No `test-config.sh` invariant updates** — adding API routes does
  not add a new context-injection skill, so the Phase 1.4 `CONTEXT_SKILLS`
  arrays are untouched.
- **No plugin version bump, no CHANGELOG entry** — feature work, not a
  release event.
- **No changes to `launch-server.sh` or `stop-server.sh`**.
  `write-visualiser-config.sh` gains a single new `project_root` emission
  (the repo root resolved via `find_repo_root` from `vcs-common.sh`); all
  other `config.json` fields are unchanged from Phase 2. Phase 4 is the
  next phase that will extend `config.json` further (watcher knobs).
- **No `dev-frontend` / `embed-dist` Cargo features** — Phase 5.

## Implementation Approach

Ten sub-phases, each depending only on earlier ones:

1. **Crate dependencies and wire types.** Add `sha2`, `gray_matter`,
   `serde_yml` as runtime deps; `tower` (util) and `http_body_util`
   as dev-deps. Introduce `src/docs.rs` with
   the `DocTypeKey` enum and the `DocType` wire struct, plus a
   `describe_types(&Config)` helper. TDD with serde round-trip tests.
2. **Slug derivation.** `src/slug.rs` — pure functions, per-DocType hand
   parsing (no `regex` dep). Table-driven tests including the critical
   `-review-` embedded-slug case. Red → Green → Refactor in a distinct
   `jj` commit per phase.
3. **Frontmatter parsing.** `src/frontmatter.rs` — three-state parser
   (`Parsed`, `Absent`, `Malformed`) with a title-fallback cascade.
   Uses `gray_matter` under the hood. Fixture-driven tests including
   live real-data edge cases (YAML block sequences, unquoted colons).
4. **FileDriver trait and LocalFileDriver.** `src/file_driver.rs` —
   trait with `list(kind)` and `read(path)`; `LocalFileDriver` impl
   with canonicalise + prefix-check safety. No `watch()` method in
   Phase 3 (avoids shipping a trait method that no implementation
   would use until Phase 4). Symlink-escape tests.
5. **Indexer.** `src/indexer.rs` — `IndexEntry` wire struct,
   `Indexer::build` (async scan over FileDriver), `rescan()`, per-type
   lookup, ID-lookup tables (`adr_id`, ticket number). SHA-256 ETag
   computed during scan; frontmatter state captured per entry.
6. **Templates virtual DocType.** `src/templates.rs` —
   `TemplateResolver` consuming `config.templates`, exposing `list()`
   (`TemplateSummary[]`) and `detail(name)` (`TemplateDetail`). Per-tier
   presence, active-tier winner. No watcher integration yet.
7. **Cluster computation.** `src/clusters.rs` — `LifecycleCluster` wire
   struct, `compute_clusters(&Indexer) -> Vec<LifecycleCluster>`,
   canonical timeline ordering (ticket → research → plan → plan-review
   → validation → PR → pr-review → decision → notes), completeness
   flags.
8. **AppState composition and router builder.** Extend `AppState` with
   `indexer`, `templates`, `clusters`; extract `build_router(state)`
   helper. `server::run` delegates. Introduce `ApiError` enum and its
   `IntoResponse` impl for consistent 400/403/404/500 responses.
9. **API routes.** `src/api/mod.rs` + per-endpoint files (`types.rs`,
   `docs.rs`, `templates.rs`, `lifecycle.rs`). Uses `tower::ServiceExt::oneshot`
   as the primary integration-test harness (introduced in this phase
   as the canonical route-level test pattern). Strong ETag emission,
   `If-None-Match` → 304, path-escape guard on `{*path}`.
10. **End-to-end fixture tree and smoke test.** Commit
    `server/tests/fixtures/meta/` with 3–5 docs per type covering
    absent-FM, malformed-FM, and review-suffix edge cases. New
    `server/tests/api_smoke.rs` spawns the real binary against the
    fixture config and asserts the full API surface via `reqwest`.

TDD discipline per sub-phase (unchanged from Phase 2):

- **Red**: add a test (unit or integration) that fails because the code
  doesn't exist or has a wrong value. Commit in a distinct `jj` revision
  where practical (most unit tests can; integration tests that need the
  type to compile usually cannot).
- **Green**: implement the minimum to make tests pass.
- **Refactor**: clean up only if the implementation is awkward.

Where tests and implementation land in the same commit, the sub-phase's
Manual Verification calls out a mutation smoke test.

---

## Phase 3.1: Crate dependencies and wire types

### Overview

Add the crates required by later sub-phases and introduce the public
`DocType` / `DocTypeKey` surface. This is the smallest possible sub-phase
that still exercises the TDD cycle: a serde round-trip test for the enum
lands red, the implementation follows. The `describe_types(&Config)`
helper is stubbed to return an empty `Vec` so that Phase 3.9's route
handler has something to call today; it is fully populated in 3.8 when
the `virtual` flag and `inKanban` / `inLifecycle` booleans are wired.

### Changes Required

#### 1. Cargo manifest

**File**: `skills/visualisation/visualise/server/Cargo.toml`
**Changes**: Extend `[dependencies]` and `[dev-dependencies]`.

```toml
[dependencies]
# (existing entries unchanged)
axum = { version = "0.7", default-features = false, features = ["http1", "tokio"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal", "sync", "time", "net", "fs"] }
tower-http = { version = "0.5", features = ["trace", "limit", "timeout"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tempfile = "3"
nix = { version = "0.28", features = ["signal", "process"] }
anyhow = "1"
thiserror = "1"
libc = "0.2"
# New in Phase 3:
sha2 = "0.10"
gray_matter = { version = "0.3", default-features = false, features = ["yaml"] }
serde_yml = "0.0.12"
hex = "0.4"

[dev-dependencies]
# (existing entries unchanged)
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time", "process"] }
assert_cmd = "2"
predicates = "3"
# New in Phase 3:
tower = { version = "0.5", features = ["util"] }
http_body_util = "0.1"
```

Notes:

- `sha2` supplies `Sha256`; `hex` gives the hex encoding for the ETag
  string. Keeping the hex encoding as a separate crate avoids pulling
  in `base16ct` or rolling a tiny encoder.
- `gray_matter` v0.3 with `default-features = false` and an explicit
  `yaml` feature keeps the dependency surface minimal; the alternatives
  (JSON, TOML) are not needed.
- `serde_yml` replaces the archived `serde_yaml`. `gray_matter`'s
  generic `Matter<Engine>` API lets us plug it in; see
  `src/frontmatter.rs` in Phase 3.3.
- `tower` with `util` feature provides `ServiceExt::oneshot` used by
  Phase 3.9's integration tests. It is **dev-dep only** — the production
  code path uses `axum::serve` exclusively.
- `http_body_util` v0.1 gives `BodyExt::collect().await` for response
  body reads in tests (axum 0.7 emits `Body` over `http-body 1.0`).

#### 2. Declare new modules in `lib.rs`

**File**: `skills/visualisation/visualise/server/src/lib.rs`
**Changes**: Add `pub mod` lines for modules added across 3.1–3.9.
Modules added in later sub-phases are declared as empty (`pub mod X;`
resolving to `src/X.rs`) — this keeps each sub-phase's change set small
while the overall module layout lands once.

```rust
//! Meta visualiser server — library crate.
//!
//! The binary (`src/main.rs`) is a thin entry point; all logic
//! lives in the modules declared here. Integration tests under
//! `server/tests/*.rs` consume these modules directly.

pub mod activity;
pub mod api;          // 3.8 (scaffold) / 3.9 (populated)
pub mod clusters;     // 3.7
pub mod config;
pub mod docs;         // 3.1
pub mod file_driver;  // 3.4
pub mod frontmatter;  // 3.3
pub mod indexer;      // 3.5
pub mod lifecycle;
pub mod server;
pub mod shutdown;
pub mod slug;         // 3.2
pub mod templates;    // 3.6
```

Per sub-phase, the referenced file is created empty (`//! Placeholder
for Phase 3.N` doc-comment only) and populated when the sub-phase
lands. `cargo check` stays green throughout because each module's
content is syntactically valid at every step.

#### 3. `docs` module — DocTypeKey and DocType

**File**: `skills/visualisation/visualise/server/src/docs.rs`
**Changes**: New file.

```rust
//! Wire-format types describing the visualiser's DocType surface.
//!
//! `DocTypeKey` enumerates the ten types the visualiser exposes:
//! nine that walk a directory on disk plus one (`templates`) that
//! resolves across three precedence tiers — see `templates.rs`.
//! `DocType` is the shape returned by `GET /api/types`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The ten DocType keys exposed by the visualiser.
///
/// Wire format is kebab-case (`plan-reviews`, `pr-reviews`). Rust-side
/// enum names are CamelCase. The `templates` variant is a *virtual*
/// DocType — it does not correspond to a single directory — and is
/// handled specially throughout the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocTypeKey {
    Decisions,
    Tickets,
    Plans,
    Research,
    PlanReviews,
    PrReviews,
    Validations,
    Notes,
    Prs,
    Templates,
}

impl DocTypeKey {
    pub fn all() -> [DocTypeKey; 10] {
        [
            DocTypeKey::Decisions,
            DocTypeKey::Tickets,
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::PlanReviews,
            DocTypeKey::PrReviews,
            DocTypeKey::Validations,
            DocTypeKey::Notes,
            DocTypeKey::Prs,
            DocTypeKey::Templates,
        ]
    }

    /// The config-side key used in `config.doc_paths`. `Templates` has
    /// no single config path, so it returns `None`.
    pub fn config_path_key(self) -> Option<&'static str> {
        match self {
            DocTypeKey::Decisions => Some("decisions"),
            DocTypeKey::Tickets => Some("tickets"),
            DocTypeKey::Plans => Some("plans"),
            DocTypeKey::Research => Some("research"),
            DocTypeKey::PlanReviews => Some("review_plans"),
            DocTypeKey::PrReviews => Some("review_prs"),
            DocTypeKey::Validations => Some("validations"),
            DocTypeKey::Notes => Some("notes"),
            DocTypeKey::Prs => Some("prs"),
            DocTypeKey::Templates => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DocTypeKey::Decisions => "Decisions",
            DocTypeKey::Tickets => "Tickets",
            DocTypeKey::Plans => "Plans",
            DocTypeKey::Research => "Research",
            DocTypeKey::PlanReviews => "Plan reviews",
            DocTypeKey::PrReviews => "PR reviews",
            DocTypeKey::Validations => "Validations",
            DocTypeKey::Notes => "Notes",
            DocTypeKey::Prs => "PRs",
            DocTypeKey::Templates => "Templates",
        }
    }

    pub fn in_lifecycle(self) -> bool {
        !matches!(self, DocTypeKey::Templates)
    }

    pub fn in_kanban(self) -> bool {
        matches!(self, DocTypeKey::Tickets)
    }

    pub fn is_virtual(self) -> bool {
        matches!(self, DocTypeKey::Templates)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    pub key: DocTypeKey,
    pub label: String,
    /// Absolute resolved path. `None` for the virtual `templates`
    /// type.
    pub dir_path: Option<PathBuf>,
    pub in_lifecycle: bool,
    pub in_kanban: bool,
    /// Serialised only when `true` (per spec's `virtual?` optional
    /// field). Rust-side convention: `serialize_if_true` via
    /// `skip_serializing_if`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub r#virtual: bool,
}

/// Build the full DocType list from the runtime config. `templates`
/// always comes last and has `dir_path: None`, `virtual: true`.
pub fn describe_types(cfg: &crate::config::Config) -> Vec<DocType> {
    let mut out = Vec::with_capacity(DocTypeKey::all().len());
    for key in DocTypeKey::all() {
        let dir_path = key
            .config_path_key()
            .and_then(|k| cfg.doc_paths.get(k).cloned());
        out.push(DocType {
            key,
            label: key.label().to_string(),
            dir_path,
            in_lifecycle: key.in_lifecycle(),
            in_kanban: key.in_kanban(),
            r#virtual: key.is_virtual(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case_round_trip_covers_every_variant() {
        // Red first: assert every variant serialises to the expected
        // kebab-case string and round-trips back to the variant.
        let pairs = [
            (DocTypeKey::Decisions, "decisions"),
            (DocTypeKey::Tickets, "tickets"),
            (DocTypeKey::Plans, "plans"),
            (DocTypeKey::Research, "research"),
            (DocTypeKey::PlanReviews, "plan-reviews"),
            (DocTypeKey::PrReviews, "pr-reviews"),
            (DocTypeKey::Validations, "validations"),
            (DocTypeKey::Notes, "notes"),
            (DocTypeKey::Prs, "prs"),
            (DocTypeKey::Templates, "templates"),
        ];
        for (variant, wire) in pairs {
            let ser = serde_json::to_string(&variant).unwrap();
            assert_eq!(ser, format!("\"{wire}\""));
            let de: DocTypeKey = serde_json::from_str(&ser).unwrap();
            assert_eq!(de, variant);
        }
    }

    #[test]
    fn all_returns_every_variant_exactly_once() {
        let mut v = DocTypeKey::all().to_vec();
        v.sort_by_key(|k| k.label().to_string());
        v.dedup();
        assert_eq!(v.len(), 10, "DocTypeKey::all must return 10 distinct variants");
    }

    #[test]
    fn templates_is_virtual_and_out_of_lifecycle() {
        assert!(DocTypeKey::Templates.is_virtual());
        assert!(!DocTypeKey::Templates.in_lifecycle());
        assert!(!DocTypeKey::Templates.in_kanban());
    }

    #[test]
    fn tickets_is_the_only_kanban_type() {
        for k in DocTypeKey::all() {
            assert_eq!(
                k.in_kanban(),
                matches!(k, DocTypeKey::Tickets),
                "in_kanban mismatch for {k:?}",
            );
        }
    }

    #[test]
    fn describe_types_populates_dir_paths_from_config() {
        let mut doc_paths = std::collections::HashMap::new();
        doc_paths.insert("decisions".into(), PathBuf::from("/abs/decisions"));
        doc_paths.insert("review_plans".into(), PathBuf::from("/abs/reviews/plans"));

        let cfg = crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths,
            templates: Default::default(),
        };

        let types = describe_types(&cfg);
        assert_eq!(types.len(), 10);
        let decisions = types.iter().find(|t| t.key == DocTypeKey::Decisions).unwrap();
        assert_eq!(decisions.dir_path.as_deref(), Some(std::path::Path::new("/abs/decisions")));
        let plan_reviews = types.iter().find(|t| t.key == DocTypeKey::PlanReviews).unwrap();
        assert_eq!(plan_reviews.dir_path.as_deref(), Some(std::path::Path::new("/abs/reviews/plans")));
        let templates = types.iter().find(|t| t.key == DocTypeKey::Templates).unwrap();
        assert!(templates.dir_path.is_none());
        assert!(templates.r#virtual);
    }

    #[test]
    fn virtual_flag_omitted_when_false_in_json() {
        let cfg = crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths: Default::default(),
            templates: Default::default(),
        };
        let types = describe_types(&cfg);
        let decisions = types.iter().find(|t| t.key == DocTypeKey::Decisions).unwrap();
        let json = serde_json::to_value(decisions).unwrap();
        assert!(json.get("virtual").is_none(), "virtual must be omitted when false");
        let templates = types.iter().find(|t| t.key == DocTypeKey::Templates).unwrap();
        let json = serde_json::to_value(templates).unwrap();
        assert_eq!(json.get("virtual"), Some(&serde_json::Value::Bool(true)));
    }
}
```

#### 4. Empty-module placeholders

The following files are created as one-line doc-comment stubs so that
`lib.rs`'s `pub mod` declarations compile cleanly. Each is populated
in its owning sub-phase.

**File**: `skills/visualisation/visualise/server/src/slug.rs`
```rust
//! Placeholder for Phase 3.2 (slug derivation).
```

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
```rust
//! Placeholder for Phase 3.3 (frontmatter parsing).
```

**File**: `skills/visualisation/visualise/server/src/file_driver.rs`
```rust
//! Placeholder for Phase 3.4 (FileDriver trait).
```

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
```rust
//! Placeholder for Phase 3.5 (Indexer).
```

**File**: `skills/visualisation/visualise/server/src/templates.rs`
```rust
//! Placeholder for Phase 3.6 (Templates virtual DocType).
```

**File**: `skills/visualisation/visualise/server/src/clusters.rs`
```rust
//! Placeholder for Phase 3.7 (Cluster computation).
```

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`
```rust
//! Placeholder for Phase 3.8/3.9 (API routes).
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path skills/visualisation/visualise/server/Cargo.toml` exits 0.
- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib docs::tests` exits 0 with 6 tests passing.
- [ ] `mise run test:unit` exits 0.
- [ ] `mise run test:integration` exits 0 (unchanged from Phase 2 — no new integration tests).
- [ ] `mise run test` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: `jj log -T 'commit_id.short() ++ " " ++ description.first_line()' -r 'ancestors(@)' | head -5` shows the test-file commit preceding the implementation commit for `docs.rs` (two distinct revisions).
- [ ] **Mutation smoke test**: temporarily change `DocTypeKey::PlanReviews`'s serde variant name to `"planreviews"` (removing the hyphen), run `cargo test --lib docs::tests`, observe `kebab_case_round_trip_covers_every_variant` fails; restore.
- [ ] `grep -F 'sha2 = "0.10"' skills/visualisation/visualise/server/Cargo.toml` matches.
- [ ] `grep -F 'gray_matter' skills/visualisation/visualise/server/Cargo.toml` matches.
- [ ] `grep -F 'tower' skills/visualisation/visualise/server/Cargo.toml` matches (in dev-deps).

---

## Phase 3.2: Slug derivation

### Overview

Implement `slug::derive(kind, filename)` as a pure function with
per-DocType parsing. Tests land first — a table-driven suite over
every live filename pattern plus the three regression cases flagged
in the research (`-review-` embedded internally, ticket without
numeric prefix, plan-review without suffix). No `regex` crate
dependency: the patterns are simple enough for hand-rolled parsing,
and keeping the dep list small matters for release-binary size.

### Changes Required

#### 1. Slug module

**File**: `skills/visualisation/visualise/server/src/slug.rs`
**Changes**: Full replacement.

```rust
//! Pure slug derivation per DocType.
//!
//! Deterministic: for every filename pattern shipped by an authoring
//! skill, `derive(kind, filename)` returns the canonical slug used
//! for cluster grouping. Unknown patterns return `None` — the caller
//! records the DocType entry but it does not participate in lifecycle
//! clustering.

use crate::docs::DocTypeKey;

/// Derive the cluster slug for a given filename under a given DocType.
///
/// Returns `None` for:
/// - Templates (which do not cluster).
/// - Filenames that don't match the DocType's pattern (e.g. an
///   editor backup `*.md~`).
pub fn derive(kind: DocTypeKey, filename: &str) -> Option<String> {
    if !filename.ends_with(".md") {
        return None;
    }
    let stem = &filename[..filename.len() - 3]; // strip ".md"

    match kind {
        DocTypeKey::Decisions => strip_prefix_numbered(stem, "ADR-"),
        DocTypeKey::Tickets => strip_prefix_ticket_number(stem),
        DocTypeKey::Plans
        | DocTypeKey::Research
        | DocTypeKey::Validations
        | DocTypeKey::Notes
        | DocTypeKey::Prs => strip_prefix_date(stem),
        DocTypeKey::PlanReviews | DocTypeKey::PrReviews => {
            let without_date = strip_prefix_date(stem)?;
            strip_suffix_review_n(&without_date)
        }
        DocTypeKey::Templates => None,
    }
}

/// Strip a `<prefix>NNNN-` head (e.g. `ADR-0002-foo` → `foo`).
fn strip_prefix_numbered(stem: &str, prefix: &str) -> Option<String> {
    let rest = stem.strip_prefix(prefix)?;
    let dash = rest.find('-')?;
    let (digits, tail) = rest.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // `tail` begins with '-'; skip it.
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
}

/// Strip a `NNNN-` head (ticket numbering).
fn strip_prefix_ticket_number(stem: &str) -> Option<String> {
    let dash = stem.find('-')?;
    let (digits, tail) = stem.split_at(dash);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
}

/// Strip a `YYYY-MM-DD-` head.
fn strip_prefix_date(stem: &str) -> Option<String> {
    // Exactly 11 leading chars must match `YYYY-MM-DD-`.
    if stem.len() < 11 {
        return None;
    }
    let (head, tail) = stem.split_at(10);
    let bytes = head.as_bytes();
    let ok = bytes.len() == 10
        && bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit());
    if !ok {
        return None;
    }
    if !tail.starts_with('-') {
        return None;
    }
    Some(tail[1..].to_string()).filter(|s| !s.is_empty())
}

/// Strip a trailing `-review-\d+` suffix. Anchored at end-of-string
/// so a slug that itself contains `-review-` internally is preserved.
fn strip_suffix_review_n(stem: &str) -> Option<String> {
    // Find the LAST occurrence of "-review-" and verify the remainder
    // is all-digits. Any non-digit after it (or no digits at all)
    // disqualifies.
    let idx = stem.rfind("-review-")?;
    let (head, tail) = stem.split_at(idx);
    let digits = &tail["-review-".len()..];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(head.to_string()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions_strip_adr_prefix() {
        let cases = &[
            ("ADR-0001-context-isolation-principles.md", Some("context-isolation-principles")),
            ("ADR-0017-configuration-extension-points.md", Some("configuration-extension-points")),
            // Four-digit prefix not required — any run of digits works.
            ("ADR-12-foo.md", Some("foo")),
            // Missing ADR- prefix.
            ("0001-context.md", None),
            // Non-digit in what should be the number.
            ("ADR-ABCD-foo.md", None),
            // No slug.
            ("ADR-0001-.md", None),
            ("ADR-0001.md", None),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::Decisions, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn tickets_strip_numeric_prefix() {
        let cases = &[
            ("0001-three-layer-review-system-architecture.md", Some("three-layer-review-system-architecture")),
            ("0029-template-management-subcommand-surface.md", Some("template-management-subcommand-surface")),
            ("1-short.md", Some("short")),
            // Missing digits.
            ("abc-foo.md", None),
            ("0001.md", None),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::Tickets, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    #[test]
    fn dated_types_strip_iso_date() {
        // Plans, research, notes, prs, validations all share this pattern.
        for kind in [
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::Notes,
            DocTypeKey::Prs,
            DocTypeKey::Validations,
        ] {
            let cases = &[
                ("2026-04-17-pr-review-agents.md", Some("pr-review-agents")),
                ("2026-02-22-pr-review-agents-design.md", Some("pr-review-agents-design")),
                // Not a valid date.
                ("20260417-foo.md", None),
                ("2026-4-17-foo.md", None),
                // No slug after date.
                ("2026-04-17-.md", None),
                ("2026-04-17.md", None),
                // Non-md.
                ("2026-04-17-foo.txt", None),
            ];
            for (input, expected) in cases {
                let got = derive(kind, input);
                assert_eq!(got.as_deref(), *expected, "{kind:?} input={input}");
            }
        }
    }

    #[test]
    fn plan_reviews_strip_date_and_review_n_suffix() {
        let cases = &[
            (
                "2026-04-18-meta-visualiser-phase-2-server-bootstrap-review-1.md",
                Some("meta-visualiser-phase-2-server-bootstrap"),
            ),
            (
                "2026-03-29-template-management-subcommands-review-1.md",
                Some("template-management-subcommands"),
            ),
            // Multi-digit review number.
            (
                "2026-04-18-foo-review-12.md",
                Some("foo"),
            ),
        ];
        for (input, expected) in cases {
            let got = derive(DocTypeKey::PlanReviews, input);
            assert_eq!(got.as_deref(), *expected, "input={input}");
        }
    }

    /// Regression test for the critical edge case flagged in research
    /// G3: a plan slug that itself contains `-review-` internally.
    /// The suffix strip must be anchored at end-of-string via
    /// `rfind("-review-")`, not the first occurrence.
    #[test]
    fn plan_review_preserves_internal_review_literal() {
        let input =
            "2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md";
        let got = derive(DocTypeKey::PlanReviews, input);
        assert_eq!(
            got.as_deref(),
            Some("initialise-skill-and-review-pr-ephemeral-migration"),
            "internal -review- must be preserved; only the trailing -review-N suffix strips",
        );
    }

    #[test]
    fn plan_review_without_suffix_returns_none() {
        // A plan-review file missing the -review-N suffix is malformed
        // for clustering purposes; return None.
        let got = derive(DocTypeKey::PlanReviews, "2026-04-18-meta-visualiser-phase-2-server-bootstrap.md");
        assert_eq!(got, None);
    }

    #[test]
    fn plan_review_with_non_numeric_suffix_returns_none() {
        // "-review-latest" doesn't satisfy the \d+ requirement.
        let got = derive(DocTypeKey::PlanReviews, "2026-04-18-foo-review-latest.md");
        assert_eq!(got, None);
    }

    #[test]
    fn pr_reviews_use_same_pattern_as_plan_reviews() {
        // Symmetry with plan-reviews: spec splits the two types for
        // indexing, but slug derivation is identical.
        let input = "2026-04-20-sample-pr-review-3.md";
        assert_eq!(
            derive(DocTypeKey::PrReviews, input).as_deref(),
            Some("sample-pr"),
            "the `-review-3` suffix strips; the prior `-review` segment (here, the last) does too",
        );
        // More realistic PR description slug:
        let input = "2026-04-20-respond-to-user-feedback-review-1.md";
        assert_eq!(
            derive(DocTypeKey::PrReviews, input).as_deref(),
            Some("respond-to-user-feedback"),
        );
    }

    #[test]
    fn templates_always_return_none() {
        for name in &["adr.md", "plan.md", "research.md", "validation.md", "pr-description.md"] {
            assert_eq!(derive(DocTypeKey::Templates, name), None);
        }
    }

    #[test]
    fn non_md_files_return_none_for_every_type() {
        for kind in DocTypeKey::all() {
            assert_eq!(derive(kind, "foo.txt"), None, "{kind:?}");
            assert_eq!(derive(kind, "README.rst"), None, "{kind:?}");
        }
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib slug::tests` exits 0 with ≥9 tests passing.
- [ ] `mise run test:unit` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: the Phase 3.2 commits show a red-first revision — the test file compiled against the Phase 3.1 `DocTypeKey` but all assertions fail (because `derive` returns `None` for every variant in the stub).
- [ ] **Mutation smoke test**: temporarily replace `stem.rfind("-review-")` with `stem.find("-review-")` in `strip_suffix_review_n`, run `cargo test --lib slug::tests`, observe `plan_review_preserves_internal_review_literal` fails; restore. Confirms the regression test guards what it claims.
- [ ] `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml -- -D warnings` exits 0 (no regressions from the hand-rolled parsing).

---

## Phase 3.3: Frontmatter parsing with three-state tolerance

### Overview

Introduce `frontmatter.rs` — a pure parser over raw markdown bytes
that emits one of three states: `Parsed(mapping)` when a valid YAML
block is found between two `---` fences, `Absent` when there is no
leading `---` fence (first-class case, not an error), and
`Malformed` when a leading fence exists but the YAML body fails to
parse. Title derivation (`frontmatter.title` → first H1 → filename)
is exposed as a separate function so the indexer can call it
uniformly in Phase 3.5.

### Changes Required

#### 1. Frontmatter module

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: Full replacement.

```rust
//! Three-state frontmatter parser over raw markdown bytes.
//!
//! Distinguishes:
//! - `Parsed(map)`: a valid YAML mapping appears between two `---`
//!   fences at the top of the file.
//! - `Absent`: the file does not begin with `---`. This is a normal
//!   case (older plans, bare notes) and does not trigger a UI banner.
//! - `Malformed`: the file begins with `---` but YAML parsing fails.
//!   Indexed raw-only; a `doc-invalid` event is emitted in Phase 4.
//!
//! The parser is pure — no IO, no allocation beyond the returned
//! value. Wire-format mappings are exposed as
//! `BTreeMap<String, serde_json::Value>` so the caller can preserve
//! any YAML scalar type the frontmatter declares, and downstream
//! serialisation to JSON is lossless.

use std::collections::BTreeMap;

/// State of a document's leading YAML frontmatter block.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterState {
    Parsed(BTreeMap<String, serde_json::Value>),
    Absent,
    Malformed,
}

impl FrontmatterState {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrontmatterState::Parsed(_) => "parsed",
            FrontmatterState::Absent => "absent",
            FrontmatterState::Malformed => "malformed",
        }
    }
}

/// Parsed result: state plus the body (markdown after the optional
/// frontmatter block and its trailing newline).
#[derive(Debug, Clone)]
pub struct Parsed {
    pub state: FrontmatterState,
    pub body: String,
}

/// Parse a markdown document's leading frontmatter.
///
/// Contract:
/// - If the file starts with a line containing exactly `---`
///   (optionally followed by CR/LF), the content up to the next
///   `---` line is treated as a YAML mapping.
/// - If no closing fence is found within the first 1 MiB of the
///   file, the state is `Malformed` (guards against a pathological
///   "opens with `---` but the closing fence is megabytes away"
///   file).
/// - If the YAML parses but is not a mapping (e.g. a list or a
///   scalar), the state is `Malformed` — the spec's DocType model
///   assumes key-value frontmatter.
pub fn parse(raw: &[u8]) -> Parsed {
    // Decode as UTF-8; invalid UTF-8 falls back to
    // `String::from_utf8_lossy` and is still attempted.
    let s = match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(raw).into_owned(),
    };

    // Absent: does not start with `---` on its own line.
    let first_line_end = s.find('\n').unwrap_or(s.len());
    let first_line = s[..first_line_end].trim_end_matches('\r');
    if first_line != "---" {
        return Parsed { state: FrontmatterState::Absent, body: s };
    }

    // Find the closing `---` fence. Must start at column 0 of a new
    // line. Scan only the first 1 MiB to bound pathological input.
    const MAX_SCAN: usize = 1 << 20;
    let scan_end = s.len().min(MAX_SCAN);
    let after_first = first_line_end + 1;
    if after_first >= s.len() {
        return Parsed { state: FrontmatterState::Malformed, body: s };
    }

    let mut close_at: Option<usize> = None;
    let mut pos = after_first;
    while pos < scan_end {
        // Line starts at `pos`. Find its end.
        let line_end = s[pos..].find('\n').map(|n| pos + n).unwrap_or(s.len());
        let line = s[pos..line_end].trim_end_matches('\r');
        if line == "---" {
            close_at = Some(line_end);
            break;
        }
        pos = line_end + 1;
    }

    let close = match close_at {
        Some(c) => c,
        None => return Parsed { state: FrontmatterState::Malformed, body: s },
    };

    // YAML body is everything between the fences, excluding the
    // fence lines themselves.
    let yaml_start = first_line_end + 1;
    let yaml_end = s[..close]
        .rfind('\n')
        .map(|n| n + 1)
        .unwrap_or(yaml_start);
    let yaml_src = &s[yaml_start..yaml_end.saturating_sub(1).max(yaml_start)];
    let body_start = (close + 1).min(s.len());
    // Skip exactly one trailing newline after the closing fence.
    let body = s[body_start..].trim_start_matches('\n').to_string();

    // Parse YAML into serde_yml::Value, then convert to serde_json::Value
    // for stable downstream serialisation.
    let value: serde_yml::Value = match serde_yml::from_str(yaml_src) {
        Ok(v) => v,
        Err(_) => return Parsed { state: FrontmatterState::Malformed, body },
    };

    let mapping = match value {
        serde_yml::Value::Mapping(m) => m,
        // Empty frontmatter (`---\n---`) parses to Value::Null.
        // Treat as an empty mapping — first-class "Parsed" state with
        // no fields — so the indexer doesn't flag a UI banner.
        serde_yml::Value::Null => serde_yml::Mapping::new(),
        _ => return Parsed { state: FrontmatterState::Malformed, body },
    };

    let mut out: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (k, v) in mapping {
        let key = match k {
            serde_yml::Value::String(s) => s,
            other => match serde_yml::to_string(&other) {
                Ok(s) => s.trim().to_string(),
                Err(_) => return Parsed { state: FrontmatterState::Malformed, body },
            },
        };
        let json_val = match yml_to_json(&v) {
            Some(v) => v,
            None => return Parsed { state: FrontmatterState::Malformed, body },
        };
        out.insert(key, json_val);
    }

    Parsed { state: FrontmatterState::Parsed(out), body }
}

fn yml_to_json(v: &serde_yml::Value) -> Option<serde_json::Value> {
    use serde_json::Value as J;
    Some(match v {
        serde_yml::Value::Null => J::Null,
        serde_yml::Value::Bool(b) => J::Bool(*b),
        serde_yml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                J::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                J::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f).map(J::Number).unwrap_or(J::Null)
            } else {
                J::Null
            }
        }
        serde_yml::Value::String(s) => J::String(s.clone()),
        serde_yml::Value::Sequence(items) => {
            let mut arr = Vec::with_capacity(items.len());
            for item in items {
                arr.push(yml_to_json(item)?);
            }
            J::Array(arr)
        }
        serde_yml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yml::Value::String(s) => s.clone(),
                    other => serde_yml::to_string(other).ok()?.trim().to_string(),
                };
                obj.insert(key, yml_to_json(v)?);
            }
            J::Object(obj)
        }
        serde_yml::Value::Tagged(_) => return None,
    })
}

/// Derive a display title for a document via the cascade:
/// frontmatter.title → first H1 → filename (stem).
pub fn title_from(parsed: &FrontmatterState, body: &str, filename_stem: &str) -> String {
    if let FrontmatterState::Parsed(m) = parsed {
        if let Some(v) = m.get("title") {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }
    for line in body.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    filename_stem.to_string()
}

/// Resolve the `ticket:` field to an `Option<String>`. Treats
/// `null`, `""`, and absence identically — the live corpus uses all
/// three forms interchangeably.
pub fn ticket_of(parsed: &FrontmatterState) -> Option<String> {
    match parsed {
        FrontmatterState::Parsed(m) => match m.get("ticket") {
            Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> Vec<u8> { s.as_bytes().to_vec() }

    #[test]
    fn parsed_extracts_mapping_and_body() {
        let raw = b("---\ntitle: Foo\nstatus: done\n---\n# Body\n\ntext\n");
        let p = parse(&raw);
        match p.state {
            FrontmatterState::Parsed(m) => {
                assert_eq!(m.get("title").and_then(|v| v.as_str()), Some("Foo"));
                assert_eq!(m.get("status").and_then(|v| v.as_str()), Some("done"));
            }
            other => panic!("expected Parsed, got {other:?}"),
        }
        assert!(p.body.starts_with("# Body"));
    }

    #[test]
    fn absent_when_no_leading_fence() {
        // Mirrors meta/notes/2026-03-30-reviewer-agent-subagent-access.md
        let raw = b("# Notes about subagents\n\nSome content.\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Absent));
        assert!(p.body.starts_with("# Notes"));
    }

    #[test]
    fn malformed_when_yaml_fails() {
        // Unclosed quoted string.
        let raw = b("---\ntitle: \"unclosed\nstatus: done\n---\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Malformed));
    }

    #[test]
    fn malformed_when_no_closing_fence() {
        let raw = b("---\ntitle: foo\nstatus: done\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Malformed));
    }

    #[test]
    fn malformed_when_yaml_root_is_not_mapping() {
        let raw = b("---\n- a\n- b\n---\nbody\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Malformed));
    }

    #[test]
    fn empty_frontmatter_parses_as_empty_mapping() {
        let raw = b("---\n---\n# Heading\n");
        let p = parse(&raw);
        match p.state {
            FrontmatterState::Parsed(m) => assert!(m.is_empty()),
            other => panic!("expected empty Parsed, got {other:?}"),
        }
    }

    #[test]
    fn title_cascade_prefers_frontmatter() {
        let raw = b("---\ntitle: From FM\n---\n# H1 Body\n");
        let p = parse(&raw);
        let t = title_from(&p.state, &p.body, "fallback");
        assert_eq!(t, "From FM");
    }

    #[test]
    fn title_cascade_falls_back_to_first_h1() {
        let raw = b("---\nstatus: done\n---\n# From H1\n# Second\n");
        let p = parse(&raw);
        let t = title_from(&p.state, &p.body, "fallback");
        assert_eq!(t, "From H1");
    }

    #[test]
    fn title_cascade_falls_back_to_filename_stem() {
        let raw = b("body without h1\n");
        let p = parse(&raw);
        let t = title_from(&p.state, &p.body, "2026-04-18-my-doc");
        assert_eq!(t, "2026-04-18-my-doc");
    }

    #[test]
    fn ticket_of_absent_value_returns_none() {
        let raw = b("---\nticket:\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state), None);
    }

    #[test]
    fn ticket_of_null_returns_none() {
        let raw = b("---\nticket: null\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state), None);
    }

    #[test]
    fn ticket_of_empty_string_returns_none() {
        let raw = b("---\nticket: \"\"\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state), None);
    }

    #[test]
    fn ticket_of_numeric_value_is_stringified() {
        let raw = b("---\nticket: 1478\n---\nbody\n");
        let p = parse(&raw);
        assert_eq!(ticket_of(&p.state).as_deref(), Some("1478"));
    }

    /// Regression: YAML block-sequence edge case noted in
    /// `meta/notes/2026-03-24-yaml-block-sequence-array-parsing.md`.
    #[test]
    fn block_sequences_survive_round_trip_to_json() {
        let raw = b("---\ntags:\n  - foo\n  - bar\n---\n");
        let p = parse(&raw);
        match p.state {
            FrontmatterState::Parsed(m) => {
                let tags = m.get("tags").unwrap();
                let arr = tags.as_array().unwrap();
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0].as_str(), Some("foo"));
                assert_eq!(arr[1].as_str(), Some("bar"));
            }
            other => panic!("expected Parsed, got {other:?}"),
        }
    }

    #[test]
    fn windows_crlf_line_endings_are_tolerated() {
        let raw = b("---\r\ntitle: Foo\r\n---\r\nbody\r\n");
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Parsed(_)));
    }

    #[test]
    fn invalid_utf8_falls_back_to_lossy_decode() {
        let mut raw = b"---\ntitle: Foo\n---\nbody\n".to_vec();
        raw.push(0xff); // invalid byte
        let p = parse(&raw);
        assert!(matches!(p.state, FrontmatterState::Parsed(_)));
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib frontmatter::tests` exits 0 with ≥14 tests passing.
- [ ] `mise run test:unit` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: the test file lands in a separate `jj` revision before the implementation; every test in that revision panics on compile or fails at runtime (the stub `frontmatter.rs` from Phase 3.1 exports only a doc-comment).
- [ ] **Mutation smoke test**: temporarily change the malformed-detection branch to return `FrontmatterState::Absent` when YAML parsing fails, run `cargo test --lib frontmatter::tests`, observe `malformed_when_yaml_fails` and `malformed_when_no_closing_fence` and `malformed_when_yaml_root_is_not_mapping` all fail; restore.
- [ ] Parse a real live file: `cat meta/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap.md | cargo run --example frontmatter-smoke 2>/dev/null` (if an example binary is added later; for Phase 3.3 a one-liner REPL check via `cargo test -- --nocapture` is sufficient).

---

## Phase 3.4: FileDriver trait and LocalFileDriver

### Overview

Introduce the `FileDriver` abstraction over disk access. In Phase 3
the trait has two methods: `list(kind)` and `read(path)`. `watch`
lands in Phase 4; `write_frontmatter` lands in Phase 8. Because the
trait is used as `dyn FileDriver` (via `Arc<dyn FileDriver>` in the
`Indexer`), methods return `Pin<Box<dyn Future + Send + '_>>` rather
than using RPITIT — RPITIT methods are not dyn-compatible on stable
Rust. No `async_trait` macro crate is needed; the desugaring is
manual and explicit. The `LocalFileDriver` impl enforces path-safety
via `std::fs::canonicalize` + a known-good prefix check — the
resolved root for each `DocTypeKey` is derived from
`config.doc_paths` at construction.

### Changes Required

#### 1. FileDriver trait and LocalFileDriver

**File**: `skills/visualisation/visualise/server/src/file_driver.rs`
**Changes**: Full replacement.

```rust
//! Disk-abstraction trait and the Phase 3 `LocalFileDriver` impl.
//!
//! The trait is deliberately small: only `list(kind)` and `read(path)`
//! ship in Phase 3. Phase 4 adds `watch(callback)`; Phase 8 adds
//! `write_frontmatter(path, patch, if_etag)`. Future alternative
//! implementations (e.g. a `GithubFileDriver`) swap the whole trait
//! impl without touching callers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::docs::DocTypeKey;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub bytes: Vec<u8>,
    /// Strong ETag, formatted as `"sha256-<hex>"`. Wire format is
    /// identical to the HTTP header value the response will emit.
    pub etag: String,
    pub mtime_ms: i64,
    pub size: u64,
}

const MAX_DOC_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

#[derive(Debug, thiserror::Error)]
pub enum FileDriverError {
    #[error("configured doc-type path missing: {kind:?}")]
    TypeNotConfigured { kind: DocTypeKey },
    #[error("path escapes the configured root: {path}")]
    PathEscape { path: PathBuf },
    #[error("not found: {path}")]
    NotFound { path: PathBuf },
    #[error("file too large: {path} is {size} bytes (limit {limit})")]
    TooLarge { path: PathBuf, size: u64, limit: u64 },
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Read-side abstraction over the meta tree.
///
/// Implementations are Send + Sync so that `Arc<dyn FileDriver>` can
/// be shared across request handlers.
///
/// Methods return `Pin<Box<dyn Future>>` rather than `impl Future`
/// because this trait is used as `dyn FileDriver` (via `Arc<dyn
/// FileDriver>` in the `Indexer`). RPITIT methods are not
/// dyn-compatible on stable Rust.
pub trait FileDriver: Send + Sync {
    fn list(
        &self,
        kind: DocTypeKey,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<PathBuf>, FileDriverError>> + Send + '_>>;

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<FileContent, FileDriverError>> + Send + '_>>;
}

/// Local-disk implementation. Holds the canonical per-type root
/// paths resolved at construction; `read` and `list` never trust
/// caller-supplied paths blindly.
pub struct LocalFileDriver {
    /// Canonicalised root per configured DocTypeKey. Keys present
    /// here are the ones for which `config.doc_paths` had an entry
    /// AND `std::fs::canonicalize` on that path succeeded at
    /// construction time. Entries for missing-on-disk configured
    /// paths use the un-canonicalised absolute path so that `list`
    /// can still return `Ok(vec![])` when the directory appears
    /// between server restarts.
    roots: HashMap<DocTypeKey, PathBuf>,
    /// Additional canonicalised directory prefixes that `read` will
    /// accept. Used for template tier directories (plugin-default,
    /// user-override, config-override) which don't belong to any
    /// `DocTypeKey` but still need path-safety validation.
    extra_roots: Vec<PathBuf>,
}

impl LocalFileDriver {
    pub fn new(
        doc_paths: &HashMap<String, PathBuf>,
        extra_roots: Vec<PathBuf>,
    ) -> Self {
        let mut roots = HashMap::new();
        for kind in DocTypeKey::all() {
            let Some(cfg_key) = kind.config_path_key() else { continue };
            let Some(raw) = doc_paths.get(cfg_key) else { continue };
            let canonical = std::fs::canonicalize(raw).unwrap_or_else(|_| raw.clone());
            roots.insert(kind, canonical);
        }
        let extra_roots = extra_roots
            .into_iter()
            .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
            .collect();
        Self { roots, extra_roots }
    }

    fn root_for(&self, kind: DocTypeKey) -> Result<&Path, FileDriverError> {
        self.roots
            .get(&kind)
            .map(|p| p.as_path())
            .ok_or(FileDriverError::TypeNotConfigured { kind })
    }

    /// Return the DocTypeKey whose root is a prefix of
    /// `canonical_path`. Used by `read` to reject paths that escape
    /// every configured root.
    fn kind_owning(&self, canonical_path: &Path) -> Option<DocTypeKey> {
        self.roots
            .iter()
            .find(|(_, root)| canonical_path.starts_with(root))
            .map(|(k, _)| *k)
    }

    /// Check whether `canonical_path` falls under any known root
    /// (doc-type roots or extra roots). Used by `read` to enforce
    /// path safety.
    fn path_is_allowed(&self, canonical_path: &Path) -> bool {
        self.kind_owning(canonical_path).is_some()
            || self.extra_roots.iter().any(|r| canonical_path.starts_with(r))
    }
}

impl FileDriver for LocalFileDriver {
    fn list(
        &self,
        kind: DocTypeKey,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<PathBuf>, FileDriverError>> + Send + '_>> {
        let root = match self.root_for(kind) {
            Ok(r) => r.to_path_buf(),
            Err(e) => return Box::pin(std::future::ready(Err(e))),
        };
        Box::pin(async move {
            let read = match tokio::fs::read_dir(&root).await {
                Ok(r) => r,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(vec![]);
                }
                Err(source) => return Err(FileDriverError::Io { path: root, source }),
            };
            let mut entries = Vec::new();
            let mut stream = read;
            loop {
                let entry = match stream.next_entry().await {
                    Ok(Some(e)) => e,
                    Ok(None) => break,
                    Err(source) => {
                        return Err(FileDriverError::Io {
                            path: root.clone(),
                            source,
                        });
                    }
                };
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("md") {
                    continue;
                }
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(source) => {
                        return Err(FileDriverError::Io { path, source });
                    }
                };
                if !file_type.is_file() {
                    continue;
                }
                entries.push(path);
            }
            Ok(entries)
        })
    }

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<FileContent, FileDriverError>> + Send + '_>> {
        let path = path.to_path_buf();
        Box::pin(async move {
            let canonical = tokio::fs::canonicalize(&path).await.map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    FileDriverError::NotFound { path: path.clone() }
                } else {
                    FileDriverError::Io { path: path.clone(), source }
                }
            })?;
            if !self.path_is_allowed(&canonical) {
                return Err(FileDriverError::PathEscape { path });
            }
            let meta = tokio::fs::metadata(&canonical).await.map_err(|source| {
                FileDriverError::Io { path: canonical.clone(), source }
            })?;
            if meta.len() > MAX_DOC_BYTES {
                return Err(FileDriverError::TooLarge {
                    path,
                    size: meta.len(),
                    limit: MAX_DOC_BYTES,
                });
            }
            let bytes = tokio::fs::read(&canonical).await.map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    FileDriverError::NotFound { path: canonical.clone() }
                } else {
                    FileDriverError::Io { path: canonical.clone(), source }
                }
            })?;
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let etag = etag_of(&bytes);
            Ok(FileContent {
                bytes,
                etag,
                mtime_ms,
                size: meta.len(),
            })
        })
    }
}

/// SHA-256 content hash formatted as `"sha256-<hex>"`. Used by the
/// indexer and the `/api/docs/{*path}` handler.
pub fn etag_of(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256-{}", hex::encode(h.finalize()))
}

/// Collect the parent directories of all template tier paths so that
/// the `LocalFileDriver` can accept `read` calls against them.
pub fn template_extra_roots(
    templates: &HashMap<String, crate::config::TemplateTiers>,
) -> Vec<PathBuf> {
    let mut dirs = std::collections::HashSet::new();
    for tiers in templates.values() {
        if let Some(co) = &tiers.config_override {
            if let Some(p) = co.parent() { dirs.insert(p.to_path_buf()); }
        }
        if let Some(p) = tiers.user_override.parent() {
            dirs.insert(p.to_path_buf());
        }
        if let Some(p) = tiers.plugin_default.parent() {
            dirs.insert(p.to_path_buf());
        }
    }
    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn seeded_driver(tmp: &Path) -> LocalFileDriver {
        let dec = tmp.join("decisions");
        std::fs::create_dir_all(&dec).unwrap();
        std::fs::write(dec.join("ADR-0001-foo.md"), "# Foo\n").unwrap();
        std::fs::write(dec.join("ADR-0002-bar.md"), "# Bar\n").unwrap();
        std::fs::write(dec.join(".gitkeep"), "").unwrap();
        let plans = tmp.join("plans"); // deliberately NOT created

        let mut map = HashMap::new();
        map.insert("decisions".into(), dec.clone());
        map.insert("plans".into(), plans);
        LocalFileDriver::new(&map, vec![])
    }

    #[tokio::test]
    async fn list_returns_only_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let mut got = d.list(DocTypeKey::Decisions).await.unwrap();
        got.sort();
        assert_eq!(got.len(), 2);
        for p in &got {
            assert!(p.to_string_lossy().ends_with(".md"));
        }
    }

    #[tokio::test]
    async fn list_missing_directory_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let got = d.list(DocTypeKey::Plans).await.unwrap();
        assert!(got.is_empty(), "missing dir must not be an error");
    }

    #[tokio::test]
    async fn list_unconfigured_type_is_not_configured_error() {
        let d = LocalFileDriver::new(&HashMap::new(), vec![]);
        let err = d.list(DocTypeKey::Notes).await.unwrap_err();
        assert!(matches!(err, FileDriverError::TypeNotConfigured { .. }));
    }

    #[tokio::test]
    async fn read_returns_bytes_and_etag() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let p = tmp.path().join("decisions").join("ADR-0001-foo.md");
        let content = d.read(&p).await.unwrap();
        assert_eq!(content.bytes, b"# Foo\n");
        assert_eq!(content.size, 6);
        assert!(content.etag.starts_with("sha256-"));
        assert_eq!(content.etag.len(), "sha256-".len() + 64);
    }

    #[tokio::test]
    async fn read_rejects_path_outside_any_configured_root() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        // A file that exists but isn't under any configured root.
        let outside = tmp.path().join("outside.md");
        std::fs::write(&outside, "x").unwrap();
        let err = d.read(&outside).await.unwrap_err();
        assert!(matches!(err, FileDriverError::PathEscape { .. }), "got {err:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn read_rejects_symlink_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let outside = tmp.path().join("secret.txt");
        std::fs::write(&outside, "s3cret").unwrap();
        let dec = tmp.path().join("decisions");
        let link = dec.join("escape.md");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let err = d.read(&link).await.unwrap_err();
        assert!(matches!(err, FileDriverError::PathEscape { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn read_missing_file_is_notfound() {
        let tmp = tempfile::tempdir().unwrap();
        let d = seeded_driver(tmp.path());
        let err = d
            .read(&tmp.path().join("decisions").join("nope.md"))
            .await
            .unwrap_err();
        assert!(matches!(err, FileDriverError::NotFound { .. }));
    }

    #[test]
    fn etag_is_stable_hex_sha256() {
        let e = etag_of(b"hello world");
        assert_eq!(
            e,
            "sha256-b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib file_driver::tests` exits 0 with ≥7 tests passing.
- [ ] `mise run test:unit` exits 0.
- [ ] `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml --tests -- -D warnings` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: red-first revision lands the test module with the Phase 3.1 trait signature imported; assertions compile but `LocalFileDriver::new` / `list` / `read` are stubbed to `todo!()` and fail.
- [ ] **Mutation smoke test**: temporarily weaken `read`'s `path_is_allowed` check to always return `true`, run `cargo test --lib file_driver::tests`, observe both `read_rejects_path_outside_any_configured_root` and `read_rejects_symlink_escape` fail; restore. (The symlink test is `#[cfg(unix)]` — verify on macOS/Linux.)
- [ ] On a real workspace, `cargo run --example list_decisions` (or an ad-hoc binary/unit test) lists every live ADR file without error. (Optional — the cargo tests cover the contract; this is smoke-level confidence against live filesystem quirks.)

---

## Phase 3.5: Indexer

### Overview

Introduce `indexer.rs` — the in-memory index over every document in
every ordinary DocType. On construction, the indexer walks every
configured `FileDriver::list` and for each returned path issues a
`FileDriver::read`, parses frontmatter, derives slug, and produces an
`IndexEntry`. Entries live behind an `Arc<RwLock<...>>` so Phase 4's
watcher can mutate them and Phase 3.9's read handlers can look them
up without contention. The indexer also builds two auxiliary lookup
tables — `adr_by_id` and `ticket_by_number` — that Phase 9 will
consume for wiki-link resolution; building them now is cheap and
avoids a second pass later.

### Changes Required

#### 1. Indexer module

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Full replacement.

```rust
//! In-memory index of every document in every ordinary DocType.
//!
//! Construction walks the configured directories once via the
//! `FileDriver`; entries land in an `Arc<RwLock<HashMap<PathBuf,
//! IndexEntry>>>`. Auxiliary tables (`adr_by_id`,
//! `ticket_by_number`) are built in the same pass. Phase 4's
//! watcher will call `rescan_path`/`forget_path` to keep the index
//! current; Phase 3 only exposes a whole-index `rescan()` that tests
//! can call after mutating fixture files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use crate::docs::DocTypeKey;
use crate::file_driver::{FileDriver, FileDriverError};
use crate::frontmatter::{self, FrontmatterState};
use crate::slug;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub r#type: DocTypeKey,
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub slug: Option<String>,
    pub title: String,
    pub frontmatter: serde_json::Value,
    pub frontmatter_state: String,
    pub ticket: Option<String>,
    pub mtime_ms: i64,
    pub size: u64,
    pub etag: String,
}

pub struct Indexer {
    driver: Arc<dyn FileDriver>,
    project_root: PathBuf,
    entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>,
    adr_by_id: Arc<RwLock<HashMap<u32, PathBuf>>>,
    ticket_by_number: Arc<RwLock<HashMap<u32, PathBuf>>>,
}

impl Indexer {
    /// Construct an indexer and perform the initial full scan.
    /// `project_root` is used to compute `rel_path` — the caller passes
    /// `cfg.project_root` (the explicit repo-root field on `Config`).
    pub async fn build(driver: Arc<dyn FileDriver>, project_root: PathBuf) -> Result<Self, FileDriverError> {
        let me = Self {
            driver,
            project_root,
            entries: Arc::new(RwLock::new(HashMap::new())),
            adr_by_id: Arc::new(RwLock::new(HashMap::new())),
            ticket_by_number: Arc::new(RwLock::new(HashMap::new())),
        };
        me.rescan().await?;
        Ok(me)
    }

    pub async fn rescan(&self) -> Result<(), FileDriverError> {
        let mut entries = HashMap::new();
        let mut adr_by_id = HashMap::new();
        let mut ticket_by_number = HashMap::new();

        for kind in DocTypeKey::all() {
            if kind == DocTypeKey::Templates {
                // Virtual type — handled by `templates.rs` not by the indexer.
                continue;
            }
            let paths = match self.driver.list(kind).await {
                Ok(p) => p,
                Err(FileDriverError::TypeNotConfigured { .. }) => continue,
                Err(e) => return Err(e),
            };
            for path in paths {
                let content = match self.driver.read(&path).await {
                    Ok(c) => c,
                    Err(FileDriverError::NotFound { .. }) => continue,
                    Err(e) => return Err(e),
                };
                let parsed = frontmatter::parse(&content.bytes);
                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let filename_stem = filename.strip_suffix(".md").unwrap_or(filename);
                let slug_val = slug::derive(kind, filename);
                let title = frontmatter::title_from(&parsed.state, &parsed.body, filename_stem);
                let ticket = frontmatter::ticket_of(&parsed.state);

                let (state_str, fm_json) = match &parsed.state {
                    FrontmatterState::Parsed(m) => {
                        let mut o = serde_json::Map::new();
                        for (k, v) in m {
                            o.insert(k.clone(), v.clone());
                        }
                        ("parsed".to_string(), serde_json::Value::Object(o))
                    }
                    FrontmatterState::Absent => ("absent".to_string(), serde_json::Value::Null),
                    FrontmatterState::Malformed => {
                        ("malformed".to_string(), serde_json::Value::Null)
                    }
                };

                // Side tables: ADR id and ticket number.
                if kind == DocTypeKey::Decisions {
                    if let Some(id) = parse_adr_id(&fm_json, filename) {
                        adr_by_id.insert(id, path.clone());
                    }
                }
                if kind == DocTypeKey::Tickets {
                    if let Some(n) = parse_ticket_number(filename) {
                        ticket_by_number.insert(n, path.clone());
                    }
                }

                let rel_path = path
                    .strip_prefix(&self.project_root)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| path.clone());

                let entry = IndexEntry {
                    r#type: kind,
                    path: path.clone(),
                    rel_path,
                    slug: slug_val,
                    title,
                    frontmatter: fm_json,
                    frontmatter_state: state_str,
                    ticket,
                    mtime_ms: content.mtime_ms,
                    size: content.size,
                    etag: content.etag,
                };
                entries.insert(path, entry);
            }
        }

        *self.entries.write().await = entries;
        *self.adr_by_id.write().await = adr_by_id;
        *self.ticket_by_number.write().await = ticket_by_number;
        Ok(())
    }

    pub async fn all_by_type(&self, kind: DocTypeKey) -> Vec<IndexEntry> {
        self.entries
            .read()
            .await
            .values()
            .filter(|e| e.r#type == kind)
            .cloned()
            .collect()
    }

    pub async fn all(&self) -> Vec<IndexEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    pub async fn get(&self, path: &Path) -> Option<IndexEntry> {
        // Try direct match first; fall back to canonicalised lookup.
        let guard = self.entries.read().await;
        if let Some(e) = guard.get(path) {
            return Some(e.clone());
        }
        if let Ok(canon) = std::fs::canonicalize(path) {
            return guard.get(&canon).cloned();
        }
        None
    }

    pub async fn adr_by_id(&self, id: u32) -> Option<IndexEntry> {
        let path = { self.adr_by_id.read().await.get(&id).cloned()? };
        self.get(&path).await
    }

    pub async fn ticket_by_number(&self, n: u32) -> Option<IndexEntry> {
        let path = { self.ticket_by_number.read().await.get(&n).cloned()? };
        self.get(&path).await
    }
}

fn parse_adr_id(fm: &serde_json::Value, filename: &str) -> Option<u32> {
    if let Some(s) = fm.get("adr_id").and_then(|v| v.as_str()) {
        if let Some(rest) = s.strip_prefix("ADR-") {
            if let Ok(n) = rest.parse::<u32>() {
                return Some(n);
            }
        }
    }
    let rest = filename.strip_prefix("ADR-")?;
    let dash = rest.find('-')?;
    rest[..dash].parse().ok()
}

fn parse_ticket_number(filename: &str) -> Option<u32> {
    let dash = filename.find('-')?;
    filename[..dash].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn seed(tmp: &Path) -> (PathBuf, std::collections::HashMap<String, PathBuf>) {
        let dec = tmp.join("meta/decisions");
        let plans = tmp.join("meta/plans");
        let reviews = tmp.join("meta/reviews/plans");
        let notes = tmp.join("meta/notes");
        for d in [&dec, &plans, &reviews, &notes] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(
            dec.join("ADR-0001-foo.md"),
            "---\nadr_id: ADR-0001\ntitle: Foo\n---\n# Body\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("2026-04-18-hello.md"),
            "---\ntitle: Hello Plan\nstatus: draft\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("2026-03-22-no-fm.md"),
            "# Ancient plan with no frontmatter\nbody\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("2026-04-01-malformed.md"),
            "---\ntitle: \"unclosed\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            reviews.join("2026-04-18-hello-review-1.md"),
            "---\ntarget: \"meta/plans/2026-04-18-hello.md\"\n---\n",
        )
        .unwrap();
        std::fs::write(
            reviews.join(
                "2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md",
            ),
            "---\ntitle: review\n---\n",
        )
        .unwrap();
        std::fs::write(
            notes.join("2026-03-30-no-fm.md"),
            "# A bare note\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert("decisions".into(), dec);
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        map.insert("notes".into(), notes);
        (tmp.to_path_buf(), map)
    }

    async fn build_indexer(tmp: &Path) -> Indexer {
        let (root, map) = seed(tmp);
        let driver: Arc<dyn FileDriver> = Arc::new(
            crate::file_driver::LocalFileDriver::new(&map, vec![]),
        );
        Indexer::build(driver, root).await.unwrap()
    }

    #[tokio::test]
    async fn scan_populates_entries_for_configured_types() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let decisions = idx.all_by_type(DocTypeKey::Decisions).await;
        assert_eq!(decisions.len(), 1);
        let plans = idx.all_by_type(DocTypeKey::Plans).await;
        assert_eq!(plans.len(), 3);
        let reviews = idx.all_by_type(DocTypeKey::PlanReviews).await;
        assert_eq!(reviews.len(), 2);
        let notes = idx.all_by_type(DocTypeKey::Notes).await;
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn etag_is_content_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let decs = idx.all_by_type(DocTypeKey::Decisions).await;
        let adr = &decs[0];
        let bytes = std::fs::read(&adr.path).unwrap();
        let expected = crate::file_driver::etag_of(&bytes);
        assert_eq!(adr.etag, expected);
    }

    #[tokio::test]
    async fn frontmatter_state_distinguishes_absent_malformed_parsed() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let plans = idx.all_by_type(DocTypeKey::Plans).await;
        let by_name: HashMap<String, IndexEntry> = plans
            .into_iter()
            .map(|e| (e.path.file_name().unwrap().to_string_lossy().to_string(), e))
            .collect();
        assert_eq!(
            by_name["2026-04-18-hello.md"].frontmatter_state,
            "parsed"
        );
        assert_eq!(
            by_name["2026-03-22-no-fm.md"].frontmatter_state,
            "absent"
        );
        assert_eq!(
            by_name["2026-04-01-malformed.md"].frontmatter_state,
            "malformed"
        );
    }

    #[tokio::test]
    async fn slug_stripped_per_type_and_review_suffix_edge_case() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let revs = idx.all_by_type(DocTypeKey::PlanReviews).await;
        let slugs: Vec<String> =
            revs.iter().filter_map(|e| e.slug.clone()).collect();
        assert!(slugs.contains(&"hello".to_string()));
        assert!(
            slugs.contains(&"initialise-skill-and-review-pr-ephemeral-migration".to_string()),
            "internal -review- must be preserved in slug; got {slugs:?}",
        );
    }

    #[tokio::test]
    async fn title_fallback_to_first_h1_when_fm_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let notes = idx.all_by_type(DocTypeKey::Notes).await;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "A bare note");
    }

    #[tokio::test]
    async fn adr_by_id_is_populated_from_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let adr = idx.adr_by_id(1).await.unwrap();
        assert_eq!(adr.r#type, DocTypeKey::Decisions);
    }

    #[tokio::test]
    async fn rescan_picks_up_filesystem_mutations() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let before = idx.all_by_type(DocTypeKey::Plans).await.len();
        std::fs::write(
            tmp.path().join("meta/plans/2026-05-01-new.md"),
            "---\ntitle: New\n---\n",
        )
        .unwrap();
        idx.rescan().await.unwrap();
        let after = idx.all_by_type(DocTypeKey::Plans).await.len();
        assert_eq!(after, before + 1);
    }

    #[tokio::test]
    async fn malformed_entry_is_still_addressable_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let path = tmp.path().join("meta/plans/2026-04-01-malformed.md");
        let entry = idx.get(&path).await.expect("malformed entry still indexed");
        assert_eq!(entry.frontmatter_state, "malformed");
        assert!(entry.etag.starts_with("sha256-"));
    }

    #[tokio::test]
    async fn scan_2000_files_completes_within_one_second() {
        let tmp = tempfile::tempdir().unwrap();
        let body = "---\ntitle: Filler\n---\n".to_string()
            + &"x".repeat(10 * 1024);

        let dirs = [
            ("decisions", "meta/decisions", "0001"),
            ("plans", "meta/plans", "2026-01-01"),
            ("review_plans", "meta/reviews/plans", "2026-01-01"),
            ("notes", "meta/notes", "2026-01-01"),
        ];
        let mut map = HashMap::new();
        for (key, rel, prefix) in &dirs {
            let dir_path = tmp.path().join(rel);
            std::fs::create_dir_all(&dir_path).unwrap();
            for i in 0..500 {
                let name = format!("{}-filler-{i:04}.md", prefix);
                std::fs::write(dir_path.join(name), &body).unwrap();
            }
            map.insert(key.to_string(), dir_path);
        }

        let driver: Arc<dyn FileDriver> = Arc::new(
            LocalFileDriver::new(&map, vec![]),
        );
        let start = std::time::Instant::now();
        let idx = Indexer::build(driver, tmp.path().to_path_buf())
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_secs() < 1,
            "scan took {elapsed:?}, expected < 1 s",
        );
        assert_eq!(idx.all().await.len(), 2000);
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib indexer::tests` exits 0 with ≥9 tests passing.
- [ ] `mise run test:unit` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: tests land first (with `Indexer::build` as `todo!()`); assertions fail at runtime. Next revision implements `rescan`; green.
- [ ] **Mutation smoke test**: change the `kind == DocTypeKey::Templates { continue }` guard to skip `DocTypeKey::PlanReviews` instead, run `cargo test --lib indexer::tests`, observe `scan_populates_entries_for_configured_types` and `slug_stripped_per_type_and_review_suffix_edge_case` fail; restore.
- [ ] Spot-scan against live data: seed an indexer at the workspace root and print the ADR count; compare to `ls meta/decisions/*.md | wc -l` (expect 21 today).

---

## Phase 3.6: Templates virtual DocType resolver

### Overview

Introduce `templates.rs` — a resolver over the `config.templates`
map that exposes per-tier presence and content without walking a
directory. The existing three-tier contract (D9 / ADR-0017) is the
precedence order: `config-override > user-override > plugin-default`.
Plugin-default is contractually always present on disk; if it is
missing at startup, the resolver records `present: false` and surfaces
it via the API so the operator can see which template is in trouble
(rather than panicking). User-override and config-override are
optional.

Template reads go through the `FileDriver` trait (Phase 3.4) rather
than calling `std::fs::read` directly. This keeps all disk I/O
non-blocking and behind the same path-safety canonicalisation that
protects doc-type reads, and means an alternative driver (e.g.
`GithubFileDriver`) can serve templates without special-casing.
`TemplateResolver::build` is therefore `async` and accepts a
`&dyn FileDriver`. The `LocalFileDriver` constructor gains an
`extra_roots: Vec<PathBuf>` parameter so that the prefix check in
`read` accepts template tier directories alongside doc-type roots.

### Changes Required

#### 1. Templates module

**File**: `skills/visualisation/visualise/server/src/templates.rs`
**Changes**: Full replacement.

```rust
//! Virtual `templates` DocType backed by the three-tier resolver.
//!
//! Loads tier content eagerly on construction so `GET /api/templates/:name`
//! is a pure cache lookup — matches the indexer's eager-read model and
//! means a user-edit to a tier-2 file at runtime does not show through
//! until Phase 4's watcher reindexes it.
//!
//! Reads go through the `FileDriver` trait so that (a) path-safety
//! canonicalisation is enforced even on config-supplied template paths,
//! and (b) an alternative driver (e.g. `GithubFileDriver`) can serve
//! templates without special-casing.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::config::TemplateTiers;
use crate::file_driver::{etag_of, FileDriver};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateTierSource {
    ConfigOverride,
    UserOverride,
    PluginDefault,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateTier {
    pub source: TemplateTierSource,
    pub path: PathBuf,
    pub present: bool,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub name: String,
    /// Exactly 3 entries, priority order (config → user → plugin).
    pub tiers: Vec<TemplateTier>,
    pub active_tier: TemplateTierSource,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDetail {
    pub name: String,
    /// Exactly 3 entries; `content` populated for present tiers.
    pub tiers: Vec<TemplateTier>,
    pub active_tier: TemplateTierSource,
}

pub struct TemplateResolver {
    /// Per-template, ordered priority list. Each entry caches its
    /// on-disk content (if present) as a `String` plus the ETag.
    by_name: HashMap<String, Vec<TemplateTier>>,
}

impl TemplateResolver {
    pub async fn build(
        templates: &HashMap<String, TemplateTiers>,
        driver: &dyn FileDriver,
    ) -> Self {
        let mut by_name = HashMap::new();
        for (name, tiers) in templates {
            let mut ordered = Vec::with_capacity(3);

            // Tier 1: config_override (optional).
            let config_path = tiers
                .config_override
                .clone()
                .unwrap_or_else(|| PathBuf::from(format!("<no config override for {name}>")));
            let (present, content, etag) = load_via_driver(
                &tiers.config_override,
                driver,
            ).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::ConfigOverride,
                path: config_path,
                present,
                active: false, // set below
                content,
                etag,
            });

            // Tier 2: user_override.
            let (present, content, etag) = load_via_driver(
                &Some(tiers.user_override.clone()),
                driver,
            ).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::UserOverride,
                path: tiers.user_override.clone(),
                present,
                active: false,
                content,
                etag,
            });

            // Tier 3: plugin_default.
            let (present, content, etag) = load_via_driver(
                &Some(tiers.plugin_default.clone()),
                driver,
            ).await;
            ordered.push(TemplateTier {
                source: TemplateTierSource::PluginDefault,
                path: tiers.plugin_default.clone(),
                present,
                active: false,
                content,
                etag,
            });

            // Mark the active tier: highest-priority present entry.
            // Falls back to plugin-default even when every tier is
            // absent, so `active_tier` always has a value.
            let active_source = ordered
                .iter()
                .find(|t| t.present)
                .map(|t| t.source)
                .unwrap_or(TemplateTierSource::PluginDefault);
            for t in &mut ordered {
                t.active = t.source == active_source;
            }

            by_name.insert(name.clone(), ordered);
        }
        Self { by_name }
    }

    pub fn list(&self) -> Vec<TemplateSummary> {
        let mut out: Vec<TemplateSummary> = self
            .by_name
            .iter()
            .map(|(name, tiers)| TemplateSummary {
                name: name.clone(),
                tiers: tiers
                    .iter()
                    .map(|t| TemplateTier {
                        // Strip `content` for the list view.
                        source: t.source,
                        path: t.path.clone(),
                        present: t.present,
                        active: t.active,
                        content: None,
                        etag: t.etag.clone(),
                    })
                    .collect(),
                active_tier: tiers
                    .iter()
                    .find(|t| t.active)
                    .map(|t| t.source)
                    .unwrap_or(TemplateTierSource::PluginDefault),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn detail(&self, name: &str) -> Option<TemplateDetail> {
        let tiers = self.by_name.get(name)?;
        let active_tier = tiers
            .iter()
            .find(|t| t.active)
            .map(|t| t.source)
            .unwrap_or(TemplateTierSource::PluginDefault);
        Some(TemplateDetail {
            name: name.to_string(),
            tiers: tiers.clone(),
            active_tier,
        })
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.by_name.keys().cloned().collect();
        v.sort();
        v
    }
}

async fn load_via_driver(
    path: &Option<PathBuf>,
    driver: &dyn FileDriver,
) -> (bool, Option<String>, Option<String>) {
    let Some(p) = path else { return (false, None, None); };
    match driver.read(p).await {
        Ok(fc) => {
            let content = String::from_utf8(fc.bytes).ok();
            (true, content, Some(fc.etag))
        }
        Err(_) => (false, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_driver::LocalFileDriver;

    fn tier(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    fn tiers_all_three(dir: &std::path::Path) -> TemplateTiers {
        TemplateTiers {
            config_override: Some(tier(dir, "cfg-adr.md", "from config")),
            user_override: tier(dir, "user-adr.md", "from user"),
            plugin_default: tier(dir, "plugin-adr.md", "from plugin"),
        }
    }

    fn test_driver(dir: &std::path::Path) -> LocalFileDriver {
        LocalFileDriver::new(&HashMap::new(), vec![dir.to_path_buf()])
    }

    #[tokio::test]
    async fn all_three_tiers_present_picks_config_override_as_active() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver).await;
        let summaries = r.list();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].active_tier, TemplateTierSource::ConfigOverride);
        let active = summaries[0].tiers.iter().find(|t| t.active).unwrap();
        assert_eq!(active.source, TemplateTierSource::ConfigOverride);
    }

    #[tokio::test]
    async fn only_plugin_default_present_picks_plugin_default_active() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let t = TemplateTiers {
            config_override: None,
            user_override: tmp.path().join("missing-user.md"),
            plugin_default: tier(tmp.path(), "plugin-adr.md", "from plugin"),
        };
        let mut map = HashMap::new();
        map.insert("adr".to_string(), t);
        let r = TemplateResolver::build(&map, &driver).await;
        let d = r.detail("adr").unwrap();
        assert_eq!(d.active_tier, TemplateTierSource::PluginDefault);
        assert_eq!(
            d.tiers.iter().filter(|t| t.present).count(),
            1,
            "only plugin-default should be present",
        );
    }

    #[tokio::test]
    async fn user_override_wins_when_config_override_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let t = TemplateTiers {
            config_override: None,
            user_override: tier(tmp.path(), "user-adr.md", "from user"),
            plugin_default: tier(tmp.path(), "plugin-adr.md", "from plugin"),
        };
        let mut map = HashMap::new();
        map.insert("adr".to_string(), t);
        let r = TemplateResolver::build(&map, &driver).await;
        let d = r.detail("adr").unwrap();
        assert_eq!(d.active_tier, TemplateTierSource::UserOverride);
    }

    #[tokio::test]
    async fn list_sorts_names_alphabetically() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("plan".to_string(), tiers_all_three(tmp.path()));
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        map.insert("research".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver).await;
        let names: Vec<String> = r.list().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["adr", "plan", "research"]);
    }

    #[tokio::test]
    async fn list_omits_content_but_detail_includes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver).await;
        let s = &r.list()[0];
        assert!(s.tiers.iter().all(|t| t.content.is_none()));
        let d = r.detail("adr").unwrap();
        let present_with_content = d
            .tiers
            .iter()
            .filter(|t| t.present && t.content.is_some())
            .count();
        assert_eq!(present_with_content, 3);
    }

    #[tokio::test]
    async fn detail_of_unknown_name_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r = TemplateResolver::build(&map, &driver).await;
        assert!(r.detail("missing").is_none());
    }

    #[tokio::test]
    async fn etag_is_stable_across_reads() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = test_driver(tmp.path());
        let mut map = HashMap::new();
        map.insert("adr".to_string(), tiers_all_three(tmp.path()));
        let r1 = TemplateResolver::build(&map, &driver).await;
        let r2 = TemplateResolver::build(&map, &driver).await;
        let a = r1.detail("adr").unwrap();
        let b = r2.detail("adr").unwrap();
        for (ta, tb) in a.tiers.iter().zip(b.tiers.iter()) {
            assert_eq!(ta.etag, tb.etag);
        }
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib templates::tests` exits 0 with ≥7 tests passing.
- [ ] `mise run test:unit` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: test file lands before the resolver implementation; tests fail in the red revision.
- [ ] **Mutation smoke test**: flip the priority order so `user_override` wins over `config_override`, run `cargo test --lib templates::tests`, observe `all_three_tiers_present_picks_config_override_as_active` and `user_override_wins_when_config_override_absent` fail; restore.

---

## Phase 3.7: Cluster computation

### Overview

Introduce `clusters.rs` — slug-based grouping of IndexEntries into
`LifecycleCluster` objects. Named `clusters` to avoid shadowing the
existing `lifecycle.rs` (which handles server-process lifecycle). The
wire type remains `LifecycleCluster` per the spec. Ordering within a
cluster follows the spec's canonical timeline: ticket → research →
plan → plan-review → validation → PR → pr-review → decision → notes,
with mtime-ascending as the secondary key inside each type.
Templates are excluded.

### Changes Required

#### 1. Clusters module

**File**: `skills/visualisation/visualise/server/src/clusters.rs`
**Changes**: Full replacement.

```rust
//! Slug-based document-lifecycle clustering.
//!
//! Runs over the indexer's `all()` snapshot and groups every entry
//! with a non-`None` slug into a `LifecycleCluster`. Ordering inside
//! a cluster follows the spec's canonical timeline: ticket → research
//! → plan → plan-review → validation → PR → pr-review → decision →
//! notes, with mtime ascending as the secondary sort inside each type.

use std::collections::HashMap;

use serde::Serialize;

use crate::docs::DocTypeKey;
use crate::indexer::IndexEntry;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completeness {
    pub has_ticket: bool,
    pub has_research: bool,
    pub has_plan: bool,
    pub has_plan_review: bool,
    pub has_validation: bool,
    pub has_pr: bool,
    pub has_pr_review: bool,
    pub has_decision: bool,
    pub has_notes: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCluster {
    pub slug: String,
    pub title: String,
    pub entries: Vec<IndexEntry>,
    pub completeness: Completeness,
}

pub fn compute_clusters(entries: &[IndexEntry]) -> Vec<LifecycleCluster> {
    let mut buckets: HashMap<String, Vec<IndexEntry>> = HashMap::new();
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) {
            continue;
        }
        let Some(slug) = e.slug.clone() else { continue };
        buckets.entry(slug).or_default().push(e.clone());
    }

    let mut clusters: Vec<LifecycleCluster> = buckets
        .into_iter()
        .map(|(slug, mut entries)| {
            entries.sort_by(|a, b| {
                canonical_rank(a.r#type)
                    .cmp(&canonical_rank(b.r#type))
                    .then(a.mtime_ms.cmp(&b.mtime_ms))
            });
            let title = derive_title(&slug, &entries);
            let completeness = derive_completeness(&entries);
            LifecycleCluster {
                slug,
                title,
                entries,
                completeness,
            }
        })
        .collect();

    clusters.sort_by(|a, b| a.slug.cmp(&b.slug));
    clusters
}

fn canonical_rank(kind: DocTypeKey) -> u8 {
    match kind {
        DocTypeKey::Tickets => 0,
        DocTypeKey::Research => 1,
        DocTypeKey::Plans => 2,
        DocTypeKey::PlanReviews => 3,
        DocTypeKey::Validations => 4,
        DocTypeKey::Prs => 5,
        DocTypeKey::PrReviews => 6,
        DocTypeKey::Decisions => 7,
        DocTypeKey::Notes => 8,
        DocTypeKey::Templates => u8::MAX, // excluded earlier
    }
}

fn derive_title(slug: &str, entries: &[IndexEntry]) -> String {
    // Prefer the entry whose title came from frontmatter (i.e.
    // starts with something other than the filename stem). Fall back
    // to the first entry's title, then to the slug itself.
    for e in entries {
        if e.frontmatter_state == "parsed" && !e.title.is_empty() {
            return e.title.clone();
        }
    }
    if let Some(e) = entries.first() {
        if !e.title.is_empty() {
            return e.title.clone();
        }
    }
    slug.to_string()
}

fn derive_completeness(entries: &[IndexEntry]) -> Completeness {
    let mut c = Completeness {
        has_ticket: false,
        has_research: false,
        has_plan: false,
        has_plan_review: false,
        has_validation: false,
        has_pr: false,
        has_pr_review: false,
        has_decision: false,
        has_notes: false,
    };
    for e in entries {
        match e.r#type {
            DocTypeKey::Tickets => c.has_ticket = true,
            DocTypeKey::Research => c.has_research = true,
            DocTypeKey::Plans => c.has_plan = true,
            DocTypeKey::PlanReviews => c.has_plan_review = true,
            DocTypeKey::Validations => c.has_validation = true,
            DocTypeKey::Prs => c.has_pr = true,
            DocTypeKey::PrReviews => c.has_pr_review = true,
            DocTypeKey::Decisions => c.has_decision = true,
            DocTypeKey::Notes => c.has_notes = true,
            DocTypeKey::Templates => {}
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(kind: DocTypeKey, slug: &str, mtime_ms: i64, title: &str) -> IndexEntry {
        IndexEntry {
            r#type: kind,
            path: PathBuf::from(format!("/x/{slug}.md")),
            rel_path: PathBuf::from(format!("{slug}.md")),
            slug: Some(slug.to_string()),
            title: title.to_string(),
            frontmatter: serde_json::Value::Null,
            frontmatter_state: "parsed".to_string(),
            ticket: None,
            mtime_ms,
            size: 0,
            etag: "sha256-00".to_string(),
        }
    }

    #[test]
    fn same_slug_clusters_into_one_entry() {
        let entries = vec![
            entry(DocTypeKey::Plans, "foo", 10, "Plan for Foo"),
            entry(DocTypeKey::PlanReviews, "foo", 20, "Review"),
            entry(DocTypeKey::Tickets, "foo", 5, "Ticket"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(clusters.len(), 1);
        let c = &clusters[0];
        assert_eq!(c.slug, "foo");
        assert_eq!(c.entries.len(), 3);
    }

    #[test]
    fn canonical_ordering_is_ticket_then_plan_then_review() {
        let entries = vec![
            entry(DocTypeKey::PlanReviews, "foo", 30, "Review"),
            entry(DocTypeKey::Plans, "foo", 20, "Plan"),
            entry(DocTypeKey::Tickets, "foo", 10, "Ticket"),
        ];
        let clusters = compute_clusters(&entries);
        let kinds: Vec<DocTypeKey> = clusters[0].entries.iter().map(|e| e.r#type).collect();
        assert_eq!(
            kinds,
            vec![
                DocTypeKey::Tickets,
                DocTypeKey::Plans,
                DocTypeKey::PlanReviews,
            ]
        );
    }

    #[test]
    fn mtime_breaks_ties_within_a_type() {
        let entries = vec![
            entry(DocTypeKey::PlanReviews, "foo", 300, "Review 3"),
            entry(DocTypeKey::PlanReviews, "foo", 100, "Review 1"),
            entry(DocTypeKey::PlanReviews, "foo", 200, "Review 2"),
        ];
        let clusters = compute_clusters(&entries);
        let titles: Vec<String> = clusters[0].entries.iter().map(|e| e.title.clone()).collect();
        assert_eq!(titles, vec!["Review 1", "Review 2", "Review 3"]);
    }

    #[test]
    fn completeness_flags_track_present_types() {
        let entries = vec![
            entry(DocTypeKey::Tickets, "foo", 10, "T"),
            entry(DocTypeKey::Plans, "foo", 20, "P"),
            entry(DocTypeKey::Decisions, "foo", 30, "D"),
        ];
        let clusters = compute_clusters(&entries);
        let c = &clusters[0].completeness;
        assert!(c.has_ticket);
        assert!(c.has_plan);
        assert!(c.has_decision);
        assert!(!c.has_research);
        assert!(!c.has_plan_review);
        assert!(!c.has_validation);
        assert!(!c.has_pr);
        assert!(!c.has_pr_review);
        assert!(!c.has_notes);
    }

    #[test]
    fn templates_are_excluded_from_clusters() {
        let mut t = entry(DocTypeKey::Plans, "shared", 10, "Plan");
        let mut tmpl = entry(DocTypeKey::Templates, "shared", 20, "Template");
        tmpl.slug = Some("shared".to_string()); // even if a caller set one
        t.slug = Some("shared".to_string());
        let clusters = compute_clusters(&[t, tmpl]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entries.len(), 1);
        assert_eq!(clusters[0].entries[0].r#type, DocTypeKey::Plans);
    }

    #[test]
    fn entries_without_slug_are_excluded() {
        let mut e = entry(DocTypeKey::Plans, "x", 10, "P");
        e.slug = None;
        let clusters = compute_clusters(&[e]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn clusters_are_sorted_by_slug_alphabetically() {
        let entries = vec![
            entry(DocTypeKey::Plans, "bravo", 10, "B"),
            entry(DocTypeKey::Plans, "alpha", 20, "A"),
            entry(DocTypeKey::Plans, "charlie", 30, "C"),
        ];
        let clusters = compute_clusters(&entries);
        let slugs: Vec<String> = clusters.iter().map(|c| c.slug.clone()).collect();
        assert_eq!(slugs, vec!["alpha", "bravo", "charlie"]);
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --lib clusters::tests` exits 0 with ≥7 tests passing.
- [ ] `mise run test:unit` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: tests land first with `compute_clusters` as `todo!()`; red. Implementation follows; green.
- [ ] **Mutation smoke test**: swap the `canonical_rank` values for `Plans` and `PlanReviews`, run `cargo test --lib clusters::tests`, observe `canonical_ordering_is_ticket_then_plan_then_review` fails; restore.

---

## Phase 3.8: AppState composition and router builder

### Overview

Wire the new modules into the live server. `AppState` grows four
`Arc`s: the existing `cfg`, plus `file_driver` (so Phase 9's
doc-fetch route can read bytes that weren't already cached by the
indexer — e.g. if a path resolves via the `{*path}` wildcard to a
non-indexed file), `indexer`, `templates`, and a pre-computed
`clusters` `RwLock`. Router construction moves from inline code in
`server::run` into a dedicated `build_router(state) -> Router`
helper so Phase 3.9's integration tests can drive the full stack via
`tower::ServiceExt::oneshot` — no real port required.

The existing placeholder `GET /` is preserved. A sentinel
`GET /api/healthz` is added so Phase 3.10's end-to-end smoke test
can confirm the API surface is reachable without depending on the
richer handlers landing. All other API routes are added in Phase
3.9.

### Changes Required

#### 1. Extend `Config` with `project_root` and update the config script

**File**: `skills/visualisation/visualise/server/src/config.rs`
**Changes**: Add `project_root: PathBuf` field.

Add `project_root: PathBuf` to the `Config` struct alongside the existing
fields, and include it in the `deny_unknown_fields`-guarded serde shape.
The field is always emitted by the script (never optional):

```rust
pub struct Config {
    pub plugin_root: PathBuf,
    pub plugin_version: String,
    pub project_root: PathBuf,   // ← new: explicit repo root
    pub tmp_path: PathBuf,
    // … other existing fields unchanged
}
```

**File**: `scripts/write-visualiser-config.sh`
**Changes**: Emit `project_root` using the existing `find_repo_root` helper
from `vcs-common.sh` (already sourced by the script):

```bash
project_root=$(find_repo_root)
# … existing config construction …
jq -n \
  --arg project_root "$project_root" \
  # … other existing args … \
  '{project_root: $project_root, …}'
```

This is the only `config.json` extension in Phase 3. All test fixtures
that construct a `Config` struct literal must include `project_root`.

#### 2. Extract router construction into a builder

**File**: `skills/visualisation/visualise/server/src/server.rs`
**Changes**: Extract `build_router` and extend `AppState`.

Replace the existing `AppState` and the router construction inside
`run` with:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub cfg: Arc<Config>,
    pub file_driver: Arc<crate::file_driver::LocalFileDriver>,
    pub indexer: Arc<crate::indexer::Indexer>,
    pub templates: Arc<crate::templates::TemplateResolver>,
    pub clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    pub activity: Arc<crate::activity::Activity>,
}

impl AppState {
    /// Construct the full AppState from a Config and an Activity tracker.
    /// The caller owns the Activity so it can also wire it into the
    /// idle-shutdown task; AppState just stores the shared handle.
    pub async fn build(
        cfg: Config,
        activity: Arc<crate::activity::Activity>,
    ) -> Result<Arc<Self>, AppStateError> {
        let cfg = Arc::new(cfg);
        let template_roots = crate::file_driver::template_extra_roots(&cfg.templates);
        let driver = Arc::new(
            crate::file_driver::LocalFileDriver::new(&cfg.doc_paths, template_roots),
        );
        let indexer = Arc::new(
            crate::indexer::Indexer::build(driver.clone(), cfg.project_root.clone()).await?,
        );
        let templates = Arc::new(
            crate::templates::TemplateResolver::build(&cfg.templates, driver.as_ref()).await,
        );
        let cluster_seed = crate::clusters::compute_clusters(&indexer.all().await);
        let clusters = Arc::new(RwLock::new(cluster_seed));
        Ok(Arc::new(Self {
            cfg,
            file_driver: driver,
            indexer,
            templates,
            clusters,
            activity,
        }))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error("indexer build failed: {0}")]
    Indexer(#[from] crate::file_driver::FileDriverError),
}

// ServerError gains a Startup variant (alongside the existing Bind,
// Signal, etc. variants from Phase 2) so that `run` can propagate
// AppState build failures with the correct semantic:
//
//     #[error("startup failed: {0}")]
//     Startup(#[from] AppStateError),

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(placeholder_root))
        .route("/api/healthz", axum::routing::get(healthz))
        // Phase 3.9 route-mount happens here via `crate::api::mount(Router, state)`.
        .merge(crate::api::mount(state.clone()))
        .route_layer(axum::middleware::from_fn_with_state(
            state.activity.clone(),
            crate::activity::middleware,
        ))
        .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .layer(middleware::from_fn(host_header_guard))
        .with_state(state)
}

async fn healthz() -> &'static str { "ok\n" }
```

The existing `run` function delegates to `AppState::build` and
`build_router`:

```rust
pub async fn run(cfg: Config, info_path: &Path) -> Result<(), ServerError> {
    // (loopback check and listener-bind unchanged)
    // ...

    let activity = Arc::new(crate::activity::Activity::new());
    let state = AppState::build(cfg, activity.clone()).await?;
    let app = build_router(state.clone());
    // (server-info / pid-file writes unchanged)
    // ...
}
```

The `placeholder_root` handler is adjusted to pull the version from
the bound `State<Arc<AppState>>` instead of `env!` directly so Phase
10's version-header work has an obvious entry point; Phase 3 keeps
the behaviour identical.

#### 3. API module skeleton

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`
**Changes**: Full replacement.

```rust
//! HTTP API surface. Concrete route handlers land in Phase 3.9;
//! this sub-phase ships the mount function so the router builder
//! can compose cleanly.

use axum::Router;
use std::sync::Arc;

use crate::server::AppState;

pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
}
```

#### 4. Shared test helper

**File**: `skills/visualisation/visualise/server/tests/common/mod.rs`
**Changes**: New file. Introduced here (Phase 3.8) so that
`router_compose.rs` can use it; Phase 3.9 test files reuse it.

```rust
//! Seeding helpers shared by integration tests. Keeps every test
//! file on the same minimal fixture shape without duplicating
//! `Config` construction boilerplate.

use std::collections::HashMap;
use std::path::Path;

use accelerator_visualiser::config::{Config, TemplateTiers};

/// Build a project tree under `tmp` containing: one ADR, one plan,
/// one plan-review (same slug as the plan → expected to cluster),
/// five plugin-default templates. Returns a `Config` pointing at it.
pub fn seeded_cfg(tmp: &Path) -> Config {
    let meta = tmp.join("meta");
    let decisions = meta.join("decisions");
    let plans = meta.join("plans");
    let reviews = meta.join("reviews/plans");
    let tmp_dir = meta.join("tmp/visualiser");
    for d in [&decisions, &plans, &reviews, &tmp_dir] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(
        decisions.join("ADR-0001-foo.md"),
        "---\nadr_id: ADR-0001\ntitle: The Foo Decision\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        plans.join("2026-04-18-foo.md"),
        "---\ntitle: The Foo Plan\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        reviews.join("2026-04-18-foo-review-1.md"),
        "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
    )
    .unwrap();

    let tpl_dir = tmp.join("plugin-templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    let mut templates = HashMap::new();
    for name in ["adr", "plan", "research", "validation", "pr-description"] {
        let pd = tpl_dir.join(format!("{name}.md"));
        std::fs::write(&pd, format!("# {name} plugin default\n")).unwrap();
        templates.insert(
            name.to_string(),
            TemplateTiers {
                config_override: None,
                user_override: meta.join(format!("templates/{name}.md")),
                plugin_default: pd,
            },
        );
    }

    let mut doc_paths = HashMap::new();
    doc_paths.insert("decisions".into(), decisions);
    doc_paths.insert("tickets".into(), meta.join("tickets"));
    doc_paths.insert("plans".into(), plans);
    doc_paths.insert("research".into(), meta.join("research"));
    doc_paths.insert("review_plans".into(), reviews);
    doc_paths.insert("review_prs".into(), meta.join("reviews/prs"));
    doc_paths.insert("validations".into(), meta.join("validations"));
    doc_paths.insert("notes".into(), meta.join("notes"));
    doc_paths.insert("prs".into(), meta.join("prs"));

    Config {
        plugin_root: tmp.to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.to_path_buf(),
        tmp_path: tmp_dir,
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.join("server.log"),
        doc_paths,
        templates,
    }
}
```

#### 5. Integration test: router composes

**File**: `skills/visualisation/visualise/server/tests/router_compose.rs`
**Changes**: New file.

```rust
//! Integration-level smoke that the Phase 3.8 router builder produces
//! a working axum service. Exercises the `tower::ServiceExt::oneshot`
//! pattern that the Phase 3.9 per-endpoint tests consume.
//!
//! Uses `common::seeded_cfg` (shared with Phase 3.9 test files) rather
//! than a local `minimal_cfg` — one fewer `Config` construction site
//! to maintain.

use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn healthz_returns_200() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(Request::builder().uri("/api/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok\n");
}

#[tokio::test]
async fn placeholder_root_is_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&body).unwrap();
    assert!(body.starts_with("accelerator-visualiser "));
}

#[tokio::test]
async fn host_header_guard_still_rejects_foreign_hosts() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let req = Request::builder()
        .uri("/api/healthz")
        .header("host", "example.com")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
```

### Success Criteria

#### Automated Verification

- [x] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --test router_compose` exits 0 with 3 tests passing.
- [x] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --tests` exits 0 (all Phase 2 integration tests still green).
- [ ] `mise run test` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: the `router_compose.rs` test file lands in its own revision before the `AppState::build`/`build_router` implementation. All three tests fail at compile time in the red revision because the symbols don't yet exist.
- [ ] **Mutation smoke test**: temporarily omit `.route("/api/healthz", ...)` from `build_router`, run `cargo test --test router_compose`, observe `healthz_returns_200` fails (404 instead of 200); restore.
- [ ] Start the real server (`ACCELERATOR_VISUALISER_BIN=... launch-server.sh`), curl `/api/healthz`, get `ok\n`.
- [ ] `/` still returns the Phase 2 placeholder — Phase 2's `shutdown.rs` integration tests must still pass without modification.

---

## Phase 3.9: API routes

### Overview

Populate the `src/api/` module directory with the seven read-only routes from the
Desired End State: `GET /api/types`, `GET /api/docs`,
`GET /api/docs/{*path}`, `GET /api/templates`,
`GET /api/templates/:name`, `GET /api/lifecycle`,
`GET /api/lifecycle/:slug`. Templates get their own namespace
(`/api/templates`) rather than being special-cased inside the
`/api/docs` routes — the response shapes are different
(`TemplateSummary`/`TemplateDetail` vs `IndexEntry`) and the route
collision between `/api/docs/templates/:name` and `/api/docs/*path`
is eliminated. Conditional GET semantics (`If-None-Match` → 304)
apply to `/api/docs/{*path}` only. Every route composes with the
Phase 2 middleware stack (host-guard, body-limit, timeout, activity)
via the `mount` helper.

### Changes Required

#### 1. API module — split into `src/api/`

Phase 3.8 created `src/api/mod.rs` as a skeleton. Phase 3.9 populates
the module directory with one file per concern. The public surface is
unchanged: `crate::api::mount(state)` is the only entry point consumed
by `build_router`.

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`
**Changes**: Full replacement of the Phase 3.8 skeleton.

```rust
//! HTTP API surface. One sub-module per route group.

mod types;
mod docs;
mod templates;
mod lifecycle;

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use crate::server::AppState;

pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/types", get(types::types))
        .route("/api/docs", get(docs::docs_list))
        .route("/api/docs/*path", get(docs::doc_fetch))
        .route("/api/templates", get(templates::templates_list))
        .route("/api/templates/:name", get(templates::template_detail))
        .route("/api/lifecycle", get(lifecycle::lifecycle_list))
        .route("/api/lifecycle/:slug", get(lifecycle::lifecycle_one))
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("invalid doc type: {0}")]
    InvalidDocType(String),
    #[error("path escape")]
    PathEscape,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::InvalidDocType(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::PathEscape => (StatusCode::FORBIDDEN, "path escape".into()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (
            status,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response()
    }
}

pub(crate) fn parse_kind(s: &str) -> Option<crate::docs::DocTypeKey> {
    serde_json::from_str::<crate::docs::DocTypeKey>(&format!("\"{s}\"")).ok()
}

pub(crate) fn api_from_fd(e: crate::file_driver::FileDriverError) -> ApiError {
    use crate::file_driver::FileDriverError as F;
    match e {
        F::PathEscape { .. } | F::TypeNotConfigured { .. } => ApiError::PathEscape,
        F::NotFound { path } => ApiError::NotFound(path.display().to_string()),
        F::TooLarge { path, size, limit } => ApiError::Internal(
            format!("{} is {} bytes (limit {})", path.display(), size, limit),
        ),
        F::Io { source, .. } => ApiError::Internal(source.to_string()),
    }
}
```

**File**: `skills/visualisation/visualise/server/src/api/types.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::docs::describe_types;
use crate::server::AppState;

#[derive(Serialize)]
pub(crate) struct TypesResponse {
    types: Vec<crate::docs::DocType>,
}

pub(crate) async fn types(
    State(state): State<Arc<AppState>>,
) -> Json<TypesResponse> {
    Json(TypesResponse {
        types: describe_types(&state.cfg),
    })
}
```

**File**: `skills/visualisation/visualise/server/src/api/docs.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::docs::DocTypeKey;
use crate::indexer::IndexEntry;
use crate::server::AppState;
use super::{ApiError, api_from_fd, parse_kind};

#[derive(Debug, Deserialize)]
struct DocsListQuery {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Serialize)]
struct DocsListResponse {
    docs: Vec<IndexEntry>,
}

pub(crate) async fn docs_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DocsListQuery>,
) -> Result<Response, ApiError> {
    let kind = parse_kind(&q.type_).ok_or(ApiError::InvalidDocType(q.type_.clone()))?;
    if kind == DocTypeKey::Templates {
        return Err(ApiError::InvalidDocType(q.type_.clone()));
    }
    let mut entries: Vec<IndexEntry> = state.indexer.all_by_type(kind).await;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Json(DocsListResponse { docs: entries }).into_response())
}

pub(crate) async fn doc_fetch(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if path.contains("..") || path.starts_with('/') {
        return Err(ApiError::PathEscape);
    }
    let abs = state.cfg.project_root.join(&path);

    let entry = state.indexer.get(&abs).await;
    if let Some(ref e) = entry {
        if let Some(inm) = headers.get("if-none-match") {
            let quoted = format!("\"{}\"", e.etag);
            if inm.to_str().ok() == Some(&quoted) || inm.to_str().ok() == Some(&e.etag) {
                return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, quoted)]).into_response());
            }
        }
    }

    let (etag, bytes) = match entry {
        Some(e) => {
            let content = state
                .file_driver
                .read(&e.path)
                .await
                .map_err(api_from_fd)?;
            (e.etag, content.bytes)
        }
        None => {
            let content = state.file_driver.read(&abs).await.map_err(api_from_fd)?;
            if let Some(inm) = headers.get("if-none-match") {
                let quoted = format!("\"{}\"", content.etag);
                if inm.to_str().ok() == Some(&quoted) || inm.to_str().ok() == Some(&content.etag) {
                    return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, quoted)]).into_response());
                }
            }
            (content.etag, content.bytes)
        }
    };

    Ok((
        StatusCode::OK,
        [
            (header::ETAG, HeaderValue::from_str(&format!("\"{etag}\"")).unwrap()),
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}
```

**File**: `skills/visualisation/visualise/server/src/api/templates.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use axum::{extract::{Path as AxumPath, State}, Json};
use serde::Serialize;

use crate::server::AppState;
use super::ApiError;

#[derive(Serialize)]
pub(crate) struct TemplatesListResponse {
    templates: Vec<crate::templates::TemplateSummary>,
}

pub(crate) async fn templates_list(
    State(state): State<Arc<AppState>>,
) -> Json<TemplatesListResponse> {
    Json(TemplatesListResponse {
        templates: state.templates.list(),
    })
}

pub(crate) async fn template_detail(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<crate::templates::TemplateDetail>, ApiError> {
    state.templates.detail(&name).map(Json).ok_or(ApiError::NotFound(name))
}
```

**File**: `skills/visualisation/visualise/server/src/api/lifecycle.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use axum::{extract::{Path as AxumPath, State}, Json};
use serde::Serialize;

use crate::server::AppState;
use super::ApiError;

#[derive(Serialize)]
pub(crate) struct LifecycleListResponse {
    clusters: Vec<crate::clusters::LifecycleCluster>,
}

pub(crate) async fn lifecycle_list(
    State(state): State<Arc<AppState>>,
) -> Json<LifecycleListResponse> {
    Json(LifecycleListResponse {
        clusters: state.clusters.read().await.clone(),
    })
}

pub(crate) async fn lifecycle_one(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Json<crate::clusters::LifecycleCluster>, ApiError> {
    let all = state.clusters.read().await;
    all.iter()
        .find(|c| c.slug == slug)
        .cloned()
        .map(Json)
        .ok_or(ApiError::NotFound(slug))
}
```

#### 2. Integration tests per route

**File**: `skills/visualisation/visualise/server/tests/api_types.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn types_returns_ten_entries_with_virtual_flag_on_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(Request::builder().uri("/api/types").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["types"].as_array().unwrap();
    assert_eq!(arr.len(), 10);
    let templates = arr.iter().find(|t| t["key"] == "templates").unwrap();
    assert_eq!(templates["virtual"], true);
    assert!(templates["dirPath"].is_null());
    let decisions = arr.iter().find(|t| t["key"] == "decisions").unwrap();
    assert!(decisions.get("virtual").is_none());
    assert!(decisions["dirPath"].is_string());
}
```

**File**: `skills/visualisation/visualise/server/tests/api_docs.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn docs_list_returns_index_entries_for_decisions() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=decisions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["docs"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "decisions");
    assert!(arr[0]["etag"].as_str().unwrap().starts_with("sha256-"));
}

#[tokio::test]
async fn docs_list_rejects_templates_virtual_type() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn docs_list_rejects_unknown_type() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=whatever")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn doc_fetch_returns_body_with_strong_etag() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state.clone());
    let rel = "meta/decisions/ADR-0001-foo.md";
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{rel}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let etag = res
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(etag.contains("sha256-"));

    // Round-trip with If-None-Match gets 304.
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{rel}"))
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn doc_fetch_rejects_path_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn doc_fetch_returns_404_for_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/meta/decisions/ADR-9999-nonexistent.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn doc_fetch_handles_percent_encoded_path() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    // %2D is a URL-encoded hyphen; axum decodes the wildcard before
    // passing it to the handler, so this should resolve to the same
    // file as the unencoded path.
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/meta/decisions/ADR%2D0001%2Dfoo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn docs_list_returns_empty_array_for_type_with_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=tickets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["docs"].as_array().unwrap().len(), 0);
}
```

**File**: `skills/visualisation/visualise/server/tests/api_templates.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn templates_list_returns_all_configured_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["templates"].as_array().unwrap();
    assert!(arr.iter().any(|s| s["name"] == "adr"));
}

#[tokio::test]
async fn template_detail_returns_three_tiers_with_plugin_default_active() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/adr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["name"], "adr");
    let tiers = v["tiers"].as_array().unwrap();
    assert_eq!(tiers.len(), 3);
    // With the seeded common config: only plugin_default exists.
    let active: Vec<&serde_json::Value> = tiers.iter().filter(|t| t["active"] == true).collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0]["source"], "plugin-default");
}

#[tokio::test]
async fn template_detail_for_unknown_name_returns_404() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

**File**: `skills/visualisation/visualise/server/tests/api_lifecycle.rs`
**Changes**: New file.

```rust
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn lifecycle_list_groups_entries_by_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["clusters"].as_array().unwrap();
    // Seeded config has one plan with slug "foo" plus an ADR with slug
    // "foo"; expect exactly one cluster with both entries.
    let foo = arr.iter().find(|c| c["slug"] == "foo").unwrap();
    assert!(foo["entries"].as_array().unwrap().len() >= 2);
    assert_eq!(foo["completeness"]["hasPlan"], true);
    assert_eq!(foo["completeness"]["hasDecision"], true);
}

#[tokio::test]
async fn lifecycle_one_returns_single_cluster_by_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/foo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["slug"], "foo");
}

#[tokio::test]
async fn lifecycle_unknown_slug_is_404() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

#### 3. Shared test helper

**File**: `skills/visualisation/visualise/server/tests/common/mod.rs`
**Changes**: Already created in Phase 3.8 (step 4). No changes needed —
the Phase 3.9 test files consume `common::seeded_cfg` as-is.

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --test api_types --test api_docs --test api_templates --test api_lifecycle --test router_compose` exits 0 with all tests passing.
- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --tests` exits 0 (no Phase 2 regression).
- [ ] `mise run test:integration` exits 0.
- [ ] `mise run test` exits 0.

#### Manual Verification

- [ ] **TDD ordering**: each `api_*.rs` test file is committed in its own `jj` revision before the matching handler is implemented in `src/api/`. For each, `cargo test --test <name>` in the red revision fails (either at compile time because the handler doesn't return the expected shape, or at runtime with 404/500).
- [ ] **Mutation smoke test** for `If-None-Match`: temporarily change the 304 branch in `doc_fetch` to always return 200, run `cargo test --test api_docs`, observe `doc_fetch_returns_body_with_strong_etag`'s 304 assertion fails; restore.
- [ ] **Mutation smoke test** for path escape: temporarily remove the `path.contains("..")` check, run `cargo test --test api_docs`, observe `doc_fetch_rejects_path_escape` fails; restore.
- [ ] Live smoke: `ACCELERATOR_VISUALISER_BIN=... launch-server.sh` against the workspace, then:
    - `curl -s '<url>/api/types' | jq '.types | length'` → 10
    - `curl -s '<url>/api/docs?type=decisions' | jq '.docs | length'` → 21
    - `curl -s '<url>/api/docs?type=plan-reviews' | jq '.docs | length'` → 8
    - `curl -s '<url>/api/templates' | jq '.templates | length'` → 5
    - `curl -si '<url>/api/docs/meta/decisions/ADR-0001-context-isolation-principles.md'` → `200 OK` with `ETag: "sha256-..."`
    - Re-run with `-H 'If-None-Match: "<etag>"'` → `304`
    - `curl -s '<url>/api/lifecycle' | jq '.clusters | length'` → non-zero

---

## Phase 3.10: End-to-end fixture tree and smoke test

### Overview

Commit a realistic `server/tests/fixtures/meta/` tree covering the
edge cases the earlier sub-phases exercised only via inline
tempdir writes: absent-FM, malformed-FM, `-review-N` suffix with
embedded `-review-` internally, and `ticket:` in both `null` and
`""` forms. Add `server/tests/api_smoke.rs` that spawns the real
binary (`env!("CARGO_BIN_EXE_accelerator-visualiser")`) against a
`config.json` that points at the fixture tree, then curls every
endpoint via `reqwest` — the same pattern Phase 2 uses for
`shutdown.rs`. This validates the binary + launcher + API stack
end-to-end, not just the axum router.

### Changes Required

#### 1. Fixture meta tree

**Directory**: `skills/visualisation/visualise/server/tests/fixtures/meta/`
**Changes**: Commit the following files.

Files per type (each with deliberately-varied frontmatter to cover
edge cases):

- `meta/decisions/ADR-0001-example-decision.md` — fully-populated
  frontmatter (`adr_id`, `title`, `status`, `tags`).
- `meta/decisions/ADR-0002-another-decision.md` — same.
- `meta/tickets/0001-first-ticket.md` — frontmatter with
  `status: todo`.
- `meta/tickets/0002-second-ticket.md` — `status: done`.
- `meta/tickets/0003-third-ticket.md` — `status:` absent (tests
  "Other" swimlane handling in Phase 7 without blocking Phase 3).
- `meta/plans/2026-01-01-first-plan.md` — full frontmatter.
- `meta/plans/2026-01-02-ancient-plan.md` — **no frontmatter**
  (Absent state).
- `meta/plans/2026-01-03-malformed-plan.md` — malformed YAML
  (unclosed quoted string).
- `meta/research/2026-01-01-first-research.md` — full frontmatter.
- `meta/reviews/plans/2026-01-01-first-plan-review-1.md` — first
  review, `target:` populated.
- `meta/reviews/plans/2026-01-04-example-and-review-some-topic-review-1.md` —
  **regression fixture**: slug contains `-review-` internally; must
  cluster to slug `example-and-review-some-topic`.
- `meta/notes/2026-01-01-first-note.md` — no frontmatter (matches
  2 of 3 live notes).
- `meta/validations/.gitkeep` — empty dir.
- `meta/prs/.gitkeep` — empty dir.
- `meta/reviews/prs/.gitkeep` — empty dir.

Templates tree:

- `templates/adr.md`, `templates/plan.md`, `templates/research.md`,
  `templates/validation.md`, `templates/pr-description.md` —
  minimal plugin-default bodies.

The full content of each fixture file is deliberately short
(one-paragraph bodies) so diffs in this directory are cheap and
human-readable.

#### 2. Smoke-test integration

**File**: `skills/visualisation/visualise/server/tests/api_smoke.rs`
**Changes**: New file.

```rust
//! End-to-end API smoke test. Spawns the real Rust binary against a
//! config.json pointing at `tests/fixtures/meta/`, then hits every
//! API surface via `reqwest`. Complements the per-route
//! `ServiceExt::oneshot` tests in Phase 3.9 by validating the
//! binary + clap + tracing + axum stack as a whole.

use std::process::Stdio;
use std::time::Duration;

use serde_json::json;
use tokio::process::Command;

#[tokio::test]
async fn api_surface_is_fully_reachable_against_fixture_meta() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/meta");
    let plugin_templates = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/templates");
    let tmp = tempfile::tempdir().unwrap();
    let cfg_path = tmp.path().join("config.json");
    let tmp_dir = tmp.path().join("visualiser");
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let mut doc_paths = serde_json::Map::new();
    for (key, rel) in [
        ("decisions", "decisions"),
        ("tickets", "tickets"),
        ("plans", "plans"),
        ("research", "research"),
        ("review_plans", "reviews/plans"),
        ("review_prs", "reviews/prs"),
        ("validations", "validations"),
        ("notes", "notes"),
        ("prs", "prs"),
    ] {
        doc_paths.insert(key.into(), json!(fixtures.join(rel)));
    }
    let mut templates = serde_json::Map::new();
    for name in ["adr", "plan", "research", "validation", "pr-description"] {
        templates.insert(
            name.into(),
            json!({
                "config_override": null,
                "user_override": fixtures.join(format!("templates/{name}.md")),
                "plugin_default": plugin_templates.join(format!("{name}.md")),
            }),
        );
    }
    let cfg = json!({
        "plugin_root": fixtures.parent().unwrap(),
        "plugin_version": "0.0.0-smoke",
        "project_root": fixtures.parent().unwrap(),
        "tmp_path": tmp_dir,
        "host": "127.0.0.1",
        "owner_pid": 0,
        "owner_start_time": null,
        "log_path": tmp_dir.join("server.log"),
        "doc_paths": doc_paths,
        "templates": templates,
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = Command::new(bin)
        .arg("--config")
        .arg(&cfg_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();

    // Poll for server-info.json (5s bound).
    let info_path = tmp_dir.join("server-info.json");
    let start = std::time::Instant::now();
    loop {
        if info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill().await;
            panic!("server-info.json did not appear in 5s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let info: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&info_path).unwrap()).unwrap();
    let base = info["url"].as_str().unwrap().trim_end_matches('/').to_string();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // /api/types → 10 entries.
    let t: serde_json::Value = client.get(format!("{base}/api/types")).send().await.unwrap().json().await.unwrap();
    assert_eq!(t["types"].as_array().unwrap().len(), 10);

    // /api/docs?type=decisions → 2 entries.
    let d: serde_json::Value = client.get(format!("{base}/api/docs?type=decisions")).send().await.unwrap().json().await.unwrap();
    assert_eq!(d["docs"].as_array().unwrap().len(), 2);

    // /api/docs?type=plan-reviews → 2 entries with expected slugs.
    let pr: serde_json::Value = client.get(format!("{base}/api/docs?type=plan-reviews")).send().await.unwrap().json().await.unwrap();
    let slugs: Vec<&str> = pr["docs"].as_array().unwrap().iter().map(|e| e["slug"].as_str().unwrap()).collect();
    assert!(slugs.contains(&"first-plan"));
    assert!(slugs.contains(&"example-and-review-some-topic"));

    // /api/templates → 5 entries.
    let tpl: serde_json::Value = client.get(format!("{base}/api/templates")).send().await.unwrap().json().await.unwrap();
    assert_eq!(tpl["templates"].as_array().unwrap().len(), 5);

    // /api/docs/{*path} with If-None-Match round-trip.
    let r1 = client
        .get(format!("{base}/api/docs/meta/decisions/ADR-0001-example-decision.md"))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    let etag = r1.headers().get("etag").unwrap().to_str().unwrap().to_string();
    let r2 = client
        .get(format!("{base}/api/docs/meta/decisions/ADR-0001-example-decision.md"))
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 304);

    // /api/lifecycle returns a non-empty cluster list.
    let lc: serde_json::Value = client.get(format!("{base}/api/lifecycle")).send().await.unwrap().json().await.unwrap();
    assert!(!lc["clusters"].as_array().unwrap().is_empty());

    // Clean up.
    let _ = child.kill().await;
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --test api_smoke` exits 0.
- [ ] `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --tests` exits 0.
- [ ] `mise run test` exits 0 (unit + integration levels, including the new api_smoke suite).
- [ ] `ls skills/visualisation/visualise/server/tests/fixtures/meta/decisions/ADR-*.md | wc -l` ≥ 2.
- [ ] `ls skills/visualisation/visualise/server/tests/fixtures/meta/reviews/plans/*-review-*.md | wc -l` ≥ 2.

#### Manual Verification

- [ ] **TDD ordering**: the fixture tree and `api_smoke.rs` test land before any handler refactor; the smoke test must pass cleanly on the Phase 3.9 codebase, so "red" here is operational (the tree doesn't exist → `env!` resolves but the path doesn't, and `api_smoke` panics on `tempdir` setup).
- [ ] **Mutation smoke test**: move one of the `meta/reviews/plans/*-review-*.md` fixture files out of the reviews directory into `meta/plans/`; run `cargo test --test api_smoke`, observe the `plan-reviews` count assertion or the slug assertion fails; restore.
- [ ] **Browser check** (manual): run `ACCELERATOR_VISUALISER_BIN=$(cargo metadata --format-version 1 | jq -r '.target_directory')/debug/accelerator-visualiser skills/visualisation/visualise/scripts/launch-server.sh` in a real shell, open `http://127.0.0.1:<port>/api/types` in a browser, confirm it renders a JSON object with a `types` array of 10 DocType entries (browsers format JSON via the server's default `application/json` response; use a JSON-viewer extension or view-source).
- [ ] **Live workspace smoke**: against the real `meta/` tree (not the fixture), confirm the API returns the live 21 ADRs, 29 tickets, 28 plans, 19 research, 8 plan-reviews.

---

## Testing Strategy

### Assertion discipline

Every `matches!` used as an assertion is wrapped in `assert!(...)`, same
as Phase 2. Reviewers run `grep -rn '^\s*matches!' skills/visualisation/visualise/server/src
skills/visualisation/visualise/server/tests` on each Phase 3 revision;
every hit must be inside an `assert!` (or a `.filter(|x| matches!(...))`
expression, which is functional, not an assertion).

### Unit Tests (cargo)

Colocated `#[cfg(test)] mod tests` per module:

- **`docs`** — DocTypeKey serde round-trip, `all()` covers 10 distinct
  variants, `in_kanban`/`in_lifecycle`/`is_virtual` flags, `describe_types`
  populates `dir_path` from config, `virtual` omitted from JSON when false.
- **`slug`** — per-DocType table-driven cases; the `-review-` embedded
  regression; templates always return None; non-md files always return None.
- **`frontmatter`** — parsed/absent/malformed triage; title cascade;
  `ticket_of` treats `null`/`""`/missing identically; block-sequence
  YAML edge case; CRLF; invalid UTF-8.
- **`file_driver`** — list filters to `.md` and skips `.gitkeep`; missing
  dir is `Ok(vec![])`; unconfigured type is an error; read returns
  bytes + strong sha256 ETag; path-escape rejected; symlink-escape
  rejected; missing file is NotFound; ETag is stable hex.
- **`indexer`** — scan populates per-type; ETag is content hash;
  frontmatter-state triage; slug derivation including the embedded
  `-review-` edge case; title fallback; adr_by_id lookup; rescan picks
  up new files; malformed entries are still addressable by path.
- **`templates`** — three-tier priority; active-tier selection; list vs
  detail content inclusion; unknown name → None; ETag stability.
- **`clusters`** — same-slug groups; canonical ordering; mtime tie-break;
  completeness flags; templates excluded; no-slug excluded; clusters
  alphabetically sorted.

### Integration Tests (cargo)

Separate `server/tests/*.rs` files driving the full router via
`tower::ServiceExt::oneshot`:

- **`router_compose.rs`** — `/api/healthz` returns 200; `/` preserved;
  host-header-guard still rejects foreign hosts.
- **`api_types.rs`** — `types` wrapper with 10-entry array; templates
  has `virtual: true` + `dirPath: null`; ordinary types have string
  `dirPath` + no `virtual`.
- **`api_docs.rs`** — docs list returns `docs` wrapper with IndexEntries
  for ordinary types; `type=templates` is rejected (400); unknown type
  is 400; empty type returns `{ "docs": [] }`; doc fetch returns strong
  ETag; `If-None-Match` → 304; path escape → 403; missing file → 404;
  percent-encoded path → 200.
- **`api_templates.rs`** — templates list returns `templates` wrapper
  with 5 summaries; template
  detail returns three tiers; unknown name → 404.
- **`api_lifecycle.rs`** — `clusters` wrapper with slug grouping;
  lifecycle one returns single cluster; unknown slug → 404.
- **`api_smoke.rs`** — end-to-end spawn-binary + fixture-meta smoke.

Phase 2 integration tests (`shutdown.rs`, `lifecycle_idle.rs`,
`lifecycle_owner.rs`, `config_cli.rs`, `config_contract.rs`,
`graceful_draining.rs`) are unchanged and must still pass.

### Integration Tests (bash)

No new bash harnesses. Phase 2's `test-launch-server.sh`,
`test-stop-server.sh`, and `test-cli-wrapper.sh` continue to guard the
launcher contract; the API surface is all-cargo-tested because it lives
inside the Rust binary.

### Manual Testing Steps

1. Build: `cargo build --release --manifest-path skills/visualisation/visualise/server/Cargo.toml`.
2. Export `ACCELERATOR_VISUALISER_BIN=$(pwd)/skills/visualisation/visualise/server/target/release/accelerator-visualiser`.
3. `bash skills/visualisation/visualise/scripts/launch-server.sh` — confirm
   `**Visualiser URL**: http://127.0.0.1:<port>`.
4. `curl -s '<url>/api/types' | jq '.types | length'` → 10.
5. `curl -s '<url>/api/docs?type=decisions' | jq '.docs[0] | {type, slug, etag, title}'`
   returns a well-formed entry.
6. `curl -sI '<url>/api/docs/meta/decisions/ADR-0001-context-isolation-principles.md' | grep -i etag`
   → `ETag: "sha256-<hex>"`.
7. Re-run with `-H 'If-None-Match: <etag>'` → status 304.
8. `curl -s '<url>/api/docs?type=plan-reviews' | jq '[.docs[] | .slug]'` — confirm
   `initialise-skill-and-review-pr-ephemeral-migration` is present (regression
   target).
9. `curl -s '<url>/api/lifecycle' | jq '.clusters | length'` — non-zero; `jq '.clusters[] | select(.slug==\"meta-visualiser-phase-2-server-bootstrap\")'`
   returns a cluster with the Phase 2 plan + review.
10. `curl -sI '<url>/api/docs/../../etc/passwd'` → 403.
11. Shutdown: `bash skills/visualisation/visualise/scripts/stop-server.sh`
    → `{"status":"stopped"}`. Browser now gets connection refused.

## Performance Considerations

- **Initial scan latency**: the indexer walks every configured directory
  serially via `tokio::fs`, reading each file once. On the live workspace
  (~100 markdown files across all types) the warm-cache scan completes
  in under 50 ms on modern hardware. The spec's non-functional target is
  "initial scan of ~2000 files under 1s"; enforced by the
  `scan_2000_files_completes_within_one_second` integration test in the
  indexer module.
- **ETag compute cost**: `sha2::Sha256` on a 10 KB markdown file is
  sub-microsecond; the scan cost is dominated by read-syscall latency,
  not hashing. No need for xxhash or faster alternatives.
- **Cluster-cache freshness**: clusters are re-derived at startup only
  in Phase 3; Phase 4's watcher adds fine-grained invalidation. For the
  current "companion window" use case this is acceptable — a full
  server restart is cheap.
- **Memory budget**: one `IndexEntry` per document with a `Vec<u8>`
  content held only transiently (released after the read handler
  responds) and an ETag cached as ~70 bytes of hex. 2000 entries with
  average 10 KB bodies = ~20 MB at peak, far below the budget.
- **Route-level latency**: `/api/docs?type=decisions` iterates the
  in-memory index; sub-millisecond for any realistic repo. `/api/docs/{*path}`
  re-reads the file to stream bytes; single-digit-millisecond on warm
  cache. No caching beyond what the OS page cache provides.

## Migration Notes

Phase 3 **adds** the following files that did not exist in Phase 2:

- `skills/visualisation/visualise/server/src/{docs,slug,frontmatter,file_driver,indexer,templates,clusters}.rs`
- `skills/visualisation/visualise/server/src/api/{mod,types,docs,templates,lifecycle}.rs`
- `skills/visualisation/visualise/server/tests/common/mod.rs`
- `skills/visualisation/visualise/server/tests/router_compose.rs`
- `skills/visualisation/visualise/server/tests/api_types.rs`
- `skills/visualisation/visualise/server/tests/api_docs.rs`
- `skills/visualisation/visualise/server/tests/api_templates.rs`
- `skills/visualisation/visualise/server/tests/api_lifecycle.rs`
- `skills/visualisation/visualise/server/tests/api_smoke.rs`
- `skills/visualisation/visualise/server/tests/fixtures/meta/**/*.md`
- `skills/visualisation/visualise/server/tests/fixtures/templates/*.md`

Phase 3 **modifies** the following files:

- `skills/visualisation/visualise/server/Cargo.toml` — adds
  `sha2`, `gray_matter`, `serde_yml`, `hex` as runtime deps;
  `tower` (util), `http_body_util` as dev-deps.
- `skills/visualisation/visualise/server/src/lib.rs` — adds
  `pub mod` entries for the eight new modules.
- `skills/visualisation/visualise/server/src/server.rs` — extends
  `AppState`; extracts `build_router`; delegates `run`; adds
  `AppStateError` and `ServerError::Startup`. The existing `process_start_time`,
  `spawn_signal_handlers`, `write_server_stopped`, `write_server_info`,
  `write_pid_file`, `host_header_guard`, and `placeholder_root`
  helpers are unchanged.

Phase 3 **does not modify**:

- `scripts/launch-server.sh`, `scripts/stop-server.sh`.
- `scripts/write-visualiser-config.sh` — except for the single
  `project_root` addition in Phase 3.8.
- `skills/visualisation/visualise/SKILL.md`.
- `.gitignore` — no new ignore rules required.
- `mise.toml`, `tasks/test/*.py` — no new test levels or components.
- `.claude-plugin/plugin.json`.

Known Phase 4+ churn points introduced by this phase:

- **`AppState` shape.** Phase 4 extends it with `sse_hub` and replaces
  the `Arc<RwLock<Vec<LifecycleCluster>>>` seed with a
  reactively-updated view; expect `AppState::build` to grow a
  `spawn_watcher` call.
- **`FileDriver` trait.** Phase 4 adds `fn watch(...)`; Phase 8 adds
  `fn write_frontmatter(...)`. `LocalFileDriver` picks up impls; any
  future `GithubFileDriver` implements both.
- **Cluster cache.** Phase 4 replaces the eager startup seed with an
  invalidate-on-watch-event approach; the `RwLock` stays but the
  update path becomes event-driven.
- **`Indexer::rescan`** is exposed in Phase 3 for tests. Phase 4 adds
  per-path `update_path`/`forget_path` methods; `rescan` is preserved
  as the full-rebuild fallback (also useful for SIGHUP in Phase 10).
- **`doc-invalid` SSE event.** Phase 3 records `frontmatterState:
  "malformed"` on affected IndexEntries; Phase 4 broadcasts the
  corresponding event.
- **Template orphans.** Phase 3 ignores tier-2 files that lack a
  tier-3 peer; Phase 5's library view will surface them under an
  "unregistered" badge.

## References

- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  (Phase 3 ownership: Gaps 1, 3, 6, 7, 10, 12; resolved decisions D5, D6,
  D7, D9).
- Design spec: `meta/specs/2026-04-17-meta-visualisation-design.md`
  (sections: § Data model, § Views § Library, § Writes and conflict
  handling § ETag definition, § Testing strategy).
- Phase 1 plan: `meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md`.
- Phase 2 plan: `meta/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap.md`.
- ADR-0017 (Configuration extension points): `meta/decisions/ADR-0017-configuration-extension-points.md`
  — authoritative for the three-tier template resolution the Phase 3.6
  resolver mirrors.
- Config path keys: `scripts/config-read-path.sh:7-18`.
- Repo-root resolver: `scripts/vcs-common.sh:8-18`.
- YAML block-sequence edge cases: `meta/notes/2026-03-24-yaml-block-sequence-array-parsing.md`.
- axum `ServiceExt::oneshot` pattern: `tower::util::ServiceExt` (tower 0.5).
- Gray-matter YAML engine: `gray_matter` crate docs; `serde_yml` as the
  YAML backend.
- SHA-256 ETag format: matches the spec's "strong validator (`sha256-<hex>`,
  no `W/` prefix)" definition in § ETag definition.
