---
type: plan-review
id: "2026-06-09-0080-detail-page-header-actions-review-1"
title: "Plan Review: Detail-Page Header Actions (Open in Editor, Copy Path)"
date: "2026-06-09T21:50:20+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-09-0080-detail-page-header-actions"
target: "plan:2026-06-09-0080-detail-page-header-actions"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, standards, usability, compatibility]
review_number: 1
review_pass: 2
tags: [design, frontend, detail-page, config, editor-deeplink, clipboard]
last_updated: "2026-06-10T20:05:25+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Detail-Page Header Actions (Open in Editor, Copy Path)

**Verdict:** REVISE

The plan is well-researched, cleanly decomposed into three independently mergeable
TDD phases, and disciplined about reuse — it faithfully mirrors the
`idle_timeout`/`kanban_config` config-plumbing precedent, isolates net-new logic into
pure, testable helpers, and keeps the `TopbarIconButton` refactor backward compatible.
The structural bones are sound and there are no critical or architecture-breaking
issues. It nonetheless needs revision before implementation: seven major findings
cluster around three themes — a clipboard failure path that is swallowed and reported
as success, a disabled "Open in editor" affordance that is inaccessible to keyboard,
touch, and screen-reader users while also conflating "not configured" with
"misconfigured", and an `editor-link` helper that has a concrete `href`-construction
bug (a doubled slash on every VS Code link) plus an unguarded `javascript:`/`data:`
scheme surface.

### Cross-Cutting Themes

- **Clipboard failure is swallowed and reported as success** (flagged by: code-quality,
  test-coverage, usability, standards) — `copyText` returns a `true`/`false` success
  signal, but `onCopyPath` discards it and always fires a `kind: 'ok'` "Copied path to
  clipboard" toast. A failed copy (both Clipboard API and `execCommand` fall through)
  shows a false confirmation, and no test covers the failure branch.
- **Disabled "Open in editor" affordance is inaccessible and shape-leaky** (flagged by:
  standards, usability, code-quality, architecture) — the unconfigured state is a
  native `<button disabled onClick={() => {}} title="Set visualiser.editor…">`. A
  `disabled` button is removed from the tab order and its `title` tooltip — the entire
  enablement-discovery mechanism — is unreachable by keyboard and never appears on
  touch (WCAG 1.3.1 / 4.1.2). The no-op `onClick` also exists only to satisfy the
  `ButtonProps` required-handler contract.
- **A configured-but-unresolvable editor silently disables with a misleading tooltip**
  (flagged by: usability, code-quality, correctness) — when `visualiser.editor` is set
  to a typo'd/unknown bare value (e.g. `webstorm`, `code`, `vscode-insider`),
  `buildEditorHref` returns null and the control falls back to the disabled state whose
  tooltip says "Set `visualiser.editor`…", telling the user to set a config they
  already set. There is no signal that the value was rejected and no surfaced list of
  valid presets.
- **Editor-link `href` construction has correctness and security gaps** (flagged by:
  correctness, security) — the VS Code branch produces a doubled slash
  (`vscode://file//Users/...`), a `://`-but-placeholder-free custom template yields a
  path-less link rather than the disabled state, and no scheme allow-list guards
  against `javascript:`/`data:` hrefs (contradicting the codebase's own
  `MarkdownRenderer` XSS posture).

### Tradeoff Analysis

- **Free-form passthrough (usability/simplicity) vs validation (security/feedback)**:
  the plan deliberately makes `editor` a free-form passthrough with "no server-side
  validation", which keeps the custom-template escape hatch simple and the server dumb.
  But the same decision is what allows the silent-disable-on-typo and the
  `javascript:`-scheme surface. Recommendation: keep the server passthrough, but add a
  *frontend* scheme allow-list in `buildEditorHref` (cheap, satisfies security) and
  distinguish the "unrecognised value" disabled sub-state in the UI (satisfies
  usability) — you get the escape hatch and the guard rails without server validation.

- **Endpoint-per-config-concern (consistency) vs consolidation (fewer round-trips)**:
  `/api/editor/config` is the third near-identical per-field endpoint+hook+query-key
  quintuplet. Following the kanban precedent is the right call for this story
  (consistency beats a speculative refactor); flag a consolidated
  `/api/visualiser/config` as a future tracked decision rather than acting now.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Code Quality / Test Coverage / Usability**: Clipboard failure is swallowed — `Copy path` always shows a success toast
  **Location**: Phase 2, Section 3 (`Copy path` button + wiring)
  `copyText` is designed to resolve `true`/`false`, but `onCopyPath` does `await copyText(relPath)` then unconditionally shows `kind: 'ok'`. On failure the user sees "Copied path to clipboard" with nothing on the clipboard, and no test exercises the failure path. Branch on the boolean (`ok` → confirmation, `!ok` → `kind: 'error'` toast) and add the failure-path test.

- 🟡 **Standards / Usability**: Disabled "Open in editor" hides its instructional tooltip from keyboard, touch, and screen-reader users
  **Location**: Phase 3, Section 2 (Open in editor control — disabled state)
  The disabled `<button>` is removed from the tab order and its `title` (the only place the "set `visualiser.editor`" instruction lives) is unreachable without a mouse hover. Fails WCAG 1.3.1 / 4.1.2. Use `aria-disabled="true"` on a still-focusable element and pair the instruction with an accessible name/description (`aria-label`/`aria-describedby`) rather than relying solely on `title`; drop the no-op `onClick`.

- 🟡 **Usability / Code Quality / Correctness**: A misconfigured editor value silently disables the button with a misleading "not configured" tooltip
  **Location**: Phase 3, Sections 1–2 (`buildEditorHref` null → disabled fallback)
  When `editor` is non-null but unresolvable (bare value matching no preset, no `://`/placeholder), the control falls back to the disabled state whose tooltip implies nothing is configured. Distinguish "not configured" (editor null) from "configured but unrecognised" (editor set, `buildEditorHref` null), and word the tooltip accordingly — ideally listing valid preset keys.

- 🟡 **Correctness**: VS Code preset produces a doubled slash before the absolute path
  **Location**: Phase 3, Section 1 (`buildEditorHref`, VS Code branch)
  `` `${preset.scheme}://file/${abs}` `` with `abs` already beginning with `/` yields `vscode://file//Users/x/a.md`. The custom-template AC example (`zed://file{abs}` → `zed://file/a%20b/c.md`) works precisely because `{abs}` carries the leading slash — the hardcoded preset adds an extra one. Change to `` `${preset.scheme}://file${abs}` `` and add a single-slash assertion test. Affects every VS Code-family link (the most common configured case).

- 🟡 **Security**: Free-form editor template flows into `<a href>` with no scheme allow-list
  **Location**: Phase 3, Section 1 (`buildEditorHref` custom-template branch)
  A custom template such as `javascript://%0aalert(document.domain)` satisfies the `://` gate and becomes a live `href`, executing script in the visualiser origin on click — contradicting the codebase's own XSS posture (`MarkdownRenderer` strips `javascript:` hrefs, locked by regression tests). The localhost/trusted-config model bounds the threat, and `config.local.md`/env are the *unreviewed* lowest-trust tiers. Add a scheme allow-list (preset + intended custom schemes) returning null for anything else; reject `javascript:`/`data:`/`vbscript:`/`blob:` with a table-driven test mirroring the MarkdownRenderer guard.

- 🟡 **Correctness**: Custom template containing `://` but no placeholder yields a path-less link instead of the disabled state
  **Location**: Phase 3, Section 1 (`buildEditorHref` custom-template branch)
  The branch fires on `includes('://')` even with no `{abs}`/`{rel}`; `replaceAll` finds nothing and the raw template (e.g. `myeditor://open`) becomes an `<a href>` that opens the editor at no file, masking the misconfiguration. Decide the contract explicitly (require a placeholder → return null otherwise, or document the passthrough) and pin it with a test.

- 🟡 **Test Coverage**: The ~12 `Config` literal call-sites and the server↔frontend contract loop lack runtime assertions
  **Location**: Phase 1, Section 2 (Typed config) + Phase 1/3 (endpoint ↔ disabled button)
  The `editor: None, editor_project: None` additions across ~12 sites are compile-checked only; the absent-field → `#[serde(default)]` → `None` path is exercised by a single extended `bare_config_json` case at most. Extend `bare_config_json` to assert both fields are `None`, add a positive parse test for a config carrying both keys, and add an `OpenInEditorButton` test asserting the disabled render when `useEditorConfig` returns `editor: null` (closing the whitespace-filter → disabled-state loop).

#### Minor

- 🔵 **Architecture**: Editor-config interpretation is split across server (project-default) and frontend (presets/templates/encoding) — defensible, but record it as a deliberate tradeoff. **Location**: Phase 1 (`editor_config.rs`) + Phase 3 (`editor-link.ts`)
- 🔵 **Architecture / Code Quality**: Disabled action forced into the `onClick`-required `ButtonProps` shape via `onClick={() => {}}` — make `onClick` optional or add an explicit disabled variant. **Location**: Phase 2/3 (polymorphic `TopbarIconButton`)
- 🔵 **Architecture**: Component home for `CopyPathButton`/`OpenInEditorButton` left undecided ("new, or co-located, or inline") — commit to extracted components so `LibraryDocView` stays a thin consumer and the affordance is reusable. **Location**: Phase 2/3
- 🔵 **Architecture**: VS Code `{abs}` deep-links implicitly assume the server and editor share a filesystem view (true for localhost) — make the single-host assumption explicit. **Location**: Phase 3 / Desired End State
- 🔵 **Architecture / Standards**: Endpoint proliferation — `/api/editor/config` is the third per-field config endpoint; following the precedent is right, but flag a future consolidation work item. **Location**: Phase 1
- 🔵 **Code Quality / Standards / Compatibility**: `ariaPressed` lives on shared `BaseProps`, so the anchor variant can still accept the toggle-only attribute — move it onto `ButtonProps`. **Location**: Phase 2, Section 1
- 🔵 **Code Quality / Standards**: Disabled styling is left as "if needed" — make `.toggle:disabled`/`[aria-disabled]` (cursor: default, muted `--ac-*`, suppressed hover/active) a definite Phase 2 deliverable. **Location**: Phase 2, Section 1 (CSS note)
- 🔵 **Correctness**: Whitespace-only `visualiser.editor` is written to config (only `[ -z ]` tested) and rescued server-side by `.trim()` — trim in the shell writer (idle_timeout precedent) or document the deferral; assert the round-trip. **Location**: Phase 1, Section 1
- 🔵 **Correctness**: `project_root.file_name()` returns `None` at filesystem root → empty `editorProject` — acceptable given domain constraints; add a documenting test. **Location**: Phase 1, Section 3
- 🔵 **Correctness**: First-render `data === undefined` causes a transient disabled→anchor flip with a brief "not configured" tooltip — gate wording on `!isPending` if it matters. **Location**: Phase 3, Section 2
- 🔵 **Test Coverage**: `buildEditorHref` edge cases not enumerated — empty `editorProject` (JetBrains), and the server-whitespace → frontend-disabled contract. **Location**: Phase 3 helper
- 🔵 **Test Coverage**: Shell precedence test is family-level; mirror the discrete idle_timeout cases (absent→key-absent, config-value, env-over-config, empty-env-falls-through) for both keys, asserting the JSON key is genuinely absent. **Location**: Phase 1 Success Criteria
- 🔵 **Test Coverage**: LibraryDocView wiring is only "consider an e2e check" — commit a route-level test asserting both actions render for a resolved doc and are absent when unresolved (exercises the `actions` guard + path plumbing). **Location**: Testing Strategy (Integration)
- 🔵 **Standards**: Query key `['editor-config']` diverges from the bare-root precedent (`kanban: () => ['kanban']`) — align to `['editor']` or flag the `-config` suffix as a deliberate new convention. **Location**: Phase 1, Section 4
- 🔵 **Standards**: `editor_config.rs` adds `#[serde(rename_all = "camelCase")]` (kanban_config does not) — keep it (cleaner) but note the intentional divergence from the cited template. **Location**: Phase 1, Section 3
- 🔵 **Compatibility**: Editor URL schemes are unverified external contracts (JetBrains hyphenated tags `web-storm`/`php-storm`; the assumption all VS Code-family editors share one shape) — cite sources in the preset table and smoke-test one per family. **Location**: Phase 3, Section 1
- 🔵 **Compatibility**: `document.execCommand('copy')` is deprecated and is effectively dead code under the localhost-secure-context assumption — keep it, but test the `false` return and comment that it's intentionally best-effort. **Location**: Phase 2, Section 2
- 🔵 **Compatibility**: `aria-pressed` emission changes from always to conditional — lock the new contract with present/absent tests (already in Phase 2 criteria). **Location**: Phase 2, Section 1
- 🔵 **Compatibility / Correctness**: Shell writer handles free-form values under macOS bash 3.2 (known project gotcha) — quote exactly as idle_timeout does and add a shell test with a `://`-and-space custom template to confirm it round-trips. **Location**: Phase 1, Section 1

#### Suggestions

- 🔵 **Security**: The editor anchor should emit `rel="noopener noreferrer"` (and consider `referrerPolicy="no-referrer"`) defensively, since custom templates can resolve to navigable `http(s)://` targets. **Location**: Phase 2/3 (anchor variant)
- 🔵 **Usability**: `Open in editor` gives no feedback when a deep-link silently fails (editor not installed / no protocol handler) — consider a one-time "Opening in your editor…" toast and/or surfacing the resolved target in the enabled anchor's tooltip. **Location**: Desired End State / Phase 3 manual verification
- 🔵 **Usability**: Consider an early (launch-time, terminal) warning when a bare `editor` value matches no preset, so misconfiguration is caught near where it's set rather than only as a disabled button. **Location**: Phase 1, Section 1
- 🔵 **Usability / Documentation**: Document the preset list and the `{abs}`/`{rel}` custom-template syntax at the user-facing config surface (`config.md` reference), with a worked example, so the escape hatch is discoverable. **Location**: What We're NOT Doing / Phase 1
- 🔵 **Correctness**: `replaceAll` correctly replaces all placeholder occurrences (right choice over `replace`) — optionally add a two-`{abs}` template test to pin the semantics. **Location**: Phase 3, Section 1

### Strengths

- ✅ Strong reuse of established patterns: the new endpoint + hook mirror
  `kanban_config.rs`/`use-kanban-config.ts` exactly, and the shell+Rust config plumbing
  mirrors `idle_timeout` — minimal novel surface to learn.
- ✅ Clean functional-core / imperative-shell separation: `editor-link.ts` and
  `clipboard.ts` are pure/near-pure helpers, trivially unit-testable without React or
  the server.
- ✅ Three independently mergeable, each-green phases with explicit dependency direction
  (Phase 2 standalone; Phase 3 depends on Phase 1 hook + Phase 2 anchor).
- ✅ The `TopbarIconButton` discriminated-union refactor keeps existing
  ThemeToggle/FontModeToggle callers type-valid and runtime-identical — verified
  backward compatible.
- ✅ Backward compatibility reasoned explicitly: `#[serde(default)]` + the
  omit-when-empty jq splice make absent config resolve to `None` → disabled button,
  confirmed against the existing `bare_config_json` test; the `deny_unknown_fields`
  coupling is acknowledged as a single-phase constraint.
- ✅ Phase 3 mandates a table-driven test across the **full** preset list (not one per
  family), directly matching the work item's AC and targeting the highest-risk net-new
  logic.
- ✅ Percent-encoding edge cases (spaces encoded, `/` preserved, the JetBrains
  `sub%20dir/a.md` example) are called out as explicit test cases; `encodePath`
  correctly preserves a leading slash.
- ✅ Correctly renders the unconfigured editor state as a `<button>` (not a disabled
  `<a>`), and emits `aria-pressed` only for genuine toggles.
- ✅ Sensible disabled-out-of-box default with a tooltip naming the exact config key —
  good zero-config DX and a discoverable enablement path (for mouse users).
- ✅ Copy path deliberately copies the portable relative path (not the ephemeral
  localhost URL) and keeps the absolute path out of the clipboard.

### Recommended Changes

1. **Consume the `copyText` boolean and branch the toast** (addresses: "Clipboard
   failure is swallowed"). `const ok = await copyText(relPath)`; show `kind: 'ok'` on
   success and a `kind: 'error'` toast on failure. Add the failure-path test to Phase 2
   Success Criteria.

2. **Make the disabled "Open in editor" accessible** (addresses: "Disabled affordance
   hides its tooltip", and the no-op `onClick` minors). Use `aria-disabled="true"` on a
   focusable element, attach the instruction via an accessible name/description, drop
   the `onClick={() => {}}`, and make `onClick` optional on `ButtonProps`. Add a
   definite `.toggle[aria-disabled]` style deliverable (cursor/opacity via `--ac-*`).

3. **Distinguish "not configured" from "configured-but-unrecognised"** (addresses:
   "Misconfigured value silently disables"). When `data.editor` is non-null but
   `buildEditorHref` returns null, render a disabled control whose tooltip says the
   value was unrecognised and lists/links valid preset keys.

4. **Fix the VS Code href and harden `buildEditorHref`** (addresses: "doubled slash",
   "placeholder-free custom template", "no scheme allow-list"). Drop the literal slash
   (`${scheme}://file${abs}`); return null for a `://` template with no `{abs}`/`{rel}`;
   add a scheme allow-list rejecting `javascript:`/`data:`/`vbscript:`/`blob:`. Add
   single-slash, placeholder-free, and dangerous-scheme tests, plus `rel="noopener
   noreferrer"` on the anchor.

5. **Close the config test loop** (addresses: "~12 Config sites / contract loop").
   Extend `bare_config_json` to assert both new fields are `None`; add a positive parse
   test; add an `OpenInEditorButton` disabled-on-`editor: null` test; specify the
   discrete shell precedence cases (incl. a `://`-and-space custom template under bash
   3.2) rather than family-level coverage.

6. **Commit the deferred structural decisions** (addresses: component-location and
   query-key minors). Extract a `DetailHeaderActions`/per-button component; pick and
   note the query-key convention; move `ariaPressed` onto `ButtonProps`.

7. **Record the deliberate tradeoffs** (addresses: architecture minors). Note the
   server/frontend split of editor-config interpretation, the single-host `{abs}`
   assumption, and the endpoint-proliferation/future-consolidation decision in the
   plan's Implementation Approach.

8. **Document the config surface** (addresses: usability/documentation suggestions).
   Add the preset list and `{abs}`/`{rel}` custom-template syntax (with a worked
   example) to the user-facing `config.md` reference, and consider a launch-time warning
   for unrecognised bare values.

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound and disciplined — reuses three established patterns
(kanban_config endpoint/hook, idle_timeout config plumbing, TopbarIconButton) rather
than inventing structure, with clean functional-core/imperative-shell separation and
independently mergeable phases. The main tension is a split of "editor config
interpretation" across server (JetBrains project default) and frontend (presets,
templates, encoding), slightly weakening cohesion. Boundaries are otherwise clear and
backward compatibility is preserved via serde defaults and the omit-when-empty jq idiom.

**Strengths**:
- Strong reuse of established patterns; the new endpoint+hook mirror kanban_config exactly.
- Clean pure-helper separation (editor-link.ts, clipboard.ts) testable without React/server.
- Three independently mergeable, each-green phases with explicit dependency direction.
- Discriminated-union TopbarIconButton keeps existing consumers backward compatible (open-closed).
- Backward compatibility and failure modes explicitly reasoned (serde default, omit-when-empty, deny_unknown_fields coupling).

**Findings**:
- 🔵 minor (high): Editor-config interpretation responsibility split across server and frontend — record as deliberate tradeoff. (Phase 1 editor_config.rs + Phase 3 editor-link.ts)
- 🔵 minor (medium): Disabled action button forced into onClick-required ButtonProps via `onClick={() => {}}` — make onClick optional or add a disabled variant. (Phase 2 polymorphic TopbarIconButton)
- 🔵 minor (medium): Frontend trusts server-canonicalised absolute path verbatim for VS Code deep-links — make the implicit single-host assumption explicit. (Phase 3 / Desired End State)
- 🔵 minor (medium): Component boundary for the two header actions left undecided — commit to extracted components keeping LibraryDocView thin. (Phase 2/3)
- 🔵 minor (high): Endpoint proliferation — each visualiser config concern gets its own endpoint; following the precedent is right but flag a future consolidation work item. (Phase 1)

### Code Quality

**Summary**: Well-researched, decomposed into three coherent TDD phases, and
consistently reuses established patterns, which bodes well for maintainability. Pure
helpers are cleanly separated and testable. Main concerns: a swallowed
clipboard-failure path that contradicts the helper's own return contract, a no-op
onClick forced by the polymorphic prop shape, an unresolved component-location
decision, and a disabled-state styling gap left as a conditional "if needed".

**Strengths**:
- Pure logic isolated into dependency-free helpers (clipboard.ts, editor-link.ts).
- Reuses concrete existing patterns end-to-end, reducing novel surface area.
- Clean phase decomposition; the resolution rule is defined once and referenced.
- Polymorphic TopbarIconButton keeps existing callers backward compatible.

**Findings**:
- 🔴 major (high): `copyText` boolean discarded — failed copy still shows a success toast. (Phase 2, Section 3)
- 🔵 minor (high): Disabled control uses `onClick={() => {}}` no-op only to satisfy required-onClick — make onClick optional. (Phase 3, Section 2)
- 🔵 minor (medium): Disabled styling left as "if needed"; `.toggle` hardcodes `cursor: pointer` with no `:disabled` rule — make it a definite deliverable. (Phase 2, Section 1 CSS note)
- 🔵 minor (medium): Component location left open across three options — commit to a single DetailHeaderActions component. (Phase 2/3)
- 🔵 minor (medium): Configured-but-unresolvable editor falls to the disabled state with a "set this config" tooltip — distinguish the two disabled cases. (Phase 3, Section 2)
- 🔵 suggestion (medium): `ariaPressed` on shared BaseProps remains available on the anchor variant (invalid ARIA) — move it onto ButtonProps. (Phase 2, Section 1)

### Test Coverage

**Summary**: Unusually test-conscious for a wiring exercise: explicitly TDD-driven,
names the net-new helpers warranting standalone unit tests, mandates a table-driven
assertion across the full preset list, and maps each phase's criteria to suites
matching existing conventions. Main gaps: failure-path coverage for the clipboard
fallback's user-facing behaviour, runtime regression coverage for the ~12 Config
literal call-sites, and a few under-specified edge cases.

**Strengths**:
- Phase 3 mandates a table-driven test across EVERY preset — the highest-risk net-new logic.
- Correctly identifies the three net-new pure helpers and assigns each a standalone unit test.
- Percent-encoding edge cases called out explicitly, matching the worked example.
- Shell precedence testing reuses the established test-write-visualiser-config.sh harness.
- TopbarIconButton backward-compat explicitly tested.

**Findings**:
- 🟡 major (high): Clipboard fallback failure behaviour not covered — failure shows a misleading success toast with no regression protection. (Phase 2 criteria + Copy-path test)
- 🟡 major (medium): ~12 Config literal call-sites compile-checked only; absent-field → None path barely exercised at runtime — extend bare_config_json and add a positive parse test. (Phase 1, Section 2)
- 🔵 minor (high): buildEditorHref edge cases not enumerated — empty editorProject (JetBrains), server-whitespace → frontend-disabled contract. (Phase 3 helper + Phase 1 endpoint)
- 🔵 minor (medium): Shell precedence test is family-level; mirror the discrete idle_timeout cases asserting the JSON key is genuinely absent. (Phase 1 criteria)
- 🔵 minor (medium): LibraryDocView wiring only "considered" for e2e — commit a route-level test for resolved/unresolved states. (Testing Strategy, Integration)

### Correctness

**Summary**: Logically well-structured; the config-precedence, whitespace-as-absent,
and env→config→omit chains are sound and faithful to the idle_timeout precedent. The
most significant risk is in the editor-link helper: the VS Code preset concatenates a
literal `file/` with a `/`-prefixed absolute path (doubled slash), and the
percent-encoding contract interacts subtly with that. A few branch-completeness and
template-substitution edge cases warrant tightening.

**Strengths**:
- env→config→omit chain faithfully modelled on idle_timeout; omit-when-empty → None → disabled.
- Whitespace-only config normalised to absent server-side via `.filter(|s| !s.trim().is_empty())`.
- Preset-vs-custom resolution order matches the canonical rule; bare non-matching values → null.
- encodePath preserves `/` separators and the leading slash; JetBrains example resolves exactly to the AC expectation.

**Findings**:
- 🔴 major (high): VS Code preset produces `vscode://file//Users/...` (doubled slash) — drop the literal slash since `abs` already begins with `/`. (Phase 3, Section 1, VS Code branch)
- 🟡 major (medium): Custom template with `://` but no `{abs}`/`{rel}` returns a path-less link verbatim instead of disabling — decide and pin the contract. (Phase 3, Section 1, custom branch)
- 🔵 minor (medium): Whitespace-only `editor` is written to config (only `[ -z ]` tested), rescued server-side by trim — trim in the shell writer or document the deferral. (Phase 1, Section 1)
- 🔵 minor (high): Disabled tooltip "Set visualiser.editor…" is misleading when the value is set-but-unrecognised — distinguish the sub-states. (Phase 3, Section 2)
- 🔵 minor (medium): `project_root.file_name()` is None at filesystem root → empty editorProject — acceptable, add a documenting test. (Phase 1, Section 3)
- 🔵 minor (low): `replaceAll` assumes ES2021 (fine on target) and correctly replaces all occurrences — optionally test a two-`{abs}` template. (Phase 3, Section 1)
- 🔵 minor (medium): First-render `data === undefined` causes a transient disabled→anchor flip with a brief "not configured" tooltip — gate wording on `!isPending` if it matters. (Phase 3, Section 2)

### Security

**Summary**: A local, single-user, localhost visualiser whose editor config is
documented as "trusted on par with the rest of the repo", which bounds the threat
surface. Two genuine concerns: (1) the free-form `editor` template is substituted into
a React `<a href>` with no scheme allow-list, producing a `javascript:`-class href
surface that contradicts the codebase's own XSS posture (MarkdownRenderer sanitizes
such hrefs and locks it with regression tests); and (2) `config.local.md`/env are
weaker, unreviewed trust tiers than the team-config "treat as code" statement implies.
Clipboard and the new endpoint are low-risk.

**Strengths**:
- Open in editor is a real `<a href>` driven by a genuine user click, not programmatic navigation.
- No new secrets/credentials/auth; the endpoint only reflects existing config + a basename, read-only.
- Copy path writes only the non-sensitive relative path on an explicit gesture; absolute path kept out.
- Feature defaults to disabled (omit-when-empty → None) — inert until explicit opt-in.

**Findings**:
- 🔴 major (high): Free-form template → `<a href>` with no scheme allow-list → `javascript:`/`data:` DOM XSS, contradicting the MarkdownRenderer XSS guard. Add an allow-list returning null for unexpected schemes + a table-driven test. (Phase 3, Section 1, custom-template branch)
- 🔵 minor (medium): `config.local.md`/env are unreviewed trust tiers the "treat config as code" statement doesn't cover — mitigated by the scheme allow-list; note in the field doc-comment. (Phase 1 / Overview)
- 🔵 suggestion (medium): Editor anchor should set `rel="noopener noreferrer"` (and consider `referrerPolicy="no-referrer"`) for the navigable-http(s) custom-template case. (Phase 2/3 anchor variant)

### Standards

**Summary**: Adheres closely to project conventions — the config-plumbing extension
mirrors idle_timeout/kanban_config faithfully and the new endpoint URL matches the
existing resource-noun hierarchy. Main gaps are accessibility: the disabled "Open in
editor" state relies on a native title tooltip unreachable by keyboard/SR users, and
the polymorphic component needs deliberate disabled/anchor affordances. A minor
query-key naming inconsistency is also worth aligning.

**Strengths**:
- New endpoint URL matches /api/kanban/config and /api/work-item/config noun hierarchy.
- Correct read-only GET semantics with staleTime: Infinity, consistent with use-kanban-config.
- Config plumbing precisely follows the idle_timeout precedent.
- New files land where expected (helpers under src/api/, endpoint as api/editor_config.rs).
- Backward-compatible config via #[serde(default)], documented in Migration Notes.
- Correctly renders the disabled state as a `<button>` not a disabled `<a>`.
- Keeps ariaPressed backward-compatible and emits aria-pressed only for toggles.

**Findings**:
- 🔴 major (high): Disabled `<button>` instruction lives only in a `title` — removed from tab order, unreachable by keyboard/SR (WCAG 1.3.1 / 4.1.2). Use aria-disabled on a focusable element + accessible name/description. (Phase 3, Section 2)
- 🔵 minor (high): No `:disabled`/`[aria-disabled]` rule on `.toggle` (still `cursor: pointer`, full hover) — add a token-based disabled treatment. (Phase 2, Section 1)
- 🔵 minor (medium): Query key `['editor-config']` diverges from the bare-root `['kanban']` precedent — align or flag the new convention. (Phase 1, Section 4)
- 🔵 minor (medium): `editor_config.rs` adds `rename_all = "camelCase"` (kanban_config does not) — keep it but note the intentional divergence. (Phase 1, Section 3)
- 🔵 minor (low): Copy path always fires an 'ok' toast without branching on copyText's boolean — use the existing error kind on failure. (Phase 2, Section 3)

### Usability

**Summary**: Strong on developer-config UX — mirrors the established config plumbing,
documents env-var precedence, ships a sensible disabled-out-of-box default, and surfaces
a discoverable field whose disabled tooltip names the exact config to set. Main gaps are
error/feedback: a bare value matching no preset silently disables with no signal; Copy
path always toasts success even on failure; and the disabled control's empty onClick +
native disabled removes it from tab order and hides its tooltip from keyboard/touch.

**Strengths**:
- Sensible zero-config default; absent editor cleanly resolves to None.
- Documented, consistent config precedence (env > personal > project).
- Disabled-state tooltip names the literal visualiser.editor config — actionable fix-it guidance (for mouse users).
- Progressive disclosure: preset keys for the common case, custom-template escape hatch for the rest.
- Dedicated endpoint + hook follow the kanban_config template.

**Findings**:
- 🔴 major (high): A misconfigured editor value silently disables with a misleading "not configured" tooltip and no list of valid presets — distinguish unconfigured vs unrecognised. (Phase 3, Section 1)
- 🟡 major (high): Copy path always shows success even when the write fails — branch on the boolean, show an error toast. (Phase 2, Section 3)
- 🟡 major (high): Disabled button hides its instruction from keyboard/touch (native disabled + title) and has a dead onClick — use aria-disabled on a focusable element + accessible description. (Phase 3, Section 2)
- 🔵 minor (medium): No early config feedback for editor values, unlike the idle_timeout block it copies — consider a launch-time warning for unrecognised bare values. (Phase 1, Section 1)
- 🔵 minor (medium): Open in editor gives no feedback when the deep-link silently fails — consider a one-time toast and/or surfacing the resolved target. (Desired End State / Phase 3 manual)
- 🔵 suggestion (medium): Custom-template placeholder vocabulary ({abs}/{rel}, the resolution rule) is undiscoverable at the config surface — document it with worked examples. (What We're NOT Doing / Phase 1)

### Compatibility

**Summary**: Overwhelmingly additive and backward compatible: new Rust fields use
#[serde(default)] under deny_unknown_fields (old configs parse cleanly, verified against
bare_config_json), the new endpoint/hook have no existing consumers, and the
TopbarIconButton refactor keeps existing button callers type-valid and runtime-identical.
The genuine risks are external: the editor deep-link schemes are unverified
cross-editor/cross-platform contracts, and execCommand('copy') is a deprecated fallback.

**Strengths**:
- config.rs change genuinely backward compatible (Option + serde default), confirmed via bare_config_json.
- TopbarIconButton refactor preserves the existing button-variant contract for ThemeToggle/FontModeToggle.
- New endpoint/hook/type/helpers are entirely additive — zero existing-consumer risk.
- Shell jq writer and Rust struct coupled in a single phase (deny_unknown_fields).
- execCommand used only as a fallback behind navigator.clipboard on the secure localhost context.

**Findings**:
- 🔵 minor (medium): Editor URL schemes are external version-dependent contracts hardcoded without a source (esp. JetBrains hyphenated tags) — cite sources and smoke-test one per family. (Phase 3, Section 1)
- 🔵 minor (medium): execCommand('copy') is deprecated and effectively dead code under the localhost-secure assumption — keep it, test the false return, comment it as best-effort. (Phase 2, Section 2)
- 🔵 minor (high): aria-pressed emission changes from always to conditional — lock the new contract with present/absent tests (already in Phase 2 criteria). (Phase 2, Section 1)
- 🔵 minor (medium): Free-form values flow through the shell writer under macOS bash 3.2 (known gotcha) — quote as idle_timeout does, add a `://`-and-space custom-template round-trip test. (Phase 1, Section 1)

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-10

**Verdict:** APPROVE

The re-review re-ran all 8 lenses against the revised plan. **Every** review-1 finding
(7 majors + 20 minors + 5 suggestions) is confirmed Resolved or accepted-as-recorded.
The edits, however, introduced **two new major issues** — both since fixed within this
pass (see "Fixed in-pass" below) — plus a handful of minors that were also addressed.
With those follow-up fixes applied and verified, no critical or major issue remains
outstanding; remaining items are minor/suggestion-level and are either addressed or
explicitly accepted as recorded tradeoffs.

### Previously Identified Issues

All review-1 findings — status from the pass-2 lenses:

- 🟡 **Code Quality / Test Coverage / Usability**: Clipboard failure swallowed — **Resolved** (handler branches on the `copyText` boolean; failure-path test added).
- 🟡 **Standards / Usability**: Disabled "Open in editor" inaccessible — **Resolved** (focusable `aria-disabled`, no native `disabled`; `.toggle[aria-disabled]` style committed). The `aria-describedby` half was found unwired in pass 2 → see new issues.
- 🟡 **Usability / Code Quality / Correctness**: Misconfigured value → misleading tooltip — **Resolved** (distinct "not recognised" sub-state).
- 🟡 **Correctness**: VS Code double slash — **Resolved** (`file${abs}` single-slash + regression test; work-item AC made concrete).
- 🟡 **Security**: No scheme allow-list (XSS) — **Resolved at the design level** (deny-list added) but pass-2 found the deny-list itself **bypassable** → see new issues.
- 🟡 **Correctness**: `://`-without-placeholder → path-less link — **Resolved** (placeholder required).
- 🟡 **Test Coverage**: Config test loop unclosed — **Resolved** (`bare_config_json` asserts `None`; positive parse test; discrete shell cases).
- 🔵 **Minors/suggestions** (20+5): `ariaPressed`→`ButtonProps`, query-key `['editor']`, shell `trim_ws`, `rename_all` note, component home, route-level test, config-docs deliverable, `rel="noopener noreferrer"`, deliberate-tradeoffs block — **all Resolved or accepted-as-recorded**. The endpoint-proliferation and `{abs}` single-host items are **Partial/accepted** (recorded tradeoffs, by design).

### New Issues Introduced (by the review-1 edits)

- 🔴 **Security / Correctness**: **Scheme deny-list was bypassable.** `schemeOf` sliced the raw scheme on the first `:` without normalising, so ` javascript:…` (leading space) and `java⇥script:…` (embedded TAB/CR/LF) evaded `BLOCKED_SCHEMES` while browsers still execute them — reintroducing the `javascript:` XSS via a hostile cloned repo's `config.md`. Relatedly, a placeholder-bearing but **scheme-less** template (e.g. `{rel}`) passed every gate and produced a relative within-SPA navigation href instead of disabling.
- 🟡 **Standards / Usability**: **`aria-describedby` declared but never wired.** The WCAG argument for the focusable-disabled redesign depended on an accessible description that `OpenInEditorButton` never actually rendered/referenced; it only set a mouse-hover-only `title`. (Touch users also can't see `title`.)
- 🔵 **Minor (code-quality)**: `data!.editor` non-null assertion guarded by a separately-derived boolean; misleading "mirrors MarkdownRenderer" rationale (MarkdownRenderer uses react-markdown's allow-list `urlTransform`, not a deny-list).
- 🔵 **Minor (standards)**: `'editor'` added to `SESSION_STABLE_QUERY_ROOTS` though the cited `kanban` precedent isn't in that set.
- 🔵 **Minor (usability)**: long unrecognised-value tooltip echoing the raw value verbatim into a native `title`.
- 🔵 **Minor (test-coverage)**: scheme-guard parser edge cases and the tooltip literal contract not pinned.

### Fixed in-pass (follow-up edits applied after the pass-2 lenses)

1. **Hardened the scheme guard** — `schemeOf` now rejects embedded TAB/CR/LF, `trimStart()`s leading whitespace, and matches a strict RFC-3986 scheme; `buildEditorHref` now requires a **non-null, non-blocked** scheme (scheme-less templates → null/disabled). Deny-list retained (preserves the editor escape hatch) but normalises before checking. Bypass-vector + scheme-less tests added.
2. **Wired `aria-describedby`** — `OpenInEditorButton` now renders a visually-hidden (`srOnly`) description element referenced by both disabled sub-states; the touch limitation is recorded as a known minor gap with a possible follow-up. Test asserts the description is referenced and the control is genuinely focusable (no native `disabled`).
3. **Minors** — bound the narrowed editor value (no `!`), corrected the deny-list rationale comment, dropped `'editor'` from `SESSION_STABLE_QUERY_ROOTS` to match the kanban precedent, truncated the echoed value + pointed to the config docs, and added the scheme-parser and tooltip-literal-contract test criteria.

### Assessment

The plan is in good shape and ready for implementation. The two new majors the
revision introduced were real (a genuine XSS bypass and an incomplete a11y fix) and
have been corrected and test-pinned in this pass. Outstanding items are minor and
either addressed or accepted as explicitly-recorded tradeoffs (endpoint proliferation,
the `{abs}` single-host assumption, the JetBrains-tag external contract verified via
manual check, and the sighted-touch disabled-hint gap). No further review pass is
required before implementation, though the implementer should treat the scheme-guard
and `aria-describedby` tests as must-haves rather than nice-to-haves.
