---
id: "0080"
title: "Detail-Page Header Actions (Open in Editor, Copy Path)"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: ready
priority: medium
type: work-item
blocked_by: ["work-item:0041", "work-item:0039"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
tags: [design, frontend, detail-page, config]
last_updated: "2026-06-09T18:46:39+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0080: Detail-Page Header Actions (Open in Editor, Copy Path)

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a visualiser user reading a document, I want `Open in editor` and
`Copy path` actions in the detail-page header, so that I can jump
straight to the document's source file in my editor or copy a path I can
reference elsewhere (a commit message, a chat, a terminal). This story
wires both actions into the existing `Page.actions` slot on the
`LibraryDocView` route: `Copy path` writes the document's project-root-
relative path to the clipboard, and `Open in editor` opens the document's
source file via a configurable editor deep-link.

## Context

The prototype's `DocPage` ships two right-aligned topbar buttons —
`Open in editor` (`Icon name="edit"`) and `Copy link` (`Icon name="link"`)
— on every detail page. They are decorative in the prototype but
represent a deliberate affordance slot. The current app's `Page` chrome
already supports this via the `actions?` prop (`PageProps`,
`Page.tsx:4-11`) but does not populate it for `LibraryDocView`.

This story deliberately diverges from the prototype's second button: a
`Copy link` action would copy the current route's absolute URL, but that
URL is bound to this ephemeral, randomly-ported localhost visualiser
instance and is meaningless to anyone else (or after the server restarts).
The useful, portable thing to copy is the document's **project-root-
relative path**, so the action is `Copy path` instead.

Editor selection reuses the **existing** visualiser configuration
mechanism — a `visualiser:` config block in `config.md`, a personal
override in `config.local.md`, and `ACCELERATOR_VISUALISER_*`
environment-variable overrides — already established by `visualiser.binary`
/ `ACCELERATOR_VISUALISER_BIN`. Work item 0100 (Configurable Visualiser
Auto-Shutdown) is a sibling that extends the same mechanism; this story
follows that pattern but consumes no code from 0100 and is not blocked by
it. The dependency is on the pre-existing config plumbing, which already
exists.

## Requirements

- Render two right-aligned action buttons in the `Page.actions` slot on
  the `LibraryDocView` detail route: `Open in editor` and `Copy path`.
  Other detail routes are out of scope for this story.
- `Copy path` writes the document's **project-root-relative path** (raw,
  not percent-encoded; forward-slash separators, e.g.
  `meta/work/0080-detail-page-header-actions.md`) to the system clipboard
  via `navigator.clipboard.writeText`, with a `document.execCommand('copy')`
  fallback, and shows a Toaster confirmation (consumes 0039). It does **not**
  copy the localhost route URL, which is instance-specific and not portable.
- `Open in editor` opens the document's **source file at the top of the
  file** (no line or column targeting) via an editor deep-link, rendered
  as a real anchor so the navigation is driven by a genuine user gesture.
- The editor is configured via a single `visualiser.editor` field, resolved
  by an ordered rule: **(1)** if the value matches a known preset key, expand
  that preset's template; **(2)** otherwise, if the value contains an
  `{abs}`/`{rel}` placeholder, treat it as a custom URL template (substitute,
  then reject if it resolves to a dangerous scheme — see below); **(3)** else,
  the value is unresolvable and `Open in editor` renders disabled. A custom
  template **must contain at least one `{abs}`/`{rel}` placeholder** — a value
  that cannot reference the file cannot open it, so a `://`-only value (no
  placeholder) is treated as unresolvable rather than emitting a path-less link.
  As a safety guard, a resolved href whose scheme is `javascript`, `data`,
  `vbscript`, `blob`, or `file` is rejected (rendered disabled), mirroring the
  app's existing href-sanitisation posture; any other (editor) scheme is allowed
  so the escape hatch still covers Zed, Sublime, etc. This single rule is the
  canonical definition; other sections refer to it rather than restating the
  condition. Two editor families have presets in this pass:
  - **VS Code family** — preset keys `vscode`, `vscode-insiders`, `vscodium`,
    `cursor`, `windsurf`, each expanding to `{scheme}://file{abs}` (`{abs}`
    carries the leading `/`, so this is a single slash before the path).
  - **JetBrains** — preset keys `idea` (IntelliJ IDEA), `web-storm` (WebStorm),
    `pycharm` (PyCharm), `php-storm` (PhpStorm), `goland` (GoLand),
    `rubymine` (RubyMine), `clion` (CLion), `rd` (Rider),
    `rustrover` (RustRover), each expanding to
    `jetbrains://{tag}/navigate/reference?project={project}&path={rel}`.
  - A **custom URL template** lets editors without a preset (e.g. Zed,
    Sublime) be wired up by the user, without being officially supported
    presets in this story.
- Template placeholders: `{scheme}` = the VS Code-family preset's URL scheme;
  `{tag}` = the JetBrains tool tag; `{project}` = the resolved JetBrains
  project name; `{abs}` = the percent-encoded absolute source path; `{rel}` =
  the percent-encoded workspace-relative source path.
- A companion `visualiser.editor_project` field supplies the JetBrains
  project name; when omitted it defaults to the workspace directory
  basename. It is ignored by non-JetBrains presets.
- The `LibraryDocView` route must make the document's source-file absolute
  path and project-root-relative path available to the action component.
  The relative path serves both `Copy path` (raw form) and the editor
  templates' `{rel}` placeholder (percent-encoded form); the absolute path
  serves the `{abs}` placeholder. Exposing these paths is in scope for this
  story if they are not already surfaced to the route.
- The active editor is selected through the visualiser configuration
  mechanism with precedence **env var → personal config → project config**
  (`ACCELERATOR_VISUALISER_EDITOR`, `ACCELERATOR_VISUALISER_EDITOR_PROJECT`).
  There is **no built-in default**: when no editor is configured, the
  `Open in editor` button renders **disabled**, with a hover tooltip that
  names the `visualiser.editor` config field (or the
  `ACCELERATOR_VISUALISER_EDITOR` env var) the user must set to enable it.
- Render both actions as **labelled pill buttons** (icon **and** visible text
  label), matching the prototype's `DocPage` `ac-topbar__btn` affordance
  (`Icon name="edit"` + "Open in editor", `Icon name="link"`/copy + "Copy
  path"). Use the current-app pill precedent established by `SortPill` /
  `FilterPill` (the realisation of the prototype's `ac-topbar__btn`); both
  buttons consume `--ac-*` tokens for colour and glyph. (Supersedes the earlier
  icon-only `TopbarIconButton` framing — the prototype shows labelled buttons.)

## Acceptance Criteria

- [ ] Given a `LibraryDocView` route, both `Open in editor` and
  `Copy path` buttons are visible in the page header.
- [ ] Given `Copy path` is clicked for a document at
  `<project-root>/meta/work/0080-detail-page-header-actions.md`, the clipboard
  contains the raw relative path `meta/work/0080-detail-page-header-actions.md`
  (no scheme, no host, not percent-encoded) and a Toaster confirmation appears
  (consumes 0039).
- [ ] Given `navigator.clipboard.writeText` is unavailable, `Copy path`
  falls back to `document.execCommand('copy')` and still shows the Toaster
  confirmation.
- [ ] The `Open in editor` control renders as an `<a>` element carrying the
  computed `href` (so navigation is driven by a genuine user gesture).
- [ ] Given a VS Code-family editor is configured, the `Open in editor`
  anchor's `href` equals `{scheme}://file{abs}` where `{abs}` is the
  percent-encoded absolute path (which already begins with `/`), producing a
  **single** slash before the path and no line or column suffix — e.g. source
  `/Users/x/a b.md` with `vscode` yields `href="vscode://file/Users/x/a%20b.md"`
  (not `vscode://file//Users/...`).
- [ ] Given a JetBrains preset is configured, the `Open in editor` anchor's
  `href` equals
  `jetbrains://{tag}/navigate/reference?project={project}&path={rel}`,
  where `{project}` resolves from `visualiser.editor_project` or, when
  unset, the workspace directory basename, and `{rel}` is the percent-encoded
  workspace-relative path — e.g. project `myrepo`, workspace root `/ws`,
  source `/ws/sub dir/a.md` yields
  `href="jetbrains://web-storm/navigate/reference?project=myrepo&path=sub%20dir/a.md"`.
- [ ] Each enumerated preset maps to its documented scheme (VS Code family)
  or tool-tag (JetBrains) — verified table-driven across the full preset
  list, not just one representative per family.
- [ ] Given `visualiser.editor` equals a preset key (e.g. `cursor`), the
  preset's template is used even though the bare value contains no `://`;
  given a value matching no preset key, it is treated as a custom template.
- [ ] Given `visualiser.editor` is a custom URL template (a value containing
  at least one `{abs}`/`{rel}` placeholder), the `Open in editor` anchor
  substitutes the percent-encoded paths into `{abs}`/`{rel}` and uses the
  result verbatim — e.g. template `zed://file{abs}` with source path
  `/a b/c.md` yields `href="zed://file/a%20b/c.md"`.
- [ ] Given a custom template with no `{abs}`/`{rel}` placeholder (e.g.
  `myeditor://open`), or a value (preset or template) that resolves to a
  `javascript:`/`data:`/`vbscript:`/`blob:`/`file:` scheme, `Open in editor`
  renders disabled rather than emitting the href.
- [ ] Given the editor is set at more than one configuration layer,
  resolution honours env var > personal config > project config.
- [ ] Given no editor is configured, `Open in editor` is disabled and its
  tooltip text contains the literal string `visualiser.editor` (and/or
  `ACCELERATOR_VISUALISER_EDITOR`), naming the config the user must set to
  enable it.
- [ ] Both buttons render as labelled pill buttons (icon + visible text
  label) matching the prototype's `ac-topbar__btn` DocPage affordance and the
  current-app `SortPill`/`FilterPill` pill precedent, and their computed
  `color` (and the glyph's `fill`/`color`) resolve to `--ac-*` token values.

## Open Questions

- None outstanding. The JetBrains project-name and config-format questions
  are resolved in Requirements and Drafting Notes.

## Dependencies

- Blocked by: 0041 (Page.actions slot exists), 0039 (Toaster for
  copy-path confirmation).
- Related: 0100 (sibling work item that extends the same visualiser config
  mechanism; **not a prerequisite** — this story consumes no 0100 code and
  depends only on the pre-existing `visualiser:` config plumbing).
- Related: 0035 (Topbar / `TopbarIconButton` styling precedent these
  buttons follow).
- Depends on (expected already present): the pre-existing `visualiser:`
  config mechanism that shipped `visualiser.binary` /
  `ACCELERATOR_VISUALISER_BIN` (config block + `config.local.md` override +
  `ACCELERATOR_VISUALISER_*` env-var precedence). Not a blocker if already
  merged; the new `visualiser.editor` keys extend it. Confirm the env-var
  precedence chain is generic rather than `binary`-specific before starting.
- Blocks: none.

## Assumptions

- The visualiser is served on localhost, so the page runs in a secure
  context and `navigator.clipboard.writeText` is available; the
  `execCommand('copy')` fallback covers any non-secure-context edge case.
- The document's source-file absolute path and its project-root-relative
  path (used by `Copy path`, and by JetBrains/`{rel}`) are known to the
  route/server. If the existing route/server document loader does not
  already emit both paths, exposing them is server-side work that falls
  within this story's scope (per the corresponding Requirement).
- `Open in editor` is best-effort: the browser provides no success or
  failure signal when invoking a custom protocol, and may show a
  first-time "Open in …?" confirmation prompt. The UI does not attempt to
  detect whether the editor actually opened.
- The default scheme question is settled as **no default** — the feature
  ships disabled until an editor is explicitly configured.

## Technical Notes

- `Page.actions` prop is already wired; this story populates it for the
  `LibraryDocView` route component.
- VS Code-family URL form is `{scheme}://file{abs}`, where `{abs}` is the
  percent-encoded absolute path beginning with `/` — i.e. a single slash after
  `file` (`vscode://file/Users/x/a%20b.md`). Percent-encode spaces and special
  characters in the filename. Line/column are intentionally omitted per the
  open-at-top decision.
- JetBrains tool tags include `idea`, `web-storm`, `pycharm`, `php-storm`,
  `goland`, `rubymine`, `clion`, `rd`, `rustrover`. Its scheme is
  project-relative and requires a project name, unlike VS Code's absolute
  path; the project name defaults to the workspace directory basename and
  is overridable via `visualiser.editor_project`.
- `visualiser.editor` resolution: if the value matches a known preset key
  it expands to that preset's template; otherwise, if it contains an
  `{abs}`/`{rel}` placeholder it is treated as a custom template, substituted,
  and then scheme-guarded; a value with no placeholder (including a `://`-only
  value) is unresolvable → disabled. Templates support `{abs}` (absolute path)
  and `{rel}` (workspace-relative path) substitutions. The VS Code-family form
  is `{scheme}://file{abs}` (single slash — `{abs}` carries the leading `/`).
- Editor deep-links must be invoked from a real anchor click (a genuine
  user gesture); programmatic `window.location` assignment is blocked by
  browsers.
- A `file://` link is **not** a viable fallback: browsers silently block
  navigation to `file://` from an `http(s)` origin (including localhost),
  and `file://` opens the file in the browser rather than an editor. This
  is why the unconfigured state is a disabled button, not a `file://`
  link.
- Clipboard: `navigator.clipboard.writeText` requires a secure context
  (localhost qualifies); fall back to `document.execCommand('copy')`. Both
  `Copy path` and `Open in editor` are localhost-served and clipboard-safe.
- `Copy path` writes the raw project-root-relative path (forward slashes,
  no percent-encoding) — the same relative path that, percent-encoded,
  becomes the editor templates' `{rel}`. Copying the localhost route URL was
  rejected: it is bound to the ephemeral, randomly-ported instance and is
  not portable.
- Editor config precedence mirrors 0100: `visualiser:` config block +
  `config.local.md` personal override + `ACCELERATOR_VISUALISER_*` env var.

## Drafting Notes

- Scope narrowed from "every detail-page route" to `LibraryDocView` only
  (confirmed with author).
- `Open in editor` opens the source file at the top — no line/column
  targeting (confirmed with author).
- Editor selection follows 0100's visualiser-config precedence model
  (env → personal → project); chosen because 0100 already establishes that
  precedent for visualiser-runtime config.
- A `file://` fallback was explored at the author's suggestion and
  rejected on research grounds (browsers block `file://` from http
  origins); the unconfigured state is therefore a disabled button with a
  tooltip.
- Interpreted "disabled with instructions" as implying **no built-in
  default editor** so the disabled state is the genuine out-of-box
  experience — flag if a `vscode` default is preferred instead.
- VS Code family + JetBrains are in scope for this pass; Zed and Sublime
  are excluded because they have no reliable URL scheme for opening a file.
- JetBrains project name resolved as: explicit `visualiser.editor_project`
  override → workspace directory basename. Defaulting (rather than
  requiring config) removes the "JetBrains lacks project name → disabled"
  case; a wrong-but-present name fails the same silent best-effort way any
  deep-link can.
- Config format settled as a hybrid `visualiser.editor` field (preset name
  or custom URL template) rather than preset-only (can't express the
  JetBrains project name or future editors) or template-only (verbose and
  error-prone for the common case). The template escape hatch keeps Zed /
  Sublime / others wireable by the user without making them officially
  supported presets in this story.
- 0100 is a **sibling, not a prerequisite**: the `visualiser:` config
  block and `ACCELERATOR_VISUALISER_*` override mechanism already exist
  (they back `visualiser.binary`), so this story extends an existing
  mechanism and consumes no 0100 code. Recorded as "Related", not
  `blocked_by` (review finding).
- Scope kept bundled deliberately: both actions share the `Page.actions`
  slot and the source design-gap grouped them as one `DetailHeaderActions`
  affordance. The editor-config subsystem is the heavier half and could be
  extracted into its own work item via `/refine-work-item` if independent
  delivery timelines become a concern (review finding — kept together for
  now rather than split).
- The prototype's second button (`Copy link`, copying the route URL) was
  repurposed as `Copy path` (copying the project-root-relative document
  path) at the author's direction: the localhost URL is instance-specific
  and not portable, whereas the relative path is meaningful in commit
  messages, chat, and terminals. Title, Summary, Requirements, and
  acceptance criteria were updated accordingly.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0039, 0041, 0035, 0100
