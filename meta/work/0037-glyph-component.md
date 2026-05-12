---
work_item_id: "0037"
title: "Glyph Component"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: ""
tags: [design, frontend, components]
---

# 0037: Glyph Component

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an end user scanning a list of documents, I want each doc type to have a distinctive icon and fill colour so that I can recognise the type of an item at a glance without reading every label. (The rest of this work item is written for the implementer of the `Glyph` component.)

Implement a `Glyph` React component that renders a per-doc-type icon at three sizes (16/24/32 px) with per-doc-type fill colours, for all twelve non-virtual `DocTypeKey` values surfaced by the current app, and introduce the supporting `--ac-doc-<key>` token sub-namespace. Consumer integration across Sidebar nav, page header eyebrows, kanban cards, lifecycle cards, timeline steps, activity-feed rows, search results, and template-type indicators is delivered by 0036/0040/0041/0042/0043/0053/0054/0055.

## Context

The current app has no doc-type icon system — items are distinguished by text labels alone. The redesigned prototype defines a `Glyph` component that renders a square doc-type icon at multiple sizes with per-doc-type fill colours.

Updated reference screenshots (`library-view-updated-light.png`, `library-view-updated-dark.png`) show the full twelve-doc-type set rendered in both light and dark mode and supersede the colour mapping informally listed in earlier prototype notes. The screenshots are the canonical source for icon shape and fill colour per doc type.

The prototype embeds Glyph inside nav items, page eyebrows, kanban cards, timeline steps, lifecycle cards, activity items, search results, and template-type indicators — making it a load-bearing shared component that downstream redesigns (Sidebar nav and activity feed, Library page wrapper, kanban card enrichment, lifecycle hexchain, Templates view, sidebar search) all depend on.

## Requirements

- Introduce a `GlyphDocTypeKey` type alias defined as `Exclude<DocTypeKey, "templates">` (or equivalently a literal union of the twelve non-virtual keys), co-located with the Glyph module. Glyph's `docType` prop accepts only this constrained type.
- Implement a `Glyph` React component accepting a `docType: GlyphDocTypeKey` prop and a `size` prop constrained to `16 | 24 | 32` (px). The canonical `DocTypeKey` union lives in `skills/visualisation/visualise/frontend/src/api/types.ts`.
- Provide icon assets and per-doc-type fill colours for **all twelve non-virtual `DocTypeKey` values**: `decisions`, `research`, `plans`, `plan-reviews`, `validations`, `prs`, `pr-reviews`, `notes`, `work-items`, `work-item-reviews`, `design-gaps`, `design-inventories`. The updated light/dark library-view screenshots are the source of truth for icon shape and colour.
- Introduce a per-doc-type token sub-namespace named `--ac-doc-<key>` in all three theme blocks of `skills/visualisation/visualise/frontend/src/styles/global.css` — light `:root`, the explicit `[data-theme="dark"]` block, and the `@media (prefers-color-scheme: dark)` mirror — plus the `skills/visualisation/visualise/frontend/src/styles/tokens.ts` mirror. The colour token table below is the contract; specific hex values are eyedroppered from the canonical screenshots during implementation and populated into the table as part of the change set.
- Source all Glyph fill colours from the `--ac-doc-<key>` tokens rather than hard-coded hex values, so light/dark theme swap is handled at the token level.
- Render the inner SVG with `aria-hidden="true"` by default — Glyph is decorative-redundant in every consumer site identified, where a visible text label naming the doc type sits adjacent.
- Expose an optional `accessibleLabel` prop; when supplied, the SVG is rendered with `role="img"` and `aria-label={accessibleLabel}` and `aria-hidden` is not set, supporting standalone use without an adjacent label.
- Add a `/glyph-showcase` route to the visualiser frontend that renders a grid of all twelve doc types × three sizes (16/24/32), viewable under both `data-theme="light"` and `data-theme="dark"`. The route is linked from a "Developer routes" section in `skills/visualisation/visualise/frontend/README.md` so downstream consumers can preview every Glyph variant. The showcase's scope is capped at this grid + theme view — any further features (in-page theme toggle, search/filter, doc-type metadata panel) belong in a separate work item.

### Colour Token Table

| `DocTypeKey` | Token name | Light hex | Dark hex |
|---|---|---|---|
| `decisions` | `--ac-doc-decisions` | TBD | TBD |
| `research` | `--ac-doc-research` | TBD | TBD |
| `plans` | `--ac-doc-plans` | TBD | TBD |
| `plan-reviews` | `--ac-doc-plan-reviews` | TBD | TBD |
| `validations` | `--ac-doc-validations` | TBD | TBD |
| `prs` | `--ac-doc-prs` | TBD | TBD |
| `pr-reviews` | `--ac-doc-pr-reviews` | TBD | TBD |
| `notes` | `--ac-doc-notes` | TBD | TBD |
| `work-items` | `--ac-doc-work-items` | TBD | TBD |
| `work-item-reviews` | `--ac-doc-work-item-reviews` | TBD | TBD |
| `design-gaps` | `--ac-doc-design-gaps` | TBD | TBD |
| `design-inventories` | `--ac-doc-design-inventories` | TBD | TBD |

Hex values are derived from `library-view-updated-light.png` and `library-view-updated-dark.png` during implementation. The token names are normative; the hex columns are filled in as part of this work item and become the contractual source of truth alongside the screenshots.

## Acceptance Criteria

- [ ] Given a Glyph is rendered with `docType="decisions"` at any of the three supported sizes, when it paints, then a square doc-type icon appears at exactly the requested pixel size with its `fill` resolving to `var(--ac-doc-decisions)`.
- [ ] Given Glyph is rendered at each of `size={16}`, `size={24}`, `size={32}`, when measured in the rendered DOM, then the bounding box matches the requested size and the rendered root element is an `<svg>` with `viewBox` set (no `<img>` raster fallback).
- [ ] Glyph renders correctly for every one of the twelve non-virtual `DocTypeKey` values (`decisions`, `research`, `plans`, `plan-reviews`, `validations`, `prs`, `pr-reviews`, `notes`, `work-items`, `work-item-reviews`, `design-gaps`, `design-inventories`), each with `fill="var(--ac-doc-<key>)"`.
- [ ] Given Glyph fill colours are defined via `--ac-doc-<key>` tokens, when `document.documentElement[data-theme]` is toggled between `light` and `dark`, then after one `requestAnimationFrame` yield the Glyph repaints with the corresponding dark-mode fill from the Colour Token Table without any React render occurring. The resolved-hex side is verified by Playwright (where the browser engine resolves `var()`); the no-render-occurred side is structurally guaranteed by Glyph holding no `useState`/`useEffect`/`useContext` (verified by source inspection, no render-counter spy required).
- [ ] Given Glyph is rendered without an `accessibleLabel` prop, when inspected for accessibility, then the inner SVG carries `aria-hidden="true"` and carries neither `role` nor `aria-label`.
- [ ] Given Glyph is rendered with `accessibleLabel="Decision"`, when inspected for accessibility, then the inner SVG carries `role="img"` and `aria-label="Decision"` and does not carry `aria-hidden`.
- [ ] Given a `docType` value not in the `GlyphDocTypeKey` union is passed (including the virtual `templates` key), when TypeScript compiles, then the call is rejected at the type level (no runtime fallback path is needed).
- [ ] Given a `size` value not in `16 | 24 | 32` is passed, when TypeScript compiles, then the call is rejected at the type level.
- [ ] A Vitest smoke test under `src/components/Glyph/Glyph.test.tsx` renders Glyph for every (docType, size) pair (12 × 3 = 36 combinations) and asserts, for each render: the root element is an `<svg>`; its `width` and `height` attributes match the requested `size`; its `viewBox` is set; and its `fill` attribute is exactly `var(--ac-doc-<key>)` for the requested doc type. (Resolution of the `var()` to a hex is JSDOM-unreliable for SVG presentation attributes and is therefore deferred to the Playwright visual-regression spec, which runs against a real browser engine.)
- [ ] A `/glyph-showcase` route renders a grid of all twelve doc types × three sizes (16/24/32), viewable under both `data-theme="light"` and `data-theme="dark"`, and is linked from a "Developer routes" section in `skills/visualisation/visualise/frontend/README.md`.

### Visual Contract Verification

These criteria pin the comparison procedure against the canonical screenshots so the visual contract is objectively checkable:

- [ ] **Colour eyedropper procedure**. Each hex value in the Colour Token Table is sampled from the centre pixel of the corresponding glyph in `library-view-updated-light.png` (Light hex column) and `library-view-updated-dark.png` (Dark hex column) using `magick identify -format "%[hex:p{x,y}]" file.png` (or equivalent tool). The implementing commit logs the (x, y) coordinate used per doc type in the commit message body for reproducibility.
- [ ] **Playwright visual-regression snapshot**. A spec under `tests/visual-regression/glyph-showcase.spec.ts` renders the `/glyph-showcase` route at viewport `1024×768`, captures a screenshot per theme, and compares against committed baselines at `tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/glyph-showcase-{light,dark}-{darwin,linux}.png` (both platforms checked in, per project convention). Baselines are captured from the showcase page itself rather than cropped from the canonical library-view screenshots — visual fidelity against the canonical screenshots is enforced by per-icon manual review during implementation; Playwright then locks in the resulting showcase pixels as the regression baseline. The spec passes when pixel-diff ≤ 5 % per snapshot, configured via Playwright's `toHaveScreenshot({ maxDiffPixelRatio: 0.05 })` to match the project precedent set by `tests/visual-regression/tokens.spec.ts`.

## Out of Scope / Downstream Verification

The following consumer surfaces thread Glyph through their views; integration is delivered and verified by their own work items, not by 0037:

- Sidebar redesign epic — 0036
- Kanban cards / lifecycle hexchain — 0040
- Page header eyebrows / Library page wrapper — 0041
- Templates view template-type indicators — 0042
- Detail-screen capability-retention spike — 0043
- Sidebar nav per-type indicators — 0053
- Sidebar search results — 0054
- Sidebar activity feed — 0055

## Open Questions

- Coordination with 0038 (Generic Chip Component): 0038 has an open question about exposing an icon slot. If Chip accepts an icon slot, Glyph should compose cleanly inside it; if not, no change to Glyph is needed. Resolution lives in 0038.

## Resolved Decisions

- **Icon-asset packaging** (resolved during planning, see `meta/plans/2026-05-12-0037-glyph-component.md`): per-doc-type component files under `src/components/Glyph/icons/<DocType>Icon.tsx` (12 files), each exporting the inner SVG content; Glyph owns the outer `<svg>` wrapper, `viewBox`, sizing, fill, and a11y attributes, and dispatches to the right icon component via a `Record<GlyphDocTypeKey, ()=>ReactElement>` map.
- **Playwright spec location and threshold** (resolved during planning): spec lives at `tests/visual-regression/glyph-showcase.spec.ts` (not `e2e/glyph.spec.ts`) and uses `maxDiffPixelRatio: 0.05` (not `0.005`) to match the project precedent set by `tests/visual-regression/tokens.spec.ts`. Both `-darwin.png` and `-linux.png` baselines are committed under `tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/`.
- **Vitest fill-attribute assertion shape** (resolved during planning): the smoke test asserts `getAttribute('fill') === 'var(--ac-doc-<key>)'` rather than `getComputedStyle(svg).fill` resolving to a hex, because JSDOM does not reliably substitute `var()` inside SVG presentation attributes. The resolved-hex check moves to Playwright.
- **Token organisation** (resolved during planning): the 12 new `--ac-doc-<key>` tokens are added directly to the existing `LIGHT_COLOR_TOKENS` and `DARK_COLOR_TOKENS` exports in `tokens.ts` (not into dedicated `LIGHT_DOC_COLOR_TOKENS` / `DARK_DOC_COLOR_TOKENS` exports), preserving the flat-record convention.
- **Frontend README** (resolved during planning): the README file does not exist today; this work item creates `skills/visualisation/visualise/frontend/README.md` from scratch with Overview / Development / Testing / Building / Developer Routes sections, listing `/glyph-showcase` under Developer Routes.

## Dependencies

- Blocked by: none. Builds on 0033 (token layer landed); this work item extends it with the per-doc-type `--ac-doc-<key>` colour sub-namespace.
- Blocks: 0036 (Sidebar redesign epic), 0040 (pipeline visualisation overhaul / kanban + lifecycle), 0041 (Library page wrapper and overview hub), 0042 (Templates view redesign), 0043 (detail-screen capability-retention spike), 0053 (sidebar nav per-type indicators), 0054 (sidebar search), 0055 (sidebar activity feed).
- Coordinates with: 0038 (Generic Chip Component) — Chip's icon-slot decision affects whether Glyph composes inside Chip; resolved in 0038.
- Reads from (artefact couplings):
  - Canonical `DocTypeKey` union in `skills/visualisation/visualise/frontend/src/api/types.ts` — any rename or relocation breaks `GlyphDocTypeKey`.
  - Canonical screenshots `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-{light,dark}.png` — source of truth for icon shape and per-doc-type fill colour; any edit to these assets requires re-deriving Glyph's SVG paths and hex values.

## Assumptions

- Asset creation for all twelve doc types is in scope of this work item, including the three (`work-item-reviews`, `design-gaps`, `design-inventories`) that were not enumerated in the earliest prototype notes but are now present in the updated library-view screenshots.
- The canonical `DocTypeKey` union in `skills/visualisation/visualise/frontend/src/api/types.ts` is the authoritative list; if a thirteenth doc type is added during implementation, the Glyph icon set is extended in lock-step rather than tolerated as a gap.
- Light/dark theme switching is handled by toggling a root-level class or attribute that the `--ac-*` tokens key off — Glyph does not implement its own theme detection.

## Technical Notes

**Size**: M — bounded single-component scope following the `OriginPill`/`TopbarIconButton` pattern, but two cost drivers above S: deriving twelve SVG shapes by visual inspection from the canonical screenshots (no prototype source on disk) and authoring a per-doc-type colour sub-namespace in both theme blocks of `global.css:66-205` plus the `tokens.ts:1-47` mirror. Threading Glyph through the eight consumer surfaces is deferred to 0036/0040/0041/0042/0043/0053/0054/0055.

- The prototype uses CSS classes like `.ac-glyph` with per-doc-type modifier classes; a React component wrapping that pattern is the obvious shape.
- The canonical `DocTypeKey` union and `DOC_TYPE_KEYS` runtime array live in `skills/visualisation/visualise/frontend/src/api/types.ts:4-19`. Glyph should consume the union directly rather than duplicate it.
- The current `DocType` interface (`types.ts:26-30`) carries only `key` and `label` — the per-type colour/icon mapping can live within the Glyph module without extending the shared `DocType` type, keeping the visual mapping co-located with the component that uses it.
- Glyph supports all twelve non-virtual doc-type keys, but `LIFECYCLE_PIPELINE_STEPS` (`types.ts:139-169`) only references eleven of them — `work-item-reviews` is absent from the pipeline chain. Glyph itself is unaffected; lifecycle-consumer work items (0040) should note that the chain renders 11 glyphs, not 12.
- The `--ac-*` token layer is already in place — `skills/visualisation/visualise/frontend/src/styles/global.css:66-205` defines the light/dark theme tokens (light `:root`, explicit dark `[data-theme="dark"]`, and `@media (prefers-color-scheme: dark)` mirror), with a TypeScript mirror in `skills/visualisation/visualise/frontend/src/styles/tokens.ts:1-47`. There is no per-doc-type colour sub-namespace yet; the `--ac-doc-<key>` sub-namespace introduced by this work item lives alongside the existing tokens in both theme blocks.
- Theming switch is a `data-theme` attribute set on `document.documentElement` by `useTheme()` (`skills/visualisation/visualise/frontend/src/api/use-theme.ts:26-50`). Glyph does not need to detect theme — referencing `var(--ac-*)` is sufficient and the theme swap is handled at the token level.
- SVG `fill`/`stroke` attributes accept `var(--ac-*)` directly in this codebase — see `skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx:25` (`fill="var(--ac-accent-2)"`) and `:31` (`stroke="var(--ac-accent)"`). This validates the planned approach.
- Closest comparable visual atom is `OriginPill` (`skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.tsx:1-13` + `OriginPill.module.css:1-23`) — function component, named export, CSS Modules, token-only colours, decorative inner element with `aria-hidden="true"`, and a `prefers-reduced-motion` guard. `TopbarIconButton.tsx:1-31` shows the SVG-via-children pattern with a `data-icon` CSS hook.
- File-layout convention: `src/components/Glyph/Glyph.tsx` + `Glyph.module.css` + `Glyph.test.tsx`, no `index.ts`, named export only.
- `PipelineDots.tsx:10-26` is the existing per-pipeline-step indicator and the most natural upgrade target once Glyph lands (per-step coloured glyph replaces the uniform dot).
- No Storybook/Histoire/Ladle is configured (`skills/visualisation/visualise/frontend/package.json:19-46`; no `.storybook/` or `*.stories.*` files). The showcase deliverable is therefore a `/glyph-showcase` route on the visualiser frontend rather than a stories-based artefact.
- `DocTypeKey` actually contains thirteen keys — the twelve scoped here plus virtual `templates` (`types.ts:8`, with `virtual: true` per `types.ts:35`). Glyph excludes `templates` because it is not a real doc type — see `GlyphDocTypeKey` in Requirements.

## Drafting Notes

- User-story framing chose **end user scanning a list** as the role rather than a developer consuming the component. The end-user framing matches the visible benefit (faster doc-type recognition); flag for review if the team prefers a developer-experience framing.
- Brought the three prototype-missing doc types (`work-item-reviews`, `design-gaps`, `design-inventories`) into Glyph's scope rather than spinning them out into a separate work item, because every list-view consumer renders a fixed twelve-key set and a partial Glyph would force consumers to special-case three keys.
- Treated a11y default as decorative (`aria-hidden="true"`) because every Glyph consumer site identified in the prototype renders Glyph adjacent to a visible doc-type label; the optional `accessibleLabel` prop covers future standalone use without forcing today's call sites to label twice. This matches the convention adopted by MUI Icon, GitHub Primer Octicons, Adobe Spectrum, IBM Carbon, and Radix AccessibleIcon.
- The updated library-view screenshots (light + dark) supersede the informal colour list ("red for Decision, orange for Research…") from earlier prototype notes; that list was incomplete (nine of twelve) and is no longer authoritative. The screenshots are the canonical asset-and-colour reference.
- Treated icon-asset packaging (per-component SVG vs sprite vs inline-SVG map) as an implementation detail rather than a Requirement, because the choice doesn't change Glyph's public API and would needlessly constrain the implementer.
- Did **not** propose changes to title, type, priority, parent, or tags — they read correctly for a load-bearing shared visual primitive.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots (canonical, twelve-doc-type, light + dark):
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-light.png`
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-dark.png`
- Screenshots (additional consumer context):
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/kanban-view.png`
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/lifecycle-cluster-detail.png`
- Related: 0033, 0036, 0038, 0040, 0041, 0042, 0043, 0053, 0054, 0055
