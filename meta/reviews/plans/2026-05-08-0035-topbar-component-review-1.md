---
date: "2026-05-08T08:21:37+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-08-0035-topbar-component.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 4
status: complete
---

## Plan Review: 0035 Topbar Component Implementation Plan

**Verdict:** REVISE

The plan delivers a thoughtfully decomposed Topbar with strong TDD discipline,
honours existing token / mocking / data-attribute conventions, and correctly
identifies the three first-of-kind frontend conventions it introduces. Across
six lenses the substantive concerns cluster around four themes: a Phase 5→6
keyframe rename and a `data-*` test-affordance pattern that creates silent
contracts between this story and the next, an under-enforced loader contract
that breadcrumb consumers cast their way through, a Phase 1 layout that does
not actually leave a 56-px row as advertised, and a `window.location` mocking
technique that is unreliable in jsdom. None of the issues block the design;
all are addressable inside the plan before implementation.

### Cross-Cutting Themes

- **Test-only `data-*` attributes leaking into production markup** (flagged
  by: test-coverage, correctness, standards, code-quality, usability) —
  `data-flush`, `data-empty`, `data-pulse`, `data-animated` are introduced
  primarily to satisfy the no-`getComputedStyle` test convention. Several
  are hard-coded to `"true"` so the tests asserting them cannot fail; the
  `data-empty` case in particular hands 0034 a latent collaboration bug
  (populated slots will collapse to zero size unless 0034 remembers to
  remove the attribute).
- **Phase 5 → Phase 6 keyframe rename** (flagged by: architecture,
  code-quality, test-coverage, usability) — naming a keyframe `pulseGreen`
  in Phase 5 and renaming it to `pulse` in Phase 6 introduces gratuitous
  diff churn, weakens phase independence, and leaves Phase 5 with no test
  that would actually catch the rename going wrong.
- **Loader-data crumb contract enforced only by convention** (flagged by:
  architecture, code-quality, correctness, usability) — `Breadcrumbs` casts
  `m.loaderData as { crumb: string }` and silently drops any route whose
  loader is missing. A future contributor who forgets a loader sees no
  warning, no type error, and no test failure — just a missing crumb.
- **Two `role="status"` live regions in persistent chrome** (flagged by:
  standards, usability) — `OriginPill`'s host string never changes, and
  `SseIndicator`'s label changes on every transient transition; the live
  regions either do nothing useful or over-announce.

### Tradeoff Analysis

- **Test affordances vs. clean production markup**: The codebase forbids
  `getComputedStyle` and CSS-class assertions, which led the plan to
  `data-*` markers. Test-coverage suggests raw-CSS string assertions
  (mirroring `global.test.ts`'s `?raw` pattern); standards suggests
  `:empty` pseudo-class for the slot case and dropping `data-flush`
  entirely. Both directions would shrink the production-side leakage.
  Recommend adopting raw-CSS assertions where structural CSS is the
  contract under test, and keeping `data-*` only for genuine runtime
  state (`data-state` on the SSE indicator).
- **Scope discipline vs. accessibility baseline**: The plan defers
  `prefers-reduced-motion` and humanised crumb titles to follow-up. The
  reduced-motion guard is one CSS rule and ships for free; deferring it
  is a poor trade. Slug-prettification is genuinely larger and the
  scope decision is defensible — but should be linked back to the AC
  line so the gap doesn't get lost.

### Findings

#### Major

- 🟡 **Correctness**: window.location redefinition pattern is unreliable in jsdom
  **Location**: Phase 5: OriginPill — test setup with `Object.defineProperty(window, 'location', ...)`
  jsdom installs `window.location` as a non-configurable accessor; naive
  redefinition frequently throws `TypeError: Cannot redefine property: location`.
  The plan cites `setup.ts` as evidence of safety, but `setup.ts` does not
  stub `location`. Combined with the architectural concern about ambient
  `window.*` reads in a leaf component, the cleaner fix is to inject host
  via a `useOrigin()` hook the test mocks via `vi.mock`.

- 🟡 **Correctness**: Phase 1 leaves no element that occupies the 56px topbar row
  **Location**: Phase 1: Foundation — RootLayout shell restructure
  The Phase 1 manual verification claims "nothing visually changes (the 56-px
  row above is empty until Phase 8)", but `.root` contains only `.body` —
  there is no element producing the 56-px row. The visible result is either
  `.body` filling the viewport (no gap) or 56 px of empty space at the
  *bottom* (because `.root` has `min-height: 100vh` and the child is
  shorter). Either way, Phase 1 cannot ship as a clean intermediate state.

- 🟡 **Test Coverage**: Phase 5 OriginPill test cannot detect the Phase 6 keyframe rename regression
  **Location**: Phase 6: SseIndicator — keyframe rename refactor
  Phase 5 only asserts `data-pulse="true"`. Phase 6 renames `@keyframes
  pulseGreen` → `pulse` and rebinds `.pulseDot`'s animation. The Phase 6
  success criterion "OriginPill.test.tsx still passes after the keyframe
  rename" is trivially true because the `data-pulse` marker is unrelated
  to the animation binding. A regression that left `.pulseDot` referencing
  the deleted `pulseGreen` keyframe would not fail any test.

- 🟡 **Test Coverage**: Bespoke breadcrumb router duplicates production route definitions and will drift
  **Location**: Phase 4: Breadcrumbs — bespoke router test setup
  Building a parallel reduced route tree with its own loaders means
  production loader changes (e.g. `'Library'` → `'Lib'`) won't fail the
  Breadcrumbs test. Reuse the production `routeTree` (as `router.test.tsx`
  already does) with `createMemoryHistory` and a `Breadcrumbs`-rendering
  wrapper, or factor loader bodies into shared exported constants.

- 🟡 **Test Coverage**: SidebarFooter behaviour parity is silently dropped
  **Location**: Phase 8 / Testing Strategy — SidebarFooter removal
  `SidebarFooter.test.tsx` covers six behaviours including `Reconnecting…`,
  `Reconnected — refreshing`, version label, and `justReconnected + open`
  precedence. Phase 8 deletes the file. The new `SseIndicator` covers four
  `connectionState` values only. The drops are likely intentional, but the
  plan does not state this; reviewers see net coverage loss with no
  rationale.

- 🟡 **Standards**: Breadcrumb markup does not follow the WAI-ARIA Breadcrumb pattern
  **Location**: Phase 4: Breadcrumbs sub-component (Breadcrumbs.tsx code block)
  The W3C WAI-ARIA Authoring Practices Breadcrumb pattern specifies an
  ordered list (`<ol>`/`<li>`) inside `<nav aria-label="Breadcrumb">`,
  with `aria-current="page"` on the final crumb. The plan uses flat
  `<span>` siblings with separators rendered inside each crumb. Screen
  readers will not announce the trail as a structured list and no item
  is identified as the current page.

- 🟡 **Usability**: Two infinite-pulse animations ship with no prefers-reduced-motion guard
  **Location**: What We're NOT Doing — prefers-reduced-motion bullet; Phases 5/6
  Persistent infinite alternating-opacity pulses on every page for every
  user, with no opt-out for users with vestibular sensitivity. The fix is
  one `@media (prefers-reduced-motion: reduce)` block disabling
  `.pulseDot` and `.sse[data-animated='true']` animations. Deferring is
  cheaper to fix now than after the shared `pulse` keyframe acquires
  more callers.

- 🟡 **Usability**: Future route authors who forget a loader silently drop out of the breadcrumb chain
  **Location**: Phase 4: Breadcrumbs — `(m.loaderData as { crumb: string })` cast
  No console warning, no dev-mode assertion, no compile error. The plan
  even relies on this silent-skip behaviour for redirect routes,
  making the missing-loader case indistinguishable from intentional
  omission. Add a dev-only `console.warn` for non-redirect matches that
  are missing `crumb`, or wrap `createRoute` with a `withCrumb()` helper
  that makes the contract obvious at the call site.

- 🟡 **Usability**: Raw slugs as crumb labels degrade end-user experience from day one
  **Location**: Desired End State; What We're NOT Doing
  Breadcrumbs like `Library / decisions / 0035-topbar-component` show
  internal slugs in chrome on every detail page. The work item AC mentions
  "pulling document titles from loader data where available". The current
  scope reads slugs as acceptable — at minimum, link the deferral to the
  follow-up story owner so the gap doesn't get lost.

#### Minor

- 🔵 **Architecture**: Breadcrumbs assumes loaderData shape via cast — type contract is implicit
  **Location**: Phase 4: Breadcrumbs (component code) and Migration Notes
  The contract that "every contributing route returns `{ crumb: string }`"
  lives only in prose. Define a shared `RouteLoaderData` type and have
  each loader annotate against it; replace the cast with a type guard.

- 🔵 **Architecture**: SidebarFooter removal silently drops `justReconnected` post-reconnect transient
  **Location**: Phase 8: RootLayout integration + SidebarFooter removal
  The "Reconnected — refreshing" 3-second window after reconnect is
  unrepresented in the new SSE indicator. Either explicitly note this in
  "What We're NOT Doing" (and follow-up to remove `justReconnected` from
  `DocEventsHandle`), or have `SseIndicator` reflect it.

- 🔵 **Architecture**: 0034 toggle slots are mounted via DOM-position coupling rather than a composition seam
  **Location**: Phase 7: Topbar composition — empty slot divs
  The slot abstraction is naming-only, not structural. 0034 will need to
  edit `Topbar.tsx` (failing the Open/Closed promise) or query DOM by
  `data-slot`. Either accept this and document it, or expose
  `themeToggleSlot?: ReactNode` props for true composition.

- 🔵 **Architecture**: First `window.location` consumer introduced inside a leaf component
  **Location**: Phase 5: OriginPill
  All other ambient I/O in this codebase is injected via factories or
  context. A `useOrigin()` hook localises the dependency and matches the
  `vi.mock` test convention used elsewhere.

- 🔵 **Architecture**: `min-height: 100vh` on `.root` plus `min-height: calc(100vh - var(--ac-topbar-h))` on `.body` allows total layout > 100vh
  **Location**: Phase 1: Foundation
  Use `height: calc(...)` on `.body` (or drop its min-height) so the body
  row is exactly the remainder. State the layout invariant explicitly.

- 🔵 **Code Quality**: Keyframe is named-then-renamed across consecutive phases
  **Location**: Phase 5 (`pulseGreen`) and Phase 6 (rename to `pulse`)
  Introduce the colour-agnostic `pulse` keyframe directly in Phase 5;
  drop the rename step from Phase 6.

- 🔵 **Code Quality**: ARIA label lookup loses ConnectionState type narrowing
  **Location**: Phase 6 SseIndicator — `LABELS: Record<string, string>`
  Type as `Record<ConnectionState, string>` to enforce exhaustive
  coverage.

- 🔵 **Code Quality**: Type assertion bypasses route-data type safety
  **Location**: Phase 4 Breadcrumbs — `m.loaderData as { crumb: string }`
  Extract a `hasCrumb` type guard so the contract is enforced at one site.

- 🔵 **Code Quality**: Dead code left behind after SidebarFooter removal
  **Location**: Phase 8 — collateral cleanup
  After the deletion, `useServerInfo()` has no production consumer and
  `DocEventsHandle.justReconnected` has no UI consumer. Either remove
  them or note them as intentionally retained.

- 🔵 **Code Quality**: OriginPill's host read is silently non-reactive
  **Location**: Phase 5
  Performance Considerations claim "read once on mount" but the
  implementation reads on every render. Align with `useState(() =>
  window.location.host)` or fix the doc.

- 🔵 **Code Quality**: Two sequential array filters obscure the intent
  **Location**: Phase 4 Breadcrumbs — pending-match filtering
  Combine into one filter with a named local predicate.

- 🔵 **Test Coverage**: SSE indicator colour mapping has no test
  **Location**: Phase 6: SseIndicator — colour-token contract
  Add a CSS-source raw-string assertion (mirror `global.test.ts`'s
  `?raw` pattern) verifying each `[data-state='X']` selector binds to
  the expected token.

- 🔵 **Test Coverage**: `Object.defineProperty(window, 'location', ...)` per-test should fully restore
  **Location**: Phase 5: OriginPill — `window.location` test isolation
  Save the original descriptor in `beforeEach` and restore in
  `afterEach`; or stub only the `host` field.

- 🔵 **Test Coverage**: Pending-state filter has no explicit test
  **Location**: Phase 4: Breadcrumbs
  Mock `useMatches` directly with a synthetic `pending` entry and
  assert it is excluded; also test the empty-list `null` render.

- 🔵 **Test Coverage**: Phase 1 asserts wrapper presence but never verifies column direction
  **Location**: Phase 1: RootLayout test
  Add a CSS-source raw-string check verifying `.root` declares
  `flex-direction: column` and `.body` declares the calc-based
  `min-height`.

- 🔵 **Test Coverage**: `/library/templates` chain coverage missing
  **Location**: Phase 4: Breadcrumbs
  Add `/library/templates` and `/library/templates/adr` cases.

- 🔵 **Test Coverage** + 🔵 **Standards** + 🔵 **Correctness**: `data-empty="true"` is hardcoded
  **Location**: Phase 7: Topbar composition
  Either compute `data-empty` from `React.Children.count(children) === 0`
  on a `<Slot>` wrapper component, drop it in favour of `:empty`
  pseudo-class CSS, or document explicitly that 0034 must remove the
  attribute when populating.

- 🔵 **Test Coverage** + 🔵 **Standards**: `data-flush` is unconditional and primarily a CSS-targeting hack
  **Location**: Phase 4: Breadcrumbs
  Drop the attribute. Use a class on the breadcrumbs root, or assert
  the structural CSS via raw-string check on the module.

- 🔵 **Correctness**: Pending-state filter is redundant given isMatch's null guard
  **Location**: Phase 4: Breadcrumbs
  `isMatch` returns false when `loaderData.crumb` is null/undefined, so
  the explicit `m.status !== 'pending'` filter is redundant. Drop it
  with a comment.

- 🔵 **Correctness**: Cast bypasses what isMatch could narrow; empty-string crumbs slip through
  **Location**: Phase 4: Breadcrumbs
  Add a length check after the filter, or replace the cast with a type
  predicate that performs the same check.

- 🔵 **Correctness**: `data-animated={animated ? 'true' : undefined}` contract is correct but undocumented
  **Location**: Phase 6: SseIndicator
  React omits attributes whose value is `undefined`. Add a comment so
  a maintainer doesn't change it to `String(animated)` and break the
  assertion.

- 🔵 **Standards**: `role="img"` on a `<div>` wrapping inline SVG plus visible text is non-idiomatic
  **Location**: Phase 3: Brand sub-component
  Drop `role="img"` and `aria-label` from the wrapper; mark the SVG
  `aria-hidden="true"`.

- 🔵 **Standards** + 🔵 **Usability**: Two `role="status"` regions over-announce
  **Location**: Phase 5 (OriginPill) and Phase 6 (SseIndicator)
  Drop `role="status"` from `OriginPill` (host never changes). Keep on
  `SseIndicator` but consider firing live updates only on
  `closed`/`reconnecting`-after-grace.

- 🔵 **Standards**: Co-located non-exported sub-components depart from one-component-per-directory convention
  **Location**: Desired End State + Phases 3-7
  Either promote each sub-component to its own sibling directory, or
  document this nested-private-sub-component pattern in Migration
  Notes.

- 🔵 **Standards**: Token group naming for layout chrome height is unsettled
  **Location**: Phase 1, Section 3
  Pick `LAYOUT_TOKENS` or extend `SPACING_TOKENS`; don't leave to
  phase-time judgement.

- 🔵 **Usability**: Test-only `data-*` attributes leak test concerns into production markup
  **Location**: Phases 4, 5, 6, 7
  Either drop them and assert via the CSS-modules object (vite.config
  enables `css: true`), or rename to `data-test-*` for clearly
  test-scoped intent.

- 🔵 **Usability**: Mid-plan keyframe rename creates diff-history friction
  **Location**: Phase 5 → Phase 6
  Name the keyframe `pulse` from the start in Phase 5; drop the
  rename note from Phase 6.

- 🔵 **Usability**: `data-slot` contract for 0034 is under-specified
  **Location**: Phase 7 Migration Notes
  Add a 1-paragraph "Slot contract" subsection or export a typed
  `<TopbarSlot name="theme-toggle">{children}</TopbarSlot>` component.

- 🔵 **Usability**: OriginPill's `typeof window !== 'undefined'` guard signals SSR awareness that doesn't apply
  **Location**: Phase 5
  Drop the SSR guard with a comment that the codebase is SPA-only,
  or use `useState(() => window.location.host)` and update the test
  expectation accordingly.

#### Suggestions

- 🔵 **Architecture** + 🔵 **Code Quality**: Cross-phase keyframe rename complicates phase rollback; `.shell` rename to `.body` is churn worth confirming
  **Location**: Phase 1 / Phase 6
  Decide on naming up front (Phase 5 ships `pulse`; outer wrapper is
  `.frame` or `.layout` so `.shell` stays put) so phases stay
  individually mergeable.

- 🔵 **Correctness**: URL-encoded slugs render verbatim in breadcrumbs
  **Location**: Phase 2: Route loaders
  React text rendering is safe; ugly slugs are an end-user concern
  documented under "Raw slugs as crumb labels" above. No code change
  required at this layer.

### Strengths

- ✅ Sub-component decomposition (Brand, Breadcrumbs, OriginPill,
  SseIndicator) gives each module a single reason to change and aligns
  with the codebase's co-located component+CSS+test triple convention.
- ✅ Loader-data contract for breadcrumbs decouples the breadcrumb
  component from route-specific knowledge and is idiomatic TanStack
  Router.
- ✅ DocEventsContext is reused as-is (not extended) and the Provider
  remains at RootLayout, preserving a single source of truth for SSE
  state.
- ✅ Origin reading via `window.location.host` rather than extending
  `useServerInfo` is a sensible scoping decision (with the tradeoff
  acknowledged).
- ✅ Token-driven layout (`--ac-topbar-h`) propagated through
  `tokens.ts` parity test makes the height a system-level invariant.
- ✅ Bottom-up phasing keeps each phase's blast radius narrow.
- ✅ Test conventions are respected: `vi.mock` for context hooks,
  role/text/data-attribute queries, no `getComputedStyle`, no
  CSS-modules class assertions.
- ✅ Phase 8 includes `git grep` checks to confirm `SidebarFooter` is
  fully removed.
- ✅ Plan explicitly calls out first-of-kind frontend conventions
  (`window.location`, `@keyframes`, `useMatches() + loaderData`) so
  the next maintainer knows where novelty lives.
- ✅ ConnectionState union is exhaustively covered by the SSE indicator
  state map.
- ✅ Loader cascade verified: `libraryDocRoute` parent chain produces
  `Library / decisions / foo` automatically without manual chaining.

### Recommended Changes

Ordered by impact:

1. **Drop the Phase 5 → Phase 6 keyframe rename** (addresses: keyframe
   rename across architecture/code-quality/test-coverage/usability).
   Name the keyframe `pulse` in Phase 5 from the start. Phase 6 only
   adds the `data-state` colour rules and `data-animated` selector.
   Removes diff churn and removes the no-op test in Phase 6's success
   criteria.

2. **Replace the `window.location` mock with a `useOrigin()` hook**
   (addresses: window.location jsdom unreliability, leaf-component
   ambient I/O, host non-reactive contradiction, SSR-guard dead
   code). Add `src/api/use-origin.ts` returning `window.location.host`
   read once at module load (or via `useState(() => …)`). `OriginPill`
   consumes it; the test mocks the hook with `vi.mock`. Removes the
   jsdom `Object.defineProperty` failure mode and the SSR guard.

3. **Rework Phase 1 so it ships an honest interim layout** (addresses:
   "no element occupies 56px row", `min-height` calc invariant). Either
   collapse Phase 1 into Phase 8 (single commit lands the Topbar and
   the column layout), mount a stub `<header
   style={{height:'var(--ac-topbar-h)'}}/>` for the interim, or change
   `.body` to `flex: 1` inside the column and drop its `min-height`
   so the layout invariant is "shell is exactly 100vh; body fills the
   remainder".

4. **Adopt the WAI-ARIA Breadcrumb pattern** (addresses: standards
   major). Render `<nav aria-label="Breadcrumb"><ol><li>…</li></ol></nav>`
   with separators between siblings (or `::before` on non-first items)
   and `aria-current="page"` on the last crumb.

5. **Make the loader-crumb contract explicit and self-correcting**
   (addresses: silent-drop usability major + architecture/code-quality
   minors on the cast). Either: (a) introduce a shared
   `RouteLoaderData` type and a `hasCrumb` type guard so Breadcrumbs
   filters via the guard with no cast, OR (b) wrap `createRoute` with
   a `withCrumb(crumb, options)` helper exported from `router.ts`.
   Either way, add a dev-only `console.warn` in `Breadcrumbs` when a
   non-redirect match is missing `crumb`.

6. **Reuse the production routeTree in Breadcrumbs tests** (addresses:
   bespoke-router drift). Mirror `router.test.tsx`'s pattern — import
   the real `routeTree` and use `createMemoryHistory({ initialEntries:
   [url] })`. Add `/library/templates` and `/library/templates/adr`
   to the case list while you're there.

7. **Replace `data-empty="true"`, `data-flush="true"`, and `data-pulse"true"`
   with their natural equivalents** (addresses: cross-cutting `data-*`
   leakage theme). For empty slots, use the CSS `:empty` pseudo-class
   so the rule self-disengages when 0034 fills them. For breadcrumbs
   "flush" margin, drop the attribute and assert the CSS rule via
   raw-string check on the module. For pulse, drop the attribute and
   assert via raw-string check on `.pulseDot { animation: ... pulse … }`.
   Keep `data-state` and `data-animated` on `SseIndicator` — those are
   genuine runtime state.

8. **Add the `prefers-reduced-motion` guard** (addresses: a11y
   regression). One CSS rule:
   ```css
   @media (prefers-reduced-motion: reduce) {
     .pulseDot, .sse[data-animated='true'] { animation: none; }
   }
   ```
   Update "What We're NOT Doing" to remove the deferral.

9. **Add SidebarFooter parity reasoning to the Testing Strategy**
   (addresses: silent coverage drop). One paragraph stating that the
   `Reconnecting…`, `Reconnected — refreshing`, and version-label
   tests are intentionally removed because the new contract is
   `SseIndicator`'s `data-state`/`aria-label` and the version display
   is dropped by design. Add a negative assertion in `Topbar.test.tsx`
   that no `Reconnected — refreshing` text appears.

10. **Settle the dead-code question** (addresses: code-quality minor).
    After removing SidebarFooter, decide whether `useServerInfo()` and
    `DocEventsHandle.justReconnected` are deferred-but-retained or
    deleted. Add a "What We're NOT Doing" bullet either way so the
    next maintainer doesn't hunt for callers.

11. **Drop `role="img"` from Brand and `role="status"` from OriginPill**
    (addresses: standards/usability minors). Brand: mark the SVG
    `aria-hidden="true"` and let the visible text be the accessible
    name. OriginPill: keep the `aria-label` on a non-`status` element.

12. **Type `LABELS` as `Record<ConnectionState, string>`** (addresses:
    code-quality minor). Import `ConnectionState` from
    `../../api/use-doc-events`. The compiler enforces exhaustiveness.

13. **Disambiguate the sub-component co-location convention** (addresses:
    standards minor). Either promote each sub-component to its own
    sibling directory, or add a "Component organisation" note to
    Migration Notes documenting the nested-private-sub-component
    pattern with its trigger condition.

14. **Pick the token group up front** (addresses: standards minor).
    State in Phase 1 whether `--ac-topbar-h` lives in a new
    `LAYOUT_TOKENS` group or extends `SPACING_TOKENS`.

15. **Document the slug-prettification deferral against the AC line**
    (addresses: usability major). The work item AC mentions "pulling
    document titles from loader data where available" — add a sentence
    naming the follow-up story (likely 0036 or 0041) so the gap is
    tracked.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan introduces the Topbar with sound architectural
decomposition: a clear sub-component split with single responsibilities,
a route-loader contract that is clean and idiomatic TanStack Router, and
explicit bottom-up phasing that keeps each module independently testable.
Coupling to existing infrastructure (DocEventsContext, design tokens) is
appropriate. A few minor concerns exist around a leaky abstraction in the
breadcrumb crumb contract, the Topbar's responsibility for a transient
placeholder layout owned by 0034, and a consolidation point where SSE
state semantics shift between SidebarFooter and SseIndicator.

**Strengths**: Sub-component decomposition; loader-data contract decouples
breadcrumb from route-specific knowledge; DocEventsContext reused as-is;
window.location read deliberately scoped over extending useServerInfo;
empty data-slot divs as a clean seam with 0034; token-driven layout via
--ac-topbar-h; bottom-up phasing keeps blast radius narrow.

**Findings**:
- Minor (medium): Breadcrumbs assumes loaderData shape via cast — implicit contract.
- Minor (medium): SidebarFooter removal silently drops `justReconnected` post-reconnect transient.
- Minor (high): 0034 toggle slots are coupled by DOM position rather than composition seam.
- Minor (medium): First `window.location` consumer in a leaf component, diverges from injection pattern.
- Minor (high): `min-height: 100vh` on `.root` plus `min-height: calc(...)` on `.body` allows total layout > 100vh.
- Suggestion (medium): Cross-phase keyframe rename complicates phase rollback.

### Code Quality

**Summary**: Well-decomposed Topbar with four narrow sub-components, each
independently testable with minimal mocking. Phasing follows TDD, naming
is consistent, design largely sticks to existing house conventions. Main
concerns: Phase-5-then-rename-in-Phase-6 keyframe churn that telegraphs
incomplete upfront design, weak typing choices in snippets (loose
Record<string,string> indexing, `loaderData as` casts), latent dead code
left behind after SidebarFooter removal, and a subtle lifecycle gap where
OriginPill's host is read once at render with no documented invalidation
contract.

**Strengths**: Component decomposition is excellent; TDD ordering explicit;
tests follow house convention (data-* markers, role queries, mocked
hooks); sub-components kept directory-private; first-of-kind conventions
called out; token-driven CSS throughout; uniform loader contract.

**Findings**:
- Minor (high): Keyframe named-then-renamed across consecutive phases.
- Minor (high): ARIA label lookup loses ConnectionState narrowing (`Record<string, string>`).
- Minor (high): Type assertion bypasses route-data type safety.
- Minor (high): Dead code (`useServerInfo`, `justReconnected`) left after SidebarFooter removal.
- Minor (medium): OriginPill's host read is silently non-reactive (doc/code mismatch).
- Minor (medium): Two sequential array filters obscure the intent.
- Minor (medium): `data-empty="true"` hardcoded so test cannot fail.
- Suggestion (medium): `.shell` rename to `.body` is churn worth confirming.

### Test Coverage

**Summary**: Plan is structured around test-first development with clear
phase-level ordering and mirrors house mocking conventions. However:
breadcrumb tests rely on a bespoke router that duplicates production route
configuration (drift risk); several testability markers double as
production behaviour but only the SSE indicator's `data-animated` is
load-bearing; OriginPill's keyframe rename in Phase 6 leaves Phase 5 tests
with no real assertion catching the regression; SSE-related contract
assertions (colour-token mapping, footer parity) are missing.

**Strengths**: Test-first ordering explicit at every phase; gates repeated;
token parity for `--ac-topbar-h`; reuses `vi.mock` pattern from
SidebarFooter test; Phase 8 negative assertion confirming footer removal;
house conventions followed; sub-component tests scoped narrowly.

**Findings**:
- Major (high): Phase 5 OriginPill test cannot detect the Phase 6 keyframe rename regression.
- Major (high): Bespoke breadcrumb router duplicates production route config and will drift.
- Major (high): SidebarFooter `justReconnected` and version-display behaviour deleted without replacement coverage rationale.
- Minor (medium): SSE colour-token contract has no test.
- Minor (medium): `data-empty="true"` hardcoded, test asserts a literal that cannot change.
- Minor (medium): `data-flush="true"` is unconditional and primarily test-only state.
- Minor (high): `Object.defineProperty(window, 'location', ...)` per-test should fully restore the original.
- Minor (medium): Pending-state filter has no explicit test.
- Minor (high): Phase 1 asserts wrapper presence but never verifies column direction.
- Minor (medium): `/library/templates` chain coverage missing from Breadcrumbs Phase 4 test.

### Correctness

**Summary**: Well-structured TDD work with mostly sound logic. State
transitions for the SSE indicator cover all four ConnectionState values,
the breadcrumb pending-state filter is correct, and the loader contract
narrows correctly via parseParams. However: jsdom's window.location object
cannot be redefined naively (the plan's mocking technique may throw); the
Phase 1 'empty 56px row' claim is geometrically incorrect; the Phase 5→6
keyframe rename relies on no other tests asserting against pulseGreen
(true today, but undocumented as a constraint); the `data-flush`
attribute test pattern conflates structural assertions with style
assumptions that won't actually be exercised.

**Strengths**: Correctly identifies that `isMatch(m, 'loaderData.crumb')`
returns false for matches without loaderData; ConnectionState union
exhaustively covered by LABELS keys; loader cascade verified for
libraryDocRoute parent chain; phase ordering bottom-up; correctly places
DocEventsContext.Provider at outer wrapper from Phase 1.

**Findings**:
- Major (high): `Object.defineProperty(window, 'location', ...)` redefinition pattern unreliable in jsdom.
- Major (high): Phase 1 leaves no element occupying the 56px topbar row.
- Minor (high): Pending-state filter is redundant given isMatch's null guard.
- Minor (medium): Cast bypasses what isMatch could narrow; empty-string crumbs slip through.
- Minor (high): `data-animated={animated ? 'true' : undefined}` contract correct but undocumented.
- Minor (medium): `data-flush` and `data-empty` used as static proxies for CSS behaviour they don't drive.
- Minor (medium): URL-encoded slugs render verbatim in breadcrumbs (acknowledged scope, but worth noting).

### Standards

**Summary**: The plan demonstrates strong awareness of the existing token
system, test-style conventions, and the data-* attribute pattern already
in use. However, it introduces several first-of-kind conventions
(sub-components co-located rather than living in their own directories,
semantic markup choices for breadcrumbs, role attribute usage on the
brand) that warrant explicit documentation, and a few semantic-HTML/
WAI-ARIA choices conflict with established patterns or W3C
recommendations.

**Strengths**: Token naming follows established `--ac-*` prefix; tests
respect vi.mock convention; data-* test affordances consistent with
existing data-state/data-stage/etc taxonomy; co-located component+CSS+test
triples; `<header role="banner">` matches WAI-ARIA landmark guidance;
Phase 8 includes git grep cleanup checks.

**Findings**:
- Major (high): Breadcrumb markup does not follow the WAI-ARIA Breadcrumb pattern (no `<ol>`/`<li>`, no `aria-current`).
- Minor (medium): `role="img"` on a `<div>` wrapping inline SVG plus visible text is non-idiomatic.
- Minor (medium): Two `role="status"` live regions on persistent chrome may over-announce.
- Minor (high): Co-located non-exported sub-components depart from one-component-per-directory convention.
- Minor (medium): Token group naming for layout chrome height is unsettled (CHROME_TOKENS vs LAYOUT_TOKENS vs SPACING_TOKENS).
- Minor (medium): `data-flush` as a CSS-targeting hook is a new convention.
- Minor (high): `data-empty="true"` is hard-coded — semantically misleading once 0034 fills the slots.

### Usability

**Summary**: The plan is well-scoped with a clean phase-by-phase TDD
progression, sensible sub-component decomposition, and explicit deferral
of theme/font wiring. From a developer-ergonomics perspective the
data-slot contract is under-specified for 0034, the breadcrumb crumb
collection silently drops routes that forget a loader, and several
test-only data-* attributes leak into production markup. From an
end-user/UX angle, raw slugs as crumbs, `role="status"` on a perpetually-
pulsing origin pill, screen-reader announcement of SSE state changes,
and the unconditional opt-out of `prefers-reduced-motion` warrant
attention even within the stated scope.

**Strengths**: Clear bottom-up phase ordering; sub-component split keeps
mocks targeted; first-of-kind conventions called out; SseIndicator
aria-label is human-readable; DocEventsContext non-throwing default
recognised as a DX affordance; Migration Notes guides 0034 on slot
positions.

**Findings**:
- Major (high): Two infinite-pulse animations ship with no `prefers-reduced-motion` guard.
- Major (high): Future route authors who forget a loader silently drop out of the breadcrumb chain.
- Major (high): Raw slugs as crumb labels degrade end-user breadcrumb experience from day one.
- Minor (high): Two `role="status"` regions on persistent chrome create noisy or misleading announcements.
- Minor (medium): Test-only `data-*` attributes leak test concerns into production markup.
- Minor (high): Mid-plan keyframe rename creates diff-history friction.
- Minor (medium): `data-slot` contract for 0034 is under-specified.
- Minor (medium): OriginPill's `typeof window !== 'undefined'` guard signals SSR awareness that doesn't apply.

---

## Re-Review (Pass 2) — 2026-05-07

**Verdict:** REVISE

The edits resolve the overwhelming majority of the previous findings —
all six architecture findings, all eight code-quality findings, all
seven standards findings, all seven correctness findings, and seven of
eight usability findings, plus all ten test-coverage findings from the
first pass. The plan has tightened materially: the loader contract is
now self-correcting (`withCrumb` + dev-warn), the layout invariant is
explicit, the keyframe is named once and shared, slot collapse is
self-disengaging via `:empty`, and test-only `data-*` markers are gone
in favour of raw-CSS source assertions.

Four new major findings surfaced from the revised approach, all
addressable inside the plan:

1. The `withCrumb()` generic signature using bare `RouteOptions` will
   not infer `loaderData` as `CrumbLoaderData` against TanStack
   1.168.23 — Phase 5's "no `as` casts" success criterion may be
   unachievable without the parenthetical fallback the plan only
   hand-waves (correctness + code-quality).
2. The Breadcrumbs dev-warn `routeId.includes('Index')` heuristic
   never matches: TanStack route IDs derive from path segments
   (`/library/`, `/`), not variable names. The warn will fire on
   every navigation (`__root__` has no crumb) and miss the redirect
   routes it claims to skip (correctness + code-quality + usability).
3. The Phase 5 Breadcrumbs URL matrix uses the production `routeTree`
   but the test setup omits the fetch-stub plumbing (`fetchTypes`,
   `fetchTemplates`, `fetchLifecycleCluster`, etc.) that the existing
   `router.test.tsx` relies on (test-coverage).
4. Phase 2's `/` redirect-chain loader assertion uses `await
   router.load()` but `router.test.tsx:24-36` documents that
   multi-hop redirects need a `waitFor`-based helper (test-coverage).

There are also two minor non-trivial items: `LAYOUT_TOKENS` keys
include the `--` prefix where existing token groups use bare
kebab-case keys (will silently break parity), and breadcrumb ancestor
crumbs are non-interactive `<span>`s rather than navigable `<Link>`s
(partial deviation from the canonical WAI-ARIA Breadcrumb pattern).

The remaining concerns are surgical: each fits inside its phase and
together would tighten the plan to APPROVE. None invalidate the
restructured approach.

### Previously Identified Issues

#### Architecture (6/6 resolved)

- 🔵 Breadcrumbs assumes loaderData shape via cast — **Resolved**.
  `withCrumb` + `hasCrumb` type guard eliminates the cast.
- 🔵 SidebarFooter removal silently drops `justReconnected` —
  **Resolved**. Explicit retention rationale in "What We're NOT Doing".
- 🔵 0034 toggle slots coupled by DOM position — **Resolved (accepted
  with documentation)**. Slot contract documented under Migration Notes.
- 🔵 `window.location` consumer in leaf component — **Resolved**.
  `useOrigin()` hook in `src/api/use-origin.ts`.
- 🔵 Layout invariant `min-height: 100vh` + `min-height: calc(...)`
  allows >100vh — **Resolved**. `.body { flex: 1 }` replaces the
  conflicting `min-height` calc.
- 🔵 Cross-phase keyframe rename — **Resolved**. Shared `ac-pulse`
  declared once in Phase 1.

#### Code Quality (8/8 resolved)

- 🔵 Keyframe named-then-renamed — **Resolved**.
- 🔵 `LABELS: Record<string, string>` loses type narrowing — **Resolved**.
  Now `Record<ConnectionState, string>`.
- 🔵 Type assertion bypasses route-data type safety — **Resolved**.
  `hasCrumb` type guard with length check replaces the cast.
- 🔵 Dead code (`useServerInfo`, `justReconnected`) — **Resolved**.
  Explicit retention with rationale.
- 🔵 OriginPill host non-reactive doc/code mismatch — **Resolved**.
  `useState` lazy initialiser matches the documented contract.
- 🔵 Two sequential array filters — **Resolved**. Single `hasCrumb`
  predicate.
- 🔵 `data-empty="true"` hardcoded — **Resolved**. Replaced by
  `:empty` CSS pseudo-class.
- 🔵 `.shell` rename to `.body` is churn — **Resolved**. Plan now
  explicitly offers an opt-out ("If you want to preserve git-blame
  fidelity, instead keep `.shell` and add a new `.root` outer
  wrapper").

#### Test Coverage (10/10 resolved)

- 🟡 Phase 5 OriginPill keyframe-rename regression — **Resolved**.
  Raw-CSS source check on `OriginPill.module.css?raw` for `ac-pulse`
  binding.
- 🟡 Bespoke breadcrumb router drift — **Resolved**. Production
  `routeTree` import.
- 🟡 SidebarFooter behaviour parity — **Resolved**. New
  Testing Strategy subsection plus negative assertion in
  `Topbar.test.tsx`.
- 🔵 SSE colour-token contract — **Resolved**. Per-state raw-CSS
  source assertions.
- 🔵 `data-empty` hardcoded — **Resolved** (via `:empty`).
- 🔵 `data-flush` unconditional — **Resolved**. Raw-CSS source
  assertion replaces the marker.
- 🔵 `Object.defineProperty(window, 'location', ...)` restoration —
  **Resolved**. `useOrigin` hook + `vi.mock`; no jsdom global
  monkey-patching.
- 🔵 Pending-state filter test — **Resolved**. Phase 5 adds explicit
  pending-state mock unit and empty-list null-render unit.
- 🔵 Phase 1 column-direction never asserted — **Resolved**. Phase 8
  RootLayout test now asserts column direction via raw-CSS source
  check.
- 🔵 `/library/templates` chain coverage — **Resolved**. Phase 5 URL
  matrix now includes `/library/templates` and
  `/library/templates/adr`.

#### Correctness (7/7 resolved)

- 🟡 `Object.defineProperty(window, 'location', ...)` jsdom
  unreliability — **Resolved**. `useOrigin` + `vi.mock`.
- 🟡 Phase 1 leaves no element occupying 56px row — **Resolved**.
  Phase 1 is now token-only; layout restructure deferred to Phase 8
  alongside the Topbar mount.
- 🔵 Pending-state filter redundant — **Resolved**. Single
  `hasCrumb` predicate (which incorporates the null guard via
  `isMatch`).
- 🔵 Cast bypasses narrowing; empty-string crumbs slip through —
  **Resolved**. `hasCrumb` checks `length > 0`.
- 🔵 `data-animated` contract undocumented — **Resolved**. Inline
  comment with explicit "do not change to `String(animated)`"
  warning.
- 🔵 `data-flush`/`data-empty` static proxies — **Resolved** via
  `:empty` CSS and raw-CSS source assertions.
- 🔵 URL-encoded slugs render verbatim — **Resolved (acknowledged)**.
  Slug prettification deferred to 0036 with the AC quote inline.

#### Standards (7/7 resolved)

- 🟡 Breadcrumb markup not WAI-ARIA — **Resolved**. `<nav><ol><li>`
  structure with `aria-current="page"` on the trailing crumb.
- 🔵 `role="img"` on Brand wrapper — **Resolved**. Wrapper is plain
  `<div>`; SVG is `aria-hidden="true"`; visible text is the
  accessible name.
- 🔵 Two `role="status"` regions over-announce — **Resolved**. Both
  `OriginPill` and `SseIndicator` drop `role="status"`.
- 🔵 Co-located non-exported sub-components — **Resolved**. Each
  sub-component lives in its own sibling directory.
- 🔵 Token group naming unsettled — **Resolved**. New `LAYOUT_TOKENS`
  group named explicitly (subject to the new minor finding below
  about its key prefix).
- 🔵 `data-flush` as CSS-targeting hook — **Resolved**. Replaced by
  raw-CSS source assertion.
- 🔵 `data-empty="true"` semantically misleading — **Resolved**.
  Replaced by `:empty` CSS pseudo-class.

#### Usability (7/8 resolved)

- 🟡 No `prefers-reduced-motion` guard — **Resolved**. Override in
  Phase 1 (keyframe declaration is observable) and per-caller
  overrides in Phases 6 and 7, with raw-CSS source assertions.
- 🟡 Future route authors silently drop from breadcrumb chain —
  **Resolved**. `withCrumb` makes the contract explicit at the call
  site; dev-only `console.warn` for missing crumbs (subject to the
  new dev-warn heuristic finding below).
- 🟡 Raw slugs as crumb labels — **Partially resolved**. Deferral
  is now explicitly tracked against 0036 with the AC quote inline,
  but the user-facing experience on day one remains slug-cased
  (see new minor finding).
- 🔵 Two `role="status"` regions over-announce — **Resolved**.
- 🔵 Test-only `data-*` attributes leak — **Resolved**. Raw-CSS
  source assertions replace the markers.
- 🔵 Mid-plan keyframe rename — **Resolved**. Shared `ac-pulse`
  named once in Phase 1.
- 🔵 `data-slot` contract for 0034 under-specified — **Resolved**.
  Migration Notes subsection covers `:empty` rule, do-not-relocate
  rule, and explicit instruction that 0034 just renders content
  inside the existing div.
- 🔵 OriginPill `typeof window` guard signals SSR awareness —
  **Resolved**. SSR guard removed; SPA-only context noted.

### New Issues Introduced

#### Major

- 🟡 **Correctness + Code Quality**: `withCrumb()` generic signature
  using bare `RouteOptions` will not infer `loaderData` as
  `CrumbLoaderData` against TanStack 1.168.23. `RouteOptions` is a
  ~15-parameter generic and the bare reference resolves to the
  default-parameterised form, which strips the param relationship
  and prevents `loaderData` from inferring at the consumption site.
  Phase 5's "no `as` casts on `loaderData`" success criterion may
  be unachievable without the `declare module` augmentation that
  the plan only hints at parenthetically.
  **Location**: Phase 2: `withCrumb()` helper.
  **Suggestion**: Either prototype the generic against TanStack
  1.168.23 first and replace the snippet with the working version,
  or commit to the augmentation route up-front and specify which
  interface gets augmented.

- 🟡 **Correctness + Code Quality + Usability**: Dev-warn heuristic
  `m.routeId.includes('Index')` never matches. TanStack derives
  route IDs from path segments (`indexRoute.routeId === '/'`,
  `libraryIndexRoute.routeId === '/library/'`,
  `rootRoute.routeId === '__root__'`); the substring `Index` only
  appears in JS variable names. As written, the dev-warn fires on
  every navigation about `__root__` (because the root route has no
  loader) and fails to suppress the redirect routes it claims to
  skip.
  **Location**: Phase 5: Breadcrumbs.tsx — dev-warn block.
  **Suggestion**: Either (a) tag intentionally crumb-less routes
  with a sentinel such as `staticData: { crumbExempt: true }` and
  filter on that, (b) maintain a small `REDIRECT_ROUTE_IDS` set in
  `router.ts` that Breadcrumbs imports, or (c) drop the routeId
  heuristic and warn on every match where `m.status === 'success'`
  and `!isMatch(m, 'loaderData.crumb')`, while filtering out
  `__root__` explicitly via `rootRouteId` from `router-core`.

- 🟡 **Test Coverage**: Phase 5 Breadcrumbs URL matrix omits the
  fetch-stub plumbing the production `routeTree` requires.
  `router.test.tsx` stubs `fetchTypes`, `fetchTemplates`,
  `fetchTemplateDetail`, `fetchLifecycleCluster`, etc.; the
  Breadcrumbs test setup says nothing about these. As written,
  the new URL matrix will hit the network on first run and fail
  or flake.
  **Location**: Phase 5: Breadcrumbs — URL matrix using production
  `routeTree`.
  **Suggestion**: Add an explicit subsection listing the fetch
  stubs required for each URL case (or extract a shared
  `setupRouterFixtures()` helper from `router.test.tsx`).

- 🟡 **Test Coverage**: Phase 2's `/` redirect-chain loader assertion
  uses `await router.load()`, but `router.test.tsx:24-36`
  explicitly notes that multi-hop redirect chains require a
  polling `waitFor` helper.
  **Location**: Phase 2: Route loaders — `/` case.
  **Suggestion**: Reuse the existing `waitForPath(router,
  '/library/decisions')` pattern from `router.test.tsx` before
  reading `router.state.matches`. Or drop the `/` case and rely on
  the explicit `/library/decisions` and `/library/decisions/some-slug`
  cases — they cover the same loader-chain behaviour without the
  redirect-settling complication.

#### Minor

- 🔵 **Code Quality**: `withCrumb` implementation introduces
  `params: any` in the resolver callback and helper body. Fix:
  `params: Record<string, string>`.
  **Location**: Phase 2: `withCrumb()` implementation.

- 🔵 **Code Quality**: `hasCrumb` predicate's narrowed shape
  `{ id: string; loaderData: { crumb: string } }` discards every
  other Match field. A maintainer adding `m.params` access inside
  the `.map` will get a confusing "property does not exist on
  CrumbMatch" error. Use an intersection: `type CrumbMatch =
  ReturnType<typeof useMatches>[number] & { loaderData: { crumb:
  string } }`.
  **Location**: Phase 5: Breadcrumbs.tsx — `hasCrumb` definition.

- 🔵 **Standards**: `LAYOUT_TOKENS` keys include the `--` prefix
  (`'--ac-topbar-h': '56px'`), but every existing token group in
  `tokens.ts` uses unprefixed kebab-case keys
  (`SPACING_TOKENS = { 'sp-1': '4px', ... }`); the parity loop
  prepends `--` at the consumption site. As written, parity will
  silently break or require special-casing.
  **Location**: Phase 1, Section 3 — `LAYOUT_TOKENS` declaration.
  **Suggestion**: `export const LAYOUT_TOKENS = { 'ac-topbar-h':
  '56px' } as const`.

- 🔵 **Standards + Usability**: Non-current breadcrumb items
  rendered as plain `<span>`s rather than `<Link>`s. The WAI-ARIA
  Breadcrumb pattern's whole point is that ancestors are
  navigable. As written, the trail is a label, not a navigation
  surface.
  **Location**: Phase 5: Breadcrumbs.tsx — crumb rendering.
  **Suggestion**: Render ancestor crumbs as `<Link to={m.pathname}>`
  (TanStack Router `Link`); keep the trailing crumb as `<span
  aria-current="page">`. Match objects already carry `pathname`.
  Or, if intentional, name the gap explicitly under "What We're
  NOT Doing" so the deviation is tracked.

- 🔵 **Correctness**: Layout invariant phrased as "shell exactly
  100vh" but the CSS uses `min-height: 100vh` (correct
  implementation; only the documented invariant overstates the
  constraint).
  **Location**: Desired End State; Phase 8 RootLayout test.
  **Suggestion**: Reword to "shell uses `min-height: 100vh`; body
  uses `flex: 1` inside the column".

- 🔵 **Correctness**: Phase 5's CSS-source assertion that
  `Breadcrumbs.module.css?raw` contains `margin-left: 0` is
  selector-agnostic. A future refactor that moves the rule to
  `.crumb` without removing it from the file would still satisfy
  the substring check.
  **Location**: Phase 5: Breadcrumbs — CSS-source assertion.
  **Suggestion**: Use a multi-line regex like
  `/\.breadcrumbs\s*\{[^}]*margin-left:\s*0/`.

- 🔵 **Test Coverage**: Phase 4 `useOrigin` rerender same-instance
  assertion is a weak guard for the read-once contract — string
  interning makes the assertion pass even if the implementation
  reads on every render.
  **Location**: Phase 4: `useOrigin` test.
  **Suggestion**: Spy on `window.location.host` access and assert
  the getter is called exactly once across multiple `rerender()`s,
  or mutate `window.location.host` between renders and assert the
  hook still returns the original value.

- 🔵 **Test Coverage**: `withCrumb()` helper has no direct unit
  test — exercised only through `router.test.tsx` and the
  Breadcrumbs component test.
  **Location**: Phase 2: withCrumb helper.
  **Suggestion**: Add a small `withCrumb.test.ts` covering both
  overloads in isolation.

- 🔵 **Test Coverage**: `Topbar.test.tsx` negative assertion for
  "Reconnected — refreshing" only proves absence under one mocked
  state. The original SidebarFooter logic emitted that text only
  when `connectionState === 'open' && justReconnected === true`,
  so the assertion would have passed even if the Topbar still
  rendered the SidebarFooter logic.
  **Location**: Phase 8: Topbar.test.tsx negative assertion.
  **Suggestion**: Add a render case where `useDocEventsContext`
  returns `{ connectionState: 'open', justReconnected: true }`
  and assert `queryByText(/Reconnected — refreshing/)` is still
  null.

- 🔵 **Usability**: Day-one breadcrumb shows raw slugs verbatim.
  The doc-type segment (`decisions`, `templates`) is a known
  finite set narrowed by `parseParams`; title-casing it via a
  one-line `String.replace` in the `withCrumb` resolver is a
  cheap stop-gap that does not require loader changes.
  **Location**: Desired End State; What We're NOT Doing.
  **Suggestion**: Apply title-casing to the doc-type segment now;
  leave full slug prettification to 0036.

#### Suggestions

- 🔵 **Architecture**: Shared `ac-pulse` keyframe is global with
  per-caller `prefers-reduced-motion` overrides. Future callers
  that forget the override degrade accessibility silently.
  Consider colocating a single override in `global.css` with a
  utility class, or document the per-caller-override rule
  explicitly.
- 🔵 **Code Quality**: Demote the `withCrumb` overload-with-generics
  block to "optional, if it composes" rather than the lead
  example, biasing toward the simpler `Record<string, string>`
  params shape + augmentation.
- 🔵 **Usability**: The `withCrumb(crumbOrResolver, options)`
  signature reads awkwardly for the param-derived case — the
  resolver references `params.x` before the reader sees the path
  declaring it. Consider flipping to `withCrumb(options,
  crumbOrResolver)` or a builder/options-key shape.

### Assessment

The plan has improved substantially. All previous major findings
across all six lenses are resolved (the only "partially resolved"
item is the slug-prettification UX deferral, which is now
appropriately tracked). The four new majors are all narrow,
implementation-time issues that surfaced from the new approach:

- The `withCrumb` generics and dev-warn heuristic are
  implementation correctness concerns that could be settled with
  a few hours of prototyping against TanStack 1.168.23 or by
  picking the explicit-list path the plan already gestures at.
- The two test-coverage majors (Breadcrumbs fetch stubs, Phase 2
  multi-hop redirect) are about specifying the test fixtures
  more precisely — both have well-known patterns in
  `router.test.tsx` to copy.
- The minor findings are mostly cosmetic (LAYOUT_TOKENS key
  prefix, CSS regex specificity, type-guard intersection,
  rerender same-instance assertion).

A second round of edits addressing the four majors and the
LAYOUT_TOKENS key prefix would put the plan at APPROVE. The
restructured approach is sound; what remains is precision.

---

## Re-Review (Pass 3) — 2026-05-08

**Verdict:** REVISE

The pass-3 edits resolve all but two of the pass-2 findings cleanly:
the `withCrumb` overload-with-generics is gone, `params: any` is
gone, the dev-warn now uses `rootRouteId` from
`@tanstack/router-core`, the `hasCrumb` shape uses an intersection
that preserves Match fields, ancestor crumbs render as TanStack
`<Link>`, the `LAYOUT_TOKENS` keys are unprefixed kebab-case, the
`useOrigin` test installs a getter spy with call-count and
mutation-stability assertions, the Topbar negative assertion is
driven with `justReconnected: true`, the layout invariant is
rephrased, and the Phase 2 redirect-chain cases are dropped in
favour of `waitForPath` for the remaining cases.

The author's two intentional non-fixes (the `withCrumb(crumb,
options)` argument order and the day-one slug crumbs) remain by
choice and are defensible.

Three new majors surface from this round, all narrow and
implementation-time:

1. 🟡 **Correctness**: `<Link to={m.pathname}>` will fail typecheck.
   TanStack Router's `Link` `to` prop is a constrained literal type
   (`ConstrainLiteral<...>`), not `string`. Every existing `Link`
   consumer in this codebase passes a literal path. Phase 5's
   typecheck success criterion is unachievable as snippeted; the
   implementer needs an explicit cast strategy, a path-union
   assertion, or a switch to `<a href>` + `router.navigate`.

2. 🟡 **Correctness**: The `withCrumb` unit test snippets call
   `loader()` and `loader({ params: { x: 'foo' } })`, but TanStack's
   `LoaderFnContext` requires `abortController`, `cause`, `context`,
   `deps`, `location`, `navigate`, `parentMatchPromise`, `preload`,
   `route`, `params` as required fields. The test will fail
   typecheck unless cast or the helper is refactored to expose a
   thinner pure `resolveCrumb()` function.

3. 🟡 **Test Coverage**: The shared `setupRouterFixtures()` helper
   does not enumerate which fetch stubs it installs — the plan
   names `fetchTypes`, `fetchTemplates`, `fetchTemplateDetail`,
   `fetchLifecycleCluster`, `etc.` But `LibraryDocView` (the
   component mounted by `/library/decisions/foo-slug`) calls
   `fetchDocs(type)` via React Query, which is not in the list.
   The Breadcrumbs URL matrix will likely hit the network on first
   run for that case (and possibly others) until the stub set is
   spelled out completely.

These are the three remaining blockers. A fourth round of edits
addressing them — plus the three new minors with high confidence
(focus-visible style on `.link`, the silent loader override in
`withCrumb`, and the under-specified `declare module` augmentation)
— would put the plan in a clean APPROVE state.

The other minors are surgical and could reasonably be addressed
during implementation if accepted as such by the author.

### Previously Identified Issues (Pass 2)

#### Architecture (2/2 resolved)

- 🔵 **Suggestion** — Shared `ac-pulse` keyframe is global with
  per-caller reduced-motion duplication. **Resolved (via documentation).**
  Migration Notes "Future pulse callers" explicitly states the
  per-caller-override rule.
- 🔵 **Suggestion** — Cross-phase keyframe rename complicates phase
  rollback. **Resolved**. `withCrumb` overload-with-generics
  replaced with single `(string | CrumbResolver, options)` signature
  + deferred `declare module` augmentation.

#### Code Quality (4/4 resolved)

- 🔵 `withCrumb` overload constraint will not compile against
  TanStack 1.168.23 — **Resolved**. Single-signature design
  eliminates the issue.
- 🔵 `params: any` casts — **Resolved**. Now `Record<string, string>`.
- 🔵 Dev-warn `routeId.includes('Index')` heuristic — **Resolved**.
  Now uses `rootRouteId` from `@tanstack/router-core`.
- 🔵 `hasCrumb` shape discards Match fields — **Resolved**.
  Intersection type preserves all Match fields.

#### Test Coverage (6/6 resolved)

- 🟡 Breadcrumbs URL matrix omits fetch stubs — **Partially resolved**
  (helper extracted but not enumerated; see new major below).
- 🟡 Multi-hop redirect under `router.load()` — **Resolved**.
  `waitForPath` used; redirect-chain cases dropped.
- 🔵 Dev-warn pinned to uncertain heuristic — **Resolved**. Three
  cases cover the production contract.
- 🔵 useOrigin rerender same-instance assertion weak — **Resolved**.
  Getter spy with call-count + stability-under-mutation cases.
- 🔵 `withCrumb` only indirectly exercised — **Resolved**. New
  direct unit suite (Phase 2 §1b).
- 🔵 Topbar negative assertion under one mocked state only —
  **Resolved**. Driven with `justReconnected: true`.

#### Correctness (4/4 resolved)

- 🟡 `withCrumb` generics will not infer `loaderData` — **Resolved
  (with caveat)**. Single-signature design + deferred augmentation;
  see new code-quality minor about the Phase 5 success criterion
  being conditional on the augmentation working.
- 🟡 Dev-warn heuristic uses variable names — **Resolved**.
  `rootRouteId` (verified exported as `'__root__'` in
  `@tanstack/router-core` 1.168.23).
- 🔵 Layout invariant `min-height: 100vh`, not exactly 100vh —
  **Resolved**. Rephrased.
- 🔵 Raw-CSS substring check selector-agnostic — **Resolved**.
  Selector-bound regex.

#### Standards (2/2 resolved)

- 🔵 `LAYOUT_TOKENS` keys with `--` prefix — **Resolved**. Now
  unprefixed kebab-case.
- 🔵 Non-current breadcrumb items as `<span>` — **Resolved**.
  Ancestors render as `<Link to={m.pathname}>`.

#### Usability (2/4 resolved; 2 unchanged by author choice)

- 🔵 Dev-warn redirect-route heuristic fragile — **Resolved**.
  `rootRouteId` filter.
- 🔵 `aria-current` on non-interactive span — **Resolved**.
  Ancestors are now `<Link>`.
- 🔵 `withCrumb(crumb, options)` argument order awkward —
  **Unchanged (by author choice)**. Argument order kept as-is.
- 🔵 Day-one raw slug crumbs — **Unchanged (by author choice)**.
  Slug stop-gap not applied; deferral to 0036 retained.

### New Issues Introduced

#### Major

- 🟡 **Correctness**: `<Link to={m.pathname}>` will fail typecheck.
  TanStack `Link`'s `to` is a constrained literal type, not
  `string`. `Match.pathname` is typed `string`. Every existing
  `<Link>` in this codebase passes a literal (`WorkItemCard.tsx:38`).
  Phase 5's typecheck criterion is unachievable as snippeted.
  **Location**: Phase 5: Breadcrumbs.tsx — ancestor link rendering.
  **Suggestion**: Pick one of (a) cast at the call site
  `<Link to={m.pathname as string}>` plus an `as any` for the
  prop generics, (b) use `router.navigate({ to: m.pathname })` from
  a regular `<a href={m.pathname} onClick={...}>` (TanStack's
  imperative navigate accepts `string`), or (c) export a thin
  `LinkToPath` wrapper from a small helper that accepts
  `pathname: string` and forwards. Update the snippet so the
  implementer doesn't hit this wall mid-phase.

- 🟡 **Correctness**: `withCrumb` unit test calls `loader()` /
  `loader({ params })` with insufficient args for TanStack's
  `LoaderFnContext` (which requires `abortController`, `cause`,
  `context`, `deps`, `location`, `navigate`, `parentMatchPromise`,
  `preload`, `route`, `params` as required fields). The test will
  fail typecheck.
  **Location**: Phase 2 §1b: `router-helpers.test.ts`.
  **Suggestion**: Either (a) extract `resolveCrumb(crumbOrResolver,
  params)` as a separately-exported pure function and unit-test
  that instead of the loader, (b) cast the loader to a thinner
  signature for the unit test invocations, or (c) build a full
  `LoaderFnContext`-shaped object via `as any`. Option (a) is
  cleanest.

- 🟡 **Test Coverage**: `setupRouterFixtures()` does not enumerate
  the fetch stubs the URL matrix actually requires.
  `LibraryDocView` calls `fetchDocs(type)` for `/library/decisions/
  foo-slug`; this is not in the listed stub set. Without it, the
  Breadcrumbs test will hit the network or flake on first render.
  **Location**: Phase 5 §0: `src/test/router-fixtures.ts`.
  **Suggestion**: Replace the trailing "etc." with an enumerated
  list of every fetch the production routes invoke for each URL
  case in the Breadcrumbs matrix. Spell out: `fetchTypes`,
  `fetchTemplates`, `fetchTemplateDetail`, `fetchDocs`,
  `fetchLifecycleClusters`, `fetchLifecycleCluster`, plus any
  kanban dependencies. State that `setupRouterFixtures()` installs
  all unconditionally with sensible defaults, and per-test
  overrides only what each case asserts on.

#### Minor

- 🔵 **Architecture + Code Quality**: `declare module` augmentation
  strategy deferred to implementer with a fallback that drops
  Phase 5's "no `as` casts" success criterion. The plan offers
  two augmentation candidates plus a fallback, leaving the type
  contract underspecified at planning time.
  **Suggestion**: Either prototype each augmentation candidate
  before Phase 2 begins and lock the choice into the plan, or
  commit explicitly to "no augmentation; `hasCrumb` is the single
  narrowing site" and remove the candidate list.

- 🔵 **Code Quality**: `Parameters<typeof createRoute>[0]` strips
  per-route generic narrowing. The helper is type-shaped at the
  input but typed-erased at the output.
  **Suggestion**: Either make `withCrumb` itself generic mirroring
  `createRoute`'s parameters, or be explicit that the helper's
  only contribution is the loader assignment with type-narrowing
  happening exclusively at `hasCrumb`.

- 🔵 **Code Quality**: `withCrumb` silently overwrites a caller's
  `loader` via `{ ...options, loader }` spread. Today no route has
  its own loader so the override is invisible, but a future
  contributor extending a route with data fetching (likely 0036's
  loader-fetched titles) loses their loader silently.
  **Suggestion**: Either compose loaders (call the caller's first
  and merge `{ crumb }` into the result), or reject `options.loader`
  at the type level via `Omit<..., 'loader'>`.

- 🔵 **Code Quality**: `src/router-helpers.test.ts` collides
  semantically with the existing `src/test/router-helpers.tsx`
  (memory-router test helper) and the new
  `src/test/router-fixtures.ts`. Three nearly-identical names.
  **Suggestion**: Name the new test `src/router-with-crumb.test.ts`
  or co-locate with `withCrumb` in `src/router.ts`. Optionally
  rename `src/test/router-helpers.tsx` to `src/test/memory-router.tsx`.

- 🔵 **Code Quality**: `Match = ReturnType<typeof useMatches>[number]`
  is fragile against TanStack's `useMatches` overloads (the
  `select` callback variant). Resolves to the default-instantiated
  shape today; minor TanStack updates could refine the union and
  break the intersection.
  **Suggestion**: Import `MakeRouteMatch` (or whichever canonical
  match type TanStack exports) and alias directly.

- 🔵 **Test Coverage**: The plan asserts `host` on jsdom's Location
  is per-property configurable but offers no anchor to verify.
  The codebase has zero existing precedent for `window.location`
  mutation. If jsdom's installed version makes `host`
  non-configurable, the Phase 4 test will throw `TypeError:
  Cannot redefine property: host`.
  **Suggestion**: Either prototype the `Object.defineProperty(
  window.location, 'host', { configurable: true, get })` pattern
  against the project's jsdom version once before committing the
  plan, or restructure `useOrigin` to take an injected reader
  (`useOrigin(read = () => window.location.host)`) so the test
  mocks the reader argument and avoids `window.location` entirely.

- 🔵 **Test Coverage**: Dev-warn unit covers three of five status
  values. `pending` and `error` are unverified. A future refactor
  that drops the `m.status === 'success'` check would silently
  start warning on every pending navigation.
  **Suggestion**: Add two more cases to the dev-warn unit
  (`status: 'pending'` and `status: 'error'`); assert no warn
  fires.

- 🔵 **Test Coverage**: `withCrumb` unit invokes the loader with
  partial args. The test does not assert that the helper accepts
  the full TanStack args shape (i.e. doesn't filter or destructure
  before passing through to the resolver).
  **Suggestion**: Pass a richer args object to assert the helper
  passes through unchanged; or extract `resolveCrumb()` and unit-
  test that pure function instead.

- 🔵 **Test Coverage**: Plan does not specify the query method for
  ancestor-link `href` assertions. `screen.getByRole('link', {
  name: '<text>' }).getAttribute('href')` is the canonical pattern.
  **Suggestion**: Specify the query method in Phase 5's test
  description so the implementer doesn't reach for
  `container.querySelector('a[href=...]')` (couples to DOM) or
  `getByText` (would also match the trailing `<span>`).

- 🔵 **Test Coverage**: Refactoring `router.test.tsx` to consume
  `setupRouterFixtures()` risks regressing previously-stable
  router test behaviour by hoisting per-test stubs into a
  global `beforeEach`.
  **Suggestion**: Make `setupRouterFixtures()` install only the
  unconditionally-needed stubs (mirror the existing
  `beforeEach` set in `router.test.tsx:54-60`); keep per-test
  stubs as explicit per-test setup.

- 🔵 **Correctness**: useOrigin test "change the spy to return
  'changed.example'" is ambiguous between `mockReturnValue` (mutates
  the existing spy) and re-issuing `Object.defineProperty` (replaces
  the descriptor). The saved-original-descriptor restoration logic
  suggests interpretation 1 is intended.
  **Suggestion**: Spell out: "call `(spy as Mock).mockReturnValue(
  'changed.example')` on the existing spy — do not redefine the
  property".

- 🔵 **Correctness**: `CrumbMatch = Match & { loaderData: { crumb:
  string } }` may collapse to `never` for branches of the Match
  union whose `loaderData` is incompatible with `{ crumb: string }`.
  Today this is benign but fragile if a future route adds a loader
  returning a different shape.
  **Suggestion**: Either define `CrumbMatch = Omit<Match,
  'loaderData'> & { loaderData: { crumb: string } }`, or add a
  comment in `Breadcrumbs.tsx` noting that adding non-crumb loaders
  elsewhere requires revisiting the type guard.

- 🔵 **Correctness**: Dev-warn condition skips `notFound` and
  `error` status matches that genuinely lost a `withCrumb` call —
  a route author whose loader throws sees no breadcrumb AND no
  warning.
  **Suggestion**: Either explicitly enumerate exclusions
  (`m.status !== 'redirected' && m.status !== 'pending' &&
  m.routeId !== rootRouteId`), or add an extra warn for
  `status: 'error'` matches.

- 🔵 **Usability**: Breadcrumb links have no `:focus-visible` style
  defined. Keyboard users get no reliable indicator of which crumb
  is focused as they tab through persistent chrome.
  **Suggestion**: Add `.link:focus-visible { outline: 2px solid
  var(--ac-accent); outline-offset: 2px; border-radius: 2px; }`
  and a raw-CSS source assertion that `:focus-visible` is declared
  on `.link`.

- 🔵 **Usability**: Dev-warn correctness depends on an unverified
  claim that TanStack marks redirect matches with
  `status: 'redirected'`. If TanStack 1.168.23 actually transitions
  redirect-source matches through `status: 'success'`, the dev-warn
  fires on every navigation through `/`, `/library`, `/lifecycle/`.
  **Suggestion**: Either cite the specific TanStack source/type
  documenting the `'redirected'` status, add a fourth test case
  driving production redirect routes through the real routeTree
  to assert no warn fires, or maintain an explicit
  `REDIRECT_ROUTE_IDS` set as a fallback.

- 🔵 **Usability**: Breadcrumb link colour is `--ac-fg-faint`, which
  signals "disabled" rather than "clickable". Resting state is
  visually identical to inert metadata text.
  **Suggestion**: Either underline links by default and remove on
  hover (inverse of current rule), or use `--ac-fg` resting with
  hover transition to `--ac-accent`.

#### Suggestions

- 🔵 **Code Quality**: Slot identity is dual-tracked across
  `className={styles.slot}` and `data-slot="..."`. Optional: a
  small `<TopbarSlot name="theme-toggle">{children}</TopbarSlot>`
  wrapper would unify the two.

### Assessment

The plan continues to improve. Pass-3 resolves nearly all pass-2
findings cleanly; the three new majors are narrow,
implementation-time issues — two are TypeScript-against-TanStack
typing concerns (`Link to=` and the loader unit test args), and
one is a documentation completeness issue (fetch-stub enumeration).
None invalidate the design, and all are addressable inside their
phases.

The pattern of "successive refinement" is paying off — each pass
shrinks the surface of unresolved concerns, and the remaining
items in pass 3 are increasingly cosmetic or implementation-detail-
level. A fourth pass focused on the three majors plus the
high-confidence minors (focus-visible, silent loader override,
declare module commitment) would put the plan firmly at APPROVE.

---

## Final Disposition (Pass 4) — 2026-05-08

**Verdict:** APPROVE

The author applied a fourth round of edits addressing the three
pass-3 majors and the high-confidence minors:

- `<Link to={m.pathname}>` replaced with `<a href> + onClick →
  router.navigate({ to: ... as never })`. Modifier-click falls
  through to the native href; plain left-click is intercepted for
  SPA navigation. Click-handler unit added covering both paths.
- `withCrumb` unit-test issue resolved by extracting a pure
  `resolveCrumb(crumbOrResolver, params)` function. Unit test
  exercises that pure function directly — no `LoaderFnContext`
  required, no casts. Test file renamed to
  `router-with-crumb.test.ts`.
- `setupRouterFixtures()` fetch stubs explicitly enumerated per
  URL in a Phase 5 §0 table; helper installs only unconditional
  stubs, per-test stubs remain explicit.
- `declare module` augmentation strategy: plan commits to "no
  augmentation; `hasCrumb` is the single narrowing site". Phase 5
  success criterion updated.
- `withCrumb` options typed `Omit<..., 'loader'>` so a caller
  passing their own loader gets a compile error.
- `:focus-visible` style added to `.link` with raw-CSS source
  assertion.
- `.link` resting colour brightened to `--ac-fg` with a subtle
  `--ac-fg-faint` underline so the affordance is visible.
- `CrumbMatch` defined via `Omit<Match, 'loaderData'> & { ... }`
  to avoid `never`-collapse.
- Dev-warn unit covers all five `m.status` values plus the
  rootRouteId exemption.
- `useOrigin` test mutation step disambiguated; jsdom
  configurability assumption gets a one-liner verification step
  with an injected-reader fallback if needed.
- Breadcrumb test uses `getByRole('link', { name })` for href
  assertions.
- Test-file naming collision avoided (`router-with-crumb.test.ts`).

The remaining unresolved minors are either lower-impact
(`Parameters<typeof createRoute>[0]` type erasure is now expected
behaviour given the no-augmentation choice; `Match` alias
fragility is marginal) or design suggestions that would expand
scope (slot dual-tracking via `<TopbarSlot>` wrapper).

The plan is approved for implementation.

### Review history

- **Pass 1** — 9 majors across 6 lenses; restructure recommended.
- **Pass 2** — Plan substantially restructured; 4 narrow new
  majors surfaced from the new approach.
- **Pass 3** — 3 narrow new majors surfaced from second-round
  edits, all TypeScript-against-TanStack typing concerns plus
  fetch-stub enumeration.
- **Pass 4** — All three majors resolved via the `<a href +
  router.navigate>` pattern, the `resolveCrumb` pure-function
  extraction, and explicit per-URL fetch-stub enumeration.

The plan is now in a clean, implementable state.
