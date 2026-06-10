---
type: plan
id: "2026-06-09-0080-detail-page-header-actions"
title: "Detail-Page Header Actions (Open in Editor, Copy Path) Implementation Plan"
date: "2026-06-09T21:10:24+00:00"
author: Toby Clemson
producer: create-plan
status: complete
work_item_id: "work-item:0080"
parent: "work-item:0080"
derived_from: ["codebase-research:2026-06-09-0080-detail-page-header-actions"]
tags: [design, frontend, detail-page, config, editor-deeplink, clipboard]
revision: "d6c9d9734f3e62cd207ad3216924ebfb4acafa81"
repository: "visualisation-system"
last_updated: "2026-06-09T21:10:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Detail-Page Header Actions (Open in Editor, Copy Path) Implementation Plan

## Overview

Wire two right-aligned icon buttons into the existing `Page.actions` slot on the
`LibraryDocView` detail route:

- **Copy path** — copies the document's raw project-root-relative path
  (`entry.relPath`, forward slashes, not percent-encoded) to the clipboard via
  `navigator.clipboard.writeText` with a `document.execCommand('copy')` fallback,
  and confirms via a Toaster.
- **Open in editor** — a real `<a href>` editor deep-link computed from a
  configurable `visualiser.editor` (preset key or custom URL template) plus
  `visualiser.editor_project`. When no editor is configured the control renders as
  a disabled `<button>` whose tooltip names the `visualiser.editor` config field.

The work is a frontend wiring exercise plus a config-plumbing extension that
mirrors the existing `visualiser.idle_timeout` / `kanban_columns` precedent. The
server already exposes both source paths the feature needs.

> **Post-implementation revision (button presentation).** This plan's Phases 2–3
> built the two actions on an icon-only, polymorphic `TopbarIconButton`, per the
> work item's original "render via `TopbarIconButton`" AC. On review against the
> design prototype (`view-library.jsx` `DocPage`), the actions are **labelled
> pill buttons** — icon **and** visible text ("Open in editor" / "Copy path") —
> in the `ac-topbar__btn` family the current app realises as `SortPill` /
> `FilterPill`. The implementation was reworked accordingly: a dedicated labelled
> `DetailHeaderActions/HeaderActionButton` (button | anchor | disabled, prototype
> `ac-topbar__btn` styling) now backs both actions, and `TopbarIconButton` was
> reverted to its original icon-only form (its polymorphism would have been dead
> code). The icons were corrected to the prototype's exact `edit` glyph and a
> copy/clipboard glyph (the renamed "Copy path" action). The work item AC was
> updated to match. All other behaviour (anchor-for-gesture, the disabled
> sub-states, the scheme guard, config plumbing) is unchanged.

## Current State Analysis

- **`Page.actions` slot exists and is unused by `LibraryDocView`.** `Page` renders
  `actions?: ReactNode` into a `data-slot="actions"` div with a `!== undefined`
  guard (`Page.tsx:8,36-38`). `LibraryDocView` currently passes only
  `eyebrow`/`title`/`subtitle`/`children` (`LibraryDocView.tsx:188-196`).
- **Both source paths are already on the route.** Each `IndexEntry` carries
  `path` (canonical **absolute** filesystem path) and `relPath` (project-root-
  **relative** path), both serialized to the frontend and both already in scope as
  `entry.path` / `entry.relPath` inside `LibraryDocView` (`types.ts:97-129`,
  `LibraryDocView.tsx:54-61,133`). **No server, type, or API change is required to
  surface them** — this drops the story's hedged "exposing paths may be server-side
  work" scope item.
- **`TopbarIconButton` is button-only and toggle-shaped.** It is hardcoded to
  `<button type="button">`, requires a non-optional `ariaPressed`, and has no
  `as`/`href`/`disabled` prop (`TopbarIconButton.tsx:4-29`). Its `.toggle` CSS
  already resolves colours to `--ac-*` tokens (`TopbarIconButton.module.css:8,17-25`).
  The AC mandates both buttons "render via the `TopbarIconButton` component" **and**
  that `Open in editor` renders as an `<a>` — so the component must become
  polymorphic.
- **Config precedence is `idle_timeout`-shaped.** File-pair precedence
  (`config.md` ← `config.local.md`) is generic via `config-read-value.sh`; the
  `ACCELERATOR_VISUALISER_*` env override is hand-rolled per setting in
  `write-visualiser-config.sh` (`:181-184` for idle_timeout). `config.rs` uses
  `#[serde(deny_unknown_fields)]` (`:14`), so a new config key must land in the
  struct **and** the jq writer together, and every literal `Config` in tests must be
  updated. The omit-when-empty jq splice (`:282`) makes an absent value resolve to
  `None`. The config field reaching the frontend (`kanban_columns`) does so via a
  dedicated `/api/kanban/config` endpoint + `useKanbanConfig` hook.
- **Toaster (0039) is ready; clipboard + editor-link logic is net-new.**
  `useToast()` → `showToast({ heading, message, kind })` is available app-wide
  (`use-toast.ts:85-103,138-140`; provider in `RootLayout`). No `navigator.clipboard`
  usage exists and there is no `utils/` directory — pure helpers live in `src/api/`
  (`path-utils.ts`, `format.ts`, `safe-storage.ts`).

## Desired End State

On a resolved `LibraryDocView` route, the page header shows two icon buttons:

1. **Copy path** copies the raw relative path and shows a Toaster confirmation;
   works via `execCommand('copy')` when `navigator.clipboard` is unavailable.
2. **Open in editor** is an `<a>` with a computed `href` when an editor is
   configured (VS Code family, JetBrains preset, or custom template), and a disabled
   `<button>` with a `visualiser.editor`-naming tooltip when not.

Verification: the full Acceptance Criteria list in
`meta/work/0080-detail-page-header-actions.md` passes; automated suites (frontend
vitest, Rust `cargo test`, shell config tests) are green; manual checks confirm
clipboard, deep-link navigation, and the disabled state.

### Key Discoveries:

- `entry.path` (absolute, `{abs}`) and `entry.relPath` (relative, `Copy path` raw +
  `{rel}`) are already in memory in `LibraryDocView` (`types.ts:99-100`,
  `LibraryDocView.tsx:54-61`). No path-exposure work.
- `deny_unknown_fields` couples `write-visualiser-config.sh` ↔ `config.rs`; the
  `idle_timeout: None` literals (~12 sites) all need the new fields
  (`config.rs:43`; `kanban_config.rs:57`; grep `idle_timeout: None`).
- The omit-when-empty jq idiom (`write-visualiser-config.sh:282`) is what makes
  "no editor configured" → field absent → `None` → button disabled.
- `kanban_config.rs:8-28` + `api/mod.rs:5,47` + `use-kanban-config.ts:5-21` is the
  exact template for the new editor-config endpoint and hook.
- For JetBrains the AC example fixes tag = preset key (`web-storm` → `web-storm`),
  and VS Code-family scheme = preset key (`cursor` → `cursor`).
- Percent-encoding must preserve `/` separators: encode each path segment with
  `encodeURIComponent` and rejoin with `/` (`/a b/c.md` → `/a%20b/c.md`).

## What We're NOT Doing

- **No per-document detail endpoint or new path fields.** Both paths already ship on
  `IndexEntry`.
- **No line/column targeting** in editor deep-links — open at the top of the file.
- **No `file://` fallback** — browsers block `file://` from an http(s) origin; the
  unconfigured state is a disabled button, not a link.
- **No built-in default editor** — the feature ships disabled until configured.
- **No server-side validation of the editor string** — `editor` / `editor_project`
  are free-form passthrough; the frontend helper interprets them.
- **No other detail routes** — `LibraryDocView` only.
- **No Zed/Sublime presets** — reachable only via the custom-template escape hatch.
- **No consumption of work item 0100 code** — we extend the pre-existing
  `visualiser:` config mechanism, not 0100.

## Implementation Approach

Three independently mergeable phases, each TDD-driven:

1. **Config foundation** — extend the config plumbing (shell + Rust) with
   `visualiser.editor` / `visualiser.editor_project`, expose them through a dedicated
   `/api/editor/config` endpoint (server resolves the JetBrains project default), and
   add the frontend `useEditorConfig` hook. No UI change.
2. **`Copy path`** — make `TopbarIconButton` polymorphic (button | anchor |
   disabled, optional `ariaPressed`), add a clipboard helper, and wire the `Copy
   path` button into `Page.actions`. Frontend-only; no dependency on Phase 1.
3. **`Open in editor`** — add the `editor-link` helper (preset/template resolution +
   percent-encoding) and wire the `Open in editor` anchor/disabled-button into
   `Page.actions`, consuming the Phase 1 hook and Phase 2 anchor variant.

Each phase ends green and shippable: Phase 1 adds dormant config + an unused
endpoint; Phase 2 delivers a working Copy path button; Phase 3 completes the editor
deep-link.

### Deliberate tradeoffs (recorded for future maintainers)

- **Editor-config interpretation is split server/frontend.** The server resolves only
  the JetBrains project-name default (it owns `project_root`); preset expansion,
  template substitution, percent-encoding, and the scheme guard live in the frontend
  (`editor-link.ts`, pure and testable). This is intentional — the server stays a dumb
  passthrough — but a future resolution-rule change may touch both layers.
- **`{abs}`-based VS Code links assume a single shared filesystem view.** `entry.path`
  is the server's canonicalised absolute path; the deep-link is consumed by an editor
  on the user's machine. This holds for the localhost single-host deployment; it would
  break under any future non-local deployment or differing mount/symlink view.
- **One endpoint per frontend-facing config field.** `/api/editor/config` is the third
  `*_config` endpoint+hook+query-key quintuplet (after kanban). Following the precedent
  is the right call for this story (consistency over a speculative refactor); a
  consolidated `/api/visualiser/config` is worth a tracked refactor work item once a
  fourth field appears, not now.

---

## Phase 1: Config plumbing + `/api/editor/config` + frontend hook

### Overview

Add `visualiser.editor` and `visualiser.editor_project` end-to-end: shell writer →
`config.json` → typed `Config` → dedicated HTTP endpoint → frontend React-Query
hook. No UI is wired yet. The server resolves the final JetBrains project name
(configured value, else basename of canonical `project_root`).

### Changes Required:

#### 1. Shell config writer

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
**Changes**: Add env→config read blocks for both keys (model the `idle_timeout`
block at `:181-184`), then add `--arg` bindings and omit-when-empty jq splices
(model `:251,282`). No shape validation — free-form passthrough.

```bash
# Editor deep-link config. Precedence: env var > visualiser.editor config key >
# (omit → field absent → Rust None → button disabled).
EDITOR="${ACCELERATOR_VISUALISER_EDITOR:-}"
if [ -z "$EDITOR" ]; then
  EDITOR="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "visualiser.editor" "" 2>/dev/null || true)"
fi
EDITOR_PROJECT="${ACCELERATOR_VISUALISER_EDITOR_PROJECT:-}"
if [ -z "$EDITOR_PROJECT" ]; then
  EDITOR_PROJECT="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "visualiser.editor_project" "" 2>/dev/null || true)"
fi

# Trim surrounding whitespace (bash 3.2-safe; uses ${%%}/${##}, NOT ${//} per the
# known macOS replacement-slash gotcha) so a whitespace-only value collapses to
# empty and is omitted by the splice below — upholding "absent → None → disabled"
# for whitespace-only values, not just genuinely-empty ones, so config.json never
# carries a spurious `"editor": "   "` key. (The server still trims defensively.)
trim_ws() { local v="$1"; v="${v#"${v%%[![:space:]]*}"}"; v="${v%"${v##*[![:space:]]}"}"; printf '%s' "$v"; }
EDITOR="$(trim_ws "$EDITOR")"
EDITOR_PROJECT="$(trim_ws "$EDITOR_PROJECT")"
```

Add to the final `jq -n` invocation:

```bash
  --arg editor "$EDITOR" \
  --arg editor_project "$EDITOR_PROJECT" \
```

And extend the trailing splice (currently `+ (if $idle_timeout == "" …)`):

```bash
  + (if $editor == "" then {} else {editor: $editor} end)
  + (if $editor_project == "" then {} else {editor_project: $editor_project} end)
```

#### 2. Typed config

**File**: `skills/visualisation/visualise/server/src/config.rs`
**Changes**: Add two optional fields to `Config` (after `idle_timeout`, `:43`):

```rust
    /// Editor deep-link selection: a preset key (e.g. `vscode`, `cursor`,
    /// `idea`, `web-storm`) or a custom URL template containing `://` or an
    /// `{abs}`/`{rel}` placeholder. Absent → `Open in editor` renders disabled.
    /// Passed through verbatim; the frontend resolves presets/templates.
    #[serde(default)]
    pub editor: Option<String>,
    /// JetBrains project name for the `{project}` placeholder. Absent →
    /// server defaults to the basename of `project_root`. Ignored by
    /// non-JetBrains presets.
    #[serde(default)]
    pub editor_project: Option<String>,
```

Update **every** literal `Config { … }` to set `editor: None, editor_project: None`
(grep `idle_timeout: None` to find all sites — `kanban_config.rs:57`, `docs.rs`,
`indexer.rs`, `work_item_config.rs`, `events.rs`, `activity.rs`, `server.rs`, plus
`config.rs` test JSONs which need no change since the fields are `#[serde(default)]`).

#### 3. Dedicated endpoint

**File** *(new)*: `skills/visualisation/visualise/server/src/api/editor_config.rs`
**Changes**: Mirror `kanban_config.rs`. Resolve the project default from
`state.cfg`:

```rust
use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::server::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorConfigBody {
    /// `None` → no editor configured → frontend renders the disabled state.
    editor: Option<String>,
    /// Resolved JetBrains project name: configured `editor_project`, else the
    /// basename of `project_root`. Always present so the frontend never derives it.
    editor_project: String,
}

pub(crate) async fn get_editor_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let editor_project = state
        .cfg
        .editor_project
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            state
                .cfg
                .project_root
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    Json(EditorConfigBody {
        editor: state.cfg.editor.clone().filter(|s| !s.trim().is_empty()),
        editor_project,
    })
}
```

> Note: `EditorConfigBody` uses `#[serde(rename_all = "camelCase")]` to emit
> `editorProject`, which the cited template `kanban_config.rs` does not (its field
> names are already camelCase). This is an intentional, cleaner divergence — the
> attribute, not a hand-camelCased field name, owns the wire casing — kept consistent
> with the frontend `EditorConfig` interface (`editorProject`).

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`
**Changes**: `mod editor_config;` (`:5` block) and the route (`:47` block):

```rust
        .route("/api/editor/config", get(editor_config::get_editor_config))
```

#### 4. Frontend type + hook + query key

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: Add the response interface:

```ts
export interface EditorConfig {
  /** Configured editor preset key or custom template; null → render disabled. */
  editor: string | null
  /** Resolved JetBrains project name (server-defaulted to project_root basename). */
  editorProject: string
}
```

**File** *(new)*: `skills/visualisation/visualise/frontend/src/api/use-editor-config.ts`
**Changes**: Model `use-kanban-config.ts`:

```ts
import { useQuery } from '@tanstack/react-query'
import { queryKeys } from './query-keys'
import type { EditorConfig } from './types'

async function fetchEditorConfig(): Promise<EditorConfig> {
  const resp = await fetch('/api/editor/config')
  if (!resp.ok) throw new Error(`/api/editor/config returned ${resp.status}`)
  return resp.json() as Promise<EditorConfig>
}

export function useEditorConfig() {
  return useQuery({
    queryKey: queryKeys.editor(),
    queryFn: fetchEditorConfig,
    staleTime: Infinity,
  })
}
```

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`
**Changes**: Add `editor: () => ['editor'] as const,` to `queryKeys` (near `:58`).
This follows the existing bare-resource-root precedent (`kanban: () => ['kanban']`)
rather than an `'editor-config'` suffix, keeping the query-key namespace consistent.
Do **not** add `'editor'` to `SESSION_STABLE_QUERY_ROOTS` (`:70-73`): the directly
analogous `kanban` config query — also `staleTime: Infinity` — is deliberately absent
from that set, so editor config matches its precedent. (If session-stable
reconnect-invalidation immunity is ever wanted, add `kanban` and `editor` together with
a rationale rather than diverging for one.)

#### 5. User-facing config documentation

**File**: the visualiser skill's config reference (the same place the other
`visualiser.*` keys — `binary`, `idle_timeout` — are documented; e.g. the
`/accelerator:visualise` SKILL.md "Overrides" section and/or `.accelerator/config.md`
comments).
**Changes**: Document `visualiser.editor` and `visualiser.editor_project` alongside the
existing keys: the full preset-key list (VS Code family + JetBrains), the
custom-template syntax with the `{abs}`/`{rel}` placeholders and the
**must-contain-a-placeholder** rule, the dangerous-scheme guard, the
`ACCELERATOR_VISUALISER_EDITOR` / `..._EDITOR_PROJECT` env overrides and precedence,
and a worked custom-template example (`zed://file{abs}`). Without this the escape hatch
and the preset vocabulary are discoverable only by reading source.

### Success Criteria:

#### Automated Verification:

- [x] Rust builds and tests pass: `mise run test:unit` (or `cargo test` in
  `skills/visualisation/visualise/server`)
- [x] New endpoint test: configured `editor`/`editor_project` round-trips; absent
  `editor` → `null`; absent `editor_project` → `project_root` basename;
  whitespace-only `editor`/`editor_project` treated as absent (`.trim().is_empty()`)
  (`editor_config.rs` test module, modelled on `kanban_config.rs:30-128`)
- [x] `config.rs` test loop closed (compile-checks alone don't exercise the runtime
  default): extend `bare_config_json` to **assert** `cfg.editor == None` and
  `cfg.editor_project == None`, AND add a positive parse test for a config.json that
  carries both keys → `Some(...)`
- [x] Frontend typecheck passes: `mise run test:unit:frontend` (typecheck) /
  `tsc --noEmit`
- [x] Shell config test asserts the **discrete** precedence cases for both `editor`
  and `editor_project`, mirroring the `idle_timeout` suite: (a) absent → JSON key
  genuinely **absent** (not empty-string); (b) config-key value emitted; (c) env var
  overrides config key; (d) empty env falls through to config; (e) whitespace-only →
  key absent; (f) a custom-template value containing `://` **and** a space round-trips
  intact under the CI bash 3.2 (guards the macOS quoting gotcha)

#### Manual Verification:

- [ ] With no `visualiser.editor` set, `curl localhost:<port>/api/editor/config`
  returns `{"editor":null,"editorProject":"<project-dir-basename>"}`
- [ ] With `visualiser.editor: cursor` in `config.md`, the endpoint returns
  `"editor":"cursor"`
- [ ] `ACCELERATOR_VISUALISER_EDITOR=vscode` overrides a `config.md` value
- [ ] `visualiser.editor_project: myrepo` is reflected as `"editorProject":"myrepo"`

---

## Phase 2: `TopbarIconButton` polymorphism + `Copy path` action

### Overview

Make `TopbarIconButton` polymorphic so it can render a `<button>`, an `<a href>`, or
a disabled `<button>`, with `ariaPressed` optional (emit `aria-pressed` only when
provided). Add a clipboard helper and wire the `Copy path` button into the
`Page.actions` slot on `LibraryDocView`. Frontend-only; no dependency on Phase 1.

### Changes Required:

#### 1. Polymorphic `TopbarIconButton`

**File**: `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.tsx`
**Changes**: Extend props to a discriminated shape. Keep the existing button +
`ariaPressed` path backward compatible (ThemeToggle/FontModeToggle keep passing
`ariaPressed`), and add anchor + disabled support. Reuse the same `.toggle` class.

```tsx
import type { ReactNode } from 'react'
import styles from './TopbarIconButton.module.css'

interface BaseProps {
  ariaLabel: string
  dataIcon: string
  children: ReactNode
  /** Native tooltip — also the disabled `Open in editor` hint. */
  title?: string
}

interface ButtonProps extends BaseProps {
  as?: 'button'
  /** Omit for a disabled / no-op control (a disabled button never fires). */
  onClick?: () => void
  /** Toggle-pressed state. Lives on the button variant only — `aria-pressed` is
   *  invalid on a link, so the anchor variant cannot accept it. */
  ariaPressed?: boolean
  /** Renders an inert but still-focusable control (see note below). */
  disabled?: boolean
  /** id of visible / SR-only text describing why the control is disabled. */
  ariaDescribedBy?: string
}

interface AnchorProps extends BaseProps {
  as: 'a'
  href: string
  /** Defaults to `noopener noreferrer`; editor links may resolve to navigable
   *  http(s) targets via a custom template. */
  rel?: string
}

export type TopbarIconButtonProps = ButtonProps | AnchorProps

export function TopbarIconButton(props: TopbarIconButtonProps) {
  const common = {
    className: styles.toggle,
    'data-icon': props.dataIcon,
    'aria-label': props.ariaLabel,
    ...(props.title !== undefined ? { title: props.title } : {}),
  }
  if (props.as === 'a') {
    return (
      <a {...common} href={props.href} rel={props.rel ?? 'noopener noreferrer'}>
        {props.children}
      </a>
    )
  }
  const isDisabled = props.disabled === true
  return (
    <button
      {...common}
      type="button"
      // `aria-disabled` (NOT the native `disabled` attribute) keeps the control
      // in the tab order so its title / description stay reachable by keyboard
      // and screen readers — the disabled `Open in editor` hint is the user's
      // only path to enabling the feature.
      aria-disabled={isDisabled || undefined}
      {...(props.ariaPressed !== undefined ? { 'aria-pressed': props.ariaPressed } : {})}
      {...(props.ariaDescribedBy !== undefined ? { 'aria-describedby': props.ariaDescribedBy } : {})}
      onClick={isDisabled ? undefined : props.onClick}
    >
      {props.children}
    </button>
  )
}
```

> **Disabled styling (definite Phase 2 deliverable, not "if needed").** `.toggle`
> currently hardcodes `cursor: pointer` and full-strength hover/active states. Add a
> `.toggle[aria-disabled="true"]` rule — `cursor: default`, a muted `--ac-*` colour
> (keep all colours on tokens to satisfy the token AC), and suppressed `:hover`/`:active`
> transitions — and ensure the `forced-colors` path still reads as inactive. Also verify
> the class renders identically on `<a>` (it should — `display: inline-flex`, token
> colours via `currentColor`).

#### 2. Clipboard helper

**File** *(new)*: `skills/visualisation/visualise/frontend/src/api/clipboard.ts`
**Changes**: Net-new pure helper with the documented fallback.

```ts
/** Copy `text` to the clipboard. Prefers the async Clipboard API (available in
 *  the localhost secure context); falls back to a hidden-textarea
 *  `document.execCommand('copy')` for non-secure-context edge cases. Resolves
 *  true on success, false on failure. */
export async function copyText(text: string): Promise<boolean> {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // fall through to execCommand
    }
  }
  return execCommandCopy(text)
}

function execCommandCopy(text: string): boolean {
  try {
    const ta = document.createElement('textarea')
    ta.value = text
    ta.setAttribute('readonly', '')
    ta.style.position = 'absolute'
    ta.style.left = '-9999px'
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand('copy')
    document.body.removeChild(ta)
    return ok
  } catch {
    return false
  }
}
```

#### 3. `Copy path` button + wiring

**File** *(new)*: `frontend/src/components/DetailHeaderActions/CopyPathButton.tsx`.
Both header actions are extracted as standalone components under a single
`DetailHeaderActions/` directory (decided here, not left open) so `LibraryDocView`
stays a thin consumer and the affordance is reusable on future detail routes; the
clipboard / editor-link wiring never lands inline in the route component.
**Changes**: A `TopbarIconButton` (button variant, no `ariaPressed`) with a
clipboard glyph (inline SVG, ThemeToggle pattern at `:16-31`), `ariaLabel="Copy
path"`. On click: `copyText(relPath)`, then branch the toast on the returned
success boolean.

```tsx
const { showToast } = useToast()
async function onCopyPath() {
  const ok = await copyText(relPath) // raw entry.relPath
  if (ok) {
    showToast({ heading: 'Copied path to clipboard', message: `\`${relPath}\``, kind: 'ok' })
  } else {
    showToast({ heading: 'Couldn’t copy path to clipboard', message: '', kind: 'error' })
  }
}
```

> `copyText` returns `true`/`false`; the handler MUST consume it so a failed copy
> (both the Clipboard API and the `execCommand` fallback fell through) surfaces the
> persistent `error` toast rather than a misleading success confirmation. The Toaster
> renders paired-backtick runs as `<code>`, so the success message shows the path in
> monospace. A copy-path domain hook (`api/use-copy-path-toast.ts`) modelled on
> `use-external-edit-toast.ts` is optional; a direct `showToast` call is acceptable.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
**Changes**: Pass `actions` to `<Page>` only when `hasResolvedDocument` (depends on a
resolved `entry`). Initially just the Copy path button:

```tsx
return (
  <Page
    eyebrow={hasResolvedDocument ? <EyebrowLabel type={type} /> : undefined}
    title={title}
    subtitle={subtitle}
    actions={
      hasResolvedDocument && entry ? (
        // From `components/DetailHeaderActions/`.
        <CopyPathButton relPath={entry.relPath} />
      ) : undefined
    }
  >
    {body}
  </Page>
)
```

### Success Criteria:

#### Automated Verification:

- [x] Frontend tests pass: `mise run test:unit:frontend`
- [x] `TopbarIconButton` test: button variant emits `aria-pressed` only when
  provided and is absent on the anchor variant; anchor variant renders `<a href>`
  with `rel="noopener noreferrer"`; disabled variant renders `<button
  aria-disabled="true">` that stays focusable (NOT the native `disabled` attribute),
  keeps its `title`, fires no `onClick`, and wires `aria-describedby` when provided
  (extend `TopbarIconButton.test.tsx`)
- [x] `.toggle[aria-disabled="true"]` style asserts an inactive affordance
  (`cursor: default`, muted `--ac-*` colour, no hover/active change)
- [x] `clipboard.ts` test: uses `navigator.clipboard.writeText` when present; falls
  back to `execCommand('copy')` when absent; returns false on failure
- [x] Copy-path success test: clicking copies the raw `relPath` (no scheme/host/encoding)
  and triggers a `kind: 'ok'` toast
- [x] Copy-path failure test: when `copyText` resolves `false`, the handler shows a
  `kind: 'error'` toast and NOT the success toast
- [x] Typecheck passes; existing `ThemeToggle`/`FontModeToggle` tests still pass
  (backward-compatible props)

#### Manual Verification:

- [ ] `Copy path` appears bottom-right of the detail header, aligned with the title
- [ ] Clicking copies `meta/work/0080-detail-page-header-actions.md` (raw) and a
  Toaster confirmation appears
- [ ] Button colours/glyph match the `TopbarIconButton` precedent in both themes

---

## Phase 3: `editor-link` helper + `Open in editor` action

### Overview

Add the pure `editor-link` helper (preset table, resolution rule, percent-encoding)
and wire the `Open in editor` control into `Page.actions`: an `<a href>` when
configured, a disabled `<button>` with a `visualiser.editor`-naming tooltip when not.

### Changes Required:

#### 1. `editor-link` helper

**File** *(new)*: `skills/visualisation/visualise/frontend/src/api/editor-link.ts`
**Changes**: Net-new. Preset table + resolution rule + segment-wise percent-encoder.

```ts
interface VscodePreset { family: 'vscode'; scheme: string }
interface JetBrainsPreset { family: 'jetbrains'; tag: string }
type Preset = VscodePreset | JetBrainsPreset

// VS Code family: {scheme}://file{abs} (single slash; {abs} carries the leading /);
// scheme == preset key.
// JetBrains: jetbrains://{tag}/navigate/reference?project={project}&path={rel};
// tag == preset key (per the story's documented contract).
export const EDITOR_PRESETS: Record<string, Preset> = {
  vscode: { family: 'vscode', scheme: 'vscode' },
  'vscode-insiders': { family: 'vscode', scheme: 'vscode-insiders' },
  vscodium: { family: 'vscode', scheme: 'vscodium' },
  cursor: { family: 'vscode', scheme: 'cursor' },
  windsurf: { family: 'vscode', scheme: 'windsurf' },
  idea: { family: 'jetbrains', tag: 'idea' },
  'web-storm': { family: 'jetbrains', tag: 'web-storm' },
  pycharm: { family: 'jetbrains', tag: 'pycharm' },
  'php-storm': { family: 'jetbrains', tag: 'php-storm' },
  goland: { family: 'jetbrains', tag: 'goland' },
  rubymine: { family: 'jetbrains', tag: 'rubymine' },
  clion: { family: 'jetbrains', tag: 'clion' },
  rd: { family: 'jetbrains', tag: 'rd' },
  rustrover: { family: 'jetbrains', tag: 'rustrover' },
}

/** Percent-encode a path while preserving `/` separators:
 *  `/a b/c.md` → `/a%20b/c.md`. */
export function encodePath(p: string): string {
  return p.split('/').map(encodeURIComponent).join('/')
}

export interface EditorLinkInputs {
  editor: string            // preset key or custom template (non-empty)
  editorProject: string     // resolved JetBrains project name
  absPath: string           // entry.path
  relPath: string           // entry.relPath
}

/** Dangerous schemes rejected even via a custom template. This is a DENY-LIST (not an
 *  allow-list) so arbitrary *editor* protocols (zed://, subl://, txmt://, …) keep
 *  working through the escape hatch — a deliberate divergence from the MarkdownRenderer,
 *  which delegates to react-markdown's allow-list `urlTransform`. A deny-list is
 *  best-effort: extend it if a new dangerous scheme emerges. The bypass-resistant
 *  normalisation in `schemeOf` (it rejects embedded TAB/CR/LF and strips leading
 *  whitespace before matching the scheme) is what makes it sound. */
const BLOCKED_SCHEMES = new Set(['javascript', 'data', 'vbscript', 'blob', 'file'])

/** Strict, lowercased scheme of a URL, or null if there is no syntactically valid one.
 *  Leading ASCII whitespace is stripped first (browsers strip it before parsing the
 *  scheme), and the scheme is matched per RFC 3986 (`ALPHA *( ALPHA / DIGIT / + - . )`)
 *  with a `:` lookahead — so a token containing whitespace or control chars does NOT
 *  match, closing the classic ` javascript:` / `java\tscript:` deny-list bypass. */
function schemeOf(url: string): string | null {
  // Reject embedded TAB/CR/LF: browsers strip these from a URL before parsing its
  // scheme, so `java\t script:` would execute as `javascript:` while slipping past
  // a naive check. Treat their presence as unresolvable.
  if (/[\t\n\r]/.test(url)) return null
  const m = /^[a-z][a-z0-9+.-]*(?=:)/i.exec(url.trimStart())
  return m ? m[0].toLowerCase() : null
}

/** Resolve the deep-link href, or null if `editor` is empty/unresolvable/unsafe. */
export function buildEditorHref(inputs: EditorLinkInputs): string | null {
  const { editor, editorProject, absPath, relPath } = inputs
  const abs = encodePath(absPath)
  const rel = encodePath(relPath)

  const preset = EDITOR_PRESETS[editor]
  if (preset) {
    if (preset.family === 'vscode') {
      // `abs` already begins with `/`, so concatenate WITHOUT an extra slash —
      // `file` + `/Users/…` == `file/Users/…`, not `file//Users/…`.
      return `${preset.scheme}://file${abs}`
    }
    return `jetbrains://${preset.tag}/navigate/reference` +
      `?project=${encodeURIComponent(editorProject)}&path=${rel}`
  }

  // Custom template: MUST carry an {abs}/{rel} placeholder (a value that cannot
  // reference the file cannot open it). A `://`-but-placeholder-free value (e.g.
  // `myeditor://open`) resolves to null → disabled, not a path-less link.
  const hasPlaceholder = editor.includes('{abs}') || editor.includes('{rel}')
  if (!hasPlaceholder) {
    return null
  }
  const href = editor.replaceAll('{abs}', abs).replaceAll('{rel}', rel)

  // Scheme guard: require a syntactically valid scheme that is NOT on the deny-list.
  // A null scheme (relative / protocol-relative href, e.g. a bare `{rel}` template)
  // is unresolvable → disabled, never emitted as an in-app navigation link.
  const scheme = schemeOf(href)
  if (scheme === null || BLOCKED_SCHEMES.has(scheme)) {
    return null
  }

  return href
}
```

> Resolution rule (canonical — kept in sync with work item 0080): **(1)** preset key →
> expand its template; **(2)** else if the value contains an `{abs}`/`{rel}` placeholder
> → custom template, substituted, control-char-rejected, then scheme-guarded (must carry
> a syntactically valid, non-blocked scheme); **(3)** else → null (disabled). This
> tightens the original "`://` **or** placeholder" gate to require a placeholder **and** a
> valid scheme, and the deny-list normalises the scheme (strip leading whitespace, reject
> embedded control chars) before checking — all decided in review and reflected back into
> 0080. `{scheme}`/`{tag}`/`{project}` are internal to the preset templates;
> `{abs}`/`{rel}` are the user-facing custom-template placeholders.

#### 2. `Open in editor` control + wiring

**File** *(new)*: `frontend/src/components/DetailHeaderActions/OpenInEditorButton.tsx`
(co-located with `CopyPathButton`).
**Changes**: Consume `useEditorConfig()`; compute the href via `buildEditorHref`.
The disabled state has **two distinct sub-states** with different tooltips:
- Configured (`editor` non-null **and** href resolves) → `TopbarIconButton` anchor
  variant (`as="a"`, `href`, `ariaLabel="Open in editor"`, edit glyph). The anchor
  defaults `rel="noopener noreferrer"`.
- **Unconfigured** (`editor` null/absent) → disabled button, tooltip naming
  `visualiser.editor` / `ACCELERATOR_VISUALISER_EDITOR` as the config to set.
- **Configured-but-unrecognised** (`editor` set but `buildEditorHref` returns null —
  a typo'd preset key, or a custom template missing a placeholder / using a blocked
  scheme) → disabled button, tooltip saying the value was **not recognised**, so the
  user does not think their config failed to load. Both disabled buttons omit
  `onClick` entirely (the component renders them `aria-disabled` and inert).

```tsx
const { data } = useEditorConfig()
const href = data?.editor
  ? buildEditorHref({ editor: data.editor, editorProject: data.editorProject, absPath, relPath })
  : null

if (href) {
  return (
    <TopbarIconButton
      as="a"
      href={href}
      ariaLabel="Open in editor"
      dataIcon="edit"
      title="Open in editor"
    >{editGlyph}</TopbarIconButton>
  )
}

// Distinguish "not configured" from "configured but unrecognised" so the tooltip is
// never misleading. Bind the narrowed value once (no non-null assertion); a still-
// loading query (`data === undefined`) falls into the unconfigured wording — see the
// transient-flip note below.
const descId = useId()
const configuredEditor = data?.editor ?? null
// Truncate the echoed value so a long custom template can't push the guidance out of
// a native tooltip; the full hint still reaches AT via the description element.
const shown =
  configuredEditor && configuredEditor.length > 40
    ? `${configuredEditor.slice(0, 40)}…`
    : configuredEditor
const title = configuredEditor
  ? `visualiser.editor value “${shown}” was not recognised — set a preset key or a ` +
    `custom template containing {abs}/{rel}; see the visualiser.editor docs for the full list`
  : 'Set visualiser.editor (or ACCELERATOR_VISUALISER_EDITOR) to enable opening files in your editor'

return (
  <>
    <TopbarIconButton
      ariaLabel="Open in editor"
      dataIcon="edit"
      disabled
      title={title}
      ariaDescribedBy={descId}
    >{editGlyph}</TopbarIconButton>
    {/* Visually-hidden description: a native `title` is mouse-hover-only, so keyboard
        and screen-reader users would otherwise never get the enablement hint. */}
    <span id={descId} className={srOnly}>{title}</span>
  </>
)
```

> Uses React's `useId` and the app's `srOnly` visually-hidden utility class. If no
> such utility exists yet, adding one (the standard clip-rect/`sr-only` pattern) is a
> Phase 3 deliverable — the `aria-describedby` target must be in the accessibility tree
> but visually hidden. The same description is wired for both disabled sub-states.
>
> Transient flip: on first render `data` is `undefined` (the session-stable query is
> resolving), so the control briefly shows the unconfigured tooltip before settling.
> This is a one-time, sub-second flip given `staleTime: Infinity`; acceptable as-is.
> If it ever matters, gate the disabled wording on `!isPending`.
>
> Touch caveat: a native `title` is not shown on touch even when focusable, and
> `aria-describedby` is AT-only. The disabled hint is therefore keyboard- and
> screen-reader-reachable but remains a known minor gap for sighted touch users; a
> visible helper affordance is a possible follow-up, out of scope here.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
**Changes**: Add `OpenInEditorButton` alongside `CopyPathButton` in the `actions`
prop, passing `entry.path` (absolute) and `entry.relPath`:

```tsx
actions={
  hasResolvedDocument && entry ? (
    <>
      <OpenInEditorButton absPath={entry.path} relPath={entry.relPath} />
      <CopyPathButton relPath={entry.relPath} />
    </>
  ) : undefined
}
```

### Success Criteria:

#### Automated Verification:

- [x] Frontend tests pass: `mise run test:unit:frontend`
- [x] `editor-link.ts` table-driven test asserts **every** preset maps to its
  documented scheme/tag (VS Code family `{scheme}://file{abs}` — a **single** slash
  before the absolute path; JetBrains
  `jetbrains://{tag}/navigate/reference?project=…&path={rel}`)
- [x] Single-slash regression test: a VS Code preset with `/Users/x/a b.md` →
  `vscode://file/Users/x/a%20b.md` (asserts **no** `file//`)
- [x] Custom-template test: `zed://file{abs}` + `/a b/c.md` → `zed://file/a%20b/c.md`
- [x] Percent-encoding test: spaces encoded, `/` preserved; JetBrains example
  `project=myrepo&path=sub%20dir/a.md`
- [x] Preset-vs-custom test: bare `cursor` → preset template (no `://` in value);
  non-matching bare value → null
- [x] Placeholder-required test: a `://` template with **no** `{abs}`/`{rel}`
  (e.g. `myeditor://open`) → null (disabled), not a path-less link
- [x] Scheme-required test: a placeholder template with **no** scheme (e.g. `{rel}`,
  `./{rel}`) → null (disabled), never emitted as a relative in-app navigation href
- [x] Dangerous-scheme test (table-driven, deny-list): `javascript:`, `data:`,
  `vbscript:`, `blob:`, `file:` templates → null; a benign editor scheme
  (`zed://file{abs}`) still resolves
- [x] Scheme-guard bypass test (the load-bearing security cases): mixed-case
  (`JavaScript:…{rel}`), leading whitespace (`"  javascript:…{rel}"`), and an embedded
  TAB/CR/LF in the scheme (`"java\tscript:…{rel}"`) all → null — pins `schemeOf`'s
  `trimStart` + control-char reject + lowercasing, not just the deny-list set
- [x] Multi-placeholder test: a template with two `{abs}` occurrences substitutes both
  (pins `replaceAll`, not replace-first)
- [x] `OpenInEditorButton` renders `<a rel="noopener noreferrer">` with the computed
  `href` when configured; renders a focusable `aria-disabled` button (assert it has
  **no** native `disabled` attribute and is reachable via `.focus()`/tab — not merely
  that `aria-disabled` is set) wired to an `aria-describedby` element carrying the hint
- [x] Tooltip-contract test (pins the distinguishing content, not just presence): the
  unconfigured `title`/description contains the literal `visualiser.editor`; the
  configured-but-unrecognised one contains a "not recognised" phrase **and** the
  offending value (truncated), so the two sub-states cannot silently swap or drop the value
- [x] Typecheck passes

#### Manual Verification:

- [ ] With `visualiser.editor: vscode`, clicking `Open in editor` opens the file in
  VS Code (first-time "Open in…?" prompt is expected/acceptable)
- [ ] With `visualiser.editor: web-storm` + `visualiser.editor_project: myrepo`, the
  `href` is the JetBrains form with the project name
- [ ] With no editor configured, `Open in editor` is disabled and its tooltip names
  `visualiser.editor`
- [ ] Both buttons sit together in the header and match the topbar icon-button style

---

## Testing Strategy

### Unit Tests:

- **`config.rs`**: parse with/without `editor`/`editor_project`; absence → `None`.
- **`editor_config.rs`**: endpoint returns configured values; `editor` absent →
  `null`; `editor_project` absent → `project_root` basename; whitespace-only treated
  as absent.
- **`clipboard.ts`**: Clipboard API path, `execCommand` fallback, failure → false.
- **`editor-link.ts`**: full preset table; VS Code single-slash regression
  (`vscode://file/Users/…`, no `file//`); custom template; percent-encoding edge cases
  (spaces, `/` preservation, leading slash); preset-vs-custom branch; null for an
  unresolvable bare value; null for a `://`-but-placeholder-free template; null for a
  placeholder-but-scheme-less template; dangerous-scheme deny-list
  (`javascript:`/`data:`/`vbscript:`/`blob:`/`file:` → null) **and** its bypass vectors
  (mixed-case, leading-whitespace, embedded TAB/CR/LF schemes → null); multi-`{abs}`
  substitution.
- **`TopbarIconButton.test.tsx`**: button/anchor/disabled variants; `aria-pressed`
  present only on the button variant when passed and absent on the anchor; disabled =
  focusable `aria-disabled` (not native `disabled`) with `title`, no `onClick` fired,
  `aria-describedby` wired; anchor emits `rel="noopener noreferrer"`; backward compat
  for ThemeToggle/FontModeToggle.

### Integration Tests:

- **Shell** (`write-visualiser-config.sh`): the discrete precedence cases for both
  keys (see Phase 1 Success Criteria); omit-when-empty and whitespace-only produce no
  JSON key; a `://`-and-space custom template round-trips intact.
- **Component**: Copy path copies raw relPath + toasts on success AND surfaces an
  `error` toast on failure; Open in editor renders the correct element per
  configured / unconfigured / configured-but-unrecognised state.
- **Route-level (committed, not "consider")**: a `LibraryDocView` test asserting both
  actions appear in `data-slot="actions"` for a resolved doc and are **absent** in the
  unresolved/loading state — exercising the actual `actions` prop guard and the
  `entry.path`/`entry.relPath` plumbing rather than the isolated buttons. An e2e check
  via `mise run test:e2e:visualiser` on a doc route is a further optional layer.

### Manual Testing Steps:

1. Launch the visualiser; open a `LibraryDocView` doc; confirm both buttons render.
2. Click `Copy path`; paste elsewhere → raw relative path; confirm toast.
3. Disable `navigator.clipboard` (or non-secure context) and re-test the fallback.
4. Set `visualiser.editor` to a VS Code preset, a JetBrains preset (+ project), and a
   custom template; verify each `href` and that the link opens the editor.
5. Unset the editor; confirm the disabled button + tooltip.

## Performance Considerations

Negligible. `/api/editor/config` is fetched once (`staleTime: Infinity`, session-
stable). The editor-link helper is pure string work computed per render of a single
button. No new indexing or server hot-path cost.

## Migration Notes

Backward compatible. Older `config.json` files without `editor`/`editor_project`
parse cleanly (`#[serde(default)]`), and the omit-when-empty jq splice keeps them
absent → `None` → disabled button. No data migration. `deny_unknown_fields` requires
the shell writer and Rust struct to land together (single phase).

## References

- Original work item: `meta/work/0080-detail-page-header-actions.md`
- Research: `meta/research/codebase/2026-06-09-0080-detail-page-header-actions.md`
- Config template (idle_timeout): `meta/plans/2026-06-06-0100-configurable-visualiser-auto-shutdown.md`
- Page.actions origin: `meta/work/0041-library-page-wrapper-and-overview-hub.md`
- TopbarIconButton precedent: `meta/work/0035-topbar-component.md`
- Toaster: `meta/work/0039-toaster-and-external-edit-notifications.md`
- Config endpoint pattern: `skills/visualisation/visualise/server/src/api/kanban_config.rs:8-28`
- Frontend config-fetch pattern: `skills/visualisation/visualise/frontend/src/api/use-kanban-config.ts:5-21`
- Governance ADRs: ADR-0016/0017 (config model/extension points), ADR-0024
  (visualiser config to frontend), ADR-0026/0035/0036/0039 (tokens/typography/radius)
