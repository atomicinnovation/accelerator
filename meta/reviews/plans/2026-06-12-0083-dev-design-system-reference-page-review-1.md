---
type: plan-review
id: "2026-06-12-0083-dev-design-system-reference-page-review-1"
title: "Plan Review: DevDesignSystem Reference Page Implementation Plan"
date: "2026-06-13T07:38:33+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-12-0083-dev-design-system-reference-page"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, correctness, test-coverage, standards, compatibility, usability, documentation]
review_number: 1
review_pass: 2
tags: [design, frontend, visualiser, design-system, scroll-spy, routing, visual-regression]
last_updated: "2026-06-13T09:24:53+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: DevDesignSystem Reference Page Implementation Plan

**Verdict:** REVISE

This is a thorough, well-structured plan: an 11-phase bottom-up decomposition
with an explicit dependency DAG, every phase independently mergeable and green,
a single shared `dev-constants.ts` as the cross-cutting seam, per-section count
oracles bound to live constants rather than frozen integers, and TDD throughout.
The findings are concentrated, not scattered — almost all of them cluster around
three decisions: the choice of a real `/dev` route + hash bridge (over the
research's recommended overlay), the under-specified scroll-spy active-section
algorithm, and the as-yet-unowned reconciliation between the plan and the work
item's acceptance criteria. One finding is critical (the code-block VR migration
silently shrinks coverage from eight languages to two), which alone moves the
verdict to REVISE; the major findings are mostly hardening and specification gaps
rather than fundamental flaws. None require abandoning the design — they require
pinning down behaviour the plan currently defers to implementation.

### Cross-Cutting Themes

- **The route-vs-overlay mounting decision drives a cluster of activation
  defects** (flagged by: architecture, correctness, code-quality) — The plan
  chose a real `/dev` TanStack route + hash-activation bridge over the research's
  explicitly-recommended overlay model. That choice (sound for URL cleanliness
  and deep-linkability) is the common root of four findings: a likely circular
  import (`router.ts → RootLayout → use-dev-activation → router.ts`), a
  pushState-based normalisation that traps the browser Back button, the
  `priorRef`/`sessionStorage` exit-restore bookkeeping the overlay would have made
  free, and a load-bearing-but-unguarded "`replaceState`/`router.navigate` don't
  fire `hashchange`" invariant. The route model is defensible, but its costs
  should be paid explicitly rather than left implicit.

- **The scroll-spy active-section algorithm is the highest-risk logic and is
  under-specified** (flagged by: correctness, code-quality, test-coverage) — The
  fix for the prototype's pinned-to-Colours defect is described only as "pick the
  topmost section within the active region," with the tie-break, the
  no-section-in-band fallback, and the cross-dispatch state left to
  implementation. There is no dedicated regression test that fails against the
  pinned behaviour, the e2e assertions risk flakiness against async observer
  dispatches, and the logic sits embedded in the large page component. This is the
  exact code the prototype got wrong.

- **Plan/work-item contract divergence is acknowledged but unowned** (flagged by:
  architecture, compatibility, documentation) — The plan's "Acceptance-criteria
  note" says AC3/AC5/AC7 "should be amended" to the canonical-route form, and the
  "returns 404" AC is satisfiable only as an SPA not-found (HTTP 200), and the work
  item's count parentheticals (Stage dots 9, glyphs 12) are stale-low versus the
  live constants the plan asserts (8, 13). No phase actually owns editing the work
  item, so the canonical record will contradict the shipped behaviour until someone
  does.

- **Activation discoverability and README documentation scope** (flagged by:
  usability, documentation) — All three triggers are effectively hidden knowledge
  (a `title` tooltip on a `userSelect:none` element, an undocumented chord, an
  undocumented hash), and Phase 11's README scope ("list DevDesignSystem + drop
  showcases") does not itemise the triggers, the chord+fallback, the canonical
  deep-link form, or the slug-vs-label mapping it documents elsewhere.

- **New `Icon` primitive's DX/standards parity with `Glyph`** (flagged by:
  standards, usability) — The Icon spec defines `IconProps` and the aria flip but
  omits the consumer-contract docstring, the DEV unknown-name warning, and the
  `framed` escape hatch that the sibling `Glyph` ships and that the eyebrow-icon
  migration (lifecycle/kanban) needs.

### Tradeoff Analysis

- **Route model vs overlay model**: The route gives clean canonical URLs
  (`/dev#colors`), real deep-linkability, and reuse of the existing provider/scroll
  tree. The overlay (research's recommendation) would have given free prior-route
  restoration (underlying route never unmounts), no `hashchange`-loop concern, and
  no router-singleton import from inside the layout. Recommendation: keep the route
  model if URL cleanliness is the priority, but harden it for the costs the overlay
  avoided (hooks not singleton; capture prior path on every non-`/dev` location
  change; `replace: true` normalisation; an explicit re-entrancy guard + test).

- **In-app discoverability (usability) vs intentionally-hidden dev tool**: Usability
  wants a visible affordance; the page is deliberately hidden from production nav.
  Recommendation: a dev tool does not need production discoverability, but it does
  need *thorough README documentation* (the cheap, durable win) and optionally a
  DEV-build console hint — full in-app surfacing is out of scope.

- **Open `size: number` (usability) vs curated union (standards)**: `Glyph` uses a
  curated size union; the Icon spec uses an open number. For freely-scaling stroke
  icons an open number is the right call. Recommendation: keep it, but document the
  intentional divergence so it doesn't read as an oversight.

### Findings

#### Critical

- 🔴 **Test Coverage**: Code-block VR coverage shrinks from 8 languages to 2; the
  resolved-colour spec will lose cells
  **Location**: Phase 9 (Code blocks) + Phase 10 (code-syntax spec repoint)
  The existing `code-block-resolved-colours.spec.ts` asserts resolved syntax
  colours across eight `code-syntax-cell-<lang>` cells (python, typescript, yaml,
  json, css, html, diff, markdown) plus the diff-override scoping cases, but Phase
  9 only ports TypeScript + Bash. Repointing the spec at `/dev` as written either
  fails (cells absent) or silently drops the per-language and diff-override
  coverage story 0076 established — contradicting the plan's "coverage is
  preserved" goal.

#### Major

- 🟡 **Architecture**: Referencing the `router` singleton from inside `RootLayout`
  creates a circular import (confidence: high)
  **Location**: Phase 4: Activation bridge hook + keychord
  Mounting `use-dev-activation.ts` in `RootLayout` and calling the exported
  `router` singleton creates `router.ts → RootLayout.tsx → use-dev-activation.ts →
  router.ts`. The codebase deliberately avoids this (Breadcrumbs uses
  `useRouter()`, not the singleton). Drive the bridge/chord through in-context
  hooks instead.

- 🟡 **Correctness**: pushState-based alias normalisation traps the browser Back
  button (confidence: high)
  **Location**: Phase 4: Activation bridge hook
  Normalising an alias via `router.navigate({ to: "/dev", hash })` defaults to
  `pushState`, leaving the alias URL in history; pressing Back fires `hashchange`,
  re-matches `DEV_ALIAS_RE`, and bounces forward again — Back is permanently
  absorbed. Use `replace: true` and add a back-navigation test.

- 🟡 **Architecture**: The hash is a read/write channel for two subsystems, kept
  from looping only by an implementation detail (confidence: medium)
  **Location**: Phase 4 (bridge reads `hashchange`) + Phase 5 (scroll-spy writes
  `replaceState`)
  The only thing preventing an activation loop is that `replaceState`/`pushState`
  don't fire `hashchange` — an invariant living in prose, not structure. A future
  switch to `location.hash = …` silently re-enters the bridge. Guard with a
  re-entrancy flag + a test.

- 🟡 **Code Quality**: A single `DevDesignSystem.tsx` concentrates 24 sections plus
  scroll-spy and theme wiring into one god-component (confidence: high)
  **Location**: Phases 5–9 (all route into one file)
  This inverts the codebase convention of one focused ~50–80-line file per
  showcase, makes per-section unit tests mount the whole page, and forces the four
  parallel content phases (6–9) to conflict on one file. Decompose into a shell +
  `sections/<Section>.tsx` + a `use-scroll-spy` hook.

- 🟡 **Code Quality**: Activation feature fragmented across five files coordinating
  via an implicit invariant (confidence: high)
  **Location**: Phase 4 (use-dev-activation.ts, RootLayout chord/Escape,
  Sidebar triple-click, dev-constants, scroll-spy hash write)
  The pieces coordinate indirectly through `location.hash` + `priorRef` + the
  unguarded `hashchange` invariant; the chord reads `pathname === "/dev"` while
  activation writes the hash, splitting "am I in dev?" across channels. Centralise
  behind `enterDev()`/`exitDev()`/`isDevActive`.

- 🟡 **Correctness**: `offsetTop - offset` deep-link scroll assumes an
  `offsetParent` the live scroll root doesn't guarantee (confidence: medium)
  **Location**: Phase 5: deep-link landing math
  `.main` declares no `position`, so a section's `offsetParent` may resolve past
  it and the computed scroll top is wrong by the layout offset — the target may not
  land in the observer's active region (failing the deep-link AC). Set `.main`
  `position: relative` or compute the delta via `getBoundingClientRect`.

- 🟡 **Correctness**: Active-section pick is under-specified at the active-region
  boundaries (confidence: medium)
  **Location**: Phase 5: active-section recompute
  With the thin ~25% active band, the rule for "no heading in band" (tall section)
  and the tie-break are undefined, so the highlight can clear or flicker — the very
  "never pinned" failure the fix targets. Specify a total order (last section whose
  top is above the band top; first/last at the extremes).

- 🟡 **Test Coverage**: Scroll-spy e2e assertions are under-specified for
  determinism, risking flaky tests (confidence: medium)
  **Location**: Phase 5: Success Criteria
  Async, batched IntersectionObserver callbacks + `scrollTo` + immediate
  `expect(hash)` is a classic flake pattern. Require retrying assertions
  (`toHaveURL`, `expect.poll`) and an explicit settle condition; no
  `waitForTimeout`.

- 🟡 **Test Coverage**: The "never pinned" regression lacks a dedicated
  failing-then-passing test (confidence: medium)
  **Location**: Phase 5: Success Criteria
  The pinned-to-Colours defect is the reason for the whole recompute, yet it's only
  a parenthetical in the general scroll-advance test. Add an assertion that, after
  scrolling past Colours, the active entry is the *next* (short) section — a test
  that fails against the prototype's behaviour.

- 🟡 **Test Coverage**: The showcase→section coverage-preservation audit is
  manual-only, with no automated guard against dropped cells (confidence: high)
  **Location**: Phase 10: row-by-row audit
  The central anti-coverage-loss control is one manual checkbox; resolved-colour
  companion specs assert hard-coded cell lists that can silently shrink on a
  `data-testid` rename. Add an automated cell-presence gate (cf. an existing
  `fixture-coverage` pattern).

- 🟡 **Test Coverage**: The manual-only cross-browser chord matrix gives no
  regression protection for the "not reserved" AC (confidence: medium)
  **Location**: Manual Testing step 5
  Acceptable as a one-time gate (the property is inherently browser-specific and
  not unit-testable in JSDOM), but the plan should bind `DEV_CHORD` to a "re-run the
  matrix if this changes" note so a future chord swap re-opens the risk visibly.

- 🟡 **Standards**: Scroll-spy TOC active item lacks `aria-current` despite an
  established codebase convention (confidence: high)
  **Location**: Phase 4/5: TOC active highlight
  Breadcrumbs (`aria-current="page"`) and the library template selection both set
  it; the plan conveys active state by CSS class alone (fails WCAG 1.4.1). Set
  `aria-current` on the active jumplink and assert it.

- 🟡 **Standards**: Triple-click and chord-only activation have no
  keyboard/pointer-agnostic equivalent (confidence: medium)
  **Location**: Phase 4: triple-click host + keychord
  A 3-click pointer gesture on a `userSelect:none` `<div>` is invisible to AT and
  unreachable by keyboard. Make the keyboard-equivalence call explicit (the chord
  is the intended keyboard path; consider a focusable host).

- 🟡 **Compatibility**: The "returns 404" AC is satisfiable only as an SPA
  not-found (HTTP 200), not a real 404 (confidence: high)
  **Location**: Phase 11: retire routes
  The app is a pure CSR SPA with no `notFoundComponent`; removing routes yields
  TanStack's default not-found served as 200 with `index.html`. Amend the AC
  wording (as the plan does for the hash AC) to "resolves to the SPA not-found UI,
  no redirect."

- 🟡 **Compatibility**: The "byte-identical Brand output" claim is contradicted by
  the `useId()` gradient id (confidence: high)
  **Location**: Phase 3: AtomicMark extraction
  A per-instance `useId()` suffix changes the `<linearGradient id>` and its
  `url(#…)` reference, so the markup is *not* byte-identical (only pixel-identical),
  and `useId()` ids are non-deterministic across runs — any Brand snapshot oracle
  must normalise the id or assert pixels.

- 🟡 **Usability**: Activation is near-undiscoverable — no in-app affordance points
  developers to the page (confidence: high)
  **Location**: Desired End State / Phase 4 triple-click / Phase 11 README
  The chord, hash, and triple-click are all hidden knowledge; a maintainer who
  hasn't read the README has essentially no discovery path. Add at least one
  low-cost affordance (README itemisation, a DEV console hint, a visible foot
  label).

- 🟡 **Usability**: The Icon primitive lacks the consumer-contract docstring and
  unknown-name guidance that `Glyph` models (confidence: medium)
  **Location**: Phase 1: Icon component
  `Glyph` ships a tinting/single-svg/ariaLabel docstring and a DEV `console.warn`
  on unknown docType; the Icon spec has neither, despite being reached for
  app-wide. Mirror Glyph's docstring and add a DEV warn listing valid
  `ICON_NAMES`.

- 🟡 **Documentation**: The "documented as a deviation" pattern names no
  authoritative home (confidence: high)
  **Location**: What We're NOT Doing; Phases 6, 8, 9; Acceptance-criteria note
  The plan repeatedly says deviations will be "documented" but never says where
  (in-page prose? code comment? work item? PR?). Since the page *is* reference
  documentation, an in-page per-section deviation note is the most discoverable
  home.

- 🟡 **Documentation**: Phase 11 README scope omits the activation triggers, chord,
  and deep-link slug form it documents elsewhere (confidence: high)
  **Location**: Phase 11, §3: README
  The README is the only persistent dev-facing home for this surface, yet Phase 11
  doesn't require itemising the three triggers, the chord (+`Cmd/Ctrl+Shift+G`
  fallback), the canonical `/dev#<section>` form, or that `<section>` is the slug
  not the label.

- 🟡 **Documentation**: The plan notes ACs "should be amended" but does not own
  editing the work item (confidence: medium)
  **Location**: Desired End State: Acceptance-criteria note; Phase 11
  No phase owns amending AC3/AC5/AC7 or reconciling the stale count parentheticals,
  so verifiers checking against the literal work-item text will see "failures" that
  are actually documented improvements.

#### Minor

- 🔵 **Architecture**: Prior-route restoration relies on ad-hoc `priorRef` +
  `sessionStorage` — specify exit-target precedence, validate the stored path
  resolves, bound the cross-tab failure mode.
  **Location**: Phase 4: bridge exit / Desired End State

- 🔵 **Architecture**: Scroll-spy binds the scroll root via
  `closest("main")` (DOM traversal), a hidden cross-component contract that breaks
  silently if a nested `<main>` is ever introduced. Expose an explicit
  `data-scroll-root` seam from RootLayout.
  **Location**: Phase 5: scroll-spy root

- 🔵 **Architecture**: The reference page hand-authors several primitives
  (breadcrumbs, library table, tier pills, toasts) instead of rendering the live
  component, risking reference/runtime drift and weakening the VR oracle (it
  snapshots the copy). Prefer presentational-prop refactors where feasible;
  document where unavoidable.
  **Location**: Phase 9: composites & chrome

- 🔵 **Code Quality**: Exit-target state has two sources (`priorRef` +
  `sessionStorage`) with no stated reconciliation rule, and accesses
  `sessionStorage` directly rather than the codebase's `safe-storage`/`storage-keys`
  wrappers (raw access can throw in private mode).
  **Location**: Phase 4: bridge

- 🔵 **Code Quality**: Re-authoring ~550 lines of `ds-*` CSS risks dragging in dead
  selectors (the plan already notes the prototype's undefined `.ds-spec*` family).
  Treat the port as selective re-authoring keyed to rendered sections.
  **Location**: Phase 4: chrome shell

- 🔵 **Correctness**: Unenforced invariant — no `DEV_SECTIONS` id may begin with
  `dev` at a word boundary, or its canonical `#dev…` hash is mis-classified as an
  alias. Add a unit assertion.
  **Location**: dev-constants.ts (DEV_ALIAS_RE)

- 🔵 **Correctness**: The overview hash write contradicts itself — `replaceState("#"
  + id)` writes `#overview`, but Desired End State says overview is bare `/dev`.
  Pick one canonical overview form and assert it.
  **Location**: Phase 5: overview hash write

- 🔵 **Correctness / Usability**: Escape-to-exit is not guarded against editable
  targets — pressing Escape inside a demo input (the search composite) would eject
  the whole page. Guard with `isEditableTarget` as the `/` handler does. *(Flagged
  independently by both the correctness and usability lenses.)*
  **Location**: Phase 4: keychord / Escape

- 🔵 **Correctness**: `priorRef` is set only on alias entry, so a direct `/dev` URL
  (or any non-alias entry) silently degrades exit-restore to `/library`. Capture
  the last non-`/dev` pathname on every location change.
  **Location**: Phase 4: bridge exit restore

- 🔵 **Test Coverage**: Chord negatives test single-modifier cases but not the
  likely accidental-fire mutations (Cmd+L without Shift, Cmd+Shift+wrong-key,
  case-sensitivity of `l`/`L`). Add them.
  **Location**: Phase 4: Success Criteria

- 🔵 **Test Coverage**: No test for Escape scoping, the `sessionStorage` cold-load
  exit path, or that `replaceState` writes don't re-enter `sync()`.
  **Location**: Phase 4/5

- 🔵 **Test Coverage**: The Colours theme-responsiveness oracle (the AC's named
  example) asserts swatch *counts* but leaves "surface tokens change between
  light/dark" to a manual check. Promote it to an automated computed-style
  assertion using the `expected-colours.ts` helpers.
  **Location**: Phase 6: Success Criteria

- 🔵 **Test Coverage**: Phase 2's icon-geometry baseline regen blesses whatever
  renders, so a genuine regression (wrong stroke-width, swapped icon) is baked into
  the baseline. Retain a few targeted unit assertions on canonical geometry.
  **Location**: Phase 2: Success Criteria

- 🔵 **Standards**: The Icon primitive omits the `framed` escape hatch the
  lifecycle/kanban eyebrow-icon migration needs (those render inside `IconFrame`).
  Either keep them custom (note in the "leave custom" list) or wrap at the call
  site.
  **Location**: Phase 1/2

- 🔵 **Standards**: New constants modules (`icon-names.ts`) diverge from the
  component-adjacent `.constants.ts` convention (`Glyph.constants.ts`,
  `BigGlyph.constants.ts`). Prefer `Icon.constants.ts`.
  **Location**: Phase 1/4

- 🔵 **Standards**: Bare `aria-hidden` JSX shorthand diverges from the codebase's
  explicit `aria-hidden="true"` (used in review/grep gates). Emit `aria-hidden={true}`.
  **Location**: Phase 1

- 🔵 **Standards**: Confirm the migrated `ThemeToggle` (`data-icon`) and `Toaster`
  (role/dismiss) test contracts survive the icon swap, and enumerate which call
  sites need an `ariaLabel`.
  **Location**: Phase 2

- 🔵 **Compatibility**: `Cmd/Ctrl+Shift+L` is not currently engine-reserved (low
  risk), but `e.key`-based matching is keyboard-layout-sensitive; consider
  `e.code === "KeyL"` or note the caveat.
  **Location**: Phase 4: keychord

- 🔵 **Compatibility**: `playwright.glyph.config.ts` `webServer.url` → `/dev`
  resolves via vite-preview SPA fallback (fine); keep `testMatch` in sync with the
  renamed specs so the standalone config doesn't match zero specs.
  **Location**: Phase 10

- 🔵 **Compatibility**: `useId()` for gradient ids is confirmed safe (pure CSR, no
  hydration) — just ensure no test asserts the literal id string.
  **Location**: Phase 3 (informational confirmation)

- 🔵 **Usability**: The `#dev/<section>` alias silently rewriting the address bar to
  `/dev#<section>` can surprise a developer who hand-typed it; document the
  canonical-vs-alias distinction in the README and a bridge comment.
  **Location**: Phase 4/5

- 🔵 **Usability**: `size: number` (open) vs `Glyph`'s curated union is a defensible
  divergence for free-scaling stroke icons — document it as intentional with the
  default + conventional ramp.
  **Location**: Phase 1: IconProps

- 🔵 **Usability**: Focus management on activate/exit is unspecified; keyboard-only
  users can lose their place to `<body>`. Move focus to the page heading on
  activate, restore it on exit, and assert it.
  **Location**: Phase 5 / Phase 4

- 🔵 **Documentation**: Phase 11's README update must also fix the "Glyph showcase
  specs" block (`:67-77`) and the VR-baseline example commands, which name spec
  filenames/config that Phase 10 renames.
  **Location**: Phase 11, §3

- 🔵 **Documentation**: The non-obvious "why" in `dev-constants.ts` /
  `use-dev-activation.ts` (alias-vs-canonical distinction, cold-load exit fallback,
  `replaceState`-doesn't-fire-`hashchange`) should be captured as call-site comments.
  **Location**: Phase 4

- 🔵 **Documentation**: The slug-vs-label divergence (`stagedots` → "Stage dots") is
  reader-facing (it's in the URL) but has no on-page bridge; consider an
  anchor-link affordance or slug in the jumplink `title`, and document the mapping.
  **Location**: Phase 4: CONTENTS aside

### Strengths

- ✅ A single shared `dev-constants.ts` owns the cross-cutting constants
  (`DEV_ALIAS_RE`, `aliasSection`, `DEV_CHORD`, `DEV_CHORD_HINT`, `DEV_SECTIONS`),
  giving the chord, marquee hint, footer hint, and TOC one source of truth and
  directly satisfying the work item's "single shared constant" / "never `⌘⇧D`"
  requirement (praised by architecture, code-quality, standards, usability).
- ✅ Per-section variant counts bind to live source constants (`ICON_NAMES.length`,
  `DOC_TYPE_KEYS.length`, `WORKFLOW_PIPELINE_STEPS.length`, `DOC_TYPE_HUE` count)
  rather than frozen integers — the correct hedge against the prototype/live count
  drift the research documents (every lens noted this).
- ✅ The 11-phase decomposition is genuinely incrementally shippable, with an
  explicit dependency DAG and each phase leaving `main` green.
- ✅ Genuine SRP refactors folded in as proper consolidation, not parallel copies:
  one `Icon` primitive replaces ~18 duplicated inline SVGs (chevron-right alone had
  5 divergent copies), and `AtomicMark` is extracted cleanly from `Brand`.
- ✅ The new Icon primitive faithfully matches the live authoring convention (24×24
  viewBox, `currentColor`, 2px rounded — citing byte-identical prototype↔live
  paths) and reproduces `Glyph`'s `ariaLabel`→`role="img"` a11y contract exactly.
- ✅ Consistently test-driven: the keychord tests reuse the proven
  `RootLayout.test.tsx:181-195` `preventDefault` template with modifier negatives,
  and the triple-click test exercises the 600 ms timing boundary.
- ✅ Correctly diagnoses the prototype's pinned-to-Colours root cause (single
  highest-ratio dispatch) and prescribes the right *class* of fix; correctly uses
  `useId()` to avoid gradient-id collisions; the `replaceState`-non-re-entrancy
  claim is spec-accurate.
- ✅ Compatibility-aware: the chord is deferred behind a recorded cross-browser
  matrix with a named fallback isolated behind two constants, and the VR baseline
  OS-coupling (git mv of `*-snapshots/`, stable `name` args, regen both darwin +
  linux, `GITHUB_TOKEN`-won't-retrigger caveat) is handled correctly.
- ✅ Deliberate prototype deviations (no compact PipelineMini, no traffic-light
  dots, three-component status/verdict/result split, radii px-ladder vs sm/md/lg,
  no `--shadow-brand`) are each named with rationale and re-expressed against live
  token names (ADR-0039/0026/0035 aligned).
- ✅ The "Acceptance-criteria note" transparently documents *why* the canonical
  `/dev#<section>` URL departs from the work item's literal AC wording rather than
  diverging silently.

### Recommended Changes

1. **Reconcile the code-block VR coverage** (addresses: *Code-block VR coverage
   shrinks from 8 to 2*) — Port all eight `code-syntax-cell-<lang>` cells (incl. the
   diff-override fixture) into Phase 9's Code-blocks section, or explicitly document
   which resolved-colour assertions are intentionally dropped and migrate the rest.
   Update the Phase 9 count oracle (currently "2") and the Phase 10 row-by-row audit
   to match. This is the critical finding gating the verdict.

2. **Fix the Back-button trap** (addresses: *pushState normalisation traps Back*) —
   Normalise aliases with `router.navigate({ …, replace: true })` so the alias URL
   never enters history, and add a bridge test that simulates a `hashchange` back to
   an alias and asserts no forward re-navigation.

3. **Pin down and harden the route-model activation** (addresses: *circular import*,
   *hash read/write looping*, *fragmented activation*, *prior-route bookkeeping*,
   *priorRef non-alias gap*, *two storage sources*) — Drive the bridge and chord
   through `useRouter()`/`useNavigate()` hooks (not the `router` singleton);
   centralise behaviour behind `enterDev()`/`exitDev()`/`isDevActive`; capture the
   prior path on every non-`/dev` location change (not only on alias entry); route
   persistence through `safe-storage`/`storage-keys`; and add a re-entrancy guard +
   test that a programmatic hash rewrite does not re-enter `sync()`.

4. **Specify and isolate the scroll-spy algorithm** (addresses: *active-section pick
   under-specified*, *deep-link offsetParent*, *never-pinned test*, *e2e
   determinism*, *recompute under-specified*) — State the total-order selection +
   tie-break + fallback rule explicitly; extract it into a pure function behind a
   `use-scroll-spy` hook unit-tested with fabricated rects; fix the deep-link scroll
   math (`.main { position: relative }` or a `getBoundingClientRect`-based delta);
   add a dedicated "leaves Colours, never pinned" regression test; and require
   retrying e2e assertions with an explicit settle condition.

5. **Add an automated VR cell-presence gate** (addresses: *manual-only coverage
   audit*) — Enumerate the required `data-testid` cells per migrated section and
   fail CI if any are missing, so coverage preservation doesn't rest on reviewer
   diligence.

6. **Own the work-item amendment as an explicit step** (addresses: *ACs unowned*,
   *404 wording*, *canonical-form divergence*) — Add a Phase 11 (or pre-implementation)
   task that edits `0083`'s AC3/AC5/AC7 to the route+bare-section form, rewrites
   "returns 404" → "resolves to the SPA not-found UI (no redirect)", and reconciles
   the stale count parentheticals (8 stages, 13 glyphs).

7. **Decompose `DevDesignSystem.tsx`** (addresses: *god-component*) — Commit the plan
   to a thin shell (chrome + scroll-spy hook + section registry) with each section
   as `sections/<Section>.tsx`, mirroring the per-showcase file convention and
   de-conflicting the parallel content phases.

8. **Close the accessibility gaps** (addresses: *aria-current*, *no keyboard
   equivalent*, *Escape unguarded*, *focus management*) — Set `aria-current` on the
   active TOC entry; guard Escape-to-exit with `isEditableTarget`; specify and test
   focus placement on activate/exit; and make the keyboard-equivalence decision for
   the triple-click host explicit.

9. **Bring `Icon` to parity with `Glyph` and correct the Brand claim** (addresses:
   *Icon docstring/unknown-name*, *framed escape hatch*, *aria-hidden shorthand*,
   *byte-identical Brand*, *size union divergence*) — Add the consumer-contract
   docstring + DEV unknown-name warn; decide the `framed` path for eyebrow icons;
   use explicit `aria-hidden={true}`; reword Brand's claim to "pixel-identical" and
   normalise the gradient id in any snapshot; document `size: number` as an
   intentional divergence.

10. **Anchor the documentation** (addresses: *deviation home*, *README scope*,
    *Glyph-spec block*, *non-obvious comments*, *slug-vs-label*, *discoverability*,
    *alias rewrite surprise*) — Name an authoritative home for deviations (in-page
    per-section note recommended); expand Phase 11's README scope to itemise the
    three triggers, the chord + fallback, the canonical deep-link form, the slug
    mapping, and the renamed glyph specs/config + baseline commands; and add
    call-site comments for the non-obvious bridge invariants.

11. **Smaller correctness/standards hardening** (addresses: *section-id `dev`
    invariant*, *overview hash self-contradiction*, *chord negatives*, *Colours
    theme assertion*, *icon-geometry unit assertions*, *constants module naming*,
    *migrated-icon contracts*) — Fold these minor items in where the relevant phase
    is touched.

## Per-Lens Results

### Architecture

**Summary**: Well-structured 11-phase bottom-up decomposition with a single shared
constants seam, clear dependency ordering, and each phase mergeable. The central
decision — a real `/dev` route + hash bridge over the research's recommended
overlay — is defensible (URL cleanliness, deep-linkability, provider-tree reuse)
and its main tradeoff (prior-route bookkeeping) is acknowledged, but the bridge's
imperative-shell machinery is under-specified and the most concrete structural risk
is a likely circular import created by referencing the `router` singleton from
inside RootLayout's subtree, which the codebase deliberately avoids.

**Strengths**:
- Single shared `dev-constants.ts` owns the cross-cutting constants, giving chord
  hint / marquee / footer / bridge / scroll-spy one source of truth.
- Choosing a real `/dev` route keeps the page inside RootLayout's provider tree and
  `<main>` scroll root, reusing existing infrastructure as the five showcase routes
  already do.
- The 11-phase decomposition is genuinely incrementally shippable with an explicit
  dependency DAG, each phase green.
- Counts bound to live constants — open-closed adherence at the test boundary.
- `AtomicMark` extraction with per-instance `useId()` improves cohesion while
  keeping Brand's output unchanged; Icon consolidates ~18 duplicated SVGs.
- The divergence from the research recommendation is explicitly named with its
  tradeoff, not an implicit pattern break.

**Findings**:
- 🟡 major (high) — *Referencing the router singleton from inside RootLayout creates
  a circular import* (Phase 4): `router.ts → RootLayout.tsx → use-dev-activation.ts
  → router.ts`. Existing code reaches routing via `useRouter()` hooks, not the
  singleton (imported only by `main.tsx`). A module cycle through the router can
  produce a TDZ-`undefined` router at import time. Drive bridge/chord through
  in-context hooks.
- 🟡 major (medium) — *Hash is a write/read channel for two subsystems kept from
  looping only by an implementation detail* (Phase 4/5): the no-loop guarantee
  (`replaceState`/`pushState` don't fire `hashchange`) lives only in prose; a future
  switch to `location.hash = …` silently re-enters the bridge. Guard with a
  re-entrancy flag + test.
- 🔵 minor (high) — *Prior-route restoration relies on ad-hoc ref + sessionStorage*
  (Phase 4): edge cases (stale stored path post-deploy, direct `/dev`, cross-tab
  sessionStorage) under-specified. Define precedence (ref → sessionStorage →
  `/library`), validate resolvability, bound the cross-tab mode.
- 🔵 minor (medium) — *Scroll-spy binds the scroll root by DOM traversal*
  (`closest("main")`, Phase 5): an implicit cross-component contract that breaks
  silently if a nested `<main>` appears. Expose an explicit `data-scroll-root` seam.
- 🔵 minor (medium) — *Canonical-form decision creates a standing divergence between
  plan and work item ACs* (Desired End State): amend the ACs as part of this plan,
  and decide whether the aliases are permanent or transitional.
- 🔵 minor (medium) — *Reference page hand-authors several primitives instead of
  reusing the live component* (Phase 9): breadcrumbs/table/tier-pills/toasts risk
  reference↔runtime drift and the VR oracle snapshots the copy. Prefer
  presentational-prop refactors; document where unavoidable.
- 🔵 minor (low) — *Three activation channels with overlapping responsibilities*
  (Phase 4): the chord re-derives `pathname === "/dev"` inline, duplicating the "in
  dev?" predicate. Centralise it.

### Code Quality

**Summary**: Well-structured for delivery — 11 mergeable phases, TDD throughout,
sensible primitive extraction, counts bound to live constants. The main
maintainability risks are concentration: a single 24-section `DevDesignSystem.tsx`
(the convention is one ~60-line file per showcase), and an activation feature
fragmented across five files coordinating through implicit invariants (hashchange
doesn't fire; priorRef vs sessionStorage as dual exit sources). Neither is fatal,
but both deserve an explicit decomposition decision in the plan.

**Strengths**:
- Constants centralised in `dev-constants.ts`, avoiding hint/handler drift.
- Per-section counts bind to live source constants, mutation-resistant.
- Genuine SRP refactors (AtomicMark out of Brand; one Icon for ~18 duplicates).
- Consistently test-driven, reusing the existing keybind test template.
- Deliberate deviations each named with rationale.

**Findings**:
- 🟡 major (high) — *Single DevDesignSystem.tsx concentrates 24 sections + scroll-spy
  + theme wiring into one god-component* (Phases 5–9): inverts the per-showcase file
  convention, forces whole-page mounts for unit tests, and conflicts the parallel
  content phases. Decompose into shell + `sections/<Section>.tsx` + `use-scroll-spy`.
- 🟡 major (high) — *Activation feature fragmented across five files coordinating via
  implicit hashchange invariant* (Phase 4): centralise behind
  `enterDev()`/`exitDev()`/`isDevActive`; add a guard/test for the no-re-entry
  invariant.
- 🔵 minor (high) — *Exit-target state has two sources and bypasses safe-storage*
  (Phase 4): define a reconciliation rule; route persistence through
  `safe-storage`/`storage-keys`.
- 🔵 minor (medium) — *Scroll-spy "recompute from all observed positions" is the most
  complex logic but under-specified* (Phase 5): extract to a pure function behind a
  hook; state the selection + tie-break rule.
- 🔵 minor (medium) — *Re-authoring ~550 lines of ds-* CSS risks dead/duplicated
  declarations* (Phase 4): port selectively (drop the undefined `.ds-spec*` family);
  co-locate section CSS if decomposed.

### Correctness

**Summary**: The central scroll-spy correction (recompute from all observed
positions) is directionally sound and the documented non-re-entrancy claims hold, so
scroll-spy hash writes correctly avoid re-entering the bridge; `DEV_ALIAS_RE` /
`aliasSection` are correct for every stated transition and the `\b` guard rejects
`#development` false matches. However two real state-correctness defects surface — a
pushState-based Back-button trap and the ported `offsetTop - offset` deep-link math
assuming an `offsetParent` the live scroll container doesn't guarantee — plus several
lower-impact boundary gaps.

**Strengths**:
- Correctly diagnoses the prototype's pinned-to-Colours root cause and prescribes
  the right class of fix.
- The `history.replaceState` non-re-entrancy claim is spec-accurate, and correctly
  extended to TanStack's pushState-based `router.navigate`.
- `DEV_ALIAS_RE` / `aliasSection` traceable-correct for all four transitions; `\b`
  rejects `#development`.
- The deep-link reader anticipates the alias-slipped-through race; Phase 3 correctly
  replaces Brand's hardcoded `id="hexg"` with per-instance `useId()`.

**Findings**:
- 🟡 major (high) — *pushState-based alias normalisation traps the browser Back
  button* (Phase 4): use `replace: true`; add a back-navigation test.
- 🟡 major (medium) — *`offsetTop - offset` deep-link scroll assumes an offsetParent
  the live scroll root doesn't guarantee* (Phase 5): `.main` declares no `position`;
  set `position: relative` or use a `getBoundingClientRect`-based delta.
- 🟡 major (medium) — *Active-section pick is under-specified at the active-region
  boundaries* (Phase 5): define a total order for the no-heading-in-band and
  tall-section cases.
- 🔵 minor (high) — *Unenforced invariant: no section id may begin with `dev`*
  (dev-constants.ts): add a unit assertion.
- 🔵 minor (medium) — *Overview hash write contradicts itself* (`#overview` vs cleared
  hash, Phase 5): pick one canonical overview form and assert it.
- 🔵 minor (medium) — *Escape exit not guarded against editable targets* (Phase 4):
  guard with `isEditableTarget`.
- 🔵 minor (medium) — *priorRef never set for non-alias entries, degrading exit
  restore to /library* (Phase 4): capture the last non-`/dev` pathname on every
  location change.

### Test Coverage

**Summary**: Unusually test-forward — every phase has an Automated Verification
block, count oracles bind to live constants, and the keybind/triple-click/bridge
unit tests reuse a proven template. But the headline coverage-preservation goal has a
concrete gap: the existing `code-block-resolved-colours.spec.ts` asserts eight
language cells plus diff overrides, while Phase 9 ports only TypeScript + Bash, so
repointing that spec at `/dev` either fails or silently loses coverage. The
scroll-spy e2e tests and the manual-only cross-browser chord matrix are also
under-specified against flakiness and regression risk.

**Strengths**:
- Count oracles bind to live constants — mutation-resistant.
- Keychord tests include negative cases and reuse the established preventDefault
  template.
- Triple-click test exercises the timing boundary.
- The bridge test enumerates the distinct path transitions.
- Phase 10 identifies the filename-keyed snapshot coupling and the
  full-page→clipped-locator conversion, gated by concrete checks.

**Findings**:
- 🔴 critical (high) — *Code-block VR coverage shrinks from 8 languages to 2;
  resolved-colour spec will lose cells* (Phase 9/10): reconcile the variant set or
  document dropped assertions and migrate the rest.
- 🟡 major (medium) — *Scroll-spy e2e assertions under-specified for determinism*
  (Phase 5): require retrying assertions + explicit settle; no `waitForTimeout`.
- 🟡 major (medium) — *"Highlight is never pinned" lacks a dedicated
  failing-then-passing test* (Phase 5): assert the active entry leaves Colours for
  the next short section.
- 🟡 major (high) — *Showcase→section coverage-preservation audit is manual-only*
  (Phase 10): add an automated cell-presence gate.
- 🟡 major (medium) — *Manual-only cross-browser chord matrix gives no regression
  protection* (Manual step 5): accept as the AC gate but bind `DEV_CHORD` to a
  "re-run the matrix if changed" note and record exact browser+version+OS.
- 🔵 minor (medium) — *Chord negatives miss wrong-key/partial-chord/case cases*
  (Phase 4).
- 🔵 minor (medium) — *No tests for Escape scope, sessionStorage cold-load exit, or
  the no-re-enter-sync guard* (Phase 4/5).
- 🔵 minor (medium) — *Colours theme-responsiveness asserts count, not change between
  themes* (Phase 6): promote to a computed-style assertion.
- 🔵 minor (low) — *Icon-geometry baseline regen blesses whatever renders* (Phase 2):
  retain targeted geometry unit assertions.

### Standards

**Summary**: Adheres closely to project conventions — the Icon primitive copies the
live SortPill/FilterPill authoring convention, its `ariaLabel`→`role=img` flip
matches the Glyph a11y contract, the uncrumbed `/dev` route matches the showcase
registration, and radii/shadows target live token names (no fictional
`--shadow-brand`). The main gaps are accessibility omissions in the chrome
(scroll-spy TOC active item never specifies `aria-current` despite a strong
precedent; triple-click/chord-only activation lacks keyboard equivalents) and a few
component-convention loose ends.

**Strengths**:
- Icon precisely matches the live 24×24 / currentColor / 2px-rounded convention,
  citing byte-identical prototype↔live paths.
- The `ariaLabel`→`role="img"` / `aria-hidden` flip reproduces the established Glyph
  a11y contract.
- The uncrumbed `/dev` route matches the showcase-route convention; "404" kept as
  default not-found, no fictional `notFoundComponent`.
- Radii/shadows re-expressed against live token names (ADR-0039/0026/0035).
- Counts bind to live constants; AtomicMark keeps Brand visually unchanged.
- `dev-constants.ts` enforces the single-source-of-truth chord hint.

**Findings**:
- 🟡 major (high) — *Scroll-spy TOC active item lacks `aria-current`* (Phase 4/5):
  set `aria-current="location"`; assert it.
- 🟡 major (medium) — *Triple-click and chord-only activation have no
  keyboard/pointer-agnostic equivalent* (Phase 4): make the keyboard-equivalence
  decision explicit; consider a focusable host.
- 🔵 minor (high) — *Icon omits the `framed` escape hatch the eyebrow migration
  needs* (Phase 1/2): keep custom or wrap in `IconFrame`; state which.
- 🔵 minor (medium) — *New constants modules diverge from `.constants.ts` naming*
  (Phase 1/4): prefer `Icon.constants.ts`.
- 🔵 minor (medium) — *Bare `aria-hidden` JSX shorthand diverges from explicit
  `aria-hidden="true"`* (Phase 1).
- 🔵 minor (low) — *Confirm migrated icons preserve `data-icon` and toast
  role/dismiss contracts* (Phase 2).

### Compatibility

**Summary**: Unusually compatibility-aware — hedges the keychord behind a
cross-browser matrix with a named fallback, binds VR contracts to stable
testids/screenshot names, and bounds icon-geometry drift as intentional. The two
genuine contract risks are the "returns 404" AC (the app has no `notFoundComponent`,
so removal yields an SPA not-found over HTTP 200, not a real 404) and the
"byte-identical Brand output" claim (contradicted by the per-instance `useId()`
gradient id). Cross-platform concerns (CSR confirmed, per-OS VR baselines) are
handled correctly.

**Strengths**:
- Keychord interceptability treated as a runtime/browser-version property with a
  manual matrix and a fallback isolated behind two constants.
- `Cmd/Ctrl+Shift+D` correctly excluded (Chromium-reserved, ignores preventDefault).
- VR baseline coupling handled (git mv of folders, stable names, both OSes,
  GITHUB_TOKEN caveat).
- Glyph size-union widening is purely additive.
- Icon-geometry drift explicitly framed as intentional and bounded.

**Findings**:
- 🟡 major (high) — *"Returns 404" is only an SPA not-found over HTTP 200* (Phase 11):
  amend the AC wording to "resolves to the SPA not-found UI (no redirect)".
- 🟡 major (high) — *"Byte-identical Brand output" contradicted by `useId()` gradient
  id* (Phase 3): reword to "pixel-identical"; normalise the id in any snapshot
  (useId ids are non-deterministic).
- 🔵 minor (medium) — *`Cmd/Ctrl+Shift+L` low-risk but `e.key` is layout-sensitive*
  (Phase 4): consider `e.code === "KeyL"` or note the caveat.
- 🔵 minor (high) — *`playwright.glyph.config.ts` health url → `/dev` resolves via
  vite-preview SPA fallback* (Phase 10): keep `testMatch` synced to the renamed
  specs.
- 🔵 minor (high) — *`useId()` for gradient ids is safe (pure CSR, no hydration)*
  (Phase 3): just don't assert the literal id string.

### Usability

**Summary**: Strong DX — IconProps faithfully mirror the Glyph/BigGlyph conventions,
the chord and exit affordances are consistent and tested, and the README rewrite is
scoped. The main gaps are discoverability of the three activation triggers (no in-app
affordance beyond a hidden title and a README line), an under-specified Icon DX
surface (no docstring, no unknown-name fallback), and a normalisation behaviour
(`#dev/colors` silently rewriting to `/dev#colors`) whose rationale is invisible at
the point of use.

**Strengths**:
- IconProps mirror the Glyph/BigGlyph DX contract — guessable without docs.
- `ICON_NAMES` tuple + `IconName` union give compile-time autocomplete and
  exhaustiveness.
- Exit affordances are redundant and consistent, all routing through one
  `exitDev()`; chord hint is a single shared constant.
- Counts bound to live constants keep the page self-updating.
- Deep-link normalisation self-heals stray forms; sessionStorage backs a cold-load
  exit target.

**Findings**:
- 🟡 major (high) — *Activation near-undiscoverable; no in-app affordance* (Desired
  End State / Phase 4 / Phase 11): add a low-cost discovery affordance + README
  itemisation.
- 🟡 major (medium) — *Icon lacks the consumer-contract docstring and unknown-name
  guidance Glyph models* (Phase 1): mirror Glyph's docstring + DEV warn.
- 🔵 minor (medium) — *`#dev/<section>` alias silently rewriting the URL can surprise*
  (Phase 4/5): document the canonical-vs-alias distinction.
- 🔵 minor (low) — *`size` as an open number diverges from Glyph's curated union*
  (Phase 1): document it as intentional.
- 🔵 minor (medium) — *Escape-to-exit could collide with Escape expectation in inputs*
  (Phase 4): apply the `isEditableTarget` guard. *(Also flagged by correctness.)*
- 🔵 minor (low) — *Keyboard-only focus management on activate/exit unspecified*
  (Phase 5): move focus to the heading on activate, restore on exit.

### Documentation

**Summary**: Strong on test-as-documentation (live-constant oracles) and includes a
clear Acceptance-criteria note explaining the canonical-URL departure, but has
several real gaps: the recurring "documented as a deviation" phrase is never anchored
to an authoritative home; Phase 11's README scope is narrower than the plan's own
Desired End State (omits the three triggers, the chord, the deep-link slug form, and
the second README block); and the plan notes the work item's ACs "should be amended"
without owning the edit.

**Strengths**:
- The Acceptance-criteria note transparently documents the canonical-URL departure.
- Per-section criteria bind counts to live constants and flag the work item's stale
  parentheticals as advisory.
- The page is itself reference documentation with self-explanatory labels and a live
  theme-flip control.
- Manual step 5 gives the cross-browser matrix a named home (work item / PR).

**Findings**:
- 🟡 major (high) — *"Documented as a deviation" names no authoritative home*
  (multiple phases): name a home; an in-page per-section note is most discoverable.
- 🟡 major (high) — *Phase 11 README scope omits the triggers, chord, and deep-link
  slug form* (Phase 11): expand the success criteria to itemise them.
- 🟡 major (medium) — *Plan notes ACs "should be amended" but does not own editing the
  work item* (Desired End State / Phase 11): make the amendment an owned step and
  reconcile stale counts.
- 🔵 minor (high) — *README "Glyph showcase specs" block + VR baseline commands need a
  broader update than Phase 11 states* (Phase 11, §3): update spec filenames + the
  config URL.
- 🔵 minor (medium) — *New cross-cutting modules lack a stated documentation
  expectation for non-obvious behaviour* (Phase 4): capture the "why" at call sites.
- 🔵 minor (medium) — *Section slug-vs-label divergence is reader-facing but not
  flagged for on-page legibility* (Phase 4): surface the slug; document the mapping.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-13

**Verdict:** COMMENT

The revision is in good shape. The single critical finding and **18 of 20 major
findings are resolved**; the two remaining "majors" are an accepted, documented
tradeoff (single-file `DevDesignSystem.tsx`, with the scroll-spy logic extracted as
a pure helper) and an inherently-manual concern (the cross-browser chord matrix,
now carrying a re-run-on-edit note). Almost all minors are resolved too. The edits
introduced **two new major issues**, both quick consistency fixes rather than
structural problems, plus a handful of minor follow-ons. Because there is no
critical and fewer than three open majors, the verdict is COMMENT — the plan is
acceptable to implement, and the two new majors are worth closing first.

### Previously Identified Issues

**Critical**
- 🔴 **Test Coverage**: Code-block VR coverage shrinks 8→2 — **Resolved** (Phase 9
  now enumerates all 8 languages + the diff-override fixture; success criteria and
  the cell-presence gate cover them).

**Major**
- 🟡 **Architecture**: Router-singleton circular import — **Resolved** (in-context
  `useRouter()`/`useNavigate()` hooks; verified Breadcrumbs uses the same pattern).
- 🟡 **Architecture**: Hash read/write loop invariant — **Resolved** (`replace:true`
  + re-entrancy guard + unit-tested invariant).
- 🟡 **Code Quality**: God-component `DevDesignSystem.tsx` — **Partially resolved /
  accepted** (single-file by decision; testability mitigated by the pure
  `pickActiveSection` helper + hook extraction — not re-raised).
- 🟡 **Code Quality**: Fragmented activation — **Resolved** (`enterDev`/`exitDev`/
  `isDevActive` surface).
- 🟡 **Correctness**: pushState Back-button trap — **Resolved** (`replace:true`,
  back-navigation test).
- 🟡 **Correctness**: `offsetTop` deep-link math — **Resolved** (`getBoundingClientRect`
  delta, offsetParent-independent).
- 🟡 **Correctness**: Active-section pick under-specified — **Resolved** (total-order
  `pickActiveSection` + fabricated-rect unit tests).
- 🟡 **Test Coverage**: Scroll-spy e2e determinism — **Resolved** (retrying
  assertions, no `waitForTimeout`).
- 🟡 **Test Coverage**: "Never pinned" no dedicated test — **Resolved** (dedicated
  leaves-Colours regression).
- 🟡 **Test Coverage**: Manual-only coverage audit — **Resolved** (automated
  cell-presence gate).
- 🟡 **Test Coverage**: Manual-only chord matrix — **Partially resolved / inherent**
  (re-run-on-edit note; browser-reservation detection can't be unit-tested).
- 🟡 **Standards**: TOC lacks `aria-current` — **Resolved** (`aria-current="location"`
  + focus management; one minor token-choice note below).
- 🟡 **Standards**: No keyboard equivalent — **Resolved** (chord is the explicit
  keyboard path via `e.code`; discoverability via foot label + console hint + README).
- 🟡 **Compatibility**: "Returns 404" — **Resolved** (SPA-not-found/HTTP-200 stated;
  AC9 amendment owned in Phase 11).
- 🟡 **Compatibility**: Brand byte-identical — **Resolved** ("pixel-identical" +
  CSR/`useId()` rationale + normalised snapshot).
- 🟡 **Usability**: Activation undiscoverable — **Resolved** (visible foot label +
  DEV console hint + README itemisation; production surfacing scoped out).
- 🟡 **Usability**: Icon lacks docstring/unknown-name — **Resolved** (docstring + DEV
  warn mirroring Glyph).
- 🟡 **Documentation**: "Deviation" has no home — **Resolved** (in-page Overview
  "Deviations" aside + per-section notes).
- 🟡 **Documentation**: README scope — **Resolved** (Phase 11 §3 itemises triggers/
  chord+fallback/canonical+alias form/slug-vs-label).
- 🟡 **Documentation**: AC amendment unowned — **Resolved** (Phase 11 §4 owns the
  edit to work item 0083).

**Minor** — substantially all resolved (selective CSS port, `Icon.constants.ts`
naming, explicit `aria-hidden`, framed eyebrow icons via `IconFrame`, migrated-icon
contracts, `data-scroll-root` seam, no-section-id-`dev` invariant, overview-hash
clear, Escape `isEditableTarget` guard, prior-path-on-every-change, chord negatives,
Colours computed-style assertion, icon-geometry assertions, `e.code` matching,
glyph-config URL, slug-in-title, bridge comments). Partial: the prior-path
persistence and the README "Glyph showcase specs" block — see new issues.

### New Issues Introduced

- 🟡 **Major — Correctness / Code Quality / Architecture (3 lenses)**: **Storage
  backend is self-contradictory.** Phase 4 says the prior path is persisted via the
  `safe-storage` helpers (`safeGetItem`/`safeSetItem`) **but those wrap
  `localStorage`**, while the same paragraph and the success criteria say
  `sessionStorage`. A `localStorage`-backed exit target is shared across tabs and
  survives sessions, so a cold-load deep-link could restore a stale/foreign route.
  Pick one backend and state it consistently (session-scoped matches the intent — a
  small guarded `sessionStorage` wrapper, since the existing helper is
  localStorage-only).
- 🟡 **Major — Test Coverage**: **Migrated glyph pixel spec asserts sizes the
  section no longer renders.** `glyph-showcase.spec.ts` loops `16/24/32`, but Phase
  7 renders doc-type glyphs at `22/28/36/48` (retaining only the 24px cell for
  `glyph-resolved-fill`). Phase 10's cell-presence gate uses a generic
  `glyph-cell-<type>-<size>` placeholder, so it won't catch the drift. State the new
  size set the renamed spec loops over and pin concrete sizes in the gate.
- 🔵 **Minor — Correctness**: Re-entrancy guard is under-specified about *which*
  programmatic writes set the "last hash" ref — a naive impl that records
  `enterDev()`'s `location.hash="#dev"` write would suppress chord/triple-click
  activation. Specify that only the bridge's own `replaceState`/scroll-spy writes
  set the ref, and test both directions.
- 🔵 **Minor — Architecture**: `data-scroll-root` introduces an implicit
  RootLayout↔dev-page coupling that breaks silently if dropped — centralise the
  attribute name as a shared constant and DEV-warn when it resolves to nothing.
- 🔵 **Minor — Architecture**: `enterDev()`/triple-click still write `location.hash`
  directly while `exitDev()`/normalisation call `navigate()` — consider having
  `enterDev()` navigate directly, leaving the hash bridge only for external alias
  URLs.
- 🔵 **Minor — Compatibility**: `e.code` is a net-new pattern; the existing keybind
  test template builds events with `key` only (so `code===""`). Chord tests must
  populate `event.code`, and the matrix should verify `code` delivery + the
  no-`KeyL`-position layout edge.
- 🔵 **Minor — Usability**: `DEV_CHORD_HINT="⌘⇧L"` (mac glyphs) under-communicates the
  cross-platform chord to Windows/Linux maintainers in the marquee/footer/console
  hint — carry both forms or platform-detect.
- 🔵 **Minor — Usability**: The `Cmd/Ctrl+Shift+G` fallback wording in the README is
  hand-written, not constant-bound — add a matrix-step check to re-verify README/hint
  against the final `DEV_CHORD_HINT` if the fallback is taken.
- 🔵 **Minor — Usability**: Triple-click has no in-app feedback (only the hover
  `title`) — weak affordance; acceptable for a dev tool, or add a subtle cue.
- 🔵 **Minor — Standards**: `aria-current="location"` diverges from the codebase's
  `"page"` token (arguably more correct for a scroll-spy) — add a one-line note so it
  isn't "corrected" back.
- 🔵 **Minor — Test Coverage**: Overview hash-clearing (bare `/dev`, no `#overview`)
  has no asserted test; focus-restore-on-exit isn't asserted across all three exit
  paths.
- 🔵 **Minor — Documentation**: Phase 11 §3 should also fix the README's stale
  `0037-glyph-component.md` doc-link (→ 0083) and the now-false "3 supported sizes
  (16/24/32)" glyph claim; and state the Overview deviations aside is the union of
  all per-section notes.

### Assessment

The plan is now sound and ready to implement. The critical coverage regression and
the high-confidence correctness/architecture defects from pass 1 are genuinely
fixed (verified against the live codebase, not just asserted). The two new majors
are both small consistency fixes — pin the storage backend, and align the glyph
pixel-spec size set with the redesigned section — that should be closed before
implementation to avoid an exit-restore correctness bug and a VR coverage gap. The
remaining minors are polish. Recommend addressing the two new majors (and, cheaply,
the re-entrancy-guard and README-accuracy minors), after which the plan is
implementation-ready without a further full review pass.

## Approval — 2026-06-13

**Verdict:** APPROVE

The two pass-2 majors and the associated minors were addressed by a follow-up polish
edit to the plan:

- **Glyph sizes** — the Doc-type-glyphs section converges on the codebase's existing
  `16/24/32` plus a net-new `48` (not the prototype's `22/28/36/48`), so the migrated
  glyph pixel baselines stay byte-stable and only the `48` cell is net-new; the
  Glyph union widens by just `48`; the cell-presence gate pins
  `glyph-cell-<type>-{16,24,32,48}`.
- **Storage backend** — the exit-target prior path now uses session-scoped storage
  (a guarded `safeSessionGet/Set` wrapper) rather than the localStorage-backed
  `safe-storage`, fixing the contradictory backend description.
- **Activation flow** — `enterDev()` navigates directly to `/dev` (the hash bridge
  handles only external alias URLs), which makes the re-entrancy guard unambiguous
  (it records only the bridge's own writes, so it cannot suppress activation).
- **Minors** — `DEV_CHORD_HINT` is platform-resolved; `e.code` flagged as a net-new
  test pattern; `aria-current="location"` documented as a deliberate divergence;
  overview hash-clearing and per-exit-path focus restore now asserted; Phase 11
  README scope extended to fix the stale `0037` link and the false "3 supported
  sizes" claim and to re-verify the chord wording on fallback; the deviations aside
  is the explicit union of per-section notes.

With the critical finding, all 20 majors (18 fixed, 2 accepted tradeoffs), and the
new pass-2 findings resolved, the plan is **approved** and ready for implementation.
The work item's acceptance-criteria amendment (AC3/AC5/AC7/AC9 + stale count
parentheticals) remains an owned step in Phase 11.
