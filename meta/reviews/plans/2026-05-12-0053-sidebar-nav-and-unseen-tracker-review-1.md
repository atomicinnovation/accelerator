---
date: "2026-05-12T21:01:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, performance, standards, usability, compatibility]
review_pass: 2
status: complete
---

## Plan Review: Sidebar Nav with Per-Type Change Indicators

**Verdict:** REVISE

The plan is well-structured, test-first, and reuses established patterns
(owning-hook + Context, factory test seam, self-cause guard) consistently.
However, eight lenses surfaced a coherent cluster of issues that need to be
resolved before implementation: two correctness bugs that will ship as user-
visible regressions (active-state prefix match collisions and doc-invalid
self-cause bypass), an under-specified reconnect/`onEvent` contract that
multiple lenses independently flagged, and a set of UI standards/usability
concerns around inert search UI, dot accessibility, and the cited CSS-token
vocabulary that does not exist.

### Cross-Cutting Themes

- **Reconnect / `onEvent` contract is ambiguous** (flagged by: Architecture,
  Code Quality, Test Coverage, Correctness, Compatibility) — the plan
  explicitly defers the reconnect-signal contract to the implementer
  ("`onReconnect` callback OR `{ type: 'reset' }` sentinel — implementation
  chooses"), and the `useDocEvents` re-plumbing has two parallel shapes
  ("Pick whichever keeps the test surface simplest"). The downstream tracker
  is forced to bind to whichever shape lands first; the existing
  `useEffect` deps array does not include the new options, so the closure
  will either capture stale callbacks or thrash the EventSource on every
  render.
- **Context-value identity will churn the Sidebar on every SSE event**
  (Code Quality, Performance, Correctness) — the proposed
  `useUnseenDocTypes` returns a fresh `useMemo({...})` whose deps change on
  every `setSeen`/`setUnseenSet` call; the `UnseenDocTypesContext.Provider`
  value identity flips, re-rendering all consumers (including the 12-item
  Sidebar plus Glyph SVGs) on every external `doc-changed`. The plan's
  "switch to membership-set accessor if profiling shows churn" telegraphs
  this without committing.
- **Inert UI affordances ship with no signal that they are disabled**
  (Standards, Usability) — the `<input type="search" disabled>` plus
  `<kbd>/</kbd>` chip have no placeholder, tooltip, or treatment; users
  hitting `/` get no feedback, screen-reader users hear "dimmed/unavailable".
  No precedent in the codebase for shipping disabled form controls as
  placeholders.
- **Unseen dot accessibility and semantics** (Standards, Usability) —
  `<span aria-label="Unseen changes" />` will not be announced by screen
  readers (bare `<span>` has no role); the dot also disappears on
  `LibraryTypeView` mount even when the user deep-linked into a child
  fileSlug, conflating "route mounted" with "user saw the change".
- **`count` field design choices** (Architecture, Code Quality,
  Compatibility, Standards) — adding `count: usize` directly to the static
  `DocType` description conflates static config with dynamic indexer state;
  `describe_types` returns objects with placeholder `count: 0` that the
  handler is expected to overwrite. The TS mirror is declared required
  rather than optional, breaking the plan's own "absent count → no badge"
  forward-compat narrative for any consumer that constructs `DocType`
  literally (e.g., `Sidebar.test.tsx:23-28`).
- **ISO-8601 vs epoch-ms storage format** (Code Quality, Standards,
  Compatibility) — three lenses independently flagged that storing
  `Date.now()` as ISO-8601 introduces a parse round-trip and brittleness
  against malformed strings, when storing the raw number is simpler,
  cheaper, and aligns with the existing `mtimeMs` representation on
  `IndexEntry`.
- **Malformed-storage and write-failure tolerance is thin** (Test Coverage,
  Correctness) — single "not json" case misses NaN-producing timestamp
  parses, wrong-shape JSON, unknown DocTypeKeys, and the `safeSetItem`
  throwing case that `use-font-mode.test.ts:59-69` already establishes as
  the canonical pattern.
- **Active-state prefix matching collides on overlapping keys**
  (Correctness, Test Coverage) — `startsWith('/library/${key}')` triggers
  two simultaneous active nav items for `plans` ↔ `plan-reviews`,
  `prs` ↔ `pr-reviews`, `work-items` ↔ `work-item-reviews`,
  `design-gaps` ↔ `design-inventories`. The rewrite is the moment to fix
  this inherited bug; the failing test as written would not catch it.

### Tradeoff Analysis

- **Phase decoupling vs. integration testability** — the plan presents
  Phases 2–4 as separately landable but defers all user-visible manual
  verification to Phase 5. Standards/usability bugs from 2–4 can only be
  observed once the dot renders, slowing bisection. Either commit Phases
  2–5 as a single landed change, or add at-least-one DevTools-observable
  manual check per phase.
- **Performance (re-render churn) vs. API ergonomics** — the
  `unseen: (type) => boolean` API reads cleanly at call sites but forces
  Sidebar-wide re-renders on every SSE event. Exposing
  `unseenSet: ReadonlySet<DocTypeKey>` as the primary surface (with
  `unseen(type)` as sugar) costs one extra field and removes the deferred
  refactor flagged by both Performance and Code Quality.
- **First-event-seeds-T silently vs. discoverability** — for a fresh
  install, dots will never appear until both (a) the user visits a type
  once and (b) a later event arrives. This is a defensible "since last
  visit" semantic but the plan should be explicit that the dot is not a
  "new to you" indicator — otherwise first-run discoverability is
  effectively label-driven only.

### Findings

#### Critical

- 🔴 **Correctness**: Active-state prefix match misfires for prefix-overlapping DocTypeKeys
  **Location**: Phase 5, Section 3 — Sidebar.tsx active-state computation
  `pathname.startsWith('/library/${key}')` matches both `plans` and `plan-reviews`
  (and similarly for prs/pr-reviews, work-items/work-item-reviews,
  design-gaps/design-inventories). Two nav items appear active simultaneously.
- 🔴 **Correctness**: `doc-invalid` events lack an etag, so self-cause echoes raise the dot
  **Location**: Phase 2 — SSE onEvent self-cause guard
  `SseDocInvalidEvent` has no `etag` field (types.ts:119-123); the guard at
  line 139 only filters `doc-changed`. The plan's "self-cause echoes never
  raise the dot" claim does not hold for invalid events.

#### Major

- 🟡 **Code Quality**: Production hook re-plumbing ambiguous; existing `useEffect` deps won't include the new options
  **Location**: Phase 2, Section 3 — "Plumb the production hook"
  Two parallel shapes left for the implementer ("Pick whichever keeps the
  test surface simplest"). The existing `useEffect` deps
  `[queryClient, createSource, registry]` either capture stale callbacks
  or thrash the EventSource on every render.
- 🟡 **Code Quality**: `onEvent` mutating two states invites stale-closure / double-write bugs
  **Location**: Phase 3, Section 3 — `useUnseenDocTypes` state model
  Dual `seen` + `unseenSet` state read from event-driven callbacks risks
  the same StrictMode / concurrent-rendering hazard already commented in
  `use-theme.ts:39-46`.
- 🟡 **Correctness**: Adding `onEvent`/`onReconnect` will trigger SSE re-subscribe per render
  **Location**: Phase 2, Section 3
  Without a ref-based callback pattern, either the closure goes stale or
  the EventSource closes and reconnects on every parent render — replaying
  buffered events as "new" and producing the dot-storm the plan claims to
  avoid.
- 🟡 **Correctness**: Reconnect-burst semantics contradict manual verification step 8
  **Location**: Phase 3 + Testing Strategy manual step 8
  "First-event seeds T silently" combined with "reconnect → no-op" still
  raises dots for any type with stored T < event-receipt time after a
  long disconnect — which manual step 8 calls "spurious". Disambiguate.
- 🟡 **Correctness**: Dual-state synchronisation invariants undefined for fresh-mount-with-pending-dot
  **Location**: Phase 3, Section 3
  Plan does not specify how `unseenSet` rehydrates on mount when localStorage
  contains stored T but the dot was visible at last unload. Dots silently
  disappear across tab reloads.
- 🟡 **Test Coverage**: Reconnect contract test allows either shape — defeats regression purpose
  **Location**: Phase 2 — Failing tests
  "Implementation chooses, the test pins the contract" lets the test be
  rewritten to whatever the implementation does.
- 🟡 **Test Coverage**: LibraryTypeView test invents a `propType` render path that does not exist
  **Location**: Phase 4, Section 1 — "No-op for invalid type" case
  Either won't compile or forces a test-only seam into production code.
- 🟡 **Test Coverage**: Malformed-storage coverage is a single case; misses NaN, wrong-shape, unknown keys
  **Location**: Phase 3, Section 2 — "Malformed storage tolerance"
  `Date.parse` on a malformed ISO string returns `NaN`; strict-gt against
  `NaN` is always false → dot silently suppressed forever.
- 🟡 **Test Coverage**: No test for `safeSetItem` failure (private mode / quota)
  **Location**: Phase 3, Section 2
  `use-font-mode.test.ts:59-69` establishes the canonical pattern.
- 🟡 **Test Coverage**: Active-state test omits child-doc URL — the dominant production path
  **Location**: Phase 5, Section 2 — active state case
  Users spend most of their time at `/library/<type>/<doc>`; the
  `startsWith` boundary is uncovered.
- 🟡 **Test Coverage**: No integration test verifies count badge updates after SSE invalidation
  **Location**: Phase 5 + Phase 2
  The core value proposition (live count badges) is untested end-to-end.
- 🟡 **Performance**: Context-value identity churn re-renders all consumers per SSE event
  **Location**: Phase 3 §3 + Phase 5
  Twelve nav items × Glyph SVG × N events/sec under filesystem-watcher
  storms (e.g., `jj checkout`). The deferred "switch to membership-set
  accessor" fix isn't actually free without a `React.memo` boundary
  around each Sidebar item.
- 🟡 **Standards**: Plan cites CSS tokens that do not exist (`--ac-bg-subtle`, `--ac-border`)
  **Location**: Phase 5, Section 4 — `Sidebar.module.css`
  Actual token vocabulary uses `--ac-bg-sunken/raised/hover/active`,
  `--ac-stroke/-soft/-strong`, `--ac-accent-faint`. Implementing as written
  ships CSS referencing undefined custom properties.
- 🟡 **Standards**: `aria-label` on a bare `<span>` is not announced by screen readers
  **Location**: Phase 5, Section 3 — `<span className={styles.dot} aria-label="…" />`
  Bare `<span>` has no implicit role; ARIA label has no effect. Existing
  `PipelineDots.tsx` precedent puts the label on an element with implicit
  list-item role.
- 🟡 **Standards**: Disabled search input + `/` kbd chip have no precedent and create a11y issues
  **Location**: Phase 5, Section 3
  Disabled inputs are announced as "dimmed/unavailable" and removed from
  tab order. No codebase precedent for shipping disabled placeholder UI.
- 🟡 **Usability**: Inert search input and `/` kbd chip with no affordance signalling they are non-functional
  **Location**: Phase 5, Section 3
  Users hitting `/` will see no response and infer the keybind is broken.
- 🟡 **Usability**: Bumping T on mount conflates "route mounted" with "user saw the change"
  **Location**: Phase 4 — `useMarkSeen` wired into `LibraryTypeView`
  Deep links to `/library/work-items/<slug>` silently clear the entire
  work-items dot even though the user never saw the list. Phase 4 test
  bullet 3 pins the worse behaviour.
- 🟡 **Usability**: Unseen dot's a11y treatment will not announce dynamic changes
  **Location**: Phase 5, Section 3
  No `role="status"` / `aria-live`; dot appearance is silent to AT users.
- 🟡 **Usability**: Badge `margin-left: auto` collides with dot placement — visual layout under-specified
  **Location**: Phase 5, Section 4
  Combined badge + dot rendering for `count > 0 && unseen === true` is
  not pinned visually or in tests.
- 🟡 **Compatibility**: Frontend `DocType.count` is required, contradicting forward-compat narrative
  **Location**: Phase 1, Step 6 — frontend `DocType` interface
  `count: number` (non-optional) makes `t.count > 0` a defensive check
  with no real fallback path; existing `Sidebar.test.tsx:23-28` mock
  fixture will fail `tsc` until Phase 5 lands, breaking the "Phase 1 ships
  standalone" claim.

#### Minor

- 🔵 **Architecture**: `DocType` conflates static description with dynamic index-derived count
  **Location**: Phase 1, Steps 3 & 5
  Future non-handler consumers of `describe_types` silently get
  `count: 0`. Consider a wire-shape envelope (`DocTypeWithCount`).
- 🔵 **Architecture**: `makeUseDocEvents` accreting subscription concern blurs responsibility
  **Location**: Phase 2
  Each future subscriber risks another callback option. Consider a
  `subscribe(handler): unsubscribe` API instead.
- 🔵 **Architecture**: Multi-tab semantics implicit — localStorage writes don't propagate
  **Location**: Phase 3
  Tab A's `markSeen` doesn't clear Tab B's dot.
- 🔵 **Architecture**: `PHASE_DOC_TYPES` hard-codes phase taxonomy client-side
  **Location**: Phase 5, Section 1
  Future server-side consumers must duplicate or re-derive.
- 🔵 **Code Quality**: ISO-8601 storage format adds parse cost vs. epoch-ms
  **Location**: Phase 3 — storage format
  Switch to `Date.now()` numbers; trivial strict-gt comparison, no parse.
- 🔵 **Code Quality**: `PHASE_DOC_TYPES` lookup silently swallows missing doc types
  **Location**: Phase 5, Section 3
  Add a dev-time warn or a `satisfies` exhaustiveness check.
- 🔵 **Code Quality**: "Switch `unseen` to a stable membership-set accessor" deferred but API already chosen
  **Location**: Performance Considerations
  Commit to `unseenSet: ReadonlySet<DocTypeKey>` now to avoid migration.
- 🔵 **Code Quality**: Line-number-driven insertion instructions are fragile and self-contradicting
  **Location**: Phase 4, Section 2
  Replace with intent-based prose ("after narrowing, before first return").
- 🔵 **Code Quality**: `count: 0` placeholder at struct construction is dead data
  **Location**: Phase 1, Step 3
  Either split wire vs descriptor, or doc-comment the handler-owned invariant.
- 🔵 **Test Coverage**: No StrictMode double-effect test for `useMarkSeen`
  **Location**: Phase 4
- 🔵 **Test Coverage**: No test that `unseen` reactivity propagates to subscribed components
  **Location**: Phase 3
- 🔵 **Test Coverage**: No test for SSE event with unknown docType
  **Location**: Phase 2
- 🔵 **Test Coverage**: Sidebar edge cases under-tested (empty docTypes, missing PHASE entry, non-Glyph key)
  **Location**: Phase 5, Section 2
- 🔵 **Test Coverage**: `counts_by_type` Templates assertion doesn't pin exclusion
  **Location**: Phase 1, Section 2
  `unwrap_or(0)` passes whether Templates is absent or zero; use
  `!contains_key`.
- 🔵 **Test Coverage**: Mocking Glyph couples Sidebar test to internal markup
  **Location**: Phase 5, Section 2
  Glyph is already shipped; render it for real.
- 🔵 **Correctness**: `useMarkSeen` between fileSlug routes — `[type]` dep behaviour
  **Location**: Phase 4
  Add a "navigating between two fileSlugs of same type does NOT re-fire
  markSeen" test.
- 🔵 **Correctness**: `counts_by_type` omits configured-but-empty types
  **Location**: Phase 1, Section 4
  Future callers must remember `unwrap_or(0)`. Document the contract.
- 🔵 **Performance**: "13× cloning" framing overstates the saving
  **Location**: Performance Considerations
  `/api/types` is fetched once per page load and never invalidated by
  doc-changed today; the new O(N) scan is a small cost, not a saving.
- 🔵 **Performance**: Synchronous localStorage write per SSE event with no coalescing
  **Location**: Phase 3, Section 3
  Watcher storms produce N sync writes. Restructure so `onEvent` does not
  write (only `markSeen` does).
- 🔵 **Performance**: `useMarkSeen` `useEffect` fires for every type change incl. back-nav
  **Location**: Phase 4, Section 2
  Add a short-circuit when stored T is fresh.
- 🔵 **Performance**: Sidebar `byKey = new Map(...)` and active-state startsWith run on every render
  **Location**: Phase 5, Section 3
  Extract `SidebarItem` as a memo component.
- 🔵 **Performance**: `counts_by_type` shares the `entries` RwLock with writers
  **Location**: Phase 1, Section 4
  If `/api/types` becomes refresh-triggered, contend with `rescan` /
  `refresh_one`. Consider an incremental count map.
- 🔵 **Standards**: Sidebar landmark structure changes without explicit accessible name
  **Location**: Phase 5, Section 3
  `<aside><nav>` loses the labelled landmark; pin `aria-labelledby` or
  drop the extra wrapper.
- 🔵 **Standards**: Inner hook signature conflicts with documented factory pattern
  **Location**: Phase 2, Section 3
  Match `makeUseTheme(prefersDark)` precedent: bind options at factory
  time, not at call site.
- 🔵 **Standards**: `useMarkSeen` breaks consumer-hook naming convention (`use*Context`)
  **Location**: Phase 3, Section 4
  Rename to `useMarkDocTypeSeen` or inline at `LibraryTypeView`.
- 🔵 **Standards**: `PHASE_DOC_TYPES` shape diverges from `LIFECYCLE_PIPELINE_STEPS`
  **Location**: Phase 5, Section 1
  Document the rationale for nested-array vs flat.
- 🔵 **Standards**: `getByRole('searchbox')` against a disabled input may be flaky
  **Location**: Phase 5, Section 2
  Tied to "don't ship disabled search" suggestion above.
- 🔵 **Usability**: First-time user sees no dots — discoverability is label-only
  **Location**: Phase 3
  Document that the dot is a "since last visit", not "new to you", signal.
- 🔵 **Usability**: Templates discoverability collapses with no sidebar entry point
  **Location**: Phase 5 + "What We're NOT Doing"
  Add a footer/secondary link, or call out explicitly as deprioritised.
- 🔵 **Usability**: Five phase subheadings always rendered — vertical space cost
  **Location**: Phase 5, Section 3
  0055 Activity feed will compete for space. Consider collapsibility or
  hide empty phases.
- 🔵 **Usability**: Phases 2–4 ship with no user-visible verification
  **Location**: Testing Strategy
  Add per-phase DevTools verification or land Phases 2–5 together.
- 🔵 **Compatibility**: Construction-site footnote miscounts callers
  **Location**: Phase 1, Step 3
  Only one `DocType { … }` literal in `docs.rs` (line 107). Correct the
  "two construction sites at 101,107" claim.
- 🔵 **Compatibility**: `useDocEvents` wrapping loses referential stability and DevTools name
  **Location**: Phase 2, Section 3
  Prefer binding options at factory time (matches `makeUseTheme`).

#### Suggestions

- 🔵 **Architecture**: Use `useMatch` instead of `pathname.startsWith` for active state
  **Location**: Phase 5
  Robustness improvement during the rewrite.

### Strengths

- ✅ Bottom-up phase decomposition with strict test-first ordering; each
  layer lands with its own failing test before the consumer is wired
- ✅ Faithfully follows the established owning-hook + Context split
  (`use-font-mode`, `use-theme`), including default-noop handle and
  `safeGetItem`/`safeSetItem` precedent
- ✅ Phase 1 is independently shippable; the `count` field is wire-additive
  and `t.count > 0` is the right rendering guard
- ✅ Self-cause guard placement after the existing `registry.has(event.etag)`
  check is the right architectural reuse
- ✅ `Indexer::counts_by_type` is a single read-lock O(N) aggregation with
  no cloning — correct data-structure choice
- ✅ Strict-greater-than (>) is the correct operator over >= for the
  same-millisecond `markSeen`/event collision case
- ✅ Effect dependency on `[type]` correctly accounts for TanStack Router's
  component reuse across `:type` changes (a `[]` deps would have been a
  real bug)
- ✅ Templates exclusion is consistent across every layer: indexer skips,
  `counts_by_type` returns no entry, handler folds `unwrap_or(0)`, Sidebar
  omits Templates from PHASE_DOC_TYPES
- ✅ Storage key `ac-seen-doc-types` follows the established `ac-*` namespace
  convention; bounded ≤13 entries
- ✅ Explicit `What We're NOT Doing` section names six deliberate non-goals
  with succinct justifications — strong evolutionary-fitness hygiene
- ✅ The `MakeUseDocEventsOptions` parameter and `useDocEvents(options?)` are
  positionally/structurally backward-compatible with all existing test and
  production callers

### Recommended Changes

Ordered by impact. Each maps to one or more findings above.

1. **Fix the active-state prefix collision** (addresses: Active-state
   prefix match critical, Active-state child-doc URL test major).
   Replace `pathname.startsWith('/library/${key}')` with
   `pathname === '/library/${key}' || pathname.startsWith('/library/${key}/')`.
   Add a Sidebar test case rendering at `/library/plan-reviews` asserting
   the `plans` item is NOT active, plus a `/library/work-items/<slug>`
   case asserting work-items IS active.

2. **Decide and pin the reconnect contract** (addresses: 4 lens findings).
   Commit to `onReconnect?: () => void` as a separate optional callback;
   remove the `{ type: 'reset' }` sentinel option from the plan. The
   tracker no-ops on reconnect. Update the Phase 2 test list to assert
   `onReconnect` is invoked once on reconnect, not "either shape".

3. **Fix the `useDocEvents` re-subscription / stale-closure pattern**
   (addresses: Code Quality major #1, Correctness major re-subscribe).
   Pin one shape in the plan: bind `options` at factory time (matching
   `makeUseTheme(prefersDark)`) OR use a `useRef` pattern inside
   `makeUseDocEvents` so callbacks can change between renders without
   re-creating the EventSource. Add a test that renders twice with new
   options and asserts the EventSource was constructed only once.

4. **Address `doc-invalid` self-cause coverage** (addresses: critical
   self-cause bypass).
   Decide and document: either (a) ignore `doc-invalid` events entirely
   inside `useUnseenDocTypes.onEvent`, or (b) extend `SseDocInvalidEvent`
   server-side to carry `etag` and route it through the same guard.
   Pin with an explicit test case.

5. **Make `DocType.count` optional in the frontend interface**
   (addresses: Compatibility major).
   `count?: number` keeps `t.count > 0` as a genuine narrowing check and
   honours the "Phase 1 ships standalone" claim. Existing fixtures (e.g.,
   `Sidebar.test.tsx:23-28`) keep typechecking.

6. **Correct the CSS-token vocabulary** (addresses: Standards major #1).
   Rewrite Phase 5 §4 against the actual tokens (`--ac-bg-sunken`,
   `--ac-stroke`, `--ac-accent-faint`, `--ac-accent`). If a new token is
   genuinely needed (e.g., `--ac-bg-subtle`), add it explicitly to
   `global.css` as a visible plan change with both light and dark values.

7. **Redesign the unseen-dot a11y treatment** (addresses: Standards major
   #2, Usability major a11y).
   Fold the dot signal into the link's accessible name (`aria-label="Decisions (unseen changes)"`) so it reads during normal sidebar
   traversal. Add a visually-hidden `<span className="sr-only">unseen changes</span>` for redundancy. Specify reading order vs. the badge.

8. **Remove the disabled search input + kbd chip from this story**
   (addresses: Standards major #3, Usability major inert UI).
   Render only a sized empty container (or nothing) until 0054 wires the
   behaviour. Update Phase 5 tests to not assert the searchbox role.

9. **Tighten `useMarkSeen` semantics** (addresses: Usability major
   route-mount conflation).
   Only bump T on the list view (no `fileSlug`), so deep links to a child
   doc do not silently clear the parent type's dot. Update Phase 4 test
   bullet 3 to assert the opposite ("Child-doc URL: markSeen NOT called").

10. **Specify the dot/badge combined layout and add a test**
    (addresses: Usability major #4).
    Pin the visual order (dot adjacent to label vs. right-edge) in
    Phase 5 §4, add a test case for `count > 0 && unseen === true`.

11. **Switch the unseen tracker's API surface to `unseenSet`**
    (addresses: Performance major, Code Quality deferred refactor).
    Expose `unseenSet: ReadonlySet<DocTypeKey>` as the primary surface
    with `unseen(type)` as sugar. Extract `SidebarItem` into a
    `React.memo`-wrapped subcomponent reading the bool via a selector;
    this gives the membership-set the memo boundary it needs.

12. **Replace ISO-8601 storage with epoch milliseconds**
    (addresses: Code Quality minor, Standards minor, Compatibility minor).
    Store numeric `Date.now()` values in the JSON object. Strict-gt is
    direct on numbers; malformed-input tolerance becomes typeof-checked.

13. **Restructure storage writes: `onEvent` does not write, only `markSeen`**
    (addresses: Performance minor write amplification, Code Quality
    `safeSetItem` failure test).
    Keep an in-memory unseen `Set` populated by `onEvent`; only `markSeen`
    persists the T value. Add a `safeSetItem` throwing test mirroring
    `use-font-mode.test.ts:59-69`.

14. **Tighten Phase 3 test coverage of malformed storage** (addresses:
    Test Coverage major #3, Correctness implicit NaN bug).
    Add cases for: (a) JSON array instead of object, (b) malformed ISO
    string causing `NaN`, (c) unknown DocTypeKey (filtered out), (d)
    mixed valid + invalid entries.

15. **Define unseen-set rehydration on mount** (addresses: Correctness
    major dual-state).
    Specify how the dot's visible state survives across reloads: either
    accept that dots clear on reload (document it) or persist the
    membership set alongside seen-times.

16. **Disambiguate reconnect-burst behaviour vs. "spurious"**
    (addresses: Correctness major reconnect bursts).
    Either rewrite manual verification step 8 to say "real changes during
    disconnect produce dots" (reconnect-replayed events count as real
    activity), or specify a different burst-suppression mechanism.

17. **Reframe the "13× cloning" Performance Considerations bullet**
    (addresses: Performance minor framing).
    State the actual baseline: `/api/types` is fetched once per page load
    today. The new O(N) scan is a small cost, not a saving. Separately
    decide whether `dispatchSseEvent` should invalidate `queryKeys.types()`
    so badges stay live.

18. **Fix the LibraryTypeView test fixture** (addresses: Test Coverage
    major #2).
    Drive the invalid-type case through the router (render at
    `/library/not-a-real-type`) instead of inventing a `propType` prop
    that doesn't exist in production.

19. **Add the omitted test cases** (addresses: Test Coverage majors
    #5, #6 + minors).
    - Active state at `/library/<type>/<slug>` (Sidebar)
    - Count badge updates after SSE invalidation (integration)
    - `safeSetItem` throws (Phase 3)
    - StrictMode double-effect on `useMarkSeen` (Phase 4)
    - Unseen reactivity propagating through context (Phase 3)
    - Empty `docTypes`, missing-PHASE entry, non-Glyph key (Phase 5)
    - `counts_by_type` Templates exclusion via `!contains_key`

20. **Smaller correctness/clarity fixes** (addresses: scattered minors).
    - Correct the "two construction sites at 101,107" footnote — there
      is one, at line 107
    - Replace line-number insertion prose in Phase 4 with intent-based
      prose ("after narrowing, before first conditional return")
    - Document multi-tab semantics in "What We're NOT Doing"
    - Rename `useMarkSeen` to `useMarkDocTypeSeen` (or inline it) for
      naming-convention symmetry
    - Add `aria-labelledby` to the inner `<nav>` referencing the LIBRARY
      heading, or drop the `<aside>` wrapper

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends well-established frontend patterns (owning-hook
+ context, factory test seam, safe-storage) consistently and decomposes the
work into clean bottom-up phases with appropriate test seams. The main
architectural concerns are (a) overloading the static `DocType` description
struct with dynamic indexer-derived state, (b) accreting a second concern
(consumer subscription) into `makeUseDocEvents` alongside its query-
invalidation responsibility, and (c) a couple of soft edges around the
multi-tab and first-event semantics of the unseen tracker. None are blocking
— they are evolutionary-fitness considerations worth either acknowledging
in the plan or refactoring early.

**Strengths**: Bottom-up phase decomposition; owning-hook + context pattern
reuse; explicit non-goals; self-cause guard reuse; wire-additive count
rollout; localised localStorage state; consistent Templates exclusion.

**Findings**:
- `DocType` conflates static description with dynamic count (minor)
- `makeUseDocEvents` accreting subscription concern (minor)
- Reconnect-reset contract ambiguity (minor)
- Multi-tab + first-event semantics implicit (minor)
- `PHASE_DOC_TYPES` taxonomy hard-coded client-side (minor)
- `startsWith` active-state fragility (suggestion)

### Code Quality

**Summary**: Test-first, respects established patterns, complexity
proportional to requirements. Concerns: `makeUseDocEvents` re-plumbing
introduces an ambiguous `options` arg; `useUnseenDocTypes` mixes setState in
event-driven callbacks; several body details deferred to the implementer
without concrete shape.

**Findings**: production-hook re-plumbing ambiguous (major); onEvent state
mutation invites stale-closure/double-write (major); ISO-8601 vs epoch-ms
(minor); reconnect contract left to implementer (minor); silent
`PHASE_DOC_TYPES` lookup miss (minor); deferred `unseenSet` accessor (minor);
line-number-driven insertion prose (minor); `count: 0` dead data at
construction (minor).

### Test Coverage

**Summary**: Genuinely test-first and covers headline behaviours, but
several risk-bearing edges are under-tested: reconnect contract in Phase 2
left to implementation, malformed-storage and write-failure thin in Phase 3,
LibraryTypeView test invents a prop-render path that doesn't exist,
Sidebar integration omits child-doc active state and end-to-end count
freshness.

**Findings**: reconnect contract unpinned (major); LibraryTypeView prop-
render path doesn't exist (major); malformed-storage single case (major);
no safeSetItem failure test (major); active-state omits child-doc URL
(major); no integration test for count freshness on SSE (major); no
StrictMode double-effect test (minor); no reactivity-propagates-via-context
test (minor); no test for unknown SSE docType (minor); Sidebar edge cases
under-tested (minor); Templates exclusion not pinned (minor); Glyph mock
couples to internal markup (minor).

### Correctness

**Summary**: Overall reasoning is sound — bottom-up phasing, strict-gt
comparison, `[type]`-dependent effect, and self-cause placement are
correct. However, three concrete bugs would survive into production:
(1) active-state `startsWith` misfires for prefix-overlapping keys, (2)
`SseDocInvalidEvent` has no `etag` so locally-caused invalid events bypass
the self-cause guard, and (3) `onEvent`/`onReconnect` options will trigger
SSE re-subscribe on every render unless `useEffect` deps are reworked.

**Findings**: active-state prefix misfires (critical); doc-invalid self-
cause bypass (critical); re-subscribe on every render (major); reconnect
burst contradicts manual step 8 (major); dual-state synchronisation
undefined (major); useMarkSeen `[type]` boundary cases (minor);
`counts_by_type` configured-but-empty types omitted (minor).

### Performance

**Summary**: Reasonable algorithmic primitives but mischaracterises the
`all_by_type` saving and underplays re-render and write amplification from
Context-driven SSE ingestion. Most material risks: (1) UnseenDocTypesContext
re-renders all consumers per event, (2) synchronous localStorage write per
event with no coalescing, (3) `/api/types` count recomputed under read-lock
on every call.

**Findings**: "13× cloning" framing overstates saving (minor); context-value
identity churn (major); synchronous localStorage write per event (minor);
useMarkSeen useEffect fires for every type change (minor); `byKey` Map +
startsWith on every render (minor); `counts_by_type` shares RwLock with
writers (minor).

### Standards

**Summary**: Largely follows project conventions (folder-per-component,
camelCase JSON, `ac-` storage prefix, owning-hook + consumer-hook split,
UPPER_SNAKE constants), but several Phase 5 details cite CSS tokens that
do not exist, and `aria-label` on a bare `<span>` is not a semantically
meaningful accessible element.

**Findings**: cites non-existent CSS tokens (major); `aria-label` on bare
`<span>` not announced (major); disabled search input has no precedent
(major); `<aside><nav>` landmark loses accessible name (minor); inner hook
signature breaks factory-binding convention (minor); ISO-8601 storage
diverges from sibling hooks' narrow types (minor); `count: 0` placeholder
breaks struct-as-intrinsic convention (minor); `useMarkSeen` breaks
consumer-hook naming (minor); `PHASE_DOC_TYPES` shape inconsistent with
`LIFECYCLE_PIPELINE_STEPS` (minor); `getByRole('searchbox')` against
disabled input may be flaky (minor).

### Usability

**Summary**: Strong IA (LIBRARY → five phases → twelve doc types with
Glyphs, counts, dots) and follows established patterns, but ships
deliberately-inert UI (search input, `/` kbd chip) without affordances
reflecting their state, and has subtle UX issues around what "seen" means
and how the unseen dot is announced to AT. The mount-bumping semantics
conflate route mount with user awareness; the dot/badge layout interaction
is under-specified.

**Findings**: inert search + kbd chip without affordance (major); T-bump
conflates mount with user awareness (major); dot a11y won't announce
dynamic changes (major); badge `margin-left: auto` collides with dot
(major); first-time user sees no dots (minor); Templates discoverability
collapses (minor); five phase subheadings always rendered (minor); Phases
2–4 ship without per-phase manual verification (minor).

### Compatibility

**Summary**: Largely sound: `count: usize` is wire-additive to a small
internal API with no schema validation; new optional callback args are
positionally compatible. Main risk is a self-inflicted TS break:
`DocType.count` is declared required, contradicting the plan's own
"absent count → no badge" forward-compat narrative.

**Findings**: frontend `DocType.count` required (major); construction-site
footnote miscounts callers (minor); `useDocEvents` wrapping loses
referential stability and DevTools name (minor); ISO-8601 storage creates
subtle migration burden (minor).

---

## Re-Review (Pass 2) — 2026-05-12

**Verdict:** COMMENT (down from REVISE)

The plan has been substantially iterated to address pass-1 findings. Both
critical correctness bugs are fixed and pinned by regression tests; 18 of
20 majors fully resolved, 2 partially resolved with documented trade-offs.
The remaining concerns are minor refinements that don't change the design
and can land alongside their respective phases.

### Previously Identified Issues

#### Critical
- ✅ **Correctness**: Active-state prefix match misfires — **Resolved**.
  `pathname === X || pathname.startsWith(X + '/')` with regression test
  for `/library/plan-reviews` not activating Plans.
- ✅ **Correctness**: `doc-invalid` events bypass self-cause guard —
  **Resolved**. Tracker ignores `doc-invalid` entirely with dedicated test.

#### Major
- ✅ **Code Quality**: Production hook re-plumbing ambiguous —
  **Resolved** via factory-unchanged + inner-hook `options?` + `useRef`
  callback storage + explicit dep-array-not-extended note.
- ✅ **Code Quality**: `onEvent` mutating two states risks stale-closure
  — **Resolved** via `seenRef` (persisted T) + `unseenSet` (transient
  membership) split with full implementation skeleton.
- ✅ **Test Coverage**: Reconnect contract unpinned — **Resolved**;
  asserts `onReconnect` called exactly once AND `onEvent` NOT called with
  sentinel.
- ✅ **Test Coverage**: LibraryTypeView `propType` invented prop —
  **Resolved**; tests drive invalid case through real router.
- ✅ **Test Coverage**: Malformed-storage single case — **Resolved**;
  four shapes (not-json, array, NaN, unknown key).
- ✅ **Test Coverage**: No `safeSetItem` failure test — **Resolved**.
- ✅ **Test Coverage**: Active-state child-doc URL — **Resolved**;
  explicit cases for `/library/<type>/<slug>` and four prefix-collision
  pairs.
- 🟡 **Test Coverage**: No integration test for count freshness —
  **Partially resolved**. Phase 1 §7 adds unit test for the invalidation
  call; no end-to-end Sidebar+QueryClient test verifies the badge
  actually transitions.
- ✅ **Correctness**: `onEvent`/`onReconnect` re-subscribes per render —
  **Resolved** via `useRef` deps-unchanged pattern + callback-stability
  test.
- ✅ **Correctness**: Reconnect-burst contradicted manual step 8 —
  **Resolved**; step rewritten to clarify "real changes during disconnect
  produce dots via replayed events".
- ✅ **Correctness**: Dual-state rehydration on mount undefined —
  **Resolved** via the `seenRef`/`unseenSet` split with the trade-off
  ("transient state does not survive reload") documented in NOT-Doing.
- ✅ **Performance**: Context-value identity churn re-renders Sidebar —
  **Resolved** via `unseenSet` `ReadonlySet` with stable-identity contract
  + tested early-return reducer.
- ✅ **Standards**: Cites non-existent CSS tokens — **Resolved**; every
  token verified against `global.css`.
- ✅ **Standards**: `aria-label` on bare `<span>` — **Resolved**; signal
  folded into link `aria-label`, dot becomes `aria-hidden="true"`.
- ✅ **Standards**: Disabled search input has no precedent — **Resolved**
  via `hidden` attribute (per user direction), removing the element from
  the a11y tree and tab order without "coming soon" affordance confusion.
- ✅ **Usability**: Inert search + kbd no affordance — **Resolved** by
  the same `hidden` mechanism.
- ✅ **Usability**: T-bump conflates mount with user awareness —
  **Resolved**; `useMarkDocTypeSeen(hasFileSlug ? undefined : type)` only
  fires on list view, deep links don't clear the parent dot.
- 🟡 **Usability**: Unseen dot dynamic announcement —
  **Partially resolved**. Static `aria-label` channel resolved (announces
  during traversal); no `aria-live` region for dynamic announcements when
  a dot raises while the user is reading elsewhere.
- ✅ **Usability**: Badge `margin-left: auto` collides with dot —
  **Resolved**; dot-before-badge DOM order pinned, combined-render test
  added.
- ✅ **Compatibility**: Frontend `DocType.count` required — **Resolved**
  via `count?: number` (optional).

### New Issues Introduced

Only **1 major** and a handful of minors — well below the REVISE threshold
of 3 majors. The 1 major is a deferred integration test that can be added
during implementation.

#### Major
- 🟡 **Test Coverage**: Count-freshness verified only at the invalidation-
  call level — no test mounts Sidebar + real `QueryClientProvider` and
  asserts a badge transitions `1 → 2` after an SSE event. A regression
  where `/api/types` is invalidated but the new count doesn't reach the
  rendered badge would pass both layers' tests.

#### Minor

- 🔵 **Code Quality**: `useMarkDocTypeSeen(hasFileSlug ? undefined : type)`
  reads as a magical ternary-as-enabled-flag at the call site. Consider
  an explicit `useMarkDocTypeSeen(type, { enabled: !hasFileSlug })`, or
  add a one-line comment.
- 🔵 **Code Quality**: `unseen(type)` callback on the handle duplicates
  `unseenSet.has(type)` — the Sidebar skeleton already uses the set
  directly. Could drop the helper to reduce API surface.
- 🔵 **Code Quality**: Final `useMemo` wrapping already-`useCallback`'d
  members is redundant given the only state-derived field is `unseenSet`.
- 🔵 **Code Quality**: `if (!t) return null` silently skips a missing
  `PHASE_DOC_TYPES` key. Add a dev-only `console.warn` so a config drift
  is visible during development.
- 🔵 **Test Coverage**: StrictMode test asserts call args but not call
  count — could pass with a wrong dep tuple. Tighten to "exactly twice
  under StrictMode".
- 🔵 **Test Coverage**: No automated test composes
  `markSeen` → `onReconnect()` → replayed `doc-changed` as a sequence to
  prove `onReconnect` does NOT reset `seenRef`.
- 🔵 **Test Coverage**: `unseenSet` identity-stability test should
  explicitly trigger via a second `doc-changed` for the same type (the
  early-return path), not a bare double-rerender.
- 🔵 **Test Coverage**: Unknown-`docType` test should assert call count
  exactly once.
- 🔵 **Test Coverage**: Persistence round-trip should add a direct
  `JSON.parse(localStorage)` assertion alongside the indirect strict-gt
  inference.
- 🔵 **Correctness**: Plan does not specify WHERE in the existing
  reconnect flow `onReconnectRef.current?.()` fires (should be AFTER
  registry reset + invalidation drain).
- 🔵 **Correctness**: `seenRef` synchronous-visibility-after-mutate not
  directly tested — add a `markSeen` → synchronous `onEvent` sequence to
  pin the contract.
- 🔵 **Performance**: Performance Considerations should explicitly note
  React Query's in-flight dedupe of `/api/types` invalidations under
  watcher storms (1 actual request per burst, not N).
- 🔵 **Performance**: `setUnseenSet` is invoked per event even when
  membership doesn't change — the reducer's `if (prev.has(type)) return
  prev` short-circuits, but React still schedules and runs the reducer.
  Could gate with an outer `unseenSet.has(type)` ref-read.
- 🔵 **Standards**: `hidden` attribute has no precedent in this frontend
  — worth a JSX comment so 0054 knows the unlock mechanism (the existing
  `{/* ... */}` block partially does this).
- 🔵 **Standards**: Factory-vs-inner-hook options pattern diverges from
  `useTheme`. Add a one-line code comment justifying the split (factory
  binds infrastructure + test seams, inner accepts per-render config).
- 🔵 **Standards**: `PHASE_DOC_TYPES` nested shape differs from
  `LIFECYCLE_PIPELINE_STEPS` flat shape — add a doc-comment explaining
  the one-to-many vs one-to-one distinction.
- 🔵 **Standards**: `DocType::count` doc-comment should lead with the
  wire guarantee ("on the JSON wire, count reflects indexed cardinality")
  rather than the in-process default.
- 🔵 **Usability**: Sighted mouse users get no hover tooltip — consider
  mirroring the link `aria-label` as a `title` attribute when
  `hasUnseen`.

#### Suggestions

- 🔵 **Architecture**: `DocType.count` conflation is partially mitigated
  by the doc-comment; a true split would be over-engineering at N=1
  consumer. Accept until a second consumer appears.
- 🔵 **Performance**: Defer `SidebarItem` `React.memo` extraction until
  profiling shows >2ms reconciliation per render under sustained event
  load.
- 🔵 **Performance**: Add a `cargo bench` for `/api/types` p99 latency
  under a synthetic watcher-storm fixture to give the incremental-counts
  trigger a concrete threshold.

### Assessment

The plan is in solid shape and ready for implementation. Both critical
correctness bugs are fixed and pinned by regression tests. The structural
concerns from pass 1 (re-subscribe pattern, dual-state model, reconnect
contract, a11y channel, token vocabulary, search input affordance) are
resolved with concrete implementation skeletons and accompanying tests.

### Follow-up edits (post pass-2)

After this re-review, the remaining minor items were folded into the
plan in a follow-up pass:

- **End-to-end count-freshness integration test** added to Phase 5 §2:
  mounts `<Sidebar>` inside a real `<QueryClientProvider>` with a
  controllable types-query mock and asserts the badge transitions
  `1 → 2` after invalidation.
- **Test assertions tightened**: StrictMode case asserts exactly two
  `markSeen` calls; identity-stability case triggers via a repeat
  `doc-changed`; persistence round-trip adds a direct
  `JSON.parse(localStorage)` assertion; unknown-`docType` case pins
  call count to exactly one; new `markSeen` → synchronous `onEvent`
  case pins ref-visibility; new reconnect → replayed-event sequence
  case pins that `onReconnect` does not reset `seenRef`.
- **`onReconnect` ordering pinned** in Phase 2: fires at the END of
  the existing reconnect block, after `registry.reset()` and the
  invalidation drain.
- **Performance Considerations** note React Query's in-flight dedupe
  of `/api/types` invalidations and add a concrete trigger threshold
  (~5ms p99) for the incremental-counts fallback.
- **Doc-comments** added: factory-vs-inner-hook split in
  `makeUseDocEvents`; `PHASE_DOC_TYPES` nested shape rationale;
  `DocType::count` doc-comment leads with the wire guarantee; JSX
  comment for the `hidden` search row links to the NOT-Doing entry.
- **API surface trimmed**: dropped the redundant `unseen(type)`
  helper from the handle (Sidebar reads `unseenSet.has(key)`
  directly); dropped the redundant `useMemo` wrapping already-stable
  callbacks; explicit `typeToMarkSeen` local replaces the
  inline-ternary call in Phase 4.
- **Dev-only `console.warn`** for missing `PHASE_DOC_TYPES` keys,
  pinned with a test asserting the warn fires.
- **Sighted mouse-user affordance** added: link `title` attribute
  mirrors `aria-label` when the type has unseen changes; pinned with
  a Sidebar test case.

The plan is now ready for implementation as-is.
