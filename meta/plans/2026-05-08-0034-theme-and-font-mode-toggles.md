---
date: "2026-05-08T19:00:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0034-theme-and-font-mode-toggles.md"
status: draft
---

# 0034 Theme and Font-Mode Toggles Implementation Plan

## Overview

Deliver light/dark theming and a `default`/`mono` font swap in the
visualiser frontend. Both halves are implemented as
attribute-on-`<html>` + React-context-backed hook + topbar toggle button.
User choice persists across reloads via `localStorage`, falling back to
the OS `prefers-color-scheme` media query on first visit, and is applied
to `<html>` by an inline parser-blocking boot script in `<head>` to
prevent flash-of-wrong-theme.

The plan is organised around test-driven development. Each phase writes
failing tests first, then the smallest production change that makes them
pass. The work splits into ten phases:

1. **Foundational dark-mode token consumption** — fix the inert-tokens
   bug in 0033 (body/html declare no `background-color`/`color`; route
   `.title` rules declare no `color`). Without this the toggle ships a
   broken dark mode regardless of attribute wiring.
2. **`[data-font="mono"]` CSS block** — author the font-mode override
   block deferred from 0033, plus parity test.
3. **`useTheme` hook + `ThemeContext`** — context-backed hook reading and
   writing `data-theme` on `<html>` and persisting to `localStorage`.
4. **`useFontMode` hook + `FontModeContext`** — same shape, for
   `data-font`.
5. **Inline boot script + `index.html` updates** — parser-blocking
   classic `<script>` as first child of `<head>`, sourced from a
   shared TS module so its behaviour is unit-testable.
6. **`ThemeToggle` component** — sun/moon icon button.
7. **`FontModeToggle` component** — `Aa`/mono glyph button.
8. **Topbar wiring** — populate the two `data-slot` divs with the new
   toggles; update the existing emptiness assertions.
9. **Re-capture dark visual-regression baselines** (manual) — the
   existing `*-dark-*.png` baselines were captured against the broken
   inert-token render and must be re-captured once Phase 1 lands.
10. **Reconcile work item AC wording** — the work item AC currently says
    `data-font="default"` but the prototype inventory and this plan use
    `data-font="display"`. Align the work item.

The toggle is binary `light` ↔ `dark` (work item AC is explicit) and
binary `display` ↔ `mono` for the font; there is no three-state
"system" toggle in the UI. The `prefers-color-scheme` mirror in
`global.css` *is* the system path: when the user has no stored
preference, the boot script writes nothing and the mirror governs
paint — so the user's OS preference continues to track dynamically
across reloads (and across runtime OS theme switches under any
browser that re-evaluates `prefers-color-scheme` live, which all
evergreens do). Once the user clicks a toggle, the boot script's
write pins the explicit attribute on subsequent visits.

## Current State Analysis

The frontend at
`skills/visualisation/visualise/frontend/` is Vite + React 19 + TanStack
Router + CSS Modules, tested with Vitest + React Testing Library + jsdom
and Playwright for visual regression.

### What 0033 has shipped

`src/styles/global.css` (214 lines) contains:

- Self-hosted woff2 `@font-face` declarations for Sora 600/700, Inter
  400/500/600/700, Fira Code 400/500 (lines 4–59).
- Full `:root` token suite at lines 61–137 (`--ac-*` colours, `--ac-font-display`,
  `--ac-font-body`, `--ac-font-mono`, type/spacing/radius/shadow scales,
  `--ac-topbar-h`).
- `[data-theme="dark"]` MIRROR-A block at lines 144–166.
- `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { ... } }`
  MIRROR-B block at lines 172–197 — byte-equivalent to MIRROR-A and used
  as the OS-preference fallback today.
- `:focus-visible` (lines 199–202), `forced-colors` override (lines
  204–208), `@keyframes ac-pulse` (lines 210–213).

`src/styles/tokens.ts` exports typed maps mirroring the CSS:
`LIGHT_COLOR_TOKENS`, `DARK_COLOR_TOKENS`, `TYPOGRAPHY_TOKENS`,
`SPACING_TOKENS`, `RADIUS_TOKENS`, `LIGHT_SHADOW_TOKENS`,
`DARK_SHADOW_TOKENS`, `LAYOUT_TOKENS`.

`src/styles/global.test.ts` (190 lines) enforces token parity using a
flat-block `readCssVar` regex, with a separate brace-balanced parity
helper for the MIRROR-A↔MIRROR-B equivalence check (lines 124–162).

`src/styles/migration.test.ts` enforces no raw hex/px/rem/em outside
`EXCEPTIONS`, no two-arg `var(--token, fallback)` form, the
`color-mix()` 8/18/30 percentage ladder, and an aggregate
`AC5_FLOOR=408, AC5_TARGET=300` ratchet on `var(--*)` references.

### What 0035 has shipped

`src/components/Topbar/Topbar.tsx:7-20` renders Brand → divider →
Breadcrumbs → spacer → OriginPill → SseIndicator → two empty
`<div className={styles.slot} data-slot="theme-toggle" />` and
`<div className={styles.slot} data-slot="font-mode-toggle" />`.

`Topbar.module.css:20-24` collapses empty slots:

```css
.slot:empty {
  width: 0;
  height: 0;
  overflow: hidden;
}
```

The rule self-disengages once a child is inserted, so 0034 does not need
to clear any class or attribute.

`Topbar.test.tsx:57-69` asserts both slots exist and have zero children
— assertions that this plan will replace with content assertions.

### Inert-token bug in 0033 (must fix before toggle is usable)

Although the `[data-theme="dark"]` and `prefers-color-scheme` overrides
exist, the page chrome consumes none of the `--ac-*` tokens at element
level:

- `global.css:1-2` is the entirety of element-level base styling:
  `*, ::before, ::after { box-sizing: border-box; }` and
  `body { margin: 0; }`. There is no `background-color`, no `color`, no
  `color-scheme` declaration anywhere. Result: the canvas stays browser-
  default white in dark mode.
- `RootLayout.module.css:1-17` styles `.root`, `.body`, `.main` for
  layout only — no `background-color`/`color`. Once `body` consumes
  `var(--ac-bg)`/`var(--ac-fg)`, `<main>` will inherit correctly via the
  cascade, so this can be addressed by the body fix alone.
- `LibraryDocView.module.css:9` declares `.title { font-size: 1.6rem;
  font-weight: 700; margin: 0 0 var(--sp-2); }` — no `color`. The
  matching `<h1 className={styles.title}>` renders near-black on
  near-black in dark mode.
- `LibraryTemplatesView.module.css:2` is the same shape — `.title` with
  no `color`.
- `LibraryTemplatesIndex.tsx:31` renders a bare `<h1>Templates</h1>`
  with no class. Once `body` declares a `color`, this will inherit
  `var(--ac-fg)`, but the heading wants `var(--ac-fg-strong)` to match
  other route headings (`KanbanBoard.module.css:7-12` and
  `LifecycleClusterView.module.css:24` already do this).
- `tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/
  *-dark-*.png` baselines were captured against the broken render (white
  canvas, near-black headings under Playwright `colorScheme: dark`
  emulation). They will fail once the canvas inverts and must be
  re-captured.

The migration test cannot catch this class of bug — it is a *negative*
literal-grep, not a positive coverage check. A CSS file with no `color`
declaration at all passes trivially. This plan adds positive assertions
(Phase 1 tests) and will rely on the re-captured visual regression
suite to catch future regressions.

### React context pattern (canonical model to imitate)

`src/api/use-doc-events.ts` defines the codebase's canonical pattern,
end-to-end:

1. Public `Handle` interface (lines 15–19).
2. Factory function `makeUseDocEvents(createSource, registry)` so tests
   can inject fakes (lines 86–160).
3. Production hook bound once: `export const useDocEvents =
   makeUseDocEvents((url) => new EventSource(url))` (line 163).
4. Default handle for context-default consumers (lines 165–169).
5. `export const DocEventsContext = createContext<Handle>(_default)`
   (line 171).
6. Consumer hook: `export function useDocEventsContext() { return
   useContext(DocEventsContext) }` (lines 173–175).

Provider wiring at `RootLayout.tsx:18-29`:
`<DocEventsContext.Provider value={docEvents}> ... </Provider>`. There
is no separate `Provider` component — the call site uses
`Context.Provider` directly. 0034's `ThemeContext` and `FontModeContext`
will follow this pattern verbatim.

Test injection: `use-doc-events.test.ts:122-158` uses
`makeUseDocEvents(vi.fn())` to inject fake factories. Consumer tests
(`Topbar.test.tsx:6-8`) `vi.mock(...)` on the hook's import path and
set return value per test.

### Test infrastructure gotchas

- `src/test/setup.ts` stubs `EventSource`, `ResizeObserver`, `scrollTo`,
  `scrollIntoView` via `Object.defineProperty` so they survive
  `vi.unstubAllGlobals()`. It does NOT touch `localStorage` or
  `documentElement`.
- jsdom provides a real `localStorage`. `vite.config.ts` sets
  `restoreMocks: true` but that does not clear storage. **0034 hook
  tests must explicitly call `localStorage.clear()` and reset
  `document.documentElement.removeAttribute('data-theme'/'data-font')`
  in `beforeEach`/`afterEach`** so tests don't leak state.
- `tests/visual-regression/tokens.spec.ts:29` already uses
  `document.documentElement.dataset.theme = 'dark'` — confirms the
  attribute name and writing path 0034 must use.

### Files this plan will touch

**Create:**
- `src/api/safe-storage.ts` + `safe-storage.test.ts` (Phase 0)
- `src/api/use-theme.ts` + `use-theme.test.ts`
- `src/api/use-font-mode.ts` + `use-font-mode.test.ts`
- `src/api/boot-theme.ts` + `boot-theme.test.ts` +
  `boot-theme.html.test.ts` (Phase 5)
- `src/components/TopbarIconButton/TopbarIconButton.tsx` +
  `TopbarIconButton.module.css` + `TopbarIconButton.test.tsx` (Phase 6.0)
- `src/components/ThemeToggle/ThemeToggle.tsx` +
  `ThemeToggle.module.css` + `ThemeToggle.test.tsx`
- `src/components/FontModeToggle/FontModeToggle.tsx` +
  `FontModeToggle.module.css` + `FontModeToggle.test.tsx`

**Modify:**
- `src/styles/global.css` — `body` bg/fg + `:root` `color-scheme`
  (Phase 1); `[data-font="mono"]` block (Phase 2).
- `src/styles/global.test.ts` — positive body/colour-scheme assertions
  (Phase 1); `[data-font="mono"]` parity test (Phase 2).
- `src/styles/tokens.ts` — `MONO_FONT_TOKENS` export (Phase 2).
- `src/routes/library/LibraryDocView.module.css` — `.title` `color`.
- `src/routes/library/LibraryTemplatesView.module.css` — `.title`
  `color`.
- `src/routes/library/LibraryTemplatesIndex.tsx` and
  `LibraryTemplatesIndex.module.css` — class up the bare `<h1>`.
- `vite.config.ts` — register the `bootThemePlugin` that inlines
  `BOOT_SCRIPT_SOURCE` into `index.html` at build time.
- `index.html` — no manual edits needed (plugin injects the script);
  no `suppressHydrationWarning` (CSR via `createRoot`).
- `src/components/RootLayout/RootLayout.tsx` — wrap with
  `ThemeContext.Provider` and `FontModeContext.Provider`.
- `src/components/Topbar/Topbar.tsx` — populate the two slot divs.
- `src/components/Topbar/Topbar.test.tsx` — replace emptiness
  assertions.
- `tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/
  *-dark-*.png` — re-capture (Phase 9, manual).
- `meta/work/0034-theme-and-font-mode-toggles.md` — `default` →
  `display` AC reconciliation (Phase 10).

## Desired End State

After this plan ships:

- A user clicking the theme toggle in the Topbar flips
  `<html data-theme="...">` between `light` and `dark`, the canvas and
  every route's content area inverts cleanly, and the choice survives a
  reload.
- A user clicking the font-mode toggle flips
  `<html data-font="...">` between `display` and `mono`, the entire app
  cascades to Fira Code in `mono` (including Brand wordmark and
  Breadcrumbs because `--ac-font-display` and `--ac-font-body` both
  remap to `--ac-font-mono`), and the choice survives a reload.
- On first visit with no `localStorage` entries, the boot script
  writes no attribute. The CSS `prefers-color-scheme` mirror in
  `global.css:172-197` paints the page according to the OS
  preference — and continues to track OS preference changes
  dynamically. The same applies for `data-font`: with no stored
  entry, no attribute is written, the existing `:root`
  `--ac-font-display`/`--ac-font-body` definitions apply, and the
  app paints in Sora/Inter.
- `localStorage`-unavailable browsers (private mode in Firefox/Safari)
  do not throw — the boot script's per-attribute `try/catch` swallows
  the `SecurityError` and the no-attribute path engages, so paint
  follows OS preference and the default font cascade.
- React's reconciler does not warn about the boot-script attribute
  writes because `src/main.tsx` mounts via `createRoot` (CSR, not
  hydration) into `<div id="root">`. The `<html>` element is never
  reconciled by React, so no hydration mismatch arises in the first
  place. No `suppressHydrationWarning` directive is required.
- All existing tests pass, plus the new tests added in each phase. The
  dark visual-regression baselines reflect the corrected render.

### Verification

- `npm run test` is green (unit + integration).
- `npm run build` succeeds (typecheck + Vite build).
- `npm run test:e2e` is green against re-captured baselines.
- Manual: open the app in a fresh profile, toggle theme and font, reload
  — preference persists, no flash. Switch OS dark mode while no
  preference is stored — app follows. Open in private-browsing — no
  console errors, OS-preference fallback works.

### Key Discoveries

- The `prefers-color-scheme` mirror at `global.css:172-197` is wrapped
  in `:root:not([data-theme="light"])`, so writing `data-theme="light"`
  in the boot script correctly suppresses the OS fallback. Writing
  `data-theme="dark"` is also fine — both paths converge on the same
  values via MIRROR-A. The "system" mode (no attribute) works at the
  CSS layer for free — and the boot script in Phase 5 deliberately
  *does not* write an attribute when there is no stored preference, so
  the no-attribute path remains the active rendering path for any
  user who hasn't toggled. The visual-regression test
  `tokens.spec.ts:78-87` continues to exercise this MIRROR-B branch.
- `--ac-font-display` and `--ac-font-body` are inherited via the
  `RootLayout.module.css:5` `font-family: var(--ac-font-body)` cascade.
  The `[data-font="mono"]` block remaps both tokens to
  `var(--ac-font-mono)`; components that explicitly use
  `--ac-font-display` (Brand wordmark) or `--ac-font-body` (most others)
  follow automatically. Components that already use `--ac-font-mono`
  (OriginPill, SseIndicator) stay mono in both modes — correct.
- The `global.test.ts` flat-block `readCssVar` helper has a
  *flat-block invariant* (no nested rules, no `@media`). Adding the
  `[data-font="mono"]` block won't break this because the helper only
  reads `:root` and `[data-theme="dark"]`. The brace-balanced
  `extractBlockBody` helper used for MIRROR parity is independent and
  also unaffected. A new positive assertion for `[data-font="mono"]`
  will need its own selector regex.
- `localStorage` keys: `ac-theme` and `ac-font-mode` (named in the 0035
  research and adopted here as canonical; no `localStorage` use exists
  anywhere else in `src/`, so collision is impossible).
- The 0033 plan deliberately excluded `[data-font="mono"]` from
  `tokens.ts` because font-mode is a token *remap*, not a new token
  family. This plan adds a `MONO_FONT_TOKENS` export anyway so the new
  parity test can iterate it the same way the existing parity tests do
  — keeps the test shape uniform.

## What We're NOT Doing

- **Not** introducing a three-state theme cycle (`light` → `dark` →
  `system`). Work item AC says binary; the plan honours it.
- **Not** syncing theme preference to the backend. `localStorage` only.
- **Not** synchronising state across browser tabs. Neither hook
  subscribes to the `storage` event, so two tabs that toggle theme
  independently will diverge until one is reloaded. Adding a
  `storage`-event listener is mechanical (a `useEffect` in each hook
  filtering by `e.key`); deferring until a third preference hook
  arrives so the listener is added once in a shared primitive.
- **Not** introducing a generic "preferences" abstraction. Each hook
  is dedicated to one attribute and one storage key. Future work
  (sidebar collapsed state, kanban filters) can extract a shared shape
  if and when a third consumer arrives.
- **Not** touching the `MarkdownRenderer.module.css` `#1e1e1e`/`#d4d4d4`
  code-block colours. They are admitted `irreducible` in
  `migration.test.ts:54-55` and stay editor-themed in both modes.
- **Not** auditing every component CSS file for missing `color`/
  `background-color` declarations beyond the four sites the research
  identified. Future drift will be caught by the re-captured visual
  regression suite (Phase 9).
- **Not** renaming `--ac-font-display`. The token name stays; only the
  attribute value `data-font="display"` aligns with the prototype
  inventory.
- **Not** extracting `ThemeProvider`/`FontModeProvider` wrapper
  components. The codebase pattern is to use `Context.Provider`
  directly at the `RootLayout` call site (see `use-doc-events.ts`).

## Implementation Approach

Each phase follows red → green → refactor:

1. Add new tests (or extend existing tests) for the next thin slice of
   behaviour. Run them and confirm they fail.
2. Make the smallest production change that turns the new tests green
   without breaking existing ones.
3. Refactor only if the green code has obvious duplication or readability
   problems.

The order is chosen so each phase compiles and tests pass on its own —
the toggle UI doesn't ship until Phase 8, and Phases 1–7 incrementally
add capability without changing user-visible behaviour.

Phase 1 lands first because it removes the inert-token bug; if it shipped
last, the toggle would technically work but produce a broken dark mode.
Phases 2–4 add CSS and hooks that have no user-visible effect until
Phases 5–8 wire them into the boot script and the Topbar.

Phase 9 is manual (Playwright baseline regeneration) and runs after
Phase 1 — it is sequenced last only because re-capturing dark
baselines once at the end is simpler than doing it twice.

## Phase 1: Foundational dark-mode token consumption

### Overview

Make the page chrome and the route titles consume `--ac-*` tokens so
the dark theme overrides actually invert the page. This is preparatory
work that must land before the toggle ships, otherwise dark mode is
broken regardless of attribute wiring.

### Changes Required

#### 1.1 Tests for body/html token consumption

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
**Changes**: append a new `describe('global body/html token consumption', ...)`
block. Use the existing `extractBlockBody` brace-balanced helper
(already used by the MIRROR-A↔MIRROR-B parity test) so the
assertions survive nested rules and don't rely on whether other
declarations sit between `{` and `}`.

```ts
// Same brace-balanced helper used for MIRROR parity, exposed if
// not already exported from global.test.ts. Returns the balanced
// body string of the *first* rule whose selector matches `selector`,
// or null if no such rule exists.
function findBlockBodyForSelector(css: string, selector: string): string | null {
  // Reuse extractBlockBody (already in this test file). Walk the
  // file looking for the selector, then extract its balanced body.
  const idx = css.indexOf(selector + ' ')
  if (idx === -1) return null
  return extractBlockBody(css, idx)
}

// Asserts there is exactly one top-level `body { ... }` rule. This
// makes the consolidation invariant explicit: a future contributor
// who adds a second body rule with a divergent declaration set
// breaks this test, not the page.
function countTopLevelBodyRules(css: string): number {
  // Strip @media/@supports blocks then count `body {` occurrences.
  const stripped = css.replace(/@[^{]+\{(?:[^{}]|\{[^}]*\})*\}/g, '')
  return (stripped.match(/(^|\s|,)body\s*\{/g) ?? []).length
}

describe('global body/html token consumption', () => {
  it('there is exactly one top-level body rule', () => {
    expect(countTopLevelBodyRules(globalCss)).toBe(1)
  })

  it('body declares background-color: var(--ac-bg)', () => {
    const body = findBlockBodyForSelector(globalCss, 'body')
    expect(body).not.toBeNull()
    expect(body!).toMatch(/background-color:\s*var\(--ac-bg\)/)
  })

  it('body declares color: var(--ac-fg)', () => {
    const body = findBlockBodyForSelector(globalCss, 'body')
    expect(body).not.toBeNull()
    expect(body!).toMatch(/(?<!background-)color:\s*var\(--ac-fg\)/)
  })

  it(':root declares color-scheme: light dark', () => {
    const root = findBlockBodyForSelector(globalCss, ':root')
    expect(root).not.toBeNull()
    expect(root!).toMatch(/color-scheme:\s*light\s+dark/)
  })
})
```

The `(?<!background-)` lookbehind on the `color` regex avoids false
matches inside `background-color`. The `countTopLevelBodyRules`
helper enforces consolidation as a tested invariant rather than a
prose instruction.

#### 1.2 Tests for route-level title token consumption

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: append a new `describe('Phase 1 (0034): route titles
consume --ac-fg-strong', ...)` block. Use a balanced-brace extraction
to find the rule by selector and assert declarations on its body —
the `[^{]*` lookahead in the original draft was brittle to any
unbalanced `{` in preceding rules.

```ts
// Reuse extractBlockBody from global.test.ts shape. If migration.test.ts
// does not yet have a shared brace-balanced extractor, add one here:
function extractBlockBody(css: string, startIdx: number): string | null {
  const open = css.indexOf('{', startIdx)
  if (open === -1) return null
  let depth = 1
  let i = open + 1
  while (i < css.length && depth > 0) {
    const ch = css[i]
    if (ch === '{') depth++
    else if (ch === '}') depth--
    i++
  }
  return depth === 0 ? css.slice(open + 1, i - 1) : null
}

describe('Phase 1 (0034): route titles consume --ac-fg-strong', () => {
  const REQUIRED = [
    { file: 'routes/library/LibraryDocView.module.css', selector: '.title' },
    { file: 'routes/library/LibraryTemplatesView.module.css', selector: '.title' },
    { file: 'routes/library/LibraryTemplatesIndex.module.css', selector: '.title' },
  ] as const

  for (const { file, selector } of REQUIRED) {
    it(`${file} ${selector} declares color: var(--ac-fg-strong)`, () => {
      const css = cssBySrcRelative.get(file)
      expect(css, `missing ${file}`).toBeDefined()
      const idx = css!.indexOf(selector)
      expect(idx, `selector ${selector} not found in ${file}`).toBeGreaterThanOrEqual(0)
      const body = extractBlockBody(css!, idx)
      expect(body, `body for ${selector} in ${file}`).not.toBeNull()
      expect(body!).toMatch(/(?<!background-)color:\s*var\(--ac-fg-strong\)/)
    })
  }
})
```

This requires `cssBySrcRelative` to be exposed/visible in the
`describe` block — it is currently a module-level `const` in
`migration.test.ts:289-292`, so the new `describe` can use it directly.

#### 1.3 Make the global tests green

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: extend the `body` rule and add a `color-scheme` declaration
to `:root`.

```css
*, ::before, ::after { box-sizing: border-box; }
body {
  margin: 0;
  background-color: var(--ac-bg);
  color: var(--ac-fg);
  font-family: var(--ac-font-body);
}
```

Note: `RootLayout.module.css:5` already declares
`font-family: var(--ac-font-body)` on `.root`, but applying it on
`body` covers the case where the boot script has applied
`data-font="mono"` before React mounts (so the body itself paints in
the correct font during the brief pre-React-mount window). This is
purely defensive — once React mounts, `.root` takes over.

Add `color-scheme` inside the existing `:root` block (immediately after
the `--ac-topbar-h: 48px;` declaration so it sits with other layout
concerns):

```css
:root {
  /* ... existing ... */
  --ac-topbar-h: 48px;

  color-scheme: light dark;
}
```

Add `color-scheme: dark` to `[data-theme="dark"]` and the
`prefers-color-scheme` mirror so the user-agent's built-in form
controls/scrollbars also flip. **The MIRROR-A↔MIRROR-B parity test
treats only `--*` declarations** (`global.test.ts:147` regex is
`/--([\w-]+):\s*([^;]+);/g`), so adding non-`--` declarations like
`color-scheme` to the two blocks does NOT trip the parity test. Add the
declaration consistently to both blocks anyway, for hygiene.

```css
[data-theme="dark"] {
  /* ...existing... */
  color-scheme: dark;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    /* ...existing... */
    color-scheme: dark;
  }
}
```

#### 1.4 Make the route-title tests green

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css`
**Change**: line 9.

```css
.title { font-size: 1.6rem; font-weight: 700; color: var(--ac-fg-strong); margin: 0 0 var(--sp-2); }
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css`
**Change**: line 2.

```css
.title { font-size: var(--size-lg); font-weight: 700; color: var(--ac-fg-strong); margin: 0 0 var(--sp-5); }
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.module.css`
**Change**: append a new `.title` rule.

```css
.title { font-size: var(--size-lg); font-weight: 700; color: var(--ac-fg-strong); margin: 0 0 var(--sp-5); }
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`
**Change**: line 31.

```tsx
<h1 className={styles.title}>Templates</h1>
```

The `--size-lg` (22px) value matches `LibraryTemplatesView.title`'s
weight class; using it (rather than the off-scale `1.6rem` of
`LibraryDocView`) keeps the templates index visually consistent with its
detail view and avoids introducing a new `irreducible` `1.6rem` entry
in `migration.test.ts`.

#### 1.5 AC5 ratchet bump (deferred)

The migration test's `AC5_FLOOR` is a coverage-health ratchet over
all `var(--*)` references in `*.module.css` / `*.global.css`. Phase 1
adds: `LibraryDocView.module.css .title` (+1 `--ac-fg-strong`),
`LibraryTemplatesView.module.css .title` (+1), and a brand-new
`.title` rule in `LibraryTemplatesIndex.module.css` (+3 — for
`--size-lg`, `--ac-fg-strong`, `--sp-5`). Net: +5 in the migration
glob. The body/`:root` additions in `global.css` are not in the glob
and don't contribute.

Subsequent phases also add token references (Phase 6: `ThemeToggle.module.css`,
Phase 7: `FontModeToggle.module.css`). To avoid three separate ratchet
bumps that each create a numeric-literal-only commit, **the
`AC5_FLOOR` is bumped exactly once after Phase 7 lands**, with a
single comment naming the work item. See Phase 7.4 for the bump.

For Phase 1, leave `AC5_FLOOR` unchanged. The migration test will
report a passing ratchet with a higher current count — that's
expected and intentional.

### Success Criteria

#### Automated Verification

- [ ] `npm run test src/styles/global.test.ts` passes the new
  `body/html token consumption` block.
- [ ] `npm run test src/styles/migration.test.ts` passes the new
  `Phase 1 (0034): route titles consume --ac-fg-strong` block.
- [ ] `npm run test` overall is green (no regressions).
- [ ] `npm run build` succeeds (typecheck + Vite build).

#### Manual Verification

- [ ] Run `npm run dev`, open the app at `localhost:5173`, manually set
  `document.documentElement.dataset.theme = 'dark'` in DevTools — canvas
  inverts to `var(--ac-bg)` (dark navy), all four route titles
  (`/library/{type}`, `/library/templates`, `/library/templates/{name}`,
  `/library/{type}/{slug}`) render in `var(--ac-fg-strong)` (white).
- [ ] Set `data-theme="light"` — canvas reverts to off-white, titles
  to `#0a111b`.

---

## Phase 2: `[data-font="mono"]` CSS block

### Overview

Author the font-mode CSS override block deferred from 0033. The block
remaps `--ac-font-display` and `--ac-font-body` to
`var(--ac-font-mono)` so cascade-driven font-family changes propagate
across the app.

### Changes Required

#### 2.1 Add `MONO_FONT_TOKENS` export

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Change**: append after `LAYOUT_TOKENS`.

```ts
// Font-mode override values — applied under [data-font="mono"]. Both
// display and body font tokens are remapped to the mono token so that
// the topbar wordmark, breadcrumbs, and body copy all switch to Fira
// Code together. Components that already reference --ac-font-mono
// directly (OriginPill, SseIndicator) are unaffected.
export const MONO_FONT_TOKENS = {
  'ac-font-display': 'var(--ac-font-mono)',
  'ac-font-body':    'var(--ac-font-mono)',
} as const

export type MonoFontToken = keyof typeof MONO_FONT_TOKENS
```

#### 2.2 Parity test

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
**Change**: import `MONO_FONT_TOKENS` and add a new `describe` block.

```ts
import {
  // ...existing...
  MONO_FONT_TOKENS,
} from './tokens'

// Reads a CSS custom property's declared value from the
// [data-font="mono"] block. Same flat-block invariant as readCssVar.
function readMonoVar(name: string): string | null {
  const blockRe = /\[data-font="mono"\]\s*\{([\s\S]*?)\}/
  const block = blockRe.exec(globalCss)?.[1] ?? ''
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const re = new RegExp(`--${escapedName}:\\s*([^;]+);`)
  return re.exec(block)?.[1].trim().toLowerCase() ?? null
}

describe('tokens.ts ↔ global.css [data-font="mono"] parity', () => {
  for (const [name, value] of Object.entries(MONO_FONT_TOKENS)) {
    it(`--${name} matches MONO_FONT_TOKENS.${name}`, () => {
      expectMatches(readMonoVar(name), value)
    })
  }
})
```

#### 2.3 Authour the CSS block

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Change**: insert after the `prefers-color-scheme` block (after line
197) and before `:focus-visible`.

```css
/* MIRROR-C: explicit [data-font="mono"] block — repoints the display
   and body font-family tokens to var(--ac-font-mono) so the cascade
   from .root flips the entire app to Fira Code. Components that
   already reference --ac-font-mono directly (OriginPill, SseIndicator)
   are unaffected. */
[data-font="mono"] {
  --ac-font-display: var(--ac-font-mono);
  --ac-font-body:    var(--ac-font-mono);
}
```

### Success Criteria

#### Automated Verification

- [ ] `npm run test src/styles/global.test.ts` passes the new
  `[data-font="mono"] parity` block.
- [ ] `npm run test src/styles/global.test.ts` parity tests for
  `:root` and `[data-theme="dark"]` still pass (no regression).
- [ ] `npm run test` overall is green.

#### Manual Verification

- [ ] Run `npm run dev`, set
  `document.documentElement.dataset.font = 'mono'` in DevTools — the
  Brand wordmark ("Accelerator", "VISUALISER"), Breadcrumbs labels, and
  all body copy render in Fira Code; OriginPill and SseIndicator (which
  used `--ac-font-mono` directly) look unchanged.
- [ ] Removing the `data-font` attribute restores Sora (display) and
  Inter (body).

---

## Phase 3: `useTheme` hook + `ThemeContext`

### Overview

Build the React-context-backed hook that owns `data-theme` state. The
hook reads the current attribute on mount, exposes a setter that
synchronously writes the attribute and persists to `localStorage`, and
falls back to OS preference + `localStorage` private-mode safety.
A small `safe-storage` helper module is extracted at the start of
this phase so both `useTheme` (this phase) and `useFontMode` (Phase 4)
can share the same try/catch wrapping.

### Changes Required

#### 3.0 Shared `safe-storage` helper

**File**: `skills/visualisation/visualise/frontend/src/api/safe-storage.ts`
**Changes**: new file — pure functions wrapping `localStorage` access.

```ts
/**
 * `localStorage.getItem` / `setItem` wrappers that swallow
 * `SecurityError` (Firefox/Safari private mode and similar).
 * Centralised so future consumers (sidebar collapsed, kanban filters,
 * etc.) inherit the same error-handling shape.
 */

export function safeGetItem(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

export function safeSetItem(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* private-browsing mode etc. — fall through silently */
  }
}
```

**File**: `skills/visualisation/visualise/frontend/src/api/safe-storage.test.ts`
**Changes**: new file.

```ts
import { describe, it, expect, afterEach, vi } from 'vitest'
import { safeGetItem, safeSetItem } from './safe-storage'

afterEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
})

describe('safeGetItem / safeSetItem', () => {
  it('round-trips a value through real localStorage', () => {
    safeSetItem('test-key', 'test-value')
    expect(safeGetItem('test-key')).toBe('test-value')
  })

  it('returns null when the key is missing', () => {
    expect(safeGetItem('missing-key')).toBeNull()
  })

  it('safeGetItem returns null when getItem throws SecurityError', () => {
    // Verify the spy is actually intercepted under jsdom (the
    // compatibility lens flagged this as worth checking).
    const spy = vi
      .spyOn(Storage.prototype, 'getItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    expect(safeGetItem('any-key')).toBeNull()
    expect(spy).toHaveBeenCalled()
  })

  it('safeSetItem does not throw when setItem throws SecurityError', () => {
    const spy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    expect(() => safeSetItem('any-key', 'any-value')).not.toThrow()
    expect(spy).toHaveBeenCalled()
  })
})
```

The `expect(spy).toHaveBeenCalled()` assertion guards against jsdom
implementations that bypass `Storage.prototype` and would otherwise
make the catch-path tests vacuously green.

#### 3.1 Test file

**File**: `skills/visualisation/visualise/frontend/src/api/use-theme.test.ts`
**Changes**: new file, comprehensive coverage.

```ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { makeUseTheme, type Theme } from './use-theme'

function resetDom(): void {
  document.documentElement.removeAttribute('data-theme')
  localStorage.clear()
}

describe('makeUseTheme', () => {
  beforeEach(resetDom)
  afterEach(resetDom)

  it('initial state reflects pre-existing data-theme attribute', () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('attribute takes precedence over conflicting localStorage', () => {
    // Locks in the boot-script ↔ React handoff: when the boot script
    // has written the attribute and storage disagrees (e.g. another
    // tab toggled), the attribute (= what the user actually sees) wins.
    document.documentElement.setAttribute('data-theme', 'dark')
    localStorage.setItem('ac-theme', 'light')
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
    // After mount, the useEffect must not clobber the attribute back
    // to whatever React initialised from a different source.
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('localStorage takes precedence over OS preference when no attribute', () => {
    localStorage.setItem('ac-theme', 'light')
    const useTheme = makeUseTheme(() => true) // OS prefers dark
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('light')
  })

  it('initial state reads localStorage when no attribute is present', () => {
    localStorage.setItem('ac-theme', 'dark')
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('initial state falls back to OS preference when no attribute or storage', () => {
    const useTheme = makeUseTheme(() => true) // OS prefers dark
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('setTheme writes the attribute on <html>', () => {
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    act(() => result.current.setTheme('dark'))
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('setTheme persists to localStorage under "ac-theme"', () => {
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    act(() => result.current.setTheme('dark'))
    expect(localStorage.getItem('ac-theme')).toBe('dark')
  })

  it('toggleTheme flips light → dark and back', () => {
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    act(() => result.current.setTheme('light'))
    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('dark')
    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('light')
  })

  it('does not throw when localStorage.setItem throws (private mode)', () => {
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(() => act(() => result.current.setTheme('dark'))).not.toThrow()
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    setItemSpy.mockRestore()
  })

  it('does not throw when localStorage.getItem throws on init', () => {
    const getItemSpy = vi
      .spyOn(Storage.prototype, 'getItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const useTheme = makeUseTheme(() => true) // OS prefers dark
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
    getItemSpy.mockRestore()
  })

  it('rejects invalid stored values and falls back to OS preference', () => {
    localStorage.setItem('ac-theme', 'midnight') // not a valid Theme
    const useTheme = makeUseTheme(() => true)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })
})
```

#### 3.2 Hook implementation

**File**: `skills/visualisation/visualise/frontend/src/api/use-theme.ts`
**Changes**: new file.

The module exports two hooks with the same shape as
`use-doc-events.ts` (`useDocEvents` owning + `useDocEventsContext`
consuming). To prevent leaf components silently importing the owning
hook and creating a parallel state machine, both exports carry
explicit JSDoc:

- `useTheme()` is the **owning** hook — owns the React state, must be
  called exactly once per app at the `RootLayout` level, return value
  passed to `<ThemeContext.Provider>`.
- `useThemeContext()` is the **consumer** hook — reads the Provider
  value. This is what every leaf component should import.

```ts
import { createContext, useCallback, useContext, useEffect, useState } from 'react'
import { safeGetItem, safeSetItem } from './safe-storage'

export type Theme = 'light' | 'dark'
export const THEME_STORAGE_KEY = 'ac-theme'

export interface ThemeHandle {
  theme: Theme
  setTheme(t: Theme): void
  toggleTheme(): void
}

export function isTheme(v: unknown): v is Theme {
  return v === 'light' || v === 'dark'
}

function readInitial(prefersDark: () => boolean): Theme {
  const attr = document.documentElement.getAttribute('data-theme')
  if (isTheme(attr)) return attr
  const stored = safeGetItem(THEME_STORAGE_KEY)
  if (isTheme(stored)) return stored
  return prefersDark() ? 'dark' : 'light'
}

export function makeUseTheme(prefersDark: () => boolean) {
  return function useTheme(): ThemeHandle {
    const [theme, setThemeState] = useState<Theme>(() => readInitial(prefersDark))

    useEffect(() => {
      document.documentElement.setAttribute('data-theme', theme)
    }, [theme])

    const setTheme = useCallback((t: Theme) => {
      setThemeState(t)
      safeSetItem(THEME_STORAGE_KEY, t)
    }, [])

    // toggleTheme routes through setTheme so persistence happens at a
    // single call site, outside the state-updater function. Calling
    // safeSetItem inside a setState updater would violate React's
    // purity contract and double-fire under StrictMode dev /
    // concurrent rendering.
    const toggleTheme = useCallback(() => {
      setTheme(theme === 'light' ? 'dark' : 'light')
    }, [theme, setTheme])

    return { theme, setTheme, toggleTheme }
  }
}

/**
 * OWNING hook — call EXACTLY ONCE at the RootLayout level. Returns
 * a fresh ThemeHandle whose value must be supplied to
 * <ThemeContext.Provider value={...}>. Leaf components must NOT
 * call this — calling it creates a parallel state machine that
 * does not observe the Provider. Use `useThemeContext()` instead.
 */
export const useTheme = makeUseTheme(
  () => window.matchMedia('(prefers-color-scheme: dark)').matches,
)

const _defaultHandle: ThemeHandle = {
  theme: 'light',
  setTheme: () => {},
  toggleTheme: () => {},
}

export const ThemeContext = createContext<ThemeHandle>(_defaultHandle)

/**
 * CONSUMER hook — reads the ThemeContext provided by RootLayout.
 * This is the hook every component should use to read or change
 * the current theme.
 */
export function useThemeContext(): ThemeHandle {
  return useContext(ThemeContext)
}
```

#### 3.3 Provider wiring

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`
**Changes**: import and wrap.

```tsx
import { useDocEvents, DocEventsContext } from '../../api/use-doc-events'
import { useTheme, ThemeContext } from '../../api/use-theme'

export function RootLayout() {
  const docEvents = useDocEvents()
  const theme = useTheme()
  // ...

  return (
    <ThemeContext.Provider value={theme}>
      <DocEventsContext.Provider value={docEvents}>
        <div className={styles.root}>
          {/* ...unchanged... */}
        </div>
      </DocEventsContext.Provider>
    </ThemeContext.Provider>
  )
}
```

### Success Criteria

#### Automated Verification

- [ ] `npm run test src/api/use-theme.test.ts` is green.
- [ ] `npm run test` overall is green (no regressions in
  `RootLayout`, `Topbar`, etc.).
- [ ] `npm run build` succeeds.

#### Manual Verification

- [ ] In a React DevTools session, locate the `ThemeContext.Provider`
  on `RootLayout` and confirm `value.theme` matches
  `<html data-theme>`.

---

## Phase 4: `useFontMode` hook + `FontModeContext`

### Overview

Mirror Phase 3 for `data-font`. Same pattern, different attribute name,
different storage key, different value set.

### Changes Required

#### 4.1 Test file

**File**: `skills/visualisation/visualise/frontend/src/api/use-font-mode.test.ts`
**Changes**: new file. Structure mirrors `use-theme.test.ts`,
substituting:

- `Theme` → `FontMode`
- `'light' | 'dark'` → `'display' | 'mono'`
- `'ac-theme'` → `'ac-font-mode'`
- `'data-theme'` → `'data-font'`
- OS-preference dependency injection: not applicable (font-mode has no
  OS preference). `useFontMode` is exported directly without a factory
  wrapper — the default when no storage and no attribute is
  `'display'`.

```ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useFontMode } from './use-font-mode'

function resetDom(): void {
  document.documentElement.removeAttribute('data-font')
  localStorage.clear()
}

describe('useFontMode', () => {
  beforeEach(resetDom)
  afterEach(resetDom)

  it('initial state reflects pre-existing data-font attribute', () => {
    document.documentElement.setAttribute('data-font', 'mono')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('mono')
  })

  it('initial state reads localStorage when no attribute is present', () => {
    localStorage.setItem('ac-font-mode', 'mono')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('mono')
  })

  it('defaults to "display" when no attribute or storage', () => {
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('display')
  })

  it('attribute takes precedence over conflicting localStorage', () => {
    document.documentElement.setAttribute('data-font', 'mono')
    localStorage.setItem('ac-font-mode', 'display')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('mono')
  })

  it('setFontMode writes the attribute on <html>', () => {
    const { result } = renderHook(() => useFontMode())
    act(() => result.current.setFontMode('mono'))
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
  })

  it('setFontMode persists to localStorage under "ac-font-mode"', () => {
    const { result } = renderHook(() => useFontMode())
    act(() => result.current.setFontMode('mono'))
    expect(localStorage.getItem('ac-font-mode')).toBe('mono')
  })

  it('toggleFontMode flips display → mono and back', () => {
    const { result } = renderHook(() => useFontMode())
    act(() => result.current.setFontMode('display'))
    act(() => result.current.toggleFontMode())
    expect(result.current.fontMode).toBe('mono')
    act(() => result.current.toggleFontMode())
    expect(result.current.fontMode).toBe('display')
  })

  it('does not throw when localStorage.setItem throws', () => {
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const { result } = renderHook(() => useFontMode())
    expect(() => act(() => result.current.setFontMode('mono'))).not.toThrow()
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
    setItemSpy.mockRestore()
  })

  it('rejects invalid stored values and falls back to "display"', () => {
    localStorage.setItem('ac-font-mode', 'serif')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('display')
  })
})
```

#### 4.2 Hook implementation

**File**: `skills/visualisation/visualise/frontend/src/api/use-font-mode.ts`
**Changes**: new file. Structure mirrors `use-theme.ts` exactly,
substituting names and value type.

```ts
import { createContext, useCallback, useContext, useEffect, useState } from 'react'
import { safeGetItem, safeSetItem } from './safe-storage'

export type FontMode = 'display' | 'mono'
export const FONT_MODE_STORAGE_KEY = 'ac-font-mode'

export interface FontModeHandle {
  fontMode: FontMode
  setFontMode(m: FontMode): void
  toggleFontMode(): void
}

export function isFontMode(v: unknown): v is FontMode {
  return v === 'display' || v === 'mono'
}

function readInitial(): FontMode {
  const attr = document.documentElement.getAttribute('data-font')
  if (isFontMode(attr)) return attr
  const stored = safeGetItem(FONT_MODE_STORAGE_KEY)
  if (isFontMode(stored)) return stored
  return 'display'
}

/**
 * OWNING hook — call EXACTLY ONCE at the RootLayout level. Returns
 * a fresh FontModeHandle whose value must be supplied to
 * <FontModeContext.Provider value={...}>. Leaf components must NOT
 * call this directly — use `useFontModeContext()`.
 */
export function useFontMode(): FontModeHandle {
  const [fontMode, setState] = useState<FontMode>(() => readInitial())

  useEffect(() => {
    document.documentElement.setAttribute('data-font', fontMode)
  }, [fontMode])

  const setFontMode = useCallback((m: FontMode) => {
    setState(m)
    safeSetItem(FONT_MODE_STORAGE_KEY, m)
  }, [])

  // toggleFontMode routes through setFontMode so persistence happens
  // outside the state-updater function — see use-theme.ts for
  // rationale (React purity / StrictMode double-invoke).
  const toggleFontMode = useCallback(() => {
    setFontMode(fontMode === 'display' ? 'mono' : 'display')
  }, [fontMode, setFontMode])

  return { fontMode, setFontMode, toggleFontMode }
}

const _defaultHandle: FontModeHandle = {
  fontMode: 'display',
  setFontMode: () => {},
  toggleFontMode: () => {},
}

export const FontModeContext = createContext<FontModeHandle>(_defaultHandle)

/**
 * CONSUMER hook — reads the FontModeContext provided by RootLayout.
 * Use this from any component that needs to read or change the
 * current font mode.
 */
export function useFontModeContext(): FontModeHandle {
  return useContext(FontModeContext)
}
```

`useFontMode` is exported directly — there is no factory wrapper.
Unlike `makeUseTheme`, which injects a real `prefersDark` dependency
for OS-preference fakeability, `useFontMode` has no equivalent
dependency to inject (font-mode has no OS-level preference signal),
so a parameterless factory would be ceremonial. If a future dependency
emerges (e.g. SSR-time initial value, telemetry hook), the factory can
be reintroduced mechanically.

#### 4.3 Provider wiring

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`
**Change**: nest `FontModeContext.Provider` inside `ThemeContext.Provider`.

```tsx
import { useTheme, ThemeContext } from '../../api/use-theme'
import { useFontMode, FontModeContext } from '../../api/use-font-mode'

export function RootLayout() {
  const docEvents = useDocEvents()
  const theme = useTheme()
  const fontMode = useFontMode()
  // ...
  return (
    <ThemeContext.Provider value={theme}>
      <FontModeContext.Provider value={fontMode}>
        <DocEventsContext.Provider value={docEvents}>
          <div className={styles.root}>
            {/* ... */}
          </div>
        </DocEventsContext.Provider>
      </FontModeContext.Provider>
    </ThemeContext.Provider>
  )
}
```

### Success Criteria

#### Automated Verification

- [ ] `npm run test src/api/use-font-mode.test.ts` is green.
- [ ] `npm run test` overall is green.
- [ ] `npm run build` succeeds.

#### Manual Verification

- [ ] React DevTools shows `FontModeContext.Provider` with
  `value.fontMode` matching `<html data-font>`.

---

## Phase 5: Inline boot script + `index.html` updates

### Overview

Add a parser-blocking classic `<script>` as the first child of `<head>`
so `data-theme` and `data-font` are present on `<html>` before the
browser parses the first stylesheet, eliminating flash-of-wrong-theme.
The script is sourced from a shared TS module
(`src/api/boot-theme.ts`) and inlined into `index.html` at build time
via Vite's `transformIndexHtml` hook, so its behaviour is unit-testable
in jsdom rather than only verifiable by source-text regex grepping.

The script applies an attribute *only* when there is a corresponding
stored preference. When `localStorage` is empty for a given key, the
script writes nothing for that attribute and lets the CSS
`prefers-color-scheme` mirror at `global.css:172-197` govern paint —
preserving the existing OS-follow-the-system path that the
`tokens.spec.ts:78-87` visual-regression test exercises. The flash-
prevention guarantee still holds because that mirror is in the
parser-blocking stylesheet that paints before any React render.

`src/main.tsx` mounts via `createRoot` (CSR, not hydration); React
never reconciles `<html>`. No `suppressHydrationWarning` directive is
applied — there is no hydration warning to suppress.

The boot script's `localStorage` reads are wrapped in independent
`try/catch` blocks per attribute so a partial failure (e.g. theme
read succeeds but font-mode read throws) does not overwrite an
already-applied attribute. The `try` blocks scope only the
`localStorage.getItem` call, mirroring the `safeGetItem` helper in
`use-theme.ts`. `matchMedia` is universally available in any browser
the project targets and is not wrapped.

### Changes Required

#### 5.1 Shared boot-theme module

**File**: `skills/visualisation/visualise/frontend/src/api/boot-theme.ts`
**Changes**: new file — pure function plus a stringified copy that
the Vite plugin injects into `index.html`.

```ts
import { THEME_STORAGE_KEY, type Theme } from './use-theme'
import { FONT_MODE_STORAGE_KEY, type FontMode } from './use-font-mode'

export interface BootDeps {
  doc: Document
  storage: Storage | null
  matchPrefersDark: () => boolean
}

/**
 * Apply persisted theme/font-mode attributes to <html>. Called both
 * by the inlined boot script (before React mounts) and by tests
 * (with stubbed deps).
 *
 * If `localStorage` is unavailable or the read throws (private-mode
 * SecurityError), the corresponding attribute is left unset and the
 * CSS prefers-color-scheme mirror handles the paint.
 */
export function applyBootAttributes(deps: BootDeps): void {
  const root = deps.doc.documentElement
  // Theme: only write when storage has a valid entry. Otherwise let
  // CSS prefers-color-scheme govern the paint.
  try {
    const t = deps.storage?.getItem(THEME_STORAGE_KEY)
    if (t === 'light' || t === 'dark') {
      root.setAttribute('data-theme', t satisfies Theme)
    }
  } catch { /* SecurityError in private mode — fall through */ }
  // Font-mode: same shape, separate try block so a font-mode failure
  // never overwrites an already-applied data-theme.
  try {
    const f = deps.storage?.getItem(FONT_MODE_STORAGE_KEY)
    if (f === 'display' || f === 'mono') {
      root.setAttribute('data-font', f satisfies FontMode)
    }
  } catch { /* SecurityError — fall through */ }
}

/**
 * The string form of the boot script that Vite inlines into
 * <head>. Must be a classic (non-module) IIFE, ES5-safe, with no
 * imports. Generated from the constants exported by use-theme/
 * use-font-mode so renames stay in sync.
 */
export const BOOT_SCRIPT_SOURCE = `(function(){
var d=document.documentElement;
try{var t=localStorage.getItem(${JSON.stringify(THEME_STORAGE_KEY)});
if(t==='light'||t==='dark')d.setAttribute('data-theme',t)}catch(e){}
try{var f=localStorage.getItem(${JSON.stringify(FONT_MODE_STORAGE_KEY)});
if(f==='display'||f==='mono')d.setAttribute('data-font',f)}catch(e){}
})()`
```

The `BOOT_SCRIPT_SOURCE` constant is the string Vite injects.
Generating it from the same constants that the React hooks consume
removes the cross-file storage-key duplication and makes a rename
mechanically safe (TS will fail to compile if a key is removed).

#### 5.2 Vite plugin to inline the boot script

**File**: `skills/visualisation/visualise/frontend/vite.config.ts`
**Changes**: add an inline plugin that uses `transformIndexHtml` to
inject the boot script as the first child of `<head>`. Place it
**before** the `react()` plugin so React's plugin can't shift its
position.

The existing `vite.config.ts` imports `defineConfig` from
`vitest/config` (not `vite`) so the embedded `test: { ... }` block
type-checks against the Vitest config schema. **Keep that import.**
Add a separate `import type { Plugin } from 'vite'` for the plugin
type.

```ts
import { defineConfig } from 'vitest/config'   // ← existing; do not change
import type { Plugin } from 'vite'             // ← add (type-only)
import react from '@vitejs/plugin-react'
import { BOOT_SCRIPT_SOURCE } from './src/api/boot-theme'

function bootThemePlugin(): Plugin {
  return {
    name: 'ac-boot-theme',
    transformIndexHtml: {
      order: 'pre',
      handler: (html) => html.replace(
        /<head([^>]*)>/,
        `<head$1>\n    <script>${BOOT_SCRIPT_SOURCE}</script>`,
      ),
    },
  }
}

export default defineConfig({
  plugins: [bootThemePlugin(), react()],
  // ... existing config ...
})
```

Reading `BOOT_SCRIPT_SOURCE` from a TS module at config-evaluation
time keeps `vite.config.ts` and the runtime hooks aligned on storage
keys.

**Note on the import graph**: `boot-theme.ts` imports the storage-key
constants from `use-theme.ts` and `use-font-mode.ts`, which import
React. Loading `vite.config.ts` therefore pulls React into the
build-config evaluation graph. This works (Vite uses esbuild to
transpile config files and their imports), but if a future
contributor wants to keep `vite.config.ts` React-free, extract the
two storage-key constants and the value-set literals to a
dependency-free `src/api/storage-keys.ts` module that both the hooks
and `boot-theme.ts` import from.

#### 5.3 Boot-script unit tests

**File**: `skills/visualisation/visualise/frontend/src/api/boot-theme.test.ts`
**Changes**: new file — execute `applyBootAttributes` against
stubbed deps and assert resulting attributes.

```ts
import { describe, it, expect, beforeEach } from 'vitest'
import { applyBootAttributes } from './boot-theme'

function makeDoc(): Document {
  // Use the live jsdom document but reset attributes per test.
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.removeAttribute('data-font')
  return document
}

function fakeStorage(items: Record<string, string>): Storage {
  return {
    getItem: (k) => (k in items ? items[k] : null),
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    length: 0,
  }
}

const throwingStorage: Storage = {
  getItem: () => { throw new DOMException('private mode', 'SecurityError') },
  setItem: () => {},
  removeItem: () => {},
  clear: () => {},
  key: () => null,
  length: 0,
}

describe('applyBootAttributes', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-font')
  })

  it('writes data-theme when storage has a valid theme entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({ 'ac-theme': 'dark' }),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('writes data-font when storage has a valid font-mode entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({ 'ac-font-mode': 'mono' }),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
  })

  it('leaves data-theme unset when storage has no theme entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({}),
      matchPrefersDark: () => true,
    })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('leaves data-font unset when storage has no font-mode entry', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({}),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })

  it('rejects invalid stored theme values and leaves attribute unset', () => {
    applyBootAttributes({
      doc: makeDoc(),
      storage: fakeStorage({ 'ac-theme': 'midnight' }),
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('does not throw when storage.getItem throws (private mode)', () => {
    expect(() => applyBootAttributes({
      doc: makeDoc(),
      storage: throwingStorage,
      matchPrefersDark: () => false,
    })).not.toThrow()
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })

  it('font-mode failure does not overwrite a successfully-applied data-theme', () => {
    // Storage that succeeds for ac-theme but throws on ac-font-mode.
    const partialFailureStorage: Storage = {
      getItem: (k) => {
        if (k === 'ac-theme') return 'dark'
        throw new DOMException('partial failure', 'SecurityError')
      },
      setItem: () => {}, removeItem: () => {}, clear: () => {},
      key: () => null, length: 0,
    }
    applyBootAttributes({
      doc: makeDoc(),
      storage: partialFailureStorage,
      matchPrefersDark: () => false,
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })
})

// PARITY SUITE — guarantees BOOT_SCRIPT_SOURCE (the string that ships)
// behaves identically to applyBootAttributes (the function that's
// directly tested above). Without this, the two implementations could
// silently diverge: a fix to the function might miss the string and
// vice versa. We evaluate BOOT_SCRIPT_SOURCE via `new Function` with a
// bound `document` and `localStorage` so the same scenarios produce
// the same DOM mutations.
import { BOOT_SCRIPT_SOURCE } from './boot-theme'

function runBootScript(opts: {
  storage: Storage | null
  resetDoc?: boolean
}): void {
  if (opts.resetDoc !== false) {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-font')
  }
  // Bind document and localStorage so the IIFE references go through
  // the parameters rather than the (real) globals. The IIFE in
  // BOOT_SCRIPT_SOURCE accesses `document` and `localStorage` as
  // free identifiers; new Function makes them parameters.
  // eslint-disable-next-line @typescript-eslint/no-implied-eval
  const fn = new Function('document', 'localStorage', BOOT_SCRIPT_SOURCE) as
    (doc: Document, storage: Storage | null) => void
  fn(document, opts.storage)
}

describe('BOOT_SCRIPT_SOURCE parity with applyBootAttributes', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.removeAttribute('data-font')
  })

  it('writes data-theme=dark when storage has ac-theme=dark', () => {
    runBootScript({ storage: fakeStorage({ 'ac-theme': 'dark' }) })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('writes data-font=mono when storage has ac-font-mode=mono', () => {
    runBootScript({ storage: fakeStorage({ 'ac-font-mode': 'mono' }) })
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
  })

  it('leaves data-theme unset when storage is empty', () => {
    runBootScript({ storage: fakeStorage({}) })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('rejects invalid stored theme values', () => {
    runBootScript({ storage: fakeStorage({ 'ac-theme': 'midnight' }) })
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
  })

  it('does not throw when storage.getItem throws (private mode)', () => {
    expect(() => runBootScript({ storage: throwingStorage })).not.toThrow()
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })

  it('font-mode failure does not overwrite a successfully-applied data-theme', () => {
    const partialFailureStorage: Storage = {
      getItem: (k) => {
        if (k === 'ac-theme') return 'dark'
        throw new DOMException('partial failure', 'SecurityError')
      },
      setItem: () => {}, removeItem: () => {}, clear: () => {},
      key: () => null, length: 0,
    }
    runBootScript({ storage: partialFailureStorage })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(document.documentElement.hasAttribute('data-font')).toBe(false)
  })
})
```

The parity suite runs each scenario through *the actual production
string* via `new Function`. If the IIFE diverges from
`applyBootAttributes` (different value-set, different attribute name,
broken try/catch scoping), the parity tests fail even when the
function-level tests pass — closing the dual-implementation drift gap.

#### 5.4 Build-output structural test

**File**: `skills/visualisation/visualise/frontend/src/api/boot-theme.html.test.ts`
**Changes**: new file — verifies that the *built* `index.html` has
the boot script as the first child of `<head>` and before any
stylesheet `<link>`. Skipped when `dist/index.html` is absent so
it doesn't fail in plain `npm run test` runs that haven't built.

```ts
import { describe, it, expect } from 'vitest'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { BOOT_SCRIPT_SOURCE } from './boot-theme'

const distHtmlPath = resolve(__dirname, '../../dist/index.html')

// Strip leading HTML comments and whitespace so the first-tag check
// isn't fooled by `<!-- ... -->` injected by future Vite plugins
// chained before bootThemePlugin.
function stripLeadingCommentsAndWhitespace(s: string): string {
  let out = s
  while (true) {
    const t = out.replace(/^\s+/, '')
    if (t.startsWith('<!--')) {
      const end = t.indexOf('-->')
      if (end === -1) return t
      out = t.slice(end + 3)
      continue
    }
    return t
  }
}

describe.skipIf(!existsSync(distHtmlPath))(
  'dist/index.html boot script structure',
  () => {
    const html = existsSync(distHtmlPath) ? readFileSync(distHtmlPath, 'utf8') : ''

    it('the first <head> child is a classic <script> tag', () => {
      const headBody = /<head[^>]*>([\s\S]*?)<\/head>/.exec(html)?.[1] ?? ''
      const cleaned = stripLeadingCommentsAndWhitespace(headBody)
      const firstTag = /<\s*([a-zA-Z][\w-]*)/.exec(cleaned)?.[1]
      expect(firstTag).toBe('script')
      const firstScript = /<script[^>]*>/.exec(cleaned)?.[0] ?? ''
      expect(firstScript).not.toMatch(/type\s*=\s*["']module["']/)
      expect(firstScript).not.toMatch(/\bdefer\b/)
      expect(firstScript).not.toMatch(/\basync\b/)
    })

    it('the boot script precedes any <link rel="stylesheet">', () => {
      const head = /<head[^>]*>([\s\S]*?)<\/head>/.exec(html)?.[1] ?? ''
      const scriptIdx = head.indexOf('<script')
      const linkIdx = head.search(/<link[^>]*rel\s*=\s*["']stylesheet["']/)
      expect(scriptIdx).toBeGreaterThanOrEqual(0)
      if (linkIdx !== -1) {
        expect(scriptIdx).toBeLessThan(linkIdx)
      }
    })

    it('the inlined script body equals BOOT_SCRIPT_SOURCE', () => {
      // Locks in the contract that the Vite plugin actually injects
      // the constant we tested in boot-theme.test.ts. A plugin
      // regression that ships an empty <script></script>, an
      // out-of-date copy, or a mangled minified form will fail here.
      expect(html).toContain(BOOT_SCRIPT_SOURCE)
    })
  },
)
```

This test is the only place that asserts the build-pipeline ordering
invariant — the unit tests in §5.3 verify behaviour, this verifies
plumbing. Run via `npm run build && npm run test src/api/boot-theme.html.test.ts`
in CI. In an interactive `npm run test` run with no `dist/`, the
suite is skipped and reports zero failures.

#### 5.5 Update `index.html`

**File**: `skills/visualisation/visualise/frontend/index.html`
**Change**: remove any pre-existing boot script — the Vite plugin
injects it. The static file is otherwise unchanged from the current
version.

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Accelerator Visualiser</title>
    <link rel="preload" as="font" type="font/woff2"
          href="/fonts/Inter-Regular.woff2" crossorigin="anonymous" />
    <link rel="preload" as="font" type="font/woff2"
          href="/fonts/Sora-Bold.woff2" crossorigin="anonymous" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

The `<html>` tag carries no `suppressHydrationWarning` — the app
uses `createRoot`, so React never reconciles `<html>` and no
hydration warning is produced for the boot-script attribute writes.

### Success Criteria

#### Automated Verification

- [ ] `npm run test src/api/boot-theme.test.ts` is green.
- [ ] `npm run build && npm run test src/api/boot-theme.html.test.ts`
  is green.
- [ ] `npm run test` overall is green.
- [ ] `npm run build` succeeds.

#### Manual Verification

- [ ] In Chrome DevTools Performance, record a reload with
  `localStorage['ac-theme']='dark'` set — no flash of light theme; the
  first paint is dark. Confirm via the Performance flame chart that
  the `<html>` element has `data-theme="dark"` from the very first
  paint.
- [ ] In a fresh browser profile (no `localStorage` entry), reload —
  `<html>` has *no* `data-theme` attribute, and the page paints
  according to the OS `prefers-color-scheme` setting (the
  `global.css:172-197` mirror handles it).
- [ ] Switch the OS dark-mode preference while the app is open with
  no stored preference — the next reload follows the new OS setting.
- [ ] Disable JavaScript (DevTools → Settings) and reload — the OS
  preference fallback in CSS still works.
- [ ] Reload with `localStorage['ac-font-mode']='mono'` set — Brand
  wordmark and Breadcrumbs render in Fira Code on first paint.
- [ ] Open in a Firefox private-browsing window — no `SecurityError`
  in console; `<html>` has no `data-theme` attribute; paint follows
  OS preference; the in-session toggle still works (writes to a
  per-session storage that doesn't persist).
- [ ] Open the app and inspect the DOM — no React hydration warnings
  in the console for any combination of stored / OS preferences (the
  app uses `createRoot`, so there is no hydration pass).

---

## Phase 6: `ThemeToggle` component

### Overview

A button rendered into the Topbar `data-slot="theme-toggle"` div that
calls `useThemeContext().toggleTheme()` on click. Visual: icon-only
button with sun (`light`) or moon (`dark`) glyph, `--radius-pill`,
ADR-0026-conformant hover/active tints, accessible name.

The button shell (sizing, hover/active tints, focus-visible inheritance,
forced-colors border) is extracted as a `TopbarIconButton` shared
component first (§6.0). `ThemeToggle` and `FontModeToggle` (Phase 7)
both compose on top of it, so the shell is defined once and any
future topbar icon control (sidebar collapse, kanban filter, etc.)
inherits the same affordance.

### Changes Required

#### 6.0 Shared `TopbarIconButton` component

**File**: `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.tsx`
**Changes**: new file.

```tsx
import type { ReactNode } from 'react'
import styles from './TopbarIconButton.module.css'

export interface TopbarIconButtonProps {
  /** Function-describing accessible name (e.g. "Dark theme"). */
  ariaLabel: string
  /** Pressed state for binary toggle buttons. */
  ariaPressed: boolean
  /** State key written to `data-icon` for CSS targeting and tests. */
  dataIcon: string
  /** Glyph / SVG content rendered inside the button. Should be
   *  decorative — wrap in `aria-hidden="true"` at the call site. */
  children: ReactNode
  onClick: () => void
}

export function TopbarIconButton(props: TopbarIconButtonProps) {
  return (
    <button
      type="button"
      className={styles.toggle}
      data-icon={props.dataIcon}
      aria-label={props.ariaLabel}
      aria-pressed={props.ariaPressed}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.module.css`
**Changes**: new file — owns the shell that was previously duplicated.

```css
.toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--ac-topbar-h);
  height: var(--ac-topbar-h);
  background: transparent;
  color: var(--ac-fg);
  border: none;
  border-radius: var(--radius-pill);
  cursor: pointer;
  font-family: var(--ac-font-body);
  font-size: var(--size-md);
  line-height: 1;
  padding: 0;
}

.toggle:hover {
  background: var(--ac-bg-hover);
}

.toggle:active {
  background: var(--ac-bg-active);
}

/* Forced-colors mode (Windows High Contrast): explicit border so
   the button is distinguishable from the topbar background. */
@media (forced-colors: active) {
  .toggle {
    border: 1px solid ButtonText;
  }
}
```

**File**: `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.test.tsx`
**Changes**: new file.

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { TopbarIconButton } from './TopbarIconButton'

describe('TopbarIconButton', () => {
  it('exposes the accessible name from ariaLabel', () => {
    render(
      <TopbarIconButton ariaLabel="Test mode" ariaPressed={false} dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button', { name: 'Test mode' })).toBeInTheDocument()
  })

  it('reflects ariaPressed state', () => {
    const { rerender } = render(
      <TopbarIconButton ariaLabel="X" ariaPressed={false} dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false')
    rerender(
      <TopbarIconButton ariaLabel="X" ariaPressed={true} dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true')
  })

  it('writes data-icon for CSS/test targeting', () => {
    render(
      <TopbarIconButton ariaLabel="X" ariaPressed={false} dataIcon="custom-icon" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'custom-icon')
  })

  it('invokes onClick when clicked', () => {
    const onClick = vi.fn()
    render(
      <TopbarIconButton ariaLabel="X" ariaPressed={false} dataIcon="x" onClick={onClick}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    fireEvent.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledTimes(1)
  })

  it('CSS module includes a @media (forced-colors: active) block', async () => {
    const css = await import('./TopbarIconButton.module.css?raw')
    expect(css.default).toMatch(
      /@media\s*\(\s*forced-colors:\s*active\s*\)[^}]*\{[^}]*border\s*:\s*1px solid\s+ButtonText/s,
    )
  })
})
```

#### 6.1 Test file

**File**: `skills/visualisation/visualise/frontend/src/components/ThemeToggle/ThemeToggle.test.tsx`
**Changes**: new file.

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ThemeToggle } from './ThemeToggle'

vi.mock('../../api/use-theme', () => ({
  useThemeContext: vi.fn(),
}))

import { useThemeContext } from '../../api/use-theme'

function mountWith(theme: 'light' | 'dark', toggle: () => void = vi.fn()) {
  vi.mocked(useThemeContext).mockReturnValue({
    theme,
    setTheme: vi.fn(),
    toggleTheme: toggle,
  })
  return render(<ThemeToggle />)
}

describe('ThemeToggle', () => {
  it('renders a button with a function-describing accessible name', () => {
    mountWith('light')
    // Function-describing label per WAI-ARIA APG: the press state
    // (aria-pressed) carries "currently active" — the label
    // describes WHAT the button is, not what clicking does. Mirrors
    // LifecycleIndex.tsx:136 codebase precedent.
    expect(screen.getByRole('button', { name: /dark theme/i })).toBeInTheDocument()
  })

  it('renders the current-state glyph in light mode', () => {
    mountWith('light')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'sun')
    // Visible glyph is also asserted so a refactor that keeps
    // data-icon in sync with state but renders a wrong character
    // does not slip through.
    expect(screen.getByRole('button').textContent).toContain('☀︎')
  })

  it('renders the current-state glyph in dark mode', () => {
    mountWith('dark')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'moon')
    expect(screen.getByRole('button').textContent).toContain('☽︎')
  })

  it('exposes aria-pressed reflecting whether dark is active', () => {
    mountWith('light')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false')
    mountWith('dark')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true')
  })

  it('calls toggleTheme on click', () => {
    const toggle = vi.fn()
    mountWith('light', toggle)
    fireEvent.click(screen.getByRole('button'))
    expect(toggle).toHaveBeenCalledTimes(1)
  })
})
```

The forced-colors block presence is asserted in
`TopbarIconButton.test.tsx` (§6.0) — `ThemeToggle` doesn't repeat the
assertion because the button shell lives in the shared component.

Convention: the icon shown reflects the *current* theme — a sun in
light mode, a moon in dark mode — matching macOS, iOS, VS Code,
Linear, and Slack. The accessible name describes the action (the
*next* state), so users always know what a click will do.

#### 6.2 Component

**File**: `skills/visualisation/visualise/frontend/src/components/ThemeToggle/ThemeToggle.tsx`
**Changes**: new file.

```tsx
import { useThemeContext } from '../../api/use-theme'
import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'
import styles from './ThemeToggle.module.css'

export function ThemeToggle() {
  const { theme, toggleTheme } = useThemeContext()
  // Current-state icon: sun for light, moon for dark.
  const icon = theme === 'light' ? 'sun' : 'moon'
  // Function-describing label paired with aria-pressed. Screen readers
  // announce as "Dark theme, toggle button, pressed" or "... not
  // pressed", which is unambiguous; an action-describing label
  // ("Switch to dark theme") combined with aria-pressed=true while
  // dark is already active reads as self-contradictory.
  return (
    <TopbarIconButton
      ariaLabel="Dark theme"
      ariaPressed={theme === 'dark'}
      dataIcon={icon}
      onClick={toggleTheme}
    >
      <span aria-hidden="true" className={styles.glyph}>
        {icon === 'sun' ? '☀︎' : '☽︎'}
      </span>
    </TopbarIconButton>
  )
}
```

Glyphs are Unicode `☀` (U+2600 BLACK SUN WITH RAYS) and `☽` (U+263D
FIRST QUARTER MOON), both followed by `︎` (the text-presentation
variation selector) so emoji-capable platforms render the monochrome
text glyph rather than a colour emoji. U+263D pairs better with
U+2600 at small icon sizes than U+263E (LAST QUARTER MOON) does.
Using Unicode glyphs rather than SVG keeps the component
zero-dependency and avoids introducing icon-asset infrastructure.

The button has no `title` attribute. `title` is unreachable to
keyboard-only users, hidden on touch, and inconsistently surfaced by
screen readers — duplicating an `aria-label` via `title` is a
recognised WCAG anti-pattern. The accessible name on `aria-label` is
the canonical source for assistive tech and for any future tooltip
component that wires through ARIA.

`aria-label="Dark theme"` (function-describing) combined with
`aria-pressed={theme === 'dark'}` follows WAI-ARIA APG guidance for
toggle buttons and matches the codebase precedent at
`LifecycleIndex.tsx:136`. Screen readers announce "Dark theme, toggle
button, not pressed" when light is active and "Dark theme, toggle
button, pressed" when dark is active — unambiguous in both states.

#### 6.3 Styles

**File**: `skills/visualisation/visualise/frontend/src/components/ThemeToggle/ThemeToggle.module.css`
**Changes**: new file — `ThemeToggle` only owns the glyph styling;
the button shell lives in `TopbarIconButton.module.css` (§6.0).

```css
.glyph {
  display: inline-block;
}
```

The shell rules (sizing, hover/active tints, forced-colors border)
are defined once in `TopbarIconButton.module.css` and apply to all
topbar icon buttons. Reduced-motion handling: the shell has no
transitions, so no `prefers-reduced-motion` block is needed. Future
transitions on the shell should use `@media (prefers-reduced-motion: reduce) { transition: none }`
to match the `OriginPill.module.css:19` / `SseIndicator.module.css:19`
codebase precedent (rather than the inverse `no-preference` shape).

#### 6.4 AC5 floor bump (deferred)

Per the consolidation in Phase 1.5, `AC5_FLOOR` is not bumped here.
The new `var(--*)` references in `TopbarIconButton.module.css`
(~8 distinct references for the shared shell) accumulate into the
single bump after Phase 7. `ThemeToggle.module.css` itself contains
no token references in this revision.

### Success Criteria

#### Automated Verification

- [x] `npm run test src/components/ThemeToggle/ThemeToggle.test.tsx` is
  green.
- [x] `npm run test src/styles/migration.test.ts` is green (AC5 floor
  bumped, no new hex/px literals).
- [x] `npm run test` overall is green.
- [x] `npm run build` succeeds.

#### Manual Verification

- [ ] Render the component in isolation (Storybook is not configured;
  use `npm run dev` and a temporary route or just the existing
  Topbar after Phase 8 lands) — clicking flips theme; icon glyph
  flips to next-state preview; hover ring is visible.
- [ ] Tab to the toggle with the keyboard — focus ring (the global
  `:focus-visible` 2px solid `--ac-accent`) is visible.
- [ ] Verify under `forced-colors: active` (Windows High Contrast
  emulator in DevTools → Rendering) — focus ring uses `Highlight`,
  glyph remains visible.

---

## Phase 7: `FontModeToggle` component

### Overview

A button rendered into the Topbar `data-slot="font-mode-toggle"` div.
Same shape and styling as `ThemeToggle`, with `Aa` glyph swapping to a
mono-styled glyph. Click cycles `display` ↔ `mono`.

### Changes Required

#### 7.1 Test file

**File**: `skills/visualisation/visualise/frontend/src/components/FontModeToggle/FontModeToggle.test.tsx`
**Changes**: new file. Mirrors `ThemeToggle.test.tsx`, substituting:

- `useThemeContext` → `useFontModeContext`
- `theme` / `toggleTheme` → `fontMode` / `toggleFontMode`
- Icon: the literal glyph `Aa` rendered in the *target* font — i.e.
  in display mode the icon previews mono (`Aa` in Fira Code), in
  mono mode the icon previews display (`Aa` in Inter). The button's
  `data-icon` attribute carries `"display"` or `"mono"` to match.
- Accessible name: `Switch to {next} font` where next is `mono` or
  `display`.

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { FontModeToggle } from './FontModeToggle'

vi.mock('../../api/use-font-mode', () => ({
  useFontModeContext: vi.fn(),
}))

import { useFontModeContext } from '../../api/use-font-mode'

function mountWith(fontMode: 'display' | 'mono', toggle = vi.fn()) {
  vi.mocked(useFontModeContext).mockReturnValue({
    fontMode,
    setFontMode: vi.fn(),
    toggleFontMode: toggle,
  })
  return render(<FontModeToggle />)
}

describe('FontModeToggle', () => {
  it('renders a button with a function-describing accessible name', () => {
    mountWith('display')
    // Function-describing label per WAI-ARIA APG (see ThemeToggle test).
    expect(screen.getByRole('button', { name: /mono font/i })).toBeInTheDocument()
  })

  it('previews the target font in display mode (mono glyph)', () => {
    mountWith('display')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'mono')
    expect(screen.getByRole('button').textContent).toContain('Aa')
  })

  it('previews the target font in mono mode (display glyph)', () => {
    mountWith('mono')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'display')
    expect(screen.getByRole('button').textContent).toContain('Aa')
  })

  it('exposes aria-pressed reflecting whether mono is active', () => {
    mountWith('display')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false')
    mountWith('mono')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true')
  })

  it('calls toggleFontMode on click', () => {
    const toggle = vi.fn()
    mountWith('display', toggle)
    fireEvent.click(screen.getByRole('button'))
    expect(toggle).toHaveBeenCalledTimes(1)
  })
})
```

The forced-colors block presence is asserted in
`TopbarIconButton.test.tsx` (§6.0); not repeated here.

#### 7.2 Component

**File**: `skills/visualisation/visualise/frontend/src/components/FontModeToggle/FontModeToggle.tsx`
**Changes**: new file.

```tsx
import { useFontModeContext } from '../../api/use-font-mode'
import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'
import styles from './FontModeToggle.module.css'

export function FontModeToggle() {
  const { fontMode, toggleFontMode } = useFontModeContext()
  // Preview the *target* font: when current is display, show Aa in
  // mono so the user sees what they're switching to. When current is
  // mono, show Aa in display.
  const icon = fontMode === 'display' ? 'mono' : 'display'
  return (
    <TopbarIconButton
      ariaLabel="Mono font"
      ariaPressed={fontMode === 'mono'}
      dataIcon={icon}
      onClick={toggleFontMode}
    >
      <span aria-hidden="true" className={icon === 'mono' ? styles.mono : styles.display}>
        Aa
      </span>
    </TopbarIconButton>
  )
}
```

The glyph `Aa` is rendered in the *target* font — Fira Code when the
icon previews `mono`, Inter when it previews `display`. Showing the
same characters in different typefaces directly previews the change
in a way that needs no accompanying label. This replaces an earlier
`Aa` ↔ `M` design that was asymmetric (two characters vs one) and
relied on `M` reading as "monospace", which is unlikely to be
self-evident to users who aren't typography-fluent.

The button has no `title` attribute (same rationale as `ThemeToggle`).
`aria-label="Mono font"` is function-describing and pairs with
`aria-pressed={fontMode === 'mono'}` per WAI-ARIA APG guidance — same
pattern as the theme toggle.

#### 7.3 Styles

**File**: `skills/visualisation/visualise/frontend/src/components/FontModeToggle/FontModeToggle.module.css`
**Changes**: new file — `FontModeToggle` only owns the glyph-specific
font overrides; the button shell lives in
`TopbarIconButton.module.css` (§6.0).

```css
/* The display glyph uses the body font (Inter); the mono glyph uses
   the mono font (Fira Code), regardless of [data-font="mono"] state.
   This makes the icon a preview of the *target* mode rather than a
   reflection of the current cascade. */
.display { font-family: var(--ac-font-body); font-weight: 600; }
.mono    { font-family: var(--ac-font-mono); font-weight: 600; }
```

Reduced-motion handling: the toggle has no transitions, so no
`prefers-reduced-motion` block is needed.

#### 7.4 Consolidated AC5 floor bump

After Phase 7's CSS lands, run `npm run test src/styles/migration.test.ts`
once. The test reports the current `var(--*)` count via its failure
message. Bump `AC5_FLOOR` to that exact value with a single comment
naming the work item — replacing the per-phase comment trail
originally proposed:

```ts
const AC5_FLOOR = NNN // 0034: route titles + ThemeToggle + FontModeToggle (+~17)
```

Net additions across 0034 in the migration glob:
- Phase 1: +5 (route titles)
- Phase 6.0: +~8 (TopbarIconButton shell, defined once)
- Phase 6: +0 (ThemeToggle CSS now contains only `.glyph` — no token refs)
- Phase 7: +~2 (FontModeToggle CSS only has `.display`/`.mono` font-family overrides)
- Total: +~15

Use the actual observed count from the failing-test message — the
estimate above is for sizing only. A single ratchet adjustment keeps
the commit history tracking real refactor checkpoints rather than
mid-implementation waypoints.

### Success Criteria

#### Automated Verification

- [x] `npm run test src/components/FontModeToggle/FontModeToggle.test.tsx`
  is green.
- [x] `npm run test src/styles/migration.test.ts` is green.
- [x] `npm run test` overall is green.
- [x] `npm run build` succeeds.

#### Manual Verification

- [ ] Render the component in the Topbar (after Phase 8) — clicking
  flips font mode; the icon glyph remains `Aa` in both modes,
  rendered in Fira Code when previewing mono and in Inter when
  previewing display.
- [ ] Verify focus ring and forced-colors behaviour as for ThemeToggle.

---

## Phase 8: Topbar wiring

### Overview

Replace the two empty slot divs in `Topbar.tsx` with `<ThemeToggle />`
and `<FontModeToggle />`. Update the existing Topbar tests that asserted
empty slots.

### Changes Required

#### 8.1 Update Topbar tests

**File**: `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.test.tsx`
**Changes**: replace lines 57-69. Add mocks for the two new context
hooks; replace the emptiness assertions with content assertions.

```tsx
// Add to existing vi.mock block at top
vi.mock('../../api/use-theme', () => ({
  useThemeContext: vi.fn(),
}))
vi.mock('../../api/use-font-mode', () => ({
  useFontModeContext: vi.fn(),
}))

import { useThemeContext } from '../../api/use-theme'
import { useFontModeContext } from '../../api/use-font-mode'

// Extend mountTopbar to set up the new context mocks
function mountTopbar(connectionState = 'open', justReconnected = false) {
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState,
    justReconnected,
    setDragInProgress: vi.fn(),
  } as any)
  vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
  vi.mocked(useThemeContext).mockReturnValue({
    theme: 'light',
    setTheme: vi.fn(),
    toggleTheme: vi.fn(),
  })
  vi.mocked(useFontModeContext).mockReturnValue({
    fontMode: 'display',
    setFontMode: vi.fn(),
    toggleFontMode: vi.fn(),
  })
  return render(<Topbar />)
}

// Replace the two `data-slot` empty-children assertions with:
it('renders a theme toggle inside the data-slot="theme-toggle" div', () => {
  mountTopbar()
  const slot = document.querySelector('[data-slot="theme-toggle"]')
  expect(slot).not.toBeNull()
  // The slot wrapper now contains a <button>
  expect(slot?.querySelector('button')).not.toBeNull()
})

it('renders a font-mode toggle inside the data-slot="font-mode-toggle" div', () => {
  mountTopbar()
  const slot = document.querySelector('[data-slot="font-mode-toggle"]')
  expect(slot).not.toBeNull()
  expect(slot?.querySelector('button')).not.toBeNull()
})

it('the theme toggle button has an accessible name', () => {
  mountTopbar()
  expect(screen.getByRole('button', { name: /dark theme/i })).toBeInTheDocument()
})

it('the font-mode toggle button has an accessible name', () => {
  mountTopbar()
  expect(screen.getByRole('button', { name: /mono font/i })).toBeInTheDocument()
})

// Click-through integration: substantiates the "exercises the wiring
// end-to-end at the unit layer" claim in Testing Strategy. Without
// these the test only asserts a button exists in the slot — a
// regression that drops onClick wiring would pass.
it('clicking the theme toggle invokes toggleTheme from the context', () => {
  const toggleTheme = vi.fn()
  vi.mocked(useThemeContext).mockReturnValue({
    theme: 'light',
    setTheme: vi.fn(),
    toggleTheme,
  })
  vi.mocked(useFontModeContext).mockReturnValue({
    fontMode: 'display',
    setFontMode: vi.fn(),
    toggleFontMode: vi.fn(),
  })
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState: 'open',
    justReconnected: false,
    setDragInProgress: vi.fn(),
  } as any)
  vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
  render(<Topbar />)
  fireEvent.click(screen.getByRole('button', { name: /dark theme/i }))
  expect(toggleTheme).toHaveBeenCalledTimes(1)
})

it('clicking the font-mode toggle invokes toggleFontMode from the context', () => {
  const toggleFontMode = vi.fn()
  vi.mocked(useThemeContext).mockReturnValue({
    theme: 'light',
    setTheme: vi.fn(),
    toggleTheme: vi.fn(),
  })
  vi.mocked(useFontModeContext).mockReturnValue({
    fontMode: 'display',
    setFontMode: vi.fn(),
    toggleFontMode,
  })
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState: 'open',
    justReconnected: false,
    setDragInProgress: vi.fn(),
  } as any)
  vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
  render(<Topbar />)
  fireEvent.click(screen.getByRole('button', { name: /mono font/i }))
  expect(toggleFontMode).toHaveBeenCalledTimes(1)
})
```

The CSS-source assertion at lines 76-80 (`.slot:empty` rule) is fine
to keep — the rule still exists, it just doesn't apply once children
are present.

#### 8.2 Update Topbar component

**File**: `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.tsx`
**Changes**: import the toggles and place them inside the slot divs.

```tsx
import { Brand } from '../Brand/Brand'
import { Breadcrumbs } from '../Breadcrumbs/Breadcrumbs'
import { OriginPill } from '../OriginPill/OriginPill'
import { SseIndicator } from '../SseIndicator/SseIndicator'
import { ThemeToggle } from '../ThemeToggle/ThemeToggle'
import { FontModeToggle } from '../FontModeToggle/FontModeToggle'
import styles from './Topbar.module.css'

export function Topbar() {
  return (
    <header className={styles.topbar}>
      <Brand />
      <div className={styles.divider} />
      <Breadcrumbs />
      <div className={styles.spacer} />
      <OriginPill />
      <SseIndicator />
      <div className={styles.slot} data-slot="theme-toggle">
        <ThemeToggle />
      </div>
      <div className={styles.slot} data-slot="font-mode-toggle">
        <FontModeToggle />
      </div>
    </header>
  )
}
```

The `.slot:empty` rule at `Topbar.module.css:20-24` self-disengages
because the divs now have child elements; the slots will size to their
content (the 48px-square toggle buttons) and participate in the topbar
flex row.

### Success Criteria

#### Automated Verification

- [x] `npm run test src/components/Topbar/Topbar.test.tsx` is green
  (new content assertions pass; old emptiness assertions removed).
- [x] `npm run test` overall is green.
- [x] `npm run build` succeeds.

#### Manual Verification

- [ ] Run `npm run dev` and load the app — both toggle buttons appear
  on the right side of the Topbar, after `OriginPill` and
  `SseIndicator`. Clicking the theme toggle inverts the canvas;
  clicking the font toggle flips the typography to Fira Code.
- [ ] Reload the page after each click — the choice persists.
- [ ] Verify in a fresh browser profile with no `localStorage` — first
  paint matches OS preference; subsequent click writes
  `localStorage`; reload preserves choice.

---

## Phase 9: Re-capture dark visual-regression baselines (manual)

### Overview

`tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/
*-dark-*.png` were captured before Phase 1 fixed the inert-token bug.
Once the canvas inverts and headings flip to `var(--ac-fg-strong)`, the
existing baselines will fail. They must be re-captured.

### Changes Required

#### 9.1 Run the visual regression suite to confirm failure

From `skills/visualisation/visualise/frontend/`:

```sh
npm run test:e2e -- --project=chromium tests/visual-regression/tokens.spec.ts
```

Expected: every `*-dark-*.png` test fails with a pixel diff above the
0.05 threshold; light variants pass unchanged.

#### 9.2 Update baselines

```sh
npm run test:e2e -- --update-snapshots tests/visual-regression/tokens.spec.ts
```

This rewrites every snapshot for which the suite ran. The implementer
should review the diff in `git`/`jj` to confirm the changes are
expected:

- Every `*-dark-*.png` differs (canvas now navy; headings now white).
- No `*-light-*.png` differs (light mode is the same — Phase 1's body
  bg/fg additions in light mode collapse to the same painted colour
  as the user-agent default).

#### 9.3 Verify

```sh
npm run test:e2e -- tests/visual-regression/tokens.spec.ts
```

All tests green.

### Success Criteria

#### Automated Verification

- [x] `npm run test:e2e` is green against the re-captured baselines.

#### Manual Verification

- [ ] Visual diff inspection of the regenerated PNGs shows: dark canvas,
  legible headings, no other visual regressions (nav, sidebar, cards
  all token-aware).
- [ ] Spot-check the `library (prefers-color-scheme: dark, no
  data-theme attribute)` test — confirms the OS-preference path
  paints identically to the explicit `data-theme="dark"` path.

---

## Phase 10: Reconcile work item AC wording

### Overview

The work item at `meta/work/0034-theme-and-font-mode-toggles.md` says
`data-font="default"` (line 35, line 69). This plan and the prototype
inventory use `data-font="display"`. Update the work item so its AC
matches what shipped.

### Changes Required

**File**: `meta/work/0034-theme-and-font-mode-toggles.md`

Replace `default` with `display` in the Requirements bullet (line 44)
and the Acceptance Criterion (line 68-69):

```diff
-- Implement a font-mode context hook that sets `data-font` (`default`/`mono`)
+- Implement a font-mode context hook that sets `data-font` (`display`/`mono`)
   on the document root.

-- [ ] Given the user clicks the font-mode toggle, then `data-font` switches
-  between `default` and `mono` and `--ac-font-display` / `--ac-font-body` both
+- [ ] Given the user clicks the font-mode toggle, then `data-font` switches
+  between `display` and `mono` and `--ac-font-display` / `--ac-font-body` both
   compute to `"Fira Code"` on `document.documentElement`.
```

Also note the rationale in a brief addendum at the bottom of the work
item Drafting Notes:

```
- Attribute value `data-font="display"` adopted (not `default`) to
  align with the prototype design inventory's canonical naming
  (`meta/design-inventories/2026-05-06-140608-claude-design-prototype/
  inventory.md:227`). The plan
  (`meta/plans/2026-05-08-0034-theme-and-font-mode-toggles.md`)
  records this decision.
```

### Success Criteria

#### Automated Verification

- [x] The work item file no longer contains the literal string
  `default/mono` or `default` in the `data-font` context.

#### Manual Verification

- [ ] Re-read the AC. They now describe what the implementation
  actually does.

---

## Testing Strategy

### Unit tests

- `use-theme.test.ts`: 9 cases covering attribute-init, storage-init,
  OS-fallback init, attribute write, storage persistence, toggle, and
  three error/safety scenarios (private mode setItem, private mode
  getItem, invalid stored value).
- `use-font-mode.test.ts`: 8 cases mirroring the theme tests minus the
  OS-preference dependency.
- `TopbarIconButton.test.tsx` (new): 5 cases — accessible name,
  aria-pressed reflection, data-icon write, onClick invocation, and
  presence of the `@media (forced-colors: active)` block in the
  shared shell CSS.
- `ThemeToggle.test.tsx` / `FontModeToggle.test.tsx`: ~5 cases each —
  accessible name, current-state glyph (sun/moon for theme;
  preview-target Aa for font-mode), aria-pressed wiring, click
  invocation. Forced-colors and shell-shape assertions live on the
  shared `TopbarIconButton` test.
- `Topbar.test.tsx`: 4 new cases (slot has button, accessible name for
  each toggle).
- `global.test.ts`: 3 new cases (body bg, body fg, color-scheme), plus
  a parity describe iterating `MONO_FONT_TOKENS`.
- `migration.test.ts`: 3 new cases (each `.title` site has
  `var(--ac-fg-strong)`), plus AC5 floor bumps in Phases 1, 6, 7.
- `boot-theme.test.ts` (new): 7 behavioural cases — runs
  `applyBootAttributes` against stubbed `Document` and `Storage`
  to verify written attributes for valid/invalid/missing storage
  entries, and the partial-failure isolation between theme and
  font-mode reads.
- `boot-theme.html.test.ts` (new, build-only): 2 structural cases on
  `dist/index.html` — first `<head>` child is a classic non-module
  `<script>`, and it precedes any stylesheet `<link>`.
- `safe-storage.test.ts` (new): cases covering normal getItem/setItem,
  SecurityError swallowing, and (per the compatibility lens) explicit
  verification that the `vi.spyOn(Storage.prototype, 'setItem')`
  pattern is intercepted under jsdom.

### Integration tests

The Topbar test now mounts the real `ThemeToggle` and `FontModeToggle`
(no `vi.mock` on those modules) — exercises the wiring end-to-end at
the unit layer.

### Visual regression

Phase 9 re-captures dark baselines. The `tokens.spec.ts` suite already
covers all major routes in both themes plus an OS-preference path; no
new tests are needed for the toggle UI itself (the toggle's pixel
output is a small icon button — visual regression is more useful at the
canvas/heading level which is already covered).

### Manual testing steps

For the final implementation pass:

1. Fresh browser profile (no `localStorage`); OS in light mode → app
   loads in light. Toggle theme → dark. Reload → still dark.
2. Same profile in dark OS mode → app loads in dark on first visit.
3. Private-browsing window → no console errors; OS-preference
   fallback works; toggling theme works in-session but doesn't
   persist.
4. With `data-theme="dark"` and `data-font="mono"` set in
   `localStorage`, reload while watching DevTools Performance — first
   paint is dark + mono; no flash.
5. Tab through Topbar → focus rings visible on both toggles; click
   each via Enter and Space — both work.
6. Toggle theme rapidly (10× clicks) — no console errors, no laggy
   render, attribute and `localStorage` stay in sync.
7. Open in an alternate browser profile while the original profile has
   custom preferences → independent storage; original profile retains
   choices.

## Performance Considerations

The boot script runs synchronously before first paint. It performs at
most two `localStorage` reads and two `setAttribute` calls — under
1ms on any browser. The classic-script parser-blocking cost is
unavoidable and is the price of FOUC prevention. The script does not
call `matchMedia` — when no stored preference exists, the
`prefers-color-scheme` mirror in `global.css:172-197` handles paint
without any JS work.

`useTheme` and `useFontMode` write the attribute via `useEffect` (after
React commit) and write `localStorage` in the action callback (during
the React render cycle). Toggle frequency is rare (single-digit clicks
per session), so write debouncing is unnecessary.

`useEffect`-based attribute writes happen one render after the boot
script's writes. There is no React hydration warning to manage:
`src/main.tsx` mounts via `createRoot` (CSR), not `hydrateRoot`, so
there is no hydration pass. React mounts into `<div id="root">` and
never reconciles `<html>`, so the boot-script attribute writes on
`<html>` are invisible to React's reconciler.

## Migration Notes

No data migration required. Existing users with no `localStorage`
entries get OS-preference behaviour on first load.

If the implementer ships a wrong attribute name in production and later
fixes it, users with stale `localStorage` entries (e.g. `default`
instead of `display`) will fall through the type-guard and revert to
`display`. No special migration needed beyond the type-guard's
`isFontMode` rejection.

## References

- Original work item: `meta/work/0034-theme-and-font-mode-toggles.md`
- Research: `meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md`
- Prerequisite plans: `meta/plans/2026-05-06-0033-design-token-system.md`,
  `meta/plans/2026-05-08-0035-topbar-component.md`
- Prior art (FOUC prevention pattern): pacocoursey/next-themes
  (`meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md:132`)
- Canonical context pattern: `src/api/use-doc-events.ts:15-175`
- Topbar slot contract: `src/components/Topbar/Topbar.tsx:7-20`,
  `Topbar.module.css:20-24`
- ADR governing CSS conventions:
  `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
