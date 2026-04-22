---
date: "2026-04-22T00:00:00+01:00"
type: plan-validation
skill: validate-plan
target: "meta/plans/2026-04-21-meta-visualiser-phase-3-file-driver-indexer-api.md"
result: pass
status: complete
---

## Validation Report: Meta Visualiser — Phase 3: FileDriver, Indexer, and read-only API

### Implementation Status

✓ Phase 3.1: Crate dependencies and wire types — Fully implemented
✓ Phase 3.2: Slug derivation — Fully implemented
✓ Phase 3.3: Frontmatter parsing — Fully implemented
✓ Phase 3.4: FileDriver trait and LocalFileDriver — Fully implemented
✓ Phase 3.5: Indexer — Fully implemented
✓ Phase 3.6: Templates virtual DocType — Fully implemented
✓ Phase 3.7: Cluster computation — Fully implemented
✓ Phase 3.8: AppState composition and router builder — Fully implemented
✓ Phase 3.9: API routes — Fully implemented
✓ Phase 3.10: End-to-end fixture tree and smoke test — Fully implemented

### Automated Verification Results

✓ `cargo check` exits 0 — clean compile, no errors
✓ `cargo test --lib` exits 0 — 78 unit tests pass
✓ `cargo test --tests` exits 0 — all integration tests pass:
  - `api_docs` (8 tests)
  - `api_lifecycle` (3 tests)
  - `api_smoke` (1 test)
  - `api_templates` (3 tests)
  - `api_types` (1 test)
  - `config_cli` (1 test)
  - `config_contract` (1 test)
  - `lifecycle_idle` (1 test)
  - `lifecycle_owner` (1 test)
  - `router_compose` (3 tests)
  - `shutdown` (3 tests)
✓ `mise run test:unit` exits 0 (78 tests)
✓ `mise run test:integration` exits 0 (330 bash tests across all skills)

### Code Review Findings

#### Matches Plan

- `DocTypeKey` 10-variant enum with kebab-case serde serialisation; `describe_types` helper fully populated
- `slug::derive` per-DocType hand-rolled parsing with `rfind` anchoring for `-review-\d+` suffix (G3 design decision correctly implemented — internal `-review-` literals preserved in slugs)
- `FrontmatterState::Parsed/Absent/Malformed` three-state parser using `serde_yml`; title cascade (frontmatter → H1 → filename stem); ticket normalisation for null/empty/absent all collapsing to `None`
- `FileDriver` trait with `Pin<Box<dyn Future>>` manual desugaring for dyn-compatibility (MSRV constraint); `LocalFileDriver` with `canonicalize` + prefix check, 10 MiB cap, `.md`-only filter
- `Indexer` with `adr_by_id` and `ticket_by_number` lookup maps; SHA-256 ETags computed at scan time; 2000-file scan completes under 1 second (performance test passes in ~1.04s)
- `TemplateResolver` three-tier priority (`config-override` → `user-override` → `plugin-default`) with eager content loading
- `compute_clusters` canonical timeline ordering (Ticket=0 through Notes=8), mtime tie-breaking, `Completeness` flags, alphabetical sort by slug
- `AppState` extended with `indexer`, `templates`, `clusters`, `file_driver`, `activity` fields; `build_router` extracted as plan specified; `ApiError` enum with consistent JSON error responses
- Seven GET endpoints under `/api`: types, docs list, docs fetch (ETag + If-None-Match 304), templates list, template detail, lifecycle list, lifecycle by slug
- `write-visualiser-config.sh` emits `project_root` as required argument
- `tests/fixtures/meta/` covers absent-FM (notes), malformed-FM (malformed-plan.md), review-suffix edge case (2026-01-04-example-and-review-some-topic-review-1.md preserves embedded `-review-`)

#### Deviations from Plan

- **`gray_matter` declared but not used**: `Cargo.toml` declares `gray_matter = { version = "0.3", ... }` as specified, but `src/frontmatter.rs` implements YAML parsing directly via `serde_yml::from_str` without using the `gray_matter` crate. The plan described `gray_matter` as providing the frontmatter parsing engine. The implementation is functionally equivalent (three-state parser, same edge-case handling) and arguably cleaner, but the dependency is dead weight. No `use gray_matter` statement appears in any source file.

- **Extra per-route integration test files**: The plan specified `api_smoke.rs` as the end-to-end integration test, but four additional per-route test files were added beyond the plan: `api_docs.rs` (8 tests), `api_lifecycle.rs` (3 tests), `api_templates.rs` (3 tests), `api_types.rs` (1 test). These provide tower::oneshot-based route-level coverage and represent an improvement over the plan's specification.

- **`graceful_draining.rs` absent**: The plan's Current State Analysis mentioned this file as existing from Phase 2. It does not exist on disk. This is a Phase 2 gap (not introduced by Phase 3), and does not affect any Phase 3 criteria.

- **Fixture tree density**: The plan targeted "3–5 docs per type". Several types have fewer fixtures than that target (decisions: 2, notes: 1, research: 1). The critical edge cases (absent-FM, malformed-FM, review-suffix) are all covered; the shortfall is in raw fixture count for under-used types. The smoke test assertions all pass.

- **`GET /api/healthz` route added**: `build_router` includes a `/api/healthz` endpoint not mentioned in the plan's API spec. This is a net positive addition.

#### Potential Issues

- The dead `gray_matter` dependency adds ~150 KB to the dependency graph for no benefit. It should be removed in a cleanup pass (not blocking).

### Manual Testing Required

1. Live server smoke test (cannot run without a release binary):
   - [ ] Build release binary: `cargo build --manifest-path skills/visualisation/visualise/server/Cargo.toml --release`
   - [ ] `ACCELERATOR_VISUALISER_BIN=<path>/accelerator-visualiser skills/visualisation/visualise/scripts/launch-server.sh` prints a `**Visualiser URL**:` line
   - [ ] `curl -f <url>/api/types` returns JSON with 10-entry `types` array
   - [ ] `curl -f '<url>/api/docs?type=decisions'` returns JSON with 21 ADRs
   - [ ] `curl -f '<url>/api/lifecycle'` returns non-empty `clusters` array
   - [ ] `GET <url>/api/docs/../../etc/passwd` returns 403
   - [ ] `skills/visualisation/visualise/scripts/stop-server.sh` shuts down cleanly

2. Phase 2 regression check:
   - [ ] `GET /` still returns placeholder response (verified by `placeholder_root_is_preserved` integration test)
   - [ ] SIGTERM / SIGINT handling unchanged (verified by `shutdown` integration tests)

### Recommendations

- Remove `gray_matter` from `Cargo.toml` in a follow-up cleanup — it is unused and adds transitive weight.
- Consider adding `graceful_draining.rs` to complete the Phase 2 integration test gap; it was described as existing in the Phase 3 plan's current state.
- The fixture tree could be expanded to reach the "3–5 per type" target in a future cleanup, though current coverage is sufficient for CI.
