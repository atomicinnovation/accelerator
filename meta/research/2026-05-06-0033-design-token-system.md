---
date: "2026-05-06T17:19:01+01:00"
researcher: "Toby Clemson"
git_commit: "bf60483a3af7e1519824704cb04cdca995bc2a70"
branch: "HEAD"
repository: "accelerator"
topic: "Implementation of work item 0033 — Design Token System"
tags: [research, codebase, design-tokens, visualiser, frontend, css, theming, typography]
status: complete
last_updated: "2026-05-06"
last_updated_by: "Toby Clemson"
---

# Research: Implementation of work item 0033 — Design Token System

**Date**: 2026-05-06T17:19:01+01:00
**Researcher**: Toby Clemson
**Git Commit**: bf60483a3af7e1519824704cb04cdca995bc2a70
**Branch**: HEAD
**Repository**: accelerator (workspaces/visualisation-system)

## Research Question

What does an implementer need to know about the existing visualiser frontend to deliver work item 0033 (Design Token System)? Specifically: where do tokens live today, how is the CSS↔TS parity invariant enforced, what is the per-file consumer surface that has to be migrated, what concrete mapping decisions have to be made along the way, and what gotchas exist in the work item's own acceptance criteria when run against the current codebase?

## Summary

The frontend is a Vite + React 19 SPA at `skills/visualisation/visualise/frontend/` with a deliberately minimal CSS-custom-property system today: **eight `--color-*` tokens** in a single `:root` block, mirrored as a frozen TypeScript object, with a parity test that loops `Object.entries(COLOR_TOKENS)` through a regex extractor against `global.css?raw`. Component CSS Modules barely consume any of those tokens — exactly **6 `var(--*)` references across two files**, all using the defensive `var(--token, #fallback)` two-arg form. Everything else — colours, font sizes, spacing, radii, shadows, the single `system-ui` body face, the lone monospace family for slugs — is hard-coded inline across **16 component CSS Module files** and the global stylesheet `wiki-links.global.css`.

0033 is the load-bearing first pass to fix this: introduce the prototype's `--ac-*` colour, typography, spacing, radius, and shadow scales (with full dark overrides under `[data-theme="dark"]`), wire the three Google Fonts in `index.html`, and migrate every literal in component modules onto the new tokens (with a documented escape-hatch for irreducible 1–3px hairlines, `0` resets, and any remaining no-clean-mapping cases). The dark token *values* ship in 0033; the toggle UI and persistence ship separately in 0034.

The existing parity-test idiom extends cleanly — the regex extractor `readCssVar` doesn't care whether a token is a hex, an `rgba(...)`, a comma-separated font-stack, or a multi-segment shadow — but the implementer needs to know that this idiom currently lives in `contrast.test.ts`, not `global.test.ts` as the work item Technical Notes claims. Most of the mapping decisions are mechanical (`#111827` → `--ac-fg-strong`, `1rem` → `--sp-4`, `9999px` → `--radius-pill`) but a small number are judgement calls that affect both AC4 (px/rem grep) and AC6 (visual parity): the recurring "error tint" trio (`#fef2f2` / `#fecaca` / `#991b1b`) has no clean `--ac-*` mapping, two blue shades (`#2563eb` vs `#1d4ed8`) need a single accent or a deliberate split, layout literals (`max-width: 800px`, `flex-basis: 220px`, `minmax(320px, 1fr)`) sit outside the spacing scale entirely, and the two existing `box-shadow` rules in `LifecycleClusterView` are coloured geometric rings, not elevation shadows.

There are also two latent issues with the work item's acceptance grep commands worth flagging early: (a) the path-based exclusion globs in AC3 and AC4 (`-g '!src/styles/global.css'`) are anchored relative to the invocation cwd — running them from the workspace root rather than the frontend root will leak `global.css` and `tokens.ts` into the match set; and (b) the work item's "17 modules" count is off by one — there are exactly **16** `*.module.css` files under `src/`.

## Detailed Findings

### Existing token authoring surface

The full current token system is **eight named colour custom properties**, defined once in `:root` and mirrored as a frozen TypeScript object.

- `skills/visualisation/visualise/frontend/src/styles/global.css:1-10` — the `:root` block; one declaration per colour, column-aligned, terminated by `;`. Lines 12-21 below define the global `:focus-visible` outline ring and the `forced-colors` override, both consuming `--color-focus-ring`. There is no `[data-theme]` block, no other token category, and no font/spacing/radius/shadow declaration outside what each component module hard-codes inline.
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:1-12` — exports `COLOR_TOKENS` as `as const` (frozen literal object) and `ColorToken` (the keyof type). The hex strings exactly mirror the CSS values, byte-for-byte, including capitalisation (`#0f172a`, all-lowercase).
- `skills/visualisation/visualise/frontend/src/styles/contrast.ts:1-24` — `contrastRatio(fg, bg)` helper that parses `#rrggbb`, applies sRGB linearisation, and returns the WCAG ratio. Used by `contrast.test.ts` to assert that token combinations meet AA.
- `skills/visualisation/visualise/frontend/src/styles/wiki-links.global.css:1-22` — the second global stylesheet, imported from `main.tsx`. Hard-codes `#9ca3af` (line 11) and `#6b7280` (line 18). The header comment explains that these classes are intentionally global because they're emitted by the remark plugin as literal kebab-case strings on `data.hProperties` (CSS-module hashing would break the selector match). That global-on-purpose policy is unchanged by 0033 — only the literals migrate.
- `skills/visualisation/visualise/frontend/src/main.tsx:7-9` — the only place global stylesheets are loaded: vendor (`highlight.js/styles/github.css`), then `./styles/global.css`, then `./styles/wiki-links.global.css`, in that order, as side-effect imports. Vite bundles them.
- `skills/visualisation/visualise/frontend/index.html` — minimal `<head>` (`<meta charset>`, `<meta viewport>`, `<title>` only). No `<link rel="preconnect">`, no `<link rel="stylesheet">`. Per AC2, three new `<link>` elements (preconnect to `fonts.googleapis.com` + `fonts.gstatic.com` + the Google Fonts stylesheet `<link>`) belong here.

### Existing CSS↔TS parity invariant

The pattern that 0033 must extend lives in `contrast.test.ts` (despite the work item Technical Notes claim that it lives in `global.test.ts`):

```ts
// skills/visualisation/visualise/frontend/src/styles/contrast.test.ts:6-18
function readCssVar(name: string): string | null {
  const re = new RegExp(`--${name}:\\s*([^;]+);`)
  const m = re.exec(globalCss)
  return m ? m[1].trim() : null
}

describe('tokens.ts is the single source of truth for :root colour values', () => {
  for (const [name, value] of Object.entries(COLOR_TOKENS)) {
    it(`global.css :root --${name} matches COLOR_TOKENS.${name}`, () => {
      expect(readCssVar(name)).toBe(value)
    })
  }
})
```

The regex captures everything between `--<name>:` and the next `;`, then `.trim()`s. It works unchanged for any token type because:

- Hex values, `rgba(...)`, comma-separated font stacks (`"Sora", system-ui, sans-serif`), multi-segment shadows (`0 1px 2px rgba(...), 0 8px 28px rgba(...)`), and unit-bearing scalars (`12px`, `0.12em`, `1.6`) all terminate with the literal declaration `;` — no inner semicolons exist in any prototype token value.
- The `Object.entries(...)` loop is the test-generation primitive: each new token category (`TYPOGRAPHY_TOKENS`, `SPACING_TOKENS`, `RADIUS_TOKENS`, `SHADOW_TOKENS`) wraps in its own `describe` block and runs the same regex helper. AC1's "extend `global.test.ts` to assert CSS↔TS parity over the full set" is mechanically straightforward — but the implementer either needs to (i) move the parity describe blocks from `contrast.test.ts` into `global.test.ts` to honour the work item's explicit file reference, or (ii) note in the PR that the work item's pointer is to the wrong file. Recommend (i) plus migrating the existing colour parity loop along with it, so the file naming reflects intent.

The dark `[data-theme="dark"]` override block is harder to test with the same regex. `readCssVar` matches the *first* occurrence of `--<name>:` in `global.css`, so for tokens that are redefined under `[data-theme="dark"]`, asserting both light and dark values requires either (a) adding a second helper that scopes regex matches to a section (e.g., capturing the `[data-theme="dark"] { ... }` block first, then running the regex inside it), or (b) splitting the colour token map into `LIGHT_COLOR_TOKENS` and `DARK_COLOR_TOKENS` with separate parity describes. Option (b) is cleaner and more idiomatic given the existing pattern.

The `?raw` import is provided by Vite (not Vitest); `vite.config.ts` is a single shared config (`defineConfig` from `vitest/config`) which means raw asset imports work in tests with no extra wiring. `package.json` shows Vite 6 + Vitest 3 + React 19. (`vite.config.ts:38-56`.)

### The current consumer surface — what will be touched

`find skills/visualisation/visualise/frontend/src -name '*.module.css'` returns exactly **16 CSS Modules**:

```
src/components/FrontmatterChips/FrontmatterChips.module.css
src/components/MarkdownRenderer/MarkdownRenderer.module.css
src/components/PipelineDots/PipelineDots.module.css
src/components/RelatedArtifacts/RelatedArtifacts.module.css
src/components/RootLayout/RootLayout.module.css
src/components/Sidebar/Sidebar.module.css
src/components/SidebarFooter/SidebarFooter.module.css
src/routes/kanban/KanbanBoard.module.css
src/routes/kanban/KanbanColumn.module.css
src/routes/kanban/WorkItemCard.module.css
src/routes/library/LibraryDocView.module.css
src/routes/library/LibraryTemplatesIndex.module.css
src/routes/library/LibraryTemplatesView.module.css
src/routes/library/LibraryTypeView.module.css
src/routes/lifecycle/LifecycleClusterView.module.css
src/routes/lifecycle/LifecycleIndex.module.css
```

(The work item Technical Notes line 204 claims "17 modules" — actual count is 16. Off-by-one; nothing in the AC depends on the exact figure.)

Plus two global stylesheets (`global.css`, `wiki-links.global.css`) and `index.html`.

#### Baseline counts for AC3, AC4, AC5 (verified at HEAD)

Run from `workspaces/visualisation-system/`:

- **AC3 hex baseline (verified):** 18 files match the hex regex with the work item's exclusion globs. Per-file totals (`rg --count-matches`):
  - LifecycleClusterView 25, LifecycleIndex 24, KanbanBoard 15, LibraryTypeView 14, MarkdownRenderer 9, LibraryDocView 8, KanbanColumn 7, FrontmatterChips 5, PipelineDots 5, LibraryTemplatesIndex 4, Sidebar 9, SidebarFooter 3, RelatedArtifacts 9, LibraryTemplatesView 10, WorkItemCard 9, wiki-links.global.css 2, plus **`src/styles/global.css:8` and `src/styles/tokens.ts:8` leaking into the match set** (see "AC3/AC4 grep glob anchoring" below).
- **AC4 px/rem baseline (verified):** 18 files match the px/rem regex with the work item's exclusion globs. Per-file totals: LifecycleClusterView 43, LifecycleIndex 37, KanbanBoard 23, MarkdownRenderer 23, LibraryDocView 22, LibraryTypeView 22, LibraryTemplatesView 20, RelatedArtifacts 18, KanbanColumn 14, FrontmatterChips 13, Sidebar 12, WorkItemCard 10, LibraryTemplatesIndex 9, PipelineDots 6, SidebarFooter 6, RootLayout 2, **`wiki-links.global.css:1` (correctly captured)**, **`src/styles/global.css:2` (leaked)**.
- **AC5 token-reference baseline (verified):** `rg 'var\(--' ... -g '*.module.css' --count-matches | awk -F: '{s+=$2} END {print s}'` returns `6` — matches the work item's stated baseline. The six sites are:
  - `src/components/SidebarFooter/SidebarFooter.module.css:6,7,19` — `var(--color-muted-text, #4b5563)`, `var(--color-divider, #e5e7eb)`, `var(--color-warning-text, #7c2d12)`.
  - `src/routes/library/LibraryDocView.module.css:24,25,26` — `var(--color-warning-bg, #fff8e6)`, `var(--color-warning-border, #d97706)`, `var(--color-warning-text, #7c2d12)`.

  Every existing site uses the **two-arg `var(--token, #fallback)` form** with the same hex literal as defined in `:root`. The fallback is redundant safety in case the cascade fails. This is a convention worth preserving (or explicitly retiring) in 0033 — both options have implications:
  - **Preserve fallbacks**: 300+ token references each carry their literal value, and AC3's hex regex would re-flag every fallback hex inside `var(--ac-fg-strong, #0A111B)` and force AC3 to fail. AC3 must therefore exclude `var()` arguments — but `rg` can't easily do that with a single regex. **The simplest reading is: drop the two-arg form for new sites and lean on the cascade.**
  - **Drop fallbacks**: cleaner, AC3 stays mechanical, but the existing 6 sites should be migrated to the no-fallback form too for consistency.

#### Per-file migration map for the three worst offenders

The codebase analyser produced concrete colour and spacing maps for the three worst offenders (`LifecycleClusterView`, `LifecycleIndex`, `KanbanBoard`). Highlights:

**Clean colour mappings** (mechanical replacements):
- `#111827` → `--ac-fg-strong` (strong heading text)
- `#374151` → `--ac-fg`
- `#6b7280` → `--ac-fg-muted`
- `#9ca3af` → `--ac-fg-faint`
- `#ffffff` → `--ac-bg-card` (card surfaces) or `--ac-bg` (page chrome)
- `#e5e7eb` → `--ac-stroke-soft` (timeline rail, card border, dashed dividers)
- `#d1d5db` → `--ac-stroke` (input/button strong border)
- `#1d4ed8` → `--ac-accent` (link hover, focus, active state)
- `#dbeafe` → `--ac-accent-tint` (active button background)
- `#f3f4f6` → `--ac-bg-sunken` (inactive/absent stage backgrounds)
- `#991b1b` → `--ac-err` (error text)

**Mappings requiring a decision**:
- `#fef2f2` (error tint background) and `#fecaca` (error border) appear in `LifecycleClusterView.module.css:117`, `LifecycleIndex.module.css:106`, `KanbanBoard.module.css:31,32,39,51,52`, **and** `#fee2e2` (deeper error tint, hover) appears at `KanbanBoard.module.css:45`. The prototype's `--ac-*` palette has no `--ac-err-tint` / `--ac-err-stroke` / `--ac-err-faint-hover`. Options: (i) extend the palette with `--ac-err-tint` and `--ac-err-stroke` (requires updating the prototype-derived inventory; cleanest but expands scope); (ii) document the four error literals in the PR description's "Irreducible-literal exceptions" list per AC4's escape-hatch (and a parallel hex-exception list for AC3 — but AC3 has no such escape-hatch as written, so this requires either an AC3 amendment or moving the error-state styles to use the existing `--ac-err` colour with `color-mix(in srgb, var(--ac-err) 8%, white)` or a new `--ac-err-faint` token); (iii) compose them in CSS via `color-mix()` from `--ac-err`, which sidesteps the AC3 grep entirely. **Option (iii) is the cleanest interpretation of "no clean mapping" without growing the token set.**
- `#2563eb` (LifecycleClusterView line 48 only, the active-stage dot fill) versus `#1d4ed8` (everywhere else, the accent). The prototype's `--ac-accent` is `#595FC8` (indigo, not blue) — so neither maps cleanly. The migration must choose between (a) snapping both to `--ac-accent` (visual change), (b) keeping the existing blue family by introducing a `--ac-accent-blue` that sits alongside `--ac-accent` indigo, or (c) accepting both as part of a wider palette change driven by AC6's ΔE < 5 visual-parity tolerance. **Option (a) accepts a small but visible hue shift on every accent surface and is consistent with the prototype's intent.** This must be acknowledged in the PR description per AC6.
- The two blue shades collapse to a single `--ac-accent` if (a) is chosen, or stay distinct only if a per-stage variant is added. The `LifecycleClusterView.module.css:50` halo (`box-shadow: 0 0 0 1.5px #1d4ed8`) is a coloured ring, not an elevation; see "Shadow tokens" below.

**Clean spacing mappings**:
- `0.25rem` (4px) → `--sp-1`; `0.5rem` (8px) → `--sp-2`; `0.75rem` (12px) → `--sp-3`; `1rem` (16px) → `--sp-4`; `1.5rem` (24px) → `--sp-5`; `2rem` (32px) → `--sp-6`.

**Off-scale spacing requiring per-call-site judgement**: `0.4rem`, `0.55rem`, `0.6rem`, `0.7rem`, `0.8rem`, `0.85rem`, `1.25rem`, `1.4rem`, `1.75rem`, `6px` (LifecycleIndex toolbar gap, LifecycleClusterView stage margin-left). For each, the choice is either (a) round into the spacing scale (mostly to the nearest `--sp-N`, accepting ±2px-class drift bounded by AC6), (b) recognise it as a font-size and migrate it to the typography scale (`--size-xs`, `--size-sm`, etc.), or (c) declare it irreducible and list it in the PR's exceptions per AC4. **The right policy is to first separate font-size literals (the majority) from spacing literals, migrate font-sizes to the typography scale, then assess remaining spacing values for round-or-keep on a per-site basis.** The codebase analyser counted ~20 off-scale values across the three modules; expect 50–70 across all 16 modules.

**Clean radius mappings**:
- `0.25rem` / `4px` → `--radius-sm`; `8px` → `--radius-md`; `12px` → `--radius-lg`; `9999px` / `999px` → `--radius-pill`.

**Off-scale radius**: `2px` (FrontmatterChips badge), `3px` (MarkdownRenderer inline code), `6px` (LifecycleIndex card, line 71). Per the prototype inventory, the four-step scale is `4 / 8 / 12 / 999`. `2px` and `3px` are sub-`--radius-sm`; flag as irreducible. `6px` rounds to `--radius-md` (8px), accepting a 2px drift, OR keeps as inline literal in PR exceptions list.

**Layout literals** that are not in the spacing scale at all (need explicit policy): `max-width: 800px` (LifecycleClusterView), `900px` (LifecycleIndex), `1100px` (LibraryDocView), `720px` (MarkdownRenderer), `600px` (LibraryTemplatesIndex), `220px` (Sidebar width), `flex-basis: 220px` (LifecycleIndex filter input), `minmax(320px, 1fr)` (lifecycle card grid), `min-width: 16rem` (Kanban column). Per the work item Requirements, these are not introduced as new tokens in 0033 ("Where no clean mapping exists, document the literal in the PR description per AC6"). The implementer should list every layout literal in the AC4 escape-hatch list as "intentional layout literal, no spacing-scale equivalent".

**Typography declarations** (every inline `font-family:` and `font-size:` must map to the new scale per Requirements bullet 6):
- The only `font-family:` that sets the UI body face is `RootLayout.module.css:1` — currently `system-ui, sans-serif`. Replace with `var(--ac-font-body)` (Inter).
- `font-family: monospace` appears in five places (slugs, IDs, paths): `WorkItemCard.module.css:23-24`, `LifecycleClusterView.module.css:25`, `LifecycleIndex.module.css:94`, `LibraryDocView.module.css:15`, `LibraryTypeView.module.css:31`. Replace with `var(--ac-font-mono)` (Fira Code).
- Display family (Sora, `var(--ac-font-display)`) — needs explicit application sites. The prototype uses Sora on `<h1>`, page titles, and chrome typographic emphasis. Strong candidates in current code: `KanbanBoard.module.css:8-12` (`.title` 1.5rem), `LibraryDocView.module.css:9` (`.title` 1.6rem), `LibraryTemplatesView.module.css:2` (`.title` 1.5rem), `LifecycleClusterView.module.css:24` (`.title` 1.4rem), `LifecycleIndex.module.css:93` (`.cardTitle` 1rem 600), `MarkdownRenderer.module.css:9-11` (`h1` 1.75rem, `h2` 1.35rem, `h3` 1.1rem). **The work item gives no explicit "Sora applies to X, Inter applies to Y" mapping** — the only AC2 floor is "each family is referenced from at least one component or global stylesheet via a typography token". Pragmatic interpretation: Sora on every `.title` / `<h1>` / `<h2>`, Inter as body default via `RootLayout`, Fira Code on the existing five monospace sites. Document the choice in the PR description.
- Font sizes: 15+ distinct rem values are in use (`0.65rem` through `1.75rem`). Mapping to the eleven-step scale (`--size-xxs: 12px` through `--size-hero: 68px`) requires bucketing. Approximate map: `0.65rem`/`0.7rem` → `--size-xxs` (12px); `0.75rem` → `--size-xxs`; `0.8rem`/`0.85rem`/`0.875rem` → `--size-xs` (14px); `0.95rem`/`1rem` → `--size-sm` (16px); `1.1rem` → `--size-md` (18px); `1.25rem`/`1.35rem` → `--size-lg` (22px) or `--size-h4` (26px); `1.4rem`/`1.5rem`/`1.6rem` → `--size-h4` or `--size-h3` (28px); `1.75rem` → `--size-h3`. Some 14px → 16px shifts (e.g., `0.875rem` → `--size-sm`) cause a ~2px size bump; AC6 covers this under the ΔE / ±2px / ±1px tolerance for spacing — but AC6 doesn't explicitly cover *typography* drift. Worth listing typographic shifts in the PR description's exceptions for clarity.

**Box-shadow declarations** — exactly **three** in the entire codebase (`rg 'box-shadow:' src/ -g '*.css'`):
- `LifecycleIndex.module.css:79` — `box-shadow: 0 1px 4px rgba(29, 78, 216, 0.12);` on `.card:hover` / `.card:focus-within`. Accent-tinted soft elevation. **Closest token: `--ac-shadow-soft`** in light theme (`0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)`) — but the prototype's value is neutral, not accent-tinted. Options: (a) replace with `--ac-shadow-soft` (loses the accent tint, but AC6 / ΔE tolerates it); (b) replace with `--shadow-crisp` (`0 1px 2px rgba(10,17,27,0.06), 0 4px 12px rgba(10,17,27,0.04)`) for closer geometry; (c) introduce a new `--shadow-accent-soft` token. **The cleanest 0033 interpretation is (a) or (b); accepting the accent-tint loss is a colour drift bounded by AC6.**
- `LifecycleClusterView.module.css:50` — `box-shadow: 0 0 0 1.5px #1d4ed8;` on `.stage::before` (active stage dot halo). **Geometric ring, not an elevation shadow.** No mapping to `--shadow-card*` / `--ac-shadow-*`. Options: (a) keep inline as a 1.5px irreducible hairline geometry, replacing `#1d4ed8` with `var(--ac-accent)`; (b) introduce a `--ring-accent` token. **Option (a) is consistent with the work item's irreducible-hairline escape-hatch.**
- `LifecycleClusterView.module.css:55` — `box-shadow: 0 0 0 1.5px #d1d5db;` on `.absent::before`. Same shape as above; replace `#d1d5db` with `var(--ac-stroke)` and keep the ring inline.

(Line 73 of `LifecycleIndex.module.css` references `box-shadow` only inside a `transition:` shorthand — not a value to migrate.)

### AC3 / AC4 grep glob anchoring (latent issue)

Both AC3 and AC4 use exclusion globs of the form `-g '!src/styles/global.css'` and `-g '!src/styles/tokens.ts'`. ripgrep evaluates these globs **relative to the cwd at invocation**, not relative to the path argument. Running the AC3 command from `workspaces/visualisation-system/` with the path argument `skills/visualisation/visualise/frontend/src/` does *not* exclude `skills/visualisation/visualise/frontend/src/styles/global.css` — it excludes only a path literally matching `src/styles/global.css` from cwd. The verified baseline above confirms this: both `src/styles/global.css:8` and `src/styles/tokens.ts:8` leak into the match set when the command is run from the workspace root.

**Workarounds**:
- Run AC3/AC4 commands from `skills/visualisation/visualise/frontend/` (frontend root). The exclusion globs then match.
- Or rewrite the exclusions as `-g '!**/styles/global.css' -g '!**/styles/tokens.ts'` (path-anywhere globs).

The implementer should agree which form is canonical and either run from the frontend root or update the AC commands. This is testability detail — not a functional blocker — but it will trip a CI-style invocation.

### Web-font wiring (AC2)

`index.html` currently has no `<head>` content beyond charset/viewport/title. Per Requirements bullet 1 and AC2, three new `<link>` elements go in `<head>`:

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Sora:wght@400;600;700&family=Inter:wght@400;500;600;700&family=Fira+Code:wght@400;500&display=swap">
```

(Exact `wght@` set is implementation choice — the prototype inventory doesn't enumerate which weights are used.)

AC2 currently asserts:
1. The fonts return 200 from `fonts.googleapis.com` / `fonts.gstatic.com` at app start, verifiable in DevTools Network.
2. Each family is referenced from at least one component or global stylesheet via a typography token.

(1) is a manual observation; not automatable in unit tests. (2) is trivially satisfied by a single `var(--ac-font-display)` reference. The pass-2 review (`meta/reviews/work/0033-design-token-system-review-1.md` line 286) flags this as a precision-tightening concern — the implementer might consider a `globalCss` regex assertion that `--ac-font-display`, `--ac-font-body`, `--ac-font-mono` are referenced from at least one `*.module.css`, but this isn't strictly required by AC2 as written.

CSP / preconnect / privacy implications captured in Dependencies (`meta/work/0033-design-token-system.md:165-168`). The pass-2 review (line 296) notes that GDPR / DPA posture for Google Fonts isn't called out alongside CSP — worth raising during PR review if the visualiser is ever deployed beyond local development.

### Existing tests that don't change

- `src/styles/global.test.ts:4-18` — three regex assertions against `globalCss?raw` for `:focus-visible` outline, outline-offset, and the `forced-colors` override. **These don't depend on token internals — all three should still pass after the migration**, provided the new `:root` block keeps the `:focus-visible` rules near the top of the file (or the regex is amended to not require `:focus-visible` to follow `:root` in any particular order, which it doesn't today either).
- `src/styles/contrast-helper.test.ts` — pure-function tests for `contrastRatio()` against WebAIM reference values. Untouched.
- `src/styles/contrast.test.ts:20-35` — three WCAG AA contrast assertions. These will need new equivalents for the `--ac-*` palette: at minimum `--ac-fg` on `--ac-bg`, `--ac-fg-muted` on `--ac-bg`, `--ac-fg` on `--ac-bg-card` for both light and dark themes (so the dark `--ac-fg: #E7E9F2` on `--ac-bg: #0A111B` passes 4.5:1, etc.). The existing three assertions stay relevant only if `--color-muted-text`, `--color-warning-text`, `--color-warning-bg`, `--color-focus-ring` survive — and the work item's bullet 4 (Requirements) explicitly excludes the legacy `--fg-*` / `--bg-*` aliases from scope, which suggests the eight existing `--color-*` tokens are being deprecated in favour of `--ac-*`. **Open: whether the eight `--color-*` tokens are kept (with the six `var(--color-*)` fallback consumers retained) or retired (with the six sites migrated to `--ac-*` equivalents).** AC1 is silent on this. Recommend: retire `--color-*`, migrate the six consumers as part of the same pass, retire `COLOR_TOKENS` from `tokens.ts`, and replace `contrast.test.ts`'s three AA assertions with `--ac-*`-based equivalents.

### Component CSS consumer wiring

CSS Modules are wired by Vite's default `*.module.css` convention (no PostCSS, no Tailwind, no vanilla-extract — confirmed: `vite.config.ts` has only `@vitejs/plugin-react`, `package.json` has no Tailwind/PostCSS). Each `.module.css` is paired with the colocated `.tsx` that imports it (e.g., `RootLayout.tsx` imports `RootLayout.module.css`). This means the migration is purely string-level inside the 16 module files; no TSX file needs to change for the colour/spacing/typography migration itself. (TSX changes will arrive in 0034+ for the toggle UI, the new Topbar, etc.)

### Test fixtures with intentional hex literals

Per Technical Notes line 215-219, three test files contain hex literals as inputs:
- `src/styles/contrast.test.ts:21-35` — `'#ffffff'` literals as background-colour arguments.
- `src/styles/contrast-helper.test.ts:6-23` — `'#000000'`, `'#ffffff'`, `'#777777'`, `'#ff0000'` as test inputs.
- `src/routes/kanban/WorkItemCard.test.tsx` — exists; the work item flags it as containing hex literals (the codebase analyser didn't read it; worth a spot-check during implementation).

AC3's `-g '!**/*.test.ts' -g '!**/*.test.tsx'` correctly excludes all three. **AC4 also has these exclusions per the corrected (pass-2) version of the work item.** The implementer should still re-grep both ACs after the migration to confirm zero matches outside the documented exception list.

## Code References

- `skills/visualisation/visualise/frontend/src/styles/global.css:1-22` — current `:root` token block plus `:focus-visible` rules; the file 0033 extends.
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:1-12` — current `COLOR_TOKENS` frozen object; the TS source-of-truth that 0033 extends.
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:1-18` — focus-ring regex tests (do not change).
- `skills/visualisation/visualise/frontend/src/styles/contrast.test.ts:6-18` — `readCssVar` regex extractor and the `Object.entries(...)` parity loop. **The pattern that 0033 extends.**
- `skills/visualisation/visualise/frontend/src/styles/contrast.test.ts:20-35` — three WCAG AA contrast assertions; will likely need to be replaced with `--ac-*` equivalents.
- `skills/visualisation/visualise/frontend/src/styles/contrast.ts:18-24` — `contrastRatio(fg, bg)` helper (untouched by 0033).
- `skills/visualisation/visualise/frontend/src/styles/wiki-links.global.css:11,18` — the only hex literals in a global stylesheet (`#9ca3af`, `#6b7280`); migrate to `--ac-fg-faint`, `--ac-fg-muted`.
- `skills/visualisation/visualise/frontend/src/main.tsx:7-9` — the global stylesheet import order (vendor → global.css → wiki-links.global.css). Likely unchanged by 0033.
- `skills/visualisation/visualise/frontend/index.html:3-7` — the `<head>` block where Google Fonts `<link>` tags belong.
- `skills/visualisation/visualise/frontend/vite.config.ts:38-56` — Vite + Vitest config; `?raw` imports work via the shared config.
- `skills/visualisation/visualise/frontend/package.json:6-31` — Vite 6, Vitest 3, React 19, no Tailwind/PostCSS.
- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.module.css:6,7,19` — half of the existing `var(--*)` consumer surface; preserves the fallback convention.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css:24-26` — the other half of the existing `var(--*)` consumer surface.
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css:1` — the only inline UI body face (`system-ui, sans-serif`); single point of replacement to `var(--ac-font-body)`.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.module.css:50,55` — the two coloured-ring `box-shadow` rules that don't map to elevation shadows.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.module.css:79` — the only true elevation shadow; closest match is `--ac-shadow-soft` or `--shadow-crisp`.

## Architecture Insights

- **Single source of truth, two surfaces.** The current pattern declares colour tokens in CSS (`:root`) and mirrors them in TypeScript (`COLOR_TOKENS`), with a parity test ensuring they don't drift. 0033 extends this verbatim — adding `TYPOGRAPHY_TOKENS`, `SPACING_TOKENS`, `RADIUS_TOKENS`, `SHADOW_TOKENS` (with light and dark colour partitions) — but the `readCssVar` regex needs to be scoped to the right CSS block when asserting dark overrides. Recommendation: split colour tokens into `LIGHT_COLOR_TOKENS` and `DARK_COLOR_TOKENS` and add a `readCssVar(name, scope)` overload that captures the relevant `[data-theme="dark"] { ... }` block first.
- **CSS Modules colocated with their consumer.** No central component library file. The migration is strictly per-`.module.css` plus `wiki-links.global.css` plus `index.html` — the TSX files are untouched. This makes 0033 a CSS-only diff with a small TS diff in `tokens.ts` plus a small test-file diff in `global.test.ts` (and possibly `contrast.test.ts`).
- **Defensive `var(--token, fallback)` convention.** Existing sites use the two-arg form. The work item is silent on whether to retain or retire it. Retaining it makes AC3 (zero hex literals) tautologically failing because every fallback hex would re-enter the match set. Retiring it is cleaner but breaks the cascade-fail safety net. **The cleanest read is to drop the fallbacks for new sites and migrate the six existing sites at the same time.**
- **Vite-native `?raw` imports for test-time parity.** Vitest reuses the Vite config (`defineConfig` from `vitest/config`), so `import css from './x.css?raw'` works without extra plugin wiring. Confirmed Vite 6 + Vitest 3 in `package.json`.
- **`wiki-links.global.css` is global on purpose** — the remark plugin emits class names as literal kebab-case strings that CSS-module hashing would break. The header comment makes this explicit. 0033 migrates the two literals inside it without changing the global-on-purpose policy.
- **Token authoring follows the prototype's *active layer* convention, not the brand palette.** Per AC1 in-scope subsections, `--atomic-*` brand colours and `--fg-*` / `--bg-*` legacy aliases are explicitly **out of scope** for 0033. Only the active `--ac-*` semantic surface tokens (light + dark), typography, spacing, radius, and shadow scales are authored. This keeps the introduction tight and means the legacy `--color-*` tokens probably get retired (open question above).
- **AC5's positive-coverage threshold has no derivation but is well-calibrated.** 300 token references across 16 modules averages ~19 per module. Worst offenders (LifecycleClusterView with 38 px/rem + 23 hex literals = ~61 sites) will easily exceed that on their own; lightest modules (RootLayout with 2 px/rem + 0 hex) will contribute 1–2. The threshold guards against deletion-only migrations and is hit naturally by a migration that does *not* aggressively use the AC4 escape-hatch. A migration that skips the typography migration entirely (~50 inline `font-size:` declarations) would struggle to reach 300; that's the threshold's signal.

## Historical Context

- `meta/work/0033-design-token-system.md` — the work item itself (exhaustively reviewed in two passes per `meta/reviews/work/0033-design-token-system-review-1.md`).
- `meta/reviews/work/0033-design-token-system-review-1.md` — pass-2 review notes that all 10 pass-1 majors are resolved or partially resolved; remaining concerns are precision-tightening (AC1 token enumeration locality, AC2 manual observation, AC6 measurement tooling). Verdict REVISE per a strict major-count threshold but the reviewer notes the work item is "implementable as-is".
- `meta/work/0034-theme-and-font-mode-toggles.md` — the immediate successor; consumes 0033's `[data-theme="dark"]` block to wire the toggle UI, font-mode swap, and `localStorage` persistence. Confirms the scope split: 0033 ships dark *values*, 0034 ships the *toggle*.
- `meta/work/0035-topbar-component.md` through `0042-templates-view-redesign.md` — eight downstream work items that all consume the token layer. None block 0033.
- `meta/work/0043-spike-detail-screen-capability-retention.md`, `0044-spike-list-screen-scope-decisions.md` — two spikes that resolve TBD items in the gap analysis (per-doc-detail screen retention; lifecycle-index sort-control reduction). Out of scope for 0033 but implementer may want to skim if any cross-cutting changes show up.
- `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md` — the canonical token table (Requirements bullet 4 and AC1 reference its subsections by name). The implementer should keep this open as a side-by-side reference while authoring `tokens.ts` and `global.css`.
- `meta/design-inventories/2026-05-06-135214-current-app/inventory.md` — the snapshot of the current app's token surface (eight `--color-*`, no scales). Useful for confirming what's being replaced.
- `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` — the gap analysis whose "Suggested Sequencing" section identifies 0033 as the load-bearing first pass.
- `meta/research/2026-05-02-design-convergence-workflow.md` and `meta/plans/2026-05-03-design-convergence-workflow.md` — research and plan for the design-convergence workflow skill that produced the inventories and gap analysis. Not directly relevant to implementation but explains how the source artefacts were generated.
- `meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md` — the originating plan that introduced `src/styles/global.css` and `src/styles/tokens.ts`. Provides historical context for *why* the eight `--color-*` tokens exist today.
- No ADR covers design tokens, theming, or typography — none of `meta/decisions/ADR-0001..0025` touches CSS architecture. ADR-0024 (Configurable Kanban Column Set) is the closest visualiser-frontend ADR. **0033 may warrant a follow-up ADR ratifying the layered token system as the canonical visualiser frontend convention.**
- No notes mention design tokens, theming, or visual redesign.

## Open Questions

These are the implementation-level decisions the work item leaves open, ordered by impact on AC outcome:

1. **Retire `--color-*` and `COLOR_TOKENS`?** AC1's in-scope set explicitly excludes `--fg-*` / `--bg-*` legacy aliases. The eight current `--color-*` tokens aren't in the prototype's `--ac-*` set either — they're a completely separate generation. Retiring them simplifies the token surface (one canonical `--ac-*` system, no parallel legacy block) but requires migrating the six existing `var(--color-*)` consumer sites in `SidebarFooter.module.css` and `LibraryDocView.module.css` and replacing the three contrast assertions in `contrast.test.ts`. Recommended decision: **retire `--color-*` as part of 0033**; the legacy block would otherwise live indefinitely as zombie code.
2. **Drop the `var(--token, #fallback)` two-arg form for new sites?** Retaining it would make AC3 fail tautologically (every fallback hex re-enters the match set). Recommended decision: **drop fallbacks for all new sites, migrate the six existing sites to no-fallback form** in the same PR, document the convention change in the PR description.
3. **Sora vs Inter assignment per surface.** AC2 only requires "each family is referenced from at least one component or global stylesheet via a typography token". The migration needs an explicit policy: which titles get Sora, which get Inter. Recommended: Sora on every `.title`, `<h1>`, `<h2>`, `.cardTitle`, `.columnHeading`; Inter as the body default in `RootLayout`; Fira Code on the existing five monospace sites. Document in the PR.
4. **Error-state colour mapping.** Three error literals (`#fef2f2`, `#fecaca`, `#fee2e2`) recur across modules with no clean `--ac-*` equivalent. Recommended: compose them via `color-mix(in srgb, var(--ac-err) 8%, white)` (light) / `color-mix(in srgb, var(--ac-err) 12%, black)` (dark) so they swap with theme; do not introduce new tokens; do not list as AC4 exceptions (because they vanish under `color-mix()`). If `color-mix()` browser support is a concern, an alternative is to introduce `--ac-err-tint` and `--ac-err-stroke` and accept the small palette extension.
5. **Two-blue collapse** (`#2563eb` + `#1d4ed8` → `--ac-accent` `#595FC8`). Recommended: collapse to a single `--ac-accent`, accept the visible hue shift bounded by AC6 ΔE < 5, document in PR.
6. **Off-scale spacing rounding policy** (`0.4rem`, `0.55rem`, `0.6rem`, `0.7rem`, `0.85rem`, `1.4rem`, `1.75rem` and similar). Recommended: separate font-size literals (migrate to typography scale), spacing literals (round to nearest `--sp-N` accepting ±2px drift), and irreducible literals (1–3px hairlines, 0 resets) per AC4 escape-hatch. Document any non-trivial roundings in the PR description.
7. **`box-shadow` mapping** for `LifecycleIndex.module.css:79` (accent-tinted soft elevation) — `--ac-shadow-soft` or `--shadow-crisp`? Recommended: `--ac-shadow-soft` (theme-swapping). The two coloured-ring shadows in `LifecycleClusterView` stay inline with `var(--ac-accent)` / `var(--ac-stroke)` colour replacement.
8. **AC3/AC4 grep glob anchoring.** Run from frontend root, or rewrite globs as `-g '!**/styles/global.css'`. Recommended: rewrite globs in the work item itself for unambiguous CI reproducibility.
9. **Where to place the parity-test extension.** The work item points to `global.test.ts`, but the existing parity loop lives in `contrast.test.ts`. Recommended: move the existing parity describe into `global.test.ts` (matching the work item's pointer) and add new describes for typography / spacing / radius / shadow alongside it; leave only the WCAG AA contrast assertions in `contrast.test.ts`.

These are not blockers — every recommendation above is a sensible default that the implementer can encode in the PR description. None require returning to the work item for revision before starting. The pass-2 review's residual majors (token enumeration locality, manual AC2, AC6 measurement tooling) are precision-tightening concerns the implementer can address inside the PR description rather than in upstream edits.

## Related Research

- `meta/research/2026-05-02-design-convergence-workflow.md` — methodology behind the inventories that source 0033's token list.
- `meta/research/2026-04-17-meta-visualiser-implementation-context.md` — early frontend scaffolding decisions (predates the design-token initiative; provides context only).
- `meta/research/2026-05-06-design-skill-localhost-and-mcp-issues.md` — concurrent in-flight design-skill plumbing work; not directly related to 0033 but worth being aware of if both branches land in the same period.
