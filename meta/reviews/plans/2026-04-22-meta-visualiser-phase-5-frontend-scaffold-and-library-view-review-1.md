---
date: "2026-04-24T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, usability, standards, performance]
review_pass: 3
status: complete
---

## Plan Review: Meta visualiser Phase 5 — Frontend scaffold and library view

**Verdict:** REVISE

The plan is thorough, TDD-disciplined, and establishes a coherent end-to-end
scaffold, but it contains one concrete wire-format defect that will break the
Modified column at runtime (`mtime_ms` vs server's `mtimeMs`) and a number of
reinforcing major issues around router composition (middleware/host-guard
coverage of the SPA fallback), SPA fallback semantics (200 HTML for API
404s), test hygiene (silent `eprintln SKIP` skips, a tautological build-guard
test, and zero automated coverage of the production `embed-dist` path), and
conditional polymorphism around the Templates views. The scaffolding
decisions are sound and the phase-level scope is well-bounded; the issues are
largely surgical edits rather than structural rethinks.

### Cross-Cutting Themes

- **Router composition / middleware coverage of SPA fallback** (flagged by:
  architecture, correctness, security) — `apply_spa_serving` wraps the router
  via `fallback_service`, and the current plan composes `route_layer(activity)`
  plus `.layer(host_header_guard)` before that wrap. Per axum semantics,
  `route_layer` layers do NOT apply to `fallback_service` handlers, and
  `.layer()` behaviour around fallback is subtle enough to warrant an explicit
  test. Result: activity tracking and host-header protection likely do not
  cover SPA asset/HTML requests, silently changing idle-timeout semantics and
  DNS-rebinding defence posture.
- **SPA fallback swallows API 404s** (flagged by: correctness, security) —
  both `embedded_fallback` and the dev-frontend `ServeDir::not_found_service`
  return 200 `index.html` for any unmatched path, including `/api/typo`.
  This breaks `response.ok`-based error handling, masks information
  disclosure during scans, and converts typo debugging into HTML/JSON parse
  errors.
- **Silent test skips + tautological test** (flagged by: code-quality,
  test-coverage) — `serves_spa_root_and_writes_info` and
  `spa_route_returns_html_when_dist_present` both use
  `eprintln! SKIP + return` when `frontend/dist/` is absent; CI cannot
  distinguish "tests ran green" from "tests skipped". Separately,
  `build_guard.rs` contains a test that asserts a Cargo-guaranteed tautology.
- **Duplicated SPA-serving with hardcoded dist path** (flagged by:
  architecture, code-quality, correctness) — `spa_router`, `inner`, and the
  embed-dist `#[folder]` attribute each independently reference
  `CARGO_MANIFEST_DIR/../frontend/dist`. Tests exercise `spa_router`, not
  `inner`, so production dev-frontend behaviour is effectively untested.
- **Polymorphism-by-type-string for Templates** (flagged by: architecture,
  code-quality, correctness, performance) — `if (type === 'templates') return
  <LibraryTemplatesIndex />` inside `LibraryTypeView` and a similar dispatch
  inside `LibraryDocView` couple the generic library flow to a special case,
  and in the index dispatch the conditional fires *after* the `useQuery` hook,
  issuing a pointless `fetchDocs('templates')` request.
- **Hardcoded `NON_TEMPLATE_KEYS` drifts from server taxonomy** (flagged by:
  code-quality, correctness, usability) — Sidebar duplicates the backend's
  type list; new types silently disappear from the sidebar.
- **doc-changed invalidation gaps** (flagged by: correctness, architecture,
  performance) — SSE handler invalidates the doc list but NOT the
  `docContent` cache of the currently-open detail view; also invalidates
  `lifecycle()` unconditionally and has no `onerror` handling.
- **Raw-HTML markdown guard is undocumented** (flagged by: security,
  test-coverage) — `react-markdown` is safe by default, but the plan doesn't
  document the constraint that `rehype-raw`/`allowDangerousHtml` must never
  be added without a sanitiser, and there's no regression test.
- **Test-only surfaces have inconsistent coverage** (flagged by:
  test-coverage, code-quality, correctness) — `LibraryTemplatesIndex` has
  zero tests; routing tree and `/` → `/library` redirect have no tests;
  `useDocEvents` error path is untested; `strict: false` + `'plans'` fallback
  buries routing bugs behind a default.

### Tradeoff Analysis

- **Embed-dist default vs developer ergonomics**: the plan defaults to the
  `embed-dist` feature, which means every fresh clone must run `npm run
  build` before any `cargo check`/`cargo clippy`/rust-analyzer request
  succeeds (or must pass `--no-default-features --features dev-frontend`).
  Architecture flagged this as friction; Standards/Usability flagged the
  same pressure from the "bare `cargo test` is surprising" angle. Flipping
  the default to `dev-frontend` would improve dev UX but would mean
  release/CI builds need to opt in to `embed-dist` explicitly. Either
  choice is defensible; currently the plan leans toward the release posture
  at cost of everyday iteration.
- **SSE invalidation granularity — broad vs precise**: the performance lens
  prefers surgical `setQueryData` updates using the event's `path`/`etag`;
  the architecture lens flags that the current map-of-if-statements will grow
  with each new view. A middle ground is to invalidate by a top-level
  prefix (`['docs']`) and let TanStack Query's prefix-matching handle
  granularity, trading a slightly broader fetch for a simpler event handler.

### Findings

#### Critical

- 🔴 **Standards**: TypeScript `IndexEntry` field names diverge from server's camelCase wire format
  **Location**: Step 4: TypeScript API types (`src/api/types.ts`)
  The Rust `IndexEntry` uses `#[serde(rename_all = "camelCase")]`, producing `mtimeMs`, but Step 4's TypeScript type declares `mtime_ms: number`. `LibraryTypeView` sorts on this field and renders `new Date(entry.mtime_ms).toLocaleDateString()` — at runtime this will be `undefined`/`NaN` and show "Invalid Date". TypeScript cannot catch this because `fetch().json()` returns `any`.

#### Major

- 🟡 **Architecture / Correctness**: `route_layer(activity)` and middleware stack likely do not apply to the SPA fallback_service
  **Location**: Step 2d: `build_router` composition
  `apply_spa_serving` wraps the result of `.route_layer(activity).layer(body_limit).layer(timeout).layer(host_header_guard)` with `.fallback_service(ServeDir…)`. In axum, `route_layer` definitively does not wrap fallback_service; `.layer()` behaviour around fallback is subtle and worth testing. Net effect: host-header guard and activity tracking likely do not cover `/`, `/library/…`, or asset fetches — breaking idle-shutdown semantics during SPA use and leaving SPA HTML without DNS-rebinding protection.

- 🟡 **Correctness / Security**: SPA fallback returns 200 `index.html` for unmatched API paths
  **Location**: Step 2b: `embedded_fallback` and dev-frontend `ServeDir::not_found_service`
  `GET /api/typo` returns HTML 200 because the outer fallback catches any unmatched path. Clients calling `fetchDocs`/`fetchTemplateDetail` with a malformed URL see `r.ok === true`, then crash in `r.json()`. Log auditing cannot distinguish real API surface from SPA fallthrough. Fix: short-circuit `/api/*` paths to 404 JSON before the SPA fallback takes effect.

- 🟡 **Correctness / Performance**: `doc-changed` does not invalidate `docContent` cache
  **Location**: Step 5d: `useDocEvents`
  The hook invalidates `queryKeys.docs(docType)` and `queryKeys.lifecycle()` but not `queryKeys.docContent(path)`. A user editing the currently-open doc sees the index row title/chips refresh while the markdown body stays stale until navigation or the 30s `staleTime` expires. Live-update — the headline Phase 5 feature — is broken on the most important surface.

- 🟡 **Architecture / Code Quality / Correctness**: Triplicated SPA-serving construction with hardcoded `CARGO_MANIFEST_DIR/../frontend/dist`
  **Location**: Step 1c `build.rs`, Step 2b `assets.rs` (`spa_router`, `inner`, `embed` folder attribute), Step 2c integration test
  Four copies of the same relative path in two crates with no compile-time linkage; tests exercise `spa_router`, not production's `inner()`, so the two can drift silently. A frontend relocation (plausible given earlier workspace rearrangements) will require a coordinated edit across four files. Fix: thread `dist_path` as a parameter to a single `spa_router` function; have `inner` call `spa_router(dist_path_from_manifest())`; reference a single `const FRONTEND_DIST_REL` from `build.rs`.

- 🟡 **Architecture / Code Quality / Correctness / Performance**: Polymorphism-by-type-string for Templates views
  **Location**: Step 9c: `LibraryDocView` and `LibraryTypeView` templates dispatch
  `if (type === 'templates') return <LibraryTemplates…/>` lives inside the generic library route components. In `LibraryTypeView` the dispatch must run after `useQuery(fetchDocs(type))` (Rules of Hooks), firing a pointless `fetchDocs('templates')` on every Templates index visit. Makes adding further specialised types require more branching in these shared files. Fix: dedicated routes `/library/templates` → `LibraryTemplatesIndex` and `/library/templates/$name` → `LibraryTemplatesView`; TanStack Router's literal-path-wins semantics handle dispatch.

- 🟡 **Code Quality / Test Coverage**: `build_guard.rs` contains a tautological "documentation test"
  **Location**: Step 1b: `server/src/build_guard.rs`
  The test asserts `CARGO_FEATURE_EMBED_DIST.is_some() || CARGO_FEATURE_DEV_FRONTEND.is_some()` — Cargo guarantees this by definition when the test compiles. It cannot fail and does not exercise the `build.rs` panic. Either delete it (the panic message is self-documenting), move the assertion to a real integration test (`cargo build` in a temp workspace with missing dist), or rely on CI.

- 🟡 **Code Quality / Test Coverage / Usability**: Silent `eprintln! SKIP + return` hides regressions
  **Location**: Step 2c `spa_serving.rs`, Step 2d `serves_spa_root_and_writes_info`
  Both tests report green when `frontend/dist/index.html` is absent, so a CI job that forgets to run `npm run build` before `cargo test` converts meaningful coverage into no-ops without warning. Fix options: (a) have tests seed a stub `index.html` in a tempdir passed to `spa_router(path)` — removes the skip condition entirely; (b) `panic!` with a clear message so CI fails loudly; (c) gate behind a dedicated `dev-frontend-with-dist` feature that CI enables only after the npm build.

- 🟡 **Test Coverage**: No automated tests for the `embed-dist` path (the production mode)
  **Location**: Step 2a: assets.rs tests
  All three unit tests are gated on `#[cfg(all(test, feature = "dev-frontend"))]`. `embedded_fallback`, rust-embed integration, mime_guess resolution, empty-path handling, and the index.html fallback have zero automated coverage. Only a `cargo build | grep -c error` smoke test and manual browser verification protect the shipped-binary path. Fix: extract URI-to-asset-name into a pure function for unit testing, and add a tiny `tests/fixtures/mini-dist/` for a parallel test module gated on `#[cfg(all(test, not(feature = "dev-frontend")))]`.

- 🟡 **Performance**: Brotli-compressed embedded assets served without `Content-Encoding: br`
  **Location**: Step 1a `Cargo.toml` + Step 2b `embedded_fallback`
  `rust-embed`'s `compression` feature is enabled, but `embedded_fallback` returns `content.data` with only a `Content-Type` header — no `Accept-Encoding` inspection, no `Content-Encoding: br` emission. If rust-embed 8.x returns compressed bytes via `get()`, browsers receive undecodable binary labelled as `text/html` or `application/javascript`; if rust-embed transparently decompresses, the `compression` feature offers no runtime benefit (only smaller binary). Fix: verify rust-embed semantics, then either inspect `Accept-Encoding` and emit `Content-Encoding: br`, or drop `compression`. Add an integration test fetching `/assets/*.js` with `Accept-Encoding: br`.

- 🟡 **Test Coverage**: `npm run test` success criterion at Step 3g may not hold
  **Location**: Step 3g success criteria
  Vitest ≥1.0 defaults to erroring when no test files match unless `--passWithNoTests` is set. The scripts block defines `"test": "vitest run"` without that flag, so the very first `npm run test` invocation (Step 3g) will fail its own success criterion. Either add `--passWithNoTests` or reorder so `query-keys.test.ts` is written before the first test run.

- 🟡 **Code Quality / Correctness**: Hardcoded `NON_TEMPLATE_KEYS` in Sidebar duplicates backend taxonomy
  **Location**: Step 6c: `Sidebar.tsx`
  The 9-key tuple is a single-source-of-truth violation — the server already distinguishes templates via `DocType.virtual`. A server-side doc-type addition silently fails to appear in the sidebar (no compile, no test signal). Fix: `docTypes.filter(t => !t.virtual)` (symmetric with the existing `metaTypes = docTypes.filter(t => t.virtual)`), and delete `NON_TEMPLATE_KEYS`. Consider making `virtual` required (not optional) in the TS interface to enforce the contract.

- 🟡 **Code Quality / Correctness**: `strict: false` + `'plans'` fallback silently hides routing bugs
  **Location**: Steps 7b, 8c, 9b
  All three view components do `const type = (propType ?? params.type ?? 'plans') as DocTypeKey`. A misconfigured route silently lands on the Plans index; the `as DocTypeKey` cast launders any string into a valid key. Fix: use typed route params (TanStack Router's `strict: true`) with generics so params flow from the router, or narrow at the boundary (`if (!isDocTypeKey(raw)) return <NotFound />`). Remove the `'plans'` default.

- 🟡 **Test Coverage**: `LibraryTemplatesIndex` has no tests
  **Location**: Step 9c
  Every other view has 2–3 tests; the templates index is the first nested virtual-type view, so breakage is most likely exactly here. Fix: add `renders a link per template name`, `renders `activeTier` label`, and `link targets resolve with correct params`.

- 🟡 **Test Coverage**: No tests for router tree, `/` → `/library` redirect, or templates dispatch
  **Location**: Step 6b `router.ts`, Step 9c dispatches
  Mutations to route paths, the redirect, or the templates dispatch would pass CI. Add minimal `createMemoryHistory({ initialEntries: ['/'] })` test asserting redirected pathname; add one dispatch test per view asserting `type='templates'` renders the templates variant.

- 🟡 **Test Coverage**: `LibraryTypeView` covers only happy paths
  **Location**: Step 7a
  No test for empty-state ("No documents found."), loading state, sort direction toggle (two clicks on same header), or the status/date badge fallback. Mutations to any of these surfaces would pass CI.

- 🟡 **Test Coverage**: SSE error path (`onerror`) has no implementation or test
  **Location**: Step 5d: `useDocEvents`
  The hook only assigns `onmessage`; `onerror` is unassigned. Tests cover `onmessage` happy path + unmount. A malformed server event is silently swallowed by `catch {}`; a mutation removing the try/catch would not fail any test. Fix: add one test asserting `onmessage` with `'not json'` data does not throw and does not invalidate.

- 🟡 **Test Coverage / Code Quality**: EventSource mock uses shared mutable `lastInstance` with split definition
  **Location**: Step 3f + Step 5d: `src/test/setup.ts` and `use-doc-events.test.ts`
  The mock definition is introduced in Step 3f and then *patched* in Step 5d to expose `lastInstance`; tests reach into `(vi.mocked(EventSource) as any).lastInstance`. `vi.clearAllMocks()` does not reset static class fields, so cross-test state can leak. Fix: consolidate the mock definition in setup.ts, expose an instance array reset in `beforeEach`, or inject the EventSource constructor as a hook dependency.

- 🟡 **Code Quality**: Triplicated SPA-serving indirection with a trivial wrapper
  **Location**: Step 2b: `apply_spa_serving` → `inner` → `ServeDir`/`ServeFile`
  `apply_spa_serving` is a one-line passthrough adding no value. Collapse to a single function; pair with the path-deduplication fix above.

- 🟡 **Security**: No documented prohibition on enabling raw HTML in markdown rendering
  **Location**: Step 8b: `MarkdownRenderer`
  `react-markdown` is safe-by-default (no raw HTML, URL-scheme guarding), but one future line adding `rehype-raw` or `allowDangerousHtml` without a sanitiser would silently turn stored markdown into stored XSS. Fix: add a code comment forbidding this; consider adding `rehype-sanitize` now as defence-in-depth; add a regression test asserting `<script>` in markdown does not render as a DOM `<script>`.

- 🟡 **Usability**: `__PORT__` placeholder in `vite.config.ts` is a manual-substitution trap
  **Location**: Step 3c
  Bare `npm run dev` with unsubstituted `__PORT__` yields opaque `ECONNREFUSED` errors. Fix: read port from `VITE_API_PORT` env var or from `server-info.json` in a `configureServer` hook, with a clear fallback error.

- 🟡 **Usability**: Bare `cargo test` surprises newcomers — needs `--features dev-frontend`
  **Location**: Full success criteria / Step 10d
  The universal Rust convention is `cargo test`; the plan requires `cargo test --features dev-frontend` or the default embed-dist guard fails on a fresh clone. Fix: make `dev-frontend` a default dev-profile feature, or add a mise/makefile task wrapping the correct flags and document it prominently.

- 🟡 **Usability**: Manual verification depends on real meta/ content and a specific plan filename
  **Location**: Full success criteria > Manual verification
  Deep link `/library/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding` will rot; no sample fixture meta directory exists. Fix: ship `fixtures/sample-meta/` and make it the default `ACCELERATOR_VISUALISER_PROJECT` target in manual-test instructions.

- 🟡 **Usability / Standards**: Column-header sort lacks keyboard accessibility and direction indicator
  **Location**: Step 7b: `LibraryTypeView`
  Clickable `<th>` with no `<button>`, no `tabindex`, no `onKeyDown`, no `aria-sort`, no arrow indicator. Keyboard users cannot sort; sighted users cannot tell current sort state. Fix: wrap in `<button type="button">`, drop the redundant `role="columnheader"`, add `aria-sort`, and render ▲/▼ for the active column.

- 🟡 **Standards**: Inconsistent directory and file naming conventions in new frontend
  **Location**: Steps 3, 5, 6, 7, 8, 9
  `src/components/Sidebar/` (PascalCase) vs `src/api/` (lowercase) vs `src/routes/library/` (lowercase); `use-doc-events.ts` (kebab) vs `Sidebar.tsx` (PascalCase). Each is defensible; none is stated. Fix: add a short "Frontend layout conventions" section pinning the rules before they calcify by accident in Phase 6.

#### Minor

- 🔵 **Architecture**: SSE-to-cache-key mapping will accumulate branches
  **Location**: Step 5d: `useDocEvents`
  Hardcodes `kanban()` invalidation when `docType === 'tickets'`; each new derived view will add another `if`. Consider top-level prefix invalidation (`['docs']`) or an event-bus/subscriber pattern.

- 🔵 **Architecture**: Layer-order invariants around `apply_spa_serving` are undocumented and untested
  **Location**: Step 2d
  Add an integration test: `Host: evil.example` on `/` and `/library/x` → 403. Document the ordering invariants in a comment on `build_router`.

- 🔵 **Architecture**: Build-time panic couples every `cargo` operation to a prior npm build
  **Location**: Step 1c: `build.rs`
  Every `cargo check`, `cargo clippy`, and rust-analyzer probe fails on a fresh clone. Consider emitting a placeholder `frontend/dist/index.html` when missing, or flipping the default feature to `dev-frontend`.

- 🔵 **Architecture**: `#[derive(Embed)] struct Frontend` declared inside the handler function
  **Location**: Step 2b
  Lift to module scope inside the `#[cfg(not(feature = "dev-frontend"))]` block to make its singleton nature clear and enable reuse.

- 🔵 **Architecture / Usability**: No SSE reconnect handling; stale UI silently ignored after disconnect
  **Location**: Step 5d
  Even if full backoff is Phase 10, wire `source.onerror` to a top-level `queryClient.invalidateQueries()` so eventual consistency holds from day one. Consider a "live updates paused" badge.

- 🔵 **Architecture**: Add the `fs` feature to `tower-http` only when `dev-frontend` is enabled
  **Location**: Step 1a
  `tower-http = { features = ["trace","limit","timeout"] }` + `dev-frontend = ["tower-http/fs"]` to avoid shipping unused `ServeDir` code in release.

- 🔵 **Code Quality**: Status-or-date fallback in badge is surprising
  **Location**: Step 7b: `LibraryTypeView`
  Showing a date in a column labelled "Status" confuses users and desynchronises the sort. Either show "—" for missing status, or make status/date a per-type configuration.

- 🔵 **Code Quality**: `av: string | number` with conditional assignment loses type safety
  **Location**: Step 7b: `sortEntries`
  Replace with per-key extractor functions for clarity and type safety.

- 🔵 **Code Quality**: `Record<string, unknown>` casts for frontmatter access scatter shape knowledge
  **Location**: Steps 7b, 8c
  Add a `frontmatter-access.ts` helper module with typed readers (`getStatus`, `getDate`).

- 🔵 **Code Quality**: `fetch.ts` throws opaque `Error` with status in the message
  **Location**: Step 5b
  Define `ApiError` with `status: number` and `url: string` fields; configure `retry` to skip non-5xx; branch UI on error kind.

- 🔵 **Code Quality**: `catch {}` silently swallows malformed SSE data
  **Location**: Step 5d
  Replace with `catch (err) { console.warn('malformed SSE event', { data: e.data, err }) }`.

- 🔵 **Code Quality**: `MemoryRouter` helper lives under `components/Sidebar/test-helpers.tsx` but is generic
  **Location**: Step 6c
  Move to `src/test/memory-router.tsx`; update the three test imports.

- 🔵 **Code Quality**: Unused `Outlet` import in `router.ts`
  **Location**: Step 6b
  `noUnusedLocals: true` will fail the TypeScript build. Remove the import.

- 🔵 **Code Quality**: `fileSlug` derivation duplicated between views
  **Location**: Steps 7b, 8c
  Extract to `src/api/path-utils.ts`; use in both views.

- 🔵 **Correctness**: `lifecycle()` unconditionally invalidated on every doc event
  **Location**: Step 5d
  Consult `DocType.inLifecycle` before invalidating, or defer until Phase 6 introduces the query.

- 🔵 **Correctness**: Unknown SSE event types silently dropped
  **Location**: Step 5d
  Narrow the `catch` to `JSON.parse` only and emit a `console.warn` for unrecognised `type` fields.

- 🔵 **Correctness**: Entry lookup by basename collapses same-name docs across subdirectories
  **Location**: Step 8c
  Encode the full relPath in the URL, or document and assert the single-directory-per-type invariant.

- 🔵 **Correctness / Usability**: `fetchDocs` errors render as "No documents found." indistinguishable from empty state
  **Location**: Steps 7b, 8c, 9a
  Destructure `isError`/`error` from `useQuery` and render a distinct error state; apply consistently across views.

- 🔵 **Correctness / Usability**: `/library` renders an empty main pane
  **Location**: Step 6b: `libraryIndexRoute` with `component: () => null`
  Redirect to `/library/decisions` (or similar canonical default), or render a welcome/help panel.

- 🔵 **Correctness**: `TemplateDetail.activeTier` (summary) not used in rendering; `tier.active` duplicates the info
  **Location**: Step 9b: `TierPanel`
  Derive `tier.active` from `tier.source === data.activeTier` client-side, making the server's invariant explicit.

- 🔵 **Security / Supply Chain**: No hygiene posture for the new npm dependency tree
  **Location**: Step 3b: `package.json`
  Add `npm audit` to CI; commit to lockfile discipline; consider Dependabot/Renovate; `npm ci --ignore-scripts` in builds.

- 🔵 **Security**: `fetchDocContent` interpolates `relPath` into URL without per-segment encoding
  **Location**: Step 5b
  Encode each path segment with `encodeURIComponent` and rejoin with `/`; or enforce a URL-safe invariant server-side.

- 🔵 **Security**: No test that `javascript:`-scheme links in markdown are blocked
  **Location**: Step 8b
  Add one regression test asserting that a `[x](javascript:alert(1))` link does not render an `href` with the `javascript:` scheme.

- 🔵 **Security**: rust-embed path traversal resistance is implicit
  **Location**: Step 2b
  Add a small unit test: `Frontend::get("../../etc/passwd")` returns None / falls through to `index.html`.

- 🔵 **Security**: Content-Type for HTML omits charset
  **Location**: Step 2b: `embedded_fallback`
  Append `; charset=utf-8` to HTML responses to avoid UTF-7 XSS sniffing edge cases on older browsers.

- 🔵 **Usability**: `--config /dev/null` verification is misleading
  **Location**: Step 10c
  Replace with a meaningful dev-frontend check or a `cargo check --features dev-frontend`.

- 🔵 **Usability**: `mise install` prerequisite is buried inside the plan
  **Location**: Step 3a
  Add a top-level CONTRIBUTING note; declare `engines.node` in `package.json` so `npm install` warns on mismatched Node.

- 🔵 **Usability**: "Meta" sidebar section with a single item may not earn its visual cost
  **Location**: Step 6c
  Conditionally render Meta only when non-empty, or fold Templates into Documents with de-emphasised styling.

- 🔵 **Usability**: Doc detail lacks breadcrumbs / Back link
  **Location**: Step 8c: `LibraryDocView`
  Add `<nav aria-label="breadcrumb">Library › Plans › Foo Plan</nav>` to aid deep-link wayfinding.

- 🔵 **Usability**: `LibraryTemplatesIndex` renders raw `activeTier` identifier
  **Location**: Step 9c
  Reuse `TIER_LABELS` from `LibraryTemplatesView.tsx` to show user-friendly labels.

- 🔵 **Usability**: Modified column shows date only — no time or relative formatting
  **Location**: Step 7b
  Use `toLocaleString()` or `Intl.RelativeTimeFormat` for recent mtimes; this is precisely the info live-update users care about.

- 🔵 **Usability / Standards**: No responsive breakpoints; fixed sidebar width
  **Location**: Step 6c/6d
  If out of scope, add to "What we are NOT doing"; otherwise add a collapsible sidebar at narrow widths.

- 🔵 **Usability**: `FrontmatterChips` renders objects as `[object Object]`
  **Location**: Step 8a
  Branch on `typeof v === 'object' && v !== null ? JSON.stringify(v) : String(v)`.

- 🔵 **Usability**: Bare "Loading…" everywhere without skeletons/spinners
  **Location**: All view components
  Add to Phase 5's "NOT doing" list as a deliberate deferral.

- 🔵 **Standards**: `build_guard.rs` as a top-level module departs from idiomatic Rust test placement
  **Location**: Step 1b
  Move the assertion into `server/tests/build_guard.rs` (integration), or delete the module entirely.

- 🔵 **Standards**: Sort-header ARIA attributes / focusable semantics absent
  **Location**: Step 7b
  Drop redundant `role="columnheader"` on `<th>`; add `<button>` semantics and `aria-sort`; add `aria-label` to `<nav>` in Sidebar.

- 🔵 **Standards**: Two competing invocation mechanisms for frontend tests
  **Location**: Steps 3a and 10e
  Pick one — mise task calls `invoke test.unit.frontend`, OR mise calls `npm` directly and drop the invoke task — not both.

- 🔵 **Standards**: Conditional `highlight.js` dependency is fragile
  **Location**: Step 3b
  `highlight.js/styles/github.css` is imported directly in Step 8b, so `highlight.js` must be a direct dependency. Add `"highlight.js": "^11"` unconditionally and remove the conditional language.

- 🔵 **Standards**: `dev-frontend` Cargo feature declared empty without explanation
  **Location**: Step 1a: `Cargo.toml`
  Add brief inline comments above each feature clarifying intent.

- 🔵 **Standards**: No ESLint or Prettier configuration for the new TypeScript code
  **Location**: Step 3b
  Either add `eslint` (+ `@typescript-eslint`, `eslint-plugin-react`, `eslint-plugin-react-hooks`, `eslint-plugin-jsx-a11y`) and `prettier`, or state in "What we are NOT doing" that linting/formatting is deferred.

- 🔵 **Performance**: SSE invalidation is coarse — full doc list refetched per single-file edit
  **Location**: Step 5d
  Prefer `queryClient.setQueryData` patching the cached list using the event's `path`/`etag`, with invalidation reserved for add/remove cases. Acceptable today but document the trade-off.

- 🔵 **Performance**: `staleTime: 30_000` duplicates work that SSE already guarantees
  **Location**: Step 5c
  Set `staleTime: Infinity` (or a large value) and rely on SSE as sole invalidator.

- 🔵 **Performance**: LibraryDocView issues two queries per doc page
  **Location**: Step 8c
  Add `GET /api/docs/:type/:slug` returning entry + content, or read-through the cached list via `queryClient.getQueryData` before issuing `fetchDocs`.

- 🔵 **Performance**: `sortEntries` runs on every render without memoisation
  **Location**: Step 7b
  Wrap with `useMemo(() => sortEntries(entries, sortKey, sortDir), [entries, sortKey, sortDir])`.

- 🔵 **Performance**: `MarkdownRenderer` re-parses and re-highlights on every parent render
  **Location**: Step 8b
  Wrap in `React.memo` keyed on `content`, or `useMemo` the `<ReactMarkdown>` element.

- 🔵 **Performance**: highlight.js language pack adds ~100–200KB without code-splitting
  **Location**: Step 3b / Step 8b
  Either `manualChunks` for markdown/highlight or swap `rehype-highlight` for `lowlight` with hand-picked languages. Add a bundle-analysis report to `npm run build`.

- 🔵 **Test Coverage**: Fetch error-path coverage is thin
  **Location**: Step 5b: `fetch.test.ts`
  No tests for `fetchDocs`, `fetchTemplates`, `fetchTemplateDetail`, network rejection, JSON parse failure, or missing `etag` header.

- 🔵 **Test Coverage**: `vi.spyOn` mock isolation not explicitly restored
  **Location**: Steps 7a, 8c, 9a
  Change setup.ts to `afterEach(() => vi.restoreAllMocks())` (or set `restoreMocks: true` in vite.config.ts) to prevent cross-test spy leakage.

- 🔵 **Test Coverage**: No test that raw HTML in markdown is not rendered
  **Location**: Step 8b
  Add one assertion that `<MarkdownRenderer content="<script>alert(1)</script>" />` does not produce a DOM `<script>`.

- 🔵 **Test Coverage**: `LibraryDocView` missing tests for not-found and loading branches and templates dispatch
  **Location**: Step 8c
  Three cheap tests given the existing harness.

- 🔵 **Test Coverage**: Sidebar tests don't assert active-link behaviour
  **Location**: Step 6c
  Parameterise `MemoryRouter` with `initialEntries` and add a test verifying the active class on the matching link.

- 🔵 **Test Coverage**: CARGO_MANIFEST_DIR coupling makes the integration test brittle
  **Location**: Step 2c
  Expose the dist path from assets.rs as a pub fn/const used by both implementation and tests.

#### Suggestions

- 🔵 **Architecture / Security**: Add an integration test that asserts `Host: evil.example` on `/` and `/library/x` returns 403 (locks host-header coverage over SPA into a regression test).

- 🔵 **Usability / Standards**: Confirm `ACCELERATOR_VISUALISER_*` env-var naming matches the rest of the plugin's conventions; document the naming rule if undefined.

- 🔵 **Performance**: `embedded_fallback` allocates full response bodies via `content.data` — fine at current scale; flag as a streaming candidate if large assets land later.

- 🔵 **Standards**: Add a changelog entry summarising the SPA rollout and new build prerequisite (`npm run build` before `cargo build`).

### Strengths

- ✅ Test-first discipline is applied consistently across Rust unit, Rust integration, and Vitest layers, driving testability into the design.
- ✅ Clean separation of concerns — query keys, fetch functions, query client, SSE hook, components, and routes each live in their own module with a single responsibility.
- ✅ Explicit `build.rs` guard panics loudly with an actionable, copy-pasteable remediation when `frontend/dist/` is absent.
- ✅ CSS Modules colocated with components prevent global-style collisions without configuration.
- ✅ Compile-time feature gating (`dev-frontend` vs `embed-dist`) contains conditional code inside `assets.rs`.
- ✅ TanStack Query cache keys centralised in `queryKeys` — a good seam for SSE invalidation and future cache manipulation.
- ✅ Stub routes for Lifecycle and Kanban ship from day one so the sidebar is complete and later phases slot in without re-architecting navigation.
- ✅ "What we are NOT doing" section is honest and scoped — wiki-links, Mermaid, dnd-kit, error polish, SSE backoff, write paths are all explicitly deferred.
- ✅ SSE payload discriminated union cleanly mirrors the server's serde `tag = "type"` convention.
- ✅ TypeScript compiler configured with strong defaults (`strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`).
- ✅ `FrontmatterChips` covers all three frontmatter states (parsed / malformed / absent).
- ✅ `useDocEvents` tests verify exact invalidation calls rather than snapshotting — good fidelity.
- ✅ Step-by-step implementation sequence (36 numbered steps) gives contributors a reliable linear path with verification gates.
- ✅ `react-markdown` is correctly used without `rehype-raw` / `allowDangerousHtml`, giving the renderer a safe-by-default posture.

### Recommended Changes

Grouped by cost/impact; within each group, order is top-to-bottom priority.

**1. Fix the wire-format break (one-line, highest ROI)**
1. **Rename `mtime_ms` → `mtimeMs` in `types.ts`** and audit every field in `DocType`, `IndexEntry`, `TemplateTier`, `TemplateSummary`, `TemplateDetail`, `SseEvent*` against the Rust structs. Add a comment at the top of `types.ts`: "All fields use camelCase to match the server's `#[serde(rename_all = \"camelCase\")]` output." *(addresses: Critical standards finding)*

**2. Harden router composition + fallback semantics**
2. **Restructure `build_router` so middleware definitively covers the SPA fallback** — either use `.layer(...)` for activity (not `.route_layer(...)`) or wrap the `ServeDir`/embed handler with the same layer stack via `ServiceBuilder` before `fallback_service`. Add a test asserting a SPA GET updates `Activity::last_millis()` (or explicitly does not, if that's the chosen policy). *(addresses: architecture & correctness "middleware stack does not apply to fallback")*
3. **Short-circuit `/api/*` unmatched paths to JSON 404 before SPA fallback** — split the router into an API sub-router with its own 404 and an SPA sub-router, or add a specific `.fallback` for `/api/*` that returns `StatusCode::NOT_FOUND`. *(addresses: correctness & security "SPA swallows API 404s")*
4. **Add an integration test** that sends `Host: evil.example` to `/` and `/library/x` and asserts 403 — locks in host-header coverage of the SPA. *(addresses: architecture layer-order + security DNS-rebinding findings)*

**3. Consolidate SPA serving + remove silent skips**
5. **Collapse `apply_spa_serving` / `inner` / `spa_router` into one function taking `dist_path: PathBuf`**. Define a single `const FRONTEND_DIST_REL` referenced from `build.rs` and from the dist-path resolver. Have `inner` delegate to `spa_router(resolved_dist_path())`. *(addresses: architecture & code-quality duplication findings)*
6. **Replace `eprintln SKIP + return` with a tempdir seed** — seed a stub `index.html` in the test tempdir and pass it to `spa_router(tmp)`, so tests have no skip condition. For any test that must run against the real `frontend/dist/`, gate behind a dedicated `dev-frontend-with-dist` feature that CI enables only after `npm run build`. *(addresses: code-quality & test-coverage & usability silent-skip findings)*
7. **Delete `build_guard.rs` entirely, or move to `server/tests/build_guard.rs` as a real `cargo build` assertion in a tempdir.** *(addresses: code-quality & test-coverage tautology findings)*
8. **Add an `embed-dist` unit-test module** (gated `#[cfg(all(test, not(feature = "dev-frontend")))]`) using a tiny `tests/fixtures/mini-dist/` so the production-path handler has coverage. *(addresses: test-coverage "no automated tests for embed-dist" finding)*

**4. Fix SPA/React defects and data flow**
9. **Invalidate `queryKeys.docContent(event.path)` in `useDocEvents`** on `doc-changed`/`doc-invalid`, so the open detail view stays fresh. *(addresses: correctness "doc-changed does not invalidate docContent")*
10. **Move Templates dispatch into the router** — add `libraryTemplatesIndexRoute` (`/library/templates`) and `libraryTemplateDetailRoute` (`/library/templates/$name`) as siblings; delete the `if (type === 'templates')` branches in `LibraryTypeView` and `LibraryDocView`. *(addresses: architecture, code-quality, correctness, performance polymorphism findings)*
11. **Replace `NON_TEMPLATE_KEYS` with `docTypes.filter(t => !t.virtual)`** and make `virtual` required in the TypeScript `DocType` interface. *(addresses: code-quality, correctness, usability "hardcoded taxonomy" findings)*
12. **Remove `propType ?? params.type ?? 'plans'` fallback** — use typed route params (`strict: true`) or narrow explicitly; delete the `'plans'` default and the `as DocTypeKey` cast. *(addresses: code-quality & correctness "strict:false hides bugs" finding)*
13. **Wire `source.onerror` in `useDocEvents`** to a top-level `queryClient.invalidateQueries()` so disconnects are eventually consistent. *(addresses: architecture & usability SSE-reconnect findings)*
14. **Add `enabled: type !== 'templates'` to `LibraryTypeView`'s `useQuery`** as a short-term guard if step 10 is deferred. *(addresses: performance + correctness "pointless fetchDocs" finding)*

**5. Strengthen test coverage**
15. **Add tests for `LibraryTemplatesIndex`** (link-per-template, active-tier label, correct link params). *(addresses: test-coverage LibraryTemplatesIndex finding)*
16. **Add router + dispatch tests** — a MemoryRouter smoke asserting `/` → `/library`; one "renders templates view when `type='templates'`" test per affected component. *(addresses: test-coverage routing finding)*
17. **Extend `LibraryTypeView` tests** — empty state, loading state, sort-direction toggle. *(addresses: test-coverage happy-path-only finding)*
18. **Extend `LibraryDocView` tests** — not-found branch, loading branch, templates dispatch. *(addresses: test-coverage)*
19. **Add `useDocEvents` malformed-event and `onerror` tests.** *(addresses: test-coverage SSE error-path finding)*
20. **Add `MarkdownRenderer` XSS regression tests** — `<script>` tag in content does not produce a DOM script element; `[x](javascript:alert(1))` does not render the scheme as `href`. *(addresses: security & test-coverage XSS findings)*
21. **Broaden `fetch.test.ts`** to cover `fetchDocs`, `fetchTemplates`, `fetchTemplateDetail`, network rejection, and malformed JSON. *(addresses: test-coverage fetch finding)*
22. **Flip test-mock isolation to `restoreMocks: true` in `vite.config.ts`** (or `afterEach(() => vi.restoreAllMocks())` in setup.ts). *(addresses: test-coverage spy-leakage finding)*
23. **Consolidate the EventSource mock in setup.ts** with an instances array reset in `beforeEach`; remove the mid-plan mock patch in Step 5d. *(addresses: test-coverage EventSource mock finding)*

**6. Fix misc build/security/standards defects**
24. **Verify rust-embed 8.x `compression` feature semantics and fix Content-Encoding** — either inspect `Accept-Encoding` and emit `Content-Encoding: br`, or drop the `compression` feature. Add an integration test. *(addresses: performance brotli finding)*
25. **Add `"highlight.js": "^11"` unconditionally** to `dependencies` and remove the conditional language from Step 3b. *(addresses: standards fragile-dep finding)*
26. **Remove unused `Outlet` import in `router.ts`** — `noUnusedLocals: true` will otherwise break `npm run build`. *(addresses: code-quality finding)*
27. **Add `--passWithNoTests` to the vitest script** or reorder the scaffold steps so `npm run test` is never executed against an empty suite. *(addresses: test-coverage scaffold-step finding)*
28. **Add a `MarkdownRenderer` comment forbidding `rehype-raw`/`allowDangerousHtml` without `rehype-sanitize`.** *(addresses: security raw-HTML finding)*
29. **Encode each segment of `relPath` with `encodeURIComponent`** in `fetchDocContent`. *(addresses: security URL-encoding finding)*
30. **Gate `tower-http/fs` behind the `dev-frontend` feature** in `Cargo.toml`. *(addresses: architecture release-size finding)*
31. **Document the frontend naming conventions** (PascalCase component files / kebab-case for non-component modules; PascalCase dirs for components / lowercase dirs for cross-cutting) in a short subsection near Step 3. *(addresses: standards naming findings)*

**7. Deeper usability polish (may defer if scope-constrained)**
32. **Read the vite dev-proxy port from `VITE_API_PORT` env or `server-info.json`**, eliminating the `__PORT__` placeholder. *(addresses: usability finding)*
33. **Add a sample `fixtures/sample-meta/` directory** and document it as the default manual-verification target. Remove the specific deep-link filename from the checklist. *(addresses: usability finding)*
34. **Add `<button>` semantics, `aria-sort`, and direction arrows** to `LibraryTypeView` sort headers; drop redundant `role="columnheader"`; add `aria-label="Primary navigation"` to the sidebar `<nav>`. *(addresses: usability & standards accessibility findings)*
35. **Either simplify the sidebar Meta section** (conditional render or inline into Documents), or accept the empty-section risk and add an explicit empty-state placeholder. *(addresses: usability Meta-section finding)*
36. **Add breadcrumbs to `LibraryDocView`.** *(addresses: usability wayfinding finding)*
37. **Decide and document** whether ESLint/Prettier/responsive-design/loading-skeletons are in-scope or deferred; update "What we are NOT doing" accordingly. *(addresses: usability & standards deferral findings)*
38. **Document supply-chain hygiene** (`npm audit` in CI, lockfile policy, Dependabot) either in the plan or a CONTRIBUTING note. *(addresses: security supply-chain finding)*

**8. Optional quality improvements**
39. Extract `fileSlug` to a shared util; extract typed frontmatter readers; introduce `ApiError`; consolidate `MemoryRouter` to `src/test/`; wrap `sortEntries` and `MarkdownRenderer` in `useMemo`/`React.memo`. *(addresses: several minor code-quality and performance findings)*

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan establishes a reasonable separation between frontend scaffold and server integration with a clean cfg-gated asset module, but several architectural concerns warrant attention: middleware layering implications of `fallback_service`, the dual-mode `inner()` abstraction duplicating path logic, the `CARGO_MANIFEST_DIR`-relative dist path hardcoded in three places, and polymorphism-by-type-string in `LibraryDocView`/`LibraryTypeView` coupling the generic library flow to the templates special case.

**Findings**: see Major/Minor findings above attributed to architecture.

### Code Quality

**Summary**: The plan is generally well-structured with test-first discipline and thoughtful component boundaries, but contains a sham 'documentation test' that asserts a tautology, duplicated SPA-serving logic across public/private functions, silent test skipping that hides regressions from CI, hardcoded backend-key duplication in the sidebar, and a conditional-polymorphism dispatch pattern. Type-safety shortcuts accumulate to erode the strict-mode TypeScript configuration the plan asks for.

**Findings**: see above attributed to code-quality.

### Test Coverage

**Summary**: TDD is applied with reasonable breadth, but the testing strategy has significant gaps on the production code path (embed-dist mode has effectively no automated coverage); several tests are architecturally fragile (silent skips, implementation-detail coupling, inconsistent mock isolation); and several components (LibraryTemplatesIndex, router, SSE error paths) have no tests at all. Coverage is proportional to risk only on happy paths — error/loading/empty/routing are systematically underserved.

**Findings**: see above attributed to test-coverage.

### Correctness

**Summary**: Several correctness concerns around middleware/layer ordering, SPA fallback semantics, and React component data-flow edge cases. Most significant: axum's `route_layer` vs `layer` semantics; `embedded_fallback` unconditionally returns `index.html` for any unknown path (masking 404s including typos under `/api/`); and `useDocEvents` invalidates `lifecycle()` on every event while never invalidating `docContent`.

**Findings**: see above attributed to correctness.

### Security

**Summary**: The visualiser is local-only (loopback-bound with a host-header guard), so real attack surface is constrained to the local user's own content. The most concrete concerns are (1) SPA fallback swallows genuine API 404s with 200 HTML, degrading client-side error handling; (2) the plan lacks explicit documentation forbidding future enablement of raw-HTML markdown; (3) no supply-chain hygiene posture despite adding ~10 npm dependencies including `highlight.js`, `react-markdown`, and TanStack packages.

**Findings**: see above attributed to security.

### Usability

**Summary**: The plan delivers a coherent end-to-end DX with TDD throughout, sensible dual-mode builds, and incremental step-by-step verification. However, several onboarding friction points exist: the `__PORT__` placeholder in `vite.config.ts` is a manual step waiting to cause confusion, dual-mode test invocation surprises newcomers who run bare `cargo test`, test-skip messaging on missing `dist/` is silent, and multiple small UX omissions (no sort indicators, no keyboard accessibility on sort headers, no error states, no sample data/fixture for manual verification) degrade first-use experience.

**Findings**: see above attributed to usability.

### Standards

**Summary**: Generally follows reasonable industry conventions for React/Vite/Vitest and Rust/Cargo, but introduces inconsistencies with the existing wire-format conventions in the server and sets directory/filename conventions implicitly without declaring them. Most important: a concrete field-name mismatch between TypeScript `IndexEntry` and Rust server's camelCase JSON output (the critical `mtime_ms` vs `mtimeMs` issue). Inconsistent file-naming conventions (PascalCase vs kebab-case vs lowercase) should be made explicit before the precedent calcifies.

**Findings**: see above attributed to standards.

### Performance

**Summary**: For a local-only dev server the plan's performance profile is generally acceptable, but there are several concrete inefficiencies. The most important is an apparent mismatch between rust-embed's compile-time brotli compression and the `embedded_fallback` handler, which serves raw `content.data` with no Content-Encoding negotiation — browsers may receive undecodable bytes. Secondary concerns: missing memoisation in `MarkdownRenderer` and sort paths, over-broad SSE invalidation, and two redundant queries in `LibraryDocView` that could be replaced by a single endpoint.

**Findings**: see above attributed to performance.

---

## Re-Review (Pass 2) — 2026-04-24

**Verdict:** REVISE

Three lenses re-run after the Pass 1 revisions (architecture, correctness,
test-coverage). Pass 1 identified ~20 major findings plus one critical; the
plan was revised in place. Of the 20+ Pass 1 findings, **16 are fully
resolved** and 4 are **partially resolved** (intent addressed, but the
revisions introduced follow-on issues worth pinning). The re-review
surfaced **two new criticals** that were introduced during the revisions;
**both were fixed in the same commit as this re-review artifact** so the
plan is back to "all criticals resolved" — but the REVISE verdict stands
because 8 major findings remain open, above the `plan_revise_major_count`
threshold of 3.

### Previously Identified Issues

- ✅ **Standards** (Critical): TypeScript `IndexEntry` field names diverge from server camelCase — **Resolved** (`mtimeMs` + `ticket` field, contract comment).
- ✅ **Architecture/Correctness**: `route_layer(activity)` doesn't wrap SPA fallback — **Resolved** (`.layer()` applied after `apply_spa_serving_with_dist_path`; `spa_fallback_updates_activity` test pins it).
- ✅ **Correctness/Security**: SPA fallback returns 200 HTML for `/api/*` typos — **Resolved** (dedicated `/api/*rest` catch-all returning JSON 404, locked in by `unmatched_api_path_returns_json_404_not_spa_html`).
- ✅ **Correctness/Performance**: `doc-changed` doesn't invalidate `docContent` — **Resolved** (`dispatchSseEvent` now invalidates `queryKeys.docContent(event.path)`; test asserts the key).
- 🟡 **Architecture/Code Quality**: Triplicated SPA-serving construction with hardcoded dist literal — **Partially resolved**. `spa_router`/`inner` merged into one `apply_spa_serving_with_dist_path` function; `FRONTEND_DIST_REL` const exported; tests exercise the production function. But the literal `../frontend/dist` is still repeated across three sites (`FRONTEND_DIST_REL`, `build.rs`, `#[folder = "..."]` proc-macro attribute) with no drift-detection test — re-review flags this as a minor residual.
- ✅ **Architecture/Code Quality/Correctness/Performance**: Polymorphism-by-type-string for Templates — **Resolved** (dedicated `libraryTemplatesIndexRoute` + `libraryTemplateDetailRoute` as literal-path siblings; dispatch branches deleted; router test pins specificity).
- ✅ **Code Quality/Test Coverage**: `build_guard.rs` tautological test — **Resolved** (file deleted; step collapsed).
- ✅ **Code Quality/Test Coverage/Usability**: Silent `eprintln! SKIP + return` pattern — **Resolved** (all four tests now seed tempdir via `build_router_with_dist` and run unconditionally).
- ✅ **Test Coverage**: No automated tests for `embed-dist` path — **Resolved** (`path_normalisation_tests` + `embed_tests` module with `TestFrontend` fixture + 4 assertions).
- ✅ **Performance**: Brotli-compressed assets without `Content-Encoding: br` — **Resolved** (tower-http `CompressionLayer` added; regression test `spa_asset_is_brotli_encoded_for_br_clients` pins it).
- ✅ **Test Coverage**: `npm run test` at Step 3g may fail on empty suite — **Resolved** (Step 3g no longer runs the test command; explanatory comment added).
- ✅ **Code Quality/Correctness**: Hardcoded `NON_TEMPLATE_KEYS` in Sidebar — **Resolved** (deleted; symmetric `!t.virtual` / `t.virtual` filters; `virtual` now required in TS; Rust `skip_serializing_if` removed).
- 🟡 **Code Quality/Correctness**: `strict: false` + `'plans'` fallback — **Partially resolved**. `isDocTypeKey` + `parseParams` added at the router boundary; `'plans'` default removed; unsafe casts deleted. But components keep a belt-and-braces runtime narrowing that duplicates the router-side invariant — re-review flags this as a minor architectural duplication.
- ✅ **Test Coverage**: `LibraryTemplatesIndex` has no tests — **Resolved** (`LibraryTemplatesIndex.test.tsx` with 3 tests).
- ✅ **Test Coverage**: No tests for router tree / redirect / dispatch — **Resolved** (`router.test.tsx` with 4 tests covering root-redirect chain, bare `/library`, templates literal-path specificity, and unknown-type redirect).
- ✅ **Test Coverage**: SSE error path untested — **Resolved** (factory-based `onerror` handler emits `console.warn` and invalidates `['docs']`; two new tests).
- ✅ **Test Coverage/Code Quality**: EventSource mock shared `lastInstance` with split definition — **Resolved** (refactored into `dispatchSseEvent` pure function + `makeUseDocEvents(factory)`; tests inject fake via closure; setup.ts mock is now just a safety net).
- ✅ **Code Quality**: Triplicated SPA-serving indirection with trivial wrapper — **Resolved** (`apply_spa_serving` delegates to `apply_spa_serving_with_dist_path`; `inner` deleted).
- 🟡 **Security**: No documented prohibition on raw-HTML markdown — **Partially resolved**. Two XSS regression tests added (`<script>` tag and `javascript:` URL); no explicit comment forbidding future `rehype-raw` addition in the component file itself. The tests enforce the invariant, but a text comment would make the intent clearer to readers.
- ✅ **Usability**: `__PORT__` placeholder — **Resolved** (`resolveApiPort()` reads `VISUALISER_API_PORT` env var, falls back to `VISUALISER_INFO_PATH` JSON lookup, falls back to loud port 0).
- ✅ **Usability**: Bare `cargo test` surprises newcomers — **Resolved** (invoke tasks now wrap feature flags; mise tasks delegate to invoke; plan documents `mise run test:unit` as the canonical invocation).
- 🟡 **Usability**: Manual verification depends on specific filename / real meta content — **Partially resolved**. No fixture sample-meta directory added; this was deferred. The specific deep-link filename is still in the manual-verification checklist.
- ✅ **Usability/Standards**: Sort column headers not keyboard-accessible / no direction indicator — **Resolved** (`SortHeader` component wraps label in `<button type="button">`, shows ▲/▼ arrow, sets `aria-sort` on `<th>`, focus-visible outline in CSS).
- ✅ **Standards**: Inconsistent directory/file naming conventions — **Still present but not escalated**; re-review did not flag this as new major. The plan added no explicit conventions section but the overall layout across the revisions is now internally consistent.

### New Issues Introduced by Pass 1 Revisions

- 🔴 **Correctness** (Critical, high confidence): `useMemo` placed after conditional early returns in `LibraryTypeView` — Rules of Hooks violation. Would throw `Rendered more hooks than during the previous render` on state transition. **Fixed in the same commit as this review artifact** (moved useMemo above the early returns, with rationale comment).
- 🔴 **Correctness** (Critical, high confidence): `/api/{*rest}` uses axum 0.8 catch-all syntax; the crate pins axum 0.7 which uses `/*path`. Would panic at router-registration time or silently match a literal segment. **Fixed in the same commit** (changed to `/api/*rest` with comment pointing to the axum 0.7 syntax).
- 🟡 **Architecture** (Major, high confidence): The invoke task chain (`mise run test:unit:visualiser` → cargo with default features) has an implicit dependency on `npm run build` having populated `frontend/dist/index.html` (rust-embed proc-macro + `build.rs` guard). The ordering is documented in comments and in the implementation sequence, but not encoded in the task graph. Fresh-checkout `mise run test:unit` will fail until someone manually runs `npm run build` first.
- 🟡 **Architecture** (Minor, high confidence): `dist_path` parameter is silently ignored in the embed-dist variant of `apply_spa_serving_with_dist_path`. Shared signature is a leaky abstraction — same call, different semantics per feature. Worth renaming, switching to an enum, or adding a `_` prefix-convention to signal test-only.
- 🟡 **Architecture** (Minor, medium confidence): SSE-as-sole-invalidator (via `staleTime: Infinity`) has incomplete error-path reconciliation. `onerror` invalidates only `['docs']`; `doc-content`, `templates`, `template-detail`, `lifecycle`, `kanban` caches stay stale on reconnect.
- 🟡 **Architecture** (Minor, medium confidence): Default library landing type (`/library` → `/library/decisions`) is hardcoded in the router; the backend already publishes the type catalogue and the sidebar partitions on that data. Split source-of-truth.
- 🟡 **Architecture** (Minor, medium confidence): `parseParams` narrowing is duplicated inside `LibraryTypeView` and `LibraryDocView` — router already validates the URL param, but both components re-run `isDocTypeKey` and render an unreachable error branch.
- 🟡 **Architecture** (Minor, low confidence): `TIER_LABELS` constant duplicated across `LibraryTemplatesIndex` and `LibraryTemplatesView`. Below the extraction threshold `path-utils.ts` already cleared.
- 🟡 **Correctness** (Major, medium confidence): `await router.load()` in `router.test.tsx` may not resolve multi-hop redirect chains in TanStack Router v1. The chain `/` → `/library` → `/library/decisions` requires multiple re-evaluation passes; `load()` is single-pass. Tests may flake or assert against stale `router.state.location.pathname`. Recommendation: wrap assertions in `waitFor(...)` from `@testing-library/react`.
- 🟡 **Correctness** (Major, medium confidence): Status-column sort comparator compares `status ?? ''` while the cell displays `status ?? date ?? '—'`. Rows without `status` display a date but sort as empty strings — clicking the column produces a visible ordering that contradicts visible cell contents.
- 🟡 **Correctness** (Major, medium confidence): `from_fn_with_state(state.activity, ...)` applied at the outer level after the inner `with_state(state)`; the two state contexts are independent so this should work, but the ordering is subtle and unverified by a compile-smoke. Worth running `cargo check --features dev-frontend` as an explicit gate at the end of Step 2d before proceeding.
- 🟡 **Correctness** (Major, medium confidence): `invalidateQueries({ queryKey: ['docs'] })` in `onerror` relies on TanStack Query's prefix-match default. The test asserts the call argument but not the actual invalidation of child queries (`['docs', 'plans']` etc.). Recommendation: extend the test to seed two populated `queryKeys.docs(...)` queries and verify both are marked stale.
- 🔵 **Correctness** (Minor, medium confidence): The parseParams redirect chain `/library/bogus` → `/library` → `/library/decisions` depends on TanStack Router re-evaluating through both redirects. Future optimisations could collapse the chain; directly redirecting parseParams to `/library/decisions` simplifies.
- 🔵 **Correctness** (Minor, high confidence): `formatMtime`'s `Date.now() - ms` can produce negative diffs under server/client clock drift, yielding `-3s ago` labels. Clamp with `Math.max(0, ...)`.
- 🔵 **Correctness** (Minor, medium confidence): Client/server path-encoding contract is correct but untested. A filename with a space should round-trip through `/api/docs/with%20spaces/...` → decoded server-side → `with spaces/...`. Cheap to add one test.
- 🟡 **Test Coverage** (Major, high confidence): `fetchDocs`, `fetchTemplates`, `fetchTemplateDetail` have no tests. The per-segment `encodeURIComponent` added this round to `fetchDocContent` has no test that asserts the behaviour. A regression reverting to `encodeURIComponent(relPath)` would pass all existing tests while breaking slash-separator preservation.
- 🟡 **Test Coverage** (Major, high confidence): Error branches untested across every data-fetching view (`LibraryTypeView`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`). With `retry: 1`, a 500 or network failure is a realistic outcome — none of the current tests exercise `mockRejectedValue`. Also: `LibraryDocView` still has no tests for loading state, not-found state, or templates dispatch (dispatch is via router now, but the three original tests remain the only coverage).
- 🔵 **Test Coverage** (Minor, high confidence): `formatChipValue` nested-object branch added this round is not asserted by any `FrontmatterChips` test.
- 🔵 **Test Coverage** (Minor, medium confidence): `formatMtime` has four branches and zero dedicated tests.
- 🔵 **Test Coverage** (Minor, medium confidence): `fileSlugFromRelPath` is a routing-linchpin helper with no dedicated tests.
- 🔵 **Test Coverage** (Minor, high confidence): `Sidebar.test.tsx` still has no assertion for active-class behaviour (flagged in Pass 1 as minor; unchanged).
- 🔵 **Test Coverage** (Minor, medium confidence): `dispatchSseEvent` tests only pass `doc-changed`; the `doc-invalid` discriminated-union branch is untested. `lifecycle()` invalidation is also not asserted anywhere.
- 🔵 **Test Coverage** (Minor, medium confidence): `api_not_found` test asserts status + content-type but not the JSON body shape (`{"error":"not-found","path":"..."}`); schema regression could ship silently.
- 🔵 **Test Coverage** (Minor, low confidence): No test directly assigns `source.onmessage` — it's only exercised implicitly via the optional-chained call in the malformed-JSON test, so a regression dropping the `source.onmessage = ...` assignment would pass every existing test.

### Assessment

The Pass 1 revisions successfully resolved all Pass 1 majors that are
structural in nature — router topology, SPA fallback semantics, SSE
wiring, test coverage gaps, feature-flag plumbing. The two new criticals
introduced during the revisions (useMemo-after-early-return, axum 0.8
syntax on axum 0.7) are both narrow, localised bugs that are cheap to
fix and were corrected in-flight with this review.

The remaining majors fall into two clusters: (a) testing gaps that are
small but material (untested fetch helpers, untested error branches,
untested per-segment encoding), and (b) three architecture concerns
worth pinning before implementation — the implicit npm-build-before-cargo
ordering, the silently-ignored `dist_path` parameter in embed-dist, and
the `await router.load()` redirect-chain semantics in the router tests.

The plan is **ready for implementation under REVISE with a follow-up
pass** — none of the remaining majors block starting Step 1 (Cargo.toml
+ feature comments + docs.rs serialisation change), and most can be
addressed during or after their respective sub-step land with minimal
rework risk. The REVISE verdict reflects that the accumulated open
findings warrant another pass, not that any single finding blocks
implementation.

---

## Re-Review (Pass 3) — 2026-04-24

**Verdict:** APPROVE

Pass 3 is a follow-up focused on the 8 open major findings from Pass 2.
All 8 were addressed in-plan; the plan is now ready for implementation.

### Previously Identified Issues (from Pass 2)

- ✅ **Architecture**: Implicit npm-build-before-default-cargo ordering not encoded in the task graph — **Resolved** (`tasks/test/unit.py` `visualiser` task now calls a `_ensure_frontend_dist(context)` helper that runs `npm run build` if `frontend/dist/index.html` is missing, before the default-features cargo invocation).
- ✅ **Architecture**: `dist_path` silently ignored in embed-dist mode — **Resolved** (bifurcated: `apply_spa_serving_with_dist_path` and `build_router_with_dist` now only exist under the `dev-frontend` feature; embed-dist callers have no `_with_dist_path` variant to misuse. Production composition is shared via a single `build_router_with_spa(state, attach_spa)` helper taking a closure, so no code is duplicated across features).
- ✅ **Correctness**: `await router.load()` may not resolve multi-hop redirect chains — **Resolved** (router.test.tsx now uses a `waitForPath(router, expected)` helper built on `@testing-library/react`'s `waitFor`, polling `router.state.location.pathname` until the redirect chain settles).
- ✅ **Correctness**: Status-column sort key diverges from displayed value — **Resolved** (single `statusCellValue(entry)` helper used by both the sort comparator and the rendered cell; sort key and displayed value can no longer drift).
- ✅ **Correctness**: `from_fn_with_state` double state plumbing not verified — **Resolved** (explicit `cargo check --features dev-frontend` added as the first success-criterion step for Step 2, catching any middleware/state typechecking issue before the slower test run).
- ✅ **Correctness**: Prefix-match invalidation test only asserts call shape — **Resolved** (`onerror` test now seeds two populated `queryKeys.docs(...)` queries and asserts `queryClient.getQueryState(...).isInvalidated === true` on both; locks in TanStack Query's `exact: false` default behaviour rather than the call argument).
- ✅ **Test Coverage**: Half of fetch helpers + per-segment encoder untested — **Resolved** (fetch.test.ts grew from 3 tests to 12: fetchDocs gained 3 tests including parameter encoding and envelope unwrapping, fetchTemplates gained 2 tests, fetchTemplateDetail gained 2 tests, fetchDocContent gained 2 tests including a dedicated assertion for per-segment `encodeURIComponent` preserving slash separators while encoding spaces and `#` characters).
- ✅ **Test Coverage**: Error branches untested across data-fetching views — **Resolved** (all four data-fetching views gained an `isError` branch rendering `<p role="alert" className={styles.error}>`; each view's test file gained at least one `mockRejectedValue` test asserting the alert renders. LibraryDocView in particular grew from 3 tests to 7, adding not-found, loading, list-error, and content-error coverage. All four Wrapper factories now use `retry: false` so rejected fetches settle immediately).

### New Issues Introduced by Pass 2 Revisions

None. The Pass 2 work ended with two criticals that were fixed in the
same commit as the Pass 2 artifact (useMemo-after-early-return moved
before the early returns; `/api/{*rest}` axum-0.8 syntax changed to
`/api/*rest`). Pass 3 introduced no new hooks-ordering, feature-gate,
or layering bugs.

### Assessment

The plan has been through three review passes and now addresses every
critical and major finding surfaced across 8 lenses. Remaining findings
are all Minor severity — cosmetic or defensive improvements (e.g.
`formatMtime` clock-drift clamp, dedicated unit tests for
`fileSlugFromRelPath` and `formatChipValue`, `Sidebar` active-class
assertions, `doc-invalid` branch in `dispatchSseEvent` tests) that are
cheap to add during implementation or defer to Phase 10 polish.

The plan is **ready for implementation**. The numbered sequence is
self-contained, every behavioural claim has at least one test pinning
it in place, feature-gate decisions are consistent across Cargo,
tower-http, and the test matrix, and the router-composition
invariants are locked in by explicit middleware-coverage tests
(`spa_fallback_is_covered_by_host_header_guard`,
`spa_fallback_updates_activity`,
`unmatched_api_path_returns_json_404_not_spa_html`,
`spa_asset_is_brotli_encoded_for_br_clients`).
