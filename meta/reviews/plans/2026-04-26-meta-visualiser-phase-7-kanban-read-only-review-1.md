---
date: "2026-04-26T12:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-26-meta-visualiser-phase-7-kanban-read-only.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, usability, standards]
review_pass: 2
status: complete
---

## Plan Review: Meta visualiser Phase 7 — Kanban read-only

**Verdict:** REVISE

The plan is meticulous, TDD-disciplined, and architecturally sound at the macro level — reading from the existing `queryKeys.docs('tickets')` cache, deferring the write path, and pinning Phase 8 as a flag-flip rather than a re-architecture. Lower-level execution details, however, contain two critical bugs that will block the TDD step gates as written (TanStack `<Link>` rendered outside router context in TicketCard/KanbanColumn tests; production `fetchDocs` throws plain `Error` so the typed `FetchError` branch is dead code) and a cluster of cross-cutting concerns around the `useSortable({ disabled: true })` strategy whose ARIA, keyboard, and test-coverage implications are misjudged. CSS tokens that don't exist in the project, a heading-level skip on the page outline, and the divergent Link `to` form round out the issues that need fixing before implementation begins.

### Cross-Cutting Themes

- **`fetchDocs` / `FetchError` mismatch** (flagged by: architecture, code-quality, correctness) — the plan's KanbanBoard error branching and Step 8a typed-error test assume `fetchDocs` throws `FetchError`, but it throws plain `Error`. The 500/404-specific copy is unreachable in production; the test passes only because the mock fakes a contract the real helper does not honour. Plan needs an explicit step to migrate `fetchDocs` to `FetchError` or drop the typed branches.
- **dnd-kit `disabled: true` strategy is over-loaded** (flagged by: test-coverage, correctness, usability) — three independent concerns share this root: (a) the planned ARIA test does not actually verify `disabled` (mutation-equivalent under `disabled: false`), (b) dnd-kit may not emit `role="button"` / `aria-roledescription` when disabled, contradicting the plan's stated rationale, and (c) if it does emit them, the misleading "sortable" affordance is a WCAG 4.1.2 (Name/Role/Value) concern and produces double-tab stops alongside the inner `<Link>`. The plan needs to either omit dnd-kit from Phase 7 entirely or redesign the wiring (Link as the sortable node, suppressed misleading ARIA while disabled).
- **Link `to` form diverges from the typed-route pattern** (flagged by: architecture, standards) — `<Link to="/library/tickets/$fileSlug" params={{ fileSlug }}>` is not how any other view in the codebase links to a doc. The canonical form is `to="/library/$type/$fileSlug" params={{ type, fileSlug }}`. The literal-segment-then-param form likely won't typecheck.
- **Test harness inconsistency** (flagged by: correctness, standards) — the `renderInBoard` / `renderColumn` helpers are defined but unused; tests use bare `render(<DndContext>...)` with no `RouterProvider`, so `<Link>` rendering will throw or assert against unresolved hrefs.

### Tradeoff Analysis

- **Architecture vs Usability** — pre-mounting the DndContext + sensors in Phase 7: Architecture sees this as a clean Phase 8 seam (no provider re-threading); Usability sees it as misleading affordance + potential keyboard/event-eating side effects from sensors that have nothing to drag. Recommendation: keep DndContext, drop the sensors and `useSortable` wiring entirely from Phase 7 (Phase 8 adds them in one step). This preserves the structural seam and removes the misleading ARIA exposure.

### Findings

#### Critical
- 🔴 **Correctness**: TicketCard / KanbanColumn tests render TanStack `<Link>` outside any router context
  **Location**: Step 6 (TicketCard tests, lines 760-855) and Step 7 (KanbanColumn tests, lines 1018-1068)
  Every `it` block calls `render(<DndContext>...<TicketCard/></DndContext>)` directly with no `RouterProvider`. The `renderInBoard`/`renderColumn` helpers are defined but unused; even if used, `RouterProvider` does not render arbitrary `children` (the `defaultComponent` prop is the fallback for routes without a component, so the matched `/kanban` route renders `KanbanStub`/`KanbanBoard` rather than the children). Every test in Steps 6 and 7 will fail with router-context errors, blocking the TDD step gates.

- 🔴 **Correctness / Architecture / Code Quality**: `fetchDocs` throws plain `Error`, not `FetchError`, so the typed-error branch is unreachable in production
  **Location**: Step 8 KanbanBoard (`errorMessageFor` and 8a 500-test); Current state line 86-87
  The plan claims the kanban error branch must use `FetchError` "consistent with Phase 6", but only the lifecycle helpers were upgraded; `fetch.ts:23-28` still throws plain `Error`. The 500 / 404 branches are unreachable, the test passes via mock fiction, and a future maintainer will be misled about runtime behaviour.

#### Major
- 🟡 **Architecture / Standards**: Link target `/library/tickets/$fileSlug` diverges from the established typed-route pattern
  **Location**: Step 6: TicketCard implementation (Link target)
  The router defines `/library/$type/$fileSlug`. Every other consumer (`LibraryTypeView.tsx:107`, `LifecycleClusterView.tsx:116`) uses `to="/library/$type/$fileSlug" params={{ type, fileSlug }}`. The literal-segment-then-param form likely fails `tsc -b` and breaks Step 6's success criterion.

- 🟡 **Test Coverage**: SSE-driven cache invalidation never tested at the kanban layer
  **Location**: Overview / Desired end state ("falls out for free"); Step 8 KanbanBoard tests
  The phase's user-facing promise is that external `status:` edits land within ~250 ms, but no KanbanBoard test fires an SSE event and verifies a card moves columns. A regression where KanbanBoard reads from a different key (e.g. `queryKeys.kanban()`, which is invalidated but never populated) would not be caught.

- 🟡 **Test Coverage**: ARIA assertion does not actually verify `disabled: true`
  **Location**: Step 6 TicketCard test "exposes dnd-kit accessibility attributes"
  dnd-kit emits `role="button"` and `aria-roledescription="sortable"` regardless of `disabled`. Mutation: changing `disabled: true` to `disabled: false` causes zero test failures. The phase's central invariant — drag is a no-op — is unpinned.

- 🟡 **Correctness**: When `useSortable` is `disabled: true`, dnd-kit may not emit `role="button"` / `aria-roledescription`
  **Location**: Step 6 TicketCard test at lines 834-853
  The plan's commentary (`disabled: true keeps the dnd-kit ARIA wiring`) is plausibly wrong — `disabled` is meant to make the item inert. Verify experimentally before committing the test; if the attributes are suppressed, the rationale for using `disabled: true` over conditional rendering collapses.

- 🟡 **Correctness**: `formatMtime` regex assertion is fragile and matches by coincidence at the 60s boundary
  **Location**: Step 6 TicketCard test line 770
  `mtimeMs = Date.now() - 60_000` may land at 59/60/61s depending on jitter; `\ds ago` is a substring match and accidentally matches the suffix of `59s ago`. Inject deterministic `now` and assert the exact rendered text.

- 🟡 **Usability**: dnd-kit `role="button"` on `<li>` wraps a `<Link>`, conflicting roles and ambiguous keyboard semantics
  **Location**: Step 6 TicketCard component
  Spreading `attributes`/`listeners` onto an `<li>` that contains a `<Link>` produces two focus stops with the same accessible name, ambiguous Enter/Space activation, and double announcements ("sortable button" then "link"). Either make the `<Link>` itself the sortable node, or omit the dnd-kit wiring in Phase 7.

- 🟡 **Usability**: `aria-roledescription="sortable"` is exposed even though no sorting is possible
  **Location**: Step 6 / Step 8 — `useSortable({ disabled: true })`
  Screen-reader users will be told each card is sortable and given drag instructions that do nothing. WCAG 4.1.2 (Name/Role/Value) regression — role-description does not match actual behaviour.

- 🟡 **Usability**: Error messages have no recovery affordance
  **Location**: Step 8 KanbanBoard `errorMessageFor` and 8a tests
  The alert tells users "Try again in a moment" but provides no in-UI way to retry without a full page reload. Add a retry button that invalidates `queryKeys.docs('tickets')` or calls `refetch()`.

- 🟡 **Standards**: CSS variables used in stylesheets do not exist in the project
  **Location**: Step 6 (TicketCard.module.css), Step 7 (KanbanColumn.module.css), Step 8 (KanbanBoard.module.css)
  `var(--border)`, `var(--surface)`, `var(--surface-hover)`, `var(--muted)`, `var(--surface-muted)`, `var(--surface-warn)`, `var(--border-warn)` are referenced but every existing CSS module uses literal hex values. The "fall back to literal palette" parenthetical is misleading; implementer must either silently introduce a new token system or rewrite the snippets.

- 🟡 **Standards**: Page outline skips heading levels — no `<h1>`/`<h2>` on the kanban page, jumps to `<h3>`
  **Location**: Step 7 (KanbanColumn) and Step 8 (KanbanBoard)
  Sidebar `<h2>` → column `<h3>` skips levels with no page-level heading. WCAG 1.3.1 / 2.4.6 regression. Add a board-level `<h1>` or `<h2>` consistent with `LibraryDocView`/`LifecycleClusterView`.

#### Minor
- 🔵 **Architecture**: Kanban-specific TicketCard placed in shared `components/` directory
  **Location**: Step 6
  TicketCard embeds dnd-kit `useSortable` (kanban-only) but lives under `components/` while `KanbanColumn` lives under `routes/kanban/`. Move TicketCard alongside its consumers to match `LifecycleClusterView`'s cohesion.

- 🔵 **Architecture**: `queryKeys.kanban()` is invalidated but has no consumer — dead architectural surface
  **Location**: Current state — `queryKeys.kanban()` and `use-doc-events.ts:26-28`
  Every ticket save invalidates a key with no reader. Either remove the invalidation until Phase 8, or comment both call sites explaining the reservation.

- 🔵 **Architecture**: Adding three ticket fixtures to `seeded_cfg` increases blast radius across all integration tests
  **Location**: Step 1b
  `seeded_cfg` is shared by every integration test. Audit count-/set-based assertions or introduce `seeded_cfg_with_tickets` to keep the new fixtures isolated.

- 🔵 **Code Quality**: `groupTicketsByStatus` mutates input ordering of grouped arrays in-place (push then sort)
  **Location**: Step 5b
  Initialise the map for known keys + `OTHER_COLUMN_KEY` up-front, drop the redundant `groups.set` inside the loop, or use a one-pass functional decomposition.

- 🔵 **Code Quality**: Applying `transform`/`transition` style on a permanently-disabled sortable is dead presentation logic
  **Location**: Step 6b TicketCard
  Either drop the inline `style` until Phase 8 or add a comment noting it's pre-wired.

- 🔵 **Code Quality**: Frontmatter narrowing is duplicated inline; would benefit from a tiny helper
  **Location**: Step 6b TicketCard
  Add `frontmatterString(entry, key): string | null` to the api utilities and reuse from TicketCard, `groupTicketsByStatus`, and `LifecycleClusterView.EntryCard`.

- 🔵 **Test Coverage**: New integration test largely duplicates existing `parsed` branch coverage
  **Location**: Step 1
  Reframe as "ticket fixtures wire into seeded_cfg" or assert something the existing tests don't (e.g., `frontmatterState === 'parsed'`).

- 🔵 **Test Coverage**: No coverage of mtime sort tie-breaking
  **Location**: Step 5
  Add a case where two entries share `mtimeMs` and assert deterministic order; either implement a stable secondary sort on `relPath` or document insertion-order tie-break explicitly.

- 🔵 **Test Coverage**: Other-key omission contract is asserted ambiguously
  **Location**: Step 5 "returns empty arrays for known columns"
  Pin `groups.has(OTHER_COLUMN_KEY) === false` (or `=== undefined`) for empty input so the contract is explicit.

- 🔵 **Test Coverage**: Four-digit zero-padding is not pinned
  **Location**: Step 6 TicketCard test
  Use a small ticket number (e.g. `0001-`) and assert rendered text is `#0001`. Removing `padStart(4, '0')` currently passes all tests.

- 🔵 **Test Coverage**: Only one HTTP error class is tested; 4xx (404) branch unverified
  **Location**: Step 8 KanbanBoard 500 error test
  Either drop the 404 special case or add a test pinning the "No tickets found." copy.

- 🔵 **Test Coverage**: Heavy use of full router for component tests — over-mocking risk and slow setup
  **Location**: Steps 6-8 component tests
  Extract a `renderWithLinkRoute` helper that mounts only the leaf route, or assert href against a synchronous `<Link>` resolution without mounting the full tree.

- 🔵 **Test Coverage**: `fetchTemplates` / `fetchTemplateDetail` not stubbed in KanbanBoard tests
  **Location**: Step 8 `beforeEach`
  Mirror the `router.test.tsx` stub set or extract a shared `setupRootLayoutMocks()` helper.

- 🔵 **Correctness**: Sort comparator does not stably order entries with equal `mtimeMs`
  **Location**: Step 5
  JS sort is stable in modern V8 but the invariant is implicit. Add a tie-breaker on `relPath` ascending or document the assumption.

- 🔵 **Correctness**: Card with no numeric prefix still links via `fileSlugFromRelPath`, but server slug is `None` for such files
  **Location**: Step 6 test "renders without a ticket number when the relPath has no numeric prefix"
  This branch cannot occur for tickets created via `ticket-next-number.sh`. Remove the test or comment that it's a defensive guard against malformed external authoring.

- 🔵 **Correctness**: Generic-error test regex over-matches the FetchError branch's copy
  **Location**: Step 8 KanbanBoard error tests
  `/something went wrong|unable to load|error/i` matches the 500-branch copy too — the two tests don't distinguish their branches. Tighten regexes.

- 🔵 **Correctness**: New ticket fixtures expand the lifecycle cluster set, potentially perturbing existing assertions
  **Location**: Step 1b
  Verify with `rg 'clusters.*len\(\)' server/tests/` that no count-based assertion exists today.

- 🔵 **Correctness**: `PointerSensor` and `KeyboardSensor` remain wired despite all sortables being disabled
  **Location**: Step 8 `KanbanBoard.tsx` `useSensors`
  Sensors attach global listeners that may pre-empt native interaction. Either gate sensor registration or comment the eager registration.

- 🔵 **Usability**: Loading state is bare text with no `aria-live`, no skeleton, no layout reservation
  **Location**: Step 8 KanbanBoard
  Render the three column shells with skeletons (preserves layout) or wrap loading in a `role="status"` / `aria-live="polite"` region.

- 🔵 **Usability**: Pluralisation handles 1 vs N but produces "0 tickets in Done" for empty columns
  **Location**: Step 7 KanbanColumn
  Hide the count chip when 0, or mark the empty-state paragraph `aria-hidden`.

- 🔵 **Usability**: Empty-column copy is ambiguous between "no data" and "genuinely empty status"
  **Location**: Step 7
  Render a board-level zero-state when total entries is 0; keep per-column "Nothing in {label} right now" only when the board has data.

- 🔵 **Usability**: "Other" swimlane label does not explain why a ticket landed there
  **Location**: Step 3 / Step 8
  Rename to "Unrecognised status" or render the offending status value as a chip on each Other-swimlane card.

- 🔵 **Usability**: Ticket number absence is silent; no fallback or explanation
  **Location**: Step 6 TicketCard
  When number is missing, render a subdued chip with the file slug or relPath leaf so the card retains a visible identifier.

- 🔵 **Standards**: No CHANGELOG entry planned for a user-visible feature
  **Location**: Implementation sequence / Full success criteria
  Append an `### Added` bullet under `## Unreleased` describing the read-only kanban view.

- 🔵 **Standards**: `renderInBoard` helper defined but unused; tests use bare `render` with `DndContext`
  **Location**: Step 6 TicketCard.test.tsx
  Either delete the helper or convert each `it` to use it. Two styles in one file is confusing.

- 🔵 **Standards**: `aria-label` on count badge duplicates heading text — risks double announcement
  **Location**: Step 7 KanbanColumn
  Either drop the column name from the badge `aria-label` or set the badge `aria-hidden` and surface the count via a visually-hidden span.

- 🔵 **Standards**: Loading state has no `role="status"` or live-region wiring
  **Location**: Step 8 KanbanBoard
  Either accept the codebase-wide silence or set a Phase 7 precedent by adding `role="status"`. Flag explicitly rather than letting the implementer guess.

- 🔵 **Standards**: Card link has no accessible name beyond the title; ticket number and mtime included in announced name
  **Location**: Step 6 TicketCard
  Restructure so the `<Link>` only wraps the title (matching `EntryCard`), or add `aria-label={\`Ticket #${number}: ${entry.title}\`}` and mark metadata `aria-hidden`.

#### Suggestions
- 🔵 **Code Quality**: Sensor construction is YAGNI for a read-only board
  **Location**: Step 8 KanbanBoard
  Add a comment explaining the eager wiring or omit sensors until Phase 8.

- 🔵 **Code Quality**: Inline `Loading…` and string copy violates the lifecycle precedent (no shared component)
  **Location**: Step 8 KanbanBoard
  Out of scope for Phase 7 but flag for a future hygiene pass: `<LoadingPlaceholder />` and `<ErrorAlert />`.

- 🔵 **Code Quality**: Inline multi-line YAML strings duplicate themselves three times in `seeded_cfg`
  **Location**: Step 1b
  Add `fn write_ticket(dir, slug, title, status)` or accept duplication for three call sites.

- 🔵 **Usability**: No automated test pins keyboard activation (Enter on card navigates)
  **Location**: Step 8 manual verification
  Add a Vitest + RTL test that focuses a TicketCard, fires `userEvent.keyboard('{Enter}')`, asserts pathname becomes `/library/tickets/{slug}`.

- 🔵 **Usability**: PointerSensor + KeyboardSensor instantiated despite all sortables disabled
  **Location**: Step 8 KanbanBoard
  Sensors may pre-empt native interaction. Drop sensors in Phase 7 and add them in Phase 8.

### Strengths

- ✅ Read-only kanban reads from existing `queryKeys.docs('tickets')` cache rather than inventing a parallel `/api/kanban` endpoint — minimises cache state, eliminates duplication, inherits SSE invalidation for free.
- ✅ Phase 7 explicitly defers typed `status` and `ticketNumber` fields on `IndexEntry` with documented rationale; the Other swimlane is a UI-only catch-all keyed off runtime narrowing rather than a persisted status — no server-side enum decision is forced.
- ✅ Pure helpers (`parseTicketNumber`, `groupTicketsByStatus`) are split from React components and independently unit-testable — strong functional core / imperative shell separation.
- ✅ TDD discipline is consistent and explicit: each step writes a failing test first, with red/green steps in the implementation sequence and named test counts in the success criteria.
- ✅ Components are small and have a single responsibility (TicketCard, KanbanColumn, KanbanBoard); composition over inheritance.
- ✅ `STATUS_COLUMNS` is a single source of truth for column keys/labels driven from one place; SCREAMING_SNAKE_CASE naming matches `LIFECYCLE_PIPELINE_STEPS` precedent.
- ✅ Server-side change is minimal and additive (fixture seeding only) — no schema, indexer, or route changes.
- ✅ Test infrastructure changes (ResizeObserver and scrollIntoView jsdom stubs) are recognised up front and added before any dnd-kit-using test runs.
- ✅ `groupTicketsByStatus` correctly short-circuits on `frontmatterState !== 'parsed'` before touching `entry.frontmatter['status']`, avoiding null-dereferences.
- ✅ The `seeded_cfg` ticket fixture slugs verifiably do not collide with the `foo` cluster slug used by `api_lifecycle.rs`.
- ✅ Strong YAGNI discipline in "What we are NOT doing" — keeps Phase 7's blast radius small.
- ✅ Error path differentiates 5xx, 404, and other failures, and explicitly avoids leaking internal status codes/URLs into user-visible copy.
- ✅ Columns are exposed as labelled `<section role=region>` with `aria-labelledby`, giving screen-reader users navigable landmarks.

### Recommended Changes

Ordered by impact. Each change references the finding(s) it addresses.

1. **Fix the test harness for TicketCard and KanbanColumn** (addresses: TanStack Link outside router context — critical)
   Either (a) make `renderInBoard` actually wire children through a route whose component renders the children (build a tiny ad-hoc route tree per test), and have every `it` block use it; or (b) replace `<Link>` href assertions with synchronous URL-shape assertions that don't require router context. Pick one and apply consistently.

2. **Migrate `fetchDocs` (and siblings) to throw `FetchError`** (addresses: critical FetchError mismatch; major dead branches; major typed-error test)
   Add a step before Step 6 that updates `fetchDocs`, `fetchTypes`, `fetchDocContent`, and template fetchers to throw `FetchError` like the lifecycle helpers. Add a unit test pinning the new contract. The Step 8a 500 test then exercises real production behaviour.

3. **Use the canonical Link `to` form** (addresses: major Link target divergence; minor standards)
   Replace `to="/library/tickets/$fileSlug" params={{ fileSlug }}` with `to="/library/$type/$fileSlug" params={{ type: 'tickets', fileSlug }}` in TicketCard and update the test assertion accordingly. Rendered href is identical; consistency win.

4. **Resolve the dnd-kit `disabled: true` strategy** (addresses: major ARIA test gap; major correctness uncertainty; major usability role conflict; major WCAG 4.1.2; minor sensor side effects)
   Strongly recommend dropping `useSortable` and sensors entirely from Phase 7. Keep the `DndContext` wrapper if it eases Phase 8 (no provider re-threading), but render cards as plain `<li><Link/></li>`. Phase 8 then adds `useSortable` (with `disabled: false`), sensors, and the mutation handlers in one cohesive change. If the `disabled: true` strategy is retained, (a) verify experimentally that ARIA attributes are emitted, (b) suppress misleading `aria-roledescription`, (c) put `setNodeRef`/`attributes`/`listeners` on the `<Link>` not the `<li>`, and (d) add a behavioural test that fires a pointer drag and asserts the transform style is unchanged.

5. **Replace CSS tokens with literal palette** (addresses: major missing design tokens)
   Replace every `var(--…)` reference in Steps 6-8 stylesheets with the literal hex values used by `LifecycleClusterView.module.css` and friends. If a token system is desired, that's a separate phase.

6. **Add a board-level heading** (addresses: major heading-level skip)
   Render `<h1>Kanban</h1>` (or `<h2>` matching lifecycle convention) in `KanbanBoard` so the page outline doesn't skip from Sidebar `<h2>` to column `<h3>`. Update tests.

7. **Test SSE-driven invalidation at the kanban layer** (addresses: major test gap)
   Add a KanbanBoard test that returns different `fetchDocs` results across two calls, fires `dispatchSseEvent` (or invalidates the query key directly), and asserts a card moves columns.

8. **Tighten test assertions** (addresses: several minor/major test issues)
   - Pin four-digit zero-padding with a small ticket number (`#0001`).
   - Pin sort tie-breaking with two entries sharing `mtimeMs`.
   - Make `formatMtime` boundary assertion deterministic via injected `now`.
   - Tighten generic-error vs 500-error regexes so they don't both match either copy.
   - Pin Other-key omission contract (`has` returns `false` for empty input).
   - Mirror `router.test.tsx` `beforeEach` stubs in KanbanBoard tests.

9. **Add a retry button to the error alert** (addresses: major no recovery affordance)
   Render a `<button>` inside the alert that calls `refetch()` or invalidates `queryKeys.docs('tickets')`.

10. **Add a CHANGELOG entry** (addresses: minor standards)
    Append to `CHANGELOG.md` `## Unreleased` an `### Added` bullet for the read-only kanban view, dnd-kit dependency, and live SSE updates.

11. **Tidy minor smells** (addresses: minor correctness / quality / usability findings)
    - Move TicketCard under `routes/kanban/` for cohesion.
    - Initialise `OTHER_COLUMN_KEY` in `groupTicketsByStatus` map up-front and drop redundant `groups.set`.
    - Drop the inline `style` from disabled sortables (or comment it).
    - Audit `seeded_cfg` consumers for count-based assertions before adding fixtures, or use a separate helper.
    - Improve "Other" swimlane copy ("Unrecognised status" + per-card status chip).
    - Hide column count chip when 0 (or hide empty-state from AT).
    - Decide loading-state convention (`role="status"` vs current silence) explicitly.
    - Trim card link accessible name to the title.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Structurally sound and reuses Phase 5/6 patterns well: reads from existing `queryKeys.docs('tickets')` cache, mounts dnd-kit providers up-front so Phase 8 only flips a flag, keeps blast radius narrow with one server-side touch (fixture seeding). However, two architectural inconsistencies will likely break the build or tests: `fetchDocs` throws plain `Error` (not `FetchError`) so the planned error-branching is dead code, and the Link target `to="/library/tickets/$fileSlug"` does not match the registered route `/library/$type/$fileSlug`.

**Strengths**:
- Read-only kanban reads from existing cache rather than inventing a parallel endpoint or duplicate query.
- DndContext + per-column SortableContext + per-card useSortable scaffolding mounted with `disabled: true` so Phase 8 only flips the flag.
- Phase 7 explicitly defers typed `status` and `ticketNumber` fields on `IndexEntry` with documented rationale.
- Other swimlane is a UI-only catch-all keyed off runtime narrowing.
- Pure helpers split from React components, independently unit-testable.
- Server-side change is minimal and additive.

**Findings**:
- Major (high): fetchDocs throws plain Error, not FetchError — error branching is dead code.
- Major (high): Link target diverges from typed-route pattern.
- Minor (medium): Kanban-specific TicketCard placed in shared `components/`.
- Minor (medium): `queryKeys.kanban()` invalidated but has no consumer.
- Minor (low): Adding fixtures to `seeded_cfg` increases blast radius.

### Code Quality

**Summary**: Well-structured: TDD throughout, helpers extracted into pure functions, components small and focused. Meaningful inconsistency in error-handling design — `errorMessageFor` branches on `FetchError.status` but production `fetchDocs` throws plain `Error`. Several lower-severity issues (dnd-kit transition style on permanently-disabled cards, sort mutation, fragile literal-text assertions on `formatMtime`).

**Strengths**:
- Pure helpers fully unit-tested and decoupled from React.
- TDD discipline explicit and per-step; red-before-green sequencing enforced.
- Components have single responsibility; composition over inheritance.
- `STATUS_COLUMNS` single source of truth.
- Naming reads cleanly with TSDoc.
- Strong YAGNI discipline.
- dnd-kit `disabled: true` strategy reasonable.

**Findings**:
- Major (high): errorMessageFor relies on FetchError but fetchDocs throws plain Error.
- Minor (high): Helper mutates input ordering of grouped arrays in-place.
- Minor (high): Transform/transition style on permanently-disabled sortable is dead presentation logic.
- Minor (high): Last-modified assertion fragile against `formatMtime` thresholds.
- Minor (medium): Frontmatter narrowing duplicated inline; needs helper.
- Suggestion (medium): Sensors constructed unconditionally — YAGNI for a read-only board.
- Suggestion (high): Inline `Loading…` violates lifecycle precedent (no shared component).
- Suggestion (medium): Inline YAML strings duplicate themselves three times in `seeded_cfg`.

### Test Coverage

**Summary**: Meticulously TDD-disciplined with red-before-green sequencing and explicit per-file test counts. Coverage of pure helpers is thorough; component tests are sensibly scoped. Main gaps: behaviours claimed to "fall out for free" (SSE-driven invalidation never tested at kanban layer), the dnd-kit disabled-state assertion (only checks ARIA attributes that are present even when disabled is false), and edge cases (mtime sort ties, the Other-key omission contract, four-digit zero-padding).

**Strengths**:
- TDD discipline explicit and per-step.
- groupTicketsByStatus has strong edge-case coverage.
- Error-path coverage solid; assertions check raw error text doesn't leak.
- Test infrastructure changes recognised up front (ResizeObserver, scrollIntoView).
- Reuse of makeIndexEntry factory keeps fixtures consistent.

**Findings**:
- Major (high): SSE-driven cache invalidation never tested at the kanban layer.
- Major (high): ARIA assertion does not actually verify `disabled: true`.
- Minor (medium): New integration test largely duplicates existing `parsed` branch coverage.
- Minor (high): No coverage of mtime sort tie-breaking.
- Minor (medium): Other-key omission contract asserted ambiguously.
- Minor (medium): Four-digit zero-padding not pinned.
- Minor (medium): Only one HTTP error class tested; 4xx (404) branch unverified.
- Minor (medium): Heavy use of full router for component tests.
- Minor (low): fetchTemplates / fetchTemplateDetail not stubbed in KanbanBoard tests.

### Correctness

**Summary**: Methodical and well-scoped, with TDD discipline and clearly-stated invariants. Several correctness bugs in the proposed test harnesses (TanStack `<Link>` requires router context that proposed test wrappers don't provide) and in the FetchError integration (production `fetchDocs` throws plain `Error`). Smaller test-regex and assertion issues compound the picture.

**Strengths**:
- `parseTicketNumber` regex correctly rejects malformed inputs.
- `groupTicketsByStatus` correctly short-circuits on `frontmatterState !== 'parsed'`.
- Map initialisation contract preserved.
- `seeded_cfg` ticket slugs verifiably don't collide with `foo` cluster.
- SSE invalidation pipeline already targets `queryKeys.docs('tickets')`.

**Findings**:
- Critical (high): TicketCard / KanbanColumn tests render TanStack `<Link>` outside any router context.
- Critical (high): `fetchDocs` throws plain `Error`, not `FetchError`.
- Major (medium): When `useSortable` is `disabled: true`, dnd-kit may not emit role/aria-roledescription.
- Major (high): `formatMtime` regex assertion fragile and matches by accident at 60s boundary.
- Minor (high): Sort comparator does not stably order entries with equal `mtimeMs`.
- Minor (medium): Card with no numeric prefix links to a slug that has no IndexEntry.
- Minor (high): Generic-error test regex over-matches the FetchError branch's copy.
- Minor (medium): New ticket fixtures expand the lifecycle cluster set.
- Minor (medium): Sensors remain wired despite all sortables being disabled.

### Usability

**Summary**: Coherent read-only kanban with sensible structural choices, but several end-user UX details around accessibility, keyboard semantics, error recovery, loading affordance, and screen-reader messaging will produce friction. Most important issues stem from layering a disabled dnd-kit sortable wrapper around a TanStack Link inside an `<li>`: conflicting roles, ambiguous keyboard semantics, misleading "sortable" role-description.

**Strengths**:
- Columns exposed as labelled `<section role=region>` with linked headings.
- STATUS_COLUMNS single source of truth, Other swimlane conditional.
- Error path differentiates 5xx, 404, and other; doesn't leak internal status codes.
- Plan acknowledges focus-ring visibility and keyboard reachability.
- SSE-driven invalidation means external edits appear without manual refresh.

**Findings**:
- Major (high): dnd-kit role=button on `<li>` wraps `<Link>`, conflicting roles and ambiguous keyboard semantics.
- Major (high): aria-roledescription="sortable" exposed even though no sorting is possible (WCAG 4.1.2).
- Major (high): Error messages have no recovery affordance.
- Minor (medium): Loading state is bare text with no aria-live, no skeleton, no layout reservation.
- Minor (high): Pluralisation produces "0 tickets in Done" for empty columns.
- Minor (medium): Empty-column copy ambiguous between "no data" and "genuinely empty status".
- Minor (medium): "Other" swimlane label doesn't explain why a ticket landed there.
- Minor (medium): Ticket number absence is silent; no fallback or explanation.
- Suggestion (low): Sensors instantiated despite all sortables disabled.
- Suggestion (medium): No automated test pins keyboard activation (Enter on card navigates).

### Standards

**Summary**: Broadly aligned with established conventions (camelCase CSS module classnames, `*.test.tsx` colocation, TDD matching Phase 5/6, file placement, ARIA `role="region"` + `aria-labelledby`). However, CSS examples reference design tokens that don't exist in the project; the page has no `<h1>`/`<h2>`, jumping straight to `<h3>` for column headings; and there is no CHANGELOG entry planned for a notable user-visible feature.

**Strengths**:
- CSS module classname casing matches established camelCase convention.
- Test-file naming and TDD-first sequencing consistent with Phases 5-6.
- Constant naming follows SCREAMING_SNAKE_CASE convention.
- ARIA pattern for columns mirrors `LifecycleClusterView`.
- Error wording sanitised, no internal-detail leakage.
- Fixture slugs sandboxed in per-test temp directory.
- Component placement is defensible.

**Findings**:
- Major (high): CSS variables used in stylesheets do not exist in the project.
- Major (high): Page outline skips heading levels — no h1/h2, jumps to h3.
- Minor (high): No CHANGELOG entry planned for a user-visible feature.
- Minor (medium): TanStack Link `to` form inconsistent with the rest of the codebase.
- Minor (medium): `renderInBoard` helper defined but unused.
- Minor (high): aria-label on count badge duplicates heading text.
- Minor (medium): Loading state has no `role="status"` or live-region wiring.
- Minor (medium): Card link has no accessible name beyond the title; metadata included in announced name.

## Re-Review (Pass 2) — 2026-04-26

**Verdict:** COMMENT

Plan is acceptable but could be improved — see major finding below.

### Previously Identified Issues

#### Critical (both resolved)
- ✅ **Correctness**: TicketCard / KanbanColumn tests render TanStack `<Link>` outside any router context — **Resolved**. Step 4 introduces `frontend/src/test/router-helpers.tsx` with `renderWithRouterAt(ui)` that registers `/library/$type/$fileSlug` so Link resolution works under tests; TicketCard and KanbanColumn tests use it consistently.
- ✅ **Correctness / Architecture / Code Quality**: `fetchDocs` throws plain `Error`, not `FetchError` — **Resolved**. Step 3 migrates all five fetch helpers (`fetchTypes`, `fetchDocs`, `fetchDocContent`, `fetchTemplates`, `fetchTemplateDetail`) to throw `FetchError` in one TDD pass with parametric tests across both 4xx and 5xx.

#### Major (9 resolved, 1 partially resolved)
- ✅ **Architecture / Standards**: Link target `/library/tickets/$fileSlug` divergence — **Resolved**. TicketCard now uses `to="/library/$type/$fileSlug" params={{ type: 'tickets', fileSlug }}` and a test pins the rendered href.
- ✅ **Test Coverage**: SSE-driven cache invalidation never tested — **Resolved**. Step 10 adds a KanbanBoard test that invalidates `queryKeys.docs('tickets')` and asserts a card moves columns plus `fetchDocs` was called twice.
- 🟡 **Test Coverage**: ARIA assertion does not actually verify `disabled: true` — **Partially resolved**. The new "does not announce a misleading 'sortable' role-description while disabled" test pins the ARIA suppression. However, the new "does not respond to drag interaction while disabled" behavioural test is a tautology — see new major finding below.
- ✅ **Correctness**: dnd-kit ARIA emission with `disabled: true` — **Resolved**. The plan now pre-empts the uncertainty by destructuring `aria-roledescription` away while disabled rather than depending on dnd-kit's emit behaviour, with a positive test (`expect(link.getAttribute('aria-roledescription')).toBeNull()`).
- ✅ **Correctness**: `formatMtime` regex fragile at 60s boundary — **Resolved**. Tests use `FROZEN_NOW = 1_700_000_000_000` and `FROZEN_NOW - 90_000` to land deterministically in the "1m ago" bucket.
- ✅ **Usability**: dnd-kit role=button on `<li>` wraps `<Link>` — **Resolved**. `setNodeRef`/`attributes`/`listeners` now attach to the `<Link>`, producing one focus stop per card.
- ✅ **Usability**: `aria-roledescription="sortable"` exposed despite no sorting — **Resolved**. Suppressed via destructure while `PHASE_7_DISABLED`.
- ✅ **Usability**: Error messages no recovery affordance — **Resolved**. Error alert renders a `<button>Retry</button>` that invalidates `queryKeys.docs('tickets')`; a test exercises the recovery path end-to-end.
- ✅ **Standards**: CSS variables used in stylesheets do not exist — **Resolved**. All `var(--…)` references replaced with literal hex values matching `LifecycleClusterView.module.css`.
- ✅ **Standards**: Page outline skips heading levels — **Resolved**. KanbanBoard renders `<h1>Kanban</h1>`; columns demoted to `<h2>` with explicit `level` assertions in tests.

#### Minor (most resolved)
- ✅ Architecture: Kanban-specific TicketCard placed in shared `components/` — **Resolved** (moved to `routes/kanban/`).
- ✅ Architecture: `queryKeys.kanban()` invalidated but no consumer — **Resolved** via documented "What we are NOT doing" entry.
- ✅ Architecture: Adding fixtures to `seeded_cfg` increases blast radius — **Resolved**. Step 1 adds an explicit `rg` audit pass plus a documented `seeded_cfg_with_tickets` fallback.
- ✅ Code Quality: `groupTicketsByStatus` redundant `groups.set` and mutation — **Resolved** with lazy-init pattern and tie-break sort.
- 🟡 Code Quality: Transform/transition style on permanently-disabled sortable — **Still present** (style remains pre-wired).
- ✅ Code Quality: Frontmatter narrowing duplicated inline — **Resolved** via `statusGroupOf` helper.
- ✅ Test Coverage: Mtime sort tie-breaking — **Resolved** with `relPath.localeCompare` tie-break and a test pinning order.
- ✅ Test Coverage: Other-key omission contract ambiguous — **Resolved** with explicit `groups.has(OTHER_COLUMN_KEY) === false` assertions.
- ✅ Test Coverage: Four-digit zero-padding not pinned — **Resolved** by using `0001-` fixture and asserting `#0001`.
- ✅ Test Coverage: Only one HTTP error class tested — **Resolved**. `fetch.test.ts` covers both 503 and 404 across all 5 helpers; the kanban-level 404 special case was dropped.
- ✅ Test Coverage: Heavy use of full router for component tests — **Resolved** via `renderWithRouterAt`.
- ✅ Test Coverage: `fetchTemplates` / `fetchTemplateDetail` not stubbed — **Resolved**. KanbanBoard `beforeEach` stubs all three root-layout fetches.
- ✅ Correctness: Sort comparator stability — **Resolved** via `relPath` tie-break.
- ✅ Correctness: Generic-error regex over-matches — **Resolved**. Both regexes tightened with negative assertions against the other branch's copy.
- ✅ Correctness: New ticket fixtures expand cluster set — **Resolved** via audit step.
- ✅ Correctness: Sensors wired despite all sortables disabled — **Resolved** via inline justification comment in Step 10.
- ✅ Usability: Loading state bare with no aria-live — **Resolved** with `role="status"`.
- ✅ Usability: Pluralisation "0 tickets in Done" — **Resolved**. Count-only aria-label + `aria-hidden` empty-state.
- 🟡 Usability: Empty-column copy ambiguity — **Partially resolved** (sighted users see "No tickets" but AT users may be silent — see new minor below).
- ✅ Usability: "Other" swimlane label doesn't explain — **Resolved**. KanbanColumn accepts a `description` prop; KanbanBoard passes "Tickets whose status is missing or not one of: todo, in-progress, done."
- ✅ Usability: Ticket number absence silent — **Resolved** via slug fallback chip.
- ✅ Standards: No CHANGELOG entry — **Resolved**. Step 12 adds an `### Added` bullet under `## Unreleased`.
- ✅ Standards: TanStack Link `to` form inconsistent — **Resolved**.
- ✅ Standards: `renderInBoard` helper unused — **Resolved** (replaced by `renderWithRouterAt`).
- ✅ Standards: aria-label on count duplicates heading — **Resolved**. Now count-only.
- ✅ Standards: Loading state no `role="status"` — **Resolved**.
- 🟡 Standards: Card link accessible name beyond title — **Still present**. The `<Link>` continues to wrap the chip, mtime, title, and type, so its accessible name concatenates them.

### New Issues Introduced

#### Major
- 🟡 **Test Coverage / Correctness**: Drag-disabled behavioural test in Step 8a is a tautology
  **Location**: Step 8a TicketCard test "does not respond to drag interaction while disabled"
  Dispatching raw `PointerEvent('pointerdown')` / `PointerEvent('pointermove')` on the link in jsdom does not engage dnd-kit's sensor pipeline (no measuring, no transform mutation) regardless of whether `disabled` is `true` or `false`. The inline `style` attribute is also written once at React render time, so firing DOM events without triggering a re-render cannot mutate it. The test passes under both states and therefore does not pin the `PHASE_7_DISABLED = true` invariant.

#### Minor (new)
- 🔵 **Code Quality**: `PHASE_7_DISABLED` constant leaks planning context into production code. Consider `disabled?: boolean` prop (default `true`) so Phase 8 flips a default at the call site instead of grepping for a phase number.
- 🔵 **Code Quality**: Silent destructure of `aria-roledescription` is fragile cross-phase. The cross-phase coupling ("Phase 8 must drop the destructure") relies on the next author remembering. Conditional strip on `disabled` would couple them in lockstep.
- 🔵 **Code Quality**: `errorMessageFor` collapses 4xx into the generic bucket — under-uses the typed FetchError surface that Step 3 was justified to enable.
- 🔵 **Correctness**: TanStack `<Link>` ref-forwarding to underlying anchor unverified by tests. v1 Link uses `forwardRef`, so this likely works, but no test pins the contract.
- 🔵 **Correctness**: `_ariaRoleDescription` unused destructure binding may trip `noUnusedLocals` / strict ESLint. Use anonymous `_` or `omit`-style helper to be safe.
- 🔵 **Correctness**: SSE re-grouping test depends on QueryClient defaults (`staleTime`, `refetchType`). A future default change to `staleTime: Infinity` would silently break it. Use `refetchQueries` directly or assert call-count before the re-render assertion.
- 🔵 **Correctness**: `renderWithRouterAt`'s typed `<Link to=...>` resolves against the global app routeTree (via module augmentation) but the helper's runtime tree only registers `/` and `/library/$type/$fileSlug`. A future test that renders a `<Link>` to another route would typecheck but navigate to nothing.
- 🔵 **Architecture**: FetchError migration is bundled inside Phase 7 — nominally about kanban. Land Step 3 as its own commit (the implementation sequence already isolates it) so git history attributes it correctly.
- 🔵 **Architecture**: `src/test/router-helpers.tsx` registering production routes (`/library/$type/$fileSlug`) creates a duplication seam — adding a new route to `router.ts` may silently break test resolution. Add a TODO comment listing intentionally mirrored routes.
- 🔵 **Architecture**: `useMemo` over `groupTicketsByStatus`'s Map result is a latent foot-gun for Phase 8 downstream memoisation — `groups.get(...)` returns a fresh array each render.
- 🔵 **Test Coverage**: Server integration test still largely overlaps existing parsed-branch coverage. Trim to the high-value `frontmatterState === 'parsed'` assertion for the exotic `blocked` ticket.
- 🔵 **Test Coverage**: SSE invalidation test pins the cache key but skips the SSE codepath itself. Consider driving `dispatchSseEvent` directly to exercise the full pipeline.
- 🔵 **Test Coverage**: `mockFetch` cast `as unknown as typeof fetch` hides response-shape drift. Use `new Response('', { status })` instead.
- 🔵 **Usability**: Empty columns may be silent for screen-reader users navigating by region — the count badge's aria-label is on a sibling span, not in the body. Test what VoiceOver actually says or move the announcement onto the empty-state paragraph.
- 🔵 **Standards**: Page-level `<h1>` introduces inconsistency with sibling top-level routes (`LifecycleIndex` has no h1, `LibraryDocView` has h1→h3). Document a follow-up to retrofit other views, or accept Phase 7 as the precedent and note explicitly.

#### Suggestions (new)
- 🔵 **Code Quality**: Inline `Loading…` paragraph remains; no shared `<LoadingStatus />` primitive — duplication hardening into a non-pattern.
- 🔵 **Code Quality**: YAML fixture strings in `seeded_cfg` still triplicated.
- 🔵 **Code Quality**: dnd-kit `attributes` spread on `<Link>` may also include `role="button"` and `tabIndex={0}`; the test only asserts `aria-roledescription` is suppressed. Consider asserting `link.getAttribute('role')` is null too.
- 🔵 **Usability**: Slug-fallback chip lacks explanatory affordance (e.g. tooltip "Filename has no ticket number prefix").

### Assessment

The plan is now in good shape to implement. Both critical findings and 9 of 10 majors are resolved with thoughtful, well-scoped changes; the cross-cutting themes (FetchError parity, dnd-kit truthfulness, Link form, CSS palette, heading hierarchy, error recovery) are addressed end-to-end with tests pinning the new contracts.

The single remaining major — the tautological drag-disabled behavioural test — is a real gap but a small one: the test wastes effort and gives false confidence rather than introducing a bug. The pragmatic fix is to replace the pointer-event approach with a structural assertion (`attributes['aria-disabled']` or `Object.keys(listeners).length === 0` or spy on `useSortable`'s argument) before declaring Step 8 green.

Several of the new minors are forward-looking (Phase 8 migration friction from `PHASE_7_DISABLED` and the silent destructure, downstream memoisation ergonomics) rather than Phase 7 blockers, and several are housekeeping (commit boundary for the FetchError migration, accessible-name cleanup). None should block implementation.

Recommendation: **proceed to implement**, addressing the drag-disabled test fix during Step 8 (replace the pointer-event assertion with a structural one — verify the test fails when `PHASE_7_DISABLED` is locally set to `false` before declaring it green). The minor findings can be triaged opportunistically as the corresponding steps land or deferred to a follow-up cleanup pass.
