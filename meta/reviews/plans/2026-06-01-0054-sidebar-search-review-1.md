---
date: "2026-06-01T21:15:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-06-01-0054-sidebar-search.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, performance, usability, security, standards]
review_pass: 2
status: complete
---

## Plan Review: 0054 Sidebar Search Input and API Search Endpoint

**Verdict:** REVISE

The plan is structurally strong — clean phase decomposition, TDD discipline, consistent precedents, and a thoughtful "What We're NOT Doing" section. However, three concerns recur across multiple lenses and warrant revision before implementation: (1) the four-bucket ranking strategy and `body_preview` matching diverge from the work item's pinned contract without acknowledgement, (2) the in-flight / empty-state rendering relies on imprecise claims about React Query's behaviour that misfire on cache-hit transitions, and (3) several test claims (mtime seeding, dedup `staleTime`, retry config, framed-Glyph assertion) are left unresolved and will produce non-deterministic tests. Secondary themes: end-user UX completeness (Escape, keyboard nav, live-region noise, flicker) and defence-in-depth for invariants (slug filter, length cap on `q`, Templates handler check).

### Cross-Cutting Themes

- **Plan diverges from work item AC/Technical Notes without acknowledgement** (flagged by: architecture, test-coverage, correctness) — The plan introduces a four-bucket ranking and `body_preview` bucket; the work item AC5 specifies plain `mtime_ms desc, path asc` and the Technical Notes explicitly scope out `body_preview` matching. The plan should either (a) drop the divergent behaviour or (b) update the work item AC and call out the resolution of the Open Question.
- **In-flight / cache-hit rendering semantics misstated** (flagged by: correctness, usability) — The plan claims React Query "auto-clears `data` for the new key", which is imprecise. Cache-hit transitions render synchronously (no transitional empty state), and the empty-state predicate `data?.length === 0 && !isFetching` will misfire when transitioning between cached-but-empty queries. Compounds the visible-flicker UX concern.
- **Test determinism gaps** (flagged by: test-coverage, correctness) — `mtime` seeding mechanism unresolved; Phase 5 tests don't pin `retry: false`; Phase 4 dedup tests depend on `staleTime` matching production; framed-Glyph assertion is "CSS class or wrapper element". Each will produce flaky or assertion-free tests as written.
- **Invariant duplication and silent failure modes** (flagged by: architecture, code-quality, security) — Slug-`Option<String>` filter is duplicated rather than enforced at projection; absolute `PathBuf` leaked in wire payload; no length cap on `q`; Templates exclusion not asserted at handler.
- **Keyboard / accessibility completeness** (flagged by: usability, standards) — No Escape handler, no arrow-key navigation, live-region announces noisily, empty state lacks `aria-live="polite"` pairing seen elsewhere in the codebase.

### Tradeoff Analysis

- **Plan compactness vs. component decomposition**: Code Quality wants `<SearchResultsPanel>` and `<SearchResultRow>` extracted; Architecture and Standards both favour the chosen "inline in Sidebar" shape as consistent with existing flat patterns. Recommendation: extract `<SearchResultsPanel>` (the four-branch state machine), keep the row JSX inline.
- **Silent error UX vs. AC fidelity**: Usability flags that silent UI on fetch failure (only `console.error`) is a poor end-user signal; the work item AC explicitly pins this behaviour. Recommendation: implement as specified for v1; add a Design Decision noting the trade-off and a follow-up work item reference.
- **In-flight clearing vs. typing flicker**: Usability prefers `placeholderData: keepPreviousData` to avoid flicker; Correctness needs to make AC9 hold across cache-hit transitions. Recommendation: adopt `keepPreviousData` and update Phase 5's wiring rationale to describe AC9 fulfilment under cache hits.

### Findings

#### Major

- 🟡 **Architecture**: Plan's four-bucket ranking diverges from the work item's `mtime_ms desc` contract without acknowledgement
  **Location**: Design Decisions (item 7) and Phase 1 — bucket-and-rank logic
  Work item AC5 specifies `mtime_ms desc, path asc`; plan codifies a 4-bucket strategy in tests (`bucket_1_exact_slug_first`, etc.) that would actively reject an AC-compliant handler. Either constrain to the AC or amend it.

- 🟡 **Correctness**: Plan includes `body_preview` bucket but work item Technical Notes scope it out
  **Location**: Phase 1 Design Decision 6 / handler step 4
  Technical Notes: "`body_preview` matching is out of scope at first cut". Plan adds bucket 4 and tests for it. Drop bucket 4 or widen scope in the work item before implementing.

- 🟡 **Test Coverage**: Server ranking tests assert a 4-bucket order that AC5 does not require — no test covers the AC's stated invariant
  **Location**: Phase 1 server tests
  Bucket tests will pass; the AC's plain mtime-desc invariant is never directly tested.

- 🟡 **Test Coverage**: mtime seeding strategy is unresolved — six of twelve server tests depend on deterministic mtime
  **Location**: Phase 1 integration tests (lines 292-297)
  `filetime` is not currently a dev-dep; plan defers the choice. Pin either `filetime::set_file_mtime` or `std::fs::File::set_modified` (MSRV 1.85 is fine) and add a helper to `tests/common/mod.rs`.

- 🟡 **Test Coverage**: Phase 5 Sidebar tests do not specify per-test `QueryClient` with `retry: false`
  **Location**: Phase 5 tests 7, 8, 9
  Production `retry: 1` will fire under fake timers and double the `console.error` count. Codify the per-suite `QueryClient` matching Phase 4's pattern.

- 🟡 **Correctness**: Empty-state predicate misfires when transitioning between cached-but-empty queries
  **Location**: Phase 5 §1 Sidebar wiring
  `data?.length === 0 && !isFetching && !isError` flips straight from rows to "No matches" on cache-hit transitions, contrary to the plan's stated AC9 mechanism.

- 🟡 **Correctness**: Dedup test relies on `staleTime` not being default-zero in test QueryClient
  **Location**: Phase 4 test 3 (`dedupes_via_react_query_cache_within_gctime`)
  Plan specifies `retry: false` for the test client but not `staleTime`. Default `staleTime: 0` would cause the test to refetch on revisit. Pin `staleTime: Infinity` in the test scaffold.

- 🟡 **Code Quality**: Sidebar gains a multi-branch state machine that should be extracted
  **Location**: Phase 5 §1
  Four interacting render branches inside Sidebar — extract `<SearchResultsPanel>` with the state machine documented in a JSDoc truth table.

- 🟡 **Code Quality**: Slug typing mismatch hides a real invariant
  **Location**: Phase 1 Design Decision 6
  `SearchResultRow.slug: String` (non-optional) vs `IndexEntry.slug: Option<String>`, with the filter only enforced in prose. Funnel projection through a single `fn project(entry: &IndexEntry) -> Option<SearchResultRow>` constructor.

- 🟡 **Security**: No length cap on `q` enables cheap server CPU amplification
  **Location**: Phase 1 Design Decision 7 / `SearchQuery` extractor
  Add a hard server-side cap (e.g. 128 or 256 bytes after trim). Client debounce is not a server-side defence.

- 🟡 **Usability**: No Escape-to-dismiss or focus-exit affordance
  **Location**: Phase 3 + Phase 5
  `/` to enter, no key to exit. Add `Escape` handler on the input (blur, optionally clear) and a test.

- 🟡 **Usability**: No keyboard navigation through results (arrow keys / Enter)
  **Location**: Phase 5
  Standard for `/`-driven palettes (GitHub, DocSearch, VS Code). Either add ArrowUp/Down + Enter in scope or explicitly defer with a follow-up work item.

- 🟡 **Usability**: In-flight result clearing will flicker as user types
  **Location**: Phase 5 in-flight handling
  Combine with the cache-hit correctness concern: adopt `placeholderData: (prev) => prev` (or `keepPreviousData`) and update wiring rationale.

- 🟡 **Usability**: `role="status"` live region will announce on every settle
  **Location**: Phase 5 empty-state markup
  Screen readers will say "No matches… No matches…" as the user types. Throttle the announcement or move it to a non-live element with a separate result-count summary.

- 🟡 **Usability**: Silent UI on fetch failure leaves user without feedback
  **Location**: Phase 5 + work item AC
  AC pins this behaviour for v1; plan should call out the trade-off and reference a follow-up.

#### Minor

- 🔵 **Architecture**: Server-side slug filtering papers over an `Option<String>` invariant that belongs in the indexer
  **Location**: Design Decision 3 / Phase 1 handler step 4
- 🔵 **Architecture**: Inline `/` keydown listener in RootLayout will need refactoring on the second hotkey
  **Location**: Phase 3
- 🔵 **Architecture**: `Indexer::all()` snapshot pattern shares lock contention with the indexer write path
  **Location**: Performance Considerations
- 🔵 **Architecture**: Error logging via `useEffect` couples logging to render lifecycle
  **Location**: Phase 5 §1 error logger
- 🔵 **Code Quality**: Inline keydown handler in RootLayout will be hard to read — extract `isPlainSlashKey` / `isEditableTarget` helpers
  **Location**: Phase 3
- 🔵 **Code Quality**: Bucket classification logic risks becoming nested if/else — use an enum `Bucket` + pure `classify` function
  **Location**: Phase 1 step 4
- 🔵 **Code Quality**: Error effect logs every render that retains the same error — risk of duplicate logs on retry
  **Location**: Phase 5 error effect
- 🔵 **Correctness**: Keybind `preventDefault` unconditional when input ref is null
  **Location**: Phase 3 §1
- 🔵 **Correctness**: AC literal reading of slug-less filter is ambiguous — confirm "filtered entirely" vs "non-link rows"
  **Location**: Design Decision 3
- 🔵 **Correctness**: `PathBuf` sort is OS-encoded; sort `rel_path.to_string_lossy()` for platform determinism
  **Location**: Phase 1 step 5
- 🔵 **Correctness**: Slug routing assumes URL-safe characters; no documented invariant on slug character set
  **Location**: Phase 5 §1
- 🔵 **Correctness**: Error logging effect ignores non-`FetchError` exceptions (network failure, JSON parse)
  **Location**: Phase 5 §1
- 🔵 **Correctness**: Server handler trims `q` after URL decode; add a `whitespace_only_q_returns_200_with_empty_results` test
  **Location**: Phase 1 integration tests
- 🔵 **Correctness**: Plan misstates React Query data lifecycle ("auto-clears data for the new key")
  **Location**: Phase 5 §1 in-flight rationale
- 🔵 **Test Coverage**: Settled-trimmed-key assertion method is vague — pin to `queryClient.getQueryState(['search', 'ab'])`
  **Location**: Phase 4 test 6
- 🔵 **Test Coverage**: Dedup tests should explicitly assert `not.toHaveBeenCalledWith('abc')` to cover AC3(b)
  **Location**: Phase 4 tests 3, 4
- 🔵 **Test Coverage**: Framed-Glyph assertion is left to implementer — pin `querySelector('span[data-doc-type]')`
  **Location**: Phase 5 test 5
- 🔵 **Test Coverage**: AC2 "inserts `/` normally" relies on JSDOM keystroke fidelity — pair with explicit `preventDefault not called` check
  **Location**: Phase 3 keybind test 2
- 🔵 **Test Coverage**: AC6 modifier-click / middle-click clause has no explicit test
  **Location**: Phase 5 test 3 — pin `tagName === 'A'` + `href` attribute
- 🔵 **Test Coverage**: Phase 5 in-flight test relies on React Query default behaviour; restructure to test contract directly
  **Location**: Phase 5 test 8
- 🔵 **Test Coverage**: Templates-exclusion test does not seed a template that would surface in `Indexer::all()`
  **Location**: Phase 1 test 3
- 🔵 **Performance**: Per-request full clone of heavy `IndexEntry` vector understated
  **Location**: Performance Considerations / Phase 1 §1 step 3
- 🔵 **Performance**: Read lock held across full vector clone serialises search against indexer writes
  **Location**: Phase 1 §1 step 3
- 🔵 **Performance**: Lowercasing `body_preview` unconditionally is wasted work for the common case
  **Location**: Phase 1 §1 step 4 — short-circuit per-bucket lowercasing
- 🔵 **Performance**: `staleTime: Infinity` + 5-min `gcTime` + no SSE invalidation = stale results across edits within 5 min
  **Location**: Performance Considerations / Phase 4 §5
- 🔵 **Performance**: Concurrent in-flight requests for different query strings can land out of order — thread `AbortSignal`
  **Location**: Phase 4 §5 / Phase 5 in-flight clearing
- 🔵 **Security**: Absolute server filesystem path leaked in wire payload despite being unused on client
  **Location**: Phase 1 Design Decision 6 — drop `path` from `SearchResultRow`, use `rel_path` or `${docType}/${slug}` as React key
- 🔵 **Security**: `FetchError` message echoes raw `q` into `console.error` (log injection / disclosure)
  **Location**: Phase 4 §3 — drop `q` from message; AC only requires `/api/search` substring
- 🔵 **Security**: Defence-in-depth: handler-level Templates assertion missing (mirror `docs.rs:35-37`)
  **Location**: Phase 1 handler
- 🔵 **Usability**: `useSearch` returns full `useQuery` result — consider narrowing to domain shape
  **Location**: Phase 4 §5
- 🔵 **Usability**: Optional `searchInputRef` prop allows silent miswiring — make required in Phase 5
  **Location**: Phase 3 / Phase 5
- 🔵 **Usability**: `/` keybind activation doesn't pin `key === '/'` vs `code === 'Slash'` — covers Shift+`/` edge cases
  **Location**: Phase 3 listener body
- 🔵 **Standards**: Empty state should pair `role="status"` with `aria-live="polite"` per Toaster / KanbanBoard / RelatedArtifacts convention
  **Location**: Phase 5 §1
- 🔵 **Standards**: Results list should have an accessible name (`aria-label="Search results"`) like other Sidebar landmarks
  **Location**: Phase 5 §1
- 🔵 **Standards**: App-wide `retry: 1` default may apply to search — confirm or override explicitly
  **Location**: Phase 4 §5
- 🔵 **Standards**: `mtime_ms` field serialises as `mtimeMs` under `rename_all = "camelCase"` — call out reconciliation with work item wire-shape doc
  **Location**: Phase 1 §1

#### Suggestions

- 🔵 **Code Quality**: Extract `<SearchResultRow result={r} />` co-located component
  **Location**: Phase 5 §1
- 🔵 **Performance**: Restate the per-bucket sort bound as O(M log M) explicitly in Performance Considerations
  **Location**: Performance Considerations
- 🔵 **Usability**: Add `:focus-visible` styling to `.resultRow` in Phase 5 CSS spec
  **Location**: Phase 5 §2 CSS
- 🔵 **Standards**: Note in Phase 3 that keybind test relies on `aria-label="Search"` from 0053
  **Location**: Phase 3 §3

### Strengths

- ✅ Strong phase decomposition — phases 1–4 land in any order, phase 5 composes; respects file-boundary independence and existing dependency graph.
- ✅ TDD-first sequencing with explicit per-AC test mapping for every phase.
- ✅ Handler shape, projection struct, and `#[serde(rename_all = "camelCase")]` all mirror the `docs.rs` precedent exactly.
- ✅ Frontend api-layer additions follow existing `use-*.ts` / `fetchTypes` / `queryKeys.search` conventions.
- ✅ Bucket precedence uses "first match wins" iteration, correctly excluding prefix matches from bucket 3 without explicit `!starts_with` predicates.
- ✅ Trim is applied client-side before debounce and server-side defensively in the handler.
- ✅ Below-2-char branch uses the existing `queryKeys.disabled('search')` sentinel — clean reuse of established gating convention.
- ✅ State ownership decision (RootLayout owns ref + listener, Sidebar owns query state) avoids premature React Context.
- ✅ Type-safe routing via `<Link to="/library/$type/$fileSlug" params={...}>` rather than raw `<a href={template}>`.
- ✅ Explicit, well-justified "What We're NOT Doing" section calls out deliberate v1 trade-offs (no useHotkey extraction, no rate limiting, no match highlighting).
- ✅ `SearchResultRow` projection decoupled from `IndexEntry` — keeps the wire schema minimal and prevents internal indexer field churn from leaking.
- ✅ XSS auto-escaping correctly identified as the defence (plain `{r.title}` JSX text nodes, not `dangerouslySetInnerHTML`).
- ✅ `encodeURIComponent` used on the client URL build, preventing `&`/`#` smuggling.
- ✅ Test conventions (per-suite providers, `vi.spyOn`, no MSW) match existing api-layer test idioms.
- ✅ Templates exclusion has an explicit regression test even though enforcement is one layer down.

### Recommended Changes

1. **Resolve the work-item contract divergence** (addresses: bucket-ranking-AC mismatch, body_preview scope, ranking tests vs AC5)
   Pick one path explicitly in the plan:
   - **Path A**: Drop bucket 4 (body_preview), reduce to 3 buckets or to plain `mtime_ms desc + path asc`. Rewrite ranking tests against the AC's literal invariant. Document this as the v1 ranking; bucketing becomes follow-up work.
   - **Path B**: Update `meta/work/0054-sidebar-search.md` to specify the 4-bucket strategy in AC5 and remove the Technical-Notes exclusion of `body_preview`. Note in the plan's Design Decisions that this resolves the Open Question.

2. **Pin the test-determinism gaps** (addresses: mtime seeding, retry config, dedup staleTime, framed Glyph, query-key introspection)
   - Choose the mtime seeding mechanism: add `filetime` to `[dev-dependencies]` (or use `std::fs::File::set_modified` if MSRV 1.85 suffices) and a `set_mtime_ms` helper in `tests/common/mod.rs`. Cite it in Phase 1.
   - Specify a per-suite `QueryClient` for Phase 5 with `{ defaultOptions: { queries: { retry: false, staleTime: Infinity } } }`. Match Phase 4.
   - Replace "CSS class or framed wrapper" with `querySelector('span[data-doc-type]')` (or equivalent stable selector). `Glyph` already produces a `data-doc-type` attribute when `framed`.
   - Replace the "introspection helper OR parallel useQuery" choice in Phase 4 test 6 with `queryClient.getQueryState(['search', 'ab'])`.

3. **Tighten the in-flight / empty-state rendering logic** (addresses: empty-state predicate misfire, "auto-clears data" misstatement, flicker)
   - Adopt `placeholderData: (prev) => prev` (or `keepPreviousData`) on the `useQuery` to hold stale rows during in-flight transitions.
   - Gate empty state on `search.isSuccess === true && search.data?.length === 0` (or track the settled key locally so the panel only renders for the current key).
   - Rewrite the Phase 5 wiring rationale to describe cache-hit semantics correctly — React Query returns the cache slot for the new key (which may be cached `data`, not `undefined`).

4. **Funnel slug filtering through projection** (addresses: slug typing mismatch, server-side filter duplication)
   - Replace prose "filter None before projection" with a typed constructor: `fn project(entry: &IndexEntry) -> Option<SearchResultRow>` is the only path that constructs a `SearchResultRow`. Bucketing iterates `entries.iter().filter_map(project)`.

5. **Extract `<SearchResultsPanel>` from Sidebar** (addresses: god-component growth, in-flight state-machine readability)
   - One sibling component owns the four-branch state machine with a JSDoc truth table at the top. Sidebar becomes a one-liner consumer.

6. **Cap `q.len()` server-side** (addresses: CPU amplification on long `q`)
   - Add a constant (e.g. `const MAX_Q_LEN: usize = 128`) and short-circuit to empty results for over-length input. One line; preserves the AC contract.

7. **Drop `path` from `SearchResultRow`; use `rel_path` or compose React `key` client-side** (addresses: absolute path information disclosure)
   - Key results by `${r.docType}/${r.slug}` (still unique). Remove `path` from the wire payload.

8. **Trim `q` out of the FetchError message** (addresses: log injection / disclosure)
   - Use `\`GET /api/search: ${r.status}\`` — still satisfies the AC's "includes /api/search" assertion.

9. **Address keyboard / accessibility completeness** (addresses: Escape, kbd nav, live-region noise, aria-live, aria-label, focus-visible)
   - Add `Escape` handler on the search input (blur + optional clear) with a Phase 3/5 test.
   - Decide arrow-key navigation through results: either in scope (add ACs + tests) or explicitly deferred with a tracked follow-up work item under "What We're NOT Doing".
   - Pair empty-state `role="status"` with `aria-live="polite"` and consider debouncing the announcement separately from visual rendering.
   - Wrap results list in `<section aria-label="Search results">` or set `aria-label` directly on the `<ul>`.
   - Add `:focus-visible` styling to `.resultRow`.

10. **Pin defensive invariants and signal handling** (addresses: Templates handler check, AbortSignal, error effect coverage, ref-null preventDefault)
    - Add a `.filter(|e| e.r#type != DocTypeKey::Templates)` defence-in-depth in the handler (mirror `docs.rs:35-37`).
    - Thread `AbortSignal` from React Query's `queryFn` context into `fetchSearch`.
    - In Phase 3 keybind: guard `preventDefault` together with `focus()` — early-return if `searchInputRef.current` is null.
    - Either move `console.error` into `fetchSearch`'s catch path or widen the Phase 5 effect to log any truthy error (not just `instanceof FetchError`).

11. **Document standards-level reconciliations** (addresses: `mtimeMs` serialisation, `retry: 1` default, `aria-label="Search"` cross-coupling, second-hotkey refactor target)
    - Add a one-line Design Decision noting the wire field is `mtimeMs` (camelCase), reconciling against the work item's `mtime_ms` notation.
    - State explicitly whether `useSearch` keeps `retry: 1` (default) or overrides to `retry: false`.
    - Note Phase 3's reliance on `aria-label="Search"` (from 0053).
    - Add a one-line note that the inline `/` listener is the deliberate v1 shape and that a `HotkeyRegistryContext` is the expected refactor target when a second hotkey lands.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan establishes clean module boundaries (server handler colocated with peers, frontend hook in the `api/` layer) and decomposes the work into independently-mergeable phases that respect the existing dependency graph. However, it diverges from the work item's ordering contract by introducing a four-bucket ranking strategy that was not specified, conflates an `Option<String>` slug invariant by filtering server-side rather than at the indexer source, and pushes the first global keydown listener and first RootLayout ref into the layout component without considering how a second hotkey consumer would force a context/registry refactor.

**Strengths**:
- Phase decomposition respects file-boundary independence.
- Handler layout mirrors the `docs.rs` precedent.
- `SearchResultRow` decoupled from `IndexEntry`.
- State ownership (RootLayout owns ref + listener) avoids premature React Context.
- Frontend api-layer follows established conventions.
- `useDebouncedValue` correctly positioned as a small single-consumer primitive.

**Findings**:
- 🟡 *major*: Plan's four-bucket ranking diverges from the work item's `mtime_ms desc` contract without acknowledgement.
- 🔵 *minor*: Server-side slug filtering papers over an `Option<String>` invariant that belongs in the indexer.
- 🔵 *minor*: Inline `/` keydown listener locks in a pattern that will need refactoring on the second hotkey.
- 🔵 *minor*: Every search request clones the full entry map under the indexer's `RwLock`, sharing contention with the write path.
- 🔵 *minor*: Error logging via `useEffect` couples logging to render lifecycle.

### Code Quality

**Summary**: The plan demonstrates strong code-quality discipline: it explicitly follows existing precedents (fetchTypes, useRelated, docs.rs handler shape), defends against over-engineering (no new context, no useHotkey extraction), and structures phases for independent testability. A handful of concerns remain around the Sidebar component's growing responsibilities, the in-flight/empty/error state machine readability, and an inconsistency between SearchResultRow's server projection and the documented IndexEntry shape that may produce ambiguous error paths.

**Strengths**:
- TDD-first sequencing for every phase.
- Explicit alignment with existing precedents.
- YAGNI applied deliberately and named.
- Single-responsibility primitives (`useDebouncedValue` ~10 lines).
- Concrete error-handling shape (`FetchError` with `/api/search` in message).
- Design decisions section captures non-obvious choices with rationale.
- Type-safe routing via `<Link>` with params.

**Findings**:
- 🟡 *major*: Sidebar gains a multi-branch state machine that should be extracted as `<SearchResultsPanel>`.
- 🟡 *major*: Slug typing mismatch (`SearchResultRow.slug: String` vs `IndexEntry.slug: Option<String>`) hides a real invariant.
- 🔵 *minor*: Inline keydown handler in RootLayout will be hard to read — extract `isPlainSlashKey` / `isEditableTarget` helpers.
- 🔵 *minor*: Bucket classification logic risks becoming nested if/else — use enum + pure `classify` function.
- 🔵 *minor*: Error effect logs every render that retains the same error.
- 🔵 *suggestion*: Extract `<SearchResultRow result={r} />` co-located component.

### Test Coverage

**Summary**: The plan has strong TDD intent with explicit per-phase test lists that map closely to most of the 13 ACs, and it sensibly mocks at fetch/hook boundaries while integration-testing the server handler end-to-end. However several test claims are technically shaky (mtime seeding deferred; `query_key_uses_settled_trimmed_value` method vague; the in-flight "auto-clears data" claim is imprecise; retry/timing controls in Phase 5 risk flakes), and the server tests assert a 4-bucket ordering that the work item's AC5 does not require, creating a coverage mismatch between tests and acceptance criteria.

**Strengths**:
- Tests are written FIRST in every phase, with explicit AC→test mapping.
- Integration tests run against `AppState::build` + `build_router` + `oneshot`.
- The api-layer split keeps unit tests fast and isolated.
- Dedup ACs get dedicated tests at the `useSearch` level with `vi.useFakeTimers`.
- Error-path coverage is explicit.
- Listener-cleanup test guards against the common `useEffect`-with-document-listener regression.

**Findings**:
- 🟡 *major*: mtime seeding strategy is unresolved — six server tests depend on deterministic mtime.
- 🟡 *major*: Server ranking tests assert a 4-bucket order that AC5 does not require.
- 🟡 *major*: Phase 5 Sidebar tests do not specify per-test `QueryClient` with `retry: false`.
- 🔵 *minor*: Settled-trimmed-key assertion method is vague — pin to `queryClient.getQueryState(['search', 'ab'])`.
- 🔵 *minor*: Dedup tests should explicitly assert `not.toHaveBeenCalledWith('abc')`.
- 🔵 *minor*: Framed-Glyph assertion is left to implementer — pin a concrete selector.
- 🔵 *minor*: AC2 "inserts `/` normally" relies on JSDOM keystroke fidelity — pair with `preventDefault not called` check.
- 🔵 *minor*: AC6 modifier-click / middle-click clause has no explicit test — pin `tagName === 'A'` + `href` attribute.
- 🔵 *minor*: Phase 5 in-flight test relies on React Query default behaviour without locking SUT.
- 🔵 *suggestion*: Templates-exclusion test doesn't seed a template that would surface in `Indexer::all()`.

### Correctness

**Summary**: The plan is generally sound on logical structure: bucket precedence is enforced by "first match wins", trimming happens before debounce so the wire payload is consistent, and the `disabled('search')` sentinel correctly nulls `data` when below the 2-char threshold. However, there are several real correctness gaps: the plan adds body_preview as a fourth ranking bucket which contradicts the work item's Technical Notes; the plan's assertion that React Query "auto-clears `data` for the new key" is imprecise and the in-flight rendering logic will misbehave on cache hits; the dedup test for AC3 example (a) relies on cache reuse that is undermined unless the test QueryClient sets `staleTime: Infinity`; and the global `/` keybind always calls `preventDefault()` even when the focus target ref is null.

**Strengths**:
- Client trim + server trim gives a consistent wire payload.
- Bucket precedence uses "first match wins".
- Bucket-1 slug equality is symmetric (both lowercased).
- Below-2-char uses `queryKeys.disabled('search')` so AC7 follows.
- Phase 5 cleanly composes prior phases.
- Phase boundaries clean; TDD sequence well-defined.

**Findings**:
- 🟡 *major*: Plan includes body_preview bucket but work item Technical Notes scope it out.
- 🟡 *major*: Empty-state predicate misfires when transitioning between cached-but-empty queries.
- 🟡 *major*: Dedup test relies on `staleTime` not being default-zero in test QueryClient.
- 🔵 *minor*: Keybind `preventDefault` unconditional when input ref is null.
- 🔵 *minor*: AC literal reading allows non-link rendering; plan filters server-side without explicit confirmation.
- 🔵 *minor*: `PathBuf` sort is OS-encoded; may be non-deterministic across platforms.
- 🔵 *minor*: Slug routing assumes URL-safe characters; no documented invariant.
- 🔵 *minor*: Error logging effect ignores non-`FetchError` exceptions.
- 🔵 *minor*: Server handler trim coverage gap — add whitespace-only `q` test.
- 🔵 *minor*: Plan misstates how React Query treats `data` across key changes.

### Performance

**Summary**: The plan's hot-path performance is broadly defensible for the stated scale (10²–10³ entries, debounced + 2-char gated requests), and the choice to keep `Indexer::all()` snapshotting rather than introduce an inverted index is appropriately conservative for v1. However, the cost analysis understates two real costs: heavy-struct cloning during the snapshot, and the lock held across the clone serialising search against indexer writes. The React Query strategy (`staleTime: Infinity` with no SSE invalidation) is acknowledged but combined with the 5-minute default `gcTime` will return stale results to repeat-typed queries within a session after file edits.

**Strengths**:
- 200 ms debounce + 2-char minimum is the right place to cut work.
- Single-pass bucket classification with first-match-wins avoids redundant lowercasing.
- Sort-per-bucket bounds work to the matching subset.
- Minimal `SearchResultRow` projection keeps payload size proportional to result count.
- Reusing `Indexer::all()` avoids new lock-contention surface area.

**Findings**:
- 🔵 *minor*: Per-request full clone of heavy `IndexEntry` vector understated.
- 🔵 *minor*: Read lock held across full vector clone serialises search against indexer writes.
- 🔵 *minor*: Lowercasing `body_preview` unconditionally is wasted work for the common case.
- 🔵 *minor*: `staleTime: Infinity` + 5-min `gcTime` + no SSE invalidation = stale results across edits.
- 🔵 *minor*: Concurrent in-flight requests for different query strings can land out of order — thread `AbortSignal`.
- 🔵 *suggestion*: Restate the per-bucket sort bound as O(M log M) explicitly.

### Usability

**Summary**: The plan is generally well-thought-through from a DX standpoint — phases are independent, hook APIs follow existing precedents, and the `searchInputRef` prop contract was deliberately chosen over context. However, the end-user UX has notable gaps: no Escape-to-dismiss, no keyboard navigation through results, `role="status"` live-region behaviour will be noisy with debounce-driven settles, in-flight result clearing is likely to flicker, and silent error handling leaves users staring at a blank panel.

**Strengths**:
- Hook APIs follow existing app conventions.
- `searchInputRef` ownership decision is the simplest possible contract for a single consumer.
- Phases are independent.
- Each phase has both automated and manual verification checklists.
- Net-new infrastructure kept as small single-consumer primitives.

**Findings**:
- 🟡 *major*: No Escape-to-dismiss or focus-exit affordance.
- 🟡 *major*: No keyboard navigation through results (arrow keys / Enter).
- 🟡 *major*: In-flight result clearing will flicker as user types.
- 🟡 *major*: `role="status"` live region will announce on every settle.
- 🟡 *major*: Silent UI on fetch failure leaves user without feedback.
- 🔵 *minor*: `useSearch` returns full `useQuery` result — under-specified contract.
- 🔵 *minor*: Optional `searchInputRef` prop allows silent miswiring.
- 🔵 *minor*: Activation logic doesn't consider Shift+`/` (?) variants or pin `key` vs `code`.
- 🔵 *suggestion*: Result row has no `:focus-visible` affordance specified.

### Security

**Summary**: The plan introduces a new GET /api/search endpoint and a global keybind on a local-dev visualiser with no authentication model, so the threat surface is modest. The most defensible findings concern (1) unbounded input on q enabling cheap CPU/memory amplification, (2) the decision to include absolute server-filesystem `PathBuf` in the wire payload when nothing on the client uses it, and (3) the `FetchError` message echoing the raw `q` string into `console.error`. None are critical given the deployment model, but each is a cheap hardening.

**Strengths**:
- React/JSX auto-escaping correctly identified as XSS defence.
- `<Link>` with type-safe `params` keeps slug out of the URL template path.
- Templates exclusion is structural with an explicit regression test.
- `encodeURIComponent` used on client URL build.
- Keybind guards prevent `/` from stealing keystrokes intended for text fields.

**Findings**:
- 🟡 *major*: No length cap on `q` enables cheap server CPU amplification.
- 🔵 *minor*: Absolute server filesystem path leaked in wire payload despite being unused on client.
- 🔵 *minor*: `FetchError` message echoes raw `q` into `console.error`.
- 🔵 *minor*: Defence-in-depth: handler-level Templates assertion missing.

### Standards

**Summary**: The plan adheres closely to existing project conventions across the board — file naming, handler signatures, serde renames, alphabetical `mod` declarations with feature-grouped route registration, React Query gating, and CSS module camelCase class names. A small number of accessibility and convention-pairing gaps are worth flagging: the empty state lacks the codebase-typical `aria-live="polite"` pairing, the results list lacks an `aria-label`/`aria-labelledby` (unlike every other Sidebar section), and the production `retry: 1` default deserves explicit consideration. None are blockers.

**Strengths**:
- Hook file naming follows established `use-*.ts` convention.
- Handler signature and serde rename match `docs.rs` precedent.
- Module declaration placement observes alphabetical ordering; route registration is feature-grouped.
- `useSearch` returns full `useQuery` result mirroring `useRelated` / `useTypes`.
- `FetchError` re-use preserves cross-cutting fetch-error contract.
- Test file naming matches existing per-suite conventions.
- CSS class names follow camelCase convention; design tokens used consistently.
- Test conventions (per-suite providers, `vi.spyOn`, `retry: false`) match existing tests.

**Findings**:
- 🔵 *minor*: Empty state should pair `role="status"` with `aria-live="polite"`.
- 🔵 *minor*: Results list should have an accessible name like other Sidebar sections.
- 🔵 *minor*: App-wide `retry: 1` default may apply to search — confirm or override.
- 🔵 *minor*: `mtime_ms` serialises as `mtimeMs` under camelCase rename — call out reconciliation with work item wire-shape doc.
- 🔵 *suggestion*: Note Phase 3 keybind test reliance on `aria-label="Search"` from 0053.

## Re-Review (Pass 2) — 2026-06-01

**Verdict:** APPROVE

Pass 1 raised 41 findings (15 major, 26 minor/suggestion). Pass 2 confirms 38 are resolved or invalid; 3 are partially resolved with concrete next-step asks that the author then addressed in pass-2 edits. Pass 2 introduced 13 new findings — 0 critical, 0 major, 7 minor, 6 suggestion — and the most load-bearing of these (stale wording, missing AC10 test, in-flight test timer specifics, `sort_by_cached_key`, pre-trim length cap, AC11 Enter-key test) have been folded into the plan. The plan is now ready for implementation.

### Previously Identified Issues

**Architecture**
- 🟡 Plan's four-bucket ranking diverges from AC5 — Invalid (work item AC5 has always specified the 4-bucket strategy; pass-1 reviewer misread the AC)
- 🔵 Slug filtering papers over Option<String> invariant — Resolved via typed `project()` constructor (Design Decision 6)
- 🔵 Inline keydown listener locks in pattern for second hotkey — Resolved via Design Decision 11 (`HotkeyRegistryContext` as v2 target) + extracted helpers
- 🔵 `Indexer::all()` snapshot lock contention — Resolved via honest Performance Considerations restatement + `all_search_projection()` follow-up
- 🔵 Error logging via useEffect couples to render lifecycle — Resolved by moving log to `fetchSearch` catch path (Design Decision 10)

**Code Quality**
- 🟡 Sidebar multi-branch state machine — Resolved via `<SearchResultsPanel>` extraction with JSDoc truth table
- 🟡 Slug typing mismatch — Resolved via typed `project()` constructor
- 🔵 Inline keydown handler hard to read — Resolved via `isPlainSlashKey` / `isEditableTarget` extraction
- 🔵 Bucket classification nested if/else — Resolved via `Bucket` enum + pure `classify` function
- 🔵 Error effect logs every retain — Resolved by relocation to `fetchSearch`
- 🔵 Extract `<SearchResultRow>` — Acknowledged (intentionally not done — `SearchResultsPanel` is small)

**Test Coverage**
- 🟡 mtime seeding strategy unresolved — Resolved via `std::fs::File::set_modified` + `set_mtime_ms` helper (no new dep)
- 🟡 Ranking tests vs AC5 — Invalid (same as architecture)
- 🟡 Phase 5 Sidebar tests don't pin QueryClient retry:false — Resolved via per-suite QueryClient with `retry: false, staleTime: Infinity, gcTime: Infinity`
- 🔵 Settled-trimmed-key assertion vague — Resolved via `queryClient.getQueryState` direct introspection
- 🔵 Dedup tests don't assert `not.toHaveBeenCalledWith('abc')` — Resolved via explicit negative assertion
- 🔵 Framed-Glyph assertion left to implementer — Resolved via `span[data-doc-type]` selector
- 🔵 AC2 relies on JSDOM keystroke fidelity — Resolved via explicit `preventDefault not called` test
- 🔵 AC6 modifier-click clause has no test — Partially resolved via `tagName === 'A'` + `href` assertion; AC11 Enter-key clause added in pass-2 cleanup (test 3a)
- 🔵 Phase 5 in-flight test relies on RQ default — Partially resolved via amended AC9; timer-advancement and `isPlaceholderData` assertion specifics added in pass-2 cleanup
- 🔵 Templates-exclusion test inadequate — Resolved via structural `Indexer::all()` assertion as tripwire

**Correctness**
- 🟡 body_preview bucket out of scope — Invalid (work item Technical Notes line 104 has always specified body_preview as bucket 4)
- 🟡 Empty-state predicate misfires on cached-empty transitions — Resolved via `placeholderData: keepPreviousData` + `!isPlaceholderData` gate
- 🟡 Dedup test relies on default staleTime — Resolved via explicit `staleTime: Infinity` pin in test QueryClient
- 🔵 Keybind preventDefault unconditional when ref null — Resolved via `if (!input) return` guard
- 🔵 PathBuf sort OS-encoded — Resolved via `rel_path.to_string_lossy()` tiebreak
- 🔵 Slug routing assumes URL-safe — Resolved via documented kebab-case invariant in Design Decision 4
- 🔵 Error effect ignores non-FetchError — Resolved via fetchSearch catch logging any non-AbortError exception
- 🔵 Server trim coverage gap — Resolved via `whitespace_only_q` test
- 🔵 "Auto-clears data" misstated — Resolved via accurate cache-slot semantics in Design Decision 8

**Performance**
- 🔵 Per-request heavy IndexEntry clone understated — Resolved via honest cost analysis + `all_search_projection()` follow-up
- 🔵 Read lock across full clone — Resolved via writer-priority queueing note
- 🔵 Lowercasing body_preview unconditionally — Resolved via classify short-circuit
- 🔵 staleTime + gcTime + no SSE — Resolved via two named follow-up options
- 🔵 Out-of-order completion — Resolved via AbortSignal threading
- 🔵 O(M log M) bound — Resolved via explicit restatement

**Usability**
- 🟡 No Escape-to-dismiss — Resolved via Escape handler + test 13
- 🟡 No keyboard navigation through results — Deferred (acceptable) with explicit Tab+Enter fallback rationale
- 🟡 In-flight result clearing flicker — Resolved via `keepPreviousData`
- 🟡 role=status announces on every settle — Deferred (acceptable) with follow-up trigger; `aria-live="polite"` retained per project convention
- 🟡 Silent UI on fetch failure — Deferred (acceptable) with explicit follow-up trigger
- 🔵 useSearch returns full useQuery result — Resolved via Design Decision 12
- 🔵 Optional searchInputRef allows miswiring — Resolved via required prop in Phase 5
- 🔵 Shift+/ not considered — Resolved via `!event.shiftKey` guard + test 9
- 🔵 No :focus-visible affordance — Resolved via CSS spec update

**Security**
- 🟡 No length cap on q — Resolved via `MAX_Q_LEN = 128` + over-length test (pre-trim ordering refined in pass-2 cleanup)
- 🔵 Absolute path leaked in payload — Resolved by removing `path` from `SearchResultRow`
- 🔵 FetchError echoes raw q — Resolved via trimmed message + test assertion
- 🔵 Defence-in-depth Templates assertion missing — Resolved via handler-level filter mirroring `docs.rs:35-37`

**Standards**
- 🔵 role=status should pair with aria-live=polite — Resolved
- 🔵 Results list needs accessible name — Resolved via `<section aria-label="Search results">`
- 🔵 retry: 1 default applies — Resolved via Design Decision 12
- 🔵 mtime_ms → mtimeMs reconciliation — Resolved via Design Decision 6 + test 15
- 🔵 keybind test depends on aria-label="Search" — Resolved via cross-coupling note

### New Issues Introduced

Pass-2 cleanups (folded in immediately):
- 🔵 **Architecture (minor)**: Manual verification step contradicted `keepPreviousData` design — Fixed (Phase 5 manual checklist now reads "previous rows remain visible until new response arrives")
- 🔵 **Code Quality (minor)**: TDD step 6 referenced now-removed useEffect error logger — Fixed (step rewritten to verify no Sidebar logging effect is added)
- 🔵 **Code Quality (minor)**: Stale test counts (12 vs 15 server, 10 vs 14 sidebar) — Fixed (counts dropped in favour of "all tests")
- 🔵 **Test Coverage (minor)**: AC10 no-substring-match exclusion lacked a dedicated test — Fixed (added test 16, `non_matching_entries_are_excluded`)
- 🔵 **Test Coverage (minor)**: AC11 Enter-key clause asserted only structurally — Fixed (added test 3a, `enter_on_focused_result_row_navigates`)
- 🔵 **Test Coverage (minor)**: in-flight test underspecified timer advancement and `isPlaceholderData` — Fixed (timer advancement and `queryClient.getQueryState` checks pinned)
- 🔵 **Performance (suggestion)**: Sort key allocates String per comparison — Fixed (switched to `sort_by_cached_key`)
- 🔵 **Security (minor)**: Length cap applied after trim allows whitespace padding — Fixed (cap now applied before trim)
- 🔵 **Test Coverage (suggestion)**: Bucket tests don't exercise mixed-case fields — Fixed (added test 17, `mixed_case_query_and_field_classify_correctly`)
- 🔵 **Usability (suggestion)**: Tab order from input to first result row not verified — Fixed (added manual verification step)

Pass-2 cleanups left as acknowledged future improvements:
- 🔵 **Architecture (suggestion)**: `searchInputRef` threads through presentational Sidebar — Note: subsumed by the documented `HotkeyRegistryContext` follow-up in Design Decision 11
- 🔵 **Architecture (suggestion)**: Bucket assignment for slug-less entries computed then discarded — Note: documented architectural seam; type system still prevents emission
- 🔵 **Code Quality (suggestion)**: Truth-table row 3 inverted-positive harder to read — Note: implementer can add inline comment when writing the code
- 🔵 **Usability (suggestion)**: Escape on empty input still blurs vs two-press pattern — Note: deliberate v1 choice (Design Decision 4-context); reconsider if user feedback surfaces

### Assessment

The plan is in good shape for implementation. Every major finding from pass 1 is either resolved with substantive structural changes (typed projection, extracted SearchResultsPanel, MAX_Q_LEN cap, placeholderData transition, AbortSignal plumbing) or explicitly deferred with documented rationale and follow-up triggers (arrow-key navigation, live-region throttling, silent error UI). The work item AC9 was amended to authorise the `keepPreviousData` shape so the plan/AC alignment is clean. Pass-2 introduced only minor/suggestion-level cleanups, all of which have been folded back into the plan. No critical or major issues remain. Approve for implementation.
