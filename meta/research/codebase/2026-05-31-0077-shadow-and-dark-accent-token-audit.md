---
type: codebase-research
id: "2026-05-31-0077-shadow-and-dark-accent-token-audit"
title: "Research: Shadow and Dark-Accent Token Audit (work item 0077)"
date: "2026-05-31T22:24:45+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0077"
topic: "Audit --ac-shadow-soft/--ac-shadow-lift and dark --ac-accent/--ac-accent-2 against prototype"
tags: [research, codebase, design-tokens, shadows, dark-theme, visualiser]
revision: "e16087c18363b92240c1c8ccb30cdd32ca861417"
repository: "build-system"
last_updated: "2026-05-31T22:24:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Shadow and Dark-Accent Token Audit (work item 0077)

**Date**: 2026-05-31T22:24:45+00:00
**Author**: Toby Clemson
**Git Commit**: e16087c18363b92240c1c8ccb30cdd32ca861417
**Repository**: build-system

## Research Question

For work item [0077 Shadow and Dark-Accent Token Audit](../../work/0077-shadow-and-dark-accent-token-audit.md):
audit the current visualiser app's `--ac-shadow-soft` / `--ac-shadow-lift`
values and the dark-theme `--ac-accent` / `--ac-accent-2` values against
the claude-design-prototype, then either align them with the prototype or
record an intentional divergence. Enumerate consumer surfaces for the four
tokens to size the verification work.

## Summary

**All four tokens already match the prototype exactly.** The current
visualiser frontend's `src/styles/global.css` declares values that are
byte-equivalent (modulo whitespace) to the prototype's `src/app.css` in
both light and dark themes — including the dark-theme shadow rgba-base
switch from `rgba(10,17,27, …)` (light) to `rgba(0,0,0, …)` (dark). This
parity is not coincidence: the per-theme shadow split was a load-bearing
finding from [the review-1 of plan 0033](../../reviews/plans/2026-05-06-0033-design-token-system-review-1.md)
that was resolved before 0033 merged, and the dark accent values were
written into `tokens.ts` directly from the prototype values during 0033.

The audit's substantive deliverable therefore reduces to:

1. **PR-description evidence** — quote the four current declarations
   alongside the four prototype declarations and state the parity.
2. **Computed-style verification** — confirm at runtime that the
   declarations resolve to `rgb(138, 144, 232)` and `rgb(232, 106, 107)`
   under `data-theme="dark"`. **No existing test reads
   `getComputedStyle(document.documentElement)` for these tokens**, and
   `frontend/README.md:82` explicitly warns against that pattern in
   Playwright (it prescribes `page.waitForFunction` polling against a
   known computed value instead). The audit must either add a new
   Playwright assertion using the `setTheme()` helper from
   `tests/visual-regression/lib/expected-colours.ts`, or perform the
   read manually and quote it in the PR description.
3. **Consumer enumeration** — 26 consumer files, ~56 consumer sites
   total (see §Consumer Enumeration). This **exceeds AC#4's six-surface
   threshold**, triggering the follow-up clause: no before/after
   baselines in this PR, raise a follow-up baseline-refresh work item.
   In practice, since no value changes, the baselines will not move
   regardless and the visual-regression suite's existing
   `tokens.spec.ts` baselines (kanban, library, lifecycle-cluster,
   etc.) already cover the highest-traffic consumers — those baselines
   currently passing IS the evidence.
4. **Migration step** — none required for any of the four tokens.

The audit closes as a parity confirmation rather than a migration. The
PR description should document the comparison; no token-value diff is
expected and no follow-up migration work item is needed (per the work
item's "no follow-up work item is created for the migration itself"
clause).

## Detailed Findings

### Current declarations vs prototype declarations

#### `--ac-shadow-soft` — light theme

**Current** ([`global.css:201`](../../../skills/visualisation/visualise/frontend/src/styles/global.css)):

```css
--ac-shadow-soft: 0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06);
```

**Prototype** (`/Users/tobyclemson/Downloads/Accelerator/src/app.css:36`, selector `:root`):

```css
--ac-shadow-soft: 0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06);
```

**Verdict**: identical.

#### `--ac-shadow-lift` — light theme

**Current** ([`global.css:202`](../../../skills/visualisation/visualise/frontend/src/styles/global.css)):

```css
--ac-shadow-lift: 0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10);
```

**Prototype** (`src/app.css:37`, `:root`):

```css
--ac-shadow-lift: 0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10);
```

**Verdict**: identical.

#### `--ac-shadow-soft` — dark theme

**Current** ([`global.css:364`](../../../skills/visualisation/visualise/frontend/src/styles/global.css), `[data-theme="dark"]`; mirrored at `global.css:422` under `@media (prefers-color-scheme: dark)`):

```css
--ac-shadow-soft: 0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4);
```

**Prototype** (`src/app.css:68`, `[data-theme="dark"]`):

```css
--ac-shadow-soft: 0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4);
```

**Verdict**: identical. Note the deliberate rgba-base switch from
`rgba(10,17,27, …)` in light to `rgba(0,0,0, …)` in dark — both apps
adopt pure black with heavier alpha (0.3 / 0.4) so the shadow is visible
against the deep-night surface (`--atomic-night-2 = rgb(10, 17, 27)`)
without being tinted by it.

#### `--ac-shadow-lift` — dark theme

**Current** ([`global.css:365`](../../../skills/visualisation/visualise/frontend/src/styles/global.css), mirrored at `global.css:423`):

```css
--ac-shadow-lift: 0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55);
```

**Prototype** (`src/app.css:69`, `[data-theme="dark"]`):

```css
--ac-shadow-lift: 0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55);
```

**Verdict**: identical.

#### `--ac-accent` — dark theme

**Current** ([`global.css:329`](../../../skills/visualisation/visualise/frontend/src/styles/global.css), mirrored at `global.css:391`):

```css
--ac-accent: #8a90e8;
```

**Prototype** (`src/app.css:63`, `[data-theme="dark"]`):

```css
--ac-accent: #8A90E8;
```

**Verdict**: identical (modulo hex casing — current app's casing is
lowercase per the convention pinned in [ADR-0026 / ADR-0035](../../decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md)).
Normalised to `rgb()`, both resolve to `rgb(138, 144, 232)`.

#### `--ac-accent-2` — dark theme

**Current** ([`global.css:330`](../../../skills/visualisation/visualise/frontend/src/styles/global.css), mirrored at `global.css:392`):

```css
--ac-accent-2: #e86a6b;
```

**Prototype** (`src/app.css:64`, `[data-theme="dark"]`):

```css
--ac-accent-2: #E86A6B;
```

**Verdict**: identical (casing). Normalised, both resolve to
`rgb(232, 106, 107)`.

### TypeScript token table parity

The values are also enshrined in `src/styles/tokens.ts`, which is the
JS-consumable mirror that the parity unit tests assert against:

- `LIGHT_SHADOW_TOKENS` at `tokens.ts:177–183` carries
  `'ac-shadow-soft': '0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)'`
  and `'ac-shadow-lift': '0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)'`.
- `DARK_SHADOW_TOKENS` at `tokens.ts:186–189` carries the `rgba(0,0,0, …)`
  dark variants verbatim.
- `DARK_COLOR_TOKENS` at `tokens.ts:82–83` carries `'ac-accent': '#8a90e8'`
  and `'ac-accent-2': '#e86a6b'`.

`src/styles/global.test.ts:202–203` already asserts dark-block coverage
extending past `--ac-shadow-lift`. Any value drift between `global.css`
and `tokens.ts` would already be caught by the existing parity test;
the absence of failures across CI is itself evidence that parity holds.

### Consumer Enumeration

Tally produced by grepping `src/` for `var(--<token>)` (declarations
in `global.css` excluded):

| Token              | Consumer sites | Files |
| ------------------ | -------------: | ----: |
| `--ac-shadow-soft` | 1              | 1     |
| `--ac-shadow-lift` | 2              | 2     |
| `--ac-accent`      | 50             | 21    |
| `--ac-accent-2`    | 3              | 3     |
| **Total (unique)** | **~56**        | **~26** |

**`--ac-shadow-soft` consumers (1):**
- [`routes/lifecycle/LifecycleIndex.module.css:79`](../../../skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.module.css) — lifecycle index card hover/focus elevation.

**`--ac-shadow-lift` consumers (2):**
- [`components/Toaster/Toaster.module.css:28`](../../../skills/visualisation/visualise/frontend/src/components/Toaster/Toaster.module.css) — toast card.
- [`components/Popover/Popover.module.css:15`](../../../skills/visualisation/visualise/frontend/src/components/Popover/Popover.module.css) — popover surface.

**`--ac-accent` consumers (21 files):**
Brand.tsx (gradient stops), Breadcrumbs, Chip (`data-variant=indigo`),
FilterPill, FrontmatterTable, PipelineDots, RelatedArtifacts, Sidebar
(active row / focus ring), SortPill, Toaster (accent badge),
KanbanColumn, WorkItemCard, LibraryTemplatesIndex, LibraryTemplatesView,
NoResultsPanel, LifecycleClusterView, LifecycleIndex, and the
`:focus-visible` outline in `global.css:439`.

**`--ac-accent-2` consumers (3 sites, 3 files):**
- [`components/Brand/Brand.tsx:15`](../../../skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx) (brand gradient stop)
- [`components/Brand/Brand.tsx:25`](../../../skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx) (brand SVG fill)
- [`components/FrontmatterTable/FrontmatterTable.module.css:39`](../../../skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.module.css) (key cell tint)

The full file-by-file list is in the [codebase-locator report
embedded above]; reproduce with:

```bash
rg --no-heading -n 'var\(--ac-shadow-soft\)|var\(--ac-shadow-lift\)|var\(--ac-accent\)\b|var\(--ac-accent-2\)' \
   skills/visualisation/visualise/frontend/src/
```

### AC#4 six-surface threshold — triggered

AC#4 of the work item reads:

> If the enumerated list exceeds 6 surfaces, capture no baselines in
> this PR and raise a follow-up work item that enumerates the deferred
> surfaces, names the themes to capture, links back to this audit as
> parent, and inherits this criterion's detection procedure.

26 consumer files (or even 21 unique files if we collapse the four-token
intersection) clearly exceeds 6. **The follow-up clause applies.**

Reading the AC#4 follow-up clause strictly: even though no value
changes here, the criterion as written requires baselines per surface,
and the follow-up path is taken when the surface count exceeds 6.
There is a tension in the criterion: a "before/after" snapshot
contract is degenerate when the values do not change. Two reasonable
readings:

1. **Strict reading**: raise a follow-up baseline-refresh work item
   anyway, even though no diff is expected, so the criterion machinery
   is honoured.
2. **Spirit reading**: because no migration occurs, AC#4's
   "any surface whose pixel diff exceeds 0.1% has its baseline
   refreshed" branch cannot fire — there is nothing to diff against.
   The existing `tokens.spec.ts` baselines passing is sufficient
   evidence; no follow-up is needed.

The work item author should pick one explicitly in the PR description.
This research recommends the spirit reading, with a one-line PR-
description acknowledgement of the threshold and the no-op outcome.

### Existing Playwright theme-toggle plumbing

The dark-theme verification machinery from
[work item 0034](../../work/0034-theme-and-font-mode-toggles.md) is
ready to use:

- **Theme setter**: [`tests/visual-regression/lib/expected-colours.ts`](../../../skills/visualisation/visualise/frontend/tests/visual-regression/lib/expected-colours.ts) exports `setTheme(page, 'light' | 'dark')` which flips `document.documentElement.dataset.theme` and waits for the attribute to land.
- **Lighter alternative**: [`tests/visual-regression/helpers.ts`](../../../skills/visualisation/visualise/frontend/tests/visual-regression/helpers.ts) exports `applyTheme(page, theme)` (same effect, rAF-await instead of `waitForFunction`).
- **Playwright config**: [`playwright.config.ts`](../../../skills/visualisation/visualise/frontend/playwright.config.ts) declares `visual-regression` and `chromium` projects; baseline directory is `tests/visual-regression/__screenshots__/`.
- **Existing baselines covering 0077's consumer surfaces**:
  - `tokens.spec.ts-snapshots/` covers `kanban`, `library`,
    `library-decisions`, `library-templates`, `library-type`,
    `lifecycle-cluster`, `lifecycle-cluster-after-click` in light and
    dark (per platform — darwin and linux).
  - The lifecycle-cluster and lifecycle-index baselines already
    cover the sole `--ac-shadow-soft` consumer (LifecycleIndex card
    hover requires interaction, but the lifecycle-cluster baseline
    covers the `--ac-accent` consumers there).

The Playwright assertion that AC#3 calls for ("computed values are
read via `getComputedStyle(document.documentElement)` under
`data-theme="dark"` and recorded") can be a few lines using
`page.evaluate` after `setTheme(page, 'dark')`:

```ts
const values = await page.evaluate(() => {
  const s = getComputedStyle(document.documentElement)
  return {
    accent:   s.getPropertyValue('--ac-accent').trim(),
    accent2:  s.getPropertyValue('--ac-accent-2').trim(),
    soft:     s.getPropertyValue('--ac-shadow-soft').trim(),
    lift:     s.getPropertyValue('--ac-shadow-lift').trim(),
  }
})
```

Caveat: `frontend/README.md:82` advises `waitForFunction` over
`getComputedStyle` for theme-swap *equality checks* because Chromium
returns the pre-swap computed value transiently after the attribute
flip. Because the `setTheme` helper already awaits the attribute
landing with `waitForFunction`, an immediate `getComputedStyle` read
in the same evaluate call is sound. This is the same pattern other
`*-resolved-colours.spec.ts` files use to read element-level
computed style.

## Code References

- [`global.css:201–202`](../../../skills/visualisation/visualise/frontend/src/styles/global.css) — light-theme shadow declarations.
- [`global.css:329–330`](../../../skills/visualisation/visualise/frontend/src/styles/global.css) — dark accent declarations (`[data-theme="dark"]` mirror A).
- [`global.css:364–365`](../../../skills/visualisation/visualise/frontend/src/styles/global.css) — dark shadow declarations (`[data-theme="dark"]` mirror A).
- [`global.css:391–392`](../../../skills/visualisation/visualise/frontend/src/styles/global.css) — dark accent declarations (`@media (prefers-color-scheme: dark)` mirror B).
- [`global.css:422–423`](../../../skills/visualisation/visualise/frontend/src/styles/global.css) — dark shadow declarations (mirror B).
- [`tokens.ts:82–83`](../../../skills/visualisation/visualise/frontend/src/styles/tokens.ts) — `DARK_COLOR_TOKENS.ac-accent` / `ac-accent-2`.
- [`tokens.ts:177–189`](../../../skills/visualisation/visualise/frontend/src/styles/tokens.ts) — `LIGHT_SHADOW_TOKENS` / `DARK_SHADOW_TOKENS`.
- [`global.test.ts:202–203`](../../../skills/visualisation/visualise/frontend/src/styles/global.test.ts) — existing parity coverage extending past `--ac-shadow-lift`.
- [`tests/visual-regression/lib/expected-colours.ts`](../../../skills/visualisation/visualise/frontend/tests/visual-regression/lib/expected-colours.ts) — `setTheme(page, theme)` helper.
- `/Users/tobyclemson/Downloads/Accelerator/src/app.css:6–70` — prototype's canonical declarations for all four tokens (both themes).

## Architecture Insights

1. **The audit is closure of a pre-existing decision, not a new
   migration.** The dark-theme shadow rgba-base switch
   (`rgba(10,17,27, …)` → `rgba(0,0,0, …)`) was the headline standards
   finding in the 0033 plan review, and the dark accent remap
   (`#595FC8` / `#CB4647` → `#8A90E8` / `#E86A6B`) is the same shift
   the prototype performs. 0033 landed both verbatim — see
   [research/codebase/2026-05-06-0033-design-token-system.md](2026-05-06-0033-design-token-system.md)
   and [reviews/plans/2026-05-06-0033-design-token-system-review-1.md](../../reviews/plans/2026-05-06-0033-design-token-system-review-1.md).

2. **Three declaration sites per token, not one.** Every dark-theme
   token is declared three times: once in `[data-theme="dark"]`
   (MIRROR-A at `global.css:313`), once in the `@media
   (prefers-color-scheme: dark)` block (MIRROR-B at `global.css:373`),
   and once in `tokens.ts` (the JS mirror). Any future edit must touch
   all three; `global.test.ts` parity tests assert MIRROR-A ↔
   MIRROR-B identity, and `prototype-tokens.fixture.test.ts` (CSS ↔
   TS) covers the JS mirror. No edits are needed for 0077, but it is
   worth flagging that the unrelated brand-layer indirection rule
   (ADR-0035 §2) does NOT apply to the dark accents — neither
   `#8a90e8` nor `#e86a6b` maps to any `--atomic-*` brand token (per
   [research 2026-05-23 on the brand-layer palette](2026-05-23-0073-atomic-brand-layer-palette.md)).
   Promoting them to the brand layer is out of scope for 0077.

3. **AC#4's six-surface follow-up clause exists to bound work, not to
   manufacture work when nothing changed.** Strict adherence to the
   criterion when no value drifts would create a follow-up work item
   that performs no migration and produces no visual diff. The author
   should pick the spirit reading and note the choice in the PR
   description.

4. **Playwright's existing baselines already cover the surfaces.**
   `tokens.spec.ts-snapshots/` captures the cross-screen surfaces in
   both themes; the resolved-colour specs
   (`chip-resolved-colours.spec.ts`, `aside-row-resolved-colours.spec.ts`,
   etc.) capture per-element accent assertions. A single new spec
   reading `getComputedStyle` on `document.documentElement` for the
   four tokens would close AC#3's residual ask without adding any new
   baseline directories.

## Historical Context

The shadow / dark-accent story has been visited multiple times:

- [`meta/research/codebase/2026-05-06-0033-design-token-system.md`](2026-05-06-0033-design-token-system.md) — the original tour of the prototype's tokens (incl. `rgba(10,17,27, …)` light and `rgba(0,0,0, …)` dark shadows).
- [`meta/reviews/plans/2026-05-06-0033-design-token-system-review-1.md`](../../reviews/plans/2026-05-06-0033-design-token-system-review-1.md) — review-1 of the 0033 plan flagged "Dark shadow overrides missed — `--ac-shadow-soft` / `--ac-shadow-lift` are theme-variant per inventory" as a major standards finding (lines 85–87, 252–254, 365). Resolved in the review-2 cycle (line 450).
- [`meta/work/0033-design-token-system.md`](../../work/0033-design-token-system.md) — the work item that introduced the five elevation tokens including the per-theme `--ac-shadow-soft` / `--ac-shadow-lift` split (lines 16, 94–104).
- [`meta/plans/2026-05-08-0034-theme-and-font-mode-toggles.md`](../../plans/2026-05-08-0034-theme-and-font-mode-toggles.md) — 0034 already asserts `#8A90E8` / `#E86A6B` resolve correctly under the theme toggle (lines 155, 165, 391).
- [`meta/research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md`](2026-05-23-0073-atomic-brand-layer-palette.md) — confirms `#8a90e8` and `#e86a6b` have no `--atomic-*` brand-token match (lines 145, 203–207, 379), so they remain as hex literals in the semantic layer.
- [`meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`](../design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md) — the gap-analysis source that produced 0077 (lines 94–96, 100, 104). The gap document treats the parity as unverified rather than confirmed; this audit closes that uncertainty as confirmed parity.
- [`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`](../design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md) — prototype inventory; its shadow table covers only light values (silent on dark), which is what flagged the audit as needed.
- [`meta/reviews/work/0077-shadow-and-dark-accent-token-audit-review-1.md`](../../reviews/work/0077-shadow-and-dark-accent-token-audit-review-1.md) — review-1 of this work item.

## Related Research

- [`2026-05-06-0033-design-token-system.md`](2026-05-06-0033-design-token-system.md) — Design Token System research.
- [`2026-05-08-0034-theme-and-font-mode-toggles.md`](2026-05-08-0034-theme-and-font-mode-toggles.md) — Theme / Font-Mode toggles research (provides the dark-theme Playwright fixture).
- [`2026-05-23-0073-atomic-brand-layer-palette.md`](2026-05-23-0073-atomic-brand-layer-palette.md) — Atomic brand-layer palette inventory (the source of truth that the dark accents are NOT brand-mapped).
- [`2026-05-07-0035-topbar-component.md`](2026-05-07-0035-topbar-component.md) — Topbar component research that touched `--ac-shadow-soft` as a subtle-elevation candidate.

## Open Questions

1. **Strict vs spirit reading of AC#4's six-surface threshold.** Pick
   one in the PR description: raise a no-op follow-up work item, or
   acknowledge the no-migration outcome and rely on existing
   `tokens.spec.ts` baselines as evidence. This research recommends
   the spirit reading.
2. **Is a new Playwright spec needed for AC#3's `getComputedStyle`
   read, or is a one-time manual capture (recorded in the PR
   description) sufficient?** The criterion as written does not
   require a permanent CI assertion; a manual capture under
   `setTheme(page, 'dark')` in a dev console, transcribed into the PR
   description, satisfies the literal text. If the team values CI
   regression coverage for the four token values specifically, add a
   spec consuming the `expected-colours.ts` table; otherwise the
   manual capture closes AC#3.
3. **Hex casing in the prototype declarations** (`#8A90E8` vs
   `#8a90e8`). The current app's lowercase casing is per ADR-0035; the
   prototype's uppercase casing is a style preference. Treating this
   as parity (resolved-`rgb()` equivalence) rather than byte-equality
   is the right call — no migration needed.
