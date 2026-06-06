---
type: codebase-research
id: "2026-06-06-0086-kanban-drag-and-drop"
title: "Research: Kanban Drag-and-Drop with Toast Confirmations (work item 0086)"
date: "2026-06-06T13:06:58+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0086"
parent: "work-item:0086"
relates_to: ["codebase-research:2026-05-31-0040-pipeline-visualisation-overhaul", "codebase-research:2026-05-27-0039-toaster-and-external-edit-notifications"]
topic: "Kanban Drag-and-Drop with Toast Confirmations"
tags: [research, codebase, kanban, drag-and-drop, dnd-kit, toaster, accessibility, frontend]
revision: "a3fa7a527033f198a9239b2c6c23a9550ba7f269"
repository: "build-system"
last_updated: "2026-06-06T13:06:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Kanban Drag-and-Drop with Toast Confirmations (work item 0086)

**Date**: 2026-06-06T13:06:58+00:00
**Author**: Toby Clemson
**Git Commit**: a3fa7a527033f198a9239b2c6c23a9550ba7f269
**Branch**: detached working copy (no named bookmark; `main` at a7f1fc14)
**Repository**: build-system

## Research Question

Research the codebase to support work item `0086-kanban-drag-and-drop.md` — a
quality pass on the already-shipped kanban drag-and-drop. Establish exactly how
the live implementation behaves across the three requirement clusters so a plan
can be written: **A** (drag-interaction fixes against the prototype design,
A1–A3), **B** (toast-confirmation loop, B1–B3), and **C** (keyboard
accessibility verify-and-harden, C1–C3). Also surface the prototype's
authoritative interaction values and resolve the open dnd-kit-vs-HTML5 library
decision.

## Summary

The feature is shipped and mature: dnd-kit drives the board, a robust
ETag-guarded `PATCH /api/docs/{path}/frontmatter` endpoint writes the status
back, columns are config-driven (ADR-0024), and the Toaster exists. The work is
genuinely a defect-and-polish pass, not a build. Key findings:

- **A1 + A3 share one root cause**: there is **no `DragOverlay`** in the board.
  dnd-kit therefore translates the *actual* card DOM node under the pointer, so
  the source card visibly leaves its slot (the A3 "disappears" symptom) and no
  cursor-following clone is rendered (the A1 "no affordance" symptom). `isDragging`
  is never destructured from `useSortable`, and the card CSS has **no** opacity /
  rotation / lift styling. **Adding a `DragOverlay` plus `isDragging` styling
  fixes both A1 and A3 together.** Note: there is **no `onDragOver` handler and no
  mid-drag state mutation** — cross-column membership is computed only in
  `onDragEnd`, so the "disappears" effect is the transform-translate of the real
  node, not an optimistic re-group.
- **A2 (drag must not navigate)** is weakly guarded: the card is a TanStack
  Router `<Link>` carrying the drag listeners directly, and the *only* separation
  between click and drag is the `PointerSensor`'s 5px distance constraint. A
  sub-threshold press-release still fires the anchor's navigation. No click
  suppression / drag-flag exists.
- **B (toast loop)**: the Toaster is **info-only** — no `kind`/`variant` field
  anywhere in the model, API, or CSS, and a regression test (`Toaster.test.tsx:168-175`)
  actively locks the single-accent styling. The board today shows an **inline
  `role="alert"` conflict banner** (`KanbanBoard.tsx:177-189`), *not* a toast, and
  **emits no success toast at all**. The semantic colour tokens (`--ac-ok`,
  `--ac-err`) already exist and are contrast-tested, so variants are an additive
  change touching `use-toast.ts`, `Toaster.tsx`, `Toaster.module.css` (+ that one
  locked test). Only **one** production `showToast` caller exists
  (`use-external-edit-toast.ts`), so an optional `kind` field is backward-compatible.
- **C (keyboard a11y)**: a `KeyboardSensor` with `sortableKeyboardCoordinates`
  is already wired and `announcements.ts` produces the four lifecycle strings, so
  C1/C2 are largely *verify* tasks. **Focus management (C3) is partial**: focus is
  restored only on the move-error path (`KanbanBoard.tsx:115-122`), and even then
  to the `<li>` wrapper rather than the focusable inner `<Link>`; there is **no**
  focus restoration on success.
- **Library decision (Open Question)**: the decision rule favours **retaining
  dnd-kit** — it already provides keyboard a11y *and* rollback, which raw HTML5 DnD
  (the prototype's mechanism) does not. The prototype is a non-persistent
  simulation with no keyboard a11y and no rollback; re-implementing those by hand
  would mean owning a custom DnD engine. Expected outcome: **dnd-kit-retention**.

## Detailed Findings

### A. Drag-interaction surface (`routes/kanban/`)

The board is a single `DndContext` in `KanbanBoard.tsx`; cards are `useSortable`
items inside per-column `SortableContext` + `useDroppable` wrappers.

- **Sensors / activation** (`KanbanBoard.tsx:49-52`): `PointerSensor` with
  `activationConstraint: { distance: 5 }` and `KeyboardSensor` with
  `coordinateGetter: sortableKeyboardCoordinates`. No `delay`/`tolerance`.
- **No `DragOverlay`** anywhere (grep returns nothing). No `onDragOver` prop on
  the `DndContext`. Only `onDragStart` (`:172`) and `onDragEnd` (`:173`) are wired.
- **WorkItemCard** (`WorkItemCard.tsx:16-44`): `useSortable({ id: entry.relPath })`
  destructures only `{ attributes, listeners, setNodeRef, transform, transition }`
  — **`isDragging` is never pulled out or used**. Inline style applies only
  `CSS.Transform.toString(transform)` + `transition` (`:39-42`). The element is a
  router `<Link to="/library/$type/$fileSlug" params={{ type: 'work-items', fileSlug }}>`,
  and `setNodeRef` + sortable `attributes` + `listeners` are *all spread onto the
  same `<Link>`* (`:35,43-44`). `role` is stripped from attributes (`:25`) so the
  anchor keeps `role="link"`.
- **WorkItemCard.module.css** (36 lines, read fully): **no** opacity, rotation,
  shadow, or any drag-state rule. The only drag feedback is the target column's
  `.columnOver` class toggled by `isOver` (`KanbanColumn.tsx:21,29`).
- **resolve-drop-outcome.ts** (`:11-42`): `resolveDropOutcome(active, over,
  entriesByRelPath, validColumnKeys)` → union `'move' | 'no-op-same-column' |
  'no-op-other-rejected' | 'no-op-unknown'`. Same-column drop → `no-op-same-column`
  (the requirement-A5 no-op is already correctly handled). Dropping on the
  read-only "Other" column → `no-op-other-rejected`.
- **Drop handling** (`KanbanBoard.tsx:104-139`): `move` → `move.mutate(...)`;
  `no-op-*` branches announce or silently return. Membership only changes here, at
  drop — confirming **no defer-to-drop bug in the data model**; the visible A3
  symptom is purely the transform on the real (non-overlaid) node.

**A1/A2/A3 implications**:
- A1 — add a `DragOverlay` rendering a clone that follows the cursor + apply
  `isDragging` styling (opacity on source/clone, rotation on source) sourced from
  the prototype values below.
- A2 — the 5px constraint is the only guard; needs either a larger constraint, an
  explicit drag-flag that suppresses the `<Link>` click, or separating the drag
  handle from the navigation target.
- A3 — rendering the dragged node into a `DragOverlay` keeps the source slot
  occupied, removing the "disappears" effect. No data change needed.

### Announcements (`announcements.ts`) — verbatim strings

`buildKanbanAnnouncements` (`:31-50`), wired at `KanbanBoard.tsx:174` via
`accessibility={{ announcements }}`. `describe(id)` → e.g. `work item 0042: Title`;
`labelFor(id)` resolves the column label. C2 must assert these exact strings:

- **Pick up** (`:35-37`): `Picked up ${describe(active.id)}.`
- **Drag-over / column change** (`:38-41`): `${describe(active.id)} is over ${labelFor(over.id)}.`
  (returns `undefined` when no `over`).
- **Drop** (`:42-45`): `Moved ${describe(active.id)} to ${labelFor(over.id)}.`
  (or `Drop of ${describe(active.id)} cancelled, no target.` when no over).
- **Cancel** (`:46-48`): `Drag of ${describe(active.id)} cancelled.`

A **separate** `role="status" aria-live="polite"` region
(`KanbanBoard.tsx:190-192`) is driven by `showAnnouncement` for no-op outcomes
only ("Card returned to {label}.", "The Other column is read-only; drops are
ignored.").

### B. Write-back path and toast loop

Chain: `handleDragEnd` (`KanbanBoard.tsx:104`) → `useMoveWorkItem`
(`use-move-work-item.ts:15`) → `patchWorkItemFrontmatter` (`fetch.ts:32`) →
`PATCH /api/docs/{path}/frontmatter` (`docs.rs:127`) → `patch_status`
(`patcher.rs:31`).

- **`useMoveWorkItem`** (`use-move-work-item.ts:15-48`): optimistic
  `onMutate` snapshot+rewrite (`:23-34`), `onError` rollback restores
  `ctx.previous` (`:40-42`), `onSuccess` only `registry.register(result.etag)` to
  suppress the self-caused SSE echo (`:36-38`), `onSettled` invalidates (`:44-46`).
  **The hook surfaces no UI** — success/error messaging is attached by the caller.
- **`patchWorkItemFrontmatter`** (`fetch.ts:32-57`): `PATCH` with `If-Match: "{etag}"`,
  body `{ "patch": { "status": "<toStatus>" } }`. `412` → throws `ConflictError`
  carrying `currentEtag` (`:46-49`); `204` → reads fresh `ETag` header, strips
  quotes, returns `{ etag }` (`:53-56`).
- **Server** (`docs.rs:127-289`): work-item-only guard (`:163-166`), status
  validated against `state.kanban_columns` (`:170-176`), `If-Match` required (missing
  → **428**, `:179-202`), ETag mismatch → **412** with `ETag` header + `{currentEtag}`
  body (`mod.rs:140-152`), success → **204** with fresh `ETag` header (`:281-289`).
  `patcher.rs` does a byte-level YAML edit of only the top-level `status:` line,
  preserving everything else; fresh ETag is `sha256-<hex>` of new bytes.
- **Current conflict UI** (the banner B2 removes): `KanbanBoard.tsx:177-189`
  renders `<div role="alert" className={styles.conflictBanner}>` with a dismiss
  button, set by `showConflict(...)` from the per-mutation `onError` (`:115-122`,
  with a 30s auto-dismiss and a `requestAnimationFrame` refocus). `conflictMessageFor`
  (`:32-40`) picks copy by error type. **There is no success toast today.** (A
  distinct `role="alert"` at `:149-160` is the board *load-failure* state — not the
  move conflict; leave it.)

**Toaster** (`Toaster.tsx`, `use-toast.ts`, `Toaster.module.css`):
- API (`use-toast.ts:9-12`): `ShowToastInput { heading, message }`; stored `Toast
  { id, heading, message }` (`:3-7`). **No `kind`/`variant`/`type` field.**
- Owner/consumer split: `useToastDispatcher()` owns state (call once — at
  `RootLayout.tsx:49`, provided at `:84`, Toaster mounted `:104-105`); leaf
  components read `useToast()`. Auto-dismiss `5_000ms` (`:22`), pause on
  hover/focus, cap 5 (`MAX_TOASTS`), portal to `document.body`.
- Rendering (`Toaster.tsx:44-96`): one hard-coded info `<svg>` icon (`:53-70`),
  `role="status" aria-live="polite"` (`:41-42`), backtick→`<code>` message
  rendering (`renderMessage`, `:10-21`).
- Styling: colour centralised in exactly two declarations —`.toast` `border-left`
  (`Toaster.module.css:26`) and `.icon` `color` (`:36`), both `var(--ac-accent)`.
  **`Toaster.test.tsx:168-175` locks this** (asserts `--ac-accent`, *not* `--ac-ok`/
  `--ac-warn`) and must be updated when variants land.
- Tokens already exist (`styles/global.css`): `--ac-ok #2e8b57` (`:99`), `--ac-warn
  #d98f2e` (`:100`), `--ac-err var(--atomic-red)` (`:101`), with dark values
  (`:361-363`) and contrast tests (`contrast.test.ts:159-167`).
- **Backward-compat**: the only production caller is `use-external-edit-toast.ts:45-48`.
  An *optional* `kind?: 'ok'|'error'|'info'` (default `info`) is fully additive;
  existing `toHaveBeenCalledWith({ heading, message })` assertions stay green.

**Suggested additive variant approach** (from analysis): add optional `kind` to
`ShowToastInput`/`Toast`, default `'info'` in `showToast`, set `data-kind={t.kind}`
on the `.toast` div, introduce a `--toast-accent: var(--ac-accent)` indirection so
`info` stays byte-identical, and override `.toast[data-kind='ok'/'error']`.

### C. Keyboard accessibility

- **C1** — `KeyboardSensor` + `sortableKeyboardCoordinates` already wired
  (`KanbanBoard.tsx:51`); `verticalListSortingStrategy` per column
  (`KanbanColumn.tsx:42`) + `closestCorners` (`:171`). Default dnd-kit keys
  (Space/Enter pick-up/drop, arrows move, Escape cancel) should already complete a
  cross-column move — **verify** via test; fix only if a defect surfaces.
- **C2** — assert the four `announcements.ts` strings above fire on pick-up,
  column change, drop, cancel.
- **C3** — focus restoration is **only** on `move.mutate` `onError`
  (`KanbanBoard.tsx:115-122`) and targets the `<li data-relpath=...>` wrapper —
  but the focusable element is the inner `<Link>`, so this may not actually land
  focus on an interactive node. **No focus restoration on success** (`onSuccess`
  only clears the conflict, `:123`) and none on no-op/cancel. C3 needs success-path
  focus return *and* correcting the focus target to the card's anchor.

### Prototype interaction values (`view-kanban.jsx` + `app.css`)

The prototype uses **raw HTML5 DnD** (`draggable`, `onDragStart`, `onDragOver`+
`preventDefault`, `onDrop`) — no library. Authoritative values:

- **Dragging source-card style** (`app.css:965`): `.ac-kcard.is-dragging {
  transform: rotate(1.5deg) scale(1.02); box-shadow: var(--ac-shadow-lift);
  border-color: var(--ac-accent); }`. Base transition `140ms ease` (`:962`).
- **Translucent clone**: **no explicit opacity** — the prototype never calls
  `setDragImage`, so the cursor-following clone is the *browser-native default drag
  image* at default translucency. **The prototype does not define a clone opacity
  value.** (A1's visual-regression baseline will need a chosen value; the prototype
  implies but does not specify it.)
- **Defer-to-drop**: card moves only in `onDrop` via a `setItems` status remap
  (`view-kanban.jsx:37-48`); `onDragOver` only `preventDefault`s.
- **Same-column drop**: explicit no-op (`view-kanban.jsx:39-40`).
- **Empty column**: static placeholder — title "Nothing here", body "Drop a work
  item to set its status to {col.key}." (`view-kanban.jsx:84-89`), dashed-border
  `.ac-empty` panel (`app.css:1352-1361`). Not a hover-only highlight.
- **Cursor**: `cursor: grab` at rest (`app.css:961`); **no `grabbing` rule** —
  drag uses the native cursor.
- **No card drop-settle animation** — only the toast animates.
- **Success toast** (`view-kanban.jsx:42-46`): `pushToast({ kind: "ok", title:
  `${dragging.id} moved to ${colKey}`, body: <>PATCH <code>/api/docs/work/{id}.md/frontmatter</code>
  → <code>204</code> · fresh ETag received</> })`. Note the prototype uses the **raw
  column key** (`in-progress`), not the display label. Work item B1 asks for the
  card + target column named with a technical body line; the live board should
  prefer the human column **label** (live board is label-driven per ADR-0024).

### Tests & visual-regression baselines

- Unit (vitest, co-located): `KanbanBoard.test.tsx`, `KanbanColumn.test.tsx`,
  `WorkItemCard.test.tsx`, `resolve-drop-outcome.test.ts`, `announcements.test.ts`,
  `use-move-work-item.test.tsx`, `Toaster.test.tsx`, `use-toast.test.tsx`.
- E2E (`frontend/e2e/`): `kanban.spec.ts` (has a `dndDrag()` pointer-simulation
  helper firing the 5px sensor), `kanban-columns.spec.ts`, `kanban-conflict.spec.ts`,
  custom-column specs. **No** dedicated keyboard-move or toast E2E; those live in
  unit suites. No screenshot snapshots in `e2e/`.
- Visual regression: baselines under
  `frontend/tests/visual-regression/__screenshots__/<spec>.spec.ts-snapshots/`,
  named `<name>-<theme>-visual-regression-<platform>.png`. Existing kanban
  baselines are the four `kanban-{dark,light}-...-{darwin,linux}.png` from
  `tokens.spec.ts` (full-route token captures). **No `drag-`/dragging-state
  baseline exists** — A1's oracle baseline is net-new. `playwright.config.ts`
  defines a `visual-regression` project that the e2e project `dependencies` on.
  Regen locally `--update-snapshots` (darwin) / via the "Update visual regression
  baselines" `workflow_dispatch` (linux).
- Run via mise: `mise run test:unit:frontend`, `mise run test:e2e:visualiser`.

## Code References

- `frontend/src/routes/kanban/KanbanBoard.tsx:49-52` — sensors + 5px constraint
- `frontend/src/routes/kanban/KanbanBoard.tsx:104-139` — `handleDragEnd`, drop resolution, mutate
- `frontend/src/routes/kanban/KanbanBoard.tsx:115-122` — error-path focus restore (to `<li>`)
- `frontend/src/routes/kanban/KanbanBoard.tsx:177-189` — inline conflict banner (B2 removes)
- `frontend/src/routes/kanban/KanbanBoard.tsx:190-192` — no-op `aria-live` region
- `frontend/src/routes/kanban/WorkItemCard.tsx:16-44` — `useSortable`, no `isDragging`, `<Link>` carries listeners
- `frontend/src/routes/kanban/WorkItemCard.module.css` — no drag-state styling (36 lines)
- `frontend/src/routes/kanban/KanbanColumn.tsx:21-42` — `useDroppable`, `.columnOver`, sortable strategy
- `frontend/src/routes/kanban/resolve-drop-outcome.ts:11-42` — drop outcome union (same-column no-op present)
- `frontend/src/routes/kanban/announcements.ts:31-50` — the four lifecycle strings
- `frontend/src/api/use-move-work-item.ts:15-48` — optimistic update + rollback (no UI)
- `frontend/src/api/fetch.ts:32-57` — `patchWorkItemFrontmatter`, If-Match, 412/204
- `frontend/src/api/use-toast.ts:3-22` — toast model/API (no `kind`), 5s dismiss, cap 5
- `frontend/src/components/Toaster/Toaster.tsx:44-96` — single info rendering
- `frontend/src/components/Toaster/Toaster.module.css:26,36` — the two accent declarations
- `frontend/src/components/Toaster/Toaster.test.tsx:168-175` — single-accent CSS lock
- `frontend/src/api/use-external-edit-toast.ts:45-48` — only production `showToast` caller
- `frontend/src/styles/global.css:99-101,361-363` — `--ac-ok`/`--ac-warn`/`--ac-err` tokens
- `frontend/src/components/RootLayout/RootLayout.tsx:49,84,104-105` — Toaster owner/mount
- `server/src/api/docs.rs:127-289` — PATCH frontmatter endpoint (work-item guard, ETag, 204/412/428)
- `server/src/patcher.rs:31-216` — byte-level `status:` YAML edit
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-kanban.jsx:37-48,82` — prototype drop logic + dragging class
- `.../prototype-full/src/app.css:961-965,1352-1361` — `grab` cursor, `is-dragging` transform, empty panel

## Architecture Insights

- **One missing `DragOverlay` explains two of three A-defects.** The cheapest
  high-leverage change is introducing a `DragOverlay` + `isDragging` styling; it
  delivers A1 and removes the A3 symptom without touching the data flow.
- **The data model already defers to drop.** There is no `onDragOver` mutation,
  so A3 is a *rendering* artefact, not a state bug — important framing for the plan
  and for not over-engineering.
- **A2 is an activation/handle problem.** Because drag listeners and navigation
  live on the same `<Link>`, the clean fixes are (a) a larger activation constraint
  / delay, (b) a drag-in-progress flag that swallows the click, or (c) splitting the
  drag handle from the navigation affordance. dnd-kit favours (a)/(b).
- **Toast variants are deliberately additive.** Tokens, single caller, and a
  property-indirection CSS approach keep `info` byte-identical and the change
  low-risk; the only friction is one intentionally strict CSS-lock test.
- **Keyboard a11y is mostly present.** C is weighted toward *recorded
  verification*; the real net-new work is C3 success-path focus return and fixing
  the focus target from the `<li>` to the card anchor.
- **Library decision resolves to dnd-kit-retention** under the work item's own
  rule: the live app already has the two things raw HTML5 DnD lacks (keyboard a11y,
  rollback). No ADR exists for this; consider recording one.

## Historical Context

- `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` — configurable
  columns via `GET /api/kanban/config` / `useKanbanConfig()`; out of scope to change.
- `meta/plans/2026-04-26-meta-visualiser-phase-8-kanban-write-path.md` — origin of
  the status PATCH endpoint, ETag/optimistic concurrency (no standalone ADR for it).
- `meta/plans/2026-04-26-meta-visualiser-phase-7-kanban-read-only.md` — read-only
  board + column consumption.
- `meta/research/codebase/2026-05-27-0039-toaster-and-external-edit-notifications.md`
  and `meta/plans/2026-05-27-0039-...md` — the Toaster (0039) 0086 extends.
- `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md` and
  its plan — the 0040 surface that shares `WorkItemCard.tsx`; the 0040 plan is the
  only doc that cross-references 0086.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the gap analysis cited by 0086's References.
- `meta/work/0044-spike-list-screen-scope-decisions.md` — the abandoned column-set
  spike (no longer a dependency).
- No ADR exists for the dnd-kit-vs-HTML5 choice; 0086 carries it as an open decision.

## Related Research

- `meta/research/codebase/2026-05-31-0040-pipeline-visualisation-overhaul.md`
- `meta/research/codebase/2026-05-27-0039-toaster-and-external-edit-notifications.md`
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`
- `meta/reviews/work/0086-kanban-drag-and-drop-review-1.md` (work-item review, staged)

## Open Questions

1. **A1 clone opacity value** — the prototype does *not* define one (relies on the
   native default drag image). A concrete opacity/rotation must be chosen for the
   `DragOverlay` clone and source card, then frozen into the net-new
   drag-state visual-regression baseline. The source-card transform is specified:
   `rotate(1.5deg) scale(1.02)` + `--ac-shadow-lift` + accent border.
2. **Success-toast column naming** — prototype uses the raw column key
   (`in-progress`); the live board is label-driven (ADR-0024). Confirm B1 should
   name the human **label** (recommended) rather than the key.
3. **A2 fix strategy** — activation-constraint tuning vs. explicit drag-flag click
   suppression vs. splitting the drag handle from the `<Link>`. Each has different
   keyboard/focus implications for C3.
4. **C3 focus target** — current error-path restore focuses the `<li>` wrapper,
   not the focusable `<Link>`; confirm the intended focus target is the card anchor.
5. **Library decision ADR** — should the dnd-kit-retention outcome be recorded as
   an ADR (none exists today)?
