---
date: 2026-05-24T23:16:46+01:00
researcher: Toby Clemson
git_commit: 0f16c47c1f7cadb49bb69b320d94c7b36565c4d9
branch: HEAD
repository: accelerator
topic: "Pre-implementation context for 0074 — Per-Doc-Type Hues on Detail Page"
tags: [research, codebase, frontend, detail-page, tokens, glyph, eyebrow, related-artifacts, e2e, visualiser]
status: complete
last_updated: 2026-05-24
last_updated_by: Toby Clemson
work_item: "0074"
---

# Research: Pre-implementation Context for 0074 — Per-Doc-Type Hues on Detail Page

**Date**: 2026-05-24T23:16:46+01:00
**Researcher**: Toby Clemson
**Git Commit**: 0f16c47c1f7cadb49bb69b320d94c7b36565c4d9
**Branch**: HEAD
**Repository**: accelerator

## Research Question

Produce a comprehensive pre-implementation context document for work item
0074 (Per-Doc-Type Hues on Detail Page): the current state of `--ac-doc-*`
token consumption, the eyebrow/detail-page surfaces, the `RelatedArtifacts`
component, the e2e infrastructure modelled by AC #1–#3, and the historical
context from sibling work items, ADRs, and prior research.

## Summary

Five inaccuracies in the work item must be reconciled before implementation
begins:

1. **No `TypeGlyph` or `StageTile` component exists.** The unified component
   is `Glyph` (`components/Glyph/Glyph.tsx`). Hub cards use it via a local
   `HubCard` component inside `LibraryOverviewHub.tsx`; the listing-route
   eyebrow uses it via a local `EyebrowLabel` helper inside
   `LibraryTypeView.tsx`. There is **no separate sidebar consumer** — the
   `Sidebar` component does not consume `--ac-doc-*` tokens at all today.
2. **No `--ac-text-muted` token exists.** The semantic muted-text token is
   `--ac-fg-muted` (resolved light value `rgb(95, 99, 120)` /
   `#5f6378`). The AC #1/#2 wording referring to "the literal RGB string
   captured for `--ac-text-muted`" should target `--ac-fg-muted` instead.
3. **The model specs are not under `e2e/`.** They live under
   `frontend/tests/visual-regression/` and run as a separate Playwright
   project (`visual-regression`) that the default `chromium` project depends
   on. AC #1–#3 should therefore land new specs in
   `tests/visual-regression/`, not `e2e/`.
4. **The detail page does not currently pass an `eyebrow` prop at all.**
   `LibraryDocView.tsx:152-156` renders `<Page title={title}
   subtitle={subtitle}>{body}</Page>` with no eyebrow. 0074 must therefore
   *introduce* an eyebrow on the detail page, not just tint an existing
   icon. The list-route `EyebrowLabel` helper is the canonical pattern to
   mirror (or extract into a shared component).
5. **`RelatedArtifacts` rows have no icons today.** Rows render a link plus
   a single text "kind" badge (`declared`/`inferred`). Per-row doc-type
   icons must be added (the work item flags this as in-scope).

The remaining context is in good shape: the doc-type key (`entry.type:
DocTypeKey`) is already plumbed end-to-end on both surfaces (detail-page
route param and `RelatedArtifactsResponse` payload), no upstream wiring is
needed, and the `--ac-doc-*` tokens are bare hex literals in light theme
(directly capturable) that collapse to `var(--atomic-white)` /
`#1d2030` in dark theme — so AC #1/#2 expectations must differ light vs
dark.

## Detailed Findings

### Token system (`--ac-doc-*` / `--ac-doc-bg-*`)

**Declared in** `skills/visualisation/visualise/frontend/src/styles/global.css`:

- Light foregrounds at lines 100-111 (`--ac-doc-decisions: #ad3437;`
  through `--ac-doc-design-inventories: #2e7e8a;`) — all bare 6-digit hex
  literals, no `hsl()` / `oklch()` / `var()` indirection.
- Light backgrounds at lines 115-126 (`--ac-doc-bg-decisions: #fbe5e6;`
  through `--ac-doc-bg-design-inventories: #dceaec;`).
- Dark-theme MIRROR-A at `[data-theme="dark"]`, lines 328-339 (all 12 fg
  tokens → `var(--atomic-white)` = `rgb(255, 255, 255)`) and 342-353 (all
  12 bg tokens → `#1d2030`).
- Dark-theme MIRROR-B at `@media (prefers-color-scheme: dark)
  :root:not([data-theme="light"])`, lines 388-411 (byte-equivalent to
  MIRROR-A — parity unit-tested by `global.test.ts`).

**Resolved-hex TS mirror** in
`skills/visualisation/visualise/frontend/src/styles/tokens.ts:35-46`
(light fg), `:50-61` (light bg), `:94-105` (dark fg, all `#ffffff`),
`:109-120` (dark bg, all `#1d2030`). A CSS↔TS parity comparator (referenced
at `tokens.ts:1-6`) drift-tests these.

**Important values for AC capture** (light theme, story-start RGB):

| Token | Hex | RGB |
|---|---|---|
| `--ac-doc-decisions` | `#ad3437` | `rgb(173, 52, 55)` |
| `--ac-doc-work-items` | `#af4b2f` | `rgb(175, 75, 47)` |
| `--ac-doc-plans` | `#3256b6` | `rgb(50, 86, 182)` |
| `--ac-doc-research` | `#b26f35` | `rgb(178, 111, 53)` |
| `--ac-doc-plan-reviews` | `#5127b5` | `rgb(81, 39, 181)` |
| `--ac-doc-pr-reviews` | `#7f2cb6` | `rgb(127, 44, 182)` |
| `--ac-doc-work-item-reviews` | `#ad3458` | `rgb(173, 52, 88)` |
| `--ac-doc-validations` | `#2e8b57` | `rgb(46, 139, 87)` |
| `--ac-doc-notes` | `#8e7b22` | `rgb(142, 123, 34)` |
| `--ac-doc-pr-descriptions` | `#4588b8` | `rgb(69, 136, 184)` |
| `--ac-doc-design-gaps` | `#5c9132` | `rgb(92, 145, 50)` |
| `--ac-doc-design-inventories` | `#2e7e8a` | `rgb(46, 126, 138)` |
| `--ac-fg-muted` (`templates` fallback) | `#5f6378` | `rgb(95, 99, 120)` |

The chip-resolved-colours spec's `hexToRgb()` helper produces exactly the
serialised form (`rgb(R, G, B)`) returned by Chromium for
`getComputedStyle().color` — reuse that helper rather than hardcoding the
table.

### `DocTypeKey` enumeration

- Defined at
  `skills/visualisation/visualise/frontend/src/api/types.ts:4-8` (13-member
  union).
- Runtime mirror `DOC_TYPE_KEYS` at lines 14-19; guard `isDocTypeKey()` at
  lines 22-24.
- Virtual-key constant `VIRTUAL_DOC_TYPE_KEYS = ['templates']` at line 30,
  with rationale in the lines 26-29 comment.
- 12-member `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` at
  `components/Glyph/Glyph.constants.ts:19`, with runtime
  `GLYPH_DOC_TYPE_KEYS` (lines 23-25) and `isGlyphDocTypeKey()` guard
  (lines 28-30).
- Human-readable labels in `DOC_TYPE_LABELS` at `api/types.ts:35-49`.

### Current `--ac-doc-*` consumers (complete list)

There are **two** consumer surfaces, both inside `Glyph`:

1. **`Glyph.tsx:110, 129`** — inline `style={{ color:
   \`var(--ac-doc-${docType})\` }}` on the inner `<svg>` (framed and
   unframed branches). Child icons use `fill="currentColor"` (contract at
   lines 60-75, 120-123).
2. **`Glyph.module.css:15-26`** — twelve attribute-selector rules of the
   form `.frame[data-doc-type="decisions"] { background:
   var(--ac-doc-bg-decisions); }` styling the `<span class="frame"
   data-doc-type=…>` wrapper (rendered by `Glyph.tsx:101-104`).

One **local override** (not a consumer): `ActivityFeed.module.css:89-92`
forces `.row [data-doc-type] { color: var(--ac-fg-faint) !important; }` to
strip the per-doc-type hue back to monochrome in activity rows.

**Indirect `Glyph` callsites** (the surfaces that get tinted):

- `routes/library/LibraryOverviewHub.tsx:72-74` — `HubCard` (overview hub
  card grid).
- `routes/library/LibraryTypeView.tsx:273-274` — `EyebrowLabel` local
  helper, used at lines 159, 170, 189, 201 in the listing-route eyebrow
  slot.
- `routes/library/LibraryTemplatesIndex.tsx:115-116` — templates list rows.
- `components/ActivityFeed/ActivityFeed.tsx:122-123` — unframed, then
  monochrome-overridden.
- `routes/glyph/GlyphShowcase.tsx:2` — dev-only `/glyph-showcase` route.

**The sidebar does not consume `--ac-doc-*` today.**
`components/Sidebar/Sidebar.tsx:56-72` renders text labels with a count
badge — no `Glyph`, no per-doc-type tint. AC #3's "sidebar (`TypeGlyph`)"
claim does not match the current code; AC #3 should be reframed to cover
`HubCard` and the listing-route `EyebrowLabel`.

### Detail-page rendering today

**Route**:
`skills/visualisation/visualise/frontend/src/router.ts:109-113` defines
`libraryDocRoute` (`path: '/$fileSlug'`, parent `libraryTypeRoute` at
lines 97-107). Final URL pattern: `/library/$type/$fileSlug`. The parent
route's `parseParams` already narrows `params.type` to `DocTypeKey` (or
redirects to `/library`).

**Component**:
`skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`:

- Lines 37-42 narrow the URL `$type` to `DocTypeKey` — same pattern as
  `LibraryTypeView.tsx:103-106`. The narrowed `type` variable is in scope
  at render time.
- Props interface (`Props` at lines 30-33) also accepts an optional `type?:
  DocTypeKey`.
- Lines 152-156: `return <Page title={title} subtitle={subtitle}>{body}
  </Page>` — **no `eyebrow` prop**. This is the gap 0074 must close.
- Lines 103-121: the `<div className={styles.aside}><section><h3>Related
  artifacts</h3>…</section></div>` block hosts the `<RelatedArtifacts
  related={related.data} … />` call (line 117). The "Related artifacts"
  heading is rendered by the caller (line 105), not the component.

**Page component**:
`skills/visualisation/visualise/frontend/src/components/Page/Page.tsx`:

- `PageProps` (lines 4-11): `eyebrow?: ReactNode`, `title`, `subtitle?`,
  `actions?`, `maxWidth?`, `children`. No `docType` or `data-doc-type`
  prop today.
- Line 27 renders `<div className={styles.eyebrow}
  data-slot="eyebrow">{eyebrow}</div>` (only when `eyebrow` is provided).
- `Page.module.css:30-40`: `.eyebrow` rules set `font-family:
  var(--ac-font-mono)`, `font-size: var(--size-eyebrow)`, `letter-spacing:
  0.12em`, `text-transform: uppercase`, `color: var(--ac-fg-faint)`. The
  *label text colour* is `--ac-fg-faint` (resolved `#8b90a3`). The inner
  Glyph's colour is **not** styled here — it comes from the inline style
  Glyph sets on its `<svg>`.

**Glyph mapping**:
`components/Glyph/Glyph.tsx:32-45` `ICON_COMPONENTS: Record<GlyphDocTypeKey,
ComponentType>` exhaustively maps each of the 12 non-virtual keys to its
icon component. `EyebrowLabel` does not maintain its own mapping — it
forwards to `<Glyph docType={type} size={16} framed />` and reads the
text label from `DOC_TYPE_LABELS[type]`.

**Detail-page eyebrow pattern to apply** (mirror of
`LibraryTypeView.tsx:270-279`):

```tsx
function EyebrowLabel({ type }: { type: DocTypeKey }) {
  return (
    <>
      {isGlyphDocTypeKey(type) && <Glyph docType={type} size={16} framed />}
      {DOC_TYPE_LABELS[type].toUpperCase()}
    </>
  )
}
```

For the virtual `templates` case the guard is false, no Glyph renders, and
the label-text path inherits `--ac-fg-faint`. **However**, the work item
specifies the templates-case icon must resolve to `--ac-fg-muted`
(`rgb(95, 99, 120)`) — implying *something* must render in the icon slot
for `templates` even though no Glyph icon is wired. This needs a small
divergence from the existing pattern (e.g. a neutral placeholder icon) or
a re-interpretation of the AC (no icon, only label text — in which case
`--ac-fg-muted` is the wrong selector and `--ac-fg-faint` is what would
actually be measured). Flagging for review.

### `RelatedArtifacts` (right-hand aside)

**File**:
`skills/visualisation/visualise/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx`.

- Props (lines 5-15): `{ related: RelatedArtifactsResponse,
  showUpdatingHint?: boolean }`.
- Three optional groups rendered as `<RelatedGroup label="Targets"
  kind="declared">`, `"Referenced by"` declared, `"Same lifecycle"`
  inferred (lines 37-57). Each group is a `<div class="group
  group{Declared|Inferred}"><h4>label</h4><ul class="groupList"><li
  class="groupItem">…</li></ul></div>` (lines 79-99).
- Each `<li>` row: `<a
  href="/library/${entry.type}/${fileSlugFromRelPath(entry.relPath)}">{entry.title
  || entry.relPath}</a><span class="badge badge{Declared|Inferred}">{kind}</span>`
  (lines 89-94). **No icon, no `<Glyph>`, no SVG, no `data-*` attributes.**
- `entry.type: DocTypeKey` is already in scope on every row (line 90 uses
  it for the href) — no upstream plumbing needed to add a per-row glyph.

**CSS module**
`components/RelatedArtifacts/RelatedArtifacts.module.css`:

- `.group` (lines 10-13): margin/padding only.
- `.groupDeclared` (line 40): `border-left: 2px solid var(--ac-accent);`
  (group-level, not per-row).
- `.groupInferred` (line 41): `border-left: 2px dashed
  var(--ac-fg-faint);`.
- `.groupItem` (lines 21-31): `display: flex; gap: 0.4rem; align-items:
  baseline; padding: 0.15rem 0;` — **no `color`, no `background-color`, no
  `border-color`, no `border-width` on the row container.** AC #2's
  "row container computed background-color/border-color/border-width
  identical pre- and post-change" is trivially satisfied as long as no
  rules are added to `.groupItem`.
- `.badge` (lines 43-53): tiny uppercase pill with `border: 1px solid
  currentColor`, `color: var(--ac-fg-muted)` overridden to
  `--ac-accent` (declared) or `--ac-fg-faint` (inferred). This is text-only
  — no SVG; orthogonal to a doc-type icon.

**Upstream data**:
`RelatedArtifactsResponse` shape at `api/types.ts:187-191` — three
`IndexEntry[]` arrays. `IndexEntry.type: DocTypeKey` at `api/types.ts:64-66`,
populated by the Rust server, fetched via
`fetchRelated()` (`api/fetch.ts:136-139`), exposed via
`useDocPageData(entry?.relPath)` in `LibraryDocView.tsx:57`.

### E2e test infrastructure

**Model specs are NOT in `e2e/`** — they are in
`skills/visualisation/visualise/frontend/tests/visual-regression/`:

- `chip-resolved-colours.spec.ts` (108 lines) — looped over light/dark
  themes and 5 chip variants, asserts `getComputedStyle().color` /
  `.backgroundColor` against a `hexToRgb(LIGHT_COLOR_TOKENS['ac-…'])`
  expected table. Uses dev-only `/chip-showcase` route
  (`router.ts:151-155`). Embeds a local `hexToRgb()` helper (lines 4-10),
  a `parseRgb()` helper that handles both legacy `rgb()` and CSS Color
  Level 4 `color(srgb …)` (lines 12-30), and a range-comparator
  `expectChannelsBetween()` (lines 32-42) for `color-mix()` outputs.
- `glyph-resolved-fill.spec.ts` (34 lines) — minimal: two flat tests
  asserting the `<svg>` for `glyph-cell-decisions-24` resolves
  `getComputedStyle().color` to
  `hexToRgb(LIGHT_COLOR_TOKENS['ac-doc-decisions'])` in light theme and
  `…DARK_COLOR_TOKENS…` in dark. Single doc-type only.
- `code-block-resolved-colours.spec.ts` — same directory, closest existing
  template for a per-token-per-theme looped spec.

**Playwright config**
(`skills/visualisation/visualise/frontend/playwright.config.ts`):

- `testDir: './e2e'` (line 6). `workers: 1` forced serial (line 11).
- `globalSetup: './e2e/global-setup.ts'`,
  `globalTeardown: './e2e/global-teardown.ts'` (lines 14-15) — set
  `process.env.BASE_URL` after server start; snapshot/restore work-item
  fixture files around mutating tests.
- Two projects (lines 21-37):
  1. `visual-regression` — `testDir: './tests/visual-regression'`, runs
     first.
  2. `chromium` — `dependencies: ['visual-regression']`, runs against
     `./e2e`.
- `webServer.command: 'node e2e/start-server.mjs'` (lines 38-46) builds
  the Rust binary with `cargo build --no-default-features --features
  dev-frontend` and writes a JSON config listing fixture `doc_paths`.

**`start-server.mjs`** (`e2e/start-server.mjs:60-92`) maps fixture
directories to `doc_paths`:

- `decisions` → `tests/fixtures/meta/decisions`
- `work` → `tests/fixtures/meta/work`
- `plans` → `tests/fixtures/meta/plans`
- `research` → `tests/fixtures/meta/research`
- `review_plans` → `meta/reviews/plans`
- `review_prs` → `meta/reviews/prs`
- `validations` → `meta/validations`
- `notes` → `meta/notes`
- `prs` → `meta/prs`

**No entries for `work_item_reviews`, `design_gaps`,
`design_inventories`** — confirming the gap 0074 calls out.

**Existing fixture directories**
(`skills/visualisation/visualise/server/tests/fixtures/meta/`):
`decisions/`, `work/`, `plans/`, `research/`, `reviews/plans/`,
`reviews/prs/`, `validations/`, `notes/`, `prs/`. **Missing**:
`work-item-reviews/`, `design-gaps/`, `design-inventories/`. All existing
fixtures are markdown with optional YAML frontmatter; shape varies (some
intentionally malformed for resilience tests).

**Detail-page URL construction** — slug derivation via
`fileSlugFromRelPath()` (`api/path-utils.ts:6-8`) strips trailing `/`
segment + `.md` extension. Two URL forms accepted by
`LibraryDocView.tsx:50-52`:

- Server-stripped numeric prefix: `/library/work-items/parent-epic` (used
  by `cross-refs.spec.ts:19`).
- Full filename minus extension: `/library/work-items/0007-parent-epic`
  (used by `RelatedArtifacts` itself at `.tsx:90` and
  `cross-refs.spec.ts:63`).

**Existing detail-page e2e exemplars** (under `e2e/`):
`cross-refs.spec.ts`, `wiki-links.spec.ts`, `mermaid.spec.ts`,
`navigation.spec.ts`. Canonical idiom: `await page.goto('/library/<type>/<slug>')`
→ `await expect(page.locator('article')).toBeVisible()` → assertions via
`page.locator()` / `page.getByRole()`.

**No fixture-aware helper exists.** `e2e/fixtures.ts` is just `export {
test, expect } from '@playwright/test'`. AC #1/#2 either need a new helper
or 12 hardcoded `(docType, slug)` pairs.

### Historical context (work items, ADRs, prior research)

**0037 — Glyph Component (done)**
(`meta/work/0037-glyph-component.md`):

- Delivered: the `Glyph` component, 12 per-type `*Icon.tsx` files, the
  `--ac-doc-<key>` token namespace in all three theme blocks plus
  `tokens.ts` mirror, the `/glyph-showcase` dev route, Vitest + Playwright
  visual-regression coverage.
- Consumer contract (lines 117-124) — load-bearing for 0074: never
  override `fill`; provide adjacent text label or `ariaLabel` (default is
  `aria-hidden`); never wrap in another `<svg>`; sizes 16/24/32 only;
  narrow `DocTypeKey` via `isGlyphDocTypeKey` / `GLYPH_DOC_TYPE_KEYS`,
  never with `as` casts.
- **Detail-page consumption was anticipated** — 0037 routed it through a
  separate "0043 detail-screen capability-retention spike". 0074 picks up
  that thread. There is no explicit prohibition; the phrase "0037
  explicitly does not consume them on the detail page" in 0074's framing
  is descriptive of what shipped, not a forbidden-list.

**0041 — Library Page Wrapper (done)**
(`meta/work/0041-library-page-wrapper-and-overview-hub.md`):

- Established the `Page` wrapper with `[data-slot="eyebrow"]` slot
  mechanism.
- Listing-view eyebrow shape: `<Glyph for doc type> + uppercase doc-type
  name`; H1 = doc-type label; subtitle = `{N} documents`.
- AC at line 134 delegates per-doc-type colour to "the Glyph component's
  per-doc-type colour mapping (0037)".
- 0041 ACs do not address detail-page eyebrow consumption — that is
  precisely 0074's gap.

**0073 — Atomic Brand-Layer Palette (ready)**
(`meta/work/0073-atomic-brand-layer-palette.md`):

- Introduces `--atomic-*` brand layer; only `--ac-*` tokens whose
  normalised hex *exactly* matches an `--atomic-*` value get rewritten via
  `var(--atomic-X)`. Near-misses stay literal.
- Resolved hex values for all `--ac-doc-*` tokens are **preserved**. AC #3
  baseline strategy in 0074 (capture literal RGB at story start) is safe.
- Reinforced by **ADR-0035** (`meta/decisions/0035-…`): brand-layer
  indirection is structural; TS-side `LIGHT_COLOR_TOKENS` /
  `DARK_COLOR_TOKENS` keep resolved hex.

**0079 — Aside Region Redesign (draft)**
(`meta/work/0079-aside-region-redesign.md`):

- Redesigns the aside structure (section vocabulary, declared/inferred
  grouping, adds a Cluster block).
- **Blocked by 0074** — the per-doc-type icon colour must land first.
- 0074 scopes aside changes to "row icon only — no background or border
  changes" to minimise conflict (work item 0074 lines 70-72).

**0082 — BigGlyph Hero Illustrations (draft)**
(`meta/work/0082-big-glyph-hero-illustrations.md`):

- Ships per-doc-type hero illustrations via a 7-tone palette from a single
  hue per type (`bigPalette(hue)`).
- **Blocked by 0074** (0082 line 76).
- Hero tint is explicitly out of scope for 0074. 0082 may eventually need
  a single hue *number* (HSL hue integer) per type rather than a resolved
  RGB — 0074 is not required to expose hue numbers.

**Design-gap research 2026-05-21**
(`meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`):

- "Token Drift" (lines 56-65) and "Net-New Features" (lines 440-444)
  describe the prototype's `TypeColourCoding` pattern: every doc type
  carries a stable hue consumed by `TypeGlyph` at three sizes per surface
  — eyebrow 16, aside row 22, landing 34. 0074 covers the eyebrow + aside
  surfaces only; landing/hero deferred to 0082.

**Prototype hue map**
(`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md:173-191`):

- Thirteen HSL hue integers (not RGB hex): `work` 12, `decisions` 355,
  `research` 28, `plans` 220, `plan-reviews` 260, `validations` 160,
  `pr-descriptions` 200, `pr-reviews` 280, `work-reviews` 340,
  `design-inventories` 185, `design-gaps` 95, `notes` 50, `templates`
  215.
- **Naming asymmetries** to flag: prototype `work` → app `work-items`;
  prototype `work-reviews` → app `work-item-reviews`; prototype
  `templates` (hue 215) → app treats `templates` as virtual.
- Prototype hue map is authoritative for *which surfaces* consume tint;
  the resolved hex values for 0074 come from 0037's eyedropper-derived
  table in `global.css:98-126`, not from the prototype's HSL.

**ADR-0026** (`meta/decisions/0026-…`) — CSS design-token application
conventions (color-mix tinting, ±2px substitution). Likely irrelevant to
0074's icon-only scope but governs the broader token consumption
discipline.

**Prior codebase research relevant to 0074**:

- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — original
  Glyph design.
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
  and `…-supplementary.md` — establishes the eyebrow pattern.
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
  — adjacent detail-page surface.
- `meta/research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md` —
  confirms resolved-value preservation.
- `meta/research/codebase/2026-05-22-0081-status-badge-component.md` —
  adjacent detail-page surface.
- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
  — confirms eyebrow size and colour concerns are orthogonal.

## Code References

- `frontend/src/styles/global.css:100-111` — light `--ac-doc-<key>` (12
  hex literals)
- `frontend/src/styles/global.css:115-126` — light `--ac-doc-bg-<key>`
- `frontend/src/styles/global.css:328-353` — dark MIRROR-A (collapse to
  white / `#1d2030`)
- `frontend/src/styles/global.css:388-411` — dark MIRROR-B (media query)
- `frontend/src/styles/global.css:81-84` — `--ac-fg`, `--ac-fg-strong`,
  `--ac-fg-muted` (`#5f6378`), `--ac-fg-faint` (`#8b90a3`)
- `frontend/src/styles/tokens.ts:35-46, 50-61, 94-105, 109-120` — TS
  resolved-hex mirror
- `frontend/src/api/types.ts:4-8` — `DocTypeKey` 13-member union
- `frontend/src/api/types.ts:30` — `VIRTUAL_DOC_TYPE_KEYS = ['templates']`
- `frontend/src/api/types.ts:35-49` — `DOC_TYPE_LABELS`
- `frontend/src/api/types.ts:64-66` — `IndexEntry.type: DocTypeKey`
- `frontend/src/api/types.ts:187-191` — `RelatedArtifactsResponse`
- `frontend/src/components/Glyph/Glyph.tsx:32-45` — `ICON_COMPONENTS` map
- `frontend/src/components/Glyph/Glyph.tsx:60-75, 120-123` — colour
  contract docstring
- `frontend/src/components/Glyph/Glyph.tsx:101-115, 124-135` — framed /
  unframed render, inline `style={{ color: var(--ac-doc-${docType}) }}`
- `frontend/src/components/Glyph/Glyph.constants.ts:19, 23-25, 28-30` —
  `GlyphDocTypeKey`, `GLYPH_DOC_TYPE_KEYS`, `isGlyphDocTypeKey`
- `frontend/src/components/Glyph/Glyph.module.css:15-26` — attribute
  selectors for `--ac-doc-bg-<key>` on `.frame`
- `frontend/src/components/Page/Page.tsx:4-11, 27` — `PageProps`, eyebrow
  slot
- `frontend/src/components/Page/Page.module.css:30-40` — `.eyebrow`
  styling (`color: var(--ac-fg-faint)`)
- `frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx:17-60,
  79-99` — top-level render and `RelatedGroup` row markup (no icons today)
- `frontend/src/components/RelatedArtifacts/RelatedArtifacts.module.css:21-53`
  — `.groupItem`, `.badge`
- `frontend/src/components/Sidebar/Sidebar.tsx:56-72` — sidebar markup
  (no `--ac-doc-*` consumption)
- `frontend/src/router.ts:97-113` — `libraryTypeRoute` (with
  `parseParams` narrowing) + `libraryDocRoute`
- `frontend/src/routes/library/LibraryTypeView.tsx:103-106, 270-279` —
  detail of the canonical `EyebrowLabel` pattern
- `frontend/src/routes/library/LibraryDocView.tsx:37-42, 57, 115-120,
  152-156` — doc-type narrowing in scope; aside hosting; **`<Page>`
  rendered without `eyebrow` prop today**
- `frontend/src/routes/library/LibraryOverviewHub.tsx:58-88` — `HubCard`
  local component (the "library hub" equivalent of the work item's
  `StageTile`)
- `frontend/src/api/path-utils.ts:6-8` — `fileSlugFromRelPath()`
- `frontend/src/api/fetch.ts:136-139` — `fetchRelated()`
- `frontend/playwright.config.ts:6, 11, 14-15, 21-37, 38-46` — config,
  projects, webServer
- `frontend/e2e/start-server.mjs:60-92, 127-128` — fixture mapping +
  port handoff
- `frontend/e2e/global-setup.ts:16-53` — `BASE_URL` env setup + fixture
  snapshot
- `frontend/e2e/fixtures.ts` — bare re-export only
- `frontend/tests/visual-regression/chip-resolved-colours.spec.ts:4-10,
  12-30, 32-42, 46-61, 75-103` — `hexToRgb`/`parseRgb`/
  `expectChannelsBetween` helpers + looped per-variant assertion
- `frontend/tests/visual-regression/glyph-resolved-fill.spec.ts` (34
  lines) — minimal `getComputedStyle().color` pattern for a single
  doc-type
- `frontend/e2e/cross-refs.spec.ts:19, 63` — both URL-form precedents

## Architecture Insights

- **Single consumer choke-point**: today, every per-doc-type tint flows
  through one component (`Glyph`). 0074 should preserve that — add `Glyph`
  to the new sites rather than re-implementing the inline-style /
  attribute-selector pair. This keeps the contract centralised and means
  ADR-0035's brand-layer rewrite continues to "just work".
- **The detail page is colour-agnostic by design.** `Page` accepts an
  arbitrary `ReactNode` eyebrow and never inspects doc-type. The cleanest
  scoping mechanism for 0074 is to render a `<Glyph>` *inside* the eyebrow
  payload (same pattern as `LibraryTypeView`), rather than introducing a
  `data-doc-type` attribute on `Page` and rewriting `Page.module.css`. The
  work item permits either approach; the in-component approach is the
  smaller diff.
- **`RelatedArtifacts` row is presentational-only.** The `IndexEntry.type:
  DocTypeKey` is already on every row; adding `<Glyph docType={entry.type}
  size={16} />` guarded by `isGlyphDocTypeKey()` is a one-line insertion
  inside `RelatedGroup`'s `<li>`. No upstream changes, no plumbing.
- **Dark theme makes AC #1/#2 theme-conditional.** All 12 `--ac-doc-*`
  tokens collapse to white in dark mode. Either: (a) restrict AC #1/#2 to
  light theme and add a separate dark-mode AC asserting white, or (b)
  parametrise the expected-RGB table by theme as
  `chip-resolved-colours.spec.ts` does. Work item 0074 currently reads as
  if light theme is the only case; this should be reconciled before
  implementation.
- **`work-item-reviews` / `design-gaps` / `design-inventories` need
  server-side wiring too.** Adding fixture directories alone is not
  enough — `e2e/start-server.mjs:60-92` must also receive `doc_paths`
  entries, and the Rust server's `DocType` registry must support these
  keys (verify before coding). The frontend `DocTypeKey` union already
  lists them, so the type system is already prepared.
- **Listing-route `EyebrowLabel` is not exported.** If the detail page
  adopts an identical pattern, consider extracting it to a shared
  component (e.g.
  `frontend/src/components/EyebrowLabel/EyebrowLabel.tsx`) rather than
  duplicating. This makes 0079's later aside redesign easier too.

## Historical Context

- `meta/work/0037-glyph-component.md` — token namespace and component
  contract origin.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` — eyebrow slot
  pattern.
- `meta/work/0073-atomic-brand-layer-palette.md` — brand-layer rewrite
  preserves resolved values; 0074 AC #3 is insulated.
- `meta/work/0079-aside-region-redesign.md` — blocked by 0074.
- `meta/work/0082-big-glyph-hero-illustrations.md` — blocked by 0074.
- `meta/decisions/0026-css-design-token-application-conventions.md` —
  token discipline governance.
- `meta/decisions/0035-brand-layer-indirection.md` — confirms
  resolved-hex preservation through brand-layer indirection.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — origin of the per-doc-type colour-coding gap.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
  — prototype's 13-entry hue map and three-surface tint vocabulary.

## Related Research

- `meta/research/codebase/2026-05-12-0037-glyph-component.md`
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
- `meta/research/codebase/2026-05-16-0041-library-page-wrapper-supplementary.md`
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
- `meta/research/codebase/2026-05-22-0081-status-badge-component.md`
- `meta/research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md`
- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`

## Open Questions

1. **AC wording mismatches.** Should the work item be updated to (a)
   correct "TypeGlyph"/"StageTile" → `Glyph` (used by `HubCard` and
   listing-route `EyebrowLabel`), (b) correct `--ac-text-muted` →
   `--ac-fg-muted`, (c) note specs land under `tests/visual-regression/`
   not `e2e/`, (d) acknowledge that the detail page does not currently
   render an eyebrow at all (so 0074 *introduces* one), and (e) drop the
   "sidebar consumption unchanged" claim from AC #3 (the sidebar
   doesn't consume `--ac-doc-*` today)?
2. **Templates eyebrow icon slot.** AC #1's "the eyebrow icon's computed
   `color` resolves to exactly the literal RGB string captured for
   `--ac-text-muted`" implies an icon element exists for the templates
   case. Either (a) the eyebrow must render a neutral placeholder icon
   for `templates`, or (b) the AC should specify "no icon element is
   rendered; the eyebrow label text's computed `color` is …" — but the
   eyebrow label colour is `--ac-fg-faint` (`#8b90a3`), not
   `--ac-fg-muted` (`#5f6378`), so this would still need an adjustment.
3. **Dark-theme coverage.** Should AC #1/#2 parametrise expected RGB by
   theme (light per the table above; dark → `rgb(255, 255, 255)`) or
   restrict to light theme and add a separate dark-theme AC?
4. **Server-side wiring for missing fixture types.** Does the Rust server's
   `DocType` registry currently support `work-item-reviews`, `design-gaps`,
   `design-inventories` as on-disk doc types? If not, the in-scope
   prerequisite in 0074 expands beyond fixtures into server-side
   plumbing.
5. **Shared `EyebrowLabel` extraction.** Is extracting the
   `LibraryTypeView.tsx:270-279` helper to a shared component within
   0074's scope, or does that belong in 0079?
