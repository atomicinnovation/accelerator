---
date: "2026-04-25T12:05:00+01:00"
type: plan-validation
skill: validate-plan
target: "meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md"
result: pass
status: complete
---

## Validation Report: Meta Visualiser Phase 5 — Frontend Scaffold and Library View

### Implementation Status

✓ Step 1: Cargo features + `build.rs` — Fully implemented
✓ Step 2: `assets.rs` + wire into `server.rs` — Fully implemented
✓ Step 3: Frontend scaffold — Fully implemented
✓ Step 4: TypeScript API types — Fully implemented
✓ Step 5: Query client + SSE hook — Fully implemented
✓ Step 6: Route tree + root layout + sidebar — Fully implemented
✓ Step 7: Library index table — Fully implemented
✓ Step 8: Library doc view + FrontmatterChips + MarkdownRenderer — Fully implemented
✓ Step 9: Templates views — Fully implemented
✓ Step 10: Invoke task wiring — Fully implemented

### Automated Verification Results

✓ `cargo test --lib --features dev-frontend` — **102 tests pass**
  - `assets::path_normalisation_tests` — 5 tests
  - `assets::dev_frontend_tests` — 3 tests
  - All existing server module tests — 94 tests

✓ `cargo test --tests --features dev-frontend` — **30 integration tests pass** across 13 files
  - `spa_serving::tests::spa_route_returns_html` — new, passes
  - `router_compose::root_serves_spa_index` — new, passes
  - All pre-existing integration tests — unchanged, pass

✓ `npm run build` — Succeeds, produces `dist/index.html` (671 KB JS, 207 KB gzip)

✓ `cargo build` (embed-dist, default features) — Compiles cleanly in 2.46s with `frontend/dist/` present

✓ `npm run test` — **61 tests pass** across 11 test files
  - `query-keys.test.ts` — 2 tests
  - `fetch.test.ts` — 12 tests
  - `use-doc-events.test.ts` — 7 tests
  - `FrontmatterChips.test.tsx` — 3 tests
  - `Sidebar.test.tsx` — 3 tests
  - `LibraryTemplatesIndex.test.tsx` — 4 tests
  - `LibraryTypeView.test.tsx` — 7 tests
  - `MarkdownRenderer.test.tsx` — 6 tests
  - `LibraryTemplatesView.test.tsx` — 5 tests
  - `LibraryDocView.test.tsx` — 7 tests
  - `router.test.tsx` — 5 tests

### Code Review Findings

#### Matches Plan

**Server-side:**
- `Cargo.toml`: `embed-dist` and `dev-frontend` features exactly match the plan. `rust-embed` with `compression`, `mime_guess`, and `tower-http` with compression features are all present.
- `build.rs`: Checks `CARGO_FEATURE_EMBED_DIST`, panics with the exact prescribed message if `frontend/dist/index.html` is missing.
- `docs.rs`: `DocType.virtual` always serialised; `skip_serializing_if` removed; `virtual_flag_always_serialised_in_json` test added.
- `assets.rs`: All three test modules present (`path_normalisation_tests`, `embed_tests`, `dev_frontend_tests`). All functions present: `normalise_asset_path`, `apply_spa_serving`, `apply_spa_serving_with_dist_path` (dev-frontend), `serve_embedded` (embed-dist), `Frontend` embed struct. Fixture at `tests/fixtures/mini-dist/`.
- `server.rs`: `build_router_with_spa` helper, `build_router`, `build_router_with_dist`, `api_not_found`, `CompressionLayer` all present. All 5 plan-specified tests present and passing: `spa_fallback_is_covered_by_host_header_guard`, `spa_fallback_updates_activity`, `unmatched_api_path_returns_json_404_not_spa_html`, `spa_asset_is_brotli_encoded_for_br_clients`, `serves_spa_root_and_writes_info`.
- `spa_serving.rs` integration test compiles and passes under `dev-frontend`.
- `lib.rs`: `pub mod assets` present.

**Frontend scaffold:**
- `mise.toml`: `node = "22"`, `test:unit:frontend` and updated `test:unit` tasks present.
- `package.json`: All required runtime and devDependencies present at specified versions.
- `vite.config.ts`: `resolveApiPort()` function with three-tier resolution strategy.
- `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`: All three present, correct references/targets.
- `src/test/setup.ts`: `MockEventSource` global stub with `vi.stubGlobal`.
- `frontend/dist/index.html`: Present (npm build was run).
- `package-lock.json`: Present (lockfileVersion 3).

**TypeScript API layer:**
- `types.ts`: `DocTypeKey`, `DOC_TYPE_KEYS`, `isDocTypeKey`, `DocType`, `IndexEntry`, `DocsListResponse`, `TemplateSummary`, `TemplateDetail`, `SseEvent` all present.
- `path-utils.ts`: `fileSlugFromRelPath` present.
- `query-keys.ts` + test: All 8 query key entries present; tests pass.
- `fetch.ts` + test: All 5 fetch functions present; 12 tests covering success paths, error paths, and URL encoding behaviour.
- `query-client.ts`: `staleTime: Infinity`, `retry: 1`.
- `use-doc-events.ts` + test: `dispatchSseEvent`, `makeUseDocEvents`, `useDocEvents` all exported. 7 tests covering invalidation rules, wiring, malformed JSON, and error-triggered prefix invalidation.

**UI components and views:**
- `main.tsx`: `RouterProvider` + `QueryClientProvider`.
- `router.ts`: All 8 routes present; `routeTree` exported; `parseParams` narrowing redirect on unknown type.
- `RootLayout.tsx`: `useDocEvents()` + `useQuery(fetchTypes)` + `Sidebar` + `Outlet`.
- `Sidebar.tsx` + test: Partitions on `virtual` flag; `Documents`/`Views`/`Meta` sections; 3 tests pass.
- `LibraryLayout.tsx`: `Outlet` wrapper.
- `LifecycleStub.tsx`, `KanbanStub.tsx`: Stub views present.
- `LibraryTypeView.tsx` + test: Sortable table, 4 sort keys, empty/loading/error states; 7 tests pass.
- `LibraryDocView.tsx` + test: Title, `FrontmatterChips`, `MarkdownRenderer`, empty related-artifacts aside; 7 tests pass.
- `FrontmatterChips.tsx` + test: Handles `parsed`/`absent`/`malformed` states; 3 tests pass.
- `MarkdownRenderer.tsx` + test: `react-markdown` + `remark-gfm` + `rehype-highlight`; XSS guards tested; 6 tests pass.
- `LibraryTemplatesIndex.tsx` + test: Template list with tier labels; 4 tests pass.
- `LibraryTemplatesView.tsx` + test: Three-tier panel layout with active badge and absent-note; 5 tests pass.

**Invoke tasks:**
- `tasks/test/unit.py`: `visualiser()` task runs `--no-default-features --features dev-frontend` first, then default features with `_ensure_frontend_dist()` auto-build guard. `frontend()` task runs `npm run test`.
- `tasks/test/integration.py`: Runs `--no-default-features --features dev-frontend` to include `spa_serving.rs` integration test.

#### Deviations from Plan

None. All specified components, tests, and wiring are present and match the plan's intent.

#### Minor Issues (non-blocking)

1. **3 unused import warnings in `server.rs`** (lines 589–591 in `serves_spa_root_and_writes_info` test): `use axum::body::Body`, `use axum::http::Request`, `use tower::ServiceExt as _` are imported but unused — the test switched to `reqwest::get` for real HTTP but left behind oneshot-pattern imports. Compiler warns but tests pass. `cargo fix` would remove them automatically.

2. **Large bundle size warning** from Vite: `dist/assets/index-*.js` is 671 KB before minification (207 KB gzip). Vite warns above 500 KB. This is expected for React 19 + TanStack Router/Query + highlight.js + react-markdown in a single chunk; code-splitting is out of scope for Phase 5.

3. **`window.scrollTo not implemented` stderr** in `router.test.tsx`: jsdom does not implement scroll restoration; TanStack Router logs these as `Error` to stderr during router navigation tests. All 5 router tests pass. This is a known jsdom limitation with TanStack Router's scroll-restoration hook.

### Manual Testing Required

1. **Start the dev server and verify in browser:**
   ```bash
   cd skills/visualisation/visualise/server
   ACCELERATOR_VISUALISER_BIN=$(pwd)/target/debug/accelerator-visualiser \
     ../scripts/launch-server.sh
   ```
   Then navigate to `http://localhost:<port>` and verify:
   - [ ] Sidebar shows all doc type labels grouped under Documents / Meta
   - [ ] Lifecycle and Kanban nav items are present
   - [ ] `/library/decisions` shows the sortable decisions table
   - [ ] Clicking a column header sorts the table; clicking again reverses it
   - [ ] Clicking a row navigates to the doc detail view with rendered markdown and frontmatter chips
   - [ ] `/library/templates` shows the templates list with tier labels
   - [ ] Clicking a template shows the three-tier panel with active badge
   - [ ] Editing a `.md` file on disk triggers a silent re-fetch in the UI (SSE live-update)

2. **Verify the embed-dist binary serves the SPA:**
   ```bash
   cd skills/visualisation/visualise/frontend && npm run build
   cd ../server && cargo build
   # Confirm the binary embeds frontend/dist at compile time
   ```

### Recommendations

- Fix the 3 unused imports in `serves_spa_root_and_writes_info` (run `cargo fix --lib -p accelerator-visualiser --tests`). Low priority — they cause only compiler warnings, not failures.
- Stub `window.scrollTo` in the test setup to silence jsdom stderr noise from `router.test.tsx`. Low priority — tests pass cleanly.
- Consider adding a `build.chunkSizeWarningLimit` to `vite.config.ts` or introducing code-splitting in a later phase (highlight.js and react-markdown are the primary contributors). Not a Phase 5 concern.
