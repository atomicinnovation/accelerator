---
date: 2026-05-08T17:30:00+01:00
researcher: Toby Clemson
git_commit: ee509027f295c920a2e6ac97327cf182aa87a69e
branch: visualisation-system (jj workspace; no bookmark on @-)
repository: accelerator
topic: "Implementation of work item 0034 — Theme and Font-Mode Toggles"
tags: [research, codebase, visualiser, theming, fonts, tokens, topbar, work-item-0034]
status: complete
last_updated: 2026-05-08
last_updated_by: Toby Clemson
---

# Research: Implementation of work item 0034 — Theme and Font-Mode Toggles

**Date**: 2026-05-08T17:30:00+01:00
**Researcher**: Toby Clemson
**Git Commit**: ee509027f295c920a2e6ac97327cf182aa87a69e (workspace parent)
**Branch**: visualisation-system jj workspace, no bookmark on `@-`
**Repository**: accelerator

## Research Question

What is the current state of the visualiser frontend, and what concrete pieces
need to be authored, to implement work item 0034 (Theme and Font-Mode Toggles)?
Specifically: what has 0033 (token system) and 0035 (Topbar) already shipped,
what scope explicitly belongs to 0034, what conventions and slot contracts must
0034 conform to, and what unanswered design questions need to be resolved
before implementation begins?

## Summary

**0033 (token system) and 0035 (Topbar) have substantially landed** — the dark
colour palette, the `[data-theme="dark"]` block, the `prefers-color-scheme`
media-query mirror, the typography token layer (`--ac-font-display`,
`--ac-font-body`, `--ac-font-mono`), and two empty placeholder slots in the
Topbar (`data-slot="theme-toggle"` and `data-slot="font-mode-toggle"`) are all
already in place.

**However, the CSS-side dark theme is not actually usable today.** Although
the `[data-theme="dark"]` and `prefers-color-scheme` overrides exist, the
page chrome (`html`/`body`, `.root`, `<main>`) declares no
`background-color` or `color` and therefore consumes none of the `--ac-*`
tokens. As a result, the canvas stays browser-default white in dark mode and
several primary headings (`.title` on `LibraryDocView`, `LibraryTemplatesView`
and the bare `<h1>Templates</h1>` on `LibraryTemplatesIndex`) declare no
`color` token either, so they fall back to the user-agent default near-black
and stay dark on dark. **0034 must include a token-consumption pass that
fixes these foundational sites before the toggle can produce a usable dark
mode.** This was not enforced by the AC3 hex-grep in 0033 because that
check only catches stray hex literals, not missing token consumption.

**0034's scope splits into two halves:**

*Foundational fixes (prerequisite for any dark mode being usable):*

A. Add `background-color: var(--ac-bg)`, `color: var(--ac-fg)` and
   `color-scheme: light dark` to `body` (or `html`) in `global.css`.
B. Add `color: var(--ac-fg-strong)` to the route-level `.title` rules and
   class up the bare `<h1>Templates</h1>` in `LibraryTemplatesIndex`.
C. Re-capture the dark visual-regression baselines in
   `tests/visual-regression/__screenshots__/` once the canvas inverts —
   the existing dark baselines enshrine the broken state and will fail
   once A and B land.

*Toggle work proper:*

1. Author a `[data-font="mono"]` CSS block in `global.css` that repoints
   `--ac-font-display` and `--ac-font-body` to `var(--ac-font-mono)` (Fira
   Code).
2. Build two React-context-backed hooks (`useTheme`, `useFontMode`) that
   write `data-theme` and `data-font` to `<html>`.
3. Persist user choice to `localStorage` under the keys `ac-theme` and
   `ac-font-mode` (named in the 0035 research).
4. Add an inline boot script to `index.html` that applies the persisted
   attributes to `<html>` before React hydrates, to prevent
   flash-of-wrong-theme.
5. Render two toggle buttons into the existing Topbar slot divs.

**The implementation has two main open design choices** that the work item
itself flags but does not resolve:

- **Theme cycle shape**: binary `light` ↔ `dark`, or three-state
  `light` ↔ `dark` ↔ `system` (no attribute, defers to `prefers-color-scheme`)?
  The CSS already supports the "no attribute" path — the question is whether
  the toggle UI surfaces that mode.
- **Font-mode toggle visibility**: in the prototype the font-mode CSS hook is
  defined but no UI control exposes it. Work item 0034 says to surface a
  `Toggle font` control in the Topbar; the open question is whether it should
  be visible to end-users or hidden behind a keyboard shortcut.

A third subordinate question — **how 0034 inserts content into Topbar slots
without relocating them** — is best answered by editing `Topbar.tsx` directly
rather than using `createPortal`, because the slot divs are already at the
correct flex positions and the `.slot:empty` rule self-disengages on first
child.

## Detailed Findings

### Current State of the Frontend (foundation 0034 sits on)

**Frontend root**:
`skills/visualisation/visualise/frontend/`
**Stack**: Vite + React 19, TanStack Router, CSS Modules. Tests via Vitest +
React Testing Library + jsdom.

#### What 0033 has shipped

`src/styles/global.css` (214 lines) defines:

- **Self-hosted woff2 `@font-face` declarations** (lines 4-59): Sora 600/700,
  Inter 400/500/600/700, Fira Code 400/500. Note: the original 0033 plan
  called for Google Fonts; the implementation chose self-hosted woff2 instead
  (preloads in `index.html:7-10` for `Inter-Regular` and `Sora-Bold`).
- **`:root` block** (lines 61-137) — full token suite: 23 `--ac-*` colour
  tokens, font-family tokens, 11-step type scale, 4 line-height tokens,
  `--tracking-caps`, 11-step `--sp-N` spacing scale, 4-step radius scale, 5
  shadow tokens, plus `--ac-topbar-h: 48px`.
- **`[data-theme="dark"]` block** (lines 144-166, "MIRROR-A") — overrides
  every dark-variant `--ac-*` colour and the two theme-variant shadows.
  Status colours (`--ac-ok`, `--ac-warn`, `--ac-err`, `--ac-violet`) are
  intentionally not redefined — they stay theme-invariant.
- **`@media (prefers-color-scheme: dark)` mirror** (lines 172-197,
  "MIRROR-B") wrapped in `:root:not([data-theme="light"])`. The comment at
  lines 168-171 makes the contract explicit:
  > "honour OS dark-mode preference until 0034 ships the toggle UI; an
  > explicit `[data-theme="light"]` opts back into light."

`src/styles/tokens.ts` (120 lines) exports typed token maps:
`LIGHT_COLOR_TOKENS`, `DARK_COLOR_TOKENS`, `TYPOGRAPHY_TOKENS`,
`SPACING_TOKENS`, `RADIUS_TOKENS`, `LIGHT_SHADOW_TOKENS`, `DARK_SHADOW_TOKENS`,
`LAYOUT_TOKENS`.

`src/styles/global.test.ts` (190 lines) enforces CSS↔TS parity with a
`readCssVar` regex that has a flat-block invariant. The test also enforces
that the explicit `[data-theme="dark"]` block and the
`@media (prefers-color-scheme: dark)` mirror declare the same set of property
names with byte-equivalent values (lines 124-162). **0034 must not break
this parity test** when authoring the `[data-font="mono"]` block.

#### What 0035 has shipped

`src/components/Topbar/Topbar.tsx` (lines 7-20) renders:

```tsx
export function Topbar() {
  return (
    <header className={styles.topbar}>
      <Brand />
      <div className={styles.divider} />
      <Breadcrumbs />
      <div className={styles.spacer} />
      <OriginPill />
      <SseIndicator />
      <div className={styles.slot} data-slot="theme-toggle" />
      <div className={styles.slot} data-slot="font-mode-toggle" />
    </header>
  )
}
```

`src/components/Topbar/Topbar.module.css` lines 20-24:

```css
.slot:empty {
  width: 0;
  height: 0;
  overflow: hidden;
}
```

The slot divs are literal empty `<div>`s — there is no children prop, no
slot-fill API, no composition seam. **0034 must edit `Topbar.tsx` directly**
to populate them. The `.slot:empty` pseudo-class rule self-disengages as soon
as a child is inserted, so no class or attribute toggling is needed
(see `meta/plans/2026-05-08-0035-topbar-component.md:1753-1760`).

`src/components/Topbar/Topbar.test.tsx` lines 57-69 currently asserts both
slots exist AND have zero children. **These two assertions will break the
moment 0034 inserts content** — they need updating to assert the new
toggle-button content instead.

#### What 0034 must add

| Concern | Status |
|---|---|
| `[data-theme="dark"]` colour overrides | shipped by 0033 |
| `prefers-color-scheme` fallback | shipped by 0033 (works without any JS) |
| `[data-font="mono"]` CSS block | **0034 (must author)** |
| `ThemeContext` + `FontModeContext` providers | **0034** |
| `useTheme` / `useFontMode` hooks | **0034** |
| Theme-toggle button | **0034 (renders into existing slot)** |
| Font-mode-toggle button | **0034** |
| Inline boot script in `index.html` for FOUC | **0034** |
| `localStorage` persistence (`ac-theme`, `ac-font-mode`) | **0034** |
| Topbar slot CSS / structural placement | shipped by 0035 |

### Dark-mode deficiencies inherited from 0033

The 0033 token system shipped a `[data-theme="dark"]` block plus
`@media (prefers-color-scheme: dark)` mirror that override every `--ac-*`
colour token. These overrides only take effect for elements that consume
the `--ac-*` tokens, and in practice the page-level chrome and several
headings do not. The result is that dark mode is unusable today: the canvas
stays white and the page heading stays near-black on near-black. **0034
must include a token-consumption pass that fixes these sites** — without
it, the toggle button will produce a broken result and the change will
need to be reverted.

#### Deficiency 1 — `body` and `html` have no token-aware background or foreground

`src/styles/global.css:1-2` is the entirety of element-level base styling:

```css
*, ::before, ::after { box-sizing: border-box; }
body { margin: 0; }
```

Missing:
- `body { background-color: var(--ac-bg); color: var(--ac-fg); }` — without
  this the canvas stays the browser default (white) regardless of
  `data-theme`, and inherited text falls back to the user-agent default
  (effectively black).
- `:root { color-scheme: light dark; }` — without this the user-agent does
  not switch its built-in form controls, scrollbars, or default canvas to
  match the chosen theme.

#### Deficiency 2 — `RootLayout` declares no background or foreground

`src/components/RootLayout/RootLayout.module.css:1-17` styles `.root`,
`.body`, and `.main` for layout only. None of them set `background-color`
or `color`. The Topbar and Sidebar repaint themselves
(`Topbar.module.css:8` uses `background: var(--ac-bg-chrome)`,
`Sidebar.module.css:6` uses `background: var(--ac-bg-sunken)`), but the
`<main>` content area has nothing token-aware backing it. Once Deficiency 1
is fixed, `<main>` will inherit the body's `var(--ac-bg)` correctly — so
this can be addressed by the body fix alone, no per-`.main` change
required.

#### Deficiency 3 — Route-level `.title` rules declare no `color`

The primary document headings on two library routes inherit the
user-agent default colour because their `.title` rules don't set one:

- `src/routes/library/LibraryDocView.module.css:9` —
  `.title { font-size: 1.6rem; font-weight: 700; margin: 0 0 var(--sp-2); }`
  applied to the `<h1>` at `LibraryDocView.tsx:73`.
- `src/routes/library/LibraryTemplatesView.module.css:2` —
  `.title { font-size: var(--size-lg); font-weight: 700; margin: 0 0 var(--sp-5); }`.

Both need `color: var(--ac-fg-strong);`.

For comparison, headings that already do this and would invert correctly
once Deficiency 1 is fixed:
`src/routes/kanban/KanbanBoard.module.css:7-12` and
`src/routes/lifecycle/LifecycleClusterView.module.css:24` — both set
`color: var(--ac-fg-strong)` on `.title`.

#### Deficiency 4 — `LibraryTemplatesIndex` renders a bare `<h1>` with no class

`src/routes/library/LibraryTemplatesIndex.tsx:31`:

```tsx
<h1>Templates</h1>
```

There is no matching `.title` rule in the route's module CSS. Once
Deficiency 1 is fixed, this h1 will inherit the body's `var(--ac-fg)` — so
it will at least be readable in both modes. If a stronger heading colour
is desired (matching the `var(--ac-fg-strong)` of the other routes' titles)
the implementer should add a `.title` class and apply it here.

#### Deficiency 5 — Visual-regression baselines enshrine the broken state

`tests/visual-regression/tokens.spec.ts:20-87` already iterates
`['light', 'dark']` for every route and explicitly sets
`document.documentElement.dataset.theme = 'dark'` (line 29) before
screenshotting. Dark baselines exist on disk under
`tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/*-dark-*.png`.

These baselines were captured against the broken render (canvas stays
white-ish under Playwright's `colorScheme: dark` emulation, headings stay
near-black). The dark visual-regression suite is therefore *passing on a
broken render* — it cannot detect Deficiencies 1-4. Once 0034 fixes them,
all dark baselines will need to be re-captured.

#### Deficiency 6 — The migration test cannot catch this class of bug

`src/styles/migration.test.ts` enforces "no raw hex/px/rem literals outside
`EXCEPTIONS`". This is a *negative* check: it catches stray literals but
does not verify any element actually consumes a token. A CSS file that
declares no `color`/`background-color` at all passes the migration test
trivially — which is exactly how Deficiencies 1-4 shipped. 0034 should
either add positive coverage (e.g. assert that `body` has a
`background-color` declaration referencing `--ac-bg`) or rely on the
re-captured visual regression suite to catch future regressions.

### Hardcoded literals — informational

A grep for `#[0-9a-fA-F]{3,8}` across `src/**/*.module.css` finds exactly
two matches, both at `src/components/MarkdownRenderer/MarkdownRenderer.module.css:14`:

```css
background: #1e1e1e; color: #d4d4d4;
```

This styles `.markdown pre` (code blocks) and is admitted as
`kind: 'irreducible'` at `migration.test.ts:54-55`. Visually intentional —
code blocks intentionally stay editor-themed in both modes — and not
contributing to the dark-mode bug.

### Source-document evidence

#### `meta/work/0034-theme-and-font-mode-toggles.md` (the work item itself)

Requirements (lines 33-39): theme context hook setting `data-theme`
(`light`/`dark`); font-mode hook setting `data-font` (`default`/`mono`);
persist to `localStorage`; surface both toggles in the Topbar; ensure every
component consumes `--ac-*` so swap is automatic without per-component
conditionals.

Acceptance criteria (lines 42-45) require: attribute switching on `<html>`,
token resolution to theme-appropriate values, persistence across reloads,
and **no flash of wrong theme** on initial paint (explicitly mentioning
"e.g. via inline boot script that reads `localStorage` and applies the
attribute before React hydrates").

Open questions (lines 49-50):
- "Should the initial theme follow the OS `prefers-color-scheme` setting
  when no stored value exists, or default to `light`?"
- "Is the font-mode toggle intended for end-users or only for
  developers/designers (i.e. should it be discoverable in the topbar or
  hidden behind a keyboard shortcut)?"

#### `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`

Lines 27, 33, 63-65 collectively prescribe the architecture: `data-theme`
on `<html>`, theme toggle in the Topbar, persistence via `localStorage`, and
the font-mode hook "mirroring the theme toggle" (same shape, different
attribute). Line 89 places theming/font-mode immediately after the token
pass in the suggested sequencing.

#### `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`

- Attribute values: `data-theme="light|dark"` and `data-font="display|mono"`
  (line 227). Note `display` not `default` — this is the prototype's name.
  The work item uses `default` (line 35); **0034 should pick one and align
  with the prototype's `display`** for consistency with the inventory's
  ground truth.
- The `[data-font="mono"]` rule (line 182) "swaps `--ac-font-display` and
  `--ac-font-body` to Fira Code" — both display and body, not just body.
- Theme toggle UX (line 225, 345): a single button in the topbar (right
  side), uses a sun/moon icon. The interaction "toggles `data-theme`".
- Font-mode toggle UX (lines 413-416): "**not exposed in the visible UI of
  this prototype build**, but defined in CSS at `[data-font="mono"]`."
  Implementer of 0034 has no prototype precedent for the font-mode toggle's
  visual design — the gap analysis says to mirror the theme toggle.

#### `meta/decisions/ADR-0026-css-design-token-application-conventions.md`

Conventions 0034's CSS must obey:

- Tinted hover/active states on toggle buttons must use
  `color-mix(in srgb, var(--ac-X) N%, var(--ac-bg))` with locked percentages:
  8% (panel wash), 18% (hover), 30% (border tint). No new percentages.
- Spacing/typography literals within ±2px of a token must substitute; em
  values are irreducible.
- Sub-pixel borders (`1px`, `1.5px`) and fixed icon dimensions (e.g. a
  `14px` sun icon) are allowed as irreducible literals but must be added
  to `EXCEPTIONS` in `src/styles/migration.test.ts`.
- A `6px` border-radius is forbidden — round to `--radius-sm` (4px) or
  `--radius-md` (8px); pill buttons use `--radius-pill` (999px).
- All blues collapse to `var(--ac-accent)`.

#### `meta/research/codebase/2026-05-07-0035-topbar-component.md` and
#### `meta/plans/2026-05-08-0035-topbar-component.md`

The 0035 plan crystallises the contract 0034 inherits:

- **Slot positions are fixed** — 0034 must not relocate them
  (`plans/2026-05-08-0035-topbar-component.md:1753-1760`).
- **`localStorage` keys named in the 0035 research**: `ac-theme` and
  `ac-font-mode` (`research/2026-05-07-0035-topbar-component.md:310-316`).
  These keys appear nowhere else; 0034 should adopt them as the canonical
  names.
- The `.slot:empty` rule "self-disengages as soon as 0034 inserts a child
  element, so 0034 does not need to clear any attribute or class — just
  render content."
- 0035 also flagged (open question 5) that two implementation paths exist
  for content insertion — replace markup in `Topbar.tsx` or use
  `createPortal`. The plan does not pick a side. Given that the slots are
  hard-coded in `Topbar.tsx` already, **direct edit is the simplest path**
  and avoids the additional indirection of a portal.

### React Context Pattern (the canonical model 0034 should imitate)

**File**: `src/api/use-doc-events.ts`. The pattern shape end-to-end:

```ts
// 1. Handle interface — public contract
export interface DocEventsHandle {
  setDragInProgress(v: boolean): void
  connectionState: ConnectionState
  justReconnected: boolean
}

// 2. Factory builds the hook (so tests can swap dependencies)
export const useDocEvents = makeUseDocEvents((url) => new EventSource(url))

// 3. Default handle — sane no-op state for context-default consumers
const _defaultHandle: DocEventsHandle = {
  setDragInProgress: () => {},
  connectionState: 'connecting',
  justReconnected: false,
}

// 4. Context with non-throwing default
export const DocEventsContext = createContext<DocEventsHandle>(_defaultHandle)

// 5. Consumer hook
export function useDocEventsContext(): DocEventsHandle {
  return useContext(DocEventsContext)
}
```

There is **no separate `Provider` component** — `RootLayout.tsx` calls the
factory hook and uses `<DocEventsContext.Provider value={...}>` directly:

```tsx
// src/components/RootLayout/RootLayout.tsx:10-30
export function RootLayout() {
  const docEvents = useDocEvents()
  // ...
  return (
    <DocEventsContext.Provider value={docEvents}>
      <div className={styles.root}>
        <Topbar />
        ...
      </div>
    </DocEventsContext.Provider>
  )
}
```

**Test injection pattern**:
- Hook unit tests inject fakes via `makeUseXxx(factory)` with a `vi.fn()`
  factory (`src/api/use-doc-events.test.ts:122-158`).
- Consumer-component tests `vi.mock(...)` on the consumer hook import and
  set return value per test (`Topbar.test.tsx:6-8`).

### Topbar font cascade (relevant for font-mode mechanics)

The Topbar has no own `font-family` declaration. Fonts cascade from
`RootLayout.module.css:5` (`.root { font-family: var(--ac-font-body); }`).
Children override per-component:

- `Brand.module.css:20` — `.brandName` uses `var(--ac-font-display)` (Sora).
- `Brand.module.css:27`, `Breadcrumbs.module.css:13` — `var(--ac-font-body)`
  (Inter).
- `OriginPill.module.css:5`, `SseIndicator.module.css:5` —
  `var(--ac-font-mono)` (Fira Code).

**Implication**: a token-level override in `[data-font="mono"]` that
remaps `--ac-font-display` and `--ac-font-body` to
`var(--ac-font-mono)` will affect the topbar uniformly via cascade —
including Brand wordmark and Breadcrumbs (because they use the display/body
tokens). The OriginPill and SseIndicator already use `--ac-font-mono`
directly and would be unaffected (correctly — they are mono in both modes).

### Test infrastructure

- `src/test/setup.ts` provides a `MockEventSource` and `MockResizeObserver`
  via `Object.defineProperty` (survives `vi.unstubAllGlobals`). Provides
  no helpers for `localStorage`, `documentElement`, or context-provider
  wrapping.
- jsdom provides a real `localStorage`. `restoreMocks: true` in
  `vite.config.ts:53` does **not** clear it — 0034 tests must explicitly
  call `localStorage.clear()` and reset `documentElement` attributes in
  `beforeEach` / `afterEach`.
- `tests/visual-regression/tokens.spec.ts:29` already uses
  `document.documentElement.dataset.theme = 'dark'` — confirms the attribute
  name and writing path 0034 should use.
- No project-specific augmentation in `src/vite-env.d.ts` is needed;
  `localStorage` is typed by lib.dom.

### Inline boot script — Vite mechanics

`vite.config.ts` registers only the `react()` plugin; no HTML-transform
plugin is in place. `index.html` is processed by Vite's default HTML
pipeline, which handles `<script type="module">` and `<link rel="preload">`
asset rewriting. **A plain inline `<script>` in `<head>` will be passed
through as-is** by Vite — this is the conventional FOUC-prevention path:

```html
<script>
  (function () {
    try {
      const t = localStorage.getItem('ac-theme')
      if (t === 'light' || t === 'dark') {
        document.documentElement.setAttribute('data-theme', t)
      }
      const f = localStorage.getItem('ac-font-mode')
      if (f === 'mono') document.documentElement.setAttribute('data-font', f)
    } catch (_) { /* private mode etc. — fall through to defaults */ }
  })()
</script>
```

A module script (i.e. running this code at the top of `src/main.tsx`) is
**not** sufficient — `<script type="module">` is deferred, so the browser
will paint the default light theme briefly before the React bundle runs.
The inline classic `<script>` placed in `<head>` (before the React module
import) is the only path that prevents FOUC in this stack.

## Code References

### Style foundation (read-only — already shipped by 0033)

- `skills/visualisation/visualise/frontend/src/styles/global.css:4-59` —
  self-hosted `@font-face` for Sora, Inter, Fira Code.
- `skills/visualisation/visualise/frontend/src/styles/global.css:88-90` —
  `--ac-font-display`, `--ac-font-body`, `--ac-font-mono` token definitions
  (the swap targets for `[data-font="mono"]`).
- `skills/visualisation/visualise/frontend/src/styles/global.css:144-166` —
  `[data-theme="dark"]` block (MIRROR-A).
- `skills/visualisation/visualise/frontend/src/styles/global.css:168-197` —
  `prefers-color-scheme` fallback comment + MIRROR-B block.
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:124-162` —
  parity test that 0034 must keep green.
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts` —
  literal-substitution gate; any new literals 0034 introduces must either
  match a substitution rule or be listed in `EXCEPTIONS`.

### Topbar slots (read-only — already shipped by 0035)

- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.tsx:7-20` —
  the slot divs.
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.module.css:20-24` —
  `.slot:empty` collapse rule.
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.test.tsx:57-69` —
  emptiness assertions that 0034 must update.
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.test.tsx:76-80` —
  CSS-source assertion (can stay).

### Provider model (canonical pattern to imitate)

- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:15-19` —
  handle interface.
- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:86-160` —
  factory pattern.
- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:165-175` —
  default handle, context, consumer hook.
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:10-30` —
  Provider wiring at app shell level.
- `skills/visualisation/visualise/frontend/src/api/use-doc-events.test.ts:122-158` —
  factory-injection test pattern.

### Files 0034 will touch

- `skills/visualisation/visualise/frontend/src/styles/global.css` — add
  `body { background-color: var(--ac-bg); color: var(--ac-fg); }` and
  `:root { color-scheme: light dark; }` (foundational fix); append
  `[data-font="mono"]` block; consider extending `migration.test.ts`
  exclusions if new literals are added.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css` —
  add `color: var(--ac-fg-strong)` to `.title` (foundational fix).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css` —
  add `color: var(--ac-fg-strong)` to `.title` (foundational fix).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`
  and its `.module.css` — class up the bare `<h1>Templates</h1>`
  (foundational fix; only needed if a stronger heading colour is desired
  beyond the inherited `var(--ac-fg)`).
- `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/*-dark-*.png` —
  re-capture all dark baselines once the canvas inverts correctly.
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts` — no change
  expected (font-mode does not introduce new token names; it remaps
  existing ones).
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts` — add
  parity assertions for the `[data-font="mono"]` block.
- `skills/visualisation/visualise/frontend/index.html` — inline boot script
  in `<head>`.
- `skills/visualisation/visualise/frontend/src/main.tsx` — likely no change
  (provider wiring lives in `RootLayout`).
- `skills/visualisation/visualise/frontend/src/api/use-theme.ts` (new) — hook
  + context.
- `skills/visualisation/visualise/frontend/src/api/use-theme.test.ts` (new).
- `skills/visualisation/visualise/frontend/src/api/use-font-mode.ts` (new).
- `skills/visualisation/visualise/frontend/src/api/use-font-mode.test.ts`
  (new).
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx` —
  add `ThemeContext.Provider` + `FontModeContext.Provider` wrappers.
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.tsx` —
  populate the two slot divs.
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.test.tsx` —
  replace emptiness assertions with content assertions.
- `skills/visualisation/visualise/frontend/src/components/ThemeToggle/ThemeToggle.tsx`
  (new), `.module.css`, `.test.tsx` — sun/moon icon button.
- `skills/visualisation/visualise/frontend/src/components/FontModeToggle/FontModeToggle.tsx`
  (new), `.module.css`, `.test.tsx` — mono toggle (UX TBD per OQ2).

## Architecture Insights

- **0033's MIRROR-A / MIRROR-B duplication** is intentional and load-bearing.
  The `[data-theme="dark"]` block is flat (no nested rules) so the parity
  test's `readCssVar` regex can extract values; the `@media` block is
  byte-equivalent to MIRROR-A and supplies the OS-preference fallback. 0034
  must not refactor these into a shared selector list and must not
  introduce any third dark-token site.
- **`data-theme` absence is meaningful**, not a bug. When `data-theme` is
  unset, `:root:not([data-theme="light"])` matches inside the
  `prefers-color-scheme` media query, so the OS preference governs the
  palette automatically. This means 0034 has a third "system" mode for
  free at the CSS layer — the only question is whether the toggle UI
  surfaces it. *However, the "free" system mode only works once the
  canvas itself consumes `--ac-bg` (Deficiency 1) — at present the OS
  preference path overrides the tokens but the body is still white.*
- **0033's AC3 grep is necessary but not sufficient.** A "no raw hex
  literals" check ensures any colour that *is* set comes from a token,
  but doesn't ensure colour is set at all. The dark-mode regressions
  enumerated above are exactly this gap: chrome and headings declare no
  `color`/`background-color`, so the AC3 grep finds nothing to flag while
  the dark theme is structurally broken.
- **Boot script before module load** is the canonical FOUC-prevention path
  in this stack. Module scripts are deferred and will paint default theme
  briefly. The inline classic `<script>` in `<head>` is the only path that
  works.
- **Direct slot editing beats portals.** The Topbar slot divs are
  hard-coded; 0034 should edit `Topbar.tsx` to render `<ThemeToggle />` and
  `<FontModeToggle />` as the slot children. `createPortal` would add
  indirection for no benefit, since Topbar already renders the slots in
  the right place.
- **Font-mode toggle is a token remap, not a per-component change.** The
  cascade through `.root` (which sets `font-family: var(--ac-font-body)`)
  means swapping the value of `--ac-font-body` propagates everywhere.
  Components that explicitly use `--ac-font-display` (e.g. Brand wordmark)
  also follow because the `[data-font="mono"]` rule remaps both tokens.
- **No localStorage usage exists yet** anywhere in the frontend. 0034 is
  introducing a persistence mechanism the rest of the app doesn't yet rely
  on; future work (e.g. sidebar collapsed state, kanban filters) will
  likely follow a similar pattern, so 0034 should aim for a clean, reusable
  shape rather than a one-off.

## Historical Context

- **`meta/decisions/ADR-0026-css-design-token-application-conventions.md`** —
  governs all CSS work post-0033. 0034's toggle button styles, hover/active
  tints, and any irreducible literals must conform.
- **`meta/research/codebase/2026-05-06-0033-design-token-system.md:30,250`** —
  confirms the scope split: 0033 ships dark *values*, 0034 ships the
  *toggle* UI and persistence.
- **`meta/plans/2026-05-06-0033-design-token-system.md:160-163,167-168,
  654-666`** — locks in the `prefers-color-scheme` mirror mechanic and the
  `[data-font="mono"]` deferred scope (repoint both `--ac-font-display`
  and `--ac-font-body`).
- **`meta/research/codebase/2026-05-07-0035-topbar-component.md:310-316`** —
  canonical localStorage key names: `ac-theme` and `ac-font-mode`.
- **`meta/plans/2026-05-08-0035-topbar-component.md:1437-1441,1465-1471,
  1753-1760`** — slot markup, `.slot:empty` rule, and the explicit "0034
  slot contract" section.
- **Memory `project_visualiser_design_system_decisions.md`** — three-family
  font stack (Sora/Inter/Fira Code), Raleway dropped, dark token values
  shipped in 0033, 0033 stays as a single story. Note: the memory says
  "Google Fonts (not self-hosted)", but the implementation in `global.css`
  uses self-hosted woff2 — the memory is stale relative to what shipped.
  This research finds self-hosted fonts in the current codebase.

## Related Research

- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — token system
  research that defined 0034's foundation.
- `meta/research/codebase/2026-05-07-0035-topbar-component.md` — Topbar research
  that defined the slot contract 0034 consumes.
- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — overarching
  design convergence approach.
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` —
  visualiser implementation background.

## Open Questions

These are the questions the implementer of 0034 must resolve before
finalising the design (most are flagged in the work item itself):

1. **Theme toggle cycle shape** (work item OQ1). The CSS supports three
   states for free: `light` (explicit), `dark` (explicit), `<no attribute>`
   (defers to OS preference). The toggle UX could be:
   - Binary cycle `light` ↔ `dark` (writes attribute on first interaction;
     no system mode in UI; ignores OS preference once user has chosen).
   - Three-state cycle `light` → `dark` → `system` → `light` (where `system`
     means clearing the attribute).
   - Two-state with a separate "auto" indicator. The work item's "single
     `Toggle theme` button cycling between values" wording suggests
     binary, but does not foreclose three-state.

2. **Font-mode toggle visibility** (work item OQ2). The prototype defines
   the CSS hook but does not expose a UI control. The work item says the
   toggle goes in the Topbar; the open question is whether it should be
   discoverable to end-users (visible button next to the theme toggle) or
   hidden behind a keyboard shortcut for developer/designer use only.
   The slot is already reserved either way (`data-slot="font-mode-toggle"`),
   so the question is *what to render into it* — a visible button, or an
   off-screen `<button>` reachable only by keyboard / dev shortcut.

3. **Attribute-value naming consistency.** The work item says
   `data-font="default"` (line 35); the prototype inventory says
   `data-font="display"` (line 227). Recommend aligning with the inventory
   (`display`/`mono`) since the inventory is the canonical token-naming
   source.

4. **Default initial theme.** When no `localStorage` value is present, does
   the boot script default to `light`, or leave the attribute unset (so the
   `prefers-color-scheme` mirror takes over)? The latter is simpler and
   matches the comment in `global.css:168-171`. The former makes the app
   light-by-default regardless of OS preference. The work item's OQ1 frames
   this as a decision.

5. **Toggle button visual design.** The prototype's theme toggle uses a
   sun/moon icon (inventory line 225); ADR-0026's `--radius-md` (8px) or
   `--radius-pill` (999px) are the candidate radii. Specific iconography
   and dimensions (icon size, button padding) need to be designed — no
   prototype-component CSS exists to copy from. The font-mode toggle has
   no prototype precedent at all.

6. **`@font-face` self-hosted vs Google Fonts memory drift.** The memory
   note `project_visualiser_design_system_decisions.md` states "Google
   Fonts (not self-hosted)" but the shipped code uses self-hosted woff2.
   This is informational and outside 0034's scope — the implementer
   should flag the memory note as stale (this research updates it).

7. **`localStorage` write timing.** Should the hook write on every state
   change, or debounce? Given typical theme-toggle frequency (rare),
   write-on-change is fine; calling out explicitly so it's not raised in
   review.

8. **Scope boundary for the foundational dark-mode fixes.** The token-
   consumption fixes (canvas background, heading colours, baseline
   re-capture) are arguably 0033 follow-up rather than 0034. They must
   land before the toggle to avoid shipping a broken-on-flip experience,
   but the implementer should decide whether to:
   - bundle them into the 0034 PR (recommended — the toggle is unusable
     without them and a single PR keeps the bisect clean), or
   - file a separate "0033.1" follow-up that lands first, with 0034 as a
     pure additive change on top.
