---
type: plan
id: "2026-06-09-0095-theme-reactive-markdown-task-list-checkboxes"
title: "Theme-Reactive Markdown Task-List Checkboxes Implementation Plan"
date: "2026-06-09T00:26:40+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0095"
parent: "work-item:0095"
derived_from: ["codebase-research:2026-06-09-0095-markdown-checkboxes-always-dark-mode-styled"]
relates_to: ["plan:2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown"]
tags: [visualiser, markdown, theme, dark-mode, checkbox, design-tokens, bug]
revision: "1a9767797beb2083943bd3ab5a44b5a430fc3d07"
repository: "visualisation-system"
last_updated: "2026-06-09T07:43:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Theme-Reactive Markdown Task-List Checkboxes Implementation Plan

## Overview

Replace the native `<input type="checkbox" disabled>` that `remark-gfm` +
`mdast-util-to-hast` emit for markdown task lists (`- [ ]` / `- [x]`) with a
custom, token-driven box + label structure, so task-list checkboxes become
theme-reactive by construction instead of relying on the user agent's
`color-scheme` paint. The native control is the one markdown element the
visualiser does not own — it is painted by the browser, and because there is no
`[data-theme="light"]` rule forcing `color-scheme: light`, an explicitly-light
page on a dark-preferring OS lets the browser paint the box dark.

This is the **first MarkdownRenderer component override beyond `pre`**: a native
`<input>` cannot be restyled into the prototype's span/box/label structure, so
(unlike the pure-CSS sibling 0094) a component change is required. The fix
follows the frozen prototype design (`ui.jsx:221-238`, `app.css:778-793`) and
the FilterPill faux-checkbox precedent, sourcing the box border/fill and the
done-state label colour from `--ac-*` tokens. It is delivered test-first in two
independently-mergeable phases.

## Current State Analysis

`MarkdownRenderer.tsx` wires `remarkGfm`, an optional `remarkWikiLinks`,
`rehypeHighlight`, and a `components` override map (`MARKDOWN_COMPONENTS`,
`MarkdownRenderer.tsx:29-44`) that currently overrides **only `pre`**. No `li`,
`ul`, or `input` override exists, so task lists pass through to the native
control generated downstream by `mdast-util-to-hast`.

The upstream list-item handler
(`frontend/node_modules/mdast-util-to-hast/lib/handlers/list-item.js:19-88`):

- triggers only when `typeof node.checked === 'boolean'` (true for `[x]`, false
  for `[ ]`); unshifts `<input type="checkbox" checked disabled>` into the
  item's **first `<p>`** (`:43-48`); sets `className: ['task-list-item']` on the
  `<li>`;
- **for tight lists** (the normal case — items not separated by blank lines)
  unwraps that `<p>` (`:70-71`: `if (child.tagName === 'p' && !loose)
  children.push(...child.children)`), so the `<input>` and label text become
  **direct children of the `<li>`**;
- **for loose lists** (any item separated by a blank line, or with multiple
  block children) the `<p>` is **kept**, so the `<input>` is nested **inside a
  `<p>`** child of the `<li>`.

So the live DOM for the tight `- [x] done\n- [ ] todo` is:

```html
<ul>
  <li class="task-list-item"><input type="checkbox" checked disabled /> done</li>
  <li class="task-list-item"><input type="checkbox" disabled /> todo</li>
</ul>
```

…but a loose list (`- [x] done\n\n- [ ] todo`) nests each input inside a `<p>`:
`<li class="task-list-item"><p><input … /> done</p></li>`.

**The fix must handle both shapes.** Rather than filter the rendered React
`children` for the `<input>` (which fails for loose lists, where the input is
nested in a `<p>` not a top-level child — the defect caught in review 1), the
plan:

1. **Overrides `input` → `null`** for the disabled task-list checkbox. This
   removes the native control wherever it sits (tight or loose), independent of
   the `<p>`-unwrapping detail, and removes the dependency on react-markdown's
   element-`type` resolution that a `c.type === 'input'` child-filter would
   couple to. Markdown produces no other `<input>`, so this is total.
2. **Reads `checked` by recursively searching the hast `node`** for the
   `input` element (descending into a `<p>` wrapper if present), so the checked
   state is correct for both shapes.
3. **Renders the box + label from an extracted `TaskListItem` component**, so
   `useId()` is called unconditionally (no Rules-of-Hooks hazard) and the label
   wraps whatever children survive the `input → null` override.

This makes the tight-vs-loose `<p>`-unwrapping behaviour **irrelevant** to
correctness — both render identically minus the native input.

There is **no task-list / checkbox CSS** anywhere in
`MarkdownRenderer.module.css` (it styles `.markdown` prose, `pre`/`.codeblock*`,
inline `code`, tables, blockquote only). The unit test
`MarkdownRenderer.test.tsx:114-123` asserts exactly two native
`input[type="checkbox"]` — it locks in the behaviour being removed and must be
rewritten.

### Key Discoveries:

- **The two-arg `var(--token, fallback)` form is banned.** `migration.test.ts`
  asserts zero `var(--ac-*, …)` two-arg sites
  (`migration.test.ts:346-353`, `VAR_FALLBACK_RE` at `:37`). The work item and
  prototype write `var(--ac-stroke-strong, var(--ac-stroke))` and
  `var(--ac-stroke-strong, var(--ac-fg-faint))`; these would **fail** the test.
  Because `--ac-stroke-strong` is defined in **both** themes
  (`global.css:94` light / `:356` dark), the fallback never fires anyway — the
  plan uses single-arg `var(--ac-stroke-strong)` everywhere, preserving the
  resolved colour while satisfying the convention.
- **All six design tokens exist in both themes**, so the box/label colours are
  theme-reactive purely by consuming them — no dark-block edit, and the
  MIRROR-A/MIRROR-B parity machinery in `global.test.ts` is not engaged
  (same "dark mode is free" property as 0094):
  `--ac-stroke-strong` (`:94`/`:356`), `--ac-stroke` (`:92`/`:354`),
  `--ac-bg-card` (`:85`/`:347`), `--ac-accent` (`:95`/`:357`),
  `--ac-fg-muted` (`:90`/`:352`), `--ac-fg-faint` (`:91`/`:353`).
- **The tick cannot clear a literal 3:1 in dark theme** — and that is an
  accepted, documented deviation. White (`#ffffff`) on the dark accent
  `#8a90e8` (`rgb(138,144,232)`) computes to **2.90:1**; on the light accent
  `rgb(89,95,200)` it is **5.39:1**. FilterPill paints the same `#ffffff` tick
  on the same `--ac-accent` in dark (also ~2.9:1) and is accepted. Per decision,
  the plan keeps the prototype's `#ffffff` tick and **softens AC2**: assert the
  tick is `#ffffff` and the fill is `--ac-accent`; the exact 3:1 ratio becomes a
  manual design note, not an automated gate.
- **No generic check Icon exists.** The prototype's `<Icon name="check"/>` was
  never ported; `Glyph` has no `check` key and a size-restricted union. The
  canonical in-repo check glyph is the local, unexported `CheckIcon` in
  `SortPill.tsx:119-135` (`viewBox="0 0 24 24"`, path `m5 12 5 5L20 7`,
  `stroke="currentColor"`). The plan adds a **local** `CheckIcon` to
  MarkdownRenderer modelled on it; `stroke="currentColor"` inherits the box's
  `color: #ffffff` so the tick is white on the accent fill.
- **`border-radius` literals are banned outright (ADR-0039), with no EXCEPTIONS
  escape.** `migration.test.ts:758-788` (`BORDER_RADIUS_LITERAL_RE`) rejects any
  literal `border-radius` value, and `:824-838` fails if any irreducible
  `EXCEPTIONS` reason contains the word "radius". The radius ladder
  (`tokens.ts:192-201`) is `--radius-0/1/2/3/4/6/8/12` (+ pill) — **no 5px
  step** — so the prototype's `5px` box corner is written as `var(--radius-4)`
  (4px, within ADR-0026's ±2px tolerance). This means there is **no `5px`
  literal to ledger** and no radius wording in any reason.
- **The `EXCEPTIONS` reverse-hygiene ledger keys per `(file, literal)` and
  requires declared == observed exactly** (`migration.test.ts:478-499`;
  `HEX_RE = /#[0-9a-fA-F]{3,8}\b/g` at `:30`, `PX_REM_EM_RE` at `:31`). The new
  box chrome introduces `1.5px`, `2px` (×2), `6px`, `9px`, `17px` (×2), and
  `#ffffff` literals in `MarkdownRenderer.module.css` — every one must be
  ledgered (`migration.test.ts:64-68`); the existing `5px` entry is left
  untouched at count 1. `12px` margin is `var(--sp-3)` and the corner is
  `var(--radius-4)` (no literal). No `font-size` literal is introduced, so the
  ADR-0036 ban (`migration.test.ts:337-344` family) does not engage.
- **`AC5_FLOOR` is a strict ratchet** (`migration.test.ts:432-448`,
  `AC5_FLOOR = 981`, slack 0 → floor must equal observed). The new CSS adds
  **8** `var(--*)` references in a `*.module.css` file (the seven colour/spacing
  refs plus `var(--radius-4)`), so the floor must be bumped to the new observed
  count (expected **989**; use the value the failing test prints).
- **react-markdown overrides are React components**, so hooks are valid — but
  `useId()` must be called **unconditionally**. The `li` override returns early
  for non-task items, so the hook lives in an extracted `TaskListItem` component
  (always rendered for task items, always calling `useId()` first) rather than
  after a conditional return in the `li` override itself. `useId()` provides the
  per-item id that gives the box its accessible name via `aria-labelledby` (see
  A11y below). (`useId` requires React 18+; the app pins React 19, so this is a
  no-op constraint, noted in a code comment.)
- **The override couples only to stable hast surface, behind caret-ranged deps.**
  The retained couplings are the `task-list-item` class name and the input
  node's `properties.checked` boolean — both stable, documented hast outputs of
  `mdast-util-to-hast` (resolved `13.2.1`, range `^13`; `react-markdown ^9`,
  `remark-gfm ^4`). The fragile `<p>`-unwrapping internal and the
  `c.type === 'input'` React-element coupling are **designed out** (see Current
  State). These markdown deps are caret-ranged with only the lockfile pinning
  them; the loose+tight unit/e2e guards added below are the loud-failure
  mechanism if a floated upgrade changes the hast shape (pinning is noted as an
  alternative in Migration Notes).
- **No fixture contains task-list syntax today** (research confirmed `- [ ]` /
  `- [x]` absent across `server/tests/fixtures/meta/`). The component change
  therefore moves **no existing VR baseline** — non-task `<ul>`/`<li>` render
  byte-identically through the overrides. The baseline churn is isolated to
  Phase 2, when the task list is added to the fixture.
- **The 0094 plan
  (`meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`)
  is the verification template**: CSS-as-text guard in `migration.test.ts`,
  real-cascade `*-resolved-styles.spec.ts` (both themes) using the shared
  `resolveToken`/`setTheme` helpers (`tests/visual-regression/lib/expected-colours.ts:79-100`),
  append-only fixtures, and the `EXCEPTIONS` discipline. 0095 reuses it and adds
  the component-test rewrite and new VR baselines 0094 did not need.

## Desired End State

Markdown task lists render as a token-driven structure that adapts to light and
dark automatically, with the native `<input>` gone:

- DOM: `ul[class*="tasklist"]` (no list marker) → `li[class*="task"]` (with a
  `taskDone` modifier when checked) → `span[class*="taskBox"]`
  (`role="checkbox"`, `aria-checked`, `aria-readonly`, `aria-labelledby`,
  containing a `<svg>` tick when checked) + `span[class*="taskLabel"]`.
- Unchecked box: `1.5px solid var(--ac-stroke-strong)` border on
  `var(--ac-bg-card)` background, `var(--radius-4)` (4px) corner, `17×17px`.
- Checked box: background and border `var(--ac-accent)`, white (`#ffffff`) tick.
- Done label: `var(--ac-fg-muted)` + `line-through` with
  `text-decoration-color: var(--ac-stroke-strong)`; not-done label renders as
  normal body text.
- No native `<input type="checkbox">` in the rendered output, in **tight or
  loose** task lists.
- Screen readers still announce checked/unchecked state (preserved via
  `role="checkbox"` + `aria-checked` + `aria-readonly`).
- Light and dark resolve correctly; FilterPill is untouched; the full vitest +
  Playwright suites pass, including the `EXCEPTIONS` hygiene check, the
  `var()`-fallback ban, the `var()`-resolves-to-declared-token test, and the
  `AC5` ratchet.

Verification: the rendered task list (a checked + an unchecked item) is captured
as a new focused visual-regression baseline in each theme (darwin + linux), and
the existing `library-doc-view` baselines are regenerated to include it.

## What We're NOT Doing

- **Not pinning `color-scheme`** or setting `accent-color` on the native control
  — the work item explicitly rejects the symptom patch in favour of owning the
  control (Open Questions: moot).
- **Not porting the prototype's global `ac-md-*` class names.** Per decision,
  the classes are module-scoped (CSS-modules) like the rest of MarkdownRenderer
  and the `.codeblock` precedent; no acceptance criterion tests class names.
- **Not using the prototype's `var(--token, fallback)` two-arg forms** — banned
  by `migration.test.ts`; single-arg `var()` is used (the fallback is moot).
- **Not changing the dark `--ac-accent`** to chase a literal 3:1 tick contrast
  (out of scope; would touch 0077's accent-token surface and break prototype +
  FilterPill parity). The `#ffffff` tick is kept and AC2 softened.
- **Not touching FilterPill** (separate component; its tests/snapshots must pass
  unchanged) or any non-task list rendering (ordinary `<ul>`/`<li>` forward
  through the overrides unchanged).
- **Not adding new design tokens** — the box chrome's off-scale pixels are
  irreducible literals admitted via the `EXCEPTIONS` ledger.

## Implementation Approach

Test-first throughout, mirroring 0094. Phase 1 delivers the complete functional
change (component + CSS + ledger + unit/CSS-as-text guards) and is fully green
**without any fixture, e2e, or baseline change** — because no fixture yet
contains a task list and non-task lists render identically, nothing visual
moves. Phase 2 introduces the task list into the fixture and owns **all**
real-cascade and visual-regression verification in one place (resolved-styles
spec, focused screenshot, and the consequent baseline regeneration). The two
phases are independently mergeable in order: Phase 1 is a self-contained
code+CSS+unit change; Phase 2 is purely additive test/fixture/baseline work that
builds on it.

Verification uses the repo's `mise` tasks (preferred over raw `npm`):

- `mise run test:unit:frontend` — Vitest (frontend unit + CSS-as-text guards +
  `EXCEPTIONS`/`var()`/`AC5` machinery).
- `mise run test:e2e:visualiser` — Playwright e2e; auto-builds the frontend +
  dev server and wires `ACCELERATOR_VISUALISER_BIN`, exercising the real
  cascade against the built SPA + server.

There is no `mise`/npm `lint` task; type-checking is
`npm --prefix skills/visualisation/visualise/frontend run typecheck`
(`tsc --noEmit`). Playwright `--update-snapshots` is not exposed via mise — see
Phase 2 for the darwin (`npx playwright … --update-snapshots`) and linux
(dispatch workflow) baseline-capture commands.

### Accessibility design (applies to Phase 1)

The native control was `disabled` but still announced its checked/unchecked
state. To preserve that without re-introducing an interactive control, the
**box** span carries `role="checkbox"`, `aria-checked={checked}`, and
**`aria-readonly` (not `aria-disabled`)**, and is given an accessible name via
`aria-labelledby` pointing at the label span (id from `useId()`). The `<svg>`
tick is `aria-hidden`.

Two deliberate ARIA decisions, recorded here and as a code comment so they are
intentional rather than incidental:

- **`aria-readonly`, not `aria-disabled`** (review-1 finding). The intent is
  "shows state, cannot be changed" — which is exactly `aria-readonly`.
  `aria-disabled` marks a control inert/unavailable, and several screen readers
  de-emphasise or skip disabled form controls, which would suppress the very
  state announcement this override exists to preserve. `aria-readonly` keeps the
  checked/unchecked state in the AT output. The manual-verification step
  confirms the announcement on at least one of NVDA / VoiceOver.
- **Role on the box, not the parent `li`** (vs FilterPill). FilterPill puts
  `role="menuitemcheckbox"` + `aria-checked` on its parent `<li>` because that
  `<li>` is an interactive menu item; overriding its implicit `listitem` role is
  correct *there*. Here the markdown `<li>` is a plain, non-interactive list
  item, so overriding its role would strip list semantics and item count from
  AT. Placing `role="checkbox"` on the box span keeps the `<ul>`/`<li>` list
  semantics intact while still exposing the checkbox — the correct WAI-ARIA
  placement for a standalone read-only control. The divergence from FilterPill
  is therefore intentional and case-driven, not an inconsistency.

---

## Phase 1: Replace the native checkbox with a token-driven box + label

### Overview

Add the `input` (→ `null`), `li`, and `ul` component overrides, the extracted
`TaskListItem` component, a local `CheckIcon`, the module-scoped CSS for the box
+ label in both states, the `EXCEPTIONS` ledger entries and `AC5_FLOOR` bump,
the rewritten structural unit tests, and the CSS-as-text guards. Covers
acceptance criteria 1–4 and 6 at the unit / CSS-text level. Fully green with no
fixture, e2e, or baseline change. Independent.

### Changes Required:

#### 1. Test — rewrite the native-checkbox unit test (write first, red)

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.test.tsx`
**Changes**: Replace the `Story 0076 AC4` test at `:115-123` (`renders a GFM
task list with interactive checkboxes`) with assertions on the new structure.
Query by ARIA/structure (not hashed class names) so it is robust to CSS-modules
hashing. Add **four** cases pinning the branches the override introduces: tight
task list, **loose** task list, mixed list, and a plain (non-task) list.

```tsx
it('renders a tight GFM task list as token-driven boxes, not native inputs (0095)', () => {
  const { container } = render(
    <MarkdownRenderer content={'- [x] done\n- [ ] todo\n'} />,
  )
  // No native control survives.
  expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(0)
  // Two read-only checkbox boxes, state preserved for AT.
  const boxes = screen.getAllByRole('checkbox')
  expect(boxes).toHaveLength(2)
  expect(boxes[0]).toHaveAttribute('aria-checked', 'true')
  expect(boxes[1]).toHaveAttribute('aria-checked', 'false')
  boxes.forEach((b) => expect(b).toHaveAttribute('aria-readonly', 'true'))
  // Accessible name comes from the label (aria-labelledby).
  expect(screen.getByRole('checkbox', { name: 'done' })).toBe(boxes[0])
  expect(screen.getByRole('checkbox', { name: 'todo' })).toBe(boxes[1])
  // Tick present only on the checked box.
  expect(boxes[0].querySelector('svg')).not.toBeNull()
  expect(boxes[1].querySelector('svg')).toBeNull()
  // Label text preserved verbatim (children-survival, not just normalised name).
  const labels = container.querySelectorAll('li [class*="taskLabel"]')
  expect(labels[0].textContent?.trim()).toBe('done')
  expect(labels[1].textContent?.trim()).toBe('todo')
  // aria-labelledby ids are unique per item (useId, not a shared constant).
  const id0 = boxes[0].getAttribute('aria-labelledby')
  const id1 = boxes[1].getAttribute('aria-labelledby')
  expect(id0).toBeTruthy()
  expect(id0).not.toBe(id1)
})

it('handles a LOOSE task list (input nested in <p>) with no native control (0095)', () => {
  // Blank line between items → loose list → mdast-util-to-hast keeps the <p>.
  const { container } = render(
    <MarkdownRenderer content={'- [x] done\n\n- [ ] todo\n'} />,
  )
  expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(0)
  const boxes = screen.getAllByRole('checkbox')
  expect(boxes).toHaveLength(2)
  expect(boxes[0]).toHaveAttribute('aria-checked', 'true')
  expect(boxes[1]).toHaveAttribute('aria-checked', 'false')
})

it('forwards a plain (non-task) list unchanged — no checkbox boxes (0095)', () => {
  const { container } = render(<MarkdownRenderer content={'- a\n- b\n'} />)
  expect(screen.queryAllByRole('checkbox')).toHaveLength(0)
  // Ordinary <li> still produced (markers come from the default <ul>).
  expect(container.querySelectorAll('ul > li')).toHaveLength(2)
})

it('renders task items as boxes even in a mixed list (0095)', () => {
  const { container } = render(
    <MarkdownRenderer content={'- [x] done\n- plain item\n'} />,
  )
  // Task item still boxed; native input still gone.
  expect(screen.getAllByRole('checkbox')).toHaveLength(1)
  expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(0)
  // A mixed list keeps its default markers — the <ul> must NOT get the
  // marker-removing tasklist class (the items.every(isTaskItem) branch).
  expect(container.querySelector('ul')?.className ?? '').not.toMatch(/tasklist/)
})
```

(`@testing-library/jest-dom` matchers `toHaveAttribute` are already used across
the suite. jsdom resolves no `var()`/cascade, so all colour/border assertions
live in Phase 2's Playwright spec, per `Glyph.test.tsx:169-172`. The
`[class*="taskLabel"]` selector matches the CSS-modules source-name prefix, the
same precedent the Playwright specs use.)

#### 2. Test — CSS-as-text guards (write first, red)

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: Add a `describe('MarkdownRenderer task-list rule (0095)')` beside
the existing 0094 block (`:520-544`), asserting the new rules consume the
intended tokens and the single-arg `var()` form.

```ts
describe('MarkdownRenderer task-list rule (0095)', () => {
  const path = 'components/MarkdownRenderer/MarkdownRenderer.module.css'
  const css = cssBySrcRelative.get(path)
  const itIfPresent = css ? it : it.skip
  itIfPresent('tasklist removes the list marker', () => {
    expect(css!).toContain('list-style: none')
  })
  itIfPresent('unchecked box borders off --ac-stroke-strong (single-arg var)', () => {
    expect(css!).toContain('border: 1.5px solid var(--ac-stroke-strong)')
  })
  itIfPresent('box fills off --ac-bg-card', () => {
    expect(css!).toContain('background: var(--ac-bg-card)')
  })
  itIfPresent('checked box fills + borders off --ac-accent', () => {
    expect(css!).toContain('background: var(--ac-accent)')
    expect(css!).toContain('border-color: var(--ac-accent)')
  })
  itIfPresent('done label is muted + struck through off --ac-stroke-strong', () => {
    expect(css!).toContain('color: var(--ac-fg-muted)')
    expect(css!).toContain('text-decoration: line-through')
    expect(css!).toContain('text-decoration-color: var(--ac-stroke-strong)')
  })
})
```

The pre-existing guardrail tests in the same file then act as ratchets that the
implementation must keep green: the `var(--*, fallback)` ban (`:346-353`), the
`var()`-resolves-to-declared-token test (`:386-423`), the `EXCEPTIONS` hygiene
check (`:478-499`), and the `AC5` ratchet (`:436-448`).

#### 3. Implementation — local `CheckIcon`

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx`
**Changes**: Add a local `CheckIcon` modelled on `SortPill.tsx:119-135`, sized
to sit inside the 17px box. `stroke="currentColor"` inherits the box's
`color: #ffffff`.

```tsx
function CheckIcon() {
  return (
    <svg
      width="11" height="11" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" strokeWidth="3"
      strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"
    >
      <path d="m5 12 5 5L20 7" />
    </svg>
  )
}
```

#### 4. Implementation — `input`/`li`/`ul` overrides + `TaskListItem`

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx`
**Changes**: Extend `MARKDOWN_COMPONENTS` (`:29-44`) with `input`, `ul`, and
`li`. Add `useId` to the React import and import `hast` types so the node shape
is expressed once (not re-cast inline at each use). The structure-shaping logic
lives in helpers + a `TaskListItem` component so that (a) `useId()` is always
called unconditionally and (b) the `task-list-item` / input couplings are named
in one place with a comment pointing at the upstream handler.

```tsx
// ADD useId to the EXISTING import — keep Children, isValidElement, useMemo,
// ReactNode, which fenceLanguageOf / the `pre` override still use:
import {
  Children,
  isValidElement,
  useId,
  useMemo,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from 'react'
import type { Element, ElementContent } from 'hast'

// The class mdast-util-to-hast stamps on task-list <li> nodes
// (handlers/list-item.js:52). The native <input> it injects (:43-48) is always
// `disabled` and sits either as a direct <li> child (tight lists) or inside a
// <p> (loose lists); `findCheckbox` searches both so the override is
// shape-agnostic.
const TASK_LIST_ITEM_CLASS = 'task-list-item'

function isTaskItem(node: Element | undefined): boolean {
  const cls = node?.properties?.className
  return Array.isArray(cls) && cls.includes(TASK_LIST_ITEM_CLASS)
}

// Recursively find the injected checkbox <input> (direct child for tight
// lists, nested in a <p> for loose lists) and return it.
function findCheckbox(children: ElementContent[]): Element | undefined {
  for (const child of children) {
    if (child.type !== 'element') continue
    if (child.tagName === 'input') return child
    const nested = findCheckbox(child.children) // child: Element ⇒ ElementContent[]
    if (nested) return nested
  }
  return undefined
}

// Always calls useId() at the top — no conditional hook. Props extend the <li>
// attribute set so forwarded props (and the hast `className`) are typed; we
// pull `className` out and compose it rather than letting `{...rest}` clobber
// the module-scoped classes.
function TaskListItem({
  checked,
  className,
  children,
  ...rest
}: { checked: boolean } & ComponentPropsWithoutRef<'li'>) {
  const labelId = useId()
  // Compose: our module classes first, then the upstream `task-list-item`
  // class (if any) — so styles.task/styles.taskDone always apply.
  const liClass = [styles.task, checked && styles.taskDone, className]
    .filter(Boolean)
    .join(' ')
  return (
    <li className={liClass} {...rest}>
      <span
        className={styles.taskBox}
        role="checkbox"
        aria-checked={checked}
        aria-readonly
        aria-labelledby={labelId}
      >
        {checked && <CheckIcon />}
      </span>
      {/* Block-level label so a loose list's <p> nests validly; it is the
          aria-labelledby target. */}
      <div id={labelId} className={styles.taskLabel}>{children}</div>
    </li>
  )
}

const MARKDOWN_COMPONENTS: Components = {
  pre({ children, node: _node, ...rest }) { /* unchanged */ },

  // Drop the GFM task-list checkbox wherever it sits (tight or loose),
  // independent of the <p>-unwrapping detail and of react-markdown's
  // element-type resolution. Scoped to the disabled checkbox mdast-util-to-hast
  // injects (list-item.js:46) so a future legitimate input is NOT swallowed.
  // INVARIANT: the `li` override below relies on this entry having removed the
  // native control from the children it wraps as the label.
  input({ node: _node, ...props }) {
    if (props.type === 'checkbox' && props.disabled) return null
    return <input {...props} />
  },

  ul({ children, node, ...rest }) {
    // Mirror the prototype's `isTaskList = items.every(...)`: only a pure
    // task list drops its markers and gutter (a mixed list keeps markers).
    const items = (node?.children ?? []).filter(
      (c): c is Element => c.type === 'element' && c.tagName === 'li',
    )
    const isTaskList = items.length > 0 && items.every(isTaskItem)
    return isTaskList
      ? <ul className={styles.tasklist} {...rest}>{children}</ul>
      : <ul {...rest}>{children}</ul>
  },

  li({ children, node, ...rest }) {
    if (!isTaskItem(node)) return <li {...rest}>{children}</li>
    const checked = Boolean(
      findCheckbox((node?.children ?? []) as ElementContent[])?.properties
        ?.checked,
    )
    // `children` is the rendered label; the native <input> is already removed
    // by the `input` override above (INVARIANT), so no child-filtering is
    // needed and the loose-list <p> wrapper (if any) is preserved intact.
    return (
      <TaskListItem checked={checked} {...rest}>
        {children}
      </TaskListItem>
    )
  },
}
```

Notes:
- **Loose lists work without special-casing**: the `input` override removes the
  control whether it is a direct `<li>` child or nested in a `<p>`, and
  `findCheckbox` reads `checked` from either shape. A loose item's label keeps
  its `<p>` wrapper — now inside a **block `<div className={styles.taskLabel}>`**
  (valid nesting; `<span>` cannot contain a `<p>`), with the inner `<p>` margins
  reset in CSS so a loose item aligns with its box like a tight one.
- **className is composed, not clobbered**: `TaskListItem` pulls `className` out
  of the forwarded props and concatenates it after `styles.task`/`styles.taskDone`,
  so the module classes always apply (a bare `{...rest}` spread would let the
  hast `task-list-item` class overwrite them — JSX last-wins).
- **No Rules-of-Hooks hazard**: `useId()` is called unconditionally inside
  `TaskListItem`, which is only ever rendered for task items.
- **Cross-entry invariant**: `li` correctness depends on the `input → null`
  entry; both carry a comment naming it, the `input` override is scoped to the
  `disabled` GFM checkbox (so it neither over-reaches nor silently breaks), and
  the tight/loose `input[type=checkbox]` count tests pin the dependency.
- **`{...rest}` is forwarded** on every branch; `node` is destructured away so
  the hast node never leaks onto the DOM (same discipline as `pre`).
- **Typed once**: `Element`/`ElementContent` from `hast` replace inline casts;
  the `items.filter((c): c is Element => …)` predicate and the
  `ComponentPropsWithoutRef<'li'>` props type keep `npm … run typecheck` green.

#### 5. Implementation — module-scoped CSS

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`
**Changes**: Append the task-list rules (direct module classes, like
`.codeblock`). Single-arg `var()`; `12px` margin via `var(--sp-3)`; corner via
`var(--radius-4)` (ADR-0039 bans literal `border-radius` — see §6); `#ffffff`
tick.

```css
/* Task lists (`- [ ]` / `- [x]`) — token-driven box + label replacing the
   native <input type="checkbox">; theme-reactive by construction. See 0095.
   Single-arg var(): --ac-stroke-strong is defined in both themes, and the
   two-arg fallback form is banned by migration.test.ts.
   border-radius is a token (--radius-4 = 4px ≈ the prototype's 5px, within
   ADR-0026 ±2px tolerance): ADR-0039 bans literal border-radius outright. */
.tasklist { list-style: none; margin: var(--sp-3) 0; padding-left: 2px; }
.task { display: flex; align-items: flex-start; gap: 9px; margin: 6px 0; }
.taskBox {
  flex: none;
  margin-top: 2px;
  width: 17px;
  height: 17px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1.5px solid var(--ac-stroke-strong);
  border-radius: var(--radius-4);
  background: var(--ac-bg-card);
  color: #ffffff;
}
/* Block label so a loose list's <p> child nests validly (a <span> cannot
   contain block content; the label is the aria-labelledby target, so invalid
   nesting could perturb the accessible name). Reset the inner prose-<p>
   margins so a loose item aligns with its box like a tight one. */
.taskLabel { min-width: 0; }
.taskLabel > p { margin: 0; }
.taskDone .taskBox {
  background: var(--ac-accent);
  border-color: var(--ac-accent);
}
.taskDone .taskLabel {
  color: var(--ac-fg-muted);
  text-decoration: line-through;
  text-decoration-color: var(--ac-stroke-strong);
}
```

These are **bare (unscoped) module classes**, following the `.codeblock*`
precedent in this file rather than the `.markdown …`-descendant pattern the prose
rules use — deliberate, because they are applied directly via `styles.task` etc.
The label text still sits inside the `.markdown` ancestor in the DOM, so it
inherits the prose `font-weight: 300` / `line-height` cascade; confirm this in
the Phase 2 baseline review (a wrong inherited weight would show as a visible
diff).

#### 6. Implementation — `EXCEPTIONS` ledger + `AC5_FLOOR`

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: In the `components/MarkdownRenderer/MarkdownRenderer.module.css`
block (`:64-68`), **add** the new task-box literals (exact counts; the hygiene
test requires declared == observed). **No reason may contain the word "radius"**
— the AC5/0090 gate (`migration.test.ts:824-838`) fails on any irreducible
reason mentioning it; the corner is a `var(--radius-4)` token, so there is no
radius literal to ledger at all.

  - `{ literal: '1.5px', count: 1, kind: 'irreducible', reason: 'task box border width — below --sp-1 floor' }`
  - `{ literal: '2px', count: 2, kind: 'irreducible', reason: 'task box margin-top + tasklist padding-left — below --sp-1 floor' }`
  - `{ literal: '6px', count: 1, kind: 'irreducible', reason: 'task row vertical margin — between --sp-1 and --sp-2' }`
  - `{ literal: '9px', count: 1, kind: 'irreducible', reason: 'task box→label gap — between --sp-2 and --sp-3' }`
  - `{ literal: '17px', count: 2, kind: 'irreducible', reason: 'task box width + height — fixed component dimension, no token' }`
  - `{ literal: '#ffffff', count: 1, kind: 'irreducible', reason: 'task tick stroke on --ac-accent — theme-invariant white (mirrors FilterPill checkmark)' }`
    — mirroring the phrasing of FilterPill's existing `#ffffff` entry
    (`migration.test.ts:256`) so the two white-on-accent entries read as the same
    convention. (The ADR-0026 appendix `#ffffff → --ac-bg-card` mapping does NOT
    apply: the tick must stay white on the accent fill, not track the card
    surface — same rationale as FilterPill, hence the shared phrasing.)
- **Do NOT touch `5px`** (stays count `1` — the inline-code padding). The task
  box corner is `var(--radius-4)`, not a `5px` literal, so there is no second
  `5px` occurrence. Leave `1px` (7), `0.4rem` (1), `4px` (1) unchanged too — the
  box border is `1.5px`, not `1px`.

Then bump the `AC5_FLOOR` (`:432`) to the new observed count. The CSS adds **8**
`var(--*)` references (`--sp-3`, `--ac-stroke-strong` ×2, `--ac-bg-card`,
`--ac-accent` ×2, `--ac-fg-muted`, `--radius-4`), so the floor moves `981` →
**989**. Run the failing test first and set `AC5_FLOOR` to the exact observed
value it reports, with a `// 0095:` comment per the bump protocol (`:425-431`).
(`--radius-4` is a declared `RADIUS_TOKENS` member, so the
`var()`-resolves-to-declared-token test stays green.)

### Success Criteria:

#### Automated Verification:

- [x] Vitest passes — rewritten unit tests (tight/loose/mixed/plain), new 0095
  CSS-as-text guards, `EXCEPTIONS` hygiene, `var()`-fallback ban,
  `var()`-resolves-to-declared-token, the **ADR-0039 border-radius literal ban**
  (`migration.test.ts:758-788`) and its **radius-reason gate** (`:824-838`), and
  the `AC5` ratchet all green: `mise run test:unit:frontend`
- [x] Type-checking clean: `npm --prefix skills/visualisation/visualise/frontend run typecheck`
- [x] The Playwright suite is unchanged and still green (no fixture/baseline
  touched in this phase): `mise run test:e2e:visualiser`

#### Manual Verification:

- [ ] Render markdown containing `- [x]` / `- [ ]` in the visualiser: the
  checkbox is a custom box (not a native control), with a white tick on the
  accent fill when checked and a struck-through muted label.
- [ ] Toggle dark mode (`document.documentElement.dataset.theme = 'dark'`): the
  unchecked box border/fill and the done-label colour all adapt; there is no
  dark-painted native control in light theme.
- [ ] On at least one of NVDA / VoiceOver, each item is announced as a
  **read-only** checkbox with its checked/unchecked state and the label as its
  name — confirming `aria-readonly` (not `aria-disabled`) keeps the state in the
  AT output. Verify a loose task list (blank line between items) announces
  identically to a tight one.

---

## Phase 2: Real-cascade verification + visual-regression baselines

### Overview

Introduce a task list into the rendered-markdown fixture and own all
real-cascade and visual-regression verification: a resolved-styles Playwright
spec (both themes) for AC1–4 and the softened AC2, a focused task-list
screenshot for AC5, and regeneration of the `library-doc-view` baselines that
the fixture append disturbs. Independent and purely additive; builds on Phase 1.

### Changes Required:

#### 1. Fixture — append a task list (one checked, one unchecked)

**File**: `skills/visualisation/visualise/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`
**Changes**: **Append** (never prepend — keeps existing `.first()` locators
stable, the 0094 invariant) **two** task lists — a tight one (the primary
subject for resolved-styles + screenshot) and a loose one (blank line between
items, so the real pipeline exercises the nested-`<p>` path Phase 1 handles).
The intervening headings keep them as two separate `<ul>`s rather than one
merged loose list.

```markdown

#### Task list (tight)

- [x] Ship the release notes
- [ ] Capture the visual-regression baselines

#### Task list (loose)

- [x] Tag the release

- [ ] Announce in the changelog
```

#### 2. Test — resolved-styles Playwright spec, both themes (write first, red)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/task-list-resolved-styles.spec.ts` (new)
**Changes**: Model on `inline-code-resolved-styles.spec.ts`; reuse `setTheme`
and `resolveToken` from `lib/expected-colours.ts:79-100`. Scope the
colour/cardinality assertions to the **first** (tight) task list so exactly one
checked + one unchecked box is in play; locate boxes by ARIA (hash-free) and
labels by the `taskDone` / `task` source-name prefixes. Cover AC1 (no native
input, unchecked box chrome), AC2 (accent fill + white tick), **AC3 (no
marker)**, and **both halves of AC4** (done = muted + struck; not-done = normal).

```ts
import { test, expect } from '@playwright/test'
import { setTheme, resolveToken } from './lib/expected-colours'

test.use({ viewport: { width: 1280, height: 720 } })

const ROOT = '[class*="markdown"]'
// First task list = the tight one appended for this spec; scoping to it keeps
// the checked/unchecked cardinality at exactly one each.
const TIGHT = `${ROOT} [class*="tasklist"] >> nth=0`
const px = (s: string) => parseFloat(s) // tolerant numeric compare for sub-px widths

for (const theme of ['light', 'dark'] as const) {
  test(`task-list boxes are token-driven (${theme})`, async ({ page }) => {
    await page.goto('/library/plans/first-plan')
    if (theme === 'dark') await setTheme(page, 'dark')
    const list = page.locator(TIGHT)
    const checkedBox = list.locator('[role="checkbox"][aria-checked="true"]')
    const uncheckedBox = list.locator('[role="checkbox"][aria-checked="false"]')
    const doneLabel = list.locator('[class*="taskDone"] [class*="taskLabel"]')
    const todoLabel = list.locator('li:not([class*="taskDone"]) [class*="taskLabel"]')

    // AC1 / requirement: no native control survives, anywhere (tight + loose).
    await expect(page.locator(`${ROOT} input[type="checkbox"]`)).toHaveCount(0)
    // Cardinality guards so a structural failure reports behaviourally, not as a
    // generic locator timeout, and a duplicate checked box can't pass silently.
    await expect(checkedBox).toHaveCount(1)
    await expect(uncheckedBox).toHaveCount(1)
    await expect(doneLabel).toHaveCount(1)
    await expect(todoLabel).toHaveCount(1)

    // AC3: the task list shows no list-item marker (real-cascade, not text).
    const listStyle = await list.evaluate((n) => getComputedStyle(n).listStyleType)
    expect(listStyle).toBe('none')

    // Unchecked box (AC1): --ac-stroke-strong border on --ac-bg-card, ~1.5px,
    // 4px corner (var(--radius-4)), 17px.
    const u = await uncheckedBox.evaluate((n) => {
      const c = getComputedStyle(n)
      return {
        borderTopColor: c.borderTopColor, backgroundColor: c.backgroundColor,
        borderTopWidth: c.borderTopWidth, borderTopLeftRadius: c.borderTopLeftRadius,
        width: c.width, height: c.height,
      }
    })
    expect(u.borderTopColor).toBe(await resolveToken(page, '--ac-stroke-strong'))
    expect(u.backgroundColor).toBe(await resolveToken(page, '--ac-bg-card'))
    expect(px(u.borderTopWidth)).toBeCloseTo(1.5, 1) // tolerate sub-px serialisation
    expect(u.borderTopLeftRadius).toBe('4px') // var(--radius-4)
    expect(u.width).toBe('17px')
    expect(u.height).toBe('17px')

    // Checked box (AC2, softened): fill + border --ac-accent, white tick present.
    const c = await checkedBox.evaluate((n) => {
      const s = getComputedStyle(n)
      return { backgroundColor: s.backgroundColor, borderTopColor: s.borderTopColor }
    })
    const accent = await resolveToken(page, '--ac-accent')
    expect(c.backgroundColor).toBe(accent)
    expect(c.borderTopColor).toBe(accent)
    // Redundant-cue guarantee the softened AC2 relies on: checked fill differs
    // from the unchecked fill, so state is conveyed independent of tick contrast.
    expect(c.backgroundColor).not.toBe(u.backgroundColor)
    const tickColor = await checkedBox.locator('svg').evaluate((n) => getComputedStyle(n).color)
    expect(tickColor).toBe('rgb(255, 255, 255)') // #ffffff tick; 3:1 is a manual note (dark ≈ 2.9:1)

    // AC4 done half: muted + line-through.
    const d = await doneLabel.evaluate((n) => {
      const s = getComputedStyle(n)
      return { color: s.color, line: s.textDecorationLine }
    })
    expect(d.color).toBe(await resolveToken(page, '--ac-fg-muted'))
    expect(d.line).toContain('line-through')

    // AC4 not-done half: normal body text — positively equals the prose body
    // colour (--ac-fg), not merely "not muted".
    const t = await todoLabel.evaluate((n) => {
      const s = getComputedStyle(n)
      return { color: s.color, line: s.textDecorationLine }
    })
    expect(t.line).not.toContain('line-through')
    expect(t.color).toBe(await resolveToken(page, '--ac-fg'))
  })
}

// AC1/AC2/AC4 divergence: box + label colours must actually change between
// themes, not merely resolve to a token value.
test('task-list colours diverge between light and dark', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  const lightStroke = await resolveToken(page, '--ac-stroke-strong')
  const lightAccent = await resolveToken(page, '--ac-accent')
  await setTheme(page, 'dark')
  expect(await resolveToken(page, '--ac-stroke-strong')).not.toBe(lightStroke)
  expect(await resolveToken(page, '--ac-accent')).not.toBe(lightAccent)
})
```

The whole-root `input[type="checkbox"]` `toHaveCount(0)` covers both the tight
and the loose list, so the real pipeline confirms the loose-list path emits no
native control (complementing the jsdom loose-list unit test).

#### 3. Test — focused visual-regression screenshot, both themes (write first)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/task-list-visual.spec.ts` (new)
**Changes**: Capture a focused screenshot of **each** task-list `ul` — the tight
list (`nth=0`) and the loose list (`nth=1`, whose label keeps a `<p>` wrapper) —
per theme, using the `applyTheme` / `animations: 'disabled'` conventions from
`library-doc-view.spec.ts`. Use a **tighter `maxDiffPixelRatio` (0.01)** than the
full-page baselines: the box + tick are a small fraction of these label-dominated
crops, so the page-level 0.05 tolerance could absorb a missing tick or
wrong-coloured border. Exact colours are already gated by the resolved-styles
spec; this screenshot's job is shape/parity (incl. the loose-list `<p>`-in-label
alignment the plan flags), so the tighter ratio is appropriate (confirm during
baseline review that the box/tick area genuinely drives the diff).

```ts
import { test, expect } from '@playwright/test'
import { applyTheme } from './helpers'

const LISTS = [
  ['tight', 0],
  ['loose', 1],
] as const

for (const theme of ['light', 'dark'] as const) {
  for (const [kind, nth] of LISTS) {
    test(`task-list ${kind} (${theme})`, async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 })
      await page.goto('/library/plans/first-plan')
      await applyTheme(page, theme)
      await expect(page.getByText('Loading…')).toHaveCount(0)
      await page.evaluate(() => document.fonts.ready.then(() => undefined))
      const list = page.locator('[class*="tasklist"]').nth(nth)
      await expect(list).toHaveScreenshot(`task-list-${kind}-${theme}.png`, {
        maxDiffPixelRatio: 0.01,
        animations: 'disabled',
      })
    })
  }
}
```

#### 4. Capture / regenerate baselines (darwin + linux)

The fixture append makes `/library/plans/first-plan` taller, so the existing
`library-doc-view-{light,dark}-*.png` baselines move and must be regenerated
alongside the new `task-list-{tight,loose}-{light,dark}-*.png` baselines. Both
`-darwin` and `-linux` PNGs must be committed (research §7).

- **Darwin (local)**: from `frontend/`,
  `ACCELERATOR_VISUALISER_BIN=<path> npx playwright test --project visual-regression --update-snapshots`
  (or scope to `task-list-visual library-doc-view`). `mise run
  test:e2e:visualiser` provisions the server binary + env if you need the path.
- **Linux**: dispatch the `Update visual regression baselines` workflow
  (`.github/workflows/update-visual-baselines.yml`, `workflow_dispatch`,
  ubuntu). It pushes with `GITHUB_TOKEN`, which does **not** re-trigger Main CI
  (known gotcha — confirm the linux PNGs landed on the branch).

### Success Criteria:

#### Automated Verification:

- [x] Playwright e2e passes incl. the new `task-list-resolved-styles` (light +
  dark), `task-list-visual` (tight + loose × light + dark) screenshots:
  `mise run test:e2e:visualiser` (451 passed; the lone flaky `kanban-keyboard`
  drag test is unrelated and passed on retry). NOTE: the `library-doc-view`
  baselines did **not** need regenerating — the appended task lists sit below
  the 1440×900 viewport fold, so the viewport screenshots are byte-identical.
- [x] Vitest stays green (no regression from the fixture/spec additions):
  `mise run test:unit:frontend`
- [x] FilterPill specs/snapshots pass unchanged (AC6): covered by the full
  suites above.
- [ ] Both `-darwin` and `-linux` baseline PNGs for `task-list-{tight,loose}-*`
  are committed. (Darwin committed; `library-doc-view-*` unchanged — below the
  fold. **Linux still pending**: dispatch the `Update visual regression
  baselines` workflow.)

#### Manual Verification:

- [ ] The committed `task-list-{tight,loose}-{light,dark}.png` baselines match
  the frozen prototype design (`ui.jsx:221-238`, `app.css:778-793`) within
  tolerance — box, tick, and struck-through done label read correctly in both
  themes; the loose list's label aligns with its box (no stray `<p>` margin).
- [ ] In dark theme the checked state is **distinguishable from the unchecked
  box at the rendered 17px size without zoom** — the white tick on the accent
  fill is the accepted ~2.9:1 deviation from the literal 3:1 AC (matching
  FilterPill), but the state is also carried redundantly by the accent fill
  (vs `--ac-bg-card`) and the struck-through muted label, so it does not rely on
  the tick contrast alone. Record the measured ratio (2.90:1 dark / 5.39:1
  light) in the `#ffffff` ledger deviation note for traceability.
- [ ] On at least one of NVDA / VoiceOver, navigating with the **browse /
  virtual cursor** (the box is intentionally not Tab-focusable), each item —
  tight and loose — is announced as a **read-only checkbox** with its
  checked/unchecked state and the label as its name.

---

## Testing Strategy

### Unit (vitest — `mise run test:unit:frontend`):

- Rewritten `MarkdownRenderer.test.tsx` task-list tests covering all four
  branches the override introduces: **tight** task list (no native input; two
  `role="checkbox"` boxes; correct `aria-checked`/`aria-readonly`; accessible
  name + verbatim label text; tick only when checked), **loose** task list (no
  native input, correct state — the review-1 regression guard), **plain
  non-task** list (no boxes; ordinary `<li>` still produced), and **mixed** list
  (task item still boxed, native input gone).
- New 0095 CSS-as-text guards in `migration.test.ts` asserting the box/label
  rules consume `--ac-stroke-strong`, `--ac-bg-card`, `--ac-accent`,
  `--ac-fg-muted` via single-arg `var()` and `list-style: none`.
- Guardrails kept green by the implementation: `var(--*, fallback)` ban,
  `var()`-resolves-to-declared-token, `EXCEPTIONS` hygiene (exact counts), and
  the `AC5` ratchet (floor bumped to the new observed count).

### Integration / e2e (Playwright real cascade — `mise run test:e2e:visualiser`):

- `task-list-resolved-styles.spec.ts` (scoped to the first/tight list, with
  cardinality guards): AC1 unchecked box (border/fill/width≈1.5px/radius), **AC3
  computed `list-style-type: none`**, AC2 checked box (accent fill/border + white
  tick), **both halves of AC4** (done = muted + line-through; not-done = normal,
  not struck), a whole-root native-input `toHaveCount(0)` covering the loose
  list, and a cross-theme divergence check — all in light + dark.
- `task-list-visual.spec.ts`: focused screenshot of the tight checked+unchecked
  pair at `maxDiffPixelRatio: 0.01`, light + dark, against newly captured
  baselines.
- For fast local iteration on one spec:
  `npm --prefix …/frontend run test:e2e -- task-list-resolved-styles` (needs the
  built server binary + `ACCELERATOR_VISUALISER_BIN`, which
  `mise run test:e2e:visualiser` provisions).

### Key edge cases (all now covered by a test):

- **Loose lists** (blank line between items → `<input>` nested in `<p>`): the
  `input → null` override + recursive `findCheckbox` handle them identically to
  tight lists; covered by a jsdom unit case and the e2e whole-root
  native-input assertion against the loose fixture list.
- **Mixed lists**: the `ul` override uses `items.every(isTaskItem)`, so a list
  mixing task and non-task items keeps default markers while task items still
  render as boxes (rare; acceptable) — pinned by a unit case.
- **Non-task lists** forward through the overrides unchanged → no VR drift on
  pages with ordinary lists (the reason Phase 1 needs no baseline work) — pinned
  by a unit case asserting zero `role="checkbox"` boxes and an ordinary `<li>`.
- **`aria-labelledby` uniqueness**: `useId()` in `TaskListItem` keeps ids unique
  across multiple task items in one document, and is called unconditionally.
- **Fixture `.first()` stability**: the task lists are appended, so existing
  `typography-resolved-sizes`/`inline-code-resolved-styles` `.first()` locators
  keep resolving to their original nodes.

## Performance Considerations

Negligible. Three additional react-markdown component overrides (`input`, `ul`,
`li`) plus a tiny `TaskListItem` component and a small inline SVG per checked
item; `findCheckbox` recurses only over a single `<li>`'s shallow subtree. No
new dependencies, no new stylesheet imports, no token changes.

## Migration Notes

No data or schema migration. Two test/code-coupled surfaces, both in Phase 1:

- **`EXCEPTIONS` ledger** (`migration.test.ts:64-68`): add `1.5px` ×1, `2px` ×2,
  `6px` ×1, `9px` ×1, `17px` ×2, `#ffffff` ×1 for `MarkdownRenderer.module.css`;
  **leave `5px` at 1** (the corner is a `var(--radius-4)` token, not a `5px`
  literal — ADR-0039 bans literal `border-radius`). No reason text may contain
  the word "radius" (the `:824-838` gate). The reverse-hygiene test enforces
  these counts exactly.
- **`AC5_FLOOR`** (`migration.test.ts:432`): bump `981` → the new observed count
  (expected `989`, +8 `var()` refs incl. `var(--radius-4)`) in the same commit,
  per the ratchet bump protocol.

Phase 2 commits new + regenerated baseline PNGs (darwin + linux) but changes no
production code.

### Dependency-coupling posture

The overrides retain two couplings to the markdown stack: the `task-list-item`
class name and the input node's `properties.checked` boolean — both stable,
documented hast outputs of `mdast-util-to-hast`. The previously fragile
`<p>`-unwrapping internal and the `c.type === 'input'` React-element check are
designed out (see Current State / Phase 1 §4). Additionally the `input → null`
override is scoped to the **`disabled` `type="checkbox"`** shape
`mdast-util-to-hast` injects (`list-item.js:46`), so a future legitimate
markdown checkbox is not silently swallowed. `react-markdown` (`^9`) and
`remark-gfm` (`^4`) are caret-ranged with only the lockfile pinning them;
`mdast-util-to-hast` is **transitive** (governed by `react-markdown`'s own range,
not a direct dependency), so pinning it would require a `package.json`
`overrides` entry, not a plain dependency bump. **Posture decision**: rely on the
new tight + loose unit/e2e guards as the loud-failure mechanism on a floated
upgrade rather than pinning — sound only if CI installs are
lockfile-deterministic (`npm ci`), which should be confirmed. If the team later
prefers belt-and-braces, exact-pin via `overrides` in a separate change.

## References

- Work item: `meta/work/0095-markdown-checkboxes-always-dark-mode-styled.md`
- Research: `meta/research/codebase/2026-06-09-0095-markdown-checkboxes-always-dark-mode-styled.md`
- Sibling precedent (verification template):
  `meta/plans/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
- Renderer + override map: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:29-44`
- Module CSS (where task rules go): `…/MarkdownRenderer/MarkdownRenderer.module.css`
- Unit test to rewrite: `…/MarkdownRenderer/MarkdownRenderer.test.tsx:114-123`
- Upstream input source: `…/frontend/node_modules/mdast-util-to-hast/lib/handlers/list-item.js:19-88`
- FilterPill faux-checkbox: `…/FilterPill/FilterPill.tsx:138-149`, `…/FilterPill/FilterPill.module.css:185-208`
- CheckIcon model: `…/SortPill/SortPill.tsx:119-135`
- Tokens + `color-scheme` + theme mirrors: `…/frontend/src/styles/global.css:85,90-95,333,341-406,412-473`
- Migration machinery: `…/frontend/src/styles/migration.test.ts:30-37,64-68,346-353,386-423,432-448,478-499,520-544`
- VR harness: `…/playwright.config.ts`, `…/tests/visual-regression/helpers.ts:14-20`, `…/tests/visual-regression/lib/expected-colours.ts:79-100`, `…/tests/visual-regression/library-doc-view.spec.ts`, `…/tests/visual-regression/inline-code-resolved-styles.spec.ts`
- Fixture: `…/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`
- Linux baseline workflow: `.github/workflows/update-visual-baselines.yml`
- Design (frozen prototype): `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/ui.jsx:221-238`, `…/src/app.css:778-793`
- ADRs: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`, `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
