---
date: 2026-05-18T22:10:38+01:00
researcher: Toby Clemson
git_commit: ee201256c147a4f4e8b8f7427f9292fd79925767
branch: HEAD
repository: accelerator
topic: "Templates View Redesign (work item 0042)"
tags: [research, codebase, templates, sse, chip, page, etag, sha256, frontend, backend]
status: complete
last_updated: 2026-05-18
last_updated_by: Toby Clemson
---

# Research: Templates View Redesign (work item 0042)

**Date**: 2026-05-18T22:10:38+01:00
**Researcher**: Toby Clemson
**Git Commit**: ee201256c147a4f4e8b8f7427f9292fd79925767
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What is the current state of the codebase relative to work item 0042
(Templates View Redesign), and what specific changes are needed across
the frontend (templates index, detail screen, SSE consumption) and the
backend (templates detail endpoint, SSE event, watcher) to deliver the
acceptance criteria?

## Summary

The redesign sits on top of infrastructure that is largely in place.
The three-tier resolution model, the per-tier `etag: Option<String>`
pattern in `sha256-<hex>` form, the `SseHub` broadcast channel, the
`/api/events` endpoint, the `Chip` palette including `neutral`,
`indigo`, and `green` variants, the `Page` wrapper, the
`MarkdownRenderer`, and the singleton `useDocEventsContext` are all
present. What is missing breaks down into five concrete additions:

1. **Backend**: a new top-level `sha256` field on `TemplateDetail`
   serialised as `Option<String>` with
   `#[serde(skip_serializing_if = "Option::is_none")]`. Plus a new
   `SsePayload::TemplateChanged { template: String, sha256: String,
   timestamp: DateTime<Utc> }` variant that auto-serialises with the
   `"type":"template-changed"` discriminator. Plus watcher wiring so
   template-tier paths (currently in `template_extra_roots` but not
   passed to `watcher::spawn`) trigger broadcasts. Plus a mechanism to
   keep the cached `TemplateResolver` fresh on those changes (it is
   built once at startup and stored as `Arc<TemplateResolver>`).
2. **Frontend types**: extend `TemplateDetail` (api/types.ts) with
   `sha256?: string` and widen the `SseEvent` union with a
   `'template-changed'` variant; teach
   `use-doc-events.dispatchSseEvent` to invalidate
   `queryKeys.templateDetail(name)` and `queryKeys.templates()` for
   that variant.
3. **Frontend index view**: replace the single `<Chip
   variant="neutral">{activeTier}</Chip>` per row with a fixed
   three-tier inline indicator (plugin default → user override → config
   override) using the neutral / indigo / green Chip variants per
   presence state. This requires the index endpoint to expose enough
   per-tier presence data — `TemplateSummary` already includes `tiers:
   TemplateTier[]` so no backend change is needed.
4. **Frontend detail view**: convert the current single-column tier
   stack into a two-column grid (left: stacked tier card with the
   accent-coloured outline ring on the winning tier; right: a preview
   pane whose first row is a non-interactive content-hash label
   `sha256-<64-hex>`). Replace the inline `TierPanel` definition and
   add a `TemplatePreviewPane` analogue.
5. **Tests**: extend `api_templates.rs` to assert the `sha256` field
   shape (present vs omitted); extend the SSE harness to cover
   `template-changed`; rewrite the two route tests for the new index
   tier-presence row, the two-column detail layout, the
   accent-ring active marker, the content-hash label, and the
   non-interactive cursor/hover behaviour.

The work is therefore primarily presentational on the frontend
(composition over established primitives) plus a small but real
backend addition (one field, one SSE variant, watcher plumbing for
template paths). No new shared frontend component is strictly
required, but extracting a `TierPresenceRow` and a
`TemplatePreviewPane` keeps the route files readable.

## Detailed Findings

### Current frontend: templates index

- Route: `withCrumb('Templates', { path: '/templates', component: LibraryTemplatesIndex })`
  registered under `libraryRoute` with literal-path precedence over the
  generic `/$type` (skills/visualisation/visualise/frontend/src/router.ts:78-85).
- File: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`.
- Data: `useQuery({ queryKey: queryKeys.templates(), queryFn: fetchTemplates })`
  (lines 18-21). No `staleTime`/`enabled` options.
- Row rendering (lines 32-41): `<ul>`-of-`<li>` with `<Link>` and a single
  `<Chip variant="neutral">{TIER_LABELS[t.activeTier]}</Chip>`.
- States: loading text, error alert, success.
- Wrapped in `<Page title="Templates">` (line 46).
- `TIER_LABELS` (lines 11-15) duplicated in detail view — opportunity to
  hoist.
- CSS module: `.list`, `.error` only; the legacy `.active` class is
  explicitly asserted gone by a CSS regression test
  (LibraryTemplatesIndex.test.tsx:71-73).
- Fixtures in tests pass `tiers: []` (LibraryTemplatesIndex.test.tsx:11-14),
  i.e. the index view does not currently read per-tier presence at all.

The redesign needs:

- A three-tier presence row per template in the fixed order
  `plugin-default → user-override → config-override`. The data is
  already available on `TemplateSummary.tiers`
  (api/types.ts:97-101) where each tier carries
  `source / present / active`.
- For each tier, derive the Chip variant:
  - `!present` → `neutral`
  - `present && !active` → `indigo`
  - `present && active` → `green`

### Current frontend: templates detail

- Route: `withCrumb(({ params }) => params.name, { path: '/templates/$name', component: LibraryTemplatesView })`
  (router.ts:87-91).
- File: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx`.
- Data: `useQuery({ queryKey: queryKeys.templateDetail(name), queryFn:
  () => fetchTemplateDetail(name), enabled: !!name })`
  (lines 20-28). Falls back to `['template-detail', '__invalid__']`
  when name absent.
- Layout (lines 47-55): single column `<div className={styles.tiers}>`
  iterating `data.tiers` into a `TierPanel` per tier.
- `TierPanel` (lines 62-78) — inline component:
  - `<section>` with `styles.absent` conditional on `!tier.present`.
  - Header row: friendly label (`TIER_LABELS`), `<Chip variant="indigo">active</Chip>`
    when `tier.source === data.activeTier`, `<Chip variant="neutral">absent</Chip>`
    when `!tier.present`, `<code>{tier.path}</code>`.
  - Body: `<MarkdownRenderer content={tier.content} />` when present,
    otherwise a muted "Not currently configured." note.
- `<Page title={name | 'Loading…' | 'Template not found'}>` (lines 34-45).
- CSS module classes: `.tiers`, `.panel`, `.absent`, `.panelHeader`,
  `.tierLabel`, `.path`, `.absentNote`, `.error`. The legacy
  `.activeBadge` is asserted gone (LibraryTemplatesView.test.tsx:84-86).

The redesign needs:

- Top-level two-column grid inside `Page`'s content slot, e.g.
  `display: grid; grid-template-columns: <left> 1fr; gap: var(--sp-5)`,
  with no responsive collapse (≥1024px assumption per AC).
- Left column: the existing stacked tier card. The winning tier is
  marked with an "accent-coloured outline ring" — replace the current
  indigo `active` chip with an accent ring around the active tier
  section (e.g. `outline: 2px solid var(--ac-accent); outline-offset:
  …`). The work item explicitly preserves the indigo `active` semantic
  ring as the existing marker form, even though prior code rendered it
  as a chip.
- Right column: the new `TemplatePreviewPane`:
  - First row: a non-interactive content-hash label showing
    `sha256-<64-hex>` (literal prefix + lowercase hex). No hover bg,
    no tooltip, no click handler, no copy/select helper, default
    cursor. Renders only when `data.sha256` is present.
  - Below: the rendered winning-tier markdown (`<MarkdownRenderer
    content={winningTier.content} />`). Note: the active/winning tier
    is the one where `tier.source === data.activeTier && tier.present`.

### Current frontend: data fetching and types

- `api/fetch.ts:104-114` defines `fetchTemplates` and
  `fetchTemplateDetail`. Both throw `FetchError` on non-2xx.
- `api/query-keys.ts:48-49`: `templates() → ['templates']`,
  `templateDetail(name) → ['template-detail', name]`.
- `api/types.ts:86-111`:
  - `TemplateTierSource = 'config-override' | 'user-override' | 'plugin-default'`
  - `TemplateTier = { source, path, present, active, content?, etag? }`
  - `TemplateSummary = { name, tiers, activeTier }`
  - `TemplateDetail = { name, tiers, activeTier }` — needs
    `sha256?: string` added.

### Current backend: templates module

- `skills/visualisation/visualise/server/src/templates.rs`.
- Types (camelCase serde):
  - `TemplateTierSource` (lines 9-15) — kebab-case serde
    (`config-override`, `user-override`, `plugin-default`).
  - `TemplateTier` (lines 17-28) — fields `source / path / present /
    active / content?: String / etag?: String`. The `etag` is in
    `sha256-<64-hex>` form, set from `FileContent.etag` (lines 158-172)
    which comes from `file_driver::etag_of` (file_driver.rs:480-484).
  - `TemplateSummary` (lines 30-36) — list-shape: `name`, `tiers`,
    `active_tier` → JSON `activeTier`.
  - `TemplateDetail` (lines 38-44) — currently identical to
    `TemplateSummary`. **No top-level `etag` or `sha256` field today.**
- Resolution algorithm (`TemplateResolver::build`, lines 51-107):
  pushes three tiers in fixed order `[ConfigOverride, UserOverride,
  PluginDefault]`; selects active via
  `ordered.iter().find(|t| t.present).map(|t| t.source)` (first-present
  by declaration order = highest-priority); flags `active` on that
  tier. Falls back to `PluginDefault` even when not present.
- `detail()` (lines 137-149) clones the entire cached tiers vector, so
  the response already includes per-tier `content` for every present
  tier.
- Confirmed by inline test `list_omits_content_but_detail_includes_it`
  (lines 261-277): list strips `content`, detail keeps it.
- Per-tier `etag` stability is already tested
  (`etag_is_stable_across_reads`, lines 289-302).

### Current backend: SSE hub and events endpoint

- `skills/visualisation/visualise/server/src/sse_hub.rs`.
- `SsePayload` (lines 15-32) uses `#[serde(tag = "type", rename_all =
  "kebab-case")]` so variant `DocChanged → "doc-changed"`,
  `DocInvalid → "doc-invalid"`. **A new variant
  `TemplateChanged` will serialise to `"template-changed"`
  automatically.**
- Each variant's fields use `#[serde(rename_all = "camelCase")]` with
  per-field `rename` for `docType`. `etag` is `Option<String>` with
  `skip_serializing_if = "Option::is_none"` — set to
  `Some("sha256-<hex>")` on create/edit, `None` on delete; the field
  is omitted from JSON when `None`.
- `SseHub` wraps `tokio::sync::broadcast::Sender<SsePayload>` with
  `subscribe()` / `broadcast()` (lines 34-51).
- `api/events.rs:18-32` mounts `GET /api/events` and turns broadcast
  receivers into an `Sse::new(stream).keep_alive(KeepAlive::default())`
  with no `event:` line — the JSON `"type"` field is the
  discriminator.
- Construction call sites today: `watcher.rs:146-150,156-165,190-215`
  (file-driven) and `api/docs.rs:240-248` (PATCH-driven). All produce
  `DocChanged` or `DocInvalid` for the existing document doc-types.
- **Templates are not currently broadcast.** `DocTypeKey::Templates`
  is filtered out by the doc-types listing
  (`api/docs.rs:35-37`) and skipped by the indexer
  (`indexer.rs:272`). The structural ability to broadcast
  `DocChanged { doc_type: Templates }` exists but is unused.

### Current backend: watcher

- `skills/visualisation/visualise/server/src/watcher.rs`.
- `spawn(dirs, …)` creates a `notify::RecommendedWatcher` watching
  each `dir` non-recursively (lines 28-98). The set of watched
  directories is `state.cfg.doc_paths.values()` only
  (`server.rs:286`).
- Tier paths for templates are computed by
  `file_driver::template_extra_roots(&cfg.templates)`
  (file_driver.rs:486-504) and passed to `LocalFileDriver` as
  `extra_roots` (server.rs:60) only for path-whitelisting — **never
  passed to `watcher::spawn`.**
- Even if template dirs were watched, the existing indexer path
  (`watcher::on_path_changed_debounced`, lines 101-170) wouldn't
  produce a `DocChanged` for templates because the indexer excludes
  `Templates` (indexer.rs:272), so `indexer.get(&path).await` returns
  `None`.
- **Implication for 0042**: the watcher needs a separate template
  branch — match path against `state.cfg.templates` tier paths and
  emit `SsePayload::TemplateChanged` directly. Or rebuild the
  `TemplateResolver` and broadcast; that requires turning
  `state.templates: Arc<TemplateResolver>` into an `ArcSwap`-style
  mutable handle (currently constructed once in `AppState::build`).

### Existing etag utility

- `file_driver::etag_of` (file_driver.rs:480-484): `pub fn etag_of(bytes:
  &[u8]) -> String` returning the prefixed form `sha256-<64-hex>`
  (71 chars total). Stability pinned by the test at lines 605-612.
- For the new top-level `sha256` field that must be a **bare**
  64-character lowercase hex digest (per the work item's spec for the
  JSON field), strip the prefix
  (`etag.strip_prefix("sha256-").unwrap_or(&etag)`) or compute
  `hex::encode(Sha256::digest(bytes))` directly. `sha2` and `hex` are
  already crate deps. Note: the per-tier `etag` field on the response
  retains the `sha256-` prefix to match existing semantics; only the
  new top-level `sha256` field is bare hex.

### Chip palette (0038)

- `components/Chip/Chip.tsx:4-14`:
  - `ChipVariant = 'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'`
  - `ChipSize = 'sm' | 'md'` (default `'sm'`).
  - Props: `variant`, `size?`, `leading?`, `'aria-label'?`, `children`.
  - Renders as a non-interactive `<span>` — no `onClick`, no hover/focus
    styling, no tooltip; suitable for the non-interactive content-hash
    label *as a base styling primitive*, but note the label is not
    actually a Chip in the work item — it's a separate static line of
    text.
- `Chip.module.css`:
  - `neutral` → muted fg + raised bg + stroke border.
  - `indigo` → `--ac-accent` text + `--ac-accent-faint` bg + `--ac-accent-tint` border.
  - `green` → `--ac-ok` text + 8% mix bg + 30% mix border.

### Page wrapper (0041)

- `components/Page/Page.tsx:4-11`:
  - Props: `eyebrow?`, `title`, `subtitle?`, `actions?`,
    `maxWidth?: 'default' | 'narrow'` (default = 1200px, narrow =
    600px), `children`.
- The body slot (`.content`) has no layout rules — composer supplies
  the grid. For 0042 use `display: grid; grid-template-columns: minmax(0, X) 1fr; gap: var(--sp-5)`.
- 0041 made `Page` the canonical owner of horizontal padding;
  `RootLayout.main` no longer pads. The templates routes already
  consume `Page`, so this redesign just changes content layout inside.

### Glyph (0037)

- `components/Glyph/Glyph.tsx:47-57`: `docType: GlyphDocTypeKey`
  (excludes `'templates'` — Glyph.constants.ts:19), `size: 16|24|32`,
  `ariaLabel?`, `framed?`.
- **There is no `templates` glyph by design.** If the redesign wants
  an icon next to each template row, either render a non-Glyph SVG or
  pick a doc-type the template targets (out of scope per work item —
  the work item does not require an icon).

### MarkdownRenderer

- `components/MarkdownRenderer/MarkdownRenderer.tsx:11-23`:
  - Props: `content: string`, `resolveWikiLink?`, `wikiLinkPattern?`.
  - Uses `remarkGfm` + `rehypeHighlight`. For template preview just
    pass `<MarkdownRenderer content={tier.content} />` with neither
    optional prop.

### SSE consumption (use-doc-events + ReconnectingEventSource)

- `api/use-doc-events.ts`:
  - Single shared instance via `DocEventsContext` (line 239),
    consumed via `useDocEventsContext()`.
  - Handle exposes `setDragInProgress`, `connectionState`,
    `justReconnected`, and `subscribe(listener) → unsubscribe`
    (lines 31-45). Multiple consumers should prefer `subscribe(...)`
    because `options.onEvent` is single-slot.
  - `dispatchSseEvent` (lines 80-111) is the central reducer mapping
    SSE events to `queryClient.invalidateQueries`. It needs a new
    branch for `'template-changed'` that invalidates
    `queryKeys.templateDetail(event.template)` and
    `queryKeys.templates()`.
  - Today `SseEvent` (api/types.ts:115-141) is a closed union of
    `doc-changed | doc-invalid`. Widen it with a third variant
    `{ type: 'template-changed', template: string, sha256: string,
    timestamp: string }`.
- `api/reconnecting-event-source.ts`:
  - Single `onmessage` field; no `addEventListener('template-changed', ...)`
    surface. All discrimination is JSON-based via the `type` field —
    consistent with how `doc-changed`/`doc-invalid` already work.
- Self-cause filter (api/self-cause.ts) operates on `etag` for
  `doc-changed`. For `template-changed` there is no write path from
  the frontend, so a self-cause filter is not needed.

### Design tokens of interest

- `--ac-accent` (light `#595fc8`, dark `#8a90e8`) — the accent-coloured
  outline ring for the winning tier.
- `--ac-accent-faint` / `--ac-accent-tint` — Chip indigo backgrounds.
- `--ac-ok` (green) — Chip green colour.
- `--ac-bg-card`, `--ac-stroke`, `--radius-md`, `--sp-4`/`--sp-5`,
  `--ac-font-mono` (for the monospace content-hash label).
- No grid utility tokens exist; use bare grid CSS inside the route's
  module.

### Test patterns

- HTTP integration (`tests/api_templates.rs:12-60`):
  `tempfile::tempdir()` → `common::seeded_cfg(tmp.path())` →
  `AppState::build` → `build_router(state)` →
  `app.oneshot(Request::builder().uri("/api/templates/...").body(Body::empty()).unwrap())`.
  Existing assertion at lines 56-60 finds the active tier in JSON;
  the new `sha256` field can be asserted with the same indexing
  style:
  ```rust
  assert_eq!(v["sha256"].as_str().unwrap().len(), 64);
  assert!(v["sha256"].as_str().unwrap().chars().all(|c| c.is_ascii_hexdigit() && (c.is_ascii_digit() || c.is_ascii_lowercase())));
  ```
  And the omission case: `assert!(v.get("sha256").is_none())`.
- SSE end-to-end (`tests/sse_e2e.rs:6-98`): boot full server,
  poll `server-info.json` for port, open `/api/events` via
  `reqwest`, mutate a watched file, substring-match incoming chunks
  for the kebab-case `"type"` value (`"template-changed"` here).
- Frontend route tests use a per-render `QueryClient` with
  `retry: false` wrapped in `MemoryRouter`; mocking via
  `vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(...)`.
  CSS regression assertions read `*.module.css?raw` and grep for
  legacy class names.

### Acceptance-criteria mapping

For each AC in `meta/work/0042-templates-view-redesign.md:101-160`:

- **AC1 (per-tier presence row on index in fixed order)** —
  LibraryTemplatesIndex.tsx must render
  `[plugin-default, user-override, config-override]` Chips inline per
  row. Source data exists on `TemplateSummary.tiers` already.
- **AC2 (chip-variant binding)** — derive
  `neutral | indigo | green` from `(present, active)`.
- **AC3 (no overrides → plugin default is winning)** — true for free
  given the resolver's first-present-wins (templates.rs:95-99).
- **AC4 (config override wins over user override)** — true for free
  given declaration order.
- **AC5 (two-column detail layout ≥ 1024px)** — CSS module update on
  LibraryTemplatesView.module.css.
- **AC6 (accent-ring active marker; non-active have no ring)** —
  replace the inline `<Chip variant="indigo">active</Chip>` with an
  outline-ring style applied to the active section.
- **AC7 (content-hash label as first row in preview pane;
  literal prefix + 64-hex; body follows immediately below)** — new
  `TemplatePreviewPane` component.
- **AC8 (backend `sha256` field on detail response;
  64-char lowercase hex of winning content)** — new
  `TemplateDetail.sha256: Option<String>` field.
- **AC9 (`sha256` omitted when winning content empty or absent;
  not `null` not `""`)** — `#[serde(skip_serializing_if =
  "Option::is_none")]`.
- **AC10 (content-hash label not displayed when `sha256` absent)** —
  conditional render in the preview pane.
- **AC11 (frontend label = backend `sha256` field exactly)** — frontend
  reads the field verbatim; no client-side hashing.
- **AC12 (SSE `template-changed` event with `{ template, sha256 }`
  updates the label within 1s without reload)** — new `SsePayload`
  variant + `use-doc-events` dispatch branch that invalidates the
  templateDetail query and lets TanStack Query refetch.
- **AC13 (label non-interactive — no cursor, hover bg, tooltip,
  click handler, copy helper)** — plain `<div>` or `<span>` with
  monospace styling; no `tabIndex`, no `title`, no event handlers,
  no `:hover` rule; explicit `cursor: default` is the default
  anyway.

### Risks and ambiguities (from the work-item review)

The Pass-4 review (`meta/reviews/work/0042-templates-view-redesign-review-1.md`)
approved the work item but flagged several minors worth resolving
during planning/implementation:

- "Stacked tier card" is ambiguous (one card with rows vs multiple
  cards). The existing implementation uses multiple `<section>` cards
  in a flex column — preserving that reads as "stacked tier cards"
  and is the lower-risk reading.
- "Existing active-tier marker, retained as-is" is ambiguous because
  the current marker is a chip, not a ring. The work item explicitly
  asks for an "accent-coloured outline ring" — treat the requirement
  text as authoritative over the "retained as-is" phrasing.
- Chip variant names "indigo / green" should be confirmed against
  0038's actual `ChipVariant` union — confirmed: those literal names
  exist (Chip.tsx:4).
- "Empty or absent winning-tier content" trigger condition for
  omission is not specified. The natural choice is: omit when
  `winning_tier.content` is `None` OR `winning_tier.content == Some("")`
  OR `winning_tier.present == false`. Document this in the plan.
- The tier-presence state matrix in the ACs covers (no overrides) and
  (user + config); the "user override only, no config override" case
  is implied but not asserted — add an explicit test.
- The SSE 1s latency boundary is not pinned (handler vs DOM). The
  existing system invalidates via TanStack Query then refetches; the
  observable update typically happens within the next tick after the
  refetch resolves.

## Code References

### Frontend (current)

- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:11-15` — `TIER_LABELS` mapping (duplicated in detail view).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:18-21` — TanStack Query loader for index.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:32-41` — per-row rendering with the single `neutral` chip showing the active tier label.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:20-28` — TanStack Query loader for detail.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:47-55` — outer `.tiers` container.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:62-78` — inline `TierPanel` with current active/absent markers.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:1-11` — current single-column layout to be replaced.
- `skills/visualisation/visualise/frontend/src/api/types.ts:86-111` — template types (needs `sha256?` added to `TemplateDetail`).
- `skills/visualisation/visualise/frontend/src/api/types.ts:115-141` — `SseEvent` union (needs `template-changed` variant).
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:104-114` — `fetchTemplates`, `fetchTemplateDetail`.
- `skills/visualisation/visualise/frontend/src/api/query-keys.ts:48-49` — `templates()`, `templateDetail(name)`.
- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:80-111` — `dispatchSseEvent` (add `template-changed` branch).
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx:4-14` — `ChipVariant` union.
- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:4-11` — `Page` props.
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:11-23` — preview body renderer.

### Backend (current)

- `skills/visualisation/visualise/server/src/templates.rs:9-44` — Rust types `TemplateTierSource`, `TemplateTier`, `TemplateSummary`, `TemplateDetail`.
- `skills/visualisation/visualise/server/src/templates.rs:51-107` — `TemplateResolver::build`, winning-tier selection at lines 95-102.
- `skills/visualisation/visualise/server/src/templates.rs:137-149` — `detail()` clones cached tiers.
- `skills/visualisation/visualise/server/src/templates.rs:158-172` — `load_via_driver` populates `etag` from `FileContent.etag`.
- `skills/visualisation/visualise/server/src/templates.rs:261-302` — inline tests for list/detail and per-tier etag stability.
- `skills/visualisation/visualise/server/src/api/templates.rs:25-34` — `template_detail` axum handler.
- `skills/visualisation/visualise/server/src/api/mod.rs:39-40` — routes mounted at `/api/templates` and `/api/templates/:name`.
- `skills/visualisation/visualise/server/src/api/mod.rs:26` — `/api/events` mount.
- `skills/visualisation/visualise/server/src/sse_hub.rs:7-32` — `ActionKind`, `SsePayload` with kebab-case discriminator.
- `skills/visualisation/visualise/server/src/sse_hub.rs:34-51` — `SseHub` API.
- `skills/visualisation/visualise/server/src/api/events.rs:18-32` — SSE endpoint.
- `skills/visualisation/visualise/server/src/file_driver.rs:480-484` — `etag_of`.
- `skills/visualisation/visualise/server/src/file_driver.rs:486-504` — `template_extra_roots`.
- `skills/visualisation/visualise/server/src/watcher.rs:28-98` — `spawn` (only watches `cfg.doc_paths`).
- `skills/visualisation/visualise/server/src/watcher.rs:101-170` — `on_path_changed_debounced`.
- `skills/visualisation/visualise/server/src/watcher.rs:146-150,156-165,190-215` — emit sites.
- `skills/visualisation/visualise/server/src/server.rs:60` — `template_extra_roots` wired into the driver only.
- `skills/visualisation/visualise/server/src/server.rs:286` — `watch_dirs = state.cfg.doc_paths.values().cloned().collect()`.
- `skills/visualisation/visualise/server/src/api/docs.rs:240-248` — PATCH-driven `DocChanged` broadcast (reference for the etag-set pattern).
- `skills/visualisation/visualise/server/tests/api_templates.rs:12-60` — HTTP test fixture pattern.
- `skills/visualisation/visualise/server/tests/sse_e2e.rs:6-98` — SSE e2e fixture pattern.

## Architecture Insights

- **Templates are a virtual doc-type with separate plumbing.** They
  appear in `VIRTUAL_DOC_TYPE_KEYS` (`api/types.ts:30`),
  `DocTypeKey::Templates` is excluded from the indexer
  (`indexer.rs:272`) and the doc-types API
  (`api/docs.rs:35-37`), the `LocalFileDriver` whitelists their tier
  paths via `extra_roots`, and the templates handler reads everything
  through `TemplateResolver`. This separation is why a sibling
  `template-changed` SSE event is more natural than overloading
  `doc-changed { docType: 'templates' }`.
- **The `etag` plumbing follows a hub-and-spoke pattern.** Every
  content surface that exposes a hash uses `Option<String>` with
  `#[serde(skip_serializing_if = "Option::is_none")]`, in
  `sha256-<hex>` form, originating from `etag_of`. The new top-level
  `sha256` field on `TemplateDetail` is a deliberate departure (bare
  hex, no prefix) — keep this distinction explicit in the response
  type and document it.
- **SSE discriminator is JSON, not the SSE `event:` line.** The
  EventSource onmessage handler relies on `event.type` from the
  parsed JSON to dispatch. Server-side renames flow via
  `#[serde(tag = "type", rename_all = "kebab-case")]` so a new Rust
  variant `TemplateChanged` is wire-correct for free.
- **Templates view uses TanStack Query, not Redux/RTK Query.** SSE
  invalidates queries; no manual cache mutation. The 1s SSE→UI bound
  in the AC depends on query invalidation latency, which is normally
  sub-frame in tests; risk lies more in dispatcher correctness than
  in network latency.
- **The watcher debounces per path and consults a `WriteCoordinator`
  to suppress self-writes.** For template-changed broadcasts that
  originate from the file system (not from API writes — there is no
  templates write endpoint today), the self-cause path is moot.
- **`TemplateResolver` is built once at startup
  (`Arc<TemplateResolver>` in `AppState`).** Live updates either
  require an interior-mutability wrapper (`ArcSwap<TemplateResolver>`
  or `tokio::sync::RwLock`) or a narrower cache invalidation strategy
  (rebuild on each detail request when an `etag` is stale, with the
  watcher merely poking the SSE channel and a "next request rebuilds"
  flag). The simpler design is `ArcSwap`: on a template-tier file
  change, rebuild the resolver, swap the `Arc`, then broadcast with
  the freshly computed `sha256`.

## Historical Context

- `meta/work/0029-template-management-subcommand-surface.md` — earlier
  templates work item that established the resolution model and CLI
  surface.
- `meta/work/0033-design-token-system.md` — design tokens used here
  (Sora display font, Inter body, Fira Code mono, `--ac-*` palette).
- `meta/work/0037-glyph-component.md` — Glyph component; intentionally
  has no `templates` doc-type icon.
- `meta/work/0038-generic-chip-component.md` — Chip palette including
  `neutral`, `indigo`, `green` variants the redesign relies on.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` — Page
  wrapper contract and the rule that `Page` owns horizontal padding
  (so the library templates routes must remain `Page`-wrapped).
- `meta/work/0055-sidebar-activity-feed.md` — established the
  `/api/events` SSE channel patterns and the `Option<String>` etag
  pattern that 0042 extends. Note: 0055 was historically classified
  as "assumed delivered" but functions as a hard infrastructural
  blocker for live updates.
- `meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md`
  — origin of `SseHub`, `notify`-based watcher, and the
  `DocChanged`/`DocInvalid` payload shapes.
- `meta/plans/2026-04-21-meta-visualiser-phase-3-file-driver-indexer-api.md`
  — origin of `etag_of` and the per-tier etag wiring.
- `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md`
  — definitive prior research on the SSE infrastructure the
  redesign extends.
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
  — Chip component prior research.
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
  — Page wrapper prior research.
- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
  — the original gap analysis listing the templates-view differences.
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
  — prototype inventory; templates view shown at
  `screenshots/templates-view.png`.
- `meta/reviews/work/0042-templates-view-redesign-review-1.md`
  — Pass-1..Pass-4 review of the work item; final verdict APPROVE
  with the minor risks listed under "Risks and ambiguities" above.

## Related Research

- `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md`
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
- `meta/research/codebase/2026-05-16-0041-library-page-wrapper-supplementary.md`
- `meta/research/codebase/2026-05-12-0037-glyph-component.md`
- `meta/research/codebase/2026-05-06-0033-design-token-system.md`
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`
- `meta/research/codebase/2026-03-29-template-management-subcommands.md`

## Open Questions

- **Resolver freshness on file changes.** Should the watcher
  (a) trigger a full `TemplateResolver` rebuild and `ArcSwap` it,
  (b) invalidate per-name and recompute lazily on next request, or
  (c) leave the resolver cached and emit only the `template-changed`
  SSE event, letting the client refetch through the existing detail
  handler (which still returns the cached resolver's stale view)? The
  third option is the simplest but defeats the purpose of live
  updates; (a) is the natural fit given how cheaply the resolver
  builds (small directory walks).
- **Bare hex vs prefixed hash on the wire.** The work item pins the
  top-level JSON field to bare 64-char lowercase hex while the
  existing per-tier `etag` keeps the `sha256-` prefix. Worth a single
  doc comment on `TemplateDetail.sha256` to flag the divergence to
  future readers.
- **Index endpoint shape unchanged?** The work item never asserts
  this, but the current `TemplateSummary` already includes everything
  the new index row needs. No `sha256` field on the index is
  necessary; only the detail response carries it. Confirm during
  planning that no AC implies otherwise.
- **Where the accent ring lives in the DOM.** The work item wants the
  ring around the winning tier *card* on the detail screen. The
  natural place is the active `<section>` in the left column, but
  spec doesn't pin whether the ring is `outline` (no layout impact)
  or `border` (layout impact). `outline` is preferred to keep card
  geometry stable.
- **Should `TierPresenceRow` be extracted to a shared component, or
  inlined in the index file?** The work item lists no sibling
  consumers, and the rule "don't add abstractions beyond what the task
  requires" suggests inlining unless tests get noisy.
