---
type: plan-validation
id: "2026-06-09-0080-detail-page-header-actions-validation"
title: "Validation Report: Detail-Page Header Actions (Open in Editor, Copy Path)"
date: "2026-06-10T21:34:09+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-09-0080-detail-page-header-actions"
target: "plan:2026-06-09-0080-detail-page-header-actions"
relates_to: []
tags: [design, frontend, detail-page, config, editor-deeplink, clipboard]
last_updated: "2026-06-10T21:34:09+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Detail-Page Header Actions (Open in Editor, Copy Path)

### Implementation Status

✓ **Phase 1: Config plumbing + `/api/editor/config` + frontend hook** — Fully implemented
✓ **Phase 2: Header action button + `Copy path` action** — Fully implemented (delivered via the documented `HeaderActionButton` revision, not the original polymorphic `TopbarIconButton`)
✓ **Phase 3: `editor-link` helper + `Open in editor` action** — Fully implemented

All three phases' automated success criteria are checked off in the plan and verified green here. Manual verification items remain unchecked — they require a running visualiser with a real editor/clipboard and are listed under *Manual Testing Required* below.

### Automated Verification Results

✓ Rust builds and tests pass: `cargo test` → **527 passed** (21 suites, 26.3s)
✓ Frontend tests + typecheck pass: `mise run test:unit:frontend` → **2323 passed** across **112 files** (13.6s)
✓ Shell config tests pass: `bash scripts/test-write-visualiser-config.sh` → **41 passed, 0 failed**

Targeted evidence per phase:

- **Phase 1**
  - `editor_config.rs` test module covers configured round-trip, absent `editor` → `null`, absent `editor_project` → `project_root` basename, and whitespace-only `editor`/`editor_project` treated as absent (`absent_editor_is_null_and_project_defaults_to_basename`, `configured_editor_and_project_round_trip`, `whitespace_only_editor_treated_as_absent`, `whitespace_only_project_falls_back_to_basename`).
  - `config.rs` adds `editor_fields_absent_resolve_to_none` (asserts `None`, not just compile-checks) and `editor_fields_parse_when_present` (asserts `Some(...)`).
  - Shell suite asserts the discrete precedence cases: (a) absent → key genuinely absent; (b) config value emitted; (c) env over config; (d) empty env falls through to config; (e) whitespace-only → absent; (f) a `://`-and-space custom template round-trips intact under bash 3.2 (guards the macOS quoting gotcha).
- **Phase 2**
  - `clipboard.ts`, `HeaderActionButton.tsx`, `CopyPathButton.tsx` and their tests present and green; disabled-variant test asserts `aria-disabled` (not native `disabled`), still focusable, keeps `title`, fires no `onClick`, wires `aria-describedby`, plus a CSS-module assertion of the inactive affordance and forced-colors border.
- **Phase 3**
  - `editor-link.ts` + `editor-link.test.ts`, `OpenInEditorButton.tsx` + test present and green; preset table, single-slash regression, custom-template, percent-encoding, placeholder-required, scheme-required, dangerous-scheme deny-list and bypass vectors all covered by passing tests.
  - Route-level `LibraryDocView.test.tsx` exercises the `actions` prop guard.

### Code Review Findings

#### Matches Plan:

- **Config struct** (`config.rs`): both optional `#[serde(default)]` fields `editor` / `editor_project` added; all `Config { … }` literals updated with `editor: None, editor_project: None`.
- **Endpoint** (`api/editor_config.rs`): mirrors `kanban_config.rs`, resolves the JetBrains project default (configured value, else `project_root` basename), uses `#[serde(rename_all = "camelCase")]` to emit `editorProject`, and treats whitespace-only values as absent — exactly as specified.
- **Routing** (`api/mod.rs`): `mod editor_config;` and `.route("/api/editor/config", get(editor_config::get_editor_config))` wired.
- **Frontend plumbing**: `EditorConfig` interface (`types.ts`), `useEditorConfig` hook with `staleTime: Infinity` (`use-editor-config.ts`), and `editor: () => ['editor'] as const` query key follow the kanban precedent precisely.
- **Shell writer** (`write-visualiser-config.sh`): env→config read blocks, bash-3.2-safe `trim_ws`, `--arg` bindings, and omit-when-empty jq splices for both keys.
- **`editor-link.ts`**: preset table, `encodePath` segment-wise encoder, `schemeOf` control-char/whitespace normalisation, and `BLOCKED_SCHEMES` deny-list match the plan's code verbatim, including the placeholder-required and scheme-required gates.
- **Route wiring** (`LibraryDocView.tsx`): `actions` populated only when `hasResolvedDocument && entry`, with `<OpenInEditorButton absPath={entry.path} relPath={entry.relPath} />` and `<CopyPathButton relPath={entry.relPath} />`.
- **Documentation** (`SKILL.md`): full preset list, custom-template syntax with the must-contain-placeholder rule, the dangerous-scheme guard, env overrides + precedence, and the `zed://file{abs}` worked example.

#### Deviations from Plan:

- **Button presentation reworked from icon-only `TopbarIconButton` to labelled `HeaderActionButton` pill buttons.** This is a *documented, intentional* revision baked into the plan itself (Overview revision note, lines 40–53) and reflected back into the work item's Acceptance Criteria (labelled-pill requirement, AC lines 123–129, 180–183). Consequences honoured by the implementation:
  - A dedicated `DetailHeaderActions/HeaderActionButton.{tsx,module.css}` backs both actions (button | anchor | disabled), with a visible text label as the accessible name and the glyph wrapped `aria-hidden`.
  - `TopbarIconButton` was correctly **reverted** to its original icon-only form — no dead polymorphism left behind (verified: no `as`/`href`/`disabled` props remain).
  - Glyphs corrected to the prototype `edit` (pencil) and a copy/clipboard glyph.
  - All other plan behaviour (anchor-for-gesture, the two disabled sub-states, scheme guard, config plumbing) is unchanged.
- The `srOnly` visually-hidden utility the plan flagged as a possible Phase-3 deliverable already existed (`global.css:504`) and is reused — no new utility needed.

These are improvements/clarifications, not regressions; the plan and work item were kept in sync with the change.

#### Potential Issues:

- **None blocking.** The recorded deliberate tradeoffs stand: editor-config interpretation is split server (project default) / frontend (preset+template+scheme guard); `{abs}`-based VS Code links assume a single shared filesystem view (valid for the localhost single-host deployment); and `/api/editor/config` is a third per-field config endpoint — a consolidated `/api/visualiser/config` refactor remains a reasonable future work item once a fourth field appears, not now.
- **Transient tooltip flip** on first render (`data === undefined` briefly shows the unconfigured wording) is acknowledged in-code as a one-time sub-second flip under `staleTime: Infinity` — acceptable as designed.
- **Touch caveat**: the disabled hint is keyboard- and screen-reader-reachable (`aria-describedby`) but a native `title` is not shown on touch — a known minor gap, explicitly out of scope.

### Manual Testing Required:

1. Clipboard / Copy path:
  - [ ] `Copy path` appears bottom-right of the detail header, aligned with the title; both buttons sit together and match the topbar pill style in both themes.
  - [ ] Clicking copies `meta/work/0080-detail-page-header-actions.md` (raw relative path) and a Toaster confirmation appears.
  - [ ] With `navigator.clipboard` disabled (or a non-secure context), the `execCommand('copy')` fallback still copies and confirms.

2. Editor deep-link:
  - [ ] With `visualiser.editor: vscode`, clicking `Open in editor` opens the file in VS Code (first-time "Open in…?" prompt expected).
  - [ ] With `visualiser.editor: web-storm` + `visualiser.editor_project: myrepo`, the `href` is the JetBrains form with the project name.
  - [ ] With no editor configured, `Open in editor` is disabled and its tooltip names `visualiser.editor`.

3. Endpoint / config precedence:
  - [ ] `curl localhost:<port>/api/editor/config` with no editor set → `{"editor":null,"editorProject":"<project-dir-basename>"}`.
  - [ ] `visualiser.editor: cursor` in `config.md` → `"editor":"cursor"`; `ACCELERATOR_VISUALISER_EDITOR=vscode` overrides it; `visualiser.editor_project: myrepo` → `"editorProject":"myrepo"`.

### Recommendations:

- **Promote the plan to complete.** All automated criteria pass; this report sets the plan `status` to `complete`.
- Run the manual checklist above against a live visualiser before final sign-off — the automated suites cover the pure logic and component contracts but cannot exercise real clipboard/editor protocol handlers.
- When a fourth frontend-facing `visualiser.*` config field is next needed, raise the consolidated `/api/visualiser/config` refactor work item noted in the plan's deliberate tradeoffs rather than adding a fourth endpoint+hook+key quintuplet.
