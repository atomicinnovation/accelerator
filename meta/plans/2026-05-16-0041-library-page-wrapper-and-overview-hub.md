---
date: "2026-05-16T09:00:00+01:00"
type: plan
producer: create-plan
work_item_id: "0041"
status: done
id: "2026-05-16-0041-library-page-wrapper-and-overview-hub"
title: "0041 — Library Page Wrapper, Overview Hub, and List Views Implementation Plan"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-16T09:00:00+01:00"
last_updated_by: Toby Clemson
revision: "4a4febd1f1ac"
repository: "ticket-management"
relates_to: ["work-item:0041", "codebase-research:2026-05-15-0041-library-page-wrapper-and-overview-hub", "codebase-research:2026-05-16-0041-library-page-wrapper-supplementary", "design-gap:2026-05-06-current-app-vs-claude-design-prototype", "plan:2026-05-15-0038-generic-chip-component"]
---

# 0041 — Library Page Wrapper, Overview Hub, and List Views Implementation Plan

## Overview

Introduce a generic `Page` wrapper, a hand-rolled `Popover` primitive, and an `--ac-content-max-width` design token, then use them to build a server-driven library overview hub at `/library`, a refactored doc-type list view with a sort pill and a doc-type-aware filter pill, two distinct empty states (doc-type-empty and filter-applied-empty), and a new `/api/library/structure` endpoint that drives both the hub and the existing Sidebar phase grouping.

The work also coordinates a `prs` → `pr-descriptions` rename across the wire format, the Rust enum variant, the human label, the `ClusterFlags::has_pr` field, the icon component, the CSS doc-type tokens, and 12 visual-regression baselines (the on-disk `meta/prs/` directory and the `doc_paths` config key both stay as `prs` per the work item's deliberate asymmetry). Five existing `<main>`-padded routes (`KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`) migrate to `Page` so that `RootLayout.main`'s `padding: var(--sp-5) var(--sp-6)` rule can be removed without visual regression, and `PageSubtitle` is deleted outright (its sole consumer `KanbanBoard` migrates to `Page` here).

The work is delivered test-first across **six phases**: foundation primitives → server endpoint → PR descriptions rename → overview hub + Sidebar migration → list view + sort/filter/empty → five-route Page migration + cleanup.

`/api/library/structure` accepts the user's active facet selections as repeated query keys (`?selection[<docType>][<facetId>]=opt1&selection[<docType>][<facetId>]=opt2`); the server recomputes facet option counts per request using the "post-other-facet, pre-own-facet" scoping rule so the FilterPill's counts respond accurately to user toggles. React Query keys include the selection so cache invalidation is correct.

## Current State Analysis

The 2026-05-15 and 2026-05-16 codebase research passes (linked under References) establish current-state context in detail. Highlights that drive the plan structure:

- **No `--ac-content-max-width` token exists**; every route hardcodes its own `max-width` and is allowlisted in `frontend/src/styles/migration.test.ts` (entries at `:122,126,129,134,154`). Five routes carry five different literals (1100/900/900/900/600px).
- **`--ac-bg` is the canonical app-background token**, applied at `body` level in `global.css:4`. There is no `--ac-bg-app`. The work item references both names; `--ac-bg` is correct.
- **`RootLayout.main`** at `RootLayout.module.css:13-17` owns horizontal padding today (`padding: var(--sp-5) var(--sp-6)`). Removing this rule requires every existing `<main>`-padded route to gain its own padding via `Page` in the same change set.
- **`KanbanBoard.module.css:1-6`** has a self-applied `.board { padding: var(--sp-4) }` that will double-pad horizontally once `Page` owns padding — the horizontal component must be dropped during migration.
- **`LibraryTypeView`** (`routes/library/LibraryTypeView.tsx`) currently renders 4 columns (Title / Status / Slug / Modified) with column-header click-sort (`:50-51, 71-74, 143-166`), no ID column, no Page wrapper, no Chip wiring (the status cell uses `<span className={styles.badge}>` against an undefined CSS class — a silent no-op), and a single bare `<p>` empty state at `:135`.
- **`PageSubtitle`** (`components/PageSubtitle/PageSubtitle.tsx`) has a single consumer: `KanbanBoard.tsx` at three call sites (`:143` loading, `:152` error, `:183-185` success with a `<Chip variant="indigo">live</Chip>` child).
- **Router redirect**: `libraryIndexRoute` at `router.ts:71-77` does `redirect({ to: '/library/$type', params: { type: 'decisions' } })`. The chained redirect from `/` (via `indexRoute` at `:60`) and the unknown-type fallback at `:101-104` both terminate at `/library/decisions` today. After this story, all three terminate at `/library`.
- **`PHASE_DOC_TYPES`** in `frontend/src/api/types.ts:228-254` is the only client-side hard-coded phase grouping. Sole consumer is `Sidebar.tsx:38-78`; both go away in this story.
- **Server `describe_types`** at `server/src/docs.rs:115-132` returns flat `Vec<DocType>` with no phase or facet metadata. The new `/api/library/structure` handler is additive (a sibling under `server/src/api/library.rs`).
- **Indexer** holds entries in `entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>` (`indexer.rs:58`). The only existing per-`DocTypeKey` reducer is `counts_by_type()` (`:326-332`). The new `library_aggregates()` method introduces multi-aggregate-per-pass under one `entries.read().await`.
- **No popover/menu/click-outside/checkbox/focus-management primitives exist anywhere** in `frontend/src/`; `package.json` has no floating-ui or Radix dependency. Hand-roll matches the codebase's house style.
- **PR descriptions rename surface**: 12 visual-regression PNG baselines, 5 frontend test fixtures with `hasPr: false`, server enum variant + `serde(rename)`, frontend `DocTypeKey` union, `DOC_TYPE_LABELS`, `LIFECYCLE_PIPELINE_STEPS`, `PHASE_DOC_TYPES`, icon file rename, 3 CSS custom-property declarations, plus `Completeness.hasPr` (`api/types.ts:149`) and `LIFECYCLE_PIPELINE_STEPS` keys at `:177-209`.

## Desired End State

After this plan completes:

- `/library` renders a server-driven overview hub with phase-grouped doc-type cards (count + latest preview), supporting both light and dark themes and three responsive breakpoints. The previous redirect to `/library/decisions` is gone.
- Every doc-type list view shares a common `Page` wrapper (eyebrow + H1 + count subtitle + actions slot + content slot), renders five columns (`ID / DATE` / `TITLE` / `STATUS` / `SLUG` / `MODIFIED`) with status-as-Chip; the first column follows a per-row fallback chain (ID pill via `formatDocId(workItemId)` when non-null; else `formatDate(frontmatter.date)`; else em-dash). The list view is sorted via a sort pill (no column-header click-sort) and filtered via a doc-type-aware filter pill whose option counts respond to active selection (computed server-side per request).
- Doc-type-empty list views render a per-doc-type empty card with the indexer-aware footer; filter-applied-empty list views render a panel inside the table area with a `Clear filters` button while keeping the page chrome.
- `KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, and `LibraryTemplatesIndex` all consume the same `Page` wrapper. `PageSubtitle` is deleted. `RootLayout.main` no longer applies horizontal padding.
- `--ac-content-max-width` is a theme-invariant design token at `1100px`; `--ac-content-max-width-narrow` is its narrower (600px) counterpart. `LibraryTemplatesIndex` opts into the narrow variant via `<Page maxWidth="narrow">` — the prop is a closed union (`'default' | 'narrow'`), not a raw CSS string, so all page widths remain policed by `migration.test.ts`.
- `PHASE_DOC_TYPES` is removed; `Sidebar.tsx` consumes the new server structure response. `Completeness.hasPr` and `ClusterFlags::has_pr` are renamed to `hasPrDescription` / `has_pr_description`. The wire token `prs` becomes `pr-descriptions` everywhere it serialises; the on-disk `meta/prs/` directory and `config_path_key()` return value stay as `prs`.

### Verification:

- `make test` (or workspace equivalent) passes including all new component tests, hook tests, server unit tests, and updated router/Library tests.
- `make lint` / `cargo clippy` / `npm run typecheck` clean.
- Playwright visual-regression suite passes after baseline regeneration for the 12 renamed PR-descriptions snapshots and the new `/library` overview hub baseline.
- Manual smoke: navigating to `/library` shows the hub; clicking a doc-type card navigates to its list view; the sort pill and filter pill open menus and update the table; filtering to no-results shows the `Clear filters` panel; visiting a zero-document doc type shows the empty card; light/dark theme toggle preserves rendering; the sidebar phase grouping reflects the server response (changing the server response alters the rendered phases).

### Key Discoveries:

- Five-route `Page` migration is heterogeneous; `LifecycleIndex` (no header today) and `LibraryDocView` (header inside a 2-column grid as `grid-area: header`) are non-obvious and need extra care.
- `migration.test.ts:394-412` declares a `REQUIRED` list asserting per-route `.title { color: var(--ac-fg-strong) }`; once `Page` owns title styling, the three migrating routes' entries (`LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`) must come out of this list.
- `migration.test.ts:131-134` asserts exact literal counts for `LibraryTypeView.module.css` (2px×3, 1px×2, 0.4rem×1, 900px×1). Counts will shift as `.sortButton` and column-header borders are removed; allowlist must update or the test fails.
- `mtime_ms == 0` is a valid sentinel (see `clusters.rs:52` precedent using `.max().unwrap_or(0)`); the new `library_aggregates` follows this — does not filter zero out.
- `WorkItemConfig` has no project-prefix accessor; project-facet derivation splits `entry.work_item_id` on the first `-`, falling back to `cfg.work_item.default_project_code`.
- The `prs` rename surface includes `Completeness.hasPr` (`api/types.ts:149`) and `LIFECYCLE_PIPELINE_STEPS` entry keyed `'hasPr'` (`api/types.ts:192-193`) — both rename to `hasPrDescription` / pipeline-step `'hasPrDescription'` with label `'PR descriptions'`.
- All blocker work items (0033 token system, 0037 Glyph, 0038 Chip) are merged; `Glyph` colours are automatic via `--ac-doc-{key}` tokens (no colour prop), and `Chip` exposes `neutral|indigo|green|amber|red|violet` variants.

## What We're NOT Doing

- **No URL state for sort or filter.** The current `LibraryTypeView` has no `validateSearch`; URL-backed sort/filter is out of scope. Sort and filter live in component state. Selection IS sent to the server (as a query parameter to `/api/library/structure`) so the server can compute scoped facet counts — but the URL the user sees stays unchanged.
- **No `meta/prs/` directory rename.** The on-disk path stays. `config_path_key()` for the variant continues returning `Some("prs")`. Test fixtures and config JSON keys that key off `doc_paths.prs` stay as `prs`.
- **No `@floating-ui/react` / Radix / Headless UI dependency.** The popover primitive is hand-rolled with a co-located `useDismiss` hook, matching the codebase's hand-rolled small-primitives house style.
- **No new frontmatter conventions.** `cluster_slug` and `project` are server-derived from existing `IndexEntry.slug` and `IndexEntry.work_item_id`. Status remains the existing `frontmatter["status"]` field.
- **No `LifecycleClusterView` migration.** It is not in the five-route list; its `800px` allowlist entry at `migration.test.ts:147` stays untouched.
- **No `make` target additions or test-runner changes.** All verification uses existing `npm test` / `cargo test` / `npx playwright test` invocations.
- **No caching of the library-structure response.** Recompute on each request, mirroring `api/types::types`. If caching is later desired, the precedent is `state.clusters` (`server.rs:46,82`) — defer. Add an inline `// PERF:` comment on `library_aggregates` documenting the recompute strategy and the threshold at which to migrate to the cached pattern (suggested trigger: p95 handler latency > 50ms, or entry count > ~10k).
- **No authoring UI for empty-state copy.** Per-doc-type description sentences are hard-coded in the frontend (`empty-descriptions.ts`); no wire field is reserved (the previous draft's `empty_description: Option<String>` field has been dropped to avoid shipping unused wire surface).
- **No JJ/Git-time alternative to filesystem mtime** for the "latest" preview. `IndexEntry.mtime_ms` is canonical, including the zero-sentinel.

## Implementation Approach

Six phases, each independently testable and shippable. Earlier phases produce primitives that later phases consume; the dependency graph is linear.

- **Phase 1 (Foundation primitives)** introduces the `--ac-content-max-width` design token, the `Page` wrapper component, and the `Popover` primitive with `useDismiss` hook. No consumer migrations yet — everything tested in isolation.
- **Phase 2 (Server library-structure endpoint)** adds `Indexer::library_aggregates`, a new `/api/library/structure` handler, and the matching frontend types + fetcher. Server-side first because Phase 4's overview hub and Phase 5's filter pill both consume it.
- **Phase 3 (PR descriptions rename)** is a coordinated frontend+server rename. Sequenced before consumers (Phases 4–6) so the new wire token, the `hasPrDescription` field, and the icon rename land before any UI that depends on them.
- **Phase 4 (Library overview hub + Sidebar migration + redirect removal)** consumes Phases 1 and 2 to build `LibraryOverviewHub`, retire `PHASE_DOC_TYPES`, migrate `Sidebar` to the server shape, and update `router.ts` + `router.test.tsx`.
- **Phase 5 (List view refactor + sort + filter + empty states)** consumes Phases 1, 2, and 3 to refactor `LibraryTypeView`, build the `SortPill` and `FilterPill`, and ship both empty-state variants.
- **Phase 6 (Five-route Page migration + cleanup)** migrates `KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, and `LibraryTemplatesIndex` to `Page`, deletes `PageSubtitle`, strips `RootLayout.main`'s padding rule, and updates `migration.test.ts` allowlists and the title-color `REQUIRED` list. Last because it is the most invasive UI migration and must land atomically with the `RootLayout` change.

Test-driven development applies throughout: each new component, hook, and server handler is preceded by a failing test that captures the contract, then implemented to satisfy the test. Migration steps that touch multiple files (Phase 3, Phase 6) drive each refactor with the existing test suite as a safety net, updating tests in lock-step with the source.

## Phase 1: Foundation Primitives

### Overview

Introduce the `--ac-content-max-width` design token, the `Page` wrapper component, and the `Popover` + `useDismiss` primitive. No consumers yet — each primitive ships with its own test suite.

### Changes Required:

#### 1. `--ac-content-max-width` and `--ac-content-max-width-narrow` design tokens

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: Add `'ac-content-max-width': '1100px'` and `'ac-content-max-width-narrow': '600px'` to the `LAYOUT_TOKENS` group (theme-invariant — single value, not split light/dark).

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: Add `--ac-content-max-width: 1100px;` and `--ac-content-max-width-narrow: 600px;` to the `:root` block only — `LAYOUT_TOKENS` are checked via `readCssVar(name, 'root')` in `global.test.ts:85-97`, matching the precedent of `--ac-topbar-h` which is declared only in `:root`. Do NOT add to the dark theme blocks (theme-invariant layout tokens belong only in `:root`).

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
**Changes**: Token parity test already iterates all `LAYOUT_TOKENS` keys against `:root`; the new keys are picked up automatically. Verify no fixture pinning the key list needs updating.

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: No allowlist change here yet. Allowlist entries for the five routes' max-width literals are removed in Phases 5 (LibraryTypeView) and 6 (the other four routes) as each route migrates to consume the token via `Page`.

#### 2. `Page` wrapper component (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/components/Page/Page.test.tsx`

Tests covering the contract:
- Renders eyebrow row when `eyebrow` prop is given (text + optional `Glyph`); omits the row when not.
- Renders required `title` as `<h1>`. When the consuming route's data is still loading, callers pass a placeholder string (e.g. `'Loading…'`) — do NOT pass `undefined`/`null`.
- Renders optional `subtitle` slot below H1 when provided; omits when not.
- Renders optional `actions` slot, right-aligned, when provided; omits when not.
- Renders children inside the content area, separated from header by a horizontal rule.
- Default `maxWidth` consumes `var(--ac-content-max-width)`. Honours `maxWidth="narrow"` by consuming `var(--ac-content-max-width-narrow)` instead.
- Applies horizontal padding `var(--sp-6)` and vertical spacing `var(--sp-5)` between header and content.
- Token-binding assertions use the established `*.module.css?raw` source-string pattern (e.g. `expect(pageCss).toMatch(/\.page\s*\{[^}]*max-width:\s*var\(--ac-content-max-width\)/)`), NOT `getComputedStyle`. jsdom returns the declared value (`var(...)`), not the resolved length, so `getComputedStyle` is unreliable for CSS custom properties. DOM-render assertions cover slot presence/absence only.

**New file**: `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css`

Styles:
- `.page { max-width: var(--ac-content-max-width); margin: 0 auto; padding: 0 var(--sp-6); }`
- `.page.narrow { max-width: var(--ac-content-max-width-narrow); }`
- `.header { display: flex; flex-direction: column; gap: var(--sp-2); padding-block: var(--sp-5); }`
- `.headerTopRow { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--sp-4); }`
- `.eyebrow { font-size: var(--ac-text-eyebrow); text-transform: uppercase; color: var(--ac-fg-muted); display: inline-flex; align-items: center; gap: var(--sp-2); }`
- `.title { color: var(--ac-fg-strong); margin: 0; }`
- `.subtitle { color: var(--ac-fg-muted); }`
- `.actions { display: inline-flex; gap: var(--sp-3); align-items: center; }`
- `.divider { border: 0; border-top: 1px solid var(--ac-stroke); margin: 0 0 var(--sp-5) 0; }`
- `.content {}` (no inherent style; consumers fill the slot)

**New file**: `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx`

```tsx
import type { ReactNode } from 'react'
import styles from './Page.module.css'

export interface PageProps {
  eyebrow?: ReactNode
  title: ReactNode
  subtitle?: ReactNode
  actions?: ReactNode
  maxWidth?: 'default' | 'narrow'
  children: ReactNode
}

export function Page({ eyebrow, title, subtitle, actions, maxWidth = 'default', children }: PageProps) {
  const className = maxWidth === 'narrow' ? `${styles.page} ${styles.narrow}` : styles.page
  return (
    <section className={className}>
      <header className={styles.header}>
        {eyebrow !== undefined && (
          <div className={styles.eyebrow}>{eyebrow}</div>
        )}
        <div className={styles.headerTopRow}>
          <div>
            <h1 className={styles.title}>{title}</h1>
            {subtitle !== undefined && (
              <div className={styles.subtitle} data-slot="subtitle">{subtitle}</div>
            )}
          </div>
          {actions !== undefined && (
            <div className={styles.actions} data-slot="actions">{actions}</div>
          )}
        </div>
      </header>
      <hr className={styles.divider} />
      <div className={styles.content}>{children}</div>
    </section>
  )
}
```

Note: `maxWidth` is intentionally typed as a closed union of named variants, not an arbitrary CSS length. This keeps the design tokens (`--ac-content-max-width` and `--ac-content-max-width-narrow`) as the single source of truth for layout max-widths and ensures every page width is policed by `migration.test.ts`.

#### 3. `useDismiss` hook (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/components/Popover/use-dismiss.test.ts`

Tests:
- Hook returns nothing; binds listeners on mount when `open === true`.
- `mousedown` outside the referenced element triggers `onDismiss`.
- `mousedown` inside the referenced element does not trigger `onDismiss`.
- `keydown` Escape triggers `onDismiss` regardless of focus.
- Listeners are removed on unmount and when `open` flips to `false`.
- Multiple instances stack independently (Sort pill and Filter pill open simultaneously dismiss independently).

**New file**: `skills/visualisation/visualise/frontend/src/components/Popover/use-dismiss.ts`

```tsx
import { useEffect, type RefObject } from 'react'

export function useDismiss(
  open: boolean,
  ref: RefObject<HTMLElement | null>,
  onDismiss: () => void,
) {
  useEffect(() => {
    if (!open) return
    const onMouseDown = (event: MouseEvent) => {
      const target = event.target as Node | null
      if (target && ref.current && ref.current.contains(target)) return
      onDismiss()
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onDismiss()
    }
    document.addEventListener('mousedown', onMouseDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('mousedown', onMouseDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open, ref, onDismiss])
}
```

#### 4. `Popover` primitive (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/components/Popover/Popover.test.tsx`

Tests use `@testing-library/user-event` for keyboard interaction; positioning assertions stub `Element.prototype.getBoundingClientRect` to return a known rect (jsdom returns all zeros by default). Token-binding assertions read `Popover.module.css?raw` rather than `getComputedStyle`.

Tests:
- Renders a trigger child and a panel child slot; trigger is always present.
- Panel has `[hidden]` attribute / `display: none` when `open === false`; visible when `open === true`.
- Clicking outside closes the popover (calls `onOpenChange(false)`).
- Pressing Escape closes the popover.
- Pressing Tab while open closes the popover and allows default tab order to proceed.
- Panel positions absolutely below the trigger: with a stubbed `getBoundingClientRect`, asserts that `style.top` and `style.left` are set to expected pixel values (trigger bottom + 4px offset; trigger left).
- Arrow key navigation (Down/Up/Home/End) moves focus among `[role="menuitem"]` / `[role="menuitemcheckbox"]` descendants.
- Enter and Space both activate the focused item (matches the WAI-ARIA menu-button pattern).
- Open transfers focus to first menuitem; close returns focus to trigger. Focus return is guarded by `if (triggerRef.current)` and runs only on the `open: true → false` transition so React 18 StrictMode double-invocation does not drop focus.
- Trigger element exposes `aria-haspopup="menu"`, `aria-expanded={open}`, and `aria-controls={panelId}`; panel has `id={panelId}` and `role="menu"`.
- Opening a second `Popover` while another is open dismisses the first (test renders two siblings; clicking the second trigger asserts the first's `onOpenChange(false)` was called).
- Typeahead is intentionally out of scope at this level; the FilterPill's `enum-with-search` variant handles type-to-filter via its own search input.

**New file**: `skills/visualisation/visualise/frontend/src/components/Popover/Popover.module.css`

Styles:
- `.popover { position: relative; display: inline-block; }`
- `.panel { position: absolute; z-index: 50; background: var(--ac-bg-card); border: 1px solid var(--ac-stroke); border-radius: var(--radius-md); padding: var(--sp-3); box-shadow: var(--shadow-pop); min-width: 240px; }`
- `.panel[hidden] { display: none; }`

**New file**: `skills/visualisation/visualise/frontend/src/components/Popover/Popover.tsx`

API:
```tsx
export interface PopoverTriggerProps {
  ref: React.RefObject<HTMLElement | null>
  'aria-haspopup': 'menu'
  'aria-expanded': boolean
  'aria-controls': string
  onClick: () => void
}

export interface PopoverProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  // Render-prop pattern: consumer spreads triggerProps onto its trigger element.
  // This is how SortPill / FilterPill wire the trigger ARIA attributes through.
  trigger: (triggerProps: PopoverTriggerProps) => ReactNode
  children: ReactNode               // the panel content (menu items)
  ariaLabel?: string                // for the panel role=menu
}
```

Implementation responsibilities:
- Generate a stable `panelId` via `useId()`; assign to the panel `id` and pass via `aria-controls`.
- Compose `triggerProps` (ref, aria-haspopup, aria-expanded, aria-controls, onClick toggling `open`) and hand them to the `trigger` render prop.
- Wrap trigger and panel in a positioning shell.
- Use `useDismiss(open, panelRef, () => onOpenChange(false))`.
- On open, focus the first `[role="menuitem"]` or `[role="menuitemcheckbox"]` inside the panel; on close, return focus to the trigger element (guarded by `if (triggerRef.current)` and gated on the `open: true → false` transition).
- Bind keydown handler on the panel for Up/Down/Home/End/Enter/Space/Tab; let consumers render `role="menuitem"` or `role="menuitemcheckbox"` rows that the keydown handler walks. Tab closes the popover and lets the browser's default tab order proceed.
- Position the panel using `getBoundingClientRect` on the trigger; default placement is bottom-start with a 4px offset.
- Register opening with a module-level "active popover" pointer (a simple `let activePopover: (() => void) | null = null` in `Popover.tsx`): when one Popover opens, it dismisses any previously-active one by calling its `onOpenChange(false)` before setting itself as active. Cleared on close.

### Success Criteria:

#### Automated Verification:

- [ ] Token parity holds: `cd skills/visualisation/visualise/frontend && npm test -- styles/global.test.ts`
- [ ] Page tests pass: `npm test -- components/Page`
- [ ] useDismiss tests pass: `npm test -- components/Popover/use-dismiss`
- [ ] Popover tests pass: `npm test -- components/Popover/Popover`
- [ ] Type check clean: `npm run typecheck`
- [ ] No lint errors: `npm run lint`
- [ ] Migration test still passes (no consumer migrations yet): `npm test -- styles/migration.test.ts`

#### Manual Verification:

- [ ] Render `Page` in a temporary scratch route; visually confirm header layout, divider, max-width centring, and `--ac-content-max-width` value matches `1100px` in computed styles.
- [ ] Render a `Popover` with three menu items in a scratch route; confirm click-outside, Escape, arrow navigation, and focus-on-open behaviour.

---

## Phase 2: Server Library-Structure Endpoint

### Overview

Add `Indexer::library_aggregates(cfg, selection)` (single-pass selection-aware aggregator producing counts, latest-per-type, and per-facet option counts under one `entries.read().await`); add `/api/library/structure` handler returning the phase-grouped structure (plus a top-level `templates` entry for the virtual templates doc type); add matching frontend types and `fetchLibraryStructure(selection)`. The endpoint accepts an optional `selection` query parameter encoding the user's active facet selections; option counts are computed using the "post-other-facet, pre-own-facet" scoping rule.

### Changes Required:

#### 1. `Indexer::library_aggregates` (TDD: tests first)

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Add a new method alongside `counts_by_type` (`:326-332`). `LibraryAggregates` colocates everything per doc type rather than splitting across parallel maps:

```rust
use std::collections::{BTreeMap, HashMap};

/// Active facet selection, scoped per doc type. Empty selection ⇒ no filtering.
/// Keyed by doc type, then by facet id (e.g. "status", "clusterSlug", "project").
/// Values are the selected option ids for that facet (OR within a facet, AND across facets).
pub type Selection = HashMap<DocTypeKey, HashMap<String, Vec<String>>>;

#[derive(Default)]
pub struct LibraryAggregates {
    pub per_type: HashMap<DocTypeKey, PerTypeAggregate>,
}

#[derive(Default)]
pub struct PerTypeAggregate {
    pub count: usize,                     // total entries (selection-unaware; for hub/overview)
    pub filtered_count: usize,            // entries matching this type's selection (used for list-view "N documents")
    pub latest: Option<LatestPreview>,    // selection-unaware: hub card always shows the absolute latest
    pub facet_options: HashMap<String, BTreeMap<String, usize>>,
    // facet_options[facet_id] => sorted map of option-id → count.
    // Counts are computed with post-other-facet, pre-own-facet scoping per facet id.
}

#[derive(Default)]
pub struct LatestPreview {
    pub title: String,
    pub slug: Option<String>,
    pub rel_path: String,                 // used as deterministic tie-break key
    pub modified_at: i64,
}

impl LatestPreview {
    fn from_entry(entry: &IndexEntry) -> Self {
        Self {
            title: entry.title.clone(),
            slug: entry.slug.clone(),
            rel_path: entry.rel_path.clone(),
            modified_at: entry.mtime_ms,
        }
    }
}

impl Indexer {
    /// Computes counts, the latest entry, and facet-option counts per doc type.
    /// Facet counts use post-other-facet, pre-own-facet scoping: for each facet,
    /// count over the set of entries matching every OTHER facet's selection
    /// (so toggling a value in facet B updates facet A's counts but does not
    /// hide facet A's own currently-selected option).
    pub async fn library_aggregates(
        &self,
        cfg: &crate::config::Config,
        selection: &Selection,
    ) -> LibraryAggregates {
        let entries = self.entries.read().await;
        let mut agg = LibraryAggregates::default();

        for entry in entries.values() {
            let per = agg.per_type.entry(entry.r#type).or_default();
            per.count += 1;

            // latest: largest mtime_ms; deterministic tie-break on smallest rel_path
            let preview = LatestPreview::from_entry(entry);
            per.latest = Some(match per.latest.take() {
                None => preview,
                Some(existing) => {
                    if preview.modified_at > existing.modified_at
                        || (preview.modified_at == existing.modified_at
                            && preview.rel_path < existing.rel_path)
                    {
                        preview
                    } else {
                        existing
                    }
                }
            });
        }

        // Facet computation: iterate entries again per doc type with selection scoping.
        // Done in a second pass for clarity; still under the same read lock.
        for (doc_type, per) in agg.per_type.iter_mut() {
            let type_selection = selection.get(doc_type);
            let type_entries: Vec<_> = entries.values().filter(|e| e.r#type == *doc_type).collect();

            // filtered_count: entries matching ALL selections for this doc type
            per.filtered_count = type_entries
                .iter()
                .filter(|e| entry_matches_all(e, cfg, type_selection))
                .count();

            // For each facet declared for this doc type, compute option counts using
            // post-other-facet scoping (filter by every facet except this one).
            for facet_id in facets_for(*doc_type) {
                let mut option_counts: BTreeMap<String, usize> = BTreeMap::new();
                for entry in &type_entries {
                    if !entry_matches_all_except(entry, cfg, type_selection, facet_id) {
                        continue;
                    }
                    if let Some(option_id) = extract_facet_value(entry, cfg, facet_id) {
                        *option_counts.entry(option_id).or_insert(0) += 1;
                    }
                }
                per.facet_options.insert(facet_id.to_string(), option_counts);
            }
        }

        agg
    }
}

// Helpers (also in indexer.rs; trivial pure functions, separately testable):
//
//   fn facets_for(doc_type: DocTypeKey) -> &'static [&'static str]
//     - ADR-style types (decisions, plans, plan-reviews, pr-reviews,
//       work-item-reviews, validations, design-gaps, design-inventories,
//       notes, pr-descriptions): &["status", "clusterSlug"]
//     - work-items: &["status", "project", "clusterSlug"]
//     - templates: &[] (virtual; emits no facets)
//
//   fn extract_facet_value(entry, cfg, facet_id) -> Option<String>
//     - "status": entry.frontmatter.get("status").and_then(as_str)
//                 when entry.frontmatter_state == "parsed"; else None
//     - "clusterSlug": entry.slug.clone() (None when the entry has no slug)
//     - "project": when entry.work_item_id is Some:
//                    split at first '-', filter non-empty prefix,
//                    else cfg.work_item.default_project_code
//                  when entry.work_item_id is None:
//                    return None (entry has no project; does not contribute to any
//                    project bucket and does not get the default project code —
//                    the default applies only to malformed/prefixless work_item_ids,
//                    not to entries that lack a work_item_id entirely)
//
//   fn entry_matches_all(entry, cfg, type_selection) -> bool
//     - true iff for every (facet_id, selected_options) in type_selection,
//       extract_facet_value(entry, cfg, facet_id) is in selected_options.
//       Empty selected_options for a facet ⇒ no filter for that facet.
//
//   fn entry_matches_all_except(entry, cfg, type_selection, except_facet) -> bool
//     - same as entry_matches_all but skips the facet equal to `except_facet`.
```

**Test file**: `skills/visualisation/visualise/server/src/indexer.rs` (existing inline tests at the bottom)
**Changes**: Add tests covering:
- Empty index returns an empty `per_type` map.
- Multiple entries per doc type aggregate `count` correctly.
- `latest` reflects the maximum `mtime_ms` per type and includes the right title/slug.
- `mtime_ms == 0` does not get filtered out (matches `clusters.rs` `.unwrap_or(0)` precedent).
- Multiple entries with identical `mtime_ms` (including the zero-sentinel case) tie-break deterministically on the smallest `rel_path`.
- `status` facet skips entries with `frontmatter_state` other than `"parsed"`.
- `clusterSlug` facet groups by `entry.slug`.
- `project` facet derives prefix from `work_item_id` (e.g. `"PROJ-0042"` → `"PROJ"`); falls back to `cfg.work_item.default_project_code` when no `-` is present; rejects empty-prefix IDs like `"-0042"`; returns `None` (no project bucket) when `work_item_id` is `None`.
- `entry_matches_all` / `entry_matches_all_except` with an empty option array for a facet (e.g. `{status: []}`) returns `true` for every entry — matching the documented contract that an empty selected_options list means "no filter for that facet". This is the canonical state after a user deselects every option without invoking Clear filters.
- With no selection, `filtered_count` equals `count` for every doc type.
- With selection `{decisions: {status: ["open"]}}`, `filtered_count` for decisions reflects only open entries; counts for the `status` facet itself are still the universe of open/blocked/etc (pre-own-facet), but counts for the `clusterSlug` facet are scoped to open entries only (post-other-facet).
- `facets_for(DocTypeKey::Templates)` returns the empty slice; aggregator emits an empty `facet_options` for templates.
- Per-helper tests for `facets_for`, `extract_facet_value`, `entry_matches_all`, and `entry_matches_all_except` (pure functions, no lock acquisition).

#### 2. `/api/library/structure` handler (TDD: integration test first)

**New file**: `skills/visualisation/visualise/server/src/api/library.rs`

Module structure:
```rust
use std::sync::Arc;
use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use crate::server::AppState;
use crate::docs::DocTypeKey;
use crate::indexer::Selection;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStructureResponse {
    pub phases: Vec<Phase>,
    /// Virtual templates entry, emitted at the top level (templates has no phase).
    pub templates: LibraryDocType,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phase {
    pub id: String,
    pub label: String,
    pub doc_types: Vec<LibraryDocType>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocType {
    pub id: DocTypeKey,
    pub label: String,
    pub count: usize,                       // selection-unaware total (for hub cards)
    pub filtered_count: usize,              // entries matching the active selection (for list-view "N documents")
    pub latest: Option<LatestPreviewWire>,
    pub filter_facets: Vec<Facet>,
}
// Note: `glyphId` and `route` are intentionally omitted. Frontend uses `id` for both
// (Glyph is keyed on DocTypeKey; Link to="/library/$type" params={{ type: id }}).
// `empty_description` is also omitted — descriptions are owned by the frontend
// (see Phase 5's empty-descriptions.ts) and we don't ship reserved-but-unused fields.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestPreviewWire {
    pub title: String,
    pub slug: Option<String>,
    pub modified_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facet {
    pub id: String,                         // camelCase facet id (e.g. "status", "clusterSlug", "project")
    pub label: String,
    pub options: Vec<FacetOption>,
}
// Note: `FacetKind` is intentionally omitted from the wire. Whether to render
// a search input above the option list (the "> 8 options" rule) is a presentation
// concern computed client-side inside FilterPill.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetOption {
    pub id: String,
    pub label: String,
    pub count: usize,
}

/// Query string carries the active selection so the server can compute scoped
/// facet counts. Encoding: repeated keys of the form
/// `selection[<docType>][<facetId>]=optA`. Multiple options on the same facet
/// repeat the key. Empty / missing selection ⇒ unconditional totals.
///
/// We do NOT use serde's `Query<T>` here because `serde_urlencoded` (which axum's
/// `Query` extractor uses) treats `selection[decisions][status]` as a single flat
/// key and does NOT decompose the bracket syntax into a nested map. Instead we
/// take `RawQuery` and parse it manually via `parse_selection_query`.

pub(crate) async fn library_structure(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> Json<LibraryStructureResponse> {
    let selection = parse_selection_query(raw.as_deref().unwrap_or(""));
    let agg = state.indexer.library_aggregates(&state.cfg, &selection).await;
    Json(build_structure(&state.cfg, &agg))
}

/// Parses repeated query keys of the form `selection[<type>][<facet>]=<option>`
/// into a `Selection`. Robust to URL-encoded values; ignores keys that don't
/// match the expected shape. Repeated keys for the same `[type][facet]` append
/// to that facet's selected-option list (OR within a facet).
///
/// Examples:
///   ""                                                    => empty Selection
///   "selection[decisions][status]=open"                   => {decisions: {status: ["open"]}}
///   "selection[decisions][status]=open&selection[decisions][status]=blocked"
///                                                         => {decisions: {status: ["open", "blocked"]}}
///   "selection[decisions][status]=open&selection[decisions][clusterSlug]=foo"
///                                                         => {decisions: {status: ["open"], clusterSlug: ["foo"]}}
///   "selection[bogus][status]=open"                       => empty (unknown doc-type silently dropped)
///   "selection[decisions][status]="                       => empty value silently dropped (no filter)
fn parse_selection_query(raw: &str) -> Selection {
    let mut out: Selection = HashMap::new();
    for (key, value) in form_urlencoded::parse(raw.as_bytes()) {
        // key is already URL-decoded by form_urlencoded; expect literal brackets.
        // Shape: selection[<type-wire-token>][<facetId>]
        let Some(rest) = key.strip_prefix("selection[") else { continue };
        let Some((type_token, rest)) = rest.split_once(']') else { continue };
        let Some(rest) = rest.strip_prefix('[') else { continue };
        let Some((facet_id, tail)) = rest.split_once(']') else { continue };
        if !tail.is_empty() { continue; }       // anything after the second `]` is malformed
        if value.is_empty() { continue; }       // empty value: documented as no filter

        let Some(doc_type) = DocTypeKey::from_wire_str(type_token) else { continue };
        out.entry(doc_type)
            .or_default()
            .entry(facet_id.to_string())
            .or_default()
            .push(value.into_owned());
    }
    out
}

// --- Decomposed helpers (each independently testable) ---

/// Static phase membership. Move to typed config (`config.toml`) in a future story
/// if/when phase configuration needs to be data-driven; for now this centralises
/// the previous client-side `PHASE_DOC_TYPES` table.
const PHASES: &[(&str, &str, &[DocTypeKey])] = &[
    ("define",   "DEFINE",   &[/* DocTypeKey variants per phase */]),
    ("discover", "DISCOVER", &[/* ... */]),
    ("build",    "BUILD",    &[/* ... */]),
    ("ship",     "SHIP",     &[/* ... */]),
    ("remember", "REMEMBER", &[/* ... */]),
];

fn build_structure(
    cfg: &crate::config::Config,
    agg: &crate::indexer::LibraryAggregates,
) -> LibraryStructureResponse {
    let phases = PHASES.iter().map(|(id, label, doc_types)| Phase {
        id: id.to_string(),
        label: label.to_string(),
        doc_types: doc_types.iter()
            .map(|dt| build_doc_type(cfg, agg, *dt))
            .collect(),
    }).collect();

    LibraryStructureResponse {
        phases,
        templates: build_doc_type(cfg, agg, DocTypeKey::Templates),
    }
}

fn build_doc_type(
    cfg: &crate::config::Config,
    agg: &crate::indexer::LibraryAggregates,
    doc_type: DocTypeKey,
) -> LibraryDocType {
    let per = agg.per_type.get(&doc_type);
    LibraryDocType {
        id: doc_type,
        label: doc_type.label().to_string(),
        count: per.map(|p| p.count).unwrap_or(0),
        filtered_count: per.map(|p| p.filtered_count).unwrap_or(0),
        latest: per.and_then(|p| p.latest.as_ref()).map(LatestPreviewWire::from),
        filter_facets: build_facets(cfg, per, doc_type),
    }
}

fn build_facets(
    cfg: &crate::config::Config,
    per: Option<&crate::indexer::PerTypeAggregate>,
    doc_type: DocTypeKey,
) -> Vec<Facet> {
    let per = match per { Some(p) => p, None => return Vec::new() };
    crate::indexer::facets_for(doc_type).iter().map(|facet_id| Facet {
        id: facet_id.to_string(),
        label: facet_label(facet_id).to_string(),
        options: per.facet_options.get(*facet_id)
            .map(|m| m.iter().map(|(id, count)| FacetOption {
                id: id.clone(),
                label: facet_option_label(cfg, doc_type, facet_id, id),
                count: *count,
            }).collect())
            .unwrap_or_default(),
    }).collect()
}

fn facet_label(facet_id: &str) -> &'static str {
    match facet_id {
        "status" => "Status",
        "clusterSlug" => "Cluster",
        "project" => "Project",
        _ => facet_id,
    }
}

// `facet_option_label`: humanises option ids (e.g. status "open" → "Open").
// `parse_selection_query`: parses the repeated-key query syntax (see definition above).
```

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`
**Changes**:
- Add `mod library;` (private — matches `lifecycle`, `templates`, `types`; only `info` is `pub(crate)`) at the alphabetical position between `kanban_config` and `lifecycle`.
- Add `.route("/api/library/structure", get(library::library_structure))` after `/api/types` in `mount()`.

**Test file**: `skills/visualisation/visualise/server/tests/api_library_structure.rs` (new)

Tests against the existing fixture index:
- Response is JSON-shaped per `LibraryStructureResponse`; phases array is non-empty; top-level `templates` field is present.
- Phase order matches DEFINE / DISCOVER / BUILD / SHIP / REMEMBER.
- Doc-type counts match `Indexer::counts_by_type` for the same fixture.
- With no `selection` query string, `filtered_count == count` for every doc type.
- `latest` is `null` for zero-count doc types and populated otherwise.
- `filter_facets` for `decisions` is `[status, clusterSlug]`; for `work-items` is `[status, project, clusterSlug]`; for `templates` (top-level) is empty.
- Facet option threshold boundary cases: build fixtures producing exactly 7, 8, and 9 unique option ids; the wire response has no `kind` field — the test asserts only that `options` arrays of the right lengths are emitted. (The "render search input above 8" rule lives client-side in `FilterPill`; see Phase 5 tests.)
- Selection scoping: with `?selection[decisions][status]=open,blocked`, assert (a) `filtered_count` for decisions reflects only open/blocked entries; (b) the `status` facet options under decisions still list every status (post-other-facet, pre-own-facet); (c) the `clusterSlug` facet options under decisions list only clusters touched by open/blocked entries.
- Selection scoping with two facets: `?selection[decisions][status]=open&selection[decisions][clusterSlug]=foo` — the `status` facet shows counts scoped to cluster foo; the `clusterSlug` facet shows counts scoped to status open.
- Selection round-trip via real HTTP request: invoke the handler through the test client with a multi-option selection (`?selection[decisions][status]=open&selection[decisions][status]=blocked&selection[decisions][clusterSlug]=foo`) and assert that the parsed `Selection` reaches the aggregator with the expected shape. This pins the actual axum/encoding contract end-to-end (not just the helper).
- Wire-token contract: per-variant `serde_json::to_value(DocTypeKey::*)` matches the expected kebab-case strings (pinned for all variants, not just `PrDescriptions`).

**`parse_selection_query` unit tests** (in `server/src/api/library.rs` inline tests):
- Empty string → empty `Selection`.
- `selection[decisions][status]=open` → `{decisions: {status: ["open"]}}`.
- Repeated key for same `[type][facet]` accumulates: `selection[decisions][status]=open&selection[decisions][status]=blocked` → `{decisions: {status: ["open", "blocked"]}}` (order preserved).
- Two facets under one doc type cross-populate: `selection[decisions][status]=open&selection[decisions][clusterSlug]=foo` → `{decisions: {status: ["open"], clusterSlug: ["foo"]}}`.
- Empty value silently dropped (`selection[decisions][status]=` → empty Selection; documented as "no filter for that facet").
- URL-encoded values round-trip: `selection[decisions][clusterSlug]=foo%20bar` → `{decisions: {clusterSlug: ["foo bar"]}}` (form_urlencoded handles the decode).
- Option ids containing reserved characters (commas, ampersands, equals signs) round-trip correctly when percent-encoded: `selection[decisions][clusterSlug]=a%2Cb` → `{decisions: {clusterSlug: ["a,b"]}}`. Unencoded commas in values are accepted literally (no special separator semantics).
- Unknown doc-type key silently dropped: `selection[bogus][status]=open` → empty Selection.
- Malformed shape silently dropped: `selection[decisions]=open` (missing facet bracket), `selectionopen` (no bracket), `[decisions][status]=open` (missing `selection` prefix) — all return empty Selection.
- Unrelated query params ignored: `other=value&selection[decisions][status]=open` → only the selection part is parsed.
- Duplicate same option preserves the duplicate (`status=open&status=open` → `["open", "open"]`) — server's `entry_matches_all` treats duplicates as the same set, so behaviour is correct; documenting that dedup is the caller's responsibility.

#### 3. Frontend types and fetcher (TDD: tests first)

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: Add new types mirroring the server response. Do **not** delete `PHASE_DOC_TYPES` yet (Phase 4 deletes it).

```ts
export interface LibraryStructureResponse {
  phases: LibraryPhase[]
  templates: LibraryDocType
}

export interface LibraryPhase {
  id: string
  label: string
  docTypes: LibraryDocType[]
}

export interface LibraryDocType {
  id: DocTypeKey
  label: string
  count: number                  // total entries (selection-unaware)
  filteredCount: number          // entries matching the active selection
  latest: LatestPreview | null
  filterFacets: LibraryFacet[]
}

export interface LatestPreview {
  title: string
  slug: string | null
  modifiedAt: number
}

export interface LibraryFacet {
  id: string                     // camelCase: "status" | "clusterSlug" | "project"
  label: string
  options: LibraryFacetOption[]
}

export interface LibraryFacetOption {
  id: string
  label: string
  count: number
}

/// Selection state for one or more doc types. Keyed by doc type, then by facet id;
/// values are arrays of selected option ids (OR within a facet, AND across facets).
/// Empty arrays / missing keys ⇒ no filter for that facet.
export type LibrarySelection = Partial<Record<DocTypeKey, LibrarySelectionPerType>>

/// Per-doc-type selection slice. This is what `FilterPill` consumes — the
/// per-type slice of `LibrarySelection`, not a separate type. Keeps the wire
/// shape and the FilterPill API on a single source of truth.
export type LibrarySelectionPerType = Record<string, string[]>
```

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.ts`
**Changes**: Add `fetchLibraryStructure(selection?: LibrarySelection): Promise<LibraryStructureResponse>` paralleling `fetchDocs`. Encodes the optional `selection` as **repeated** query keys: for each `(docType, facetId, optionId)` triple, append `selection[<docType>][<facetId>]=<optionId>` via `URLSearchParams.append` (which produces repeated keys natively). Empty arrays for a facet and empty per-type objects are normalised away before encoding so the URL is canonical (matching the cache-key normalisation in `query-keys.ts`). When `selection` is empty/undefined the query string is omitted entirely.

Example encodings:
- `fetchLibraryStructure()` → `GET /api/library/structure`
- `fetchLibraryStructure({decisions: {status: ['open']}})` → `GET /api/library/structure?selection%5Bdecisions%5D%5Bstatus%5D=open`
- `fetchLibraryStructure({decisions: {status: ['open', 'blocked']}})` → `…?selection%5Bdecisions%5D%5Bstatus%5D=open&selection%5Bdecisions%5D%5Bstatus%5D=blocked`
- Option ids containing reserved characters are percent-encoded by `URLSearchParams.append` automatically.

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`
**Changes**: Add tests asserting:
- (a) `fetchLibraryStructure()` with no args hits `GET /api/library/structure` with no query string.
- (b) With a single-option selection, the URL contains one `selection[type][facet]=value` parameter.
- (c) With a multi-option selection, the URL contains repeated keys (not comma-separated).
- (d) Empty arrays for a facet are omitted from the URL (normalised away).
- (e) Empty per-type objects (e.g. `{decisions: {}}`) are omitted.
- (f) Option ids with reserved characters (`,`, `&`, `=`) are percent-encoded.
- (g) The normalised URL for `{decisions: {status: []}}` matches the URL for `{}` — single canonical form.

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`
**Changes**: Add `libraryStructure: (selection?: LibrarySelection) => ['library-structure', normaliseSelection(selection)] as const`. The selection is part of the cache key so React Query correctly invalidates when the user toggles a facet option. Follows the existing function-returning-readonly-tuple convention.

`normaliseSelection(selection)` is a small helper (co-located in `query-keys.ts` and exported) that produces a canonical form. The contract:
1. Drop facet entries whose option array is empty.
2. Drop facet entries whose value is `undefined` (legal under `Partial<Record<...>>`).
3. Drop per-type objects that are empty after step 1.
4. Drop per-type entries whose value is `undefined`.
5. Sort each remaining option array ascending (`Array.prototype.sort()`) — option order does not affect the server-side semantics (`entry_matches_all` treats selected options as a set), so two semantically-equivalent selections must hash to the same React Query cache key.
6. Reconstruct the object with facet keys in sorted order within each per-type slice, and doc-type keys in sorted order at the top level. Use `JSON.parse(JSON.stringify(...))` after building from sorted entries, or build manually via sorted-key iteration.
7. Return `{}` when there's nothing left.

This guarantees `libraryStructure()`, `libraryStructure({})`, `libraryStructure({decisions: {status: []}})`, and `libraryStructure({decisions: {status: ['open', 'blocked']}})` vs `libraryStructure({decisions: {status: ['blocked', 'open']}})` all hash to the canonical form — preventing duplicate fetches between (a) `RootLayout`'s unscoped fetch and `LibraryTypeView`'s scoped-but-currently-empty fetch, and (b) two components that toggle the same selection in different orders.

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.test.ts`
**Changes**: Cover the new key:
- Empty selection ↔ `['library-structure', {}]`.
- `{decisions: {status: []}}` normalises to `['library-structure', {}]` (cache parity with unscoped).
- `{decisions: {}}` normalises to `['library-structure', {}]` (empty per-type collapses).
- `{decisions: undefined}` normalises to `['library-structure', {}]` (undefined per-type collapses).
- `{decisions: {status: ['open']}}` produces a distinct tuple.
- Non-empty selection and `undefined` selection produce different keys; non-empty and `{}` produce different keys.
- **Order canonicalisation**: `normaliseSelection({decisions: {status: ['open', 'blocked']}})` and `normaliseSelection({decisions: {status: ['blocked', 'open']}})` produce deep-equal results — option arrays sort ascending so the cache key is identical regardless of toggle order.
- **Facet-key canonicalisation**: `normaliseSelection({decisions: {status: ['open'], clusterSlug: ['foo']}})` and `normaliseSelection({decisions: {clusterSlug: ['foo'], status: ['open']}})` produce deep-equal results.

### Success Criteria:

#### Automated Verification:

- [ ] Indexer aggregator tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml indexer`
- [ ] API integration test passes: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --test api_library_structure`
- [ ] Existing API smoke and config-contract tests still pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml`
- [ ] Frontend fetcher tests pass: `cd skills/visualisation/visualise/frontend && npm test -- api/fetch.test.ts api/query-keys.test.ts`
- [ ] No clippy lints: `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml`
- [ ] Type check clean: `npm run typecheck`

#### Manual Verification:

- [ ] Start the dev server: `bash scripts/test-launch-server.sh` (or equivalent), then `curl http://localhost:<port>/api/library/structure | jq` and confirm the phase structure renders with non-zero counts and populated `latest` previews on a populated meta directory.

---

## Phase 3: PR Descriptions Rename

### Overview

Coordinated rename of the `prs` wire token to `pr-descriptions`, the human label to `PR descriptions`, the Rust enum variant to `DocTypeKey::PrDescriptions`, the field `Completeness.hasPr` / `ClusterFlags::has_pr` to `hasPrDescription` / `has_pr_description`, the icon component file, the CSS doc-type token `--ac-doc-prs` to `--ac-doc-pr-descriptions`, and the 12 visual-regression PNG baselines. The on-disk `meta/prs/` directory and `config_path_key()` return value stay as `prs` per the work item's deliberate asymmetry.

### Changes Required:

#### 1. Server-side rename (TDD: existing tests as safety net)

**File**: `skills/visualisation/visualise/server/src/docs.rs`
**Changes**:
- Line 16: rename enum variant `Prs` → `PrDescriptions`. **Do not** add an explicit `#[serde(rename = "pr-descriptions")]` annotation — the enum-level `rename_all = "kebab-case"` already emits `pr-descriptions` from `PrDescriptions`, and adding a per-variant rename only for this variant breaks uniformity with every other variant. The wire token is instead pinned by a serialisation contract test (see below).
- Line 34: update `DocTypeKey::all()` array to use `PrDescriptions`.
- Line 52: `config_path_key()` for `PrDescriptions` continues returning `Some("prs")` (unchanged semantics). Add a comment above this arm: `// Wire token renamed to "pr-descriptions" in work item 0041; on-disk path` `// and config key intentionally retained as "prs" for back-compat with` `// user config files. See plan 2026-05-16-0041.`
- Line 70: `label()` for `PrDescriptions` returns `"PR descriptions"` (was `"PRs"`).
- Line 150: update inline test pairs to use `PrDescriptions` and `"pr-descriptions"` / `"PR descriptions"`. **Add** a per-variant wire-token contract test: iterate `DocTypeKey::all()` and assert each variant's `serde_json::to_value` equals the expected kebab-case string (`"decisions"`, `"plans"`, `"pr-descriptions"`, etc.). This pins the wire contract for every variant, not just this rename.
- **Add** an inverse helper `DocTypeKey::from_wire_str(s: &str) -> Option<DocTypeKey>` that returns `Some(variant)` when `s` matches a kebab-case wire token, `None` for unknown strings (no panic, no Err — silent drop is the documented contract for `parse_selection_query` consumers). Sketch:
  ```rust
  impl DocTypeKey {
      pub fn from_wire_str(s: &str) -> Option<Self> {
          Self::all().iter().copied().find(|dt| dt.wire_str() == s)
      }
      pub fn wire_str(self) -> &'static str {
          match self {
              DocTypeKey::Decisions => "decisions",
              DocTypeKey::Plans => "plans",
              DocTypeKey::PrDescriptions => "pr-descriptions",
              // ...one arm per variant; pinned by the per-variant wire-token contract test.
          }
      }
  }
  ```
- **Add** a per-variant round-trip test: iterate `DocTypeKey::all()`, assert `DocTypeKey::from_wire_str(dt.wire_str()) == Some(dt)`. Also assert `from_wire_str("bogus")` returns `None`.

**File**: `skills/visualisation/visualise/server/src/clusters.rs`
**Changes**:
- Line 16: rename `pub has_pr: bool` → `pub has_pr_description: bool` on `ClusterFlags`.
- Line 76: update sort weight match to use `PrDescriptions` variant.
- Line 108: rename `has_pr: false` → `has_pr_description: false` initialiser.
- Line 123: update `has_pr_description` mapping (was `has_pr`).
- Line 208: update inline test assertion `assert!(!c.has_pr)` → `assert!(!c.has_pr_description)`.

**File**: `skills/visualisation/visualise/server/src/slug.rs`
**Changes**:
- Line 27: update slug rule for `PrDescriptions` variant.
- Line 142: update inline test pairs.

**Test fixtures stay as-is** (they key off `doc_paths.prs`, which keeps the `prs` config key):
- `skills/visualisation/visualise/server/tests/fixtures/config.valid.json:20` — leave `"prs": "..."`.
- `skills/visualisation/visualise/server/tests/fixtures/config.optional-override-null.json:20` — leave.
- `skills/visualisation/visualise/server/tests/common/mod.rs:58` — leave `doc_paths.insert("prs".into(), ...)`.
- `skills/visualisation/visualise/server/tests/config_contract.rs:62` — leave `"prs"` in expected key list.
- `skills/visualisation/visualise/server/tests/api_smoke.rs:27` — leave `("prs", "prs")` tuple.

**File**: `skills/visualisation/visualise/server/tests/fixtures/meta/prs/42-add-config-layer.md`
**Changes**: No frontmatter change (frontmatter already says `type: pr-description`, which is internal frontmatter content unrelated to the wire token).

#### 2. Frontend-side rename

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**:
- Lines 4-8: in `DocTypeKey` union, rename `'prs'` → `'pr-descriptions'`.
- Lines 14-19: update `DOC_TYPE_KEYS` array entry.
- Lines 35-49: in `DOC_TYPE_LABELS`, change `'prs': 'PR'` to `'pr-descriptions': 'PR descriptions'`.
- Line 149: in `Completeness`, rename `hasPr: boolean` → `hasPrDescription: boolean`.
- Lines 175-209: in `LIFECYCLE_PIPELINE_STEPS`, rename `key: 'hasPr'` → `key: 'hasPrDescription'`, `docType: 'prs'` → `docType: 'pr-descriptions'`, `label: 'PR'` → `label: 'PR descriptions'`, `placeholder: 'no PR yet'` → `placeholder: 'no PR description yet'`. Update the `PipelineStepKey` type union accordingly (line 175).
- Line 247: in `PHASE_DOC_TYPES.ship.docTypes`, rename `'prs'` → `'pr-descriptions'`.

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx`
**Changes**:
- Line 11: rename import `import { PrsIcon } from './icons/PrsIcon'` → `import { PrDescriptionsIcon } from './icons/PrDescriptionsIcon'`.
- Line 55: update mapping `'prs': PrsIcon` → `'pr-descriptions': PrDescriptionsIcon`.

**File rename**: `skills/visualisation/visualise/frontend/src/components/Glyph/icons/PrsIcon.tsx` → `PrDescriptionsIcon.tsx` (use `jj mv`). Inside, rename exported function `PrsIcon` → `PrDescriptionsIcon`.

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: Lines 38, 79: rename token key `'ac-doc-prs'` → `'ac-doc-pr-descriptions'` in both light and dark token groups.

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: Lines 107, 206, 252: rename `--ac-doc-prs` → `--ac-doc-pr-descriptions` in all three theme blocks.

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`
**Changes**: Lines 40, 44: update fixture labels (`'PR reviews'` stays as-is for `pr-reviews` key; `'PRs'` becomes `'PR descriptions'` for the renamed key).

**File**: `skills/visualisation/visualise/frontend/src/router.test.tsx`
**Changes**: Lines 93, 190: rename `hasPr: false` → `hasPrDescription: false`.

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`
**Changes**: Lines 140, 329, 346: rename `hasPr: false` → `hasPrDescription: false`.

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.test.tsx`
**Changes**: Line 12: rename `hasPr: false` → `hasPrDescription: false`.

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`
**Changes**: Line 14: rename.

**File**: `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.test.tsx`
**Changes**: Line 8: rename.

**File**: `skills/visualisation/visualise/frontend/src/api/use-unseen-doc-types.ts`
**Changes**: Inside `parseStored()`, before the `isDocTypeKey(key)` filter, add a one-shot in-place key migration: if the parsed JSON contains a `"prs"` key, rewrite it to `"pr-descriptions"` (preserving the epoch ms value), then drop the old key. Without this, any user with existing browser state silently loses their last-seen timestamp for the type, and the PR-descriptions card resurfaces as unseen.

```ts
// Inside parseStored(), after JSON.parse, before isDocTypeKey filter:
if (parsed && typeof parsed === 'object' && 'prs' in parsed) {
  parsed['pr-descriptions'] = parsed['prs']
  delete parsed['prs']
}
```

Add a test asserting the migration: pre-populate localStorage with `{"prs": 12345}`, call the parser, assert the result contains `"pr-descriptions": 12345` and not `"prs"`.

**Search-and-replace check**: After the above, run the verification grep specified in the Success Criteria below.

#### 3. Visual-regression baselines

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/glyph-showcase.spec.ts`
**Changes**: Update whatever array/loop drives the snapshot id for the renamed glyph so Playwright emits paths with `pr-descriptions-...` instead of `prs-...`. Playwright matches snapshots by computed path derived from the test id, so this edit must land in lock-step with the file renames below.

**Directory**: `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/`
**Changes**: Rename 12 files via `jj mv`:
- `prs-{16,24,32}-{light,dark}-visual-regression-darwin.png` → `pr-descriptions-{16,24,32}-{light,dark}-visual-regression-darwin.png` (6 files)
- Same 6 for `-linux.png` variant

Verify byte-identity (the icon visual didn't change, only the file name):

```bash
# Pre-rename: capture hashes
shasum -a 256 prs-*.png > /tmp/prs-pre.sha
# After jj mv:
shasum -a 256 pr-descriptions-*.png | sed 's/pr-descriptions/prs/' > /tmp/prs-post.sha
diff /tmp/prs-pre.sha /tmp/prs-post.sha
```

The `diff` must be empty. If linux baselines are not present locally, regenerate them on CI via `npx playwright test --update-snapshots` and verify the new hashes match the originals.

#### 4. Skill documentation

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**: Line 19: update human label `**PRs directory**:` → `**PR descriptions directory**:`. The CLI argument `prs` (passed to `config-read-path.sh`) stays.

#### 5. Helper scripts (config-key references stay)

No changes required to:
- `frontend/e2e/start-server.mjs:69` — keys off config `prs`.
- `scripts/write-visualiser-config.sh:60,158,184` — keys off config `prs`.
- `scripts/test-launch-server.sh:75` — keys off config `prs`.

### Success Criteria:

#### Automated Verification:

- [ ] Server unit tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml`
- [ ] Server integration tests pass: `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --tests`
- [ ] Frontend unit tests pass: `cd skills/visualisation/visualise/frontend && npm test`
- [ ] Frontend type check clean: `npm run typecheck`
- [ ] Lint clean: `npm run lint` and `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml`
- [ ] No residual `prs` references in renamed surface: `rg -w 'prs' skills/visualisation/visualise/{frontend/src,server/src} -g '*.{ts,tsx,rs,css}'` returns only deliberately-retained occurrences: (a) the `config_path_key()` arm in `docs.rs`, (b) any `meta/prs/` path string literals, (c) slug literals derived from on-disk filenames. Separate per-identifier checks: `rg -w 'hasPr' …`, `rg 'PrsIcon' …`, `rg -- '--ac-doc-prs' …`, `rg 'DocTypeKey::Prs\b' …` — each should return zero matches outside test fixtures pinned by Phase 3 §1.
- [ ] Visual regression suite passes after rename: `cd skills/visualisation/visualise/frontend && npx playwright test tests/visual-regression/glyph-showcase.spec.ts`

#### Manual Verification:

- [ ] Start the dev server and visit `/glyph-showcase`; confirm the renamed glyph still renders correctly under both themes.
- [ ] `curl http://localhost:<port>/api/types | jq '.types[] | select(.key=="pr-descriptions")'` returns the renamed entry with `label: "PR descriptions"` and `dirPath` pointing at `meta/prs/`.

---

## Phase 4: Library Overview Hub + Sidebar Migration + Redirect Removal

### Overview

Build `LibraryOverviewHub` consuming the Phase 2 endpoint via `useQuery`. Remove the `/library` → `/library/decisions` redirect, replacing `libraryIndexRoute` with a `component` reference. Migrate `Sidebar.tsx` to consume the new server structure response; delete `PHASE_DOC_TYPES`. Update `router.test.tsx` and `Sidebar.test.tsx`.

### Changes Required:

#### 1. `LibraryOverviewHub` component (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.test.tsx`

Tests:
- Renders the eyebrow `LIBRARY` (with library Glyph), H1 `All artifacts in meta/` (with `meta/` styled inline mono), and the documented subtitle copy.
- Given a mocked `useQuery` returning `phases: [{id: "define", docTypes: [t1]}, {id: "discover", docTypes: [t2, t3]}]` and `templates: {...}`, renders two phase groupings in that order with one and two cards respectively, plus a `templates` card under its own section. (Asserts the structure is server-driven, not hard-coded.)
- Each card renders the doc-type Glyph + label on a single row at top-left, count at top-right, and `latest · {title}` preview line below.
- Non-zero-count cards are wrapped in `<Link to="/library/$type" params={{type: id}}>`.
- Zero-count doc types render a dimmed card (no `<Link>` wrapper, `aria-disabled="true"`) with the muted `no documents yet` line in place of the latest preview. Click does not navigate. Rationale: empty cards as links result in a redundant two-click path to an empty list view; dimming + disabling makes the empty state legible at the hub level.
- Loading state renders a placeholder; error state renders an error message; both still wrap in `Page`.
- Theme verification: assert source-string token usage via `LibraryOverviewHub.module.css?raw` rather than `data-theme` DOM attribute (jsdom cannot resolve CSS custom property values; this is the established pattern from `PageSubtitle.test.tsx`).
- Cache-warm no-fetch: pre-warm the React Query cache for `queryKeys.libraryStructure()` (no selection arg) with a fixture response, then mount `LibraryOverviewHub` and assert the mocked `fetchLibraryStructure` is invoked zero additional times. This pins the load-bearing coupling between RootLayout's unscoped fetch and the hub's data source — a future change adding a selection argument to the hub's `useQuery` would silently double-fetch and this test would fail loudly.

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.module.css`

Styles (sketch):
- `.hubGrid { display: grid; gap: var(--sp-5); grid-template-columns: 1fr; }`
- `@media (min-width: 640px) { .hubGrid { grid-template-columns: repeat(2, 1fr); } }`
- `@media (min-width: 1024px) { .hubGrid { grid-template-columns: repeat(3, 1fr); } }`
- `.phaseSection { display: contents; }` (or wrap as needed)
- `.phaseHeader { font-size: var(--ac-text-eyebrow); text-transform: uppercase; color: var(--ac-fg-muted); padding-block: var(--sp-3); }`
- `.card { background: var(--ac-bg-card); border: 1px solid var(--ac-stroke); border-radius: var(--radius-md); padding: var(--sp-4); display: flex; flex-direction: column; gap: var(--sp-3); }`
- `.cardTopRow { display: flex; align-items: center; justify-content: space-between; }`
- `.cardLatest { color: var(--ac-fg-muted); font-size: var(--ac-text-sm); }`
- `.cardEmpty { color: var(--ac-fg-faint); font-style: italic; }`

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.tsx`

Implementation:
- Calls `useQuery({ queryKey: queryKeys.libraryStructure(), queryFn: () => fetchLibraryStructure() })` directly — exactly the same key (no selection arg) as RootLayout's parallel call. React Query deduplicates by structural key equality, so only ONE network request is made per page load even though both call sites are mounted; the hub reads from the shared cache. This avoids the prop-drilling complication that route-tree `<Outlet />` doesn't accept arbitrary props. The cache-warm no-fetch assertion in the hub test (see Phase 4 §1 tests above) and the `normaliseSelection`-based key canonicalisation together pin this contract.
- Wraps content in `<Page eyebrow={...} title={...} subtitle={...}>`.
- Iterates `phases.map(phase => …)`; per phase, renders header + grid of cards via `phase.docTypes.map(dt => <Card />)`. Renders the `templates` entry under its own section at the end of the grid.
- Card uses `<Glyph docType={dt.id} size={24} />` (the wire response no longer carries a separate `glyphId` — `id` is sufficient) and `<Link to="/library/$type" params={{type: dt.id}}>` for non-zero counts. Zero-count cards render the dimmed `aria-disabled` variant described in the tests above.

#### 2. Router change

**File**: `skills/visualisation/visualise/frontend/src/router.ts`
**Changes**:
- Lines 71-77: replace `libraryIndexRoute`'s `beforeLoad` redirect with `component: LibraryOverviewHub`.
- Lines 101-104: leave `libraryTypeRoute.parseParams` fallback alone — it already targets `/library`, so the unknown-type case lands consistently on the new hub.
- Line 60: `indexRoute` redirect `/` → `/library` is unchanged (still terminates correctly).

**File**: `skills/visualisation/visualise/frontend/src/router.test.tsx`
**Changes**:
- Lines 40-44: redirect test from `/`. Update terminal expectation from `/library/decisions` to `/library`.
- Lines 46-49: bare `/library` redirect test. Update terminal expectation from `/library/decisions` to `/library`.
- Lines 71-77: invalid-type fallback. Update terminal expectation from `/library/decisions` to `/library`.

#### 3. Sidebar migration to server shape

**Decision**: Prop-driven. `RootLayout` owns the `useQuery({ queryKey: queryKeys.libraryStructure(), queryFn: () => fetchLibraryStructure() })` call and passes `phases` and `templates` to `Sidebar` as new props, mirroring the existing `docTypes` flow.

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**:
- Extend the props interface: add `phases: LibraryPhase[]` and `templates: LibraryDocType` alongside the existing `docTypes: DocType[]`. The `docTypes` prop stays — it carries `dirPath`, which the Sidebar still needs for unrelated affordances and which `LibraryStructureResponse` does not duplicate.
- Replace the `PHASE_DOC_TYPES.map(...)` loop at `:38-78` with `phases.map(...)`. Each phase iterates `phase.docTypes.map(dt => ...)`. Doc-type label and count come from `dt.label` / `dt.count`; route path is `/library/${dt.id}`. Active-state matching unchanged.
- Render the templates link from the top-level `templates` prop (replaces the current `byKey.get('templates')` lookup).
- Unseen-marker logic (the per-key Set lookup) is unchanged — it keys off `dt.id`.
- Remove the `import { PHASE_DOC_TYPES } from '../../api/types'` import.

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`
**Changes**:
- Lines 267, 279: update tests that previously used `PHASE_DOC_TYPES` to use the new structure shape.
- Add coverage for "phase order is server-driven" by rendering with two different mock phase orderings and asserting different output.

#### 4. Retire `PHASE_DOC_TYPES`

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: Delete the `PHASE_DOC_TYPES` constant block at lines 220-254 (including JSDoc comment).

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`
**Changes**: Add `useQuery({ queryKey: queryKeys.libraryStructure(), queryFn: () => fetchLibraryStructure() })` alongside the existing `useQuery({ queryKey: queryKeys.types() })`. Thread the new `phases` and `templates` data into `<Sidebar phases={…} templates={…} docTypes={…} />`. The Sidebar is prop-driven (matching its existing `docTypes` flow). `LibraryOverviewHub` is rendered through the `<Outlet />` and calls its own `useQuery` with the same key (see Phase 4 §1) — React Query deduplicates so only one network fetch occurs per page load. The shared-key invariant is pinned by (a) `normaliseSelection`'s canonical-form tests in `query-keys.test.ts`, (b) the cache-warm no-fetch assertion in the LibraryOverviewHub test.

### Success Criteria:

#### Automated Verification:

- [ ] LibraryOverviewHub tests pass: `cd skills/visualisation/visualise/frontend && npm test -- routes/library/LibraryOverviewHub`
- [ ] Updated router tests pass: `npm test -- router.test`
- [ ] Updated Sidebar tests pass: `npm test -- components/Sidebar/Sidebar.test`
- [ ] All other tests still pass: `npm test`
- [ ] Type check clean: `npm run typecheck`
- [ ] No residual `PHASE_DOC_TYPES` references: `rg "PHASE_DOC_TYPES" skills/visualisation/visualise/frontend/src` returns no matches.

#### Manual Verification:

- [ ] Start dev server, visit `/library`; confirm overview hub renders with phase groupings, doc-type cards with counts and latest previews. Confirm clicking a card navigates to `/library/<type>`.
- [ ] Visit `/library/bogus`; confirm fallback now lands on `/library` (the hub) instead of `/library/decisions`.
- [ ] Toggle dark mode; confirm hub renders cleanly with all token-driven colours.
- [ ] Confirm sidebar phase grouping still renders correctly (now from server response).
- [ ] Check responsive grid: shrink the viewport below 640px (1 column), 640–1024px (2 columns), ≥1024px (3 columns).

---

## Phase 5: List View Refactor + Sort + Filter + Empty States

### Overview

Refactor `LibraryTypeView` to use `Page`, restructure to five columns with the new ID pill and Chip-based status cell, and remove column-header click-sort. Build `SortPill` and `FilterPill` components consuming the Phase 1 popover primitive. Build `EmptyState` (doc-type-empty) and `NoResultsPanel` (filter-applied-empty) components. Wire facet metadata from the Phase 2 server response.

### Changes Required:

#### 1. ID formatter helper (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/doc-type-id.test.ts`

Tests:
- `formatDocId('PROJ-0001')` → `'PROJ-0001'` (already-formatted IDs pass through).
- `formatDocId('PROJ-1')` → `'PROJ-0001'` (zero-pads numeric part to 4 digits).
- `formatDocId('PROJ-12345')` → `'PROJ-12345'` (5-digit IDs are passed through, not truncated; the server's `id_pattern` default is `{number:04d}` but the formatter accepts longer numerals).
- `formatDocId(null)` → `''`.
- `formatDocId('NOTPREFIXED')` returns the original string unchanged (defensive: only formats when matching a `<prefix>-<digits>` pattern).
- `formatDocId('proj1-0001')` → `'proj1-0001'` (alphanumeric prefix accepted; case preserved).

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/doc-type-id.ts`

```ts
export function formatDocId(workItemId: string | null | undefined): string {
  if (!workItemId) return ''
  // Match any non-dash prefix followed by a dash and digits.
  // Broader than ASCII letters so alphanumeric and unicode prefixes pass through.
  const match = workItemId.match(/^([^-]+)-(\d+)$/)
  if (!match) return workItemId
  const [, prefix, digits] = match
  return `${prefix}-${digits.padStart(4, '0')}`
}
```

#### 2. `SortPill` component (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/components/SortPill/SortPill.test.tsx`

Tests:
- Pill renders the active option label (e.g. `Recently modified`) — visible in the pill's closed state so the user can tell what sort is active without opening the menu.
- Pill renders a small direction indicator (↓ for descending-by-recency or Z→A; ↑ for ascending) alongside the label.
- Clicking the pill opens the menu (panel becomes visible). Trigger exposes `aria-haspopup="menu"`, `aria-expanded`, `aria-controls`.
- Menu shows a `SORT BY` header and 5 options: `Recently modified`, `Oldest first`, `Title (A → Z)`, `Title (Z → A)`, `ID (ascending)`.
- Options use `role="menuitem"` (single-select; not menuitemcheckbox).
- Selected option is highlighted and shows a checkmark.
- Selecting an option calls `onChange` with the new value and updates the pill label and direction indicator; the menu closes.
- Menu dismisses on click-outside and Escape.
- Arrow-key navigation moves focus among options; Enter and Space both select.

**New files**:
- `skills/visualisation/visualise/frontend/src/components/SortPill/SortPill.tsx`
- `skills/visualisation/visualise/frontend/src/components/SortPill/SortPill.module.css`

API:
```ts
export type SortOption =
  | 'recently-modified' | 'oldest-first' | 'title-asc' | 'title-desc' | 'id-asc'

export interface SortPillProps {
  value: SortOption
  onChange: (next: SortOption) => void
}
```

#### 3. `FilterPill` component (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.test.tsx`

Tests:
- Pill labelled `▽ Filter` (or just `Filter` with leading icon). Trigger exposes `aria-haspopup="menu"`, `aria-expanded`, `aria-controls`.
- Clicking opens a menu with a `FILTER` header.
- Given a doc type whose facets are `[{id: 'status', label: 'Status', options: [...]}, {id: 'clusterSlug', label: 'Cluster', options: [...11 options]}]`, menu shows two facet sections; the second section renders an inline search input above its options because option count > 8 (computed client-side as `facet.options.length > 8`, no server-emitted `kind` field).
- Status-facet options render as `Chip` instances using `statusToChipVariant`.
- Search input above a long facet's option list: placeholder is `Filter {facet.label}…`; arrow-down from the input moves focus to the first option list item; Escape with non-empty input clears it first, second Escape closes the popover; printable characters typed while the input has focus do NOT trigger the Popover's menu-key handler.
- Options use `role="menuitemcheckbox"` with `aria-checked`; Enter and Space both toggle the option's selection without closing the panel. Closing happens only via Escape or click-outside.
- Multiple options within a facet OR; multiple facets AND. Tested via passing a fixture and asserting `onChange` payload.
- Each option shows a count to its right (read from `facets[i].options[j].count`).
- Selection round-trip: setting `selection={{status: ['open']}}` renders `aria-checked="true"` on the matching option; toggling fires `onChange({status: []})`. The component does not maintain internal selection state — it's controlled by the parent (`LibraryTypeView`).
- Menu dismisses on click-outside and Escape; a `Clear filters` button appears in the menu footer when any option is selected and calls `onChange({})`.
- `isFetching={true}` renders a pulse/spinner indicator on the closed trigger pill (e.g. a subtle dot or animated underline) so the user can tell a refetch is in flight after toggling an option. Test asserts the indicator is present when `isFetching` is true and absent when it's false.

**New files**:
- `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.tsx`
- `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.module.css`

API:
```ts
import type { LibrarySelectionPerType } from '../../api/types'

export interface FilterPillProps {
  facets: LibraryFacet[]
  selection: LibrarySelectionPerType
  onChange: (next: LibrarySelectionPerType) => void
  isFetching?: boolean             // pulse the trigger while the parent's query is refetching
}
```

Note: `FilterPill` consumes `LibrarySelectionPerType` directly (the per-doc-type slice of `LibrarySelection`). There is no separate `FilterSelection` type — the wire shape and the FilterPill API share one source of truth. `LibraryTypeView` (the parent) holds the slice in `useState<LibrarySelectionPerType>({})` and constructs the full `LibrarySelection` for the query via `{ [type]: selection }` at the fetch boundary.

Notes on the count-display contract:
- Option counts shown next to each entry come from `facets[i].options[j].count` in the response. The server computes these using the "post-other-facet, pre-own-facet" scoping rule (Phase 2): when the user toggles a value in facet B, the next response cycle delivers updated counts for facet A. The FilterPill is purely a render layer over whatever counts the server emits; it does not recompute anything client-side.
- The parent (`LibraryTypeView`) is responsible for refetching `libraryStructure(selection)` whenever `selection` changes; React Query handles caching by selection.

#### 4. `EmptyState` (doc-type-empty) component (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.test.tsx`

Tests:
- Renders large doc-type Glyph, the path heading derived from the server's `DocType.dirPath` field (e.g. `meta/prs/` for the renamed `pr-descriptions` type — the on-disk directory is intentionally retained, see Phase 3 §1), the `no {type-plural} yet.` headline, the description sentence, and the indexer-aware footer. **Do not** synthesise the path heading from `{type}` directly — `pr-descriptions` is the wire token, not the on-disk directory.
- Description sentence comes from a per-doc-type lookup table (hard-coded in the frontend per work item).
- Footer string matches the literal `New files added to {dirPath} are picked up live — this view will populate as soon as the indexer sees them.`
- Completeness check (data-driven): iterate `DOC_TYPE_KEYS` and assert that `EMPTY_DESCRIPTIONS[key]` and `EMPTY_TYPE_PLURALS[key]` are both non-empty strings for every key. This catches missed-key regressions when new doc types are added (or when the `prs` → `pr-descriptions` rename touches the table).

**New files**:
- `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.tsx` — accepts a `docType` and a `dirPath` prop; renders the empty card.
- `skills/visualisation/visualise/frontend/src/routes/library/empty-descriptions.ts` — exports `EMPTY_DESCRIPTIONS: Record<DocTypeKey, string>` and `EMPTY_TYPE_PLURALS: Record<DocTypeKey, string>` (e.g. `'pr-descriptions': 'pr descriptions'`). TypeScript's record-completeness check enforces all keys at compile time; the runtime test enforces non-empty values.

#### 5. `NoResultsPanel` (filter-applied-empty) component (TDD: tests first)

**New file**: `skills/visualisation/visualise/frontend/src/routes/library/NoResultsPanel.test.tsx`

Tests:
- Renders the headline `no results match your filter`.
- Renders an explanatory sentence.
- Renders a summary of the currently-active filters above the button (e.g. `Active filters: Status: open, blocked · Cluster: foo`) so the user can see what's narrowing the result set before clearing.
- Renders a `Clear filters` button; clicking calls the supplied `onClear` callback.

API:
```ts
export interface NoResultsPanelProps {
  selection: LibrarySelectionPerType
  facets: LibraryFacet[]   // used to humanise facet/option ids in the summary
  onClear: () => void
}
```

**New files**:
- `skills/visualisation/visualise/frontend/src/routes/library/NoResultsPanel.tsx`
- `skills/visualisation/visualise/frontend/src/routes/library/NoResultsPanel.module.css`

#### 6. `LibraryTypeView` refactor (TDD: drive via existing test file rewrite)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.test.tsx`
**Changes**: Rewrite the existing 18 tests to cover the new contract:
- Page header (eyebrow + title + count subtitle + actions slot showing sort + filter pills).
- Five columns: `ID / DATE`, `TITLE`, `STATUS`, `SLUG`, `MODIFIED`.
- First-column rendering follows a deterministic per-row fallback chain. `entry.frontmatter` is typed `Record<string, unknown>` so the date branch narrows explicitly:
  - (a) if `entry.workItemId` is non-null, render `<span className={styles.idPill}>{formatDocId(entry.workItemId)}</span>`;
  - (b) else `const dateRaw = entry.frontmatter['date']; if (typeof dateRaw === 'string' && !Number.isNaN(Date.parse(dateRaw)))` render via `formatDate(dateRaw)`;
  - (c) else render an em-dash placeholder (`'—'`).
  Tests cover all four cases per doc type: ID-only, date-only (string-valued), both-present (ID wins), and a row where `frontmatter.date` is a non-string (number/array/object) — that row must fall through to the em-dash branch, not coerce.
- Status rendered as `<Chip>` via `statusToChipVariant`.
- Modified rendered via `formatMtime`.
- Column-header clicks no longer change sort.
- Sort-pill default is `Recently modified`; selecting `Title (A → Z)` reorders the list.
- Sort tie-breaker matrix — one test per sort option (5 cases) plus three sub-cases for `recently-modified`:
  - `recently-modified`: equal `mtimeMs` → workItemId ascending when both present; mtime equal + one workItemId null → entry with workItemId wins (or null sorts last — pick one and document); mtime equal + workItemIds equal → relPath ascending.
  - `oldest-first`: same tie-break chain, applied to ascending mtime.
  - `title-asc`: equal titles → workItemId ascending then relPath ascending.
  - `title-desc`: equal titles → workItemId ascending then relPath ascending (tie-break direction is independent of primary direction).
  - `id-asc`: equal workItemId (e.g. two entries with null workItemId) → relPath ascending.
- Filter selections combine OR within facet, AND across facets — fetched via `fetchLibraryStructure(selection)` so the server returns scoped counts.
- Doc-type-empty: when `entries.length === 0`, sort and filter pills are hidden and the `EmptyState` card renders. Assert the pills are NOT in the document (not just hidden via CSS).
- Filter-applied-empty: when the active selection produces `filtered_count === 0` but `count > 0`, page chrome and pills stay visible and the `NoResultsPanel` renders with a working `Clear filters` button.
- Transition: starting from a filter-applied-empty state, clicking `Clear filters` calls `onChange({})` and the next render shows the full table (not the EmptyState — `count > 0` so the doc-type-empty branch should not engage).
- End-to-end selection wiring (integration): mount LibraryTypeView with a mocked `fetchLibraryStructure` that returns distinct responses for different selections. Toggle a FilterPill option and assert (a) `fetchLibraryStructure` is invoked a second time with the new `LibrarySelection`, (b) the rendered facet counts switch to those from the second response, (c) the subtitle's `filteredCount` reflects the new response. Then toggle a second option and assert a third invocation with the accumulated selection. Then click `Clear filters` and assert the cache key reverts to the no-selection variant (the unscoped response is reused from cache, no third network call). This pins the full loop in one test so a wiring break (forgotten cache key, missing query-fn argument, mis-encoded URL) fails loudly.
- `keepPreviousData` refetch behaviour: after the first successful response renders, toggle a FilterPill option but resolve the second `fetchLibraryStructure` only after a delay (e.g. a deferred promise). While the second response is in flight, assert (a) the row count and rendered rows still reflect the *first* response (rows do not blank), (b) the subtitle still shows the first response's `filteredCount`, (c) the FilterPill's `isFetching` indicator is visible. Then resolve the deferred promise and assert all three switch to the second response. A regression that removes `placeholderData: keepPreviousData` would blank the table mid-flight and this test would fail.
- Cross-doc-type bleed guard: mount LibraryTypeView at `/library/decisions`, wait for the first response, then re-mount at `/library/plans` with the plans `fetchLibraryStructure` deferred. While the plans query is in flight, assert the subtitle is `'Loading…'` (not the decisions count) and the FilterPill renders no decisions facets. Then resolve and assert plans data appears. This pins the `currentTypeData` derivation that filters `query.data` to the active type.
- Pending-state subtitle: with a mocked query in cold-pending state (no previous data), assert the subtitle is `'Loading…'` and NOT the literal `'undefined documents'`.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx`
**Changes**:
- Remove `SortHeader` (`:143-166`), `toggleSort` (`:71-74`), and the sort-by-column-header click handler in the `<th>` rendering (`:106-132`).
- Replace `useState<SortKey>` / `useState<SortDir>` with a single `useState<SortOption>('recently-modified')` plus the comparator switch.
- Replace local sort state by a `sortBy(option, entries)` function with the documented per-option tie-break matrix (see tests above).
- Add a `useState<LibrarySelectionPerType>` initialised to `{}` for this doc type's selection.
- Add a `useQuery({ queryKey: queryKeys.libraryStructure({ [type]: selection }), queryFn: () => fetchLibraryStructure({ [type]: selection }), placeholderData: keepPreviousData })` call. The query is keyed on selection so toggling a facet option refetches; previously-fetched selections stay cached.
- `placeholderData: keepPreviousData` ensures stale rows and counts stay on screen during in-flight refetches instead of blanking; pair with `isFetching` to indicate the refresh.
- **Guard against cross-doc-type bleed**: `keepPreviousData` returns the previous query's data even when the new query key has a different `type`. To prevent `LibraryTypeView` for `/library/plans` from briefly showing decisions' rows/counts/facets, derive `currentTypeData` as `query.data?.phases.flatMap(p => p.docTypes).concat([query.data.templates]).find(dt => dt.id === type)` (or read from the per-doc-type structure however the response shape allows). All subtitle / row-count / FilterPill renders read from `currentTypeData`, not from `query.data` directly. When `currentTypeData` is undefined (cross-type navigation in flight, or cold cache), the subtitle falls back to the `'Loading…'` placeholder and the rows render the skeleton/empty content rather than the previous type's data.
- Read this doc type's `filteredCount` and `filterFacets` from the response; feed `filterFacets` into `<FilterPill facets={…} selection={selection} onChange={setSelection} isFetching={query.isFetching} />`.
- Wrap content in `<Page eyebrow={<><Glyph docType={type} size={16} />{label.toUpperCase()}</>} title={label} subtitle={currentTypeData ? `${currentTypeData.filteredCount} documents` : 'Loading…'} actions={<><SortPill .../><FilterPill .../></>}>`. The subtitle reads from `currentTypeData` (a derived value that filters `query.data` to the active type — see the cross-bleed guard above), so it falls back to `'Loading…'` when (a) the query is cold-pending OR (b) `keepPreviousData` is serving the previous *doc type's* data. Add tests asserting (i) the subtitle is `'Loading…'` for a cold-pending query (not the literal `'undefined documents'`), (ii) the subtitle is `'Loading…'` when navigating decisions → plans while the plans query is in flight (not the decisions count).
- Render the new column layout: `<th>ID / DATE</th><th>TITLE</th><th>STATUS</th><th>SLUG</th><th>MODIFIED</th>`. Each `<th>` is a plain non-button cell.
- Status cell uses `<Chip variant={statusToChipVariant(entry.frontmatter.status)}>`.
- First-column cell follows the per-row fallback chain: ID pill via `formatDocId(entry.workItemId)` when non-null; else `const dateRaw = entry.frontmatter['date']; typeof dateRaw === 'string' && !Number.isNaN(Date.parse(dateRaw))` → `formatDate(dateRaw)`; else `'—'`.
- Empty-state branch: if `count === 0` (the type has no entries at all), render `<EmptyState docType={type} dirPath={docType.dirPath} />` instead of the table; do not render the actions slot.
- Filter-applied-empty branch: if `filteredCount === 0 && count > 0`, render `<NoResultsPanel onClear={() => setSelection({})} />` instead of the table body; keep the pills.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.module.css`
**Changes**:
- Delete `.container`'s 900px `max-width` (Page now owns it).
- Delete `.sortButton` rule.
- Delete `.empty` rule (no longer used; EmptyState owns its styling).
- Delete `.badge` rule (was a no-op anyway).
- Add `.idPill` rule for the monospace ID pill.

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**:
- Line 134: remove the `LibraryTypeView.module.css` 900px allowlist entry (consumes `--ac-content-max-width` now via Page).
- Lines 131-133: update the `LibraryTypeView.module.css` `2px` / `1px` / `0.4rem` literal counts to match the new file (likely all become 0 after `.sortButton` / column-header borders are removed).

### Success Criteria:

#### Automated Verification:

- [ ] doc-type-id helper tests pass: `cd skills/visualisation/visualise/frontend && npm test -- routes/library/doc-type-id`
- [ ] SortPill tests pass: `npm test -- components/SortPill`
- [ ] FilterPill tests pass: `npm test -- components/FilterPill`
- [ ] EmptyState tests pass: `npm test -- routes/library/EmptyState`
- [ ] NoResultsPanel tests pass: `npm test -- routes/library/NoResultsPanel`
- [ ] Updated LibraryTypeView tests pass: `npm test -- routes/library/LibraryTypeView`
- [ ] Migration test passes (allowlist + literal counts updated): `npm test -- styles/migration.test.ts`
- [ ] Type check clean: `npm run typecheck`
- [ ] Lint clean: `npm run lint`

#### Manual Verification:

- [ ] Visit `/library/decisions`; confirm new column layout, ID pill, status chip, and Page chrome.
- [ ] Click the sort pill; confirm menu opens with five options, selecting one re-orders the list.
- [ ] Click a column header; confirm nothing happens (no sort change).
- [ ] Click the filter pill; confirm menu shows facet sections appropriate to the doc type. For decisions, expect Status + Cluster slug. For work-items, expect Status + Project + Cluster slug.
- [ ] Toggle a status filter; confirm rows filter and option counts in other facets update.
- [ ] Filter to no-results; confirm NoResultsPanel renders with `Clear filters` button; clicking restores full list.
- [ ] Visit a doc type with zero documents; confirm EmptyState renders with the correct path, headline, description, and footer; pills are hidden.
- [ ] Confirm light/dark theme rendering.

---

## Phase 6: Five-Route Page Migration + Cleanup

### Overview

Migrate `KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, and `LibraryTemplatesIndex` to consume `Page`. Strip `RootLayout.main`'s padding rule. Delete `PageSubtitle`. Update `migration.test.ts` allowlist (four entries removed) and the `.title { color: var(--ac-fg-strong) }` `REQUIRED` list (three entries removed). This is the most invasive UI step and must land atomically — the `RootLayout` change has no per-route fallback once shipped.

### Changes Required:

#### 1. KanbanBoard migration (TDD: drive via existing tests)

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx`
**Changes**: Update tests that previously asserted `PageSubtitle` rendering — assert `Page` rendering with `title="Kanban"` (and `<Chip>live</Chip>` in the actions slot only on the success branch; loading and error branches omit the chip). Verify all three branches (loading, error, success) wrap in `<Page>`.

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.tsx`
**Changes**:
- Line 20: replace `import { PageSubtitle } from '../../components/PageSubtitle/PageSubtitle'` with `import { Page } from '../../components/Page/Page'`.
- Lift the `Page` wrapper above the branch conditional rather than duplicating it across loading / error / success branches. Compute branch content as a local variable, then return a single `<Page title="Kanban" actions={query.isSuccess ? <Chip variant="indigo">live</Chip> : undefined}>{content}</Page>`. Avoids triplicating the wrapper and keeps title/actions logic in one place.

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.module.css`
**Changes**:
- Line 5: change `.board { padding: var(--sp-4) }` to `.board { padding-block: var(--sp-4) }` (drops horizontal padding to avoid double-padding now that Page owns the horizontal axis).

#### 2. LifecycleIndex migration (TDD: tests first because the route lacks a header today)

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.test.tsx`
**Changes**: Add tests asserting `Page` wraps the content with `title="Lifecycle"`. The toolbar (filter input + 3 SortButtons) renders in the content slot directly above the table, NOT in the Page `actions` slot — the toolbar is too wide for the header right-rail on narrow viewports, and the filter input's growth behaviour would push the title down. All four branches (loading / error / empty / success) wrap in `Page`.

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx`
**Changes**:
- Lift the `Page` wrapper above the branch conditional: compute branch content, return `<Page title="Lifecycle">{toolbar}{branchContent}</Page>` once. Toolbar renders as a `<div className={styles.toolbar}>…</div>` above whatever the active branch returns.
- Lines 72-87: extract toolbar contents into a stable JSX expression for the content slot.
- Loading / error / empty branches that previously returned bare `<p>` outside `.container` now return the `<p>` inside `<Page>`'s content slot, beneath the toolbar.

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.module.css`
**Changes**:
- Line 1: delete `.container` `max-width: 900px` (Page owns it; route consumes the canonical 1100px).

#### 3. LibraryDocView migration (TDD: drive via existing tests)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.test.tsx`
**Changes**: Update tests asserting the header layout — title is now in `Page`'s slot; `FrontmatterChips` is in the `subtitle` slot; the two-column grid (body + aside) lives inside `Page`'s content slot. Assert all branches wrap in `<Page>`: loading (placeholder title `'Loading…'`), error/not-found (placeholder title `'Document not found'`), and success (`title={entry.title}`).

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
**Changes**:
- Lines 70-78: lift the `<Page>` wrapper above the branch conditional. Compute `title`, `subtitle`, and `content` local variables in an explicit `if/else if/else` block, then return a single `<Page title={title} subtitle={subtitle}>{content}</Page>`. The non-success branches assign placeholder titles (`'Loading…'`, `'Document not found'`) and bare `<p>` content; the success branch assigns `entry.title` and renders the two-column grid.
- Keep `entry!` (or compute the success-branch values inside an `else if (entry)` block so TypeScript narrows the `Entry | undefined` to `Entry` naturally — the lift doesn't perform the narrowing on its own). Either pattern is fine; the original `entry!` assertion is sound. The plan does NOT claim the assertion is "no longer needed" — it just lifts the wrapper.

Example sketch:
```tsx
let title: ReactNode = 'Loading…'
let subtitle: ReactNode | undefined = undefined
let content: ReactNode = <p>Loading…</p>
if (query.isError || (!query.isPending && !entry)) {
  title = 'Document not found'
  content = <p>Document not found</p>
} else if (entry) {
  title = entry.title
  subtitle = <FrontmatterChips frontmatter={entry.frontmatter} state={entry.frontmatterState} />
  content = <article className={styles.article}>{/* body + aside grid */}</article>
}
return <Page title={title} subtitle={subtitle}>{content}</Page>
```
- The two-column grid (body + aside) becomes the content slot of the success branch. Adjust `LibraryDocView.module.css` accordingly: `.article` becomes the grid container, no longer carrying header.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css`
**Changes**:
- Line 6: drop `max-width: 1100px` (Page owns it; this route uses canonical 1100).
- Lines 3-9: remove `header header` from `grid-template-areas`; the grid now expresses only `body aside`.
- Remove `.title` styling (Page owns title).

#### 4. LibraryTemplatesView migration (trivial swap)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.test.tsx`
**Changes**: Update tests asserting `Page` renders with `title={name}`. Cover all branches: success (`title={name}`), template-not-found (`title="Template not found"`).

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx`
**Changes**:
- Lines 40-43: lift `<Page>` above the branch conditional. Compute branch content; return single `<Page title={name ?? 'Template not found'}>{content}</Page>`.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css`
**Changes**:
- Line 1: delete `.container` `max-width: 900px`.
- Delete `.title { margin: 0 0 var(--sp-5) }` rule.

#### 5. LibraryTemplatesIndex migration (with maxWidth="narrow")

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.test.tsx`
**Changes**: Update tests asserting `Page` renders with `title="Templates"` and `maxWidth="narrow"` (which binds to `--ac-content-max-width-narrow: 600px` from Phase 1 §1). Cover branches: loading, error, empty templates list, success.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`
**Changes**:
- Lines 31-33: lift `<Page>` above the branch conditional. Return single `<Page title="Templates" maxWidth="narrow">{content}</Page>`. Note the named `"narrow"` variant — the 600px literal lives only in `global.css` as a design token (per Phase 1 §1), not in JSX.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.module.css`
**Changes**:
- Line 1: delete `.container` `max-width: 600px` (Page now owns this via the prop override).
- Delete `.title { margin: 0 0 var(--sp-5) }` rule.

#### 6. RootLayout padding rule strip

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css`
**Changes**:
- Line 16: change `.main { flex: 1; overflow: auto; padding: var(--sp-5) var(--sp-6); }` to `.main { flex: 1; overflow: auto; }`.

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.test.tsx` (if exists; otherwise skip)
**Changes**: Verify no test asserts the removed padding.

#### 7. PageSubtitle deletion

**Files to delete**:
- `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.tsx`
- `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.module.css`
- `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.test.tsx` (if exists)
- The `PageSubtitle/` directory itself.

**Search check**: `rg "PageSubtitle" skills/visualisation/visualise/frontend/src` returns no matches.

#### 8. migration.test.ts allowlist + REQUIRED list updates

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**:
- Line 122: remove `LibraryDocView.module.css 1100px` allowlist entry (consumes token now).
- Line 126: remove `LibraryTemplatesIndex.module.css 600px` allowlist entry. The 600px literal is replaced by `var(--ac-content-max-width-narrow)` consumed via `<Page maxWidth="narrow">` — the literal exists only once in the codebase, in `global.css`.
- Line 129: remove `LibraryTemplatesView.module.css 900px` allowlist entry (consumes token now).
- Line 154: remove `LifecycleIndex.module.css 900px` allowlist entry (consumes token now).
- Lines 394-412 (`REQUIRED` list): remove the three entries asserting `.title { color: var(--ac-fg-strong) }` for `LibraryDocView.module.css`, `LibraryTemplatesView.module.css`, and `LibraryTemplatesIndex.module.css` — `Page` now owns title styling.
- Verify line 147 (`LifecycleClusterView.module.css 800px`) and line 123 (`LibraryDocView.module.css 260px` aside column) stay — out of scope.

### Success Criteria:

#### Automated Verification:

- [ ] All updated route tests pass: `cd skills/visualisation/visualise/frontend && npm test -- routes/kanban routes/lifecycle/LifecycleIndex routes/library/LibraryDocView routes/library/LibraryTemplatesView routes/library/LibraryTemplatesIndex`
- [ ] Migration test passes with updated allowlist and REQUIRED list: `npm test -- styles/migration.test.ts`
- [ ] Full test suite passes: `npm test`
- [ ] Type check clean: `npm run typecheck`
- [ ] Lint clean: `npm run lint`
- [ ] No PageSubtitle imports remain: `rg "PageSubtitle" skills/visualisation/visualise/frontend/src` returns nothing.
- [ ] Playwright visual regression passes (some baselines will need regeneration for routes whose chrome changed): `npx playwright test`

#### Manual Verification:

- [ ] Visit each migrated route and confirm visual rendering matches pre-migration:
  - `/kanban` — Kanban title, "live" chip in actions slot, board layout unchanged below.
  - `/lifecycle` — Lifecycle title with toolbar in actions slot, cards render below.
  - `/library/<type>/<doc>` — Doc title in Page header with FrontmatterChips below; body + aside grid renders unchanged.
  - `/library/templates` — Templates title centred at 600px; list renders below.
  - `/library/templates/<name>` — Template name in Page header; tiers render below.
- [ ] Confirm horizontal padding is consistent across all routes (Page provides `var(--sp-6)` on both sides).
- [ ] Confirm overview hub from Phase 4 still renders correctly.
- [ ] Confirm `LibraryTypeView` from Phase 5 still renders correctly.
- [ ] Toggle dark mode across all migrated routes; confirm no regressions.
- [ ] Verify scroll behaviour: long content scrolls cleanly inside `<main>` (still has `overflow: auto`).

---

## Testing Strategy

### Unit Tests

- **Page wrapper**: Slot rendering, `*.module.css?raw` source-string assertions for token bindings (`var(--ac-content-max-width)` / `--ac-content-max-width-narrow` / `--sp-*`), narrow-variant class application. (Not `getComputedStyle` — jsdom returns the declared `var(...)` value, not the resolved length.)
- **Popover + useDismiss**: Open/close lifecycle, click-outside, Escape, Tab/Space/Enter activation, focus management with strict-mode guard, arrow-key navigation, trigger ARIA attributes (`aria-haspopup`/`-expanded`/`-controls`), second-popover dismissal via the module-level singleton.
- **`Indexer::library_aggregates`**: Empty index, multi-entry aggregation, latest-mtime correctness with deterministic `rel_path` tie-break, status-facet filtering on `frontmatter_state`, project-prefix derivation with fallback and `None` handling, empty-array selection semantics.
- **`parse_selection_query`**: 10 cases covering empty input, single key, repeated-key accumulation, multi-facet cross-population, empty-value drop, URL-encoded values, percent-encoded reserved chars, unknown doc-type drop, malformed shapes, unrelated query params.
- **`/api/library/structure` handler**: Phase ordering, facet emission per doc type (ADR-style vs work-items vs templates), 7/8/9-option boundary fixtures verifying option-array lengths, selection-scoped count tests (single-facet and two-facet cases), full-shape HTTP round-trip assertion.
- **`DocTypeKey` wire-token contract**: Per-variant `serde_json::to_value` and per-variant `from_wire_str` round-trip assertions over `DocTypeKey::all()`.
- **`formatDocId`**: Zero-padding, pre-formatted pass-through, null/empty handling, malformed-prefix defensive return, 5+ digit pass-through, alphanumeric prefix.
- **`SortPill` / `FilterPill`**: Menu lifecycle, option selection, OR/AND combinatorics, count rendering, search input above large facets (boundary at 8/9), FilterPill `isFetching` indicator true/false, `menuitemcheckbox` with `aria-checked` toggle-without-close.
- **`normaliseSelection`**: Empty cases collapse to `{}`, option-array sort canonicalisation, facet-key sort canonicalisation, undefined per-type handling.
- **`EmptyState` / `NoResultsPanel`**: Per-doc-type description lookup, data-driven completeness check across `DOC_TYPE_KEYS`, indexer footer literal derived from `dirPath`, Clear-filters callback, active-filters summary text.
- **`LibraryOverviewHub`**: Server-driven phase rendering, zero-doc card variant (dimmed + `aria-disabled`), Glyph + Link integration, cache-warm no-fetch assertion.
- **`LibraryTypeView`**: Five-column rendering, sort tie-breaker, doc-type-empty vs filter-empty branching.

### Integration Tests

- **Server smoke**: `/api/library/structure` returns valid JSON against the existing fixture index; phase order, counts, latest, facets all line up.
- **Frontend route**: `LibraryOverviewHub` mounted under the test router round-trips `useQuery` against a mocked fetcher.
- **Sidebar**: With a mocked `phases` prop, asserts the rendered phase grouping matches the data.

### Visual Regression

- Regenerate the 12 `pr-descriptions-*` glyph baselines in Phase 3 (rename via `jj mv`).
- Regenerate `/library` baseline in `tokens.spec.ts` after Phase 4 (was a redirect, now is the hub).
- Update `/library/decisions` baseline after Phase 5 (chrome changed substantially).
- Update baselines for each migrated route after Phase 6 (Page wrapper changed chrome).

### Manual Testing Steps

1. Walk every doc-type list view and confirm Page chrome, sort, filter, and column rendering.
2. Toggle a status filter on `decisions`; confirm rows filter, option counts in other facets update.
3. Drive the filter to no-results; click `Clear filters`; confirm full list returns.
4. Visit a zero-document doc type; confirm EmptyState card with correct copy.
5. Click a doc-type card on `/library`; confirm navigation to `/library/<type>` works.
6. Toggle light ↔ dark mode on every screen; confirm no token-violation regressions.
7. Resize the browser through 640px and 1024px breakpoints on `/library`; confirm grid column count.
8. Confirm `KanbanBoard` "live" chip still shows in the success branch.
9. Confirm `LibraryDocView`'s aside column still aligns next to the body after the grid restructure.
10. Confirm `LibraryTemplatesIndex` is centred at 600px (narrower than other pages).

## Performance Considerations

- `Indexer::library_aggregates(selection)` does two passes over `entries.values()` under a single `entries.read().await` (first pass for counts/latest, second per-doc-type pass for facet scoping). Same complexity class as `counts_by_type` × number of facets per doc type — at v1 scale (low thousands of entries, ≤3 facets per type), still well under 10ms even cold. The selection-aware scoping does add cost on every keystroke-equivalent (every toggle refetches), but React Query caches per selection so back-and-forth toggling is free after the first request. No caching at this stage — recompute per request. An inline `// PERF:` comment on `library_aggregates` documents the trigger for migrating to the `state.clusters`-style cached pattern.
- `/api/library/structure` allocates the response per-request with serde. Response size scales with (phase count × doc-type count × facet-option count). A meta directory with 13 doc types and ~50 unique slugs per type produces a response well under 100KB. Acceptable for a single-fetch-per-page-load.
- Hand-rolled popover positioning uses `getBoundingClientRect` only when the popover opens. No layout-thrash risk.
- Five-route Page migration adds zero new render cost — `Page` is a presentational wrapper around existing route content.

## Migration Notes

- **Breaking redirect removal: `/library` → `/library/decisions`.** Users with bookmarks, links in docs, screenshots, or muscle-memory targeting `/library` will land on the new overview hub rather than the decisions list view. Deep links to `/library/decisions` itself continue to work. No external scripts observed in the codebase, but the broader user-facing impact (bookmarks, internal docs) cannot be enumerated from the code alone — flag in release notes.
- **Stale-bundle compat after PR-descriptions rename.** Browser tabs holding a cached pre-upgrade frontend bundle that reconnect to the upgraded server will receive `pr-descriptions` wire tokens that the old `isDocTypeKey` filter rejects, causing PR descriptions to disappear from the sidebar/lifecycle until the user reloads. Acceptable for a local dev tool; mention in release notes ("reload tabs predating the upgrade").
- **localStorage `seen-doc-types` key migration.** The `parseStored()` helper in `frontend/src/api/use-unseen-doc-types.ts` includes a one-shot in-place key rename (`"prs"` → `"pr-descriptions"`) so existing users do not lose their last-seen timestamps after upgrade. See Phase 3 §2.
- **No data migration needed** — all server changes are read-only / response-shape additive.
- **Visual regression baseline regeneration** is the only manual artifact step. Runner: `npx playwright test --update-snapshots tests/visual-regression/`. Verify byte-identity for the 12 renamed PR-descriptions baselines via `shasum -a 256` (Phase 3 §3).
- **Rollback strategy**: Phases 1, 2, 3 can be reverted independently. Phase 6 is the riskiest because it removes `RootLayout.main`'s padding atomically with five-route migration; revert means restoring both the `.main` rule and every route's `Page` adoption together.

## References

- Work item: `meta/work/0041-library-page-wrapper-and-overview-hub.md`
- Primary research: `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
- Supplementary research: `meta/research/codebase/2026-05-16-0041-library-page-wrapper-supplementary.md`
- Source design gap: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Design inventories: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/` (canonical screenshots)
- Blocker work items: 0033 (Design Token System) `done`, 0037 (Glyph Component) `done`, 0038 (Generic Chip Component) `done`
- Sibling work items: 0042 (Templates View Redesign) `draft`, 0044 (Spike: Confirm List-Screen Scope Decisions) `draft`
- Architecturally-relevant ADRs: 0024 (server-driven kanban column config — precedent for server-driven library structure), 0026 (CSS design-token application conventions)
- Similar implementation precedent: `meta/plans/2026-05-15-0038-generic-chip-component.md` (token + component + migration shape)
