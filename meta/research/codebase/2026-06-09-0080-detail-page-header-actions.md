---
type: codebase-research
id: "2026-06-09-0080-detail-page-header-actions"
title: "Research: Detail-Page Header Actions (Open in Editor, Copy Path) — story 0080"
date: "2026-06-09T19:08:54+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0080"
parent: "work-item:0080"
topic: "Detail-page header actions: Open in editor and Copy path for LibraryDocView"
tags: [research, codebase, visualiser, detail-page, config, editor-deeplink, clipboard, toaster]
revision: "450c4de8765ccbe351e838e419e2621a01f5c047"
repository: "visualisation-system"
last_updated: "2026-06-09T19:08:54+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Detail-Page Header Actions (Open in Editor, Copy Path) — story 0080

**Date**: 2026-06-09 19:08 UTC
**Author**: Toby Clemson
**Git Commit**: 450c4de8765ccbe351e838e419e2621a01f5c047
**Branch**: HEAD (detached; jj workspace `visualisation-system`)
**Repository**: visualisation-system

## Research Question

For story `meta/work/0080-detail-page-header-actions.md`: how does the existing
codebase support wiring two right-aligned action buttons (`Open in editor`,
`Copy path`) into the `Page.actions` slot on the `LibraryDocView` route? What
already exists, and what is net-new? Specifically:

- The `Page.actions` slot and how `LibraryDocView` renders `<Page>`.
- The `TopbarIconButton` styling precedent and whether it can render an `<a>`.
- Whether the route already has the document's absolute and project-root-relative
  source paths (needed by `Copy path`, `{abs}`, `{rel}`).
- The visualiser config mechanism (`visualiser.binary` / `ACCELERATOR_VISUALISER_*`)
  and the exact template for adding `visualiser.editor` / `visualiser.editor_project`.
- The Toaster API (0039) and the absence of any clipboard helper.

> **Path note.** All source lives in the jj workspace checkout at
> `…/accelerator/workspaces/visualisation-system/`. Paths below are written
> relative to that workspace root. The visualiser app is under
> `skills/visualisation/visualise/` (`frontend/`, `server/`, `scripts/`), and
> the generic shell config layer is under the top-level `scripts/`.

## Summary

The story is **mostly a frontend wiring exercise plus a config-plumbing extension**;
the server already has everything `Copy path` and the editor templates need.

1. **`Page.actions` slot exists and is unused by `LibraryDocView`.** `Page` renders
   `actions?: ReactNode` into a `data-slot="actions"` div; `LibraryDocView` currently
   passes only `eyebrow`/`title`/`subtitle`/`children`. Wiring the slot is a one-prop
   addition.

2. **Both source paths are ALREADY on the route — zero net-new server work.** Every
   `IndexEntry` carries `path` (canonical **absolute** filesystem path) and `relPath`
   (project-root-**relative** path), both already serialized to the frontend and both
   already in scope inside `LibraryDocView` as `entry.path` / `entry.relPath`. The
   relative path is already rendered in the "File" aside; the absolute path is present
   but currently unused. This contradicts the story's hedged assumption that exposing
   the paths "may be server-side work" — it is not.

3. **`TopbarIconButton` cannot render an `<a>` today.** It is hardcoded to
   `<button type="button">`, requires a non-optional `ariaPressed` boolean (a
   toggle-only attribute), and has no `as`/`href` prop. The acceptance criterion that
   `Open in editor` render as a real `<a>` with an `href` therefore requires either
   extending `TopbarIconButton` to be polymorphic, or a sibling anchor component that
   reuses the `.toggle` CSS class. Its colour tokens already resolve to `--ac-*`
   (`--ac-fg-muted` rest → `--ac-fg-strong` hover/active), satisfying the token criterion.

4. **The config mechanism is real but the env→config precedence is hand-rolled per
   setting, not generic.** The file-pair precedence (`config.md` team ← `config.local.md`
   personal) is centralised and generic; the `ACCELERATOR_VISUALISER_*` env override is
   implemented ad hoc per key in the launcher. `idle_timeout` (story 0100) is the closest
   analog for a non-binary scalar and gives a precise template — but note `idle_timeout`
   is **server-only**; the field that is actually surfaced to the frontend over HTTP is
   `kanban_columns`. So `visualiser.editor` is a hybrid: follow `idle_timeout` for the
   launcher + `config.rs` hops, follow `kanban_columns`/`/api/kanban/config` for the
   API + frontend hops.

5. **The Toaster (0039) is ready; the clipboard helper is net-new.** `useToast()` →
   `showToast({ heading, message, kind })` is available app-wide (provider mounted in
   `RootLayout`), `info`/`ok` auto-dismiss after 5s, heading-only toasts are supported,
   and the `use-external-edit-toast.ts` hook is the canonical "domain hook fires a toast"
   pattern. There is **no** existing `navigator.clipboard` / `execCommand` usage and **no**
   `utils/` directory; pure helpers conventionally live in `src/api/` (e.g.
   `api/clipboard.ts`).

## Detailed Findings

### 1. The `Page.actions` slot and how `LibraryDocView` renders `<Page>`

- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:8` —
  `actions?: ReactNode` is declared on the props interface; destructured at line 17.
- `Page.tsx:36-38` — rendered conditionally with a `!== undefined` guard:
  ```tsx
  {actions !== undefined && (
    <div className={styles.actions} data-slot="actions">{actions}</div>
  )}
  ```
  (The guard is `!== undefined`, not truthiness, so `null`/`false` still render the
  wrapper. `data-slot="actions"` is a natural test anchor.)
- `Page.module.css:23-28` — `.headerTopRow` is a flex row, `justify-content: space-between`,
  `align-items: flex-end`, so actions are pushed right and bottom-aligned against the title.
- `Page.module.css:64-68` — `.actions` is `display: inline-flex; gap: var(--sp-2);
  align-items: center;` — multiple buttons sit in a row.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:188-196` —
  the current `<Page>` call passes only `eyebrow`, `title`, `subtitle`, `children`. No
  `actions` prop today:
  ```tsx
  return (
    <Page
      eyebrow={hasResolvedDocument ? <EyebrowLabel type={type} /> : undefined}
      title={title}
      subtitle={subtitle}
    >
      {body}
    </Page>
  )
  ```
- `LibraryDocView.tsx:186` — `hasResolvedDocument = Boolean(entry && content.data)`; the
  new buttons will likely want the same guard since they depend on a resolved entry/path.

**Conclusion**: wiring the slot is a single prop addition; no `Page` changes needed.

### 2. `TopbarIconButton` styling precedent — and the `<a>` gap

- `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.tsx:4-15` —
  props interface:
  ```tsx
  export interface TopbarIconButtonProps {
    ariaLabel: string
    ariaPressed: boolean   // REQUIRED, non-optional — emits aria-pressed always
    dataIcon: string       // → data-icon attribute (CSS + test targeting)
    children: ReactNode    // inline SVG glyph
    onClick: () => void
  }
  ```
- `TopbarIconButton.tsx:18-29` — **hardcoded `<button type="button">`**; no `as`/`href`/`role`
  prop, no anchor branch. It always emits `aria-pressed`, which is invalid on a link.
- `TopbarIconButton.module.css` — the `.toggle` rule resolves colours to `--ac-*` tokens:
  resting `color: var(--ac-fg-muted)` (line 8); hover `background: var(--ac-bg-hover);
  color: var(--ac-fg-strong)` (17-20); active `background: var(--ac-bg-active);
  color: var(--ac-fg-strong)` (22-25). `background: transparent` + `border: none` at rest;
  a `forced-colors` border rule at 29-33.
- Tokens are defined in `skills/visualisation/visualise/frontend/src/styles/global.css`
  (`--ac-fg-muted` ~line 90/352/425, `--ac-fg-strong` ~89/351/424, `--ac-bg-hover`,
  `--ac-bg-active`). Glyph SVGs use `stroke="currentColor"`, so they inherit the button's
  `color` token → satisfies the AC "computed color and glyph fill/color resolve to `--ac-*`".

**Existing consumers / the precedent to copy** (neither is mounted in `Page.actions` today —
both currently live in the topbar chrome):
- `components/ThemeToggle/ThemeToggle.tsx:9-31` — passes `ariaLabel`, `ariaPressed`,
  `dataIcon`, `onClick`, and an inline `<svg aria-hidden="true" width="16" height="16"
  viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" ...>` as children.
- `components/FontModeToggle/FontModeToggle.tsx:7-17` — same pattern.

**Glyph/icon system note**: `components/Glyph/Glyph.tsx` is **document-type-only** (keyed by
`DocTypeKey`; SVGs in `Glyph/icons/`). There is **no** generic `edit` or `link` UI icon
anywhere in the tree — header-action icons are supplied as inline `<svg>` children to
`TopbarIconButton` (the ThemeToggle/FontModeToggle pattern). New `edit` / clipboard glyphs
are net-new inline SVGs.

**Implication for the two buttons**:
- `Copy path` (a button) maps cleanly to `TopbarIconButton` via `onClick`; you must still
  pass an `ariaPressed` (currently required — `false` works but is semantically a toggle attr).
- `Open in editor` (a real `<a href>`) is **not supported**. Options: (a) make
  `TopbarIconButton` polymorphic (`as`/`href`, conditional `aria-pressed`); or (b) add a
  sibling anchor component reusing the `.toggle` class. Either way the `.toggle` CSS is the
  reusable piece. This is the single biggest "shape" decision for the frontend half.

### 3. The document's source paths are already on the route (no net-new server work)

`LibraryDocView` does **not** load a per-document "detail" payload. It fetches the whole
doc-type list and `.find()`s the entry client-side, then fetches raw body content by
relative path:

- `LibraryDocView.tsx:48-52` — `useQuery({ queryFn: () => fetchDocs(type!) })` → `IndexEntry[]`.
- `LibraryDocView.tsx:54-56` — `entry = entries.find(e => e.slug === fileSlug ||
  fileSlugFromRelPath(e.relPath) === fileSlug)`.
- `LibraryDocView.tsx:61` — `const { content, related } = useDocPageData(entry?.relPath)`.
- `LibraryDocView.tsx:133` — already renders `entry.relPath` in the "File" aside:
  `<p className={styles.meta}>{entry.relPath}</p>`.

**Path-like fields on the entry** (`frontend/src/api/types.ts:97-129`, `interface IndexEntry`):
- `path: string` (line 99) — the **absolute** filesystem path (server populates it canonical).
- `relPath: string` (line 100) — the **project-root-relative** path.
- `slug` (101), `workItemId` (102), `clusterKey` (128) — not filesystem paths.
- There is no separate `absolutePath`/`sourcePath` — `path` IS absolute, `relPath` IS relative.

**Server side** (both already serialized):
- `server/src/api/docs.rs:25-41` — `docs_list` returns `DocsListResponse { docs: Vec<IndexEntry> }`
  serialized as-is. `doc_fetch` (`docs.rs:43-95`, `GET /api/docs/{path}`) returns only raw
  bytes + etag — no path fields.
- `server/src/indexer.rs:162-198` — `#[derive(Serialize)] #[serde(rename_all = "camelCase")]
  struct IndexEntry`; `pub path: PathBuf` (166) → JSON `path`; `pub rel_path: PathBuf` (167)
  → JSON `relPath`.
- `server/src/indexer.rs:1324-1327` — `rel_path` computed as `path.strip_prefix(project_root)`
  (falls back to full path if prefix mismatch).
- `server/src/indexer.rs:259-267` — `project_root` is canonicalised once; the entry `path`
  is the canonical absolute path (`build_entry` at 339 / refresh at 459).
- `frontend/src/api/fetch.ts:87-92` — `fetchDocs` returns `IndexEntry[]` verbatim;
  `fetch.ts:94-100` — `fetchDocContent(relPath)` returns only `{ content, etag }`.

**Conclusion**: both `entry.path` (absolute, for `{abs}`) and `entry.relPath` (relative, for
`Copy path` raw form + `{rel}` percent-encoded) are already in memory inside `LibraryDocView`.
**No server, type, or API change is required** to surface them. The story's Assumptions/
Requirements hedge ("exposing them is server-side work … within scope") is over-cautious —
flag this in planning to drop that scope.

> Minor: the doc comment on `IndexEntry.path` (`types.ts:99`) does not state "absolute"; the
> value is correct but the comment could be annotated for clarity.

### 4. Config mechanism + the precise `visualiser.editor` template

**How precedence works today**:
- **File-pair precedence is generic** (centralised, reusable): `scripts/config-common.sh`
  defines the team/local file pair; `scripts/config-read-value.sh` reads `section.subkey`
  keys from YAML frontmatter, processing team then local (last-writer-wins, so
  `config.local.md` overrides `config.md`). `visualiser.editor` / `visualiser.editor_project`
  are read by this **unchanged** — both are just `visualiser.<subkey>`.
- **Env→config precedence is NOT generic** — it is hand-rolled per setting in the launcher.
  `ACCELERATOR_VISUALISER_BIN` (`launch-server.sh:107-110`) and the idle-timeout env override
  (`write-visualiser-config.sh:175-184`) each implement their own `if [ -n "${VAR:-}" ]` →
  `config-read-value.sh <key>` fallback. There is no shared `ACCELERATOR_VISUALISER_<KEY>`
  resolver. **You must add the env-override block yourself** for `editor`/`editor_project`.

**`idle_timeout` end-to-end (the template), workspace paths**:

1. **Launcher read + env + jq** — `skills/visualisation/visualise/scripts/write-visualiser-config.sh`:
   - env override + config fallback (lines 175-184): `IDLE_TIMEOUT="${ACCELERATOR_VISUALISER_IDLE_TIMEOUT:-}"`;
     if empty, `config-read-value.sh "visualiser.idle_timeout" ""`. Precedence: **env > config key > omit**.
   - jq write (227-282): `--arg idle_timeout "$IDLE_TIMEOUT"` (251) spliced **omit-when-empty**
     so an empty value drops the key and Rust's `#[serde(default)]` applies:
     `+ (if $idle_timeout == "" then {} else {idle_timeout: $idle_timeout} end)` (282).
     (`kanban_columns` by contrast is always present via `--argjson`.)
2. **`config-read-value.sh`** (`scripts/config-read-value.sh`) — splits `visualiser.idle_timeout`
   into section/subkey (32-39), awk-reads the indented subkey under the `visualiser:` header
   (56-91), team-then-local precedence (114-130). **No edit needed.**
3. **`config.rs`** (`skills/visualisation/visualise/server/src/config.rs`):
   - `#[serde(default)] pub idle_timeout: Option<String>` (38-43).
   - **`#[serde(deny_unknown_fields)]` (line 14)** — a new key in config.json with no struct
     field is a hard parse error, so the struct field and the jq key are coupled.
   - resolver `resolve_idle_limit_ms` (346-369); `ConfigError::InvalidIdleTimeout` (393-397).
   - **Every test that builds a literal `Config` must add the new field** (struct exhaustiveness
     + `deny_unknown_fields`). `idle_timeout: None` appears at ~12 sites — `server.rs:621`,
     `docs.rs:364,408`, `indexer.rs:1686,1856`, `api/kanban_config.rs:57`,
     `api/work_item_config.rs:51`, `api/events.rs:59`, `api/activity.rs:74`, plus `config.rs`
     test JSONs. Grep `idle_timeout: None` to find them all.
4. **Server wiring + API** — `idle_timeout` is **server-only** (feeds the lifecycle watcher;
   stored on `AppState` as `idle_limit_ms`, `server.rs:46/63-64/128-129`) and is **never sent
   to the frontend**. The config field that IS surfaced is `kanban_columns` via:
   - `server/src/api/kanban_config.rs:8-28` — `KanbanConfigBody`/`KanbanColumnDto`
     `#[derive(Serialize)]` + `get_kanban_config(State(state))` → `Json(...)`.
   - `server/src/api/mod.rs:5` — `mod kanban_config;`; `mod.rs:47` —
     `.route("/api/kanban/config", get(kanban_config::get_kanban_config))`.
5. **Frontend hook + type** — `frontend/src/api/use-kanban-config.ts:5-21` (response interface +
   fetch fn + React-Query hook with `staleTime: Infinity`); `frontend/src/api/types.ts:344-347`
   (`KanbanColumn` TS type); `frontend/src/api/query-keys.ts:58` (`kanban: () => ['kanban']`),
   optionally `SESSION_STABLE_QUERY_ROOTS` (70-73).

**Checklist to add `visualiser.editor` (+ `editor_project`)**:
1. `write-visualiser-config.sh` — add env+config read block (model 181-184) for both keys; add
   `--arg editor` / `--arg editor_project` (model 251) and omit-when-empty jq splices (model 282).
2. `config.rs` — add `#[serde(default)] pub editor: Option<String>` and
   `pub editor_project: Option<String>` to `Config`; update all ~12 `idle_timeout: None`
   literals to also set the new fields. (Editor strings are likely free-form passthrough — no
   resolver/`ConfigError` needed unless you validate.)
3. `server.rs` — optionally store on `AppState` (or read `state.cfg.editor` directly).
4. `server/src/api/editor_config.rs` **(NEW)** — `EditorConfigBody { editor, editor_project }`
   Serialize DTO + `get_editor_config` handler (model `kanban_config.rs`).
5. `server/src/api/mod.rs` — `mod editor_config;` + `.route("/api/editor/config", …)`.
6. `frontend/src/api/types.ts` — `EditorConfig` interface.
7. `frontend/src/api/use-editor-config.ts` **(NEW)** — fetch fn + `useEditorConfig` hook
   (model `use-kanban-config.ts`).
8. `frontend/src/api/query-keys.ts` — `editorConfig: () => ['editor-config']`; optionally add
   to `SESSION_STABLE_QUERY_ROOTS`.

> The preset-expansion + custom-template resolution rule (preset key → template; else
> `://`/`{abs}`/`{rel}` → verbatim) and the percent-encoding of `{abs}`/`{rel}`, plus the
> JetBrains `{project}` default-to-workspace-basename logic, are **all net-new** and not modelled
> by any existing code. They are pure frontend logic (a `src/api/editor-link.ts`-style helper)
> operating on the fetched `editor`/`editor_project` config plus `entry.path`/`entry.relPath`.
> The workspace basename for the JetBrains `{project}` default can be derived client-side from
> the absolute `entry.path` minus `entry.relPath`, or surfaced explicitly — a planning decision.

### 5. Toaster (0039) ready; clipboard helper net-new

- `frontend/src/api/use-toast.ts` — `ToastKind = 'info' | 'ok' | 'error'` (3);
  `ShowToastInput { heading: string; message: string; kind?: ToastKind }` (12-16);
  `showToast(input): number` (85-103). **No `duration` param** — auto-dismiss is fixed at
  `TOAST_AUTO_DISMISS_MS = 5_000` (26) for `info`/`ok`; `error` persists (97-99).
  Heading-only toasts supported (`Toaster.tsx:113-117` omits the body when `message === ''`).
  Consumer hook `useToast()` at 138-140.
- Minimal confirmation call:
  ```ts
  const { showToast } = useToast()
  showToast({ heading: 'Copied path to clipboard', message: '', kind: 'ok' })
  ```
- Provider mounted in `components/RootLayout/RootLayout.tsx:49` (`useToastDispatcher()`),
  `:84` (`<ToastContext.Provider>`), with `<Outlet/>` (where `LibraryDocView` renders) inside
  it (`:94-96`) and `<Toaster/>` at `:104-105`. So `useToast()` resolves to the real handle
  in `LibraryDocView`.
- **Domain-hook pattern** — `api/use-external-edit-toast.ts` is the canonical example: a
  module-scope heading constant + message builder, `const { showToast } = useToast()`, dispatch
  with a plain object (`:45-48`). `Toaster`'s `renderMessage` turns paired-backtick runs into
  inline `<code>` — so `` message: `\`${relPath}\`` `` would render the copied path as monospace.
  (A button handler can call `showToast` directly; the headless `ExternalEditToast` wrapper only
  exists because external edits arrive via SSE.)
- **No clipboard anything exists** — grep for `clipboard|execCommand|writeText` across `src/`
  returns nothing. **No `utils/` directory** exists; pure helpers live in `src/api/` (e.g.
  `api/path-utils.ts`, `api/format.ts`, `api/safe-storage.ts`). A net-new `api/clipboard.ts`
  (with the `navigator.clipboard.writeText` → `document.execCommand('copy')` fallback) and
  optionally an `api/use-copy-path-toast.ts` matches the layout.

### 6. Routing context

- `frontend/src/router.ts:45` imports `LibraryDocView`; `:112-113` registers it at
  `path: '/$fileSlug'` with `component: LibraryDocView`, nested under `/library` / `/$type`.
  The `type` and `fileSlug` route params are how the route resolves which entry to display.

## Code References

- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:8,17,36-38` — `actions` slot
- `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css:23-28,64-68` — header row + actions layout
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:48-61,133,186,188-196` — route data + current `<Page>` call + relPath render
- `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.tsx:4-15,18-29` — props + hardcoded `<button>`
- `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.module.css:7-33` — `--ac-*` token colours
- `skills/visualisation/visualise/frontend/src/components/ThemeToggle/ThemeToggle.tsx:9-31` / `FontModeToggle/FontModeToggle.tsx:7-17` — inline-SVG-as-children precedent
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx` — doc-type-only glyphs (no generic edit/link icon)
- `skills/visualisation/visualise/frontend/src/api/types.ts:97-129` — `IndexEntry` (`path` absolute, `relPath` relative); `:344-347` — `KanbanColumn`
- `skills/visualisation/visualise/server/src/indexer.rs:162-198,259-267,1324-1327` — `IndexEntry` serde + canonical path / rel_path computation
- `skills/visualisation/visualise/server/src/api/docs.rs:25-41` — `docs_list` returns entries verbatim
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:87-100` — `fetchDocs` / `fetchDocContent`
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:175-184,227-282` — idle_timeout env override + jq omit-when-empty
- `scripts/config-read-value.sh:32-39,56-91,114-130` — 2-level key read + team/local precedence
- `skills/visualisation/visualise/scripts/launch-server.sh:107-110` — `ACCELERATOR_VISUALISER_BIN` precedence (binary)
- `skills/visualisation/visualise/server/src/config.rs:14,38-43,346-369,393-397` — `deny_unknown_fields`, `idle_timeout`, resolver, error
- `skills/visualisation/visualise/server/src/api/kanban_config.rs:8-28` + `api/mod.rs:5,47` — config-to-frontend endpoint pattern
- `skills/visualisation/visualise/frontend/src/api/use-kanban-config.ts:5-21`, `query-keys.ts:58,70-73` — frontend config-fetch pattern
- `skills/visualisation/visualise/frontend/src/api/use-toast.ts:3,12-16,26,85-103,138-140` — toast API
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:49,84,94-96,104-105` — toast provider mount
- `skills/visualisation/visualise/frontend/src/api/use-external-edit-toast.ts:14-48` — domain-hook toast pattern
- `skills/visualisation/visualise/frontend/src/router.ts:45,112-113` — LibraryDocView route registration

## Architecture Insights

- **The "detail" object is just a matched `IndexEntry` from the bulk list**, not a dedicated
  per-doc endpoint. This is why both paths are conveniently already in hand — the same entry
  that drives the eyebrow/title/aside also carries `path` and `relPath`.
- **`deny_unknown_fields` couples the jq writer and the Rust struct.** Any new config key must
  land in both `write-visualiser-config.sh` and `config.rs` together, and every literal `Config`
  in tests must be updated. This is the main mechanical cost of the config extension.
- **The omit-when-empty jq splice is the idiom that makes `#[serde(default)] Option<String>`
  produce `None`.** Reuse it verbatim for `editor`/`editor_project` so "no editor configured"
  → field absent → `None` → button renders disabled (the story's out-of-box state).
- **Env precedence is per-setting, not framework-level** — don't assume a generic resolver;
  write the explicit `${ACCELERATOR_VISUALISER_EDITOR:-}` / `..._EDITOR_PROJECT` blocks.
- **`TopbarIconButton` is button-only and toggle-shaped** (`ariaPressed` required). The story's
  `<a href>` requirement for `Open in editor` is the one place the existing component does not
  fit; resolving that (polymorphic vs sibling component) is the key frontend design call.
- **Token + styling governance** is set by ADR-0026 (CSS design-token conventions),
  ADR-0035 (brand-layer indirection), ADR-0036 (typography), ADR-0039 (border-radius). The new
  buttons should consume `--ac-*` via the `.toggle` precedent rather than raw values.
- **Config governance**: ADR-0016 (userspace configuration model), ADR-0017 (configuration
  extension points), and ADR-0024 (visualiser kanban-column config — the precedent for a
  `visualiser.*` key reaching the frontend) bound how `visualiser.editor` should be introduced.

## Historical Context

- `meta/work/0080-detail-page-header-actions.md` — the story under research; its review at
  `meta/reviews/work/0080-detail-page-header-actions-review-1.md`. No plan or prior codebase
  research existed for 0080 before this document.
- Dependencies: `meta/work/0039-toaster-and-external-edit-notifications.md` (Toaster, with
  `meta/research/codebase/2026-05-27-0039-…md`, `meta/plans/2026-05-27-0039-…md`);
  `meta/work/0041-library-page-wrapper-and-overview-hub.md` (introduced `Page.actions`, with
  `meta/research/codebase/2026-05-15-0041-…md` + supplementary + plan);
  `meta/work/0035-topbar-component.md` (TopbarIconButton precedent, with research + plan).
- Sibling config work: `meta/work/0100-configurable-visualiser-auto-shutdown.md` (idle_timeout —
  the closest config template; `meta/research/codebase/2026-06-06-0100-…md`,
  `meta/plans/2026-06-06-0100-…md`, validation report). The richest source on
  `visualiser.binary` / `ACCELERATOR_VISUALISER_*` is
  `meta/research/codebase/2026-06-06-0101-unified-dev-task-for-visualiser.md`.
- Source design-gap: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  (with companion design inventories under `meta/research/design-inventories/2026-05-21-*/`).
- Config ADRs: `meta/decisions/ADR-0016-userspace-configuration-model.md`,
  `ADR-0017-configuration-extension-points.md`, `ADR-0024-visualiser-kanban-column-config.md`.
  Styling ADRs: `ADR-0026`, `ADR-0035`, `ADR-0036`, `ADR-0039`. No ADR exists for editor
  deep-links or clipboard — 0080 introduces these affordances with no prior decision recorded.

## Related Research

- `meta/research/codebase/2026-06-06-0100-configurable-visualiser-auto-shutdown.md` — sibling
  config mechanism (idle_timeout), the closest template for `visualiser.editor`.
- `meta/research/codebase/2026-06-06-0101-unified-dev-task-for-visualiser.md` — richest source
  on `visualiser.binary` / `ACCELERATOR_VISUALISER_*` / launcher.
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md` — origin of
  the `Page.actions` slot.
- `meta/research/codebase/2026-05-07-0035-topbar-component.md` — `TopbarIconButton` origin.
- `meta/research/codebase/2026-05-27-0039-toaster-and-external-edit-notifications.md` — Toaster.

## Open Questions

1. **`Open in editor` `<a>` shape** — extend `TopbarIconButton` to be polymorphic (`as`/`href`,
   conditional `aria-pressed`) vs. a sibling anchor component reusing `.toggle`? This is a
   planning decision; both satisfy the `--ac-*` token AC. (Research finding, not in the story.)
2. **Drop the "expose paths" scope** — the story hedges that surfacing `{abs}`/`{rel}`/relative
   paths "may be server-side work". It is **not**: `entry.path` and `entry.relPath` are already
   on the route. Planning should remove that scope item and the corresponding server work.
3. **JetBrains `{project}` default source** — the workspace basename can be derived client-side
   from `entry.path` minus `entry.relPath`, or surfaced explicitly from the server. The former
   needs no server change; confirm which is preferred.
4. **Where the editor config is surfaced** — a dedicated `/api/editor/config` endpoint (matching
   the per-concern `kanban_config` pattern) vs. folding into an existing config endpoint. The
   established pattern favours a dedicated endpoint.
5. **Preset/template + percent-encoding logic is entirely net-new** — no existing code models
   preset expansion, `{scheme}`/`{tag}`/`{project}`/`{abs}`/`{rel}` substitution, or
   percent-encoding. This is the bulk of the genuinely new frontend logic and should be a
   well-tested standalone helper (e.g. `src/api/editor-link.ts`).
