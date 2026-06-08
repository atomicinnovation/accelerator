---
type: codebase-research
id: "2026-06-09-0095-markdown-checkboxes-always-dark-mode-styled"
title: "Research: Theme-Reactive Markdown Task-List Checkboxes (0095)"
date: "2026-06-08T23:31:30+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0095"
parent: "work-item:0095"
relates_to: ["codebase-research:2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown"]
topic: "Theme-Reactive Markdown Task-List Checkboxes"
tags: [research, codebase, visualiser, markdown, theme, dark-mode, checkbox, design-tokens]
revision: "6f3f510d7176c1bb35f6ab123c200dac343dcc7e"
repository: "visualisation-system"
last_updated: "2026-06-08T23:31:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Theme-Reactive Markdown Task-List Checkboxes (0095)

**Date**: 2026-06-08T23:31:30+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 6f3f510d7176c1bb35f6ab123c200dac343dcc7e
**Branch**: (detached HEAD / workspace `visualisation-system`)
**Repository**: visualisation-system

## Research Question

For work item 0095 ("Theme-Reactive Markdown Task-List Checkboxes"): how are
GFM task-list checkboxes currently rendered in the visualiser's
`MarkdownRenderer`, why do they paint with a dark appearance in light theme,
and what is the concrete code surface — component, CSS, tokens, tests, and
visual-regression baselines — needed to replace the native control with a
custom token-driven glyph matching the frozen prototype design?

## Summary

The defect is real and well-characterised by the work item. GFM task lists
currently render as **native, app-unstyled `<input type="checkbox" disabled>`
controls**. The visualiser applies no CSS to them at all — their only
theme-aware lever is the UA `color-scheme` property, and there is **no
`[data-theme="light"]` rule forcing `color-scheme: light`**, so an
explicitly-light page on a dark-preferring OS lets the browser paint the
checkbox dark.

The fix has two moving parts:

1. **Component**: stop the native `<input>` from rendering and emit the
   prototype's custom structure (`ul.ac-md-tasklist` → `li.ac-md-task` +
   `span.ac-md-task__box` + `span.ac-md-task__label`). The established
   mechanism for this is react-markdown's `components` override map — the
   same lever the existing `pre` override uses (`MarkdownRenderer.tsx:29-44`).
2. **CSS + tokens**: add token-driven rules so the box border/fill and the
   done-state label colour come from `--ac-*` tokens, making them
   theme-reactive by construction. **All six tokens the design references are
   already defined in both themes**, so the prototype's `var(..., fallback)`
   forms never actually fall back.

Two important divergences from the sibling 0094 fix shape the work:

- **0094 was pure-CSS** (no component change); **0095 requires a component
  change** because a native `<input>` cannot be restyled into the prototype's
  span/box/label structure. This is the first MarkdownRenderer override beyond
  `pre`.
- **There is no `Icon`/`Glyph` `name="check"` component** in the live codebase
  (the prototype's `<Icon name="check" size={11}/>` was never ported). The
  tick must be an inline SVG (model: `SortPill.tsx`'s `CheckIcon`, path
  `m5 12 5 5L20 7`, `stroke="currentColor"`) or a pure-CSS pseudo-element
  (model: FilterPill's faux-checkbox with a hardcoded `#fff` tick).

Verification follows the well-trodden 0094 path: a fast CSS-as-text guard in
`migration.test.ts`, real-cascade `getComputedStyle` assertions in a Playwright
`*-resolved-styles.spec.ts` (both themes), a new fixture containing task-list
syntax, and **new light + dark visual-regression baselines** (darwin locally
via `--update-snapshots`, linux via the dispatch workflow). The existing unit
test that asserts native checkboxes (`MarkdownRenderer.test.tsx:115-123`) must
be rewritten, and any new chrome literals (1.5px border, 5px radius, 17px box,
9px gap, etc.) must be ledgered in the `EXCEPTIONS` array per ADR-0026 §3.

## Detailed Findings

### 1. Current rendering: where the native checkbox comes from

`MarkdownRenderer.tsx` wires `remark-gfm` (always), an optional `remarkWikiLinks`
plugin, `rehype-highlight`, and a `components` override map (`MARKDOWN_COMPONENTS`)
into `ReactMarkdown`.

- Plugin wiring: `MarkdownRenderer.tsx:69-85` (remark tuple memoised on
  resolver/pattern identity; `remarkGfm` always first).
- Override map: `MarkdownRenderer.tsx:29-44` — currently overrides **only `pre`**.
  No `li`, `ul`, or `input` override exists, so task lists pass straight through.
- The actual checkbox markup is generated downstream by
  `mdast-util-to-hast@13.2.1`, not by remark-gfm. Its `list-item` handler:
  - `node_modules/mdast-util-to-hast/lib/handlers/list-item.js:27` — triggers
    only when `typeof node.checked === 'boolean'` (true for `[x]`, false for
    `[ ]`, absent for non-task items);
  - `:43-48` — unshifts `<input type="checkbox" checked={…} disabled>` into the
    item's first `<p>`;
  - `:52` — sets `className: ['task-list-item']` on the `<li>`.

So the live DOM for `- [x] done\n- [ ] todo` is:

```html
<ul>
  <li class="task-list-item"><input type="checkbox" checked disabled /> done</li>
  <li class="task-list-item"><input type="checkbox" disabled /> todo</li>
</ul>
```

There is **no task-list / checkbox CSS** anywhere in
`MarkdownRenderer.module.css` (confirmed — the module has rules for headings,
paragraphs, `pre`, `.codeblock*`, inline `code`, tables, blockquote only).

### 2. Why it paints dark in light theme (`color-scheme` defect)

`--ac-*` tokens are declared per theme in `src/styles/global.css`:

- light under `:root` (`global.css:76-334`),
- dark under explicit `[data-theme="dark"]` ("MIRROR-A", `:341-406`),
- dark again under `@media (prefers-color-scheme: dark)` with selector
  `:root:not([data-theme="light"])` ("MIRROR-B", `:412-473`), a byte-equivalent
  hand-duplicate enforced by a parity test (`global.test.ts:125`).

`color-scheme` declarations:

- `global.css:333` — `:root { … color-scheme: light dark; }`
- `global.css:405` — `[data-theme="dark"] { … color-scheme: dark; }`
- `global.css:471` — the MIRROR-B mirror → `color-scheme: dark;`

**There is no `[data-theme="light"]` rule anywhere** forcing
`color-scheme: light`, and **`accent-color` is never set** (the app's
`--ac-accent` custom property is unrelated to the native CSS `accent-color`
property). `data-theme` is written to `<html>` by both a pre-React boot script
(`boot-theme.ts:20-38`) and the React effect (`use-theme.ts:30-32`), so after
first paint the element explicitly carries `data-theme="light"` or `"dark"`.
In the explicit-light-on-dark-OS case, `:root`'s permissive `light dark` lets
the UA honour the OS dark preference for native form controls — exactly the
bug. (Replacing the native control sidesteps this entirely; pinning
`color-scheme` is explicitly *not* the chosen fix per the work item's Open
Questions.)

### 3. Theme tokens the design needs — all defined in both themes

The design references `var(--ac-stroke-strong, var(--ac-stroke))` and
`var(--ac-stroke-strong, var(--ac-fg-faint))`. **`--ac-stroke-strong` exists in
both themes**, so the fallbacks never fire under normal cascade:

| Token | Light (`:root`) | Dark | Refs |
|---|---|---|---|
| `--ac-stroke-strong` | `rgba(32,34,49,0.18)` | `rgba(255,255,255,0.16)` | `global.css:94` / `:356`,`:429` |
| `--ac-stroke` | `rgba(32,34,49,0.10)` | `rgba(255,255,255,0.08)` | `:92` / `:354`,`:427` |
| `--ac-bg-card` | `rgb(255,255,255)` | `#131524` | `:85` / `:347`,`:420` |
| `--ac-accent` | `rgb(89,95,200)` | `#8a90e8` | `:95` / `:357`,`:430` |
| `--ac-fg-muted` | `rgb(95,99,120)` | `#a0a5b8` | `:90` / `:352`,`:425` |
| `--ac-fg-faint` | `#8b90a3` | `#6c7088` | `:91` / `:353`,`:426` |

Because the design only **consumes** already-themed colour tokens (it does not
add new ones), the MIRROR-A/MIRROR-B parity machinery in `global.test.ts` is
not engaged — no dark-block edit is required (mirroring 0094's "dark mode is
free" discovery).

### 4. The reference pattern: FilterPill faux-checkbox (and the missing Icon)

FilterPill is the closest existing theme-reactive faux-checkbox and is named in
the work item as the model. It is a **separate component, out of scope**, but
its technique is directly reusable:

- Markup (`FilterPill.tsx:146-149`): a single childless `<span aria-hidden>`;
  a11y semantics live on the parent `<li role="menuitemcheckbox" aria-checked>`
  (`:138-145`).
- CSS (`FilterPill.module.css:185-208`):
  - unchecked box → `border: 1px solid var(--ac-stroke-strong)` on
    `background: var(--ac-bg-card)`, `13×13px`, `border-radius: var(--radius-2)`;
  - checked box → `background: var(--ac-accent)`, `border-color: var(--ac-accent)`;
  - tick → `::after` pseudo-element (rotated `border-left`/`border-bottom` L
    shape), colour **hardcoded `#ffffff`**, NOT token-driven.

The check glyph options:

- **No generic `Icon` component exists.** The prototype's `name`-based `Icon`
  was deliberately not ported (comments at `routes/kanban/icons.tsx:1-4`, etc.).
- The `Glyph` component (work item 0037, `Glyph/Glyph.tsx`) is keyed off the 13
  doc-type keys only — **no `check`** — and its `size` is restricted to the
  `16 | 24 | 32` union, so the prototype's `size={11}` is not expressible.
- The canonical in-repo check glyph is the **local, unexported `CheckIcon` in
  `SortPill.tsx:119-135`**: a `viewBox="0 0 24 24"` SVG, path `m5 12 5 5L20 7`,
  `stroke="currentColor"` (so it inherits the box's `color`). This is the
  token-friendly route — the box sets `color: #fff` (as the prototype CSS does
  at `app.css:787`) and the tick inherits it.

### 5. The override-map mechanism (how the component change is made)

react-markdown's `components` map keys on the post-rehype HTML tag name; for
each match it calls the override with `{ children, node, ...rest }`, where
`node` is the hast node (must be destructured away to avoid leaking onto the
DOM, as `pre` does with `_node` at `MarkdownRenderer.tsx:30`).

Two viable approaches, both grounded in existing code:

- **(A) Override map** — add `li` (and `input`) keys to `MARKDOWN_COMPONENTS`.
  The `li` override receives `node.properties.className` containing
  `'task-list-item'` and can branch: when present, read the child `<input>`'s
  `checked` prop, drop the input, and emit the box+label structure; otherwise
  render a plain `<li>`. This is the most direct route and mirrors the `pre`
  override exactly.
- **(B) Remark plugin** — a `visit(tree, 'listItem', …)` transform attaching
  `data.hName`/`data.hProperties`/`data.hChildren`, modelled on
  `wiki-link-plugin.ts:48-65,26-36,123-136` (the codebase's only AST transform).
  Heavier; (A) is preferred since the checkbox markup is a downstream hast
  detail, not something remark-gfm itself emits.

Note the prototype markup (`ui.jsx:221-238`) sets the box `aria-hidden="true"`
and puts no interactive semantics on the item — the live a11y question (the
native input is `disabled`, so non-interactive already) should be decided
during planning; FilterPill's `aria-checked` parent pattern is the precedent if
semantics are wanted.

### 6. Target design (frozen prototype)

- Markup: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/ui.jsx:221-238`.
- Styles: same prototype's `src/app.css:778-793`:
  - `.ac-md ul.ac-md-tasklist { list-style: none; … }` (no bullet),
  - `.ac-md-task` is `display:flex; align-items:flex-start; gap:9px`,
  - `.ac-md-task__box` → `17×17px`, `border: 1.5px solid
    var(--ac-stroke-strong, var(--ac-stroke))`, `border-radius:5px`,
    `background: var(--ac-bg-card)`, `color:#fff`,
  - `.is-done .ac-md-task__box` → `background/border-color: var(--ac-accent)`,
  - `.is-done .ac-md-task__label` → `color: var(--ac-fg-muted);
    text-decoration: line-through; text-decoration-color:
    var(--ac-stroke-strong, var(--ac-fg-faint))`.
- Tick colour is hardcoded `#fff` painted on the accent fill — reads correctly
  in both themes (same trick FilterPill uses).

### 7. Tests and visual-regression baselines

**Unit (vitest)**:

- `MarkdownRenderer.test.tsx:115-123` currently asserts exactly two
  `input[type="checkbox"]` with `.checked` true/false. This test **locks in the
  behaviour being removed** and must be rewritten to assert the new structure
  (no native input; `.ac-md-task`, `.ac-md-task__box`, `is-done` modifier,
  tick presence).
- CSS-as-text guards live in `src/styles/migration.test.ts` (model:
  `migration.test.ts:455-471`, the existing MarkdownRenderer text guard, and
  the 0094 describe added at `:206-231`). Add assertions that the new rules
  consume the intended tokens.
- **ADR-0026 §3 `EXCEPTIONS` ledger** (`migration.test.ts:46-69`): a
  reverse-hygiene test (`:431-452`) fails if declared count ≠ observed count
  per `(file, literal)`. The new chrome literals — `1.5px` (border, "coloured
  ring width" category), `5px` (radius, off-scale / "in-between radii"), `17px`
  (fixed component dimension), `9px` gap, `2px` top margin — must be ledgered
  for `MarkdownRenderer.module.css` exactly. The hardcoded `#fff` tick colour is
  an irreducible literal too (a deliberate cross-theme constant, like
  FilterPill's; cf. ADR-0026 appendix `#ffffff` → `--ac-bg-card`, which would be
  wrong here since it must stay white on the accent fill — document the
  deviation).
- ADR-0036 font-size ban does not apply (no `font-size` literal introduced;
  the prototype's `14.5px` body size is already the module's concern, not the
  checkbox's).

**E2e / real-cascade (Playwright)** — jsdom resolves no `var()`/cascade, so
computed-style ACs must be Playwright, not vitest (documented at
`Glyph.test.tsx:169-172`):

- Config: `frontend/playwright.config.ts` — `visual-regression` project
  (testDir `tests/visual-regression`, snapshotDir
  `tests/visual-regression/__screenshots__`), `workers: 1`.
- Theme loop pattern: `for (const theme of ['light','dark'])` + `applyTheme`
  (`tests/visual-regression/helpers.ts:14-20`; dark sets
  `documentElement.dataset.theme='dark'` then waits a rAF). `tokens.spec.ts`
  also covers the `emulateMedia({colorScheme:'dark'})` OS-preference path.
- Resolved-styles model: 0094's `inline-code-resolved-styles.spec.ts` plus the
  shared `resolveToken(page, token)` helper in
  `tests/visual-regression/lib/expected-colours.ts` and `setTheme`. Assert the
  box border colour resolves to `--ac-stroke-strong` (unchecked) /
  `--ac-accent` (checked), fill `--ac-bg-card` / `--ac-accent`, tick `#fff`,
  and label `--ac-fg-muted` + `line-through` on done items. The AC's 3:1
  tick-to-fill contrast holds because `#fff` on both `--ac-accent` values
  (light `rgb(89,95,200)`, dark `#8a90e8`) clears 3:1.
- The `[class*="markdown"]` source-name-prefix selector is the precedent for
  locating CSS-modules-hashed classes (`typography-resolved-sizes.spec.ts`);
  but the new classes are **global** (`ac-md-tasklist` etc., from the prototype
  naming) if added globally, or module-scoped if added to
  `MarkdownRenderer.module.css` — planning must decide which, since it affects
  the locator. (The prototype uses global `ac-md-*` classes; the live module
  uses CSS-modules hashed names like `.markdown`.)

**Visual-regression baselines**:

- Baselines: `tests/visual-regression/__screenshots__/<spec>.spec.ts-snapshots/`,
  named `<id>-<theme>-<project>-<platform>.png` with both `-darwin` and
  `-linux` committed.
- Tolerance: `maxDiffPixelRatio: 0.05` per `toHaveScreenshot` call;
  `animations: 'disabled'`. No global threshold override (Playwright default
  per-pixel `threshold: 0.2`).
- **No existing fixture contains task-list syntax** — a search for `- [ ]` /
  `- [x]` across `server/tests/fixtures/meta/` returns nothing. The
  rendered-markdown "kitchen sink" is `library-doc-view.spec.ts` →
  `/library/plans/first-plan` → `server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`.
  New task-list syntax must be **appended** to a fixture (the 0094 invariant:
  append-never-prepend so `.first()` locators stay stable).
- Regeneration: `mise run test:e2e:visualiser` only *runs/compares*; the
  `--update-snapshots` flag is not exposed via mise/npm. Darwin baselines:
  `ACCELERATOR_VISUALISER_BIN=<path> npx playwright test --project
  visual-regression --update-snapshots` from `frontend/`. Linux baselines:
  dispatch the `Update visual regression baselines` workflow
  (`.github/workflows/update-visual-baselines.yml`, `workflow_dispatch`,
  ubuntu, pushes with `GITHUB_TOKEN` so it does not re-trigger Main CI — known
  gotcha). Both PNG sets must be committed; the VR acceptance criterion cannot
  pass until they exist.

## Code References

- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:29-44` — `MARKDOWN_COMPONENTS` override map (only `pre`); the lever to extend.
- `…/MarkdownRenderer/MarkdownRenderer.tsx:69-85` — plugin/ReactMarkdown wiring.
- `…/MarkdownRenderer/wiki-link-plugin.ts:48-65,26-36,123-136` — AST-transform model (remark route, if chosen).
- `…/MarkdownRenderer/MarkdownRenderer.module.css` — no task-list CSS today; where new rules go.
- `…/MarkdownRenderer/MarkdownRenderer.test.tsx:115-123` — native-checkbox unit test to rewrite.
- `…/frontend/node_modules/mdast-util-to-hast/lib/handlers/list-item.js:27-52` — source of the `<input>` + `task-list-item` class.
- `…/frontend/src/components/FilterPill/FilterPill.tsx:146-149` + `FilterPill.module.css:185-208` — faux-checkbox pattern.
- `…/frontend/src/components/SortPill/SortPill.tsx:119-135` — `CheckIcon` inline SVG (`m5 12 5 5L20 7`, `currentColor`).
- `…/frontend/src/components/Glyph/Glyph.tsx` — doc-type glyph (0037); no `check`, size-restricted.
- `…/frontend/src/styles/global.css:85,90-95,333,341-406,412-473` — tokens, `color-scheme`, theme mirrors.
- `…/frontend/src/api/use-theme.ts:30-32`, `…/api/boot-theme.ts:20-38` — `data-theme` writers.
- `…/frontend/src/styles/migration.test.ts:46-69,431-452,455-471` — `EXCEPTIONS` ledger + CSS-as-text guards.
- `…/frontend/playwright.config.ts`, `…/tests/visual-regression/helpers.ts:14-20`, `…/tests/visual-regression/library-doc-view.spec.ts`, `…/tests/visual-regression/lib/expected-colours.ts` — VR harness.
- `…/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md` — fixture behind `/library/plans/first-plan` (no task list yet).
- `.github/workflows/update-visual-baselines.yml` — linux baseline regeneration.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/ui.jsx:221-238`, `…/src/app.css:778-793` — target design.

## Architecture Insights

- **Theme-reactivity-by-token, not by detection** (ADR-0026, work item 0037):
  components source colours from `--ac-*` tokens and never branch on theme. The
  native checkbox violates this because it is painted by the UA, not the app.
  Replacing it with a token-driven span is the architecturally consistent fix —
  pinning `color-scheme` would merely patch the symptom on a control the app
  still does not own.
- **Override map is the extension point** for per-element markdown
  customisation; `pre` is the in-repo precedent. This is the first override
  beyond `pre`, so it sets the pattern for future ones (e.g. 0094 deliberately
  avoided adding a `code` override and did it in CSS — 0095 cannot, because the
  native `<input>` structure is wrong, not just unstyled).
- **The 0094 verification stack is the template**: CSS-as-text guard +
  real-cascade Playwright spec + `EXCEPTIONS` ledger + append-only fixtures +
  resolved-token assertions. 0095 reuses it almost verbatim, adding the
  component-test rewrite and the new VR baselines that 0094 (consuming
  already-baselined surfaces) did not need.
- **Irreducible chrome literals must be ledgered** (ADR-0026 §3). The prototype
  box uses several off-scale pixel values; the reverse-hygiene test makes
  silent introduction impossible. This is the most likely source of a
  red-on-first-run surprise.

## Historical Context

- `meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
  — the direct sibling bug and **strongest precedent**: same MarkdownRenderer,
  same token surface, full worked example of the test-first CSS-token approach,
  the `EXCEPTIONS` ledger discipline, the `resolveToken` Playwright helper, and
  the append-only-fixture invariant. 0094 explicitly lists 0095 as related
  markdown work.
- `meta/research/codebase/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
  — codebase research for that sibling.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` —
  governs `color-mix()` tints, ±2px substitution tolerance, the irreducible-
  literal categories (§3), and the `EXCEPTIONS` mechanism. §5 (theme-invariant
  code-block palette) explains the `:root`-only family pattern; the checkbox is
  *not* such a family (it consumes themed tokens).
- `meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` —
  token indirection layer the `--ac-*` values resolve through.
- `meta/work/0037-glyph-component.md` (+ plan/research) — the token-driven,
  theme-reactive glyph pattern the work item cites as the model.
- `meta/work/0034-theme-and-font-mode-toggles.md` (+ plan/research) — the
  `data-theme` / `color-scheme` mechanism.
- `meta/work/0076-code-block-syntax-highlight-palette.md` (+ plan/research) —
  introduced the existing `pre` override and the markdown code-block chrome.
- `meta/work/0077-shadow-and-dark-accent-token-audit.md` — dark `--ac-accent`
  correctness (the value the checked box fills with).
- `meta/reviews/work/0095-markdown-checkboxes-always-dark-mode-styled-review-1.md`
  — the work-item review.
- **Gap**: no plan, codebase research, or validation existed for 0095 before
  this document.

## Related Research

- `meta/research/codebase/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
- `meta/research/codebase/2026-05-12-0037-glyph-component.md`
- `meta/research/codebase/2026-05-08-0034-theme-and-font-mode-toggles.md`
- `meta/research/codebase/2026-05-21-0076-code-block-syntax-highlight-palette.md`

## Open Questions

1. **Override route (A vs B)**: `li`/`input` component overrides (preferred,
   mirrors `pre`) vs a remark `listItem` AST transform. Decide in planning.
2. **Global vs module-scoped class names**: the prototype uses global `ac-md-*`
   classes; the live module uses CSS-modules hashed names. This affects whether
   to add rules to `MarkdownRenderer.module.css` (hashed) or a global
   stylesheet, and changes the Playwright locator. The rest of the live
   MarkdownRenderer uses module-scoped `.markdown …` descendant selectors —
   consistency argues for module-scoped, but the prototype's class contract is
   `ac-md-tasklist` etc.
3. **Accessibility semantics**: the native input is `disabled` (non-interactive)
   today; the prototype box is `aria-hidden`. Confirm whether any `aria-checked`
   / role treatment is wanted (FilterPill's parent-role pattern is the
   precedent) or whether a purely-decorative box with a text label suffices.
4. **`#fff` tick as an irreducible literal**: it must stay white on the accent
   fill in both themes (so the ADR-0026 appendix's `#ffffff → --ac-bg-card`
   mapping does NOT apply here). Record the deviation in the `EXCEPTIONS`
   reason text.
5. **Which spec file hosts the new task-list cases and baselines** —
   `library-doc-view.spec.ts` (existing markdown route) vs a new dedicated spec,
   and which fixture to append the task-list syntax to.
