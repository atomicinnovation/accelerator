---
date: 2026-05-14T20:08:27+01:00
researcher: Toby Clemson
git_commit: 08a7f5e3cdca3fb84bae5b5ce3a98c909ad2cbb7
branch: main
repository: accelerator
topic: "Generic Chip Component (work item 0038)"
tags: [research, codebase, chip, design-system, tokens, frontend, components]
status: complete
last_updated: 2026-05-14
last_updated_by: Toby Clemson
---

# Research: Generic Chip Component (work item 0038)

**Date**: 2026-05-14T20:08:27+01:00
**Researcher**: Toby Clemson
**Git Commit**: 08a7f5e3cdca3fb84bae5b5ce3a98c909ad2cbb7
**Branch**: main
**Repository**: accelerator

## Research Question

What does the visualiser frontend codebase look like with respect to chip/pill/badge components today, and what does work item 0038 need in order to deliver a generic `<Chip>` with five named variants (`green`, `indigo`, `amber`, `neutral`, `sm`) that replaces open-coded status pills across kanban cards, lifecycle cards, library tables, and templates indicators? Specifically:

- What chip-like components already exist, and what do they look like?
- Where are status pills open-coded today and what colour/styling do they use?
- Is work item 0033 (token system) actually done, and does it expose semantic status tokens Chip can lean on?
- What does the prototype's `.ac-chip--*` CSS actually look like? (Are the rules captured anywhere?)
- What component conventions (file layout, styling, prop shape, tests) must Chip follow?
- What historical context (work items, ADRs, plans, inventories) bears on the implementation?

## Summary

The visualiser frontend at `skills/visualisation/visualise/frontend/` already has a complete `--ac-*` design token layer (0033 is shipped) and a strong, repeatable component convention: every component is a folder of `Foo.tsx` + `Foo.module.css` + `Foo.test.tsx`, no `index.ts` barrels, named exports, CSS modules, variant selection via `data-*` attributes, and tests that regex-match the imported `?raw` CSS to lock variant→token bindings. `OriginPill` and `SseIndicator` are the canonical analogues for Chip.

However, the codebase is **not yet ready to drop Chip in unchanged**:

1. **No semantic status tokens exist for the four variants**. Tokens cover surfaces/foreground/stroke/accent/per-doc-type, plus a thin `--ac-ok` / `--ac-warn` / `--ac-err` / `--ac-violet` trio that is **only defined in the light block**. Dark mode does not redefine them. Chip's variants `green`/`indigo`/`amber`/`neutral` map most naturally to `--ac-ok` / `--ac-accent` (or `--ac-violet`) / `--ac-warn` / surface neutrals, but acceptance criterion 3 ("Chip variant colours swap correctly between light and dark theme") forces 0038 to add dark-theme overrides — i.e. a small token-layer extension that today's tokens don't cover.

2. **The prototype CSS for `.ac-chip` was never captured**. The runtime crawl in `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/` only enumerated class names. No `padding`, `border-radius`, `font-size`, or hex values are recorded anywhere in the repo. Concrete values must be re-extracted from the live prototype (URL in the inventory frontmatter) or pinned down in a design spec before implementing.

3. **Open-coded status indicators do exist but are sparser and more colour-agnostic than the work item implies**. Only four call sites actually render a coloured/pill label across the named surfaces, and most of them are *single neutral pills* rather than status-coloured chips: lifecycle cluster (`.statusBadge`, neutral), library type view (`.badge`, neutral), templates view (`.activeBadge`, accent-tinted), templates view (`.panel.absent` state). **Kanban renders no status pill at all today** — `WorkItemCard` has an `idChip` text but no coloured background. The "replace open-coded pills" migration is therefore as much an *introduction* as a replacement in three of the four surfaces.

4. **`FrontmatterChips` is structurally a key/value pair list, not a status pill** — the work item's claim that its "inner chip rendering should consume the generic Chip variants" is half-true at best: today its `.chip` class uses `--ac-bg-sunken` + `--radius-sm` (rounded rectangle, not pill) and doesn't vary colour by status. Migration would require either re-styling FrontmatterChips around Chip with the `neutral` variant, or accepting that the two chip primitives diverge intentionally.

5. **A separate `.ac-pulse` element exists in the prototype** (topbar status, SSE indicator) and is documented as distinct from `.ac-chip`. Open Question 2 in 0038 ("icon slot for the green pulse on `live` indicators") concretely means: should the kanban `live` chip nest a `.ac-pulse`? The prototype doesn't confirm this either way, and the current `OriginPill` already implements its own pulse pattern that could be lifted into Chip.

The work item's "this is just a token-consuming component" framing is accurate in spirit but understates the scope: 0038 must extend the token layer with status semantics, add dark overrides, choose between text-only and dot/icon-bearing chip variants, and **introduce** chips into three surfaces that don't currently have them — not merely replace boilerplate.

## Detailed Findings

### 1. Existing chip-like components

Three pre-existing components touch the chip/pill space, none of them generic:

- **`FrontmatterChips`** — `src/components/FrontmatterChips/FrontmatterChips.tsx`. Renders key/value frontmatter pairs as rounded-rectangle "chips" (`background: var(--ac-bg-sunken); border-radius: var(--radius-sm); padding: var(--sp-1) var(--sp-2);` at `FrontmatterChips.module.css:3-5`). Has a malformed-state branch (`FrontmatterChips.tsx:11`) and a malformed-banner style (`FrontmatterChips.module.css:9-13` using `color-mix(--ac-warn, --ac-bg)`). Not a pill — corners are `--radius-sm`, not `--radius-pill`.
- **`OriginPill`** — `src/components/OriginPill/OriginPill.tsx`. Closest analogue to the eventual Chip. Inline-flex pill with a pulsing green dot (`background: var(--ac-ok)`, `border-radius: var(--radius-pill)`) and a mono host string. Has the `@media (prefers-reduced-motion: reduce)` block disabling the animation. Demonstrates the pulse-inside-pill pattern that may answer Chip's icon-slot open question.
- **`SseIndicator`** — `src/components/SseIndicator/SseIndicator.tsx`. Not a pill but the canonical example of `data-state`-driven variant colour selection (`SseIndicator.tsx:17-22`; `SseIndicator.module.css:11-17` maps each state to `--ac-ok` / `--ac-warn` / `--ac-fg-faint` / `--ac-err`).

Beyond these, every other "chip-like" rendering in the codebase is an open-coded `<span>` inside its consumer's CSS module. No central `Chip`, `Badge`, `Pill`, or `Tag` primitive exists.

A repo-wide search for `.ac-chip` / `.ac-chip--*` returns zero matches — the prototype's BEM vocabulary has not been ported into the current frontend.

### 2. Open-coded status pill inventory across the four target surfaces

The "open-coded chips" the work item proposes to replace are sparser and less colourful than the spec implies. The complete inventory across the four named surfaces:

| Surface | File:line | Class | Style | Colour-coded by status? |
|---|---|---|---|---|
| Kanban | `src/routes/kanban/WorkItemCard.tsx` | — | `idChip` text only; no coloured pill | n/a — no status pill rendered |
| Kanban | `src/routes/kanban/KanbanColumn.module.css:19` | — | `--radius-pill` count badge | No — column count, not status |
| Lifecycle | `src/routes/lifecycle/LifecycleClusterView.tsx:125` | `.statusBadge` | `LifecycleClusterView.module.css:95-100` — pill, `--ac-stroke-soft` bg, `--ac-fg` fg | **No** — single neutral pill regardless of status value |
| Library | `src/routes/library/LibraryTypeView.tsx:123` | `.badge` | `LibraryTypeView.module.css:26-30` — pill, `--ac-stroke-soft` bg, `--ac-fg` fg | **No** — single neutral pill |
| Templates | `src/routes/library/LibraryTemplatesView.tsx:60` | `.activeBadge` | `LibraryTemplatesView.module.css:8-10` — pill, `--ac-accent-tint` bg, `--ac-accent` fg, `font-weight: 600` | Single accent-tinted pill (only renders for the active tier) |
| Templates | `src/routes/library/LibraryTemplatesView.tsx:57` (panel) | `.panel.absent` | `LibraryTemplatesView.module.css:5` — `opacity: 0.55` + sunken bg | Not a chip — panel-level absent state |
| Templates | `src/routes/library/LibraryTemplatesIndex.tsx:38` | `.active` | `LibraryTemplatesIndex.module.css:5` — `--size-xxs`, `--ac-fg-muted` | Plain text, no pill |

Implications:

- **Lifecycle and Library are the only true status pills today, and both are neutral** — they could be migrated to `Chip variant="neutral"` with no visual regression. To actually colour-code them by status (the prototype intent) is a separate enrichment that 0038 enables but doesn't itself perform; the actual colouring belongs in 0040 / 0041 / 0042.
- **Templates' `activeBadge` is the only chip already colour-coded** — accent-tinted, so closest to the prototype's `indigo` variant.
- **Kanban has no status chip today**. 0040 (kanban overhaul) will be the surface that *introduces* a status chip on `WorkItemCard`, not the one that *replaces* one.
- The work item's acceptance criterion "`grep` for open-coded inline chip styles … returns no results once the migration is complete" therefore reduces to: replace four CSS-module classes (`.statusBadge`, `.badge`, `.activeBadge`, plus optionally `Sidebar.module.css:158` pill-radius and `LibraryTemplatesIndex.module.css:5`'s `.active` text) with Chip — that's a tractable migration scope.

### 3. State of the `--ac-*` token system (work item 0033)

**0033 is implemented and shipped.** Single source of truth is `src/styles/global.css` (lines 69-251) mirrored to TypeScript at `src/styles/tokens.ts`. CSS↔TS parity is asserted by `src/styles/global.test.ts:127`. Token migration progress is tracked by an exception ledger at `src/styles/migration.test.ts:28-49`.

What's available:

- **Surfaces**: `--ac-bg`, `--ac-bg-raised`, `--ac-bg-sunken`, `--ac-bg-chrome`, `--ac-bg-sidebar`, `--ac-bg-card`, `--ac-bg-hover`, `--ac-bg-active` (`global.css:71-78`).
- **Foreground**: `--ac-fg`, `--ac-fg-strong`, `--ac-fg-muted`, `--ac-fg-faint` (`global.css:79-82`).
- **Strokes**: `--ac-stroke`, `--ac-stroke-soft`, `--ac-stroke-strong` (`global.css:83-85`).
- **Accent (indigo-leaning)**: `--ac-accent`, `--ac-accent-2`, `--ac-accent-tint`, `--ac-accent-faint` (`global.css:86-89`). Light: `#595fc8`, dark: `#8a90e8`.
- **Generic feedback colours (LIGHT ONLY)**: `--ac-ok: #2e8b57`, `--ac-warn: #d98f2e`, `--ac-err: #cb4647`, `--ac-violet` (`global.css:90-93`). **These are not redefined in the dark `[data-theme="dark"]` block (`global.css:169-207`) — they cascade unchanged into dark mode.**
- **Per-doc-type colours**: 12 tokens (`--ac-doc-decisions` through `--ac-doc-design-inventories`) with full light + dark coverage (`global.css:98-109` and `192-203`).
- **Typography, spacing, radius, shadow, layout**: comprehensive coverage including `--radius-pill: 999px` (`global.css:149`) and `--size-xxs` / `--sp-1`/`--sp-2`.

Theme switching wiring (three coordinated mechanisms):

1. **CSS** — `:root` (light), `[data-theme="dark"]` (dark override), and `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` (system-preference mirror). The dark and `@media` blocks are byte-identical, enforced by `global.test.ts`.
2. **Boot** — `src/api/boot-theme.ts:27` reads `localStorage[THEME_STORAGE_KEY]` and sets `data-theme` on `<html>` before React mounts (avoids flash).
3. **Runtime** — `src/api/use-theme.ts:31` sets `data-theme` on attribute change; `useThemeContext()` exposes state.

**Gaps relevant to Chip**:

- No `--ac-chip-*` token namespace.
- No semantic status tokens (`--ac-status-done`, `--ac-status-in-progress`, etc.).
- `--ac-ok` / `--ac-warn` / `--ac-err` are light-only. To meet acceptance criterion 3 ("Chip variant colours swap correctly between light and dark theme"), 0038 must either:
  - **Option A**: add dark overrides for the existing generic feedback trio (smallest delta; benefits the whole codebase).
  - **Option B**: introduce a parallel `--ac-chip-green-bg`/`--ac-chip-green-fg`/etc. namespace with both light and dark values (more verbose; isolates the change).
  - **Option C**: introduce a semantic `--ac-status-*` namespace (most "design system" — neutral about how it's applied).

This is the central design-token decision 0038 has to make. ADR-0026 (CSS design-token application conventions, accepted 2026-05-07) sets the precedent for how to think about this but does not pre-decide it.

### 4. Prototype `.ac-chip` CSS — not captured anywhere

The runtime crawl at `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/` captured only:

- Screenshots (22 PNGs of light/dark/hover states).
- An inventory listing class names and use-sites (`inventory.md:272-274`).
- Doc-type SVG glyphs (`assets/`).

It captured **no CSS rules, no markup, no padding/font-size/border-radius/colour values, no icon-slot definition**. The inventory's full chip section is three sentences:

> ### Chip (`.ac-chip`)
> - **Variants**: `.ac-chip--green` (Done / accepted), `.ac-chip--indigo` (In progress / live), `.ac-chip--amber` (Approve w/ changes), `.ac-chip--neutral`, `.ac-chip--sm`.
> - **Used on screens**: every screen displaying a status — page subtitle, kanban cards, timeline cards, lifecycle cards, library tables, templates "active/absent" indicators.
> - **Purpose**: pill status badge.

Visual evidence from screenshots: chips are pill-shaped (`border-radius ≈ 999px`), small (~11-12px text), fully filled with a light tint of the variant colour and darker text. `Accepted` chips in `library-decisions.png` are green-tinted; `Proposed` chips are violet/indigo-tinted; `Approve w/ changes` chips in lifecycle are amber-tinted. The kanban page subtitle shows a `live` chip in green; whether it has a leading `.ac-pulse` dot cannot be confirmed at screenshot resolution.

The `.ac-pulse` element is documented **separately** in the same inventory (`inventory.md:259-261`) as the topbar status / SSE indicator's breathing dot — it is not stated to nest inside `.ac-chip`.

**Action for the implementation plan**: re-extract the chip CSS from the live prototype URL recorded in the inventory frontmatter (`https://64bfef0a-…claudeusercontent.com/v1/design/projects/…/serve/Accelerator%20Visualiser.html`) before nailing down padding/font-size/border-radius/hex values. Treating the chip dimensions as TBD until then is honest; guessing from screenshots is not.

### 5. React component conventions Chip must follow

Surveyed canonical components (`OriginPill`, `SseIndicator`, `FrontmatterChips`, `PipelineDots`, `TopbarIconButton`, `FontModeToggle`). Pattern is consistent and unambiguous:

- **Folder layout**: `src/components/Chip/` containing `Chip.tsx`, `Chip.module.css`, `Chip.test.tsx`. No `index.ts` barrels anywhere in `src/components/`.
- **Styling**: CSS modules (`*.module.css`). Every value comes from a token (`var(--ac-*)`, `var(--sp-*)`, `var(--size-*)`, `var(--radius-*)`). No Tailwind, no styled-components, no inline styles, no `clsx` dependency.
- **Variant wiring**: `data-variant={variant}` + `data-size={size}` attributes on the element, with CSS selectors like `.chip[data-variant='green']`. This is the `SseIndicator.tsx:17-22` / `SseIndicator.module.css:11-17` pattern. Class-name concatenation exists in `PipelineDots` but the `data-*` approach is preferred — it tests trivially via `querySelector('[data-variant="green"]')`.
- **Prop typing**: exported `interface ChipProps` with string-literal unions:
  ```ts
  export type ChipVariant = 'green' | 'indigo' | 'amber' | 'neutral'
  export type ChipSize = 'default' | 'sm'
  export interface ChipProps {
    variant?: ChipVariant
    size?: ChipSize
    children: ReactNode
  }
  ```
- **Exports**: named only — `export function Chip(...)`. No `export default`.
- **Tests**: Vitest + `@testing-library/react`. Standard idioms:
  - render assertions via `screen.getByText` / `getByRole` / `getByLabelText`,
  - `data-*` attribute assertions via `document.querySelector('[data-variant="green"]')`,
  - **and** a nested `describe('CSS source assertions', …)` that imports `./Chip.module.css?raw` and regex-asserts that each variant binds to the right token. This locks variant→token mapping in test rather than letting CSS drift silently.

`OriginPill` is the closest reference implementation — see Code References below for full quotes.

### 6. Historical context: work items, ADRs, plans

- **Blocker is satisfied**: `meta/work/0033-design-token-system.md` (status: done).
- **No plan yet exists for 0038**. The closest precedent is `meta/plans/2026-05-12-0037-glyph-component.md` (Glyph plan) — a sibling design-system component on the same phase. Reuse its plan structure.
- **No ADR for status / chip semantics yet exists**, but ADR-0026 (CSS design-token application conventions, accepted 2026-05-07) sets the precedent for where chip token decisions live. If 0038 adds a semantic `--ac-status-*` namespace or dark overrides for `--ac-ok`/`--ac-warn`/`--ac-err`, that decision likely warrants a small ADR.
- **0038 is not part of any epic**. The "frontmatter and linkage improvements" epic (0057) is unrelated. The design-system cluster around 0033/0034/0035/0036/0037/0038/0039/0040/0041/0042 stands as a loose group without a named epic.
- **Downstream consumers (0040 kanban, 0041 library, 0042 templates) are all draft.** 0038 unblocks the *colour* enrichment in each, but the consumers themselves do other things too (kanban card layout, library overview hub, templates redesign). They will adopt Chip but Chip doesn't decide their card/page-level structure.

## Code References

Chip-relevant code in the visualiser frontend (all paths relative to the workspace root `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/visualisation-system/`):

### Existing chip-like components
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx` — Key/value frontmatter pair list; malformed-state branch at line 11.
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css:3-13` — Current "chip" rule (rounded rectangle, not pill) + malformed banner.
- `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.tsx` — Canonical pill analogue with pulsing dot.
- `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.module.css` — Full token-driven pill CSS with `prefers-reduced-motion` block.
- `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.tsx:17-22` — Canonical `data-state` variant wiring.
- `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.module.css:11-17` — Canonical variant→token CSS mapping.

### Open-coded status pills (migration targets)
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.tsx:124-125` — `.statusBadge` open-coded pill.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.module.css:95-100` — `.statusBadge` style (neutral).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:123` — `.badge` open-coded pill.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.module.css:26-30` — `.badge` style (neutral).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:57-60` — `.tierLabel` / `.activeBadge` (accent-tinted).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:5-10` — `.panel.absent` and `.activeBadge` styles.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:38` — `.active` text indicator (no pill today).
- `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx:30` — `idChip` text (no coloured pill today).

### Token system (work item 0033)
- `skills/visualisation/visualise/frontend/src/styles/global.css:69-162` — Light `:root` token block.
- `skills/visualisation/visualise/frontend/src/styles/global.css:90-93` — `--ac-ok` / `--ac-warn` / `--ac-err` / `--ac-violet` (light-only; not redefined in dark).
- `skills/visualisation/visualise/frontend/src/styles/global.css:169-207` — `[data-theme="dark"]` overrides.
- `skills/visualisation/visualise/frontend/src/styles/global.css:213-251` — `@media (prefers-color-scheme: dark)` mirror.
- `skills/visualisation/visualise/frontend/src/styles/global.css:149` — `--radius-pill: 999px`.
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts` — TypeScript mirror of CSS tokens.
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:127` — CSS↔TS parity assertion.
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:28-49` — Hardcoded-hex exception ledger.
- `skills/visualisation/visualise/frontend/src/api/boot-theme.ts:27` — Pre-mount theme attribute set.
- `skills/visualisation/visualise/frontend/src/api/use-theme.ts:31` — Runtime theme switching.

## Architecture Insights

- **Token application convention is already crystallised.** ADR-0026 and the migration ledger in `migration.test.ts` mean Chip cannot legitimately introduce hardcoded hex; it must extend the token layer if it needs new colours. The decision is *which* tokens to add, not whether to use them.
- **Variant selection via `data-*` attributes (not className concatenation) is the house style** for components with mutually exclusive states/variants. This is consistent across `SseIndicator` and partially in `PipelineDots`. Chip should follow this — both for stylistic consistency and because it makes the test pattern (`querySelector('[data-variant="green"]')`) work out of the box.
- **CSS-source regex assertions are a load-bearing testing pattern.** This is unusual but valuable: it lets the test suite catch silent drift between variant prop names and the tokens they bind to. Every Chip variant should be asserted this way in `Chip.test.tsx`.
- **The chip vocabulary is "open-coded but neutral" today, not "open-coded but colourful".** The migration is mostly: (1) extract the pill shape into Chip, (2) keep all four current sites on `variant="neutral"`, (3) leave the actual status→colour mapping to the consumer redesigns (0040/0041/0042). This makes 0038 small and decouples it from the larger redesigns — but the work item should be explicit about that boundary.
- **Pulse pattern already lives in `OriginPill`.** If Chip needs an icon/dot slot for the kanban `live` chip (Open Question 2), the cleanest move is to extract `OriginPill`'s `.pulseDot` rule and the `prefers-reduced-motion` block into a shared CSS pattern (or expose `<Chip dot>` / `<Chip dot={<Pulse />}>` API). Either way, the reduced-motion safeguard is non-negotiable — it's a precedent.

## Historical Context

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` (accepted 2026-05-07) — Sets out token application conventions (typography, theming, semantic colour). Chip's token-layer extension should be argued in language consistent with this ADR.
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — Codebase research that fed into the now-shipped token system. Confirms why `--ac-ok` / `--ac-warn` / `--ac-err` are light-only (they were placeholders pending semantic-status work).
- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — Closest sibling research; Chip and Glyph are parallel design-system components on the same delivery phase. The Glyph plan at `meta/plans/2026-05-12-0037-glyph-component.md` is the structural template for a Chip plan.
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md:272-274` — The only repo-resident enumeration of `.ac-chip--*` variants.
- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md:35` — Restates the same five-variant list with the same labels; no extra detail.
- `meta/work/0036-sidebar-redesign.md` — Marked as an epic but does NOT list 0038 as a child. 0038 is currently an independent story in the loose design-system cluster.

## Related Research

- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — Token system precursor.
- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — Parallel design-system component.
- `meta/research/codebase/2026-05-08-0034-theme-and-font-mode-toggles.md` — Theme switching mechanism (informs the dark-mode token decision for Chip).
- `meta/research/codebase/2026-05-07-0035-topbar-component.md` — Adjacent component research with overlapping styling concerns (topbar status pill).

## Open Questions

These extend (rather than replace) the work item's two open questions:

1. **Which token strategy?** Should 0038 (a) add dark overrides for the existing light-only `--ac-ok`/`--ac-warn`/`--ac-err`/`--ac-violet`, (b) introduce a `--ac-chip-*` namespace, or (c) introduce a semantic `--ac-status-*` namespace? This decision likely deserves its own short ADR.
2. **Does `FrontmatterChips` actually consume Chip?** Today its `.chip` is a rounded *rectangle* (`--radius-sm`) holding key/value pairs — visually and semantically distinct from a status pill. Either restyle it to use `Chip variant="neutral"` (sacrificing the rectangular shape) or accept that the two chip primitives coexist and refine the work-item wording.
3. **Does `live` chip nest `.ac-pulse`?** The prototype doesn't confirm it. `OriginPill` already has the canonical pulse implementation that could be lifted into Chip. Decision needed before the API shape is fixed (`<Chip dot>` boolean? `<Chip leading>{node}</Chip>` slot? Composition with a separate `<Pulse />` component?).
4. **Is the "replace open-coded chips across kanban/lifecycle/library/templates" migration in-scope for 0038, or split off into 0040/0041/0042?** The current open-coded inventory is small enough that 0038 could do it in one shot (lifecycle `.statusBadge`, library `.badge`, templates `.activeBadge`, sidebar pill-radius element), but doing so couples Chip's delivery to those surfaces' redesigns. Cleaner split: 0038 ships Chip + neutral migrations only; status→colour mapping decisions live in the consumer stories.
5. **Re-extracting the prototype chip CSS** — should this happen as part of 0038's planning, or is the look-and-feel deliberately a designer judgement call made during implementation? The current inventory captured no dimensional values; guessing from screenshots is unsafe for things like padding/font-size that interact with adjacent text typography.
6. **Acceptance criterion 4 wording**: "`grep` for open-coded inline chip styles across kanban / lifecycle / library / templates surfaces returns no results once the migration is complete." With the actual call-site count being 4 not "many", this AC could be made concrete: "the four classes `.statusBadge`, `.badge`, `.activeBadge` (templates), and the pill-radius element in `Sidebar.module.css` are removed".
