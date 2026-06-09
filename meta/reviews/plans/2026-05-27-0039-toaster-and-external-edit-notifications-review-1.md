---
date: "2026-05-30T00:00:00+01:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-27-0039-toaster-and-external-edit-notifications"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 3
status: complete
id: "2026-05-27-0039-toaster-and-external-edit-notifications-review-1"
title: "2026-05-27-0039-toaster-and-external-edit-notifications-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-30T00:00:00+01:00"
last_updated_by: Toby Clemson
---

## Plan Review: Toaster and External-Edit Notifications

**Verdict:** REVISE

The plan is genuinely strong: it is well-grounded in the real codebase, composes
on mature SSE/self-cause/React-Query infrastructure rather than fighting it, and
its two most load-bearing correctness claims (the subscriber fires *before* the
self-cause drop, and the registry is a shared singleton) were verified against
the actual code and hold up. The functional-core/imperative-shell separation,
TDD sequencing, and convention-matching are all commendable. However, nine major
findings cluster around three fixable areas — an under-specified Phase 5
integration-test mechanism that cannot actually exercise the two hardest ACs as
written, a timer lifecycle with no unmount cleanup, and a set of end-user
accessibility gaps (auto-dismiss timing, per-toast live regions, unbounded
stack) that sit in tension with the plan's explicit "not doing" exclusions.
None are structural; all can be resolved with targeted edits before
implementation.

### Cross-Cutting Themes

- **Phase 4 resolver diverges from `LibraryDocView`** (flagged by: architecture, code-quality) — The new `useActiveDocRelPath` reads `useParams` only, but `LibraryDocView` resolves identity from `propType ?? params.type` and also surfaces query errors. The "single source of truth" framing is therefore illusory: the refactor can't fully replace the view's logic, leaving two near-duplicate, drift-prone resolution paths.
- **Phase 5 integration test cannot deliver an event through the real dispatch path** (flagged by: test-coverage, with correctness echoing the path-format dependency) — `RootLayout` instantiates the production `useDocEvents` singleton with no EventSource seam, so "deliver an event AND exercise real `dispatchSseEvent`" is self-contradictory. This is where the two highest-risk ACs (real-resolution positive correlation, content-refresh) are supposed to be proven.
- **Toast accessibility: per-toast `aria-live` + unbounded stack + 5s hard dismiss** (flagged by: usability, standards) — Live-region semantics on each dynamically-inserted card (rather than a stable container), no stack cap, and a fixed 5s auto-dismiss with explicitly no pause-on-hover together create WCAG concerns (2.2.1 timing; unreliable/garbled SR announcements during edit bursts).
- **Hook API ergonomics & naming** (flagged by: standards, usability) — The owning/consumer naming inverts the house convention (`useToastDispatcher` owns, `useToast` consumes — the opposite of `useTheme`/`useThemeContext`), the "call once" contract is documented-not-enforced, and positional `showToast(heading, message)` invites silent argument transposition.

### Tradeoff Analysis

- **Accessibility vs YAGNI/minimalism**: Usability wants pause-on-hover, a stack cap, and an options-object API; the plan's "What We're NOT Doing" section deliberately excludes pause-on-hover and queue limits as scope control. Recommendation: accept the small pause-on-hover + stack-cap additions (they're cheap, the `autoDismissMs` seam already makes them testable, and they address a real WCAG 2.2.1 concern), but treat the options-object API as optional — the single current consumer makes it low-urgency.
- **Phase 4 extraction depth vs scope**: Architecture/code-quality want either a fully parameterised resolver `LibraryDocView` can consume, or honest "scoped resolver" framing. The plan already hedges ("if it adds risk, the hook stands alone"). Recommendation: pick one explicitly rather than leaving it as an implementation-time coin flip — the cheapest honest option is to drop the "single source of truth" claim and scope the hook as params-only for the RootLayout subscriber.

### Findings

#### Major

- 🟡 **Test Coverage**: Phase 5 integration test has no seam to deliver an SSE event through the real dispatch path
  **Location**: Phase 5: RootLayout wiring + end-to-end integration (Test-First)
  `RootLayout` calls the production singleton `useDocEvents` directly and sets `DocEventsContext` to it, so an outer test provider is shadowed and there is no EventSource factory injection point. Capturing the `subscribe` listener requires the real `onmessage`, but that is also what runs `dispatchSseEvent` — you cannot get one without the other under the singleton, so the "real invalidation + delivered event" promise is internally inconsistent.

- 🟡 **Test Coverage**: Phase 5 self-cause and timing tests rely on the module-singleton registry with no reset/isolation
  **Location**: Phase 5: Test-First (self-caused negative case)
  The plan mutates the shared `defaultSelfCauseRegistry` singleton (real `Date.now()` TTL, no `reset()` between tests) while other suites in the same run also touch it; fake timers won't advance its clock unless `Date.now` is mocked. Cross-test pollution and timer/clock divergence make the self-cause and auto-dismiss cases order-dependent.

- 🟡 **Test Coverage**: Mixing fake timers with async React Query settling in one test is a known flake source
  **Location**: Phase 5: Test-First (content-refresh AC + auto-dismiss in context)
  Fake timers (4s/5.5s auto-dismiss) combined with React Query async invalidation and `waitFor` settling interact poorly unless timers are explicitly configured; the plan doesn't say how the two regimes are kept separate, risking hangs or spurious passes on the content-refresh AC.

- 🟡 **Architecture**: Resolver extraction diverges from LibraryDocView's prop-or-param contract, so it is not a true single source of truth
  **Location**: Phase 4: Shared active-doc relPath resolver
  `LibraryDocView` resolves from `propType ?? params.type` and is rendered with props in the router and tests, whereas the new hook reads `useParams` only. The claimed shared-source-of-truth refactor can't replace the view's logic without behaviour change, so two divergent copies of the resolution rule will drift — and a future doc-ID migration must touch both.

- 🟡 **Architecture**: Two independent 5s windows (toast auto-dismiss, self-cause TTL) are coincidentally equal with no shared constant
  **Location**: Phase 1 / Phase 3: auto-dismiss timing vs self-cause TTL coupling
  `TOAST_AUTO_DISMISS_MS` (5s) and the self-cause registry TTL (5s, `self-cause.ts:16`) are treated as unrelated, but suppression depends on event arrival landing inside the TTL window relative to the local write. The two temporally-coupled constants live in different modules with no documented relationship; tuning either could silently change behaviour.

- 🟡 **Code Quality**: Timer Map has no unmount cleanup — leaked timers fire setToasts after unmount
  **Location**: Phase 1, Section 1: Toast model + hook (useToastDispatcher)
  `useToastDispatcher` stores per-toast timers in `timersRef` but provides no `useEffect` cleanup on unmount, unlike every other transient-timer hook in the codebase (`use-deferred-fetching-hint.ts:27-29`, `use-doc-events.ts:234-237`). Pending callbacks fire `setToasts` after unmount — the exact leak the Performance Considerations section claims to avoid, and a source of noisy/flaky Phase 1 tests.

- 🟡 **Standards**: Owning/consumer hook naming inverts the established convention
  **Location**: Phase 1: Toast context, stack, and dispatcher (use-toast.ts)
  House convention: owning hook = bare name (`useTheme`), consumer = `...Context` suffix (`useThemeContext`). The plan inverts this — consumer is `useToast()`, owning is `useToastDispatcher()` (a name used nowhere else), with no `...Context` consumer. The work item *does* mandate `useToast()` as the dispatcher-facing API, so the bare name is constrained; the departure should be made deliberate and signposted.

- 🟡 **Usability**: 5s auto-dismiss with no pause-on-hover risks WCAG 2.2.1 timing failure
  **Location**: Phase 2 (Toaster) + "What We're NOT Doing" (no pause-on-hover)
  A backtick-wrapped relative path + verb on a fixed 5s timer with no pause/extend mechanism is marginal for sighted users to read a long path and impossible for SR users to replay. This is a WCAG 2.2.1 (Timing Adjustable) concern; pause-on-hover/focus-within is a small, already-testable (`autoDismissMs` seam) addition.

- 🟡 **Usability**: Per-toast live regions plus unbounded stack can flood/garble screen-reader announcements
  **Location**: Phase 2: aria-live placement; Phase 1: unbounded Toast[] stack
  Each card is its own dynamically-created `role="status" aria-live="polite"` region, and the stack has no cap. Live regions announce most reliably when the container pre-exists; per-toast regions plus rapid edit bursts can pile up, drop, or garble announcements, and the viewport can grow without bound.

#### Minor

- 🔵 **Test Coverage**: Route change mid-subscription is asserted nowhere despite being the central ref-pattern claim
  **Location**: Phase 3: External-edit subscriber (Test-First)
  No test rerenders with a changed `relPath` then fires an event to confirm the single listener reads the new value. A regression moving the `relPath` read into the effect closure (stale capture / re-subscribe) would pass every planned test.

- 🔵 **Test Coverage**: Undefined etag (self-cause never matches) edge case is untested
  **Location**: Phase 3: External-edit subscriber (Test-First)
  `etag` is optional and `has(undefined)` returns `false`, so a no-etag event always toasts. Phase 3 covers "in registry" / "not in registry" but not the `etag: undefined` boundary.

- 🔵 **Test Coverage**: Stack-ordering and concurrent-timer independence under-specified for the stacking AC
  **Location**: Phase 1: Toast context, stack, and dispatcher (Test-First)
  Automated cases assert dismissing one of two leaves the other, but not that two staggered toasts each auto-dismiss at their own +5s. A shared/reset timer bug would pass.

- 🔵 **Test Coverage**: Icon-render AC leans on a brittle data-testid rather than a semantic hook
  **Location**: Phase 2: Toaster presentational component (Test-First)
  The icon AC checks `data-testid="ac-toaster-icon"` on an `aria-hidden` decorative SVG; a testid rename breaks it while an empty SVG carrying the testid passes. Consider also asserting `card.querySelector('svg')`.

- 🔵 **Code Quality**: Resolver diverges from LibraryDocView, so the DRY "single source of truth" goal is only partially met
  **Location**: Phase 4, Section 2: Refactor LibraryDocView to consume it
  The snippet omits the prop-override path and the `isError`/`error` surfacing the view depends on; the plan's own guard concedes the refactor likely won't happen, leaving two subtly different resolution paths. (Same root concern as the architecture major.)

- 🔵 **Code Quality**: Injectable autoDismissMs is the only seam — document why it isn't also a showToast option
  **Location**: Phase 1, Section 1: useToastDispatcher (autoDismissMs argument)
  A hook-level param existing only for tests (AC tests use the default anyway) is mildly surprising; a future per-toast-duration need would find the seam at the wrong level. Add a one-line YAGNI note.

- 🔵 **Code Quality**: Bundling stable values (registry, showToast) into the per-render ref obscures intent
  **Location**: Phase 3, Section 1: useExternalEditToast
  Only `relPath` varies; `registry`/`showToast` are stable context handles and `subscribe` is `useCallback([])`-stable. The precedent (`use-doc-events.ts:148-151`) refs only genuinely-varying callbacks. Ref only `relPath` or comment the rationale.

- 🔵 **Code Quality**: Inline SVG path is a placeholder with no chosen glyph
  **Location**: Phase 2, Section 1: Toaster.tsx
  Icon ships as `<path d="…">` and close as `{/* × glyph */}`; left literal it ships a broken icon. Name a concrete glyph in the plan so implementation is mechanical.

- 🔵 **Correctness**: event.path === relPath correlation depends on an unverifiable server serialization contract
  **Location**: Phase 3: External-edit subscriber
  Both values are plain `string` with no normalization at the comparison site; a server format divergence (separators, leading `./`, casing) fails silently with no toast and no error. Pin it with a real-shaped fixture in Phase 5 and a comment documenting the exact-equality contract.

- 🔵 **Correctness**: deleted-action correlation works only by an undocumented timing coincidence
  **Location**: Phase 3 / Phase 5: deleted-action correlation ordering
  The delete toast fires only because subscribers run (`:210`) before `dispatchSseEvent` (`:227`) invalidates the docs list and flips `relPath` to undefined. Correct today, but a refactor to the post-drop `onEvent` slot would silently break delete-only. Add an explicit deleted-action Phase 5 case + a comment.

- 🔵 **Correctness**: self-cause non-consuming 5s TTL can suppress a genuine external edit on etag reuse
  **Location**: Phase 3: self-cause exclusion
  A genuinely-external write reusing an etag this client registered within the prior 5s is wrongly suppressed. Improbable with content-derived etags; document the assumption so it's revisited if etag generation changes.

- 🔵 **Architecture**: Toaster portals to document.body while sitting inside a provider tree — state the placement-vs-portal constraint
  **Location**: Phase 5: RootLayout wiring
  The component must be a React descendant of `ToastContext.Provider` while escaping the layout DOM. A future provider reshuffle could move `<Toaster/>` out of the subtree and silently fall back to the no-op handle (toasts vanish, no type error). Add an invariant comment at the mount point.

- 🔵 **Architecture**: Subscriber's self-cause check relies on the shared singleton rather than an injected boundary
  **Location**: Phase 3: External-edit subscriber (registry.has timing)
  Dispatcher-drop and subscriber-suppress agree only because no `SelfCauseContext` provider is added (both default to the module singleton). A future provider with a different registry for some subtree could silently desync the two paths. Note that any future provider must wrap both.

- 🔵 **Standards**: data-testid uses an `ac-` prefix that no existing component uses
  **Location**: Phase 2: Toaster presentational component
  Every existing testid is unprefixed kebab-case (`activity-live-badge`, `status-badge`); `ac-toaster-icon`/`ac-toaster-viewport` stand out. The `ac-` prefix is the CSS-class namespace, not the testid namespace. Use `toaster-icon`/`toaster-viewport`.

- 🔵 **Standards**: Multiple concurrent role="status" live regions may double-announce
  **Location**: Phase 2: Toaster presentational component (accessibility)
  Per-card live regions are announced inconsistently across screen readers vs the existing KanbanBoard single-persistent-region pattern. (Reinforces the usability major.) Prefer a single persistent region on `.viewport`.

- 🔵 **Standards**: No focus management when a toast is dismissed
  **Location**: Phase 2 / Phase 5: close button focus management
  Clicking close removes the focused button, orphaning focus to `document.body`. Specify post-dismiss focus behaviour or document it as an accepted limitation.

- 🔵 **Usability**: "Call exactly once" owning-hook contract is documented but not enforced
  **Location**: Phase 1: useToastDispatcher / useToast JSDoc
  Nothing stops a leaf calling `useToastDispatcher()`, silently creating a disconnected second stack whose toasts never appear — a no-error, no-toast bug. Consider a dev-only "instantiated more than once" warning.

- 🔵 **Usability**: Positional showToast(heading, message) args are an easy-to-swap footgun
  **Location**: Phase 1: ToastHandle.showToast signature
  Two same-typed positional strings transpose silently with no type error. An options object (`showToast({ heading, message })`) is self-documenting and extensible; cheapest before adoption grows.

- 🔵 **Usability**: No focus management or keyboard dismissal beyond tabbing to the close button
  **Location**: Phase 2: close button + viewport keyboard semantics
  Keyboard users must Tab past all page content to reach a control that may auto-dismiss first; no Escape-to-dismiss. Document intended keyboard behaviour so it's tested.

#### Suggestions

- 🔵 **Architecture**: ToastHandle binds Toast to a fixed heading/message shape — limits future variants
  **Location**: Phase 1: Toast context, stack, and dispatcher
  Adding a status/variant or per-kind icon later would be a cross-module change. Consider an optional defaulted `variant`/`kind` field now, or note it as a conscious minimal choice.

- 🔵 **Correctness**: showToast registers the timer after scheduling setTimeout
  **Location**: Phase 1: useToastDispatcher (timer registration ordering)
  Harmless at the 5s default; only a theoretical concern under a zero-delay test. No change required given idempotent filter/clear logic.

- 🔵 **Usability**: A single info notification styled with the success (--ac-ok) token may miscommunicate
  **Location**: Phase 2 styles / Desired End State
  A green icon on an informational "External edit detected" notice reads as "success". Prefer a neutral/info accent (or `--ac-warn`) and lock it with a `?raw` token assertion.

### Strengths

- ✅ Rides on existing `dispatchSseEvent` invalidation rather than duplicating or fighting it — the content-refresh AC is satisfied by code already in the imperative shell, keeping the subscriber a pure notifier.
- ✅ Verified-correct core claims: subscriber fires before the self-cause drop (`:210` vs `:220`), and the registry is genuinely shared (`defaultSelfCauseRegistry` is both the `SelfCauseContext` default and `makeUseDocEvents`'s default arg), so dispatcher-drop and subscriber-suppress agree without a provider.
- ✅ Faithful reuse of house patterns: no-op default context handle (never throws), owning/consumer hook split, immutable collection state, the ref-mutation idiom, and the SseIndicator component trio with `?raw` CSS-token assertions.
- ✅ Strong functional-core separation: `ACTION_VERB` is an exhaustively-typed `Record<ActionKind, string>` and `externalEditMessage` is a pure, independently-testable helper; the headless `ExternalEditToast` wrapper keeps `Toaster` presentational.
- ✅ Per-AC traceability is explicit, including each verb (created/updated/deleted) verified independently, and the auto-dismiss AC is tested against the real 5s default rather than mocked away.
- ✅ The manual-dismiss-clears-timer logic correctly resolves the dismiss-vs-auto-dismiss race; ids are monotonic and never reused; timer id typing matches convention.
- ✅ All cited design tokens and `global.css` line numbers verified accurate; the relPath-correlation fragility is explicitly acknowledged and deferred to a tracked future work item.

### Recommended Changes

1. **Specify the Phase 5 integration-test seam** (addresses: "Phase 5 integration test has no seam…", "fake timers + async query", "real-resolution positive correlation/content-refresh ACs"). Decide explicitly in the plan: either (a) build the integration test on `makeUseDocEvents(fakeFactory)` with a fake EventSource (as `use-doc-events`'s own tests do) rendered via a harness that supplies that handle, or (b) split coverage — prove content-refresh via a direct `dispatchSseEvent(event, queryClient)` call against a seeded `docContent` query, and prove correlation+toast via the captured-listener path — documenting that these are two tests, not one end-to-end. Also state the fake-timer-vs-real-timer regime split (correlation/refresh on real timers with `findBy*`; only the dedicated auto-dismiss case on fake timers).

2. **Add unmount cleanup to `useToastDispatcher`** (addresses: "Timer Map has no unmount cleanup"). Add `useEffect(() => () => { for (const t of timersRef.current.values()) clearTimeout(t); timersRef.current.clear() }, [])`, matching the clearTimeout-on-teardown convention. This also de-flakes the Phase 1 tests.

3. **Isolate the self-cause registry in tests** (addresses: "module-singleton registry with no reset/isolation"). Have Phase 5 construct a fresh `createSelfCauseRegistry()` per test and inject it via `SelfCauseContext.Provider` (the `use-move-work-item.test.tsx` pattern), with `reset()` in `beforeEach`; if fake timers are used, ensure the registry's clock is the same mocked clock.

4. **Resolve the Phase 4 framing** (addresses: "Resolver extraction diverges from LibraryDocView" ×2). Either parameterise the hook to accept optional `type`/`fileSlug` overrides and expose `isError`/`error` so `LibraryDocView` can fully consume it, OR drop the "single source of truth" language and scope the hook explicitly as a params-only resolver for the RootLayout subscriber. Pick one in the plan rather than deferring to implementation time.

5. **Strengthen toast accessibility** (addresses: "5s auto-dismiss / WCAG 2.2.1", "per-toast live regions + unbounded stack", "concurrent role=status double-announce", focus-management findings). Make the persistent `.viewport` a single `aria-live="polite"` region with plain `role="status"` card children; add pause-on-hover/focus-within (the `autoDismissMs` seam makes it testable); add a small max-stack cap (e.g. 3–5, dropping oldest); and specify post-dismiss focus behaviour. Reconcile this against the "What We're NOT Doing" exclusions (pause-on-hover, queue limits) — these specific additions are accessibility-driven, not feature creep.

6. **Add the missing test cases** (addresses: route-change-mid-subscription, undefined-etag, staggered-stack-timers, deleted-action ordering). Phase 3: rerender with changed `relPath` + assert single `subscribe` call; `etag: undefined` → toast fires. Phase 1: two staggered `showToast` calls each dismiss at their own +5s. Phase 5: explicit `action: 'deleted'` case against a still-resolved relPath.

7. **Signpost the hook API decisions** (addresses: "naming inverts convention", "call exactly once unenforced", "positional args footgun", "deleted-action timing", "shared-singleton invariant", "portal placement invariant"). Note the work-item-mandated `useToast()` name as a deliberate departure; keep the emphatic "OWNING hook — call ONCE" docstring; consider an options-object `showToast` (optional, one consumer); and add brief code comments documenting the load-bearing invariants (pre-drop ordering for delete, shared-registry agreement, descendant-of-provider for the portal, exact-string path equality).

8. **Lock concrete values** (addresses: placeholder SVG, `--ac-ok` token choice, `ac-` testid prefix). Name the actual glyph and close `×` markup; choose a neutral/info accent token (not success-green) and assert it via `?raw`; drop the `ac-` testid prefix to match house convention (`toaster-icon`/`toaster-viewport`).

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound and well-grounded in existing patterns — composes on mature SSE/self-cause/React-Query infrastructure, follows the no-op-default context + owning/consumer-hook convention, keeps Toaster presentational with a headless subscriber, and explicitly acknowledges its main tradeoff (relPath correlation deferred to a future doc-ID). Notable gaps: an incomplete Phase 4 extraction undermining the "single source of truth" goal, and an undischarged coupling between the toast 5s auto-dismiss and the self-cause 5s TTL.

**Findings**:
- 🟡 **major** (high) — Phase 4: Resolver extraction diverges from LibraryDocView's prop-or-param contract (`propType ?? params.type`, rendered with props in router/tests), so it is not a true single source of truth; two divergent copies will drift and a future doc-ID migration must touch both. Suggest: parameterise the hook with optional overrides, or drop the "single source of truth" framing.
- 🟡 **major** (medium) — Phase 1/3: Two independent 5s windows (toast auto-dismiss `TOAST_AUTO_DISMISS_MS`, self-cause TTL `self-cause.ts:16`) are coincidentally equal with no shared constant or acknowledged relationship; tuning either could silently change suppression/notification behaviour. Suggest: document the independence/coincidence with a cross-reference comment.
- 🔵 **minor** (high) — Phase 5: Toaster portals to `document.body` while needing to be a descendant of `ToastContext.Provider`; a future provider reshuffle could move it out and silently fall back to the no-op handle (toasts vanish, no type error). Suggest: keep `<Toaster/>`/`<ExternalEditToast/>` adjacent inside the provider with an invariant comment.
- 🔵 **minor** (medium) — Phase 3: Subscriber self-cause check relies on the shared `defaultSelfCauseRegistry` singleton (agreement maintained by *absence* of a provider); a future `SelfCauseContext` provider for some subtree could desync dispatcher-drop and subscriber-suppress. Suggest: note any future provider must wrap both.
- 🔵 **suggestion** (medium) — Phase 1: `ToastHandle` hard-codes `{heading, message}` + single glyph; future variants (success/warn/error, actions, per-kind icons) would be a cross-module change. Suggest: optional defaulted `variant`/`kind` field, or note the minimal choice is conscious.

### Code Quality

**Summary**: Well-structured and pragmatic, closely mirroring established conventions (no-op default handles, owning/consumer split, ref-mutation pattern, immutable collection state, SseIndicator trio). Snippets are clean and proportionate. Main gaps: an incomplete timer lifecycle in the Phase 1 dispatcher (no unmount cleanup → post-unmount state updates) and a Phase 4 resolver that diverges from LibraryDocView enough that "shared source of truth" is only partially achievable.

**Findings**:
- 🔴/🟡 **major** (high) — Phase 1: Timer Map has no unmount cleanup; pending auto-dismiss callbacks fire `setToasts` on an unmounted component, unlike `use-deferred-fetching-hint.ts:27-29` / `use-doc-events.ts:234-237`. Produces React unmount warnings and dangling timers — the very leak Performance Considerations claims to avoid. Suggest: add a cleanup `useEffect` clearing all timers.
- 🔵 **minor** (high) — Phase 4: Resolver omits LibraryDocView's prop-override (`:38-39`) and `isError`/`error` surfacing (`:44`); the plan's own guard concedes the refactor likely won't happen, leaving two drift-prone paths. Suggest: commit to a parameterised hook or drop the "shared source of truth" framing.
- 🔵 **minor** (medium) — Phase 1: Injectable `autoDismissMs` is the only test seam and AC tests use the default anyway; the asymmetry (hook-level param, no per-toast duration) is mildly surprising. Suggest: a one-line YAGNI comment.
- 🔵 **minor** (medium) — Phase 3: Bundling stable values (`registry`, `showToast`) into the per-render ref alongside the only-varying `relPath` obscures intent vs the precedent that refs only varying callbacks. Suggest: ref only `relPath` or comment the rationale.
- 🔵 **minor** (low) — Phase 2: Inline SVG ships as placeholder `<path d="…">` / `{/* × glyph */}`; left literal it ships a broken icon. Suggest: name a concrete glyph mirroring an existing icon.

### Test Coverage

**Summary**: Unusually rigorous — TDD per phase, each suite modelled on a verified existing template, all 7 ACs mapped to concrete named cases. All five referenced templates exist and behave as described. Principal weakness: the Phase 5 integration mechanism — RootLayout calls the production `useDocEvents` singleton with no EventSource seam, so "real dispatchSseEvent invalidation + delivered event" is internally inconsistent, and that's exactly where the two hardest ACs live. Secondary: fake-timer/async-query interaction, shared-singleton isolation, and a few untested edges.

**Findings**:
- 🔴/🟡 **major** (high) — Phase 5: No seam to deliver an SSE event through the real dispatch path; capturing the listener requires the real `onmessage` which is what runs `dispatchSseEvent`. May silently degrade to asserting on a mocked subscribe that never invalidates. Suggest: use `makeUseDocEvents(fakeFactory)`, or split content-refresh (direct `dispatchSseEvent` call) from correlation (captured listener).
- 🟡 **major** (high) — Phase 5: Self-cause/timing tests mutate the module-singleton `defaultSelfCauseRegistry` (real `Date.now()` TTL, no reset, shared across suites); fake timers don't advance its clock. Cross-test pollution and clock divergence → flaky/order-dependent. Suggest: fresh `createSelfCauseRegistry()` per test via `SelfCauseContext.Provider`, `reset()` in `beforeEach`.
- 🟡 **major** (medium) — Phase 5: Mixing fake timers with React Query async settling + `waitFor` is a known flake source; the plan doesn't keep the regimes separate. Suggest: correlation/refresh on real timers, only auto-dismiss on fake; or `vi.useFakeTimers({ toFake: [...] })`.
- 🔵 **minor** (high) — Phase 3: Route-change-mid-subscription (the central ref-pattern claim) is asserted nowhere; a stale-capture/re-subscribe regression would pass. Suggest: rerender with changed relPath, fire event, assert toast for new value AND single `subscribe` call.
- 🔵 **minor** (high) — Phase 3: `etag: undefined` boundary (`has(undefined) === false` → always external) is untested. Suggest: matching path + undefined etag → `showToast` IS called.
- 🔵 **minor** (medium) — Phase 1: Staggered-stack independent timers under-specified; a shared/reset-timer bug would pass. Suggest: two staggered `showToast` calls, each dismisses at own +5s.
- 🔵 **minor** (medium) — Phase 2: Icon AC leans on a brittle `data-testid` on an `aria-hidden` SVG; testid rename breaks it, empty SVG passes. Suggest: also assert `card.querySelector('svg')`.

### Correctness

**Summary**: The plan's central correctness claims hold up against the actual code: the subscriber genuinely fires before the self-cause drop (`:210` vs `:220`), the registry is genuinely shared (`defaultSelfCauseRegistry` is both the `SelfCauseContext` default and `makeUseDocEvents`'s default arg), and the ref-mutation pattern faithfully mirrors `use-doc-events.ts:148-151`. Timer logic is sound (stable empty-dep `dismissToast` reading refs, timer cleared on manual dismiss, monotonic non-reused ids). Remaining concerns: an unprovable-from-frontend path-format dependency, a known etag-reuse suppression edge, and the deleted-action ordering which is correct but undocumented/fragile.

**Findings**:
- 🔵 **minor** (medium) — Phase 3: `event.path === relPath` hinges on byte-identical server serialization vs indexer `relPath`; both plain `string`, no normalization at the comparison site. A server format divergence fails silently with no toast/error. Suggest: real-shaped fixture in Phase 5 + a contract comment.
- 🔵 **minor** (high) — Phase 3/5: Deleted-action toast fires only because subscribers run (`:210`) before `dispatchSseEvent` (`:227`) flips relPath to undefined — correct but an undocumented timing coincidence; a move to the post-drop `onEvent` slot would silently break delete-only. Suggest: explicit deleted-action Phase 5 case + comment.
- 🔵 **minor** (medium) — Phase 3: Non-consuming 5s-TTL `has()` wrongly suppresses a genuine external edit that reuses an etag registered within the prior 5s. Improbable with content-derived etags. Suggest: document the assumption.
- 🔵 **suggestion** (high) — Phase 1: `showToast` registers the timer into `timersRef` after `setTimeout`; harmless at the 5s default, only theoretical under a zero-delay test. No change required.

### Standards

**Summary**: Adheres closely to house conventions — component trio (named export, no barrel, `styles`, decorative `aria-hidden` SVG, `?raw` CSS assertions), the no-op-default context pattern, and the no-underscore `noopHandle` naming. Cited tokens/line numbers in `global.css` verified accurate; `role="status"`/`aria-live="polite"` matches the KanbanBoard pattern. Main gaps: a discoverability-affecting inversion of owning-vs-consumer hook naming and an `ac-`-prefixed `data-testid` scheme no existing component uses.

**Findings**:
- 🟡 **major** (high) — Phase 1: Owning/consumer naming inverts convention — consumer is bare `useToast()`, owning is `useToastDispatcher()` (vs `useTheme`/`useThemeContext`), no `...Context` consumer. The work item mandates `useToast()` as the dispatcher API, so signpost the deliberate departure and keep the emphatic owning-hook docstring.
- 🔵 **minor** (high) — Phase 2: `data-testid="ac-toaster-icon"/"ac-toaster-viewport"` uses an `ac-` prefix no existing testid uses (all are unprefixed kebab-case). Suggest: `toaster-icon`/`toaster-viewport`.
- 🔵 **minor** (medium) — Phase 2: Multiple concurrent dynamically-inserted `role="status"` regions may double-announce vs the KanbanBoard single-persistent-region pattern. Suggest: single persistent region on `.viewport`.
- 🔵 **minor** (medium) — Phase 2/5: No focus management when a toast is dismissed (focus orphaned to `document.body`). Suggest: specify post-dismiss focus or document as accepted limitation.

### Usability

**Summary**: Clean, convention-following developer API (no-op default, owning/consumer split, returned id) with the basic a11y primitives right (labelled close button, aria-hidden icon, role=status). Most material gaps are end-user a11y: a hard 5s auto-dismiss with explicitly no pause-on-hover (WCAG 2.2.1), an unbounded stack that can flood a polite live region, and per-toast (not container) live-region semantics. Developer side: the "call once" contract is documented-not-enforced and positional `showToast` args are an easy-to-swap footgun.

**Findings**:
- 🟡 **major** (high) — Phase 2 + "What We're NOT Doing": 5s auto-dismiss with no pause-on-hover is a WCAG 2.2.1 (Timing Adjustable) concern for a long backtick-path payload; SR users can't replay. Suggest: pause-on-hover/focus-within (the `autoDismissMs` seam makes it testable).
- 🟡 **major** (medium) — Phase 2/1: Per-toast dynamically-created live regions + unbounded stack flood/garble SR announcements during edit bursts and grow the viewport without bound. Suggest: single persistent `aria-live` region on `.viewport` + a small max-stack cap (3–5, drop oldest).
- 🔵 **minor** (high) — Phase 1: "Call exactly once" owning-hook contract is JSDoc-only; a leaf calling `useToastDispatcher()` silently creates a disconnected stack (no error, no toast). Suggest: dev-only "instantiated more than once" warning.
- 🔵 **minor** (medium) — Phase 1: Positional `showToast(heading, message)` (two same-typed strings) transposes silently. Suggest: options object `showToast({ heading, message })` — self-documenting and extensible.
- 🔵 **minor** (medium) — Phase 2: No focus management or Escape-to-dismiss; keyboard users Tab past all content to a control that may auto-dismiss first. Suggest: document intended keyboard behaviour so it's tested.
- 🔵 **suggestion** (low) — Phase 2 styles: Icon tentatively `var(--ac-ok)` (success-green) on an informational notice miscommunicates. Suggest: neutral/info accent (or `--ac-warn`) + `?raw` token assertion.

## Re-Review (Pass 2) — 2026-05-28

**Verdict:** COMMENT

All six lenses re-ran against the revised plan, delta-focused. **Every prior
finding (9 major, 17 minor, 3 suggestion) is RESOLVED**, and no new critical or
major issues were introduced. The remaining findings are minor/suggestion-level
and all stem from the new logic the revision added (pause/resume, the
`MAX_TOASTS` cap, the always-mounted live region) — the plan is ready to
implement. After the re-review, the strongest new finding (impure state
updaters) and several other nits were also applied to the plan.

### Previously Identified Issues

Architecture:
- 🟡 **Phase 4 resolver divergence / single-source-of-truth** — Resolved (reframed as a deliberate params-only resolver; LibraryDocView refactor explicitly out of scope).
- 🟡 **Coincident 5s windows (auto-dismiss vs self-cause TTL)** — Resolved (documented as independent).
- 🔵 **Toaster portal-vs-provider-descendant hazard** — Resolved (INVARIANT comment at the RootLayout wiring site).
- 🔵 **Shared defaultSelfCauseRegistry coupling** — Resolved (shared-registry invariant note + future-provider caveat).
- 🔵 **ToastHandle fixed shape limits variants** — Resolved (options-object `showToast`, extensible to variant/duration).

Code Quality:
- 🔴 **Timer Map unmount cleanup** — Resolved (teardown `useEffect` clears the timer map; idiomatic capture verified).
- 🔵 **Resolver DRY framing** — Resolved (honest out-of-scope framing).
- 🔵 **autoDismissMs seam undocumented** — Resolved (YAGNI note).
- 🔵 **Ref-bundling obscures intent** — Resolved (note distinguishing the one reactive value).
- 🔵 **Placeholder SVG** — Resolved (concrete info + close glyphs).

Test Coverage:
- 🔴 **Phase 5 had no seam to deliver an SSE event** — Resolved (two-shape split; APIs verified to exist).
- 🟡 **Shared-singleton registry, no isolation** — Resolved (per-test `createSelfCauseRegistry()` via `SelfCauseContext.Provider` + `reset()`; precedent confirmed).
- 🟡 **Fake-timer × async-query flake** — Resolved (documented timer-regime split).
- 🔵 **Route-change-mid-subscription untested** — Resolved (Phase 3 case added).
- 🔵 **Undefined-etag untested** — Resolved (Phase 3 case added).
- 🔵 **Staggered-stack timers under-specified** — Resolved (Phase 1 case added).
- 🔵 **Brittle icon data-testid** — Resolved (also asserts an actual `<svg>`).

Correctness:
- 🔵 **event.path === relPath unverifiable contract** — Resolved (comment + real-shaped Phase 5 fixture).
- 🔵 **Deleted-action ordering coincidence** — Resolved (comment + explicit Phase 5 deleted case; comment later reframed to the accurate reason).
- 🔵 **Self-cause TTL etag-reuse suppression** — Resolved (documented; inherent to pre-existing registry design).
- 🔵 **Timer registered after scheduling** — Resolved (`arm(id)` helper).

Standards:
- 🟡 **Owning/consumer naming inversion** — Resolved (signposted as work-item-mandated departure).
- 🔵 **`ac-` testid prefix** — Resolved (`toaster-icon`/`toaster-viewport`; only `--ac-*` tokens remain).
- 🔵 **Concurrent role=status regions** — Resolved (single persistent region mirroring KanbanBoard).
- 🔵 **No post-dismiss focus management** — Resolved (reasoned accepted limitation).

Usability:
- 🟡 **5s dismiss, no pause (WCAG 2.2.1)** — Resolved (pause-on-hover/focus; fresh-window resume judged acceptable).
- 🟡 **Per-toast live regions + unbounded stack** — Resolved (single region + `MAX_TOASTS` cap).
- 🔵 **Call-once contract unenforced** — Resolved (accepted house-convention tradeoff).
- 🔵 **Positional showToast args** — Resolved (options object).
- 🔵 **Focus/keyboard dismissal** — Resolved (pause-on-focus; Escape deferred).
- 🔵 **`--ac-ok` success-green miscommunicates** — Resolved (neutral token + `?raw` assertion).

### New Issues Introduced (by the revision's new logic)

- 🔵 **minor** (correctness + code-quality, 2 lenses) — Side effects (`arm`/`clearTimeout` + `timersRef` mutation) inside `setToasts` updaters violate reducer purity; StrictMode double-invokes them. Both agents confirmed the guards make it idempotent (no observable leak), but the pattern is fragile. **Applied:** reworked the dispatcher so updaters are pure (`slice(-MAX_TOASTS)`), with timer bookkeeping moved to plain handlers + a reconcile `useEffect` keyed on `toasts`; `resumeToast` reads a `toastsRef` mirror.
- 🔵 **minor** (correctness) — A paused-but-never-resumed toast persists with no timer until manual dismiss; not documented as acceptable. **Applied:** documented as an accepted terminal state (not a leak).
- 🔵 **minor** (test-coverage) — Captured-listener shape can't render real RootLayout (it self-provides the DocEvents singleton), so the provider-nesting INVARIANT is untested. **Applied:** clarified shape 1 uses a substitute tree + added a small real-RootLayout assertion for the nesting invariant.
- 🔵 **minor** (usability) — Full interactive cards (incl. close button) sit inside the `aria-live` region, diverging from the text-only KanbanBoard precedent; SR may announce button chrome. **Applied:** extended the Phase 5 SR manual check (move close button out of the announced subtree if verbose).
- 🔵 **minor** (usability) — `MAX_TOASTS` drop-oldest can silently discard an unseen external-edit notice during a burst. **Applied:** documented as an accepted information-loss tradeoff (content still refreshes via React Query independently); "N files changed" coalescing noted as future option.
- 🔵 **minor** (code-quality + usability) — `.icon` token left as a `--ac-fg-muted`/`--ac-warn` hedge. **Applied:** pinned to `var(--ac-fg-muted)`.
- 🔵 **suggestion** (architecture) — Delete-ordering comment overstated the dependency (relPath comes from the cached query, not synchronous SSE state). **Applied:** reframed the comment to the accurate reason (pre-drop slot is the only tier that sees all events).
- 🔵 **suggestion** (architecture) — Resume-restarts-full-window couples to the short-window assumption; revisit if a per-toast `duration` is ever added. Noted as future work; no change now.
- 🔵 **suggestion** (standards) — Keyboard pause-on-focus relies on bubbling from the close button (the only focusable child); parity is approximate. Acceptable as-is; re-verify if a future toast gains an action link.

### Assessment

The plan is in good shape and ready to implement. The revision cleanly resolved
every prior finding, and the re-review's new findings were minor consequences of
the added accessibility/robustness logic — the high-value ones (impure updaters,
token hedge, RootLayout wiring test, delete-ordering comment, two tradeoff
docs) have now been applied. What remains is genuinely optional polish
(Escape-to-dismiss, duration-aware resume, SR verbosity to verify during the
Phase 5 manual pass). No further review pass is needed before implementation.

---
*Re-review generated by /review-plan*

## Re-Review (Pass 3) — 2026-05-30

**Verdict:** APPROVE

Final polish applied to the plan: Escape-to-dismiss (dismisses the most-recent
toast via a document-level `keydown` listener attached only while the stack is
non-empty), with a Phase 2 unit case and a Phase 5 manual check covering it.
Updated the Desired End State, Testing Strategy summary, and removed
"Escape-to-dismiss can be a follow-up" from "What We're NOT Doing". The
remaining open suggestions (duration-aware resume) are tied to a feature not in
scope; the SR-verbosity check is already in the Phase 5 manual list.

The plan's frontmatter status has been moved from `draft` to `approved`. No
further review pass is required. Ready for `/implement-plan`.

---
*Final approval — /review-plan*
