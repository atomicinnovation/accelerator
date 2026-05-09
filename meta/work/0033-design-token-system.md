---
work_item_id: "0033"
title: "Design Token System"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: done
priority: high
parent: ""
tags: [ design, frontend, tokens, foundation ]
---

# 0033: Design Token System

**Type**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Introduce the prototype's layered token system — colour (light and dark
layers), typography, spacing/radius, and shadow tokens — as a single
foundational pass across `src/styles/`, then replace inline values throughout
component CSS modules and global stylesheets so every surface consumes named
tokens rather than hard-coded hex/px/rem literals.

## Context

The current app exposes only eight named CSS custom properties for colour and
relies on roughly fourteen hard-coded hex values scattered across CSS modules.
It has no formal typography scale (body/chrome use `system-ui`, code uses
`monospace`, sizes are set ad hoc per component), no spacing or radius scales (
each module hard-codes `px` and `rem` values inline; recurring radii `2px`/
`3px`/`4px`/`6px`/`8px`/`9999px` are repeated without naming), and only one
shadow rule (`box-shadow: 0 1px 4px rgba(29, 78, 216, 0.12)` on lifecycle card
hover).

The prototype defines a layered token system: a brand palette (`--atomic-*`),
legacy semantic aliases (`--fg-*`, `--bg-*`, `--accent`, `--stroke`), an active
semantic surface layer (`--ac-*`), an eleven-step typography size scale (
`--size-xxs: 12px` through `--size-hero: 68px`), four line-height tokens (
`--lh-tight 1.05` through `--lh-loose 1.6`), a `--tracking-caps: 0.12em` token,
an eleven-step spacing scale (`--sp-1: 4px` through `--sp-11: 124px`), a
four-step radius scale (`--radius-sm` through `--radius-pill`), and five
elevation tokens (`--shadow-card`, `--shadow-card-lg`, `--shadow-crisp`, plus
per-theme `--ac-shadow-soft` and `--ac-shadow-lift`).

The "Suggested Sequencing" section of the gap analysis identifies this token
pass as the load-bearing prerequisite for every subsequent component-level
redesign — until the tokens exist, no component re-skin can land without
immediately diverging from the target.

Reference screenshots:
`meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
`main-dark.png` (token system in both theme states).

## Requirements

Umbrella rule: every token enumerated in the bullets below is defined in
both `src/styles/global.css` (as a CSS custom property under `:root`, with
dark colour values mirrored under `[data-theme="dark"]`) and
`src/styles/tokens.ts` (as a TypeScript export); `src/styles/global.test.ts`
is extended to assert CSS↔TS parity over the full set.

- Add the three-family typography stack (`Sora` display, `Inter` body,
  `Fira Code` mono) plus the eleven-step size scale, four line-height tokens,
  and `--tracking-caps`.
  Load the three web fonts via Google Fonts (`<link rel="preconnect">` for
  `fonts.googleapis.com` / `fonts.gstatic.com` plus the corresponding stylesheet
  `<link>` in `index.html`). Raleway is deliberately excluded — the prototype
  inventory's Crawl Notes flag it as unreferenced by any `--ac-*` token, and
  Sora covers the support role it was originally slated for.
- Introduce the eleven-step spacing scale (`--sp-1: 4px` through
  `--sp-11: 124px`) and four-step radius scale (`--radius-sm: 4px`,
  `--radius-md: 8px`, `--radius-lg: 12px`, `--radius-pill: 999px`).
- Introduce the five elevation tokens (`--shadow-card`, `--shadow-card-lg`,
  `--shadow-crisp`, plus per-theme `--ac-shadow-soft` and `--ac-shadow-lift`).
- Define the complete `--ac-*` colour palette as enumerated in the prototype
  inventory's token table at
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`.
  Both layers ship in this story: the light values under `:root` and the
  full dark override block under `[data-theme="dark"]`. The user-facing
  toggle UI and `data-theme` preference persistence are deferred to 0034.
- Migrate every hard-coded hex literal in component CSS modules **and global
  stylesheets** (`src/styles/wiki-links.global.css` included) onto the named
  `--ac-*` tokens. Where no clean mapping exists, document the literal in
  the PR description per AC6 rather than introducing new tokens in this pass.
- Replace every inline `font-family` / `font-size` declaration in component
  CSS modules with token references so headings, body, and eyebrow labels
  share a single scale.
- Replace every inline spacing and radius literal in component CSS modules
  with `var(--sp-N)` and `var(--radius-*)` references, except for irreducible
  geometry (1–3px hairlines, `0` resets) which is documented in the PR
  description per AC4's escape-hatch.

## Acceptance Criteria

- [ ] (AC1) `src/styles/tokens.ts` and `src/styles/global.css` define every
  token from the following subsections of the prototype inventory's token
  table at
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
  (subsection headings as they appear in the inventory):
  - "Active semantic layer — `--ac-*` (light)"
  - "Active semantic layer — `--ac-*` (dark, overrides under `[data-theme="dark"]`)"
  - "Typography" (eleven-step size scale, four line-height tokens,
    `--tracking-caps`)
  - "Spacing" (eleven-step `--sp-N`)
  - "Radius" (four-step `--radius-*`)
  - "Shadow" (`--shadow-card`, `--shadow-card-lg`, `--shadow-crisp`,
    `--ac-shadow-soft`, `--ac-shadow-lift`)

  Subsections explicitly excluded from this story: "Brand palette —
  `--atomic-*`" (raw brand colours not consumed by components) and "Legacy
  semantic aliases — `--fg-*` / `--bg-*` / `--accent` / `--stroke`"
  (deprecated by the active `--ac-*` layer). `src/styles/global.test.ts`
  is extended to assert CSS↔TS parity over the full in-scope set.
- [ ] (AC2) Web fonts (`Sora`, `Inter`, `Fira Code`) return 200 from
  `fonts.googleapis.com` / `fonts.gstatic.com` at app start (verifiable in
  the DevTools Network panel), and each family is referenced from at least
  one component or global stylesheet via a typography token.
- [ ] (AC3) Running `rg '#[0-9a-fA-F]{3,8}\b' --type css --type ts
  skills/visualisation/visualise/frontend/src/ -g '!**/*.test.ts'
  -g '!**/*.test.tsx' -g '!src/styles/global.css' -g '!src/styles/tokens.ts'`
  returns zero matches.
- [ ] (AC4) Running `rg '\b\d+(\.\d+)?(px|rem)\b'
  skills/visualisation/visualise/frontend/src/ -g '*.css'
  -g '!src/styles/global.css' -g '!src/styles/tokens.ts'
  -g '!**/*.test.ts' -g '!**/*.test.tsx'` returns only literals listed
  under "Irreducible-literal exceptions" in the PR description (1–3px
  hairlines, `0px` / `0rem` resets, and any other inline literal whose
  token equivalent is intentionally absent). Every other px/rem literal
  has been replaced.
- [ ] (AC5) Running `rg 'var\(--' skills/visualisation/visualise/frontend/src/
  -g '*.module.css' --count-matches | awk -F: '{s+=$2} END {print s}'`
  reports at least 300 token references across all component CSS modules
  combined (current baseline: 6). This guards against deletion-only
  migrations that would pass AC3/AC4 without actually consuming the new
  tokens.
- [ ] (AC6) For each route captured in
  `meta/design-inventories/2026-05-06-135214-current-app/screenshots/`
  (kanban, library, library-type, library-decisions, templates,
  lifecycle-cluster-detail, lifecycle-cluster-after-click), the implementer
  captures a fresh screenshot at the same viewport dimensions used for the
  inventory baseline after the token migration. Each pair is reviewed
  side-by-side by the PR reviewer; differences are bounded by ΔE < 5 on
  any colour, ±2px on any spacing, and ±1px on any radius (a "clean
  equivalent" in the new scale). Any pixel-diff region exceeding 5% of the
  route's viewport area is listed in the PR description with a one-line
  justification.

## Open Questions

- None at this time. Prior open decisions resolved during the
  `/review-work-item` pass on 2026-05-06: colour-mapping fallback is
  handled via the PR-listed exception pattern in Requirements bullet 5
  and AC6; test-fixture hex/px-rem retention is locked in by AC3's and
  AC4's grep exclusions; the four-way split alternative documented in
  Drafting Notes is declined; Raleway is intentionally dropped; fonts
  load from Google Fonts (not self-hosted).

## Dependencies

- Blocked by: none — this is the foundational pass.
- Depends on (external): Google Fonts (`fonts.googleapis.com`,
  `fonts.gstatic.com`) for the three web fonts. CSP and runtime availability
  must accommodate these origins; ops/security review should be aware of the
  third-party coupling.
- Depends on (artefact): the prototype inventory at
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
  is the canonical source of token names and values; revisions to the
  inventory may invalidate tokens shipped by this story.
- Blocks: 0034 (Theming & FontMode toggle UI — immediate successor),
  0035, 0036, 0037, 0038, 0039, 0040, 0041, 0042 (component-level
  redesigns — all depend on the token layer).

## Assumptions

- The token migration is intended to be visually neutral against the *current*
  app where mappings are clean. Both the light and dark colour layers ship
  in this story (the dark override block under `[data-theme="dark"]`); only
  the user-facing toggle UI and `data-theme` preference persistence are
  delivered separately by 0034.

## Technical Notes

- Frontend root is `skills/visualisation/visualise/frontend/`; all `src/...`
  paths in Requirements resolve under that directory, not the workspace root.
- Existing token surface is colour-only and minimal:
  `src/styles/global.css:1-10`
  defines eight `--color-*` custom properties; `src/styles/tokens.ts:1-10`
  mirrors them as a frozen `COLOR_TOKENS` object. `src/styles/global.test.ts`
  asserts CSS↔TS parity — extending tokens requires extending this test, not
  bypassing it.
- The prototype has no source code in this repo. Only the runtime-derived
  inventory at
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
  (see `inventory.md:22-23`) and screenshots exist. Token values must be
  reconstructed from the inventory's tables, not copied from a prototype
  `tokens.ts` / `global.css`.
- Web fonts: `index.html` currently has no `<link>` tags for fonts; new
  `<link rel="preconnect">` and Google Fonts `<link>` tags belong inside the
  existing `<head>`. Global CSS imports are wired at `src/main.tsx:7-9`.
- Component CSS modules consume essentially zero tokens today (6 `var(--*)`
  references across 17 modules; the rest are inline literals). Worst
  offenders by literal count:
  `src/routes/lifecycle/LifecycleClusterView.module.css` (23 hex, 38 px/rem),
  `src/routes/lifecycle/LifecycleIndex.module.css` (22 hex, 32 px/rem),
  `src/routes/kanban/KanbanBoard.module.css` (15 hex, 20 px/rem). Total
  across `src/`: 168 hex literals across 21 files; 223 px/rem literals
  across 18 files.
- A second global stylesheet, `src/styles/wiki-links.global.css`, also
  hard-codes hex (`#9ca3af` line 11, `#6b7280` line 18) — include in the
  migration.
- Test fixtures (`src/styles/contrast-helper.test.ts`,
  `src/styles/contrast.test.ts`,
  `src/routes/kanban/WorkItemCard.test.tsx`) contain hex literals as
  inputs; AC3 and AC4 explicitly exclude `*.test.ts` and `*.test.tsx`
  globs so those fixtures are intentionally retained as-is.
- Vite + React 19 stack (`vite.config.ts`); CSS Modules wired by Vite's
  default `*.module.css` convention. No Tailwind / PostCSS plugins /
  vanilla-extract — token authoring stays in plain CSS custom properties
  consumed via `var(--token-name)`.
- Per the gap analysis "Suggested Sequencing", this pass lands before any
  other redesign work (preserved from prior Technical Notes).

## Drafting Notes

- Treated colour, typography, spacing/radius, and shadow as a single coherent
  foundational story rather than four separate stories, on the basis that
  they share the same files (`src/styles/tokens.ts`, `src/styles/global.css`),
  are sequenced together by the gap analysis, and would naturally land as
  one tightly-sequenced PR series. Decision reaffirmed during the
  `/review-work-item` pass on 2026-05-06: keep as one story.
- Raleway dropped from the typography stack per the prototype inventory's
  Crawl Notes ("possibly aspirational, not referenced by any `--ac-*`
  token"); Sora covers the support role Raleway was originally slated for.
- Web fonts loaded from Google Fonts rather than self-hosted, accepting the
  third-party runtime dependency and CSP/privacy implications captured in
  Dependencies.
- Both light and dark `--ac-*` colour values ship in 0033 (the dark
  override block under `[data-theme="dark"]`); only the toggle UI and
  preference persistence are deferred to 0034.
- Acceptance criteria refined during the `/review-work-item` pass:
  AC1 enumerates against the inventory, AC4's grep glob widened to all
  CSS with test exclusions plus an irreducible-literal escape-hatch, AC5
  added as a positive-coverage check, AC6 tightened with a ΔE / pixel
  tolerance.

## References

- Source:
  `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Current inventory:
  `meta/design-inventories/2026-05-06-135214-current-app/inventory.md`
- Target inventory:
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
- Screenshots:
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
  `main-dark.png`
