---
type: codebase-research
id: "2026-07-13-docs-site-visualiser-design-alignment"
title: "Research: Can the docs website align with the visualiser design system?"
date: "2026-07-13T08:32:46+00:00"
author: "Phil Helm"
producer: research-codebase
status: complete
topic: "Aligning the docs website with the visualiser design system"
tags: [research, codebase, docs-site, starlight, design-tokens, visualiser, theming]
revision: "94dc5abbbce25f61c38d50d8ce90a49d40736ab9"
repository: "barcelona"
last_updated: "2026-07-13T08:45:00+00:00"
last_updated_by: "Phil Helm"
last_updated_note: "Resolved all open questions with user decisions"
schema_version: 1
---

# Research: Can the docs website align with the visualiser design system?

**Date**: 2026-07-13T08:32:46+00:00
**Author**: Phil Helm
**Git Commit**: 94dc5abbbce25f61c38d50d8ce90a49d40736ab9
**Branch**: docs/0179-docs-polish
**Repository**: barcelona

## Research Question

Can we make the docs website align with the visualiser design system?

## Summary

Yes — and the mechanics are unusually favourable. The docs site
(`docs-site/`, Astro + Starlight 0.40) currently ships **stock Starlight
theming**: its only custom CSS is a 13-line helper file with no colour,
font, or token overrides. The visualiser frontend has a mature,
ADR-governed design system (theme-invariant `--atomic-*` brand palette +
semantic `--ac-*` tokens, Sora/Inter/Fira Code, spacing/radius/shadow
ladders) in a single token sheet.

Starlight is designed for exactly this kind of retheme: every visual
decision is a `--sl-*` custom property inside `starlight.*` cascade
layers, and user `customCss` is injected first and unlayered, so plain
`:root` re-declarations win. Alignment is therefore mostly a **token
mapping exercise** (`--atomic-*`/`--ac-*` values → `--sl-*` properties)
plus self-hosting the three font families. No prior decision blocks
this — 0177 explicitly deferred theming and the active 0179 plan covers
config polish only — so it is **new, additive scope**, and per
ADR-0026/0035 conventions it should get its own supplementary ADR and a
drift guard for any duplicated token values.

## Detailed Findings

### Docs site: current styling surface

- `docs-site/` is Astro ^6.4.5 + `@astrojs/starlight` ^0.40.0
  (`docs-site/package.json`), deployed to GitHub Pages under base
  `/accelerator` (`docs-site/astro.config.mjs`).
- The only custom CSS is `docs-site/src/styles/custom.css` (13 lines):
  `.dark-only`/`.light-only` image visibility keyed off `data-theme`,
  and a `.centred-image` helper. Wired via `customCss` at
  `docs-site/astro.config.mjs:24`.
- No `--sl-*` overrides, no fonts, no component overrides exist. Logos
  (light/dark PNGs, `replacesTitle: true`) at `astro.config.mjs:31-35`;
  plugins are `starlightImageZoom` and `starlightLinksValidator`
  (`astro.config.mjs:61-64`).
- Build tasks: `tasks/docs.py` (`docs:build` with strict link
  validation, `docs:serve`, `docs:generate` for per-skill reference
  pages); CI publishes `docs-site/dist`
  (`.github/workflows/main.yml` ~line 398).

### Visualiser design system (source of truth)

All in `skills/visualisation/visualise/frontend/src/styles/global.css`:

- **Brand layer `--atomic-*`** (theme-invariant, `global.css:256-292`):
  night rgb(14,15,25), night-2 rgb(10,17,27), ink rgb(32,34,49), red
  rgb(203,70,71), indigo rgb(89,95,200), indigo-tint rgb(193,197,255),
  medium-purple #965DD9, cream-can #F5C25F, pastel-green #6BE58B,
  aquamarine #73E4E2, malibu #72CBF5, marigold #F9DE6F, white, bone
  rgb(251,252,254), slate rgb(95,99,120), geyser #D3DBE0. Mirrored to
  `BRAND_COLOR_TOKENS` in `tokens.ts` and drift-tested against
  `src/styles/fixtures/prototype-tokens.json`.
- **Semantic layer `--ac-*`**: light at `global.css:80-102` (`--ac-bg`
  = bone, `--ac-fg` #14161f, `--ac-accent` = indigo, `--ac-accent-2` =
  red, ink-based rgba strokes); dark at `global.css:354-421` (`--ac-bg`
  = night-2, `--ac-bg-card` #131524, `--ac-fg` #e7e9f2, `--ac-accent`
  #8a90e8, `--ac-err` #e86a6b).
- **Typography** (`global.css:12-74, 157-202`): self-hosted woff2 —
  Sora 600/700 (display), Inter 300–700 (body), Fira Code 400/500
  (mono), served from `/fonts/`; size ladder `--size-<px×10>`
  (ADR-0043), line heights `--lh-tight`…`--lh-prose`,
  `--tracking-caps` 0.12em.
- **Spacing/radius/shadow**: `--sp-1..11` (4→124px,
  `global.css:205-215`); `--radius-0..12` + pill/full (ADR-0039,
  `global.css:223-232`); `--shadow-card`, `--ac-shadow-soft/lift`.
- **Code surface, theme-invariant** (`global.css:299-344`): `--code-bg`
  #0e1320, `--code-fg` #d7dcec, plus 27 `--tk-*` syntax tokens (string
  #6be58b, keyword #c1c5ff, number #f9de6f) mapped to hljs classes by
  `code-syntax.global.css`.
- **Theming mechanism**: `[data-theme="dark"]` canonical block +
  byte-equivalent `prefers-color-scheme` mirror (parity asserted in
  `global.test.ts`); runtime hook `src/api/use-theme.ts` with FOUC
  guard in `storage-keys.ts:19-21`.

### Starlight's customisation surface (how alignment would land)

From `docs-site/node_modules/@astrojs/starlight/style/props.css`:

- **Everything is a CSS custom property in `@layer starlight.base`**,
  dark by default on `:root` with light overrides on
  `:root[data-theme='light']` — note this is the *opposite* default to
  most sites but matches a `data-theme` attribute model very like the
  visualiser's.
- **Key variables to map**: accent triple `--sl-color-accent-low /
  -accent / -accent-high`; gray scale `--sl-color-white`,
  `--sl-color-gray-1..6(+7 in light)`, `--sl-color-black`; semantic
  `--sl-color-text`, `--sl-color-bg`, `--sl-color-bg-nav`,
  `--sl-color-bg-sidebar`, `--sl-color-bg-inline-code`,
  `--sl-color-hairline*`; status hue triples; text scale
  `--sl-text-*`; layout `--sl-content-width`, `--sl-nav-height`, etc.
- **Fonts**: set `--sl-font` and `--sl-font-mono` (unset by default;
  consumed via `--__sl-font` indirection, `props.css:97-98`,
  `reset.css:24,66`) and load font files through `customCss`.
- **Override precedence**: user `customCss` is imported first and
  unlayered (`components/Page.astro:3`), so plain `:root` /
  `[data-theme='light']` declarations in `custom.css` beat all
  Starlight styles — no `!important`, no forking.
- **Component overrides**: 26 overridable `.astro` slots via the
  `components:` config (`utils/user-config.ts:205`) — e.g. `Header`,
  `Sidebar`, `PageTitle`, `ThemeSelect` — available if token overrides
  alone aren't enough, and an override can wrap the default component.

### Proposed mapping sketch (illustrative)

| Starlight | Light | Dark |
|---|---|---|
| `--sl-color-accent` | `--atomic-indigo` rgb(89,95,200) | #8a90e8 (visualiser dark accent) |
| `--sl-color-bg` | bone rgb(251,252,254) | night-2 rgb(10,17,27) |
| `--sl-color-bg-sidebar` | #f7f8fb | card-family #131524 |
| `--sl-color-text` | #14161f | #e7e9f2 |
| `--sl-color-hairline*` | ink rgba 0.06/0.10/0.18 | white rgba equivalents |
| `--sl-font` | Inter | Inter |
| `--sl-font-mono` | Fira Code | Fira Code |

Headings could adopt Sora via a small unlayered rule; code blocks are
rendered by Shiki (`astro.config.mjs` markdown pipeline), so matching
the visualiser's always-dark `--code-bg #0e1320` + `--tk-*` palette
would mean either a custom Shiki theme or CSS-variable-based Shiki
theming — the one area needing more than variable overrides.

### Governance and prior decisions

- **ADR-0056** (Starlight for docs site) — no styling constraints.
- **ADR-0026 + ADR-0035** — token application conventions and the
  brand-layer indirection rule: exact-hex matches must reference
  `--atomic-*`; tints via `color-mix` at locked 8/18/30%; spacing
  snapped to `--sp-*` within ±2px; new `:root`-only token families
  require a supplementary ADR. Both accepted and immutable (ADR-0031).
- **0177** explicitly listed theming as out of scope; **0179** (active
  plan, this branch) covers Starlight config polish + splash page only.
  So this work is additive scope, not a reversal.
- Repo pattern for sharing values across builds is **duplication with a
  drift guard** (cf. `prototype-tokens.json` fixture test, and the
  hand-duplicated 80-col width noted in CLAUDE.md). The docs site is a
  separate npm build from the frontend, so brand values copied into
  `custom.css` should get a drift test (e.g. compare against
  `frontend/src/styles/fixtures/prototype-tokens.json` from a Python
  or Node check in `tasks/`).

## Code References

- `docs-site/astro.config.mjs:24` — `customCss` hook point
- `docs-site/src/styles/custom.css` — current (near-empty) custom CSS
- `docs-site/node_modules/@astrojs/starlight/style/props.css` — full
  `--sl-*` token inventory (dark `:root` lines 2–121, light 123–174)
- `docs-site/node_modules/@astrojs/starlight/components/Page.astro:3` —
  user CSS imported first, unlayered → wins over theme layers
- `skills/visualisation/visualise/frontend/src/styles/global.css` —
  canonical token sheet (brand 256–292, semantic light 80–102, dark
  354–421, fonts 12–74, spacing 205–215, radius 223–232, code 299–344)
- `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json`
  — drift-guard fixture for brand values
- `tasks/docs.py` — docs build/serve/generate tasks

## Architecture Insights

- Both systems use a `data-theme` attribute with an OS-preference
  fallback, so light/dark semantics transfer cleanly; Starlight even
  ships its own theme toggle, removing any need to port `use-theme.ts`.
- Starlight's layered-CSS design means alignment needs no component
  forking for the colour/font layer — a single well-commented
  `custom.css` (or a new `theme.css` alongside it) suffices.
- The visualiser's brand/semantic two-layer indirection maps naturally
  onto Starlight: declare `--atomic-*` in docs custom CSS, then assign
  `--sl-*` from them, preserving the ADR-0035 single-source ethos.
- Fonts must be self-hosted twice (frontend `/fonts/` and
  `docs-site/public/fonts/`) or extracted to a shared location; a
  copy step in `tasks/docs.py` could keep them in sync.

## Historical Context

- `meta/decisions/ADR-0056-astro-starlight-for-documentation-site.md`
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  (+ `ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md`)
- `meta/research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md`
- `meta/research/codebase/2026-07-10-0179-make-the-docs-amazing.md` and
  `meta/plans/2026-07-10-0179-make-the-docs-amazing.md` — active docs
  polish work; theming deliberately not included
- `meta/research/codebase/2026-05-06-0033-design-token-system.md`,
  `2026-05-08-0034-theme-and-font-mode-toggles.md`,
  `2026-05-31-0077-shadow-and-dark-accent-token-audit.md`,
  `2026-05-21-0076-code-block-syntax-highlight-palette.md` — token
  system evolution

## Related Research

- `meta/research/codebase/2026-07-10-0177-documentation-site-for-docs-tree.md`
- `meta/research/codebase/2026-06-29-0175-slim-readme-split-docs-tree.md`
- `meta/research/codebase/2026-06-12-0083-dev-design-system-reference-page.md`

## Open Questions

All resolved — see Follow-up Research below.

## Follow-up Research 2026-07-13T08:45:00+00:00

The five open questions were reviewed with the user and resolved:

- **Code-block palette — full match via custom Shiki theme.** Author a
  Shiki JSON theme from the visualiser's `--code-*`/`--tk-*` palette
  (`global.css:299-344`) so docs code blocks are identical to the app
  in both modes (the code surface is theme-invariant, always dark).
- **Fidelity — colours + fonts plus component polish.** Beyond the
  `--sl-*` token mapping and typography, restyle Starlight surfaces
  (sidebar, asides, cards, hairlines, radii, shadows) to visualiser
  conventions (`--radius-4`, ink/white rgba strokes, soft shadows) via
  unlayered CSS. Stop short of `components:` slot overrides.
- **Governance — drift guard only, no ADR.** Add a test comparing
  brand hex values in docs custom CSS against
  `frontend/src/styles/fixtures/prototype-tokens.json`; skip the
  supplementary ADR for now.
- **Work item — new phase in the 0179 plan**
  (`meta/plans/2026-07-10-0179-make-the-docs-amazing.md`), continuing
  on the current `docs/0179-docs-polish` branch.
- **Fonts — trimmed set.** Self-host Inter 400/600/700, Sora 600/700,
  Fira Code 400 as woff2 in `docs-site/public/fonts/` (copied from the
  frontend's `/fonts/` assets).
