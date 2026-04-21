---
date: "2026-04-21T10:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-21-meta-visualiser-phase-3-file-driver-indexer-api.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, security, performance, usability]
review_pass: 1
status: complete
---

## Plan Review: Meta Visualiser — Phase 3: FileDriver, Indexer, and read-only API

**Verdict:** REVISE

The plan is detailed, well-structured, and exercises serious TDD discipline
across ten sub-phases. Module boundaries are clean, the trait-staged
evolution for `FileDriver` is thoughtful, and edge-case coverage for the
critical slug/frontmatter logic is strong. However, a high-confidence
off-by-one in the `project_root = cfg.tmp_path.ancestors().nth(2)`
derivation — replicated at two sites and relied on by both the
integration tests and the `/api/docs/{*path}` handler — will make
the plan's own Green-step tests fail and blocks Desired End State
bullet 5. Several other structural concerns (a polymorphic
`/api/docs?type=...` response shape, `TemplateResolver` bypassing
the `FileDriver` abstraction, `Indexer<D>` generic erasing the
trait at `AppState`, no size cap on file reads, unactionable error
bodies) warrant addressing before Phase 3.9 lands.

### Cross-Cutting Themes

- **`project_root = tmp_path.ancestors().nth(2)` is wrong** (flagged by:
  architecture, correctness, test-coverage, security, code-quality) —
  yields `<repo>/meta`, not `<repo>`. Breaks `/api/docs/{*path}` for
  every URL under `meta/...`, makes `doc_fetch_returns_body_with_strong_etag`
  and `api_smoke.rs` unreachable on 200, and leaves the indexer's
  `rel_path` offset by one segment. Root cause: inferring repo root from
  `tmp_path` depth instead of threading `project_root` through config.
- **`If-None-Match` / ETag cache path has multiple defects** (flagged by:
  code-quality, test-coverage, performance, correctness, usability) —
  accepts both quoted and unquoted forms without testing the unquoted
  branch; `doc_fetch` re-reads + re-hashes bytes even for the 304 case;
  canonical-vs-non-canonical lookup key mismatch between indexer storage
  and `Indexer::get`; RFC 7232 `*` / weak / multi-tag forms ignored.
- **`TemplateResolver` bypasses the `FileDriver` abstraction**
  (flagged by: architecture, code-quality, performance) — synchronous
  `std::fs::read` inside an async `AppState::build`; no path-safety
  guarantees on tier paths; future `GithubFileDriver` swap cannot cover
  templates transparently.
- **`FileDriver` trait abstraction partially defeated**
  (flagged by: architecture, usability) — `Indexer<D: FileDriver>` is
  concretised at `AppState` as `Arc<Indexer<LocalFileDriver>>`, and
  `DocTypeKey` match arms with `_ => {}` catchalls silently lose new
  variants — the trait exists syntactically but not in practice.
- **Test config fixtures will drift** (flagged by: test-coverage,
  usability) — `router_compose.rs::minimal_cfg` duplicates
  `common::seeded_cfg` with different layouts, and the `api_smoke.rs`
  fixture config is hand-rolled rather than produced by
  `write-visualiser-config.sh`. Drift will eventually bite.
- **Non-unix symlink-escape test is incorrect** (flagged by:
  correctness, test-coverage) — `hard_link` fallback does not exercise
  symlink resolution; on non-Unix the test will fail for the wrong
  reason.

### Tradeoff Analysis

- **Performance vs simplicity (scan parallelism)**: the plan defers
  `JoinSet`-based parallel scanning to Phase 10. Performance lens warns
  the sequential scan risks missing the 2000-file/<1s NFR on cold
  caches; Architecture and Code-Quality lenses prefer the simpler
  serial version for now. Resolution: add the benchmark now so the
  deferral is data-driven rather than blind.
- **API shape consistency vs spec conformance**: Usability lens wants
  `/api/templates` split out from `/api/docs?type=templates`; the spec
  currently models templates as a DocType. Resolution: either update
  the spec or take the usability cost knowingly.
- **Defense in depth vs simplicity (path guard)**: Security lens
  treats the `contains("..")` check as heuristic; Correctness lens
  notes it's layered behind canonicalise+prefix check anyway. Both
  agree the layered model is fine — but the plan should comment the
  layering explicitly to prevent future refactors from removing
  either half.

### Findings

#### Critical

- 🔴 **Architecture / Correctness / Test-coverage**: `project_root = cfg.tmp_path.ancestors().nth(2)` yields `<repo>/meta`, not `<repo>`
  **Location**: Phase 3.8 (`AppState::build`), Phase 3.9 (`doc_fetch`)
  `tmp_path = <repo>/meta/tmp/visualiser` → `ancestors()` yields
  `[self, meta/tmp, meta, <repo>, /]` so `.nth(2) = <repo>/meta`.
  `doc_fetch` then joins URL path `meta/decisions/ADR-0001...md` to get
  `<repo>/meta/meta/decisions/...md` (nonexistent → 404). Breaks every
  per-doc fetch, makes `doc_fetch_returns_body_with_strong_etag` and
  `api_smoke.rs` 200-assertions unreachable, and offsets `IndexEntry.rel_path`
  by one segment. Fix by threading an explicit `project_root` field
  through `Config` (have `scripts/write-visualiser-config.sh` emit it)
  rather than inferring from `tmp_path`.

#### Major

- 🟡 **Architecture**: `TemplateResolver` reads disk directly, bypassing the FileDriver abstraction
  **Location**: Phase 3.6 `templates.rs::load()`
  Calls `std::fs::read` — the only module outside `file_driver.rs` that
  touches disk. Loses path-safety guarantees on tier paths, makes a
  future `GithubFileDriver` non-transparent, and gives Phase 4's
  template hot-reload no hook into the watcher. Suggest routing template
  reads through `FileDriver` or adding a template-aware root to
  `LocalFileDriver`.

- 🟡 **Architecture**: Generic `Indexer<D>` defeats the FileDriver abstraction at AppState
  **Location**: Phase 3.5 / 3.8
  `AppState` concretises as `Arc<Indexer<LocalFileDriver>>`, so every
  downstream caller is statically coupled to `LocalFileDriver`. Spec
  promises drop-in `GithubFileDriver` swap; current encoding requires a
  global recompile-and-edit. Suggest storing as `Arc<dyn FileDriver>`.

- 🟡 **Architecture**: `AppState`/`activity` split creates two parallel state containers
  **Location**: Phase 3.8 `build_router(state, activity)`
  `Activity` is passed externally; when Phase 4 adds `sse_hub` every
  test file grows a third parameter. Suggest moving `activity` into
  `AppState` or introducing an `AppComponents` bundle.

- 🟡 **Code-quality**: Error wrapping via `std::io::Error::new(... e.to_string())` discards structured info
  **Location**: Phase 3.8 `run` → `AppStateError` mapped to `ServerError::Bind`
  An indexer-build failure surfaces as a `Bind` error with a
  synthesised io::Error — the taxonomy lies, and the source chain is
  truncated. Add `ServerError::Startup { source: AppStateError }`.

- 🟡 **Code-quality / Performance**: Synchronous `std::fs::read` in `TemplateResolver::build` blocks the async runtime
  **Location**: Phase 3.6 `load()` called from async `AppState::build`
  Mixing `std::fs` and `tokio::fs` at the same layer is a maintenance
  hazard; Phase 4 template hot-reload will have to rewrite this.
  Make `build` async using `tokio::fs::read`.

- 🟡 **Code-quality**: `parse_kind` round-trips a string through `serde_json::from_str` to decode an enum
  **Location**: Phase 3.9 `api.rs::parse_kind`
  `serde_json::from_str::<DocTypeKey>(&format!("\"{s}\""))` manufactures
  a JSON literal just to reuse the kebab-case mapping. Quotes inside `s`
  would break the format. Use `serde_plain::from_str` or a
  `FromStr` impl.

- 🟡 **Code-quality**: `Indexer::rescan`'s silent `continue` on NotFound masks list/read race
  **Location**: Phase 3.5 `rescan` inner loop
  Silent skip turns a disk mutation during scan into an invisible
  missing entry. Add a `tracing::debug!` (or `warn!`) on the branch.

- 🟡 **Code-quality**: `HeaderValue::from_str(...).unwrap()` in the hot path
  **Location**: Phase 3.9 `doc_fetch`
  ETag is SHA-256 hex today so it cannot fail, but `unwrap` on a
  request handler panics a worker. Use `HeaderValue::from_static`
  where possible or return `Result` via `.map_err(...)`. Consider
  `axum_extra::TypedHeader<headers::ETag>`.

- 🟡 **Correctness**: Non-unix symlink-escape test uses `hard_link`
  **Location**: Phase 3.4 `read_rejects_symlink_escape`
  On `#[cfg(not(unix))]` it falls back to `std::fs::hard_link`, which
  does not redirect — the file ends up inside the root and the
  `PathEscape | NotFound` assertion will unwrap-err on an `Ok`.
  Gate the test `#[cfg(unix)]` only.

- 🟡 **Correctness**: Canonical-vs-non-canonical path-key mismatch between indexer storage and lookup
  **Location**: Phase 3.5 `Indexer::get`, Phase 3.9 `doc_fetch`
  The indexer stores entries keyed on canonical paths (because
  `LocalFileDriver::new` canonicalises roots). `doc_fetch` looks up by
  `project_root.join(url_path)` (non-canonical) with a
  `std::fs::canonicalize` fallback. Cache-hit path is effectively never
  taken; every request re-hashes. Canonicalise the lookup key once.

- 🟡 **Security**: No size cap on file reads — malicious-DoS via large files
  **Location**: Phase 3.4/3.5/3.9 — `LocalFileDriver::read`
  `tokio::fs::read` unconditionally buffers full file bytes. A single
  1 GB markdown file under any `doc_paths` directory OOMs the server at
  startup or on request. Add `MAX_DOC_BYTES` (8–16 MiB) with a
  `FileDriverError::TooLarge { path, size }` mapping to 413.

- 🟡 **Security**: Fragile `project_root` inference also weakens defense-in-depth
  **Location**: Phase 3.9 `doc_fetch`
  Combined with the critical `nth(2)` bug above: if `project_root`
  silently becomes `/`, the only remaining barrier is the
  canonicalise+prefix check. Make `project_root` explicit in `Config`.

- 🟡 **Performance**: `doc_fetch` re-reads + re-hashes files even for 304 hits
  **Location**: Phase 3.9 `doc_fetch`
  `file_driver.read(&e.path)` runs before the `If-None-Match` check;
  the re-computed `content.etag` is discarded in favour of the cached
  `e.etag`. Every polling refresh pays an avoidable disk round-trip.
  Check ETag first; only read bytes on a cache miss.

- 🟡 **Performance**: Sequential scan may miss 2000-file/<1s NFR on cold cache
  **Location**: Phase 3.5 `rescan`; Phase 3 § What We're NOT Doing
  2000 × ~2 ms cold-cache read ≈ 4 s, 4× the NFR. The "Phase 10 polish"
  deferral has no benchmark guarding it. Add `JoinSet` parallelism now
  (~20 LOC) or add a timed test against 2000 synthetic files.

- 🟡 **Test-coverage**: Percent-encoded `..` path escape not covered
  **Location**: Phase 3.9 `doc_fetch_rejects_path_escape`
  Only literal `../etc/passwd` is tested; axum's URL decode catches
  `%2e%2e` today but a future guard or extractor change would silently
  regress. Add `/%2e%2e/etc/passwd`, `/meta/%2e%2e/...`, and
  `//etc/passwd` as additional 403 cases.

- 🟡 **Test-coverage**: Unquoted `If-None-Match` branch is dead-letter
  **Location**: Phase 3.9 `doc_fetch_returns_body_with_strong_etag`
  Handler accepts both quoted and unquoted but test round-trips only
  the quoted form (response header). Add a second request stripping
  the quotes, or delete the unquoted branch if it isn't contractual.

- 🟡 **Test-coverage**: `rescan`'s NotFound-continue branch has no regression test
  **Location**: Phase 3.5 `rescan`
  The only place the plan implements a race-window policy has no test.
  Use a mock `FileDriver` (or tempfile+delete between list and read) to
  assert rescan returns `Ok(())` with the vanished path absent.

- 🟡 **Test-coverage**: Empty-directory list behaviour not asserted at HTTP layer
  **Location**: Phase 3.9 `api_docs.rs`
  Desired End State #9 (`type=prs` returns `[]` when empty) has no
  direct HTTP test. Only `decisions` and `templates` are asserted.
  Add `GET /api/docs?type=prs` → 200 + `[]` against `seeded_cfg`.

- 🟡 **Test-coverage**: `router_compose.rs::minimal_cfg` duplicates `common::seeded_cfg` with different layouts
  **Location**: Phase 3.8 vs 3.9
  Drift risk compounds as Phase 4+ adds tests. Delete `minimal_cfg`
  and consume `common::seeded_cfg` (or introduce `common::bare_cfg`).

- 🟡 **Usability**: `/api/docs?type=templates` returns a different JSON shape than other types
  **Location**: Phase 3.9 `docs_list`
  Ordinary types return `IndexEntry[]`; `type=templates` returns
  `TemplateSummary[]`. Forces every consumer to switch on the query
  string they sent. Promote to `/api/templates`; leave `/api/docs` with
  a single uniform meaning.

- 🟡 **Usability**: Error bodies are unactionable and debug-formatted
  **Location**: Phase 3.9 `ApiError::IntoResponse`
  `InvalidDocType({0:?})` produces `"invalid doc type: \"whatever\""`
  (debug-quoted inside JSON escaping). `NotFound`/`PathEscape` give no
  hint what was missing or which path escaped. Use `Display`, list
  valid enums, differentiate NotFound per-route.

- 🟡 **Usability**: `/api/docs/templates/:name` collides visually with `/api/docs/{*path}`
  **Location**: Phase 3.9 `mount`
  One URL shape yields JSON, the other yields markdown. Axum precedence
  resolves it, but consumer repos with a literal `templates/` path lose
  coverage forever. Move template detail to `/api/templates/:name`.

- 🟡 **Usability**: Adding a new `DocTypeKey` variant silently degrades cluster/completeness behaviour
  **Location**: Phase 3.1/3.7 — `derive_completeness` ends in `_ => {}`
  A new variant contributes nothing to completeness flags or cluster
  ordering, with no compile-time failure. Replace `_` catchalls with
  exhaustive arms (spell out `DocTypeKey::Templates => {}` etc.), or
  lift shared behaviour onto `DocTypeKey` methods.

#### Minor

- 🔵 **Architecture**: `etag_of` lives in `file_driver.rs` but is consumed by templates, indexer, tests
  **Location**: Phase 3.4 `file_driver::etag_of`
  Pure content-hashing helper with no dependency on the driver trait.
  Extract to `etag.rs`/`hash.rs` before Phase 8's writer adds a second
  consumer.

- 🔵 **Architecture**: Stated module layout (`api/mod.rs + per-endpoint files`) doesn't match implementation
  **Location**: Implementation Approach § 9 vs Phase 3.9
  Overview says `src/api/mod.rs + types.rs/docs.rs/...`; Phase 3.9
  ships a flat `src/api.rs`. By Phase 8 the docs handler will push
  several hundred lines. Either reconcile or take the structured split
  up front.

- 🔵 **Architecture**: Whole-map `rescan` couples awkwardly with Phase 4's per-path updates
  **Location**: Phase 3.5 `rescan`
  Phase 4's watcher will need fine-grained invalidation; current swap
  pattern races with it. Implement rescan as diff-then-apply now to
  match Phase 4's shape.

- 🔵 **Code-quality**: `frontmatter_state` serialised as `String` rather than typed enum
  **Location**: Phase 3.5 `IndexEntry`
  Stringly-typed comparisons (`e.frontmatter_state == "parsed"`) lose
  exhaustiveness. Make it a serde-tagged enum.

- 🔵 **Code-quality**: `yml_to_json` silently treats YAML `Tagged` values as malformed
  **Location**: Phase 3.3 `yml_to_json`
  Legal YAML tags (`!!timestamp`, `!custom`) are flagged malformed.
  Unwrap into inner value or stringify, or document the choice.

- 🔵 **Code-quality**: Config-override sentinel `PathBuf` leaks into wire format
  **Location**: Phase 3.6 `TemplateResolver::build`
  `PathBuf::from(format!("<no config override for {name}>"))` is
  serialised as `path` alongside `present: false`. Make `path:
  Option<PathBuf>`.

- 🔵 **Code-quality**: `Indexer::get`'s sync `std::fs::canonicalize` inside async
  **Location**: Phase 3.5 `get`
  Mismatched with `LocalFileDriver::read`'s `tokio::fs::canonicalize`.
  Use the async variant consistently.

- 🔵 **Code-quality**: `ApiError::Internal(String)` loses structured `FileDriverError` context
  **Location**: Phase 3.9 `api_from_fd`
  Flattens `Io { path, source }` to `source.to_string()`. Preserve the
  source via `#[source]` and log the path via `tracing::error!`.

- 🔵 **Correctness**: Empty-FM test relies on unverified `serde_yml::from_str("")` behaviour
  **Location**: Phase 3.3 `empty_frontmatter_parses_as_empty_mapping`
  Early-return `Parsed(BTreeMap::new())` when `yaml_src.trim().is_empty()`
  to avoid backend-dependent empty-document semantics.

- 🔵 **Correctness**: `adr_id` numeric frontmatter silently falls back to filename
  **Location**: Phase 3.5 `parse_adr_id`
  `adr_id: 17` (unquoted) parses as number; `as_str()` returns `None`.
  Handle `Value::Number` too, or warn on filename/frontmatter mismatch.

- 🔵 **Correctness**: Per-file non-NotFound errors abort entire rescan
  **Location**: Phase 3.5 `rescan`
  One IO error drops the whole index — contradicts the
  partial-content design intent. Log and `continue` unless the error
  is config-level.

- 🔵 **Security**: Path-escape string guard is shallow
  **Location**: Phase 3.9 `doc_fetch`
  `contains("..")` doesn't catch Windows drive prefixes, NUL bytes, or
  full-width dots. Layered defence still holds. Tighten to per-segment
  check and comment layering explicitly.

- 🔵 **Security**: Internal error responses echo raw IO text
  **Location**: Phase 3.9 `api_from_fd`
  Log details; return a constant body (`{"error":"internal error"}`)
  with an opaque correlation id.

- 🔵 **Security**: YAML parser may allow anchor/alias amplification
  **Location**: Phase 3.3 frontmatter parse
  `serde_yml` may not guard against billion-laughs. Reject `&`/`*`
  anchors in source before parse, or verify `serde_yml` caps.

- 🔵 **Security**: `/api/types` discloses absolute filesystem paths
  **Location**: Phase 3.1 `DocType.dir_path`
  Leaks username/workspace layout. Serve repo-relative paths.

- 🔵 **Security**: Non-indexed-path branch widens fetchable surface
  **Location**: Phase 3.9 `doc_fetch` fallback when indexer miss
  `.gitkeep`, `*.swp`, and nested READMEs become fetchable even though
  they aren't listed. Restrict to indexed entries or document the
  escape hatch.

- 🔵 **Security**: Blocking `std::fs::canonicalize` in `Indexer::get`
  **Location**: Phase 3.5 `get`
  Can stall a tokio worker on slow FS mounts. Use `tokio::fs::canonicalize`.

- 🔵 **Performance**: `lifecycle_list` deep-clones the full cluster Vec every request
  **Location**: Phase 3.9 `lifecycle_list`
  Store as `Arc<Vec<LifecycleCluster>>` under the lock; clone the Arc,
  not the Vec.

- 🔵 **Test-coverage**: 2000-file scan-under-1s NFR has no automated test
  **Location**: Performance Considerations § scan
  Add a `#[ignore]`-guarded (or bench) test: seed 2000 small .md files,
  time `Indexer::build`, assert < 3 s.

- 🔵 **Test-coverage**: Mutation-smoke-test protocol is manual-only
  **Location**: Phase 3 overall
  Reviewer-diligence-dependent. Consider `cargo-mutants` on critical
  modules.

- 🔵 **Test-coverage**: Integration test doesn't verify `type=templates` list omits `content`
  **Location**: Phase 3.9 `api_docs.rs`
  Unit test covers the resolver; HTTP-layer test asserts only the
  name field. Assert `tiers[*].content` is absent at the HTTP layer.

- 🔵 **Test-coverage**: `AppState::build` error path has no test
  **Location**: Phase 3.8
  No test asserts an initial-scan failure propagates as
  `AppStateError::Indexer`. Add one `#[cfg(unix)]` permission-denied test.

- 🔵 **Test-coverage**: Cluster title fallback chain not fully covered
  **Location**: Phase 3.7 `derive_title`
  Only the `parsed` happy path is tested. Add a cluster where every
  entry has `frontmatter_state: "absent"` asserting `title == slug`.

- 🔵 **Usability**: Repo-relative-path URL contract pushes layout onto consumers
  **Location**: Phase 3.9 `/api/docs/{*path}`
  Consider adding `entry.url` on `IndexEntry` so consumers follow a
  link rather than construct one.

- 🔵 **Usability**: ETag header quoting tolerance is inconsistent
  **Location**: Phase 3.9 `doc_fetch`
  Document which form is contractual (quoted only) and remove the
  unquoted fallback or add a dedicated test for it.

- 🔵 **Usability**: `api_smoke.rs` fixture config is hand-rolled, likely to drift from `write-visualiser-config.sh`
  **Location**: Phase 3.10
  Invoke the real shell script from the test (or use a shared template).

#### Suggestions

- 🔵 **Code-quality**: Many module-level doc comments explain WHAT, not WHY
  Audit `//!` blocks; keep only design rationale and non-obvious invariants.

- 🔵 **Code-quality**: `IndexEntry` has 11 fields and is cloned in every list response
  Introduce `IndexEntrySummary` without the `frontmatter` payload.

- 🔵 **Performance**: `all_by_type` iterates the full HashMap
  Maintain a parallel `HashMap<DocTypeKey, Vec<PathBuf>>` secondary
  index.

- 🔵 **Security / Usability**: `If-None-Match` matcher is strict-literal — no multi-tag / `*` / `W/`
  Split on commas, accept `*`, trim `W/`. Low-priority today, matters once polling increases.

- 🔵 **Usability**: `/api/healthz` returns `"ok\n"` plain text, inconsistent with JSON API
  Return `Json({"status":"ok"})` so monitoring tools can ingest
  consistently.

- 🔵 **Usability**: `api_smoke.rs` redirects stdout/stderr to /dev/null
  Capture instead and print on panic, or gate on `ACCELERATOR_VISUALISER_TEST_LOG=1`.

- 🔵 **Usability**: `frontmatter_state` wire field is stringly-typed
  Serialise the `FrontmatterState` enum directly with kebab-case
  rename.

- 🔵 **Usability**: `/api/types` `in_kanban`/`in_lifecycle` booleans are derivable from `key`
  Consider a `capabilities: string[]` field for future-proofing.

### Strengths

- ✅ Module boundaries mirror the spec's architecture cleanly
  (file_driver / indexer / templates / clusters / api) with clear SRP.
- ✅ `clusters.rs` naming disambiguates from existing `lifecycle.rs`
  thoughtfully while preserving the spec's `LifecycleCluster` wire type.
- ✅ Router extraction into `build_router(state, activity)` enables
  `tower::ServiceExt::oneshot` integration tests — a structural win
  over Phase 2's child-process test pattern.
- ✅ Staged trait evolution documented (Phase 3 read-only, Phase 4
  watch, Phase 8 writes) — callers and future drivers see the growth
  path.
- ✅ Virtual-type handling is coherent: `DocTypeKey::Templates` threads
  through `is_virtual()`, `config_path_key() -> None`, and the
  indexer guard uniformly.
- ✅ TDD discipline explicit: each sub-phase has a mutation-smoke-test
  recipe backing up the Red-Green-Refactor cycle, and `matches!`
  assertions are gated into `assert!(...)`.
- ✅ Path-safety is layered: `LocalFileDriver::new` canonicalises roots
  once, `read` canonicalises requests, and the prefix check is the
  authoritative defence behind the first-line string guard.
- ✅ Three-state `FrontmatterState` (`Parsed`/`Absent`/`Malformed`)
  models domain reality faithfully; test coverage includes YAML
  block-sequences, CRLF, unclosed quotes, non-mapping roots, invalid
  UTF-8.
- ✅ `ticket_of` uniformly collapses `null` / `""` / absent / numeric
  forms, matching observed live-corpus variation.
- ✅ Regression test for the `-review-` embedded-slug edge case is
  guarded at three layers (slug, indexer, api_smoke).
- ✅ Test pyramid well-balanced: pure unit, `oneshot` integration, and
  binary-spawning smoke.
- ✅ ETag is correctly cached at scan time on `IndexEntry` — the right
  lifecycle even if the handler then fails to use it.

### Recommended Changes

Ordered by impact:

1. **Fix `project_root` derivation.** (addresses the critical)
   Extend `config.json` with an explicit `project_root` field populated
   by `scripts/write-visualiser-config.sh`. Remove both
   `cfg.tmp_path.ancestors().nth(2)` sites and use `cfg.project_root`
   directly. Add a unit test asserting correctness for both the
   `seeded_cfg` layout and the live-workspace layout.

2. **Route template reads through the FileDriver.** (addresses
   TemplateResolver bypass + sync-in-async)
   Make `TemplateResolver::build` async and have it take
   `Arc<dyn FileDriver>` (or add tier paths to `LocalFileDriver::roots`
   under a synthetic template key and use the normal read path).

3. **Split templates out of `/api/docs`.** (addresses polymorphic
   shape and route-collision)
   Introduce `GET /api/templates` returning `TemplateSummary[]` and
   `GET /api/templates/:name` returning `TemplateDetail`. Drop
   `templates` from the `?type=` enum; keep `/api/types` entry for
   discoverability. Update integration tests and the smoke test.

4. **Make `doc_fetch` ETag-first.** (addresses 304 re-read and
   canonical-key mismatch)
   Pull ETag from the cached `IndexEntry`, compare `If-None-Match`
   before any disk read, only call `file_driver.read` for 200s. Use
   canonicalised paths as the lookup key so the cache-hit path is
   the common case.

5. **Add `MAX_DOC_BYTES` cap.** (addresses file-size DoS)
   Check `metadata.len()` in `LocalFileDriver::read`; return
   `FileDriverError::TooLarge { path, size }` mapped to 413.

6. **Fix the symlink-escape test.** (addresses incorrect
   cross-platform fallback)
   Mark `read_rejects_symlink_escape` as `#[cfg(unix)]` and remove the
   `hard_link` fallback (or implement a proper Windows symlink).

7. **Replace `_ => {}` catchalls.** (addresses enum-evolution footgun)
   Make every match on `DocTypeKey` exhaustive. For
   `derive_completeness`, spell out every variant (including
   `Templates => {}`) so the compiler flags future additions.

8. **Consolidate test fixtures.** (addresses drift risk)
   Delete `router_compose.rs::minimal_cfg`; use
   `tests/common/mod.rs::seeded_cfg` or `common::bare_cfg`. In
   `api_smoke.rs`, either call `scripts/write-visualiser-config.sh`
   or share a JSON template.

9. **Add `ServerError::Startup { source: AppStateError }`.**
   (addresses misleading Bind error at startup)
   Replace the `io::Error::new(...)` wrap in `run`.

10. **Fix error-body quality.** (addresses unactionable messages)
    Use `Display` not Debug formatting in `ApiError`; differentiate
    `NotFound` per-route; list valid enums on `InvalidDocType`.

11. **Add percent-encoded + NotFound-race + empty-type-list tests.**
    Specifically: `/%2e%2e/etc/passwd`, a `FileDriver` mock causing
    `NotFound` mid-rescan, `GET /api/docs?type=prs` → 200 `[]`.

12. **Add a 2000-file scan benchmark** (gated on `--ignored` or a
    dedicated `test:performance` task).

13. **Drop `Indexer<D>` generic in favour of `Arc<dyn FileDriver>`.**
    (addresses trait abstraction)
    Requires returning `Pin<Box<dyn Future<…>>>` from the trait or
    using `trait_variant`. Keeps `AppState` swappable.

14. **Move `activity` into `AppState`.** (addresses dual-container)
    Reduces Phase 4 churn in every test.

15. **Split `src/api.rs` into `src/api/{mod,types,docs,templates,lifecycle}.rs`.**
    Anchor the boundary before Phase 8 bloats the docs handler.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan lays out a cleanly staged, module-boundaried
progression, but three architectural pressure points need attention
before Phase 4/8: the `project_root` derivation miscounts ancestors;
`Indexer<D: FileDriver>` concretises at `AppState` in a way that
erases the trait's abstraction value; and `TemplateResolver` reads
disk directly rather than through the `FileDriver`, silently
bypassing path-safety and making a future driver swap non-trivial.

**Strengths**: Module boundaries mirror spec; `clusters.rs` naming
disambiguates thoughtfully; router extraction enables `oneshot`
tests; staged trait evolution is documented; virtual-type handling
is coherent; FileDriver exposes path-safety at a single point.

**Findings**:

- 🔴 critical (high): `project_root` via `.nth(2)` lands on `meta/`,
  not repo root — Phase 3.5 / 3.8 / 3.9.
- 🟡 major (high): `TemplateResolver` bypasses FileDriver — Phase
  3.6.
- 🟡 major (high): Generic `Indexer<D>` defeats trait abstraction at
  AppState — Phase 3.5 / 3.8.
- 🟡 major (medium): AppState/activity split creates two parallel
  state containers — Phase 3.8.
- 🔵 minor (high): `etag_of` in `file_driver.rs` has four consumers
  — extract to dedicated module.
- 🔵 minor (high): Stated `api/mod.rs + per-endpoint` layout not
  implemented — Implementation Approach vs 3.9.
- 🔵 minor (medium): Whole-map rescan awkward for Phase 4 per-path
  invalidation — 3.5.
- 🔵 minor (high): `TemplateResolver::build` sync std::fs in async
  context — 3.6.

### Code Quality

**Summary**: Well-structured and TDD-serious, with tight SRP and
good pure/impure separation. Concrete issues: three overlapping
error taxonomies without a clean bridge; blocking IO in async
construction; a handful of request-path unwrap/expect patterns; a
hacky serde-based string-to-enum helper; several primitive-obsession
smells (`frontmatter_state: String`, config-override sentinel path).

**Strengths**: Small focused modules; pure-function boundary for
slug/frontmatter; disambiguated `clusters.rs`; `Arc<D>` DI; structured
`thiserror` variants with `#[source]`; meaningful `FrontmatterState`
domain enum.

**Findings**:

- 🟡 major (high): Error wrapping loses structured info via
  io::Error::new — 3.8 `run`.
- 🟡 major (high): Sync `std::fs::read` in async build — 3.6.
- 🟡 major (high): `parse_kind` round-trips through JSON — 3.9.
- 🟡 major (medium): Silent `continue` on NotFound without logging
  — 3.5 `rescan`.
- 🟡 major (medium): `HeaderValue::from_str(...).unwrap()` on hot
  path — 3.9.
- 🔵 minor (high): `If-None-Match` accepts quoted+unquoted ad hoc —
  3.9.
- 🔵 minor (high): `frontmatter_state: String` primitive-obsession
  — 3.5.
- 🔵 minor (high): `Tagged` YAML silently marks Malformed — 3.3.
- 🔵 minor (medium): `strip_suffix_review_n` double-suffix untested
  — 3.2.
- 🔵 minor (medium): Config-override placeholder PathBuf sentinel —
  3.6.
- 🔵 minor (medium): Sync `std::fs::canonicalize` in async `get` —
  3.5.
- 🔵 minor (medium): `tmp_path.ancestors().nth(2)` undocumented
  brittle idiom — 3.8.
- 🔵 minor (medium): `ApiError::Internal(String)` flattens
  FileDriverError — 3.9.
- 🔵 minor (low): Many doc comments explain WHAT — throughout.
- 🔵 minor (low): `IndexEntry` has 11 fields, cloned on every list —
  3.5.

### Test Coverage

**Summary**: Broad and well-proportioned — every new module has
colocated unit tests with table-driven regressions, every route has an
`oneshot` integration test, and `api_smoke.rs` drives the real binary
against a fixture tree. Two critical tests
(`doc_fetch_returns_body_with_strong_etag` and `api_smoke.rs`) assume
a path-resolution model `doc_fetch`'s `tmp_path.ancestors().nth(2)`
does not produce, so they will fail despite the Success Criteria
marking them as expected to pass. Edge cases the plan calls out —
percent-encoded `..`, unquoted `If-None-Match`, concurrent
rescan/delete, empty-per-type list at HTTP — have no automated
regression.

**Strengths**: Colocated `#[cfg(test)]` per module; test pyramid
balanced; mutation-smoke-test recipes per sub-phase; three-state
frontmatter well-covered; path-safety defended in depth; `ticket:`
three forms covered; assertion discipline with
`grep 'matches!'` gate.

**Findings**:

- 🔴 critical (high): `doc_fetch` path resolution won't find files
  under either test's layout — Phase 3.9 / 3.10.
- 🟡 major (high): Percent-encoded `..` not tested — 3.9.
- 🟡 major (medium): Unquoted `If-None-Match` dead-letter — 3.9.
- 🟡 major (high): `rescan`'s NotFound-continue branch untested —
  3.5.
- 🟡 major (medium): Empty-directory list not asserted at HTTP layer
  — 3.9.
- 🟡 major (high): `minimal_cfg` vs `seeded_cfg` drift risk — 3.8 /
  3.9.
- 🔵 minor (medium): Non-unix symlink-escape uses `hard_link` —
  3.4.
- 🔵 minor (medium): 2000-file NFR has no automated benchmark —
  Performance.
- 🔵 minor (medium): Mutation-smoke protocol is manual-only.
- 🔵 minor (medium): List-vs-detail content distinction not
  HTTP-tested — 3.9.
- 🔵 minor (low): `AppState::build` error path untested — 3.8.
- 🔵 minor (medium): Cluster title fallback chain not fully covered
  — 3.7.

### Correctness

**Summary**: Mostly well-traced; one high-confidence off-by-one in
the `.nth(2)` project-root derivation; a non-unix symlink-test bug;
a cache-key canonical/non-canonical mismatch; a handful of minor
issues around empty-YAML, If-None-Match, numeric `adr_id`, and
error propagation.

**Strengths**: `rfind("-review-")` correctly anchors; frontmatter
slicing handles `---\n---` edge; three-state triage has strong
branch coverage; `LocalFileDriver::read` canonicalises before prefix
check; `ticket_of` uniformly collapses forms; deterministic wire
ordering via `DocTypeKey::all()` + sort-by-slug.

**Findings**:

- 🔴 critical (high): `.nth(2)` yields `<repo>/meta`, not `<repo>` —
  3.8 / 3.9.
- 🟡 major (high): Non-unix `hard_link` fallback doesn't test
  symlink escape — 3.4.
- 🟡 major (medium): Canonical/non-canonical cache-key mismatch —
  3.5 / 3.9.
- 🔵 minor (medium): Empty-FM relies on unverified
  `serde_yml::from_str("")` — 3.3.
- 🔵 minor (medium): `If-None-Match` strict-literal misses RFC forms
  — 3.9.
- 🔵 minor (high): `adr_id` numeric value silently falls through —
  3.5.
- 🔵 minor (high): Path-escape guard is heuristic only — 3.9.
- 🔵 minor (medium): AppStateError wrapped as ServerError::Bind —
  3.8.
- 🔵 minor (medium): `rescan` aborts whole scan on per-file non-NotFound
  — 3.5.
- 🔵 minor (low): Config-override sentinel path on wire — 3.6.

### Security

**Summary**: Sensible threat model for a loopback-only read-only
service. Defensible weak spots: unbounded response-body buffering,
fragile `project_root` inference, and a coarse `contains("..")`
first-line check whose correctness rests on axum 0.7 URL-decoding.
Not catastrophic given the loopback trust boundary.

**Strengths**: Layered path safety (string guard + canonicalise +
prefix check); canonical roots at driver construction; reads
canonical target not original path; preserved host-header guard +
127.0.0.1 bind; strong SHA-256 ETag cached at scan time; bounded
1 MiB frontmatter scan; no `regex` crate (no ReDoS); read-only +
loopback eliminates CSRF/CORS; sanitised 4xx status mapping.

**Findings**:

- 🟡 major (high): No file-size cap — 3.4 / 3.5 / 3.9.
- 🟡 major (medium): Fragile `project_root` weakens defence in depth
  — 3.9.
- 🔵 minor (high): Path-escape string guard is shallow — 3.9.
- 🔵 minor (high): Internal error responses echo raw IO text — 3.9.
- 🔵 minor (medium): YAML anchor amplification possible — 3.3.
- 🔵 minor (high): `/api/types` leaks absolute filesystem paths —
  3.1.
- 🔵 minor (medium): Non-indexed-path fallback widens fetchable
  surface — 3.9.
- 🔵 minor (low): Blocking `std::fs::canonicalize` in async handler
  — 3.5.
- 🔵 suggestion (medium): `If-None-Match` doesn't honour multi-tag /
  `*` / `W/` — 3.9.

### Performance

**Summary**: Meets NFR on paper via a fully-serial I/O loop exposed
to cold-cache latency, no benchmarks, no measurement hooks.
`/api/docs/{*path}` issues redundant disk read + SHA-256 even for
304 hits, `/api/lifecycle` deep-clones the full cluster Vec per
request, and `TemplateResolver::build` blocks async on sync IO. All
fixable in Phase 3 without scope creep.

**Strengths**: ETag cached once during scan; HashMap path lookup;
async `read_dir`; reasoned memory budget; cluster cache pre-seeded;
O(N log N) cluster sort.

**Findings**:

- 🟡 major (high): `doc_fetch` re-reads + re-hashes on 304 — 3.9.
- 🟡 major (high): Sequential scan may miss 2000-file/<1s NFR — 3.5.
- 🔵 minor (high): `lifecycle_list` deep-clones entire Vec — 3.9.
- 🔵 minor (high): Sync std::fs::read in async `TemplateResolver::build`
  — 3.6.
- 🔵 suggestion (medium): No regression benchmark for NFR —
  Performance Considerations.
- 🔵 suggestion (medium): O(N) per-type filter scan — 3.5.

### Usability

**Summary**: Small, consistent, well-tested read-only API. Strong
test DX (shared `common/`, `oneshot` as first-class). Main debts:
polymorphic `/api/docs?type=templates` response shape; unactionable
error bodies; fragile `/api/docs/{*path}` URL contract; silent `_ => {}`
catchalls that lose new DocTypeKey variants; duplicated test fixtures
likely to drift.

**Strengths**: Consistent kebab-case wire; sensible defaults (empty
dirs → `[]`, `ticket:` three forms → `None`); strong test
ergonomics; explicit ETag contract; virtual-type handling clean;
`frontmatterState` exposed for progressive disclosure; TDD
mutation-smoke recipes explicit.

**Findings**:

- 🟡 major (high): Polymorphic `/api/docs?type=templates` shape —
  3.9.
- 🟡 major (high): Unactionable debug-formatted error bodies — 3.9.
- 🟡 major (high): `/api/docs/templates/:name` collides with
  wildcard — 3.9.
- 🟡 major (high): `_ => {}` catchalls silently lose new
  `DocTypeKey` variants — 3.7.
- 🔵 minor (medium): Web-pathy URL contract pushes layout onto
  consumers — 3.9.
- 🔵 minor (high): ETag quoting tolerance inconsistent — 3.9.
- 🔵 minor (high): `minimal_cfg` vs `seeded_cfg` duplication — 3.8.
- 🔵 minor (medium): `api_smoke.rs` config drift risk — 3.10.
- 🔵 suggestion (high): `/api/healthz` plain-text inconsistency —
  3.8.
- 🔵 suggestion (medium): Binary stdout/stderr hidden on smoke
  failure — 3.10.
- 🔵 suggestion (medium): `frontmatter_state` stringly-typed wire —
  3.5.
- 🔵 suggestion (low): `/api/types` booleans redundant with `key`
  — 3.1.
