---
date: 2026-05-15T18:41:25+01:00
researcher: Toby Clemson
git_commit: 77c4cfe1aa718e2ca1d381780dc170b1b3db9334
branch: HEAD (jj change stlzowrnoslm; main at f5c54374)
repository: accelerator
topic: "Library Page Wrapper, Overview Hub, and List Views (work item 0041) — codebase context"
tags: [research, codebase, visualiser, library, page-wrapper, sort, filter, server-driven, doc-types, popover]
status: complete
last_updated: 2026-05-15
last_updated_by: Toby Clemson
last_updated_note: "Resolved all eight open questions via Q&A with work-item author; updated work item 0041 to reflect decisions."
---

# Research: Work Item 0041 — Library Page Wrapper, Overview Hub, and List Views

**Date**: 2026-05-15T18:41:25+01:00
**Researcher**: Toby Clemson
**Git Commit**: 77c4cfe1aa718e2ca1d381780dc170b1b3db9334
**Branch**: HEAD (jj change `stlzowrnoslm`; `main` at `f5c54374`)
**Repository**: accelerator (visualisation-system workspace)

## Research Question

For the work item at `meta/work/0041-library-page-wrapper-and-overview-hub.md`, surface the current-state codebase context needed to plan implementation: the existing list view, the `PageSubtitle` it will subsume, the router redirect being removed, the client-side `PHASE_DOC_TYPES` constant being retired, the design-system primitives the new components depend on (Glyph / Chip / tokens), the server's `describe_types` surface that will be extended, the indexer fields backing counts and "latest", facet data sources, and the (absent) popover/checkbox/search-input precedents.

## Summary

The work item lands cleanly on the existing visualiser surfaces — every claimed file and line range checks out within ±10 lines, every blocker work item (0033 tokens, 0037 Glyph, 0038 Chip) is `done`, and every screenshot referenced in `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/` exists. Notable gaps and surprises:

- **No `--ac-content-max-width` token exists** in `tokens.ts`. Every route hardcodes its own `max-width` (allowlisted in `styles/migration.test.ts`). The work item assumes this token — it will need to be introduced as part of the Page wrapper work or replaced with a hardcoded value.
- **`--ac-bg-app` does not exist either.** The canonical app-background token is `--ac-bg`, applied at `body` level in `global.css:4`. The work item references `--ac-bg-app` "(or the canonical app-background token from 0033)" — `--ac-bg` is the answer.
- **Horizontal padding already lives at the shell** (`RootLayout.module.css`, `.main { padding: var(--sp-5) var(--sp-6) }`). The new `Page` wrapper must not re-add horizontal padding or `RootLayout.module.css` must change in lock-step.
- **`KanbanBoard.tsx` has three `PageSubtitle` call sites, not one** (lines 143, 152, 183-185). The shim re-export needs to keep all three working.
- **`router.test.tsx` tests three redirect cases** that depend on the removed redirect (lines 40-44, 46-49, 71-77), plus `indexRoute` at `router.ts:60` also redirects `/` → `/library`. `router-with-crumb.test.ts` does **not** depend on the redirect — only the pure `resolveCrumb` helper.
- **`LibraryTypeView` empty state is at line 135**, not 142; this is a single inline `<p>` with no `<th>` styling defined (`styles.badge` referenced but absent — silent no-op).
- **`statusToChipVariant` is NOT currently used in `LibraryTypeView`** — the status cell renders a bare `<span className={styles.badge}>`. The work item says "reuse `statusToChipVariant`"; in practice this is new wiring, not a reuse.
- **There is no `ID / DATE` column today** and no `{DOC_TYPE_PREFIX}-{0000}` formatter exists in the codebase. `IndexEntry.workItemId: string | null` carries the ID but is unused by `LibraryTypeView`. The work item's ID rendering is genuinely net-new code.
- **`PR descriptions` rename is a wider surface than the work item lists**: the kebab wire form `prs` appears in the server enum `DocTypeKey::Prs`, in `config_path_key()`, in `slug.rs` rules, in `clusters.rs`, in 12 `--ac-doc-prs` token references in `global.css`, and in `Glyph.tsx`. Server `label()` says `"PRs"` (plural); frontend `DOC_TYPE_LABELS['prs']` says `"PR"` (singular) — these already disagree. The rename to `pr-descriptions` (kebab) / `'PR descriptions'` (label) is a coordinated frontend+server change.
- **There is no popover/menu/click-outside/checkbox primitive anywhere in `frontend/src/`** and zero floating/popover libraries in `package.json`. Both the floating menu primitive and the checkbox row are genuinely net-new patterns. The codebase's house style (Sidebar, Topbar, SortButton) is "small hand-rolled primitives with CSS-module styling" — a hand-rolled popover fits that style better than introducing `@floating-ui/react`.
- **`cluster_slug` is not a frontmatter field** — it is the same string as `IndexEntry.slug`, derived per-doc-type by `slug::derive` in `server/src/slug.rs`. `clusters.rs` already buckets by it.
- **`project` is not on `IndexEntry`** — it is derivable from `IndexEntry.work_item_id` (prefix before the dash) or from `config.work_item.default_project_code` (`config.rs:70`). The server has to do this computation when building the work-items filter facet.

The work item is well-scoped, internally consistent, and lands on a stable foundation. The main implementation risks are the three net-new primitives (Page wrapper, popover, server `/api/library/structure` handler) — each of which is dependency for several others. The work item already captures the internal ordering.

---

## Detailed Findings

### Frontend — `LibraryTypeView` and table chrome

File: [`skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx`](skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx)

**Sort state** (`LibraryTypeView.tsx:50-51`) lives entirely in component-local `useState`:

```tsx
const [sortKey, setSortKey] = useState<SortKey>('mtime')
const [sortDir, setSortDir] = useState<SortDir>('desc')
```

- `type SortKey = 'title' | 'slug' | 'status' | 'mtime'` (`:13-14`); default `mtime` / `desc`.
- `toggleSort` (`:71-74`) flips `sortDir` on active-column re-click; resets to `asc` on column switch.
- `SortHeader` (`:143-166`) renders a `<button type="button" className={styles.sortButton}>` inside each `<th>` with an inline `▲ / ▼` arrow.
- Comparator `sortEntries` (`:24-38`) sorts on `title`, `slug ?? ''`, `statusCellValue(entry)` (which reads `frontmatter.status ?? frontmatter.date ?? ''`), or `mtimeMs`.
- ARIA: `ariaSortFor` (`:83-84`) sets `aria-sort` to `'ascending' | 'descending' | 'none'`.

**No URL state.** No `validateSearch` on `libraryTypeRoute` (`router.ts:98-108`); no `useSearch` references in `routes/library/**`. Removing column-header click-sort breaks no URL contract — only in-component state is lost (and that already resets on remount).

**Table** (`:106-132`) has 4 columns — `Title`, `Status`, `Slug`, `Modified` — versus the target 5: `ID / DATE`, `TITLE`, `STATUS`, `SLUG`, `MODIFIED`. No ID column today; no DATE column; status currently precedes slug. Status cell:

```tsx
<td>
  <span className={styles.badge}>
    {statusCellValue(entry) || '—'}
  </span>
</td>
```

`styles.badge` is **not defined** in `LibraryTypeView.module.css` (only `.container`, `.table`, `.sortButton`, `.slug`, `.mtime`, `.empty`, `.error` exist). The class silently no-ops. `statusToChipVariant` is **not imported** here yet; the test `status-variant.test.ts:43` even anticipates the gap (`'returns neutral for ISO date strings (fallback used by LibraryTypeView)'`).

**Modified cell** (`:128-130`): `<td className={styles.mtime}>{formatMtime(entry.mtimeMs)}</td>`, imported at `:5`. `formatMtime` (`api/format.ts:19-27`) returns `'—'` for non-positive ms, `'just now'` for negative diff, then a `<n>s/m/h/d/w ago` ladder, then a locale date string for `>30d`.

**ID rendering**: There is no ID column. `entry.workItemId: string | null` is unused. No `formatWorkItemId` / `formatDocId` / `DOC_TYPE_PREFIX` helper exists anywhere. The slug column (`:127`) renders `entry.slug ?? '—'` raw.

**Empty state** at line **135** (work item said 142):

```tsx
{entries.length === 0 && <p className={styles.empty}>No documents found.</p>}
```

Rendered inside `<div className={styles.container}>`, after the `<table>` (so an empty `<tbody>` ships alongside).

**Data flow**: route → `useParams` (`:47-55`) → `useQuery({ queryKey: queryKeys.docs(type), queryFn: () => fetchDocs(type!) })` (`:65-69`) → `GET /api/docs?type=<type>` returns `DocsListResponse.docs: IndexEntry[]` (`api/fetch.ts:64-69`, `api/types.ts:64-84`). `useMarkDocTypeSeen` (`:60`) clears unseen markers. Sorting happens in a `useMemo` (`:78-81`) client-side.

`IndexEntry` (`api/types.ts:64-80`) carries: `type`, `path`, `relPath`, `slug`, `workItemId`, `title`, `frontmatter`, `frontmatterState`, `workItemRefs`, `mtimeMs`, `size`, `etag`, `bodyPreview`. Title is always a non-null string; slug is `string | null`.

### Frontend — `PageSubtitle`, `KanbanBoard`, router

File: [`frontend/src/components/PageSubtitle/PageSubtitle.tsx`](skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.tsx)

```tsx
import type { ReactNode } from 'react'
import styles from './PageSubtitle.module.css'

export interface PageSubtitleProps {
  title: string
  children?: ReactNode
}

export function PageSubtitle({ title, children }: PageSubtitleProps) {
  const hasChildren = children !== undefined && children !== null && children !== false
  return (
    <header className={styles.pagehead}>
      <h1 className={styles.title}>{title}</h1>
      {hasChildren && (
        <div className={styles.subtitle} data-slot="subtitle">{children}</div>
      )}
    </header>
  )
}
```

Shim contract: named export `PageSubtitle` (function), named export `PageSubtitleProps` interface, props `title: string` required + `children?: ReactNode` optional. DOM: `<header class={styles.pagehead}>` → `<h1 class={styles.title}>` + (optional) `<div class={styles.subtitle} data-slot="subtitle">`. The truthiness check explicitly admits `''` and `0` but rejects literal `false`.

**`KanbanBoard.tsx` has three consumer call sites**, not one:
- `KanbanBoard.tsx:143` (loading branch): `<PageSubtitle title="Kanban" />`.
- `KanbanBoard.tsx:152` (error branch): `<PageSubtitle title="Kanban" />`.
- `KanbanBoard.tsx:183-185` (success branch):
  ```tsx
  <PageSubtitle title="Kanban">
    <Chip variant="indigo">live</Chip>
  </PageSubtitle>
  ```

The shim must pass through both `title`-only and `title` + arbitrary children.

**Router** (`frontend/src/router.ts`):

- `libraryIndexRoute` (`:71-77`) — the redirect being removed:
  ```ts
  const libraryIndexRoute = createRoute({
    getParentRoute: () => libraryRoute,
    path: '/',
    beforeLoad: () => {
      throw redirect({ to: '/library/$type', params: { type: 'decisions' } })
    },
  })
  ```
- `libraryTypeRoute.parseParams` (`:101-104`) — invalid-doc-type fallback already targets `/library`, so once `libraryIndexRoute` becomes the hub it lands consistently. No code change required here.
- Sibling routes registered (`:159-173`): `libraryRoute` (parent), `libraryIndexRoute`, `libraryTemplatesIndexRoute`, `libraryTemplateDetailRoute`, `libraryTypeRoute.addChildren([libraryDocRoute])`. Comment notes literal-path specificity orders templates before generic `$type`.
- `indexRoute` at `router.ts:60` does `throw redirect({ to: '/library' })` — so the root `/` test will also need to update its terminal expectation.

**Tests**: `router.test.tsx` has three cases depending on the redirect:
- `:40-44` — `/` → `/library/decisions` (chains through `/library`).
- `:46-49` — bare `/library` → `/library/decisions` (the redirect being removed).
- `:71-77` — unknown type `/library/bogus` → `/library/decisions` (piggy-backs via the chain).

All three must change their terminal expectation from `/library/decisions` to `/library`. The crumb tests (`:127-214`) visit `/library/decisions` directly and are unaffected. `router-with-crumb.test.ts` only tests the pure `resolveCrumb` helper — no redirect dependency.

### Frontend — Doc-type metadata layer

File: [`frontend/src/api/types.ts`](skills/visualisation/visualise/frontend/src/api/types.ts)

`DocTypeKey` union (`:4-8`) — 13 keys: `decisions | work-items | plans | research | plan-reviews | pr-reviews | work-item-reviews | validations | notes | prs | design-gaps | design-inventories | templates`.

`DOC_TYPE_KEYS` (`:14-19`) — `readonly DocTypeKey[]`, drives `isDocTypeKey` (`:22-24`) and router `parseParams`.

`DOC_TYPE_LABELS` (`:35-49`) — `Readonly<Record<DocTypeKey, string>>`. Singular labels. Note: `'prs': 'PR'` here vs server `label()` `"PRs"` — already inconsistent.

`PHASE_DOC_TYPES` (`:228-254`) — shape is a `as const` tuple of objects `{ phase: string, label: string, docTypes: readonly DocTypeKey[] }`. Phase ids are inline string literals (`'define' | 'discover' | 'build' | 'ship' | 'remember'`). No exported `Phase` type. `Templates` is intentionally omitted. JSDoc at `:220-227` calls out promotion to a server-side definition "when a second consumer appears" — work item 0041 is that second consumer.

**Consumers**:
- `PHASE_DOC_TYPES`: production = `Sidebar.tsx:3,38,47` (sole); test = `Sidebar.test.tsx:267,279`.
- `DOC_TYPE_KEYS`: `use-unseen-doc-types.ts:11,137`, `Glyph.tsx:3,32`, `Glyph.test.tsx:5,38`, `styles/global.test.ts:15,233,235,242`.
- `DOC_TYPE_LABELS`: `ActivityFeed.tsx:10,50,57` (fallback when `useQuery` cold), `GlyphShowcase.tsx:3,30`, parity tests.

**`Sidebar.tsx`** consumption (`:38-81`):
- Outer `PHASE_DOC_TYPES.map(phase => …)` emits `<section>` per phase with `<h3>{phase.label.toUpperCase()}</h3>`.
- Inner `phase.docTypes.map((key: DocTypeKey) => …)` looks up live `DocType` via `byKey.get(key)` from a `Map` built off the `docTypes` prop (`:15-18`). Missing entries dev-warn and return null.
- Link: `<Link to="/library/$type" params={{ type: key }}>` (`:61-63`).
- Label rendered from `t.label` (server payload), NOT from `DOC_TYPE_LABELS`.
- **No glyph in the phase loop** — Sidebar only uses KanbanIcon/LifecycleIcon in its VIEWS section.
- Count at `:72-74`: `{t.count !== undefined && t.count > 0 && (<span className={styles.count}>{t.count}</span>)}`. Active state matches pathname `/library/${key}` or prefix (`:52-54`).

### Frontend — `statusToChipVariant` and `formatMtime`

[`frontend/src/api/status-variant.ts`](skills/visualisation/visualise/frontend/src/api/status-variant.ts):

```ts
const GREEN = new Set(['done','complete','accepted','approved','implemented','final','shipped'])
const INDIGO = new Set(['inprogress','reviewed','ready','active','proposed','live'])
const AMBER = new Set(['approvewithchanges','approvewchanges','review','revised'])
const RED = new Set(['blocked','rejected','deprecated','superseded','abandoned'])

function normalise(value: unknown): string {
  if (typeof value !== 'string') return ''
  return value.trim().toLowerCase().replace(/[\s_\-/]+/g, '')
}

export function statusToChipVariant(value: unknown): ChipVariant {
  const key = normalise(value)
  if (GREEN.has(key)) return 'green'
  if (INDIGO.has(key)) return 'indigo'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}
```

Normalisation strips whitespace, underscores, dashes, slashes; lowercases. `"In Progress"`, `"in-progress"`, `"in_progress"` all collapse to `"inprogress"`.

[`frontend/src/api/format.ts`](skills/visualisation/visualise/frontend/src/api/format.ts) lines 19-27:

```ts
export function formatMtime(ms: number, now: number = Date.now()): string {
  if (ms <= 0) return '—'
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0) return 'just now'
  const short = formatElapsedShort(diffSec)
  if (short !== null) return short
  if (diffSec < 30 * 86400) return `${Math.floor(diffSec / (7 * 86400))}w ago`
  return new Date(ms).toLocaleDateString()
}
```

### Frontend — design primitives (Glyph, Chip, tokens, layout shell)

**Glyph** (`frontend/src/components/Glyph/Glyph.tsx`):
- Signature: `Glyph({ docType, size, ariaLabel }: GlyphProps)`, where `docType: GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` (12 non-virtual types), `size: 16 | 24 | 32`, `ariaLabel?: string`.
- **Colour is automatic** — no colour prop. The `<svg>` sets `style={{ color: 'var(--ac-doc-${docType})' }}` (`Glyph.tsx:109`); children use `fill="currentColor"`. JSDoc explicitly forbids overriding `fill`/`color` on ancestors.
- 12 icon files in `frontend/src/components/Glyph/icons/` mapped at `:45-58`.
- Per-doc-type colours come from `tokens.ts:29-40` (light) and `:70-81` (dark), wired through `global.css:98-109` / `:197-208`.
- Helpers exported: `GlyphDocTypeKey`, `GLYPH_DOC_TYPE_KEYS`, `isGlyphDocTypeKey`.
- Typical consumer pattern (`ActivityFeed.tsx:122-124`): narrow with `isGlyphDocTypeKey`, then render with no colour/style props.

**Chip** (`frontend/src/components/Chip/Chip.tsx`, 32 lines):
- `ChipVariant = 'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'` (matches WI 0041 exactly).
- `ChipSize = 'sm' | 'md'` (default `sm`).
- `ChipProps = { variant, size?, leading?: ReactNode, 'aria-label'?: string, children: ReactNode }`.
- Renders `<span class={styles.chip} data-variant={variant} data-size={size}>` with optional `<span data-slot="leading">` and `<span class={styles.label}>` wrapper.
- Pill shape only (`border-radius: var(--radius-pill)`). Mono font, uppercase, `letter-spacing: 0.02em`, `1px solid var(--ac-stroke)`, `background: var(--ac-bg-raised)` overridden per variant.
- **Violet is light-mode-only** — dark theme block does not redeclare `--ac-violet`. WI 0041 doesn't request violet so this is fine, but worth knowing.

**Design tokens** (`frontend/src/styles/tokens.ts`, 164 lines; `frontend/src/styles/global.css`, 286 lines):

Token verification against WI 0041's CSS list:

| Required by WI 0041 | Status |
|---|---|
| `--ac-bg-card` | ✓ exists (`global.css:76` / `:177`) |
| `--ac-bg-app` | ✗ does not exist — canonical is `--ac-bg` (applied at `body` in `global.css:4`) |
| `--ac-fg-muted` | ✓ exists |
| `--ac-fg-strong` | ✓ exists |
| `--ac-stroke` | ✓ exists (siblings: `--ac-stroke-soft`, `--ac-stroke-strong`) |
| `--ac-content-max-width` | ✗ **does not exist** — each route hardcodes (1100/900/800/720/600px), allowlisted in `styles/migration.test.ts:68,122,126,129,134,147,154` |
| `--sp-6` | ✓ 32px |
| `--sp-5` | ✓ 24px |

Dark theme is declared three times in `global.css` (deliberate duplication, parity-asserted): MIRROR-A `[data-theme="dark"]` (`:171-212`), MIRROR-B `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` (`:218-259`), MIRROR-C `[data-font="mono"]` (`:266-269`). Any new tokens introduced by WI 0041 must be added to all three places.

**App layout shell** (`frontend/src/components/RootLayout/RootLayout.tsx:30-48`):

```tsx
<div className={styles.root}>
  <Topbar />
  <div className={styles.body}>
    <Sidebar docTypes={docTypes} />
    <main className={styles.main}>
      <Outlet />
    </main>
  </div>
</div>
```

`RootLayout.module.css` (18 lines):
```css
.root { display: flex; flex-direction: column; min-height: 100vh; font-family: var(--ac-font-body); }
.body { display: flex; flex: 1; }
.main { flex: 1; overflow: auto; padding: var(--sp-5) var(--sp-6); }
```

**Implications for the Page wrapper**:
- `.main` already applies `padding: var(--sp-5) var(--sp-6)` (24px vertical, 32px horizontal). The Page wrapper must not re-add horizontal padding or `.main`'s rule must change in lockstep. Recommend: Page applies vertical spacing (`--sp-5` gap between header and content) but no horizontal padding; or RootLayout's `.main` drops its padding so Page can own both.
- No max-width is applied at the shell — each route currently supplies its own `max-width: <px>; margin: 0 auto`. Centralising in Page either requires introducing `--ac-content-max-width` in `tokens.ts` + all three `global.css` mirror blocks + updating the migration allowlist, or hardcoding a value in `Page.module.css`.
- App background is set by `body { background-color: var(--ac-bg) }` in `global.css:4` — not by RootLayout. No `--ac-bg-app` token exists; use `--ac-bg`.

### Server — `describe_types`, indexer, facet sources

**`describe_types`** ([`server/src/docs.rs:115-132`](skills/visualisation/visualise/server/src/docs.rs)):

```rust
pub fn describe_types(cfg: &crate::config::Config) -> Vec<DocType> {
    let mut out = Vec::with_capacity(DocTypeKey::all().len());
    for key in DocTypeKey::all() {
        let dir_path = key.config_path_key()
            .and_then(|k| cfg.doc_paths.get(k).cloned());
        out.push(DocType {
            key, label: key.label().to_string(),
            dir_path, in_lifecycle: key.in_lifecycle(),
            in_kanban: key.in_kanban(), r#virtual: key.is_virtual(),
            count: 0,
        });
    }
    out
}
```

`DocType` struct (`docs.rs:90-113`, `#[serde(rename_all = "camelCase")]`) has: `key`, `label`, `dirPath`, `inLifecycle`, `inKanban`, `virtual`, `count`. Returned via `GET /api/types` from [`server/src/api/types.rs:14-21`](skills/visualisation/visualise/server/src/api/types.rs):

```rust
pub(crate) async fn types(State(state): State<Arc<AppState>>) -> Json<TypesResponse> {
    let mut types = describe_types(&state.cfg);
    let counts = state.indexer.counts_by_type().await;
    for t in &mut types { t.count = counts.get(&t.key).copied().unwrap_or(0); }
    Json(TypesResponse { types })
}
```

So `count` is already patched in by the handler — not by `describe_types`. The structure response will follow the same pattern.

**API directory layout** (`server/src/api/`):
- `mod.rs` — `mount()` router (`:23-44`), `ApiError`, `parse_kind`, `api_from_fd`.
- `activity.rs`, `docs.rs`, `events.rs`, `info.rs`, `kanban_config.rs`, `lifecycle.rs`, `related.rs`, `templates.rs`, `types.rs`, `work_item_config.rs`.
- New `library.rs` (or extending `types.rs`) is the natural home for `/api/library/structure`. `mount()` is at `api/mod.rs:23-44`. Top-level route registration is in `server/src/server.rs:165-170`.

**Indexer** ([`server/src/indexer.rs`](skills/visualisation/visualise/server/src/indexer.rs)):

`IndexEntry` (`:15-34`):
```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub r#type: DocTypeKey,
    pub path: PathBuf, pub rel_path: PathBuf,
    pub slug: Option<String>,
    pub work_item_id: Option<String>,
    pub title: String,
    pub frontmatter: serde_json::Value,
    pub frontmatter_state: String,
    pub work_item_refs: Vec<String>,
    pub mtime_ms: i64, pub size: u64, pub etag: String,
    pub body_preview: String,
}
```

Primary index: `entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>` (`:58`). No per-doc-type bucket — queries linearly filter `entries.values()`:

```rust
pub async fn all_by_type(&self, kind: DocTypeKey) -> Vec<IndexEntry>  // :316-324
pub async fn counts_by_type(&self) -> HashMap<DocTypeKey, usize>      // :326-332
```

`Templates` is skipped during `rescan` (`:127-129`) so it observes `count == 0` and is absent from both maps.

**Deriving "latest" per doc type**: `max(mtime_ms)` across `entries.values().filter(|e| e.r#type == kind)` — single pass per type, or one full pass building both `counts_by_type` and `latest_by_type` together (`HashMap<DocTypeKey, IndexEntry>` keyed by max mtime). The work item's "latest · {title}" preview requires the title from this entry too.

**Facet data sources**:

- **`status`** — `entry.frontmatter.get("status")` directly (frontmatter is `serde_json::Value`). No server-side normalisation today; values flow through as-is. The frontend's `statusToChipVariant.normalise` is the only normaliser, so option labels need to be canonicalised either server- or client-side. Cleanest: server returns raw status strings; frontend bucket-by-normalisation when rendering.
- **`cluster_slug`** — derived. There is NO `cluster_slug` field on `IndexEntry`. The "cluster slug" in WI 0041 is the same string as `IndexEntry.slug` (or `frontmatter.cluster_slug` — verify against actual data). `slug::derive` (`slug.rs:14-36`) computes it from filename. For work-items: `derive_work_item_with_regex` (`slug.rs:7-12`). For ADRs/plans: post-prefix portion of the filename stem. `clusters.rs:34` already buckets `e.slug.clone()` for cluster aggregation.
- **`project`** — NOT a frontmatter field, NOT on `IndexEntry`. Implicit in `IndexEntry.work_item_id` (e.g. `"PROJ-0042"`). Read prefix before the dash, or use `config.work_item.default_project_code` from `config.rs:70`. `WorkItemConfig::extract_id` (`config.rs:111-127`) is where IDs are formed.

**Route registration pattern**: `mount()` in `api/mod.rs:23-44` returns a `Router::new()` with `.route("/api/types", get(types::types))` and friends — one line per endpoint. Adding `/api/library/structure` is one `.route(...)` plus a `use` line.

**Doc type registry**: the Rust enum `DocTypeKey` in `server/src/docs.rs:4-20` with `#[serde(rename_all = "kebab-case")]` is the canonical source. `DocTypeKey::all()` (`:23-39`) returns the closed 13-element array. Per-variant metadata comes from `config_path_key()`, `label()`, `in_lifecycle()`, `in_kanban()`, `is_virtual()`. The set is hardcoded at compile time, NOT derived from filesystem layout.

### Popover / floating-menu / checkbox / search precedents

**Zero existing precedent** for floating menus, popovers, click-outside, Escape-to-close, or checkboxes anywhere in `frontend/src/`. Searches across `useFloating`, `@floating-ui`, `@radix-ui`, `headlessui`, `Popover`, `dropdown`, `Dropdown`, `useClickOutside`, `aria-haspopup`, `role="menu"`, `role="listbox"`, `Escape`, `keydown`, `type="checkbox"` all returned zero matches.

`package.json` (production deps only): `@dnd-kit/{core,sortable,utilities}`, `@tanstack/react-query` ^5, `@tanstack/react-router` ^1, `highlight.js` ^11, `react`/`react-dom` ^19, `react-markdown` ^9, `rehype-highlight` ^7, `remark-gfm` ^4. **No floating/popover library.**

**Closest existing precedent** for inline filter/sort UI: `frontend/src/routes/lifecycle/LifecycleIndex.tsx:72-87` (file is 143 lines, not 75-142 as WI 0041 says) — a flex `toolbar` containing inline `<input type="search">` (`:75-82`) + `Sort:` label + three `SortButton` toggles (`:83-86`, `SortButton` defined `:123-142` with `aria-pressed` and conditional active class). State is two `useState` hooks lifted into the page; `useMemo` derives `visible` from `sortClusters(filterClusters(...))`. The popover-driven version of WI 0041 replaces these inline toggles with menu-rendered options.

**Existing search inputs** (both raw `<input type="search">` with CSS module class):
- `LifecycleIndex.tsx:75-82` — `aria-label="Filter clusters"`, `placeholder="Filter…"`, controlled.
- `Sidebar.tsx:25-34` — wrapped in `searchRow` div with leading `<SearchIcon />` and trailing `<kbd>/</kbd>` slash-hint. Comment marks it as design-review placeholder; functional wiring deferred to work item 0054.

**Codebase house style**: small hand-rolled primitives with CSS-module styling (Sidebar inline SVG icons, hand-rolled `PipelineDots`, `TopbarIconButton`, `SortButton`). A hand-rolled popover (with own `useDismiss`-style hook for click-outside + Escape) fits this style better than introducing `@floating-ui/react`. Recommend co-locating the dismiss hook with the popover primitive.

### `PR descriptions` rename — full surface

All current usages use the OLD form. None of `pr_descriptions` / `pr-descriptions` / `PR descriptions` exists.

**Frontend** (`skills/visualisation/visualise/frontend/src/`):
- `api/types.ts:7,17` — `'prs'` in `DocTypeKey` union and `DOC_TYPE_KEYS` array.
- `api/types.ts:41` — label `'pr-reviews': 'PR review'`.
- `api/types.ts:45` — label `'prs': 'PR'`.
- `api/types.ts:192-193` — pipeline-step labels `'PR'`, `'PR review'`.
- `api/types.ts:247` — `docTypes: ['prs', 'pr-reviews']` in `PHASE_DOC_TYPES.ship`.
- `components/Glyph/Glyph.tsx:55` — `'prs': PrsIcon`.
- `components/Glyph/icons/PrsIcon.tsx` — icon file (rename file too).
- `components/Sidebar/Sidebar.test.tsx:40,44` — fixture labels `'PR reviews'`, `'PRs'`.
- `styles/tokens.ts:38,79` — token name `'ac-doc-prs'`.
- `styles/global.css:107,206,252` — `--ac-doc-prs` CSS custom properties (all three dark-theme mirror blocks).

**Server** (`skills/visualisation/visualise/server/src/`):
- `docs.rs:16` — enum variant `Prs`.
- `docs.rs:34,150` — array entries / test pairs.
- `docs.rs:52` — `config_path_key()` → `Some("prs")`.
- `docs.rs:70` — `label()` → `"PRs"` (plural; differs from frontend `DOC_TYPE_LABELS['prs'] = 'PR'`).
- `clusters.rs:76,123` — sort weight + `has_pr` mapping.
- `slug.rs:27,142` — slug rules.
- `indexer.rs:1862` — directory path `meta/reviews/prs` (this references the `pr-reviews` dir, NOT pr-descriptions — unaffected by rename).

The kebab wire form `prs` round-trips via `#[serde(rename_all = "kebab-case")]` (`docs.rs:5`); changing the variant to `PrDescriptions` flips the wire token to `pr-descriptions`. Frontend `DocTypeKey` union must match exactly.

**`config.doc_paths` key** is the `config_path_key()` return — currently `"prs"`. Renaming requires deciding whether the config key changes too (and whether the filesystem dir `meta/prs/` renames). The work item says "file path slugs" is in scope but doesn't say whether the on-disk dir renames; this is a question for planning.

---

## Code References

### Frontend — files the work item will touch
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:13-166` — full refactor: remove `SortHeader` / `toggleSort` / `sortEntries`, add Page wrapper, restructure columns, wire status chips, new empty state.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.module.css:1-32` — CSS module needs new classes; `.badge` removed; consider whether to keep file-local CSS or fold into a `Page.module.css`.
- `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.tsx` — becomes thin shim re-exporting Page.
- `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.module.css` — likely retained as Page's stylesheet; or replaced.
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.tsx:20,143,152,183-185` — three consumer call sites; should keep working unchanged via shim.
- `skills/visualisation/visualise/frontend/src/router.ts:60,71-77,101-104` — remove `libraryIndexRoute` redirect; replace with `component: LibraryOverviewHub` (new); leave fallback alone.
- `skills/visualisation/visualise/frontend/src/router.test.tsx:40-44,46-49,71-77` — three test cases need terminal-path updates from `/library/decisions` to `/library`.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:3,15-18,38-81` — migrate from `PHASE_DOC_TYPES` constant to server-driven structure response; keep `docTypes` prop OR switch to consuming the new structure query.
- `skills/visualisation/visualise/frontend/src/api/types.ts:228-254` — delete `PHASE_DOC_TYPES`; introduce `LibraryStructure` / `Phase` / per-doc-type-with-facets types.
- `skills/visualisation/visualise/frontend/src/api/fetch.ts` — add `fetchLibraryStructure` (new function paralleling `fetchDocs`).
- `skills/visualisation/visualise/frontend/src/api/query-keys.ts` — add `libraryStructure` key.
- `skills/visualisation/visualise/frontend/src/api/status-variant.ts:1-25` — already perfect; reuse unchanged.
- `skills/visualisation/visualise/frontend/src/api/format.ts:19-27` — already perfect; reuse unchanged.
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx` — reuse unchanged (colour automatic).
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx` — reuse unchanged.
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:108-120` — add `--ac-content-max-width` here if introducing.
- `skills/visualisation/visualise/frontend/src/styles/global.css:135-145,228-238` — add `--ac-content-max-width` in all three mirror blocks if introducing.
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:68,122,126,129,134,147,154` — update allowlist when routes start consuming the new token.
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css:7` — coordinate horizontal padding with new Page wrapper.

### Frontend — new files needed
- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx` (+ `.module.css`, `.test.tsx`) — generic page wrapper.
- `skills/visualisation/visualise/frontend/src/components/Popover/Popover.tsx` (+ `useDismiss` hook, CSS module, tests) — net-new floating-menu primitive.
- `skills/visualisation/visualise/frontend/src/components/SortPill/SortPill.tsx` (or under Library/) — pill + menu.
- `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.tsx` (or under Library/) — pill + facet sections + search + checkboxes.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.tsx` — new component for `/library`.
- `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.tsx` — doc-type-empty card.
- `skills/visualisation/visualise/frontend/src/routes/library/doc-type-id.ts` — `formatWorkItemId({DOC_TYPE_PREFIX}-{0000})` helper.

### Server — files the work item will touch
- `skills/visualisation/visualise/server/src/docs.rs:4-20,90-132` — either extend `DocType` (and `describe_types`) with phase/facet/latest fields, OR add new structure types.
- `skills/visualisation/visualise/server/src/api/types.rs:14-21` — either extend the handler or leave alone if adding a sibling.
- `skills/visualisation/visualise/server/src/api/mod.rs:23-44` — add new route line if adding a sibling handler.
- `skills/visualisation/visualise/server/src/api/library.rs` — new file (if sibling-handler path chosen).
- `skills/visualisation/visualise/server/src/indexer.rs:15-34,58,316-332` — add `latest_by_type` query method and facet-bucket helpers.
- `skills/visualisation/visualise/server/src/clusters.rs:34,76,123` — `cluster_slug` derivation already there; reuse or adapt.

### Server — `PR descriptions` rename surface (additional)
- `skills/visualisation/visualise/server/src/docs.rs:16,34,52,70,150` — enum variant + array + config key + label.
- `skills/visualisation/visualise/server/src/slug.rs:27,142` — slug rules.
- `skills/visualisation/visualise/server/src/clusters.rs:76,123` — sort + `has_pr` mapping.

## Architecture Insights

- **Server-driven by indexer, not config**. The doc-type registry is a hardcoded Rust enum (`DocTypeKey::all()`); counts, latest, and per-facet option-counts all derive from `Indexer.entries`. There is no separate config file or filesystem-discovery step. Promoting phase groupings server-side is a matter of authoring the structure response in Rust — there's no schema-loading layer to design.
- **Frontend is server-of-truth-mirroring**, not server-of-truth-locally. `DOC_TYPE_KEYS` and `DOC_TYPE_LABELS` already mirror the server enum; `PHASE_DOC_TYPES` is the lone holdout. The migration is a natural completion of an existing pattern.
- **Per-doc-type metadata layers**: `(a)` enum-level facts in `DocTypeKey` (compile-time), `(b)` config-level facts in `cfg.doc_paths` (runtime), `(c)` index-level facts in `Indexer.entries` (live data). The new structure response is just `(a)` + `(b)` + a derived `(c)` projection.
- **CSS module + token-only styling** is the house style. `Chip` and `Glyph` both follow it strictly. The work item's "no hard-coded colour or shadow values appear in the hub's CSS" requirement is reachable but requires confirming a new `--ac-content-max-width` token (or hardcoding) and accepting `--ac-bg` as the canonical app background.
- **Dark theme triplication**: any new token must be declared in three places in `global.css` (light `:root`, `[data-theme="dark"]`, `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])`). Tests in `styles/global.test.ts` enforce parity.
- **`RootLayout.main` already does container layout** (overflow + padding). The Page wrapper should NOT compete with this — recommended split: RootLayout owns viewport scrolling + outer padding; Page owns max-width centring + header/content separation + inner spacing.
- **No URL state currently** for library list views — sort and filter live entirely in component state. Introducing URL-backed sort/filter (e.g. `?sort=title&status=active`) is OUT of scope per the work item but easy to layer in later via TanStack Router `validateSearch`.

## Historical Context

- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` — source gap analysis; sections include Token Drift, Component Drift, Screen Drift, Net-New Features, Removed Features, Suggested Sequencing.
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md` — paired inventory of the Claude prototype design.
- `meta/research/design-inventories/2026-05-06-135214-current-app/inventory.md` — paired snapshot of the live app.
- `meta/plans/2026-04-22-meta-visualiser-phase-5-frontend-scaffold-and-library-view.md` — established the original `LibraryTypeView` scaffold.
- `meta/plans/2026-04-21-meta-visualiser-phase-3-file-driver-indexer-api.md` — Indexer and read-only API foundations the structure endpoint will extend.
- `meta/plans/2026-05-05-add-missing-templates-to-visualiser.md` — most recent doc-type-set change (added `templates` as virtual).
- `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` — precedent for server-driven config (kanban columns). Likely informs how a server-driven phase structure should be shaped.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` — token application conventions; constrains how new components consume `--ac-*` tokens.
- `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` — multi-field work-item cross-reference aggregation; relevant to how `work_item_refs` flows through `IndexEntry`.
- Blocker work items 0033, 0037, 0038 all `done`. Sibling 0042 (Templates View) `draft`. Siblings 0036 (Sidebar Redesign), 0040 (Pipeline Visualisation), 0043 / 0044 (list-screen spikes) all `draft` — may interact with this work item's scope; worth checking 0044 in particular for list-screen decisions that may have moved.

## Related Research

- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` — initial implementation context, config-read-path, directory discovery.
- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — design convergence workflow.
- `meta/research/codebase/2026-05-03-update-visualiser-for-work-item-terminology.md` — visualiser terminology update; relevant precedent for the `PR descriptions` rename mechanics.

## Open Questions

All eight open questions identified during research were resolved in a follow-up Q&A pass with the work-item author on 2026-05-15. Resolutions are captured in the work item's Drafting Notes (see `meta/work/0041-library-page-wrapper-and-overview-hub.md`) and the relevant Acceptance Criteria. Summary:

1. **`--ac-content-max-width` token** — **resolved**: introduce the token now in `tokens.ts` (LAYOUT_TOKENS, theme-invariant) + all three light/dark mirror blocks in `global.css` + update `migration.test.ts` allowlist as routes migrate. Canonical value TBD per route during implementation.
2. **Page wrapper / RootLayout padding** — **resolved**: `Page` owns both axes (horizontal + vertical padding + max-width). `RootLayout.main` drops its `padding: var(--sp-5) var(--sp-6)` rule. All five `<main>`-padded routes (`KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`) migrate to `Page` in this work item to preserve visual output.
3. **Popover primitive** — **resolved**: hand-rolled with co-located `useDismiss` hook (click-outside + Escape). Matches the codebase's hand-rolled small-primitives house style. Keyboard navigation (arrow keys, Enter, focus on open/close) is specified as a requirement of the primitive.
4. **`meta/prs/` directory rename** — **resolved**: directory stays at `meta/prs/`. Only the wire-token (`'prs'` → `'pr-descriptions'`, kebab-case to match every other doc type) and the Rust variant (`Prs` → `PrDescriptions` with `#[serde(rename = "pr-descriptions")]`) rename. `config_path_key()` continues returning `Some("prs")`. Deliberate but acceptable name asymmetry.
5. **Server `label()` vs frontend `DOC_TYPE_LABELS`** — **resolved by work item text**: both become `"PR descriptions"` canonically; the current inconsistency (server `"PRs"`, frontend `"PR"`) is resolved by aligning both to the new label.
6. **`cluster_slug` semantics** — **resolved**: server-derived from `IndexEntry.slug` (the kebab tail computed by `slug::derive`). `clusters.rs:34` already buckets by it. No new frontmatter field introduced.
7. **`project` facet derivation** — **resolved**: derive at request time from the prefix of `IndexEntry.workItemId` (e.g. `"PROJ-0042"` → `"PROJ"`), falling back to `config.work_item.default_project_code`. Per-request derivation is fine at expected index sizes.
8. **Sort secondary key** — **resolved**: `IndexEntry.workItemId` ascending when both entries have one, else `IndexEntry.relPath` ascending. Total comparator (no nulls reach the sort step).

## References

- Work item: `meta/work/0041-library-page-wrapper-and-overview-hub.md`
- Source research: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Design inventory: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/`
- Blocker work items: 0033 (Design Token System), 0037 (Glyph Component), 0038 (Generic Chip Component) — all `done`.
- Sibling work items: 0042 (Templates View Redesign), 0044 (Spike: Confirm List-Screen Scope Decisions) — both `draft`; verify ordering before/during planning.
