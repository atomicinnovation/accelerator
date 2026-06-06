---
type: plan
id: "2026-06-06-0086-kanban-drag-and-drop"
title: "Kanban Drag-and-Drop with Toast Confirmations Implementation Plan"
date: "2026-06-06T13:44:13+00:00"
author: "Toby Clemson"
producer: create-plan
status: accepted
work_item_id: "work-item:0086"
parent: "work-item:0086"
derived_from: ["codebase-research:2026-06-06-0086-kanban-drag-and-drop"]
relates_to: ["work-item:0040", "work-item:0039"]
tags: [design, frontend, kanban, drag-and-drop, dnd-kit, toaster, accessibility]
revision: "1142b697961a6827cc98770834ed9e1e0ea933b4"
repository: "build-system"
last_updated: "2026-06-06T19:15:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Kanban Drag-and-Drop with Toast Confirmations Implementation Plan

## Overview

A defect-and-polish pass on the already-shipped kanban drag-and-drop. The board
already drives drag-and-drop via dnd-kit, writes status back through an
ETag-guarded `PATCH /api/docs/{path}/frontmatter` endpoint, and renders
config-driven columns (ADR-0024). The Toaster exists but is info-only and is not
wired into the board. This plan: (A) fixes the drag interaction against the
prototype design, (B) completes the toast-confirmation loop, and (C) verifies
and hardens keyboard accessibility — converging the live board onto the
prototype's interaction (`view-kanban.jsx`) without rebuilding any of the
shipped machinery.

## Current State Analysis

The feature is mature; the gaps are localised:

- **No `DragOverlay`.** `KanbanBoard.tsx` wires only `onDragStart`/`onDragEnd`
  on the `DndContext` (`KanbanBoard.tsx:169-174`), with no overlay. dnd-kit
  therefore CSS-transform-translates the *real* card node under the pointer.
  `WorkItemCard` destructures only `{ attributes, listeners, setNodeRef,
  transform, transition }` — **`isDragging` is never read** (`WorkItemCard.tsx:17-19`)
  — and `WorkItemCard.module.css` has **no** opacity/rotation/lift styling. This
  single omission produces both the A1 symptom (no cursor-following affordance)
  and the A3 symptom (source card visibly leaves its slot).
- **Membership already defers to drop.** There is no `onDragOver` handler and no
  mid-drag state mutation; the cross-column move is computed only in
  `handleDragEnd` → `resolveDropOutcome` → `move.mutate` (`KanbanBoard.tsx:104-139`).
  So A3 is a *rendering* artefact (transform of the non-overlaid node), **not** a
  data-model bug. No state change is needed for A3.
- **A2 is weakly guarded.** The card is a TanStack Router `<Link>` and the
  sortable `listeners` are spread onto that same `<Link>` (`WorkItemCard.tsx:34-45`).
  The only separation between a click and a drag is the `PointerSensor`'s
  `{ distance: 5 }` constraint (`KanbanBoard.tsx:50`). A sub-threshold
  press-release still fires the anchor's navigation. There is no drag-flag /
  click suppression.
- **Toaster is info-only.** `use-toast.ts` has no `kind`/`variant` field; the
  single info icon and the two `var(--ac-accent)` colour declarations
  (`Toaster.module.css:26,36`) are hard-coded, and `Toaster.test.tsx:168-175`
  actively *locks* the single-accent styling. The board emits **no success
  toast** and shows an **inline `role="alert"` conflict banner** for failures
  (`KanbanBoard.tsx:177-189`), set from the per-mutation `onError`
  (`KanbanBoard.tsx:115-122`). The only production `showToast` caller is
  `use-external-edit-toast.ts:45-48`.
- **Keyboard a11y is mostly present.** `KeyboardSensor` +
  `sortableKeyboardCoordinates` (`KanbanBoard.tsx:51`) and the four
  `announcements.ts` lifecycle strings are wired. **Focus restoration is partial**:
  only on the error path, and it targets the `<li data-relpath=…>` wrapper
  (`KanbanBoard.tsx:117-121`) rather than the focusable inner `<Link>`. There is
  **no** focus restoration on success.
- **Tokens exist.** `--ac-ok #2e8b57`, `--ac-warn #d98f2e`,
  `--ac-err var(--atomic-red)` are defined with dark values and contrast tests
  (`global.css:99-101,361-363`), so toast variants are an additive,
  already-themed change.

### Key Discoveries:

- One missing `DragOverlay` explains two of three A-defects — `KanbanBoard.tsx:169-174`.
- The data model already defers to drop (no `onDragOver` mutation) —
  `KanbanBoard.tsx:104-139`, `resolve-drop-outcome.ts:11-42`.
- A2 is an activation/handle problem: listeners + navigation share one `<Link>` —
  `WorkItemCard.tsx:34-45`.
- Toast variants are additive: optional `kind`, one caller, tokens present, one
  intentionally-strict CSS-lock test to update — `use-toast.ts:9-12`,
  `Toaster.module.css:26,36`, `Toaster.test.tsx:168-175`,
  `use-external-edit-toast.ts:45-48`.
- The four announcement strings C2 must assert verbatim — `announcements.ts:35-48`.
- Focus restore exists only on error and targets the wrong node —
  `KanbanBoard.tsx:115-122`.
- Prototype dragging style (source card): `rotate(1.5deg) scale(1.02)` +
  `var(--ac-shadow-lift)` + accent border; `cursor: grab` at rest, no `grabbing`
  rule; empty column = static "Nothing here" / "Drop a work item to set its
  status to {key}." panel — `app.css:961-965,1352-1361`, `view-kanban.jsx:84-89`.

### Resolved decisions (carried from research Open Questions):

1. **A2 fix** → **drag-in-progress flag** that suppresses the `<Link>` click;
   genuine clicks still navigate.
2. **A1 clone opacity** → **0.6** for the `DragOverlay` clone; source card keeps
   the prototype transform (`rotate(1.5deg) scale(1.02)` + `--ac-shadow-lift` +
   accent border). This pair is frozen into the net-new drag-state
   visual-regression baseline (A1's oracle).
3. **Success-toast column naming** → **human label** (ADR-0024 board is
   label-driven), not the raw key.
4. **C3 focus target** → the card's `<Link>` anchor (the only focusable node), in
   its final resting column (target on success, source on revert).
5. **Library decision** → **retain dnd-kit** (it already provides the keyboard
   a11y + rollback raw HTML5 DnD lacks). Recorded in this plan's prose only — **no
   ADR** for this story.

## Desired End State

- Dragging a card shows a 0.6-opacity clone following the cursor while the source
  card stays in its slot with the prototype's rotated/lifted styling; a
  visual-regression baseline locks this.
- A drag never navigates; a genuine click always navigates.
- A successful move (`204`) raises a **success** toast naming the card and target
  **label** in plain user-facing copy; a failed/`412` move raises an **error**
  toast (announced assertively and persistent) and reverts the card; the inline
  conflict banner is gone.
- The Toaster supports `info` (unchanged, byte-identical), `ok`, and `error`
  variants; `error` toasts are routed to an assertive (`role="alert"`) live region
  and do not auto-dismiss, while `info`/`ok` stay polite and auto-dismiss.
- A keyboard-only user can pick up, move across columns, and drop a card;
  announcements fire with the exact `announcements.ts` strings; focus returns to
  the card anchor in its final resting column on both success and revert.
- All behaviours hold for both a 3-column and a 5-column (≥30-char wrapping
  label) live config.
- A recorded side-by-side comparison gives every section-A interaction aspect a
  parity / fixed / follow-up verdict, with A1–A3 at least *fixed*.

## What We're NOT Doing

- Not rebuilding drag-and-drop, the PATCH write-back path, optimistic
  update/rollback, or column configuration (ADR-0024).
- Not moving off dnd-kit to raw HTML5 DnD.
- Not adding card reordering within a column (cross-column status moves only).
- Not mutating frontmatter fields other than `status`.
- Not changing the column set or labels (config-driven, out of scope).
- Not touching the board *load-failure* `role="alert"` (`KanbanBoard.tsx:149-160`)
  — only the move *conflict* banner is removed.
- Not writing an ADR for the library decision (recorded in prose only).

## Implementation Approach

Six phases, each independently mergeable and each leaving the board shippable.
Test-driven where a behavioural oracle exists (the visual-regression baseline is
A1's oracle; unit/E2E tests are written before the implementation they assert).
Sequenced so dependencies flow forward: Toaster variants first (pure additive
foundation), then the board toast loop that consumes them, then the
drag-rendering fixes, the click/drag separation, the keyboard hardening, and
finally the recorded convergence + cross-config verification that spans all of
A/B/C.

All frontend work lives under
`skills/visualisation/visualise/frontend/`. Run unit tests with
`mise run test:unit:frontend`, E2E/visual-regression with
`mise run test:e2e:visualiser`, and type-checking with `mise run typecheck`
(there is no separate lint task).

---

## Phase 1: Toaster success / error variants (B3)

### Overview

Make the Toaster variant-aware with an optional `kind` defaulting to `info`, so
existing callers are byte-identical and the board can raise `ok`/`error` toasts
in Phase 2. Additive only.

### Changes Required:

#### 1. Toast model + dispatcher

**File**: `frontend/src/api/use-toast.ts`
**Changes**: Add an optional `kind` to `ShowToastInput` and `Toast`; default it
to `'info'` in `showToast`; thread it into the stored toast.

```ts
export type ToastKind = 'info' | 'ok' | 'error'

export interface Toast {
  id: number
  heading: string
  message: string
  kind: ToastKind
}

export interface ShowToastInput {
  heading: string
  message: string
  kind?: ToastKind
}

// in showToast:
const showToast = useCallback(
  ({ heading, message, kind = 'info' }: ShowToastInput): number => {
    const id = nextIdRef.current++
    // Eviction policy: error toasts persist and are EXEMPT from the cap; only
    // auto-dismissing kinds (info/ok) are capped at MAX_TOASTS. This prevents a
    // burst of later toasts silently evicting a persistent error the user has
    // not acknowledged.
    setToasts((prev) => {
      const next = [...prev, { id, heading, message, kind }]
      const errors = next.filter((t) => t.kind === 'error')
      const capped = next.filter((t) => t.kind !== 'error').slice(-MAX_TOASTS)
      // re-merge preserving insertion order:
      return next.filter((t) => errors.includes(t) || capped.includes(t))
    })
    // kind-aware auto-dismiss: info/ok auto-dismiss after the existing
    // TOAST_AUTO_DISMISS_MS; error toasts persist (no auto-dismiss).
    if (kind !== 'error') arm(id)
    return id
  },
  [arm],
)
```

(The exact re-merge above is illustrative — the requirement is: **error toasts
never auto-dismiss and are never evicted by the `MAX_TOASTS` cap; the cap applies
only to `info`/`ok`.** Implement it however reads cleanest and assert both halves
in tests.)

**Dismissal (resolved, not conditional).** The shipped Toaster already exposes
the keyboard-accessible dismissal paths persistent errors need — a per-toast close
button (`Toaster.tsx:75-95`, `aria-label="Dismiss notification"`) and
Escape-dismisses-topmost (`Toaster.tsx:28-34`). No new dismiss control is needed;
state this as the dismissal contract, and add a test that an `error` toast is
dismissable via the close button and via Escape.

**Escape collision with drag-cancel.** The Toaster's Escape handler is a
document-level `keydown` listener with no `stopPropagation`, and dnd-kit also
cancels an active drag on Escape. Because errors now persist (the overlap window is
no longer the old auto-dismiss blip), a single Escape during a keyboard drag would
both cancel the drag **and** dismiss a lingering error toast. Gate the Toaster's
Escape-dismiss on **no active drag** (read the board's drag state, or skip
dismissal while a drag is in progress) so the two Escape semantics don't collide on
one keypress; add a test covering Escape-during-drag.

**Close-glyph contrast.** The close control uses `--ac-fg-faint`, which is the one
relevant token **not** covered by the existing `contrast.test.ts` suite — and it is
now the load-bearing dismissal affordance for persistent errors. Either extend
`contrast.test.ts` to assert `--ac-fg-faint` against `--ac-bg-card` at the WCAG
1.4.11 3:1 threshold in both themes, or state explicitly that the focus-visible
outline (`--ac-accent`, already 3:1-tested) is the conformance-bearing affordance
and the resting glyph is supplementary.

#### 2. Toaster rendering

**File**: `frontend/src/components/Toaster/Toaster.tsx`
**Changes**: Set `data-kind={t.kind}` on the `.toast` div for CSS targeting, and
swap the icon per kind (info circle-i; ok check; error alert-triangle/x). Add a
visually-hidden (sr-only) severity prefix keyed off `kind` ("Error: " / "Success:
") so the variant is perceivable to assistive tech and colour-blind users
independent of the colour/icon (the icon stays `aria-hidden`). Render the prefix as
the **first announced child** of the toast (preceding the heading in DOM order) so
it survives the empty-message omission on heading-only success toasts — the
announcement reads "Success: 0086 moved to In progress". Assert the announced
region's text begins with the severity prefix for the heading-only case.

**sr-only utility (new).** No `sr-only`/visually-hidden helper exists in the
frontend today (every decorative-vs-announced distinction currently uses
`aria-hidden` on visible nodes). Add a shared `.srOnly` utility using the standard
clip-rect/absolute-position recipe (announced, **not** `display:none`) — in
`global.css` so it is reusable — and apply it to the severity prefix.

**Kind-aware live region (a11y).** `info`/`ok` toasts stay polite
(`role="status" aria-live="polite"`); `error` toasts are announced assertively
(`role="alert" aria-live="assertive"`) so a failed/reverted move interrupts
rather than queuing. Render **two sibling live-region containers** in the viewport
— a polite region for `info`/`ok` and an assertive region for `error` — both
rendering from the **same ordered `toasts` array filtered by kind**, sharing the
existing close / hover-pause / resume / Escape handlers so ordering and per-toast
behaviour are preserved across the split. This preserves the assertiveness of the
`role="alert"` conflict banner that Phase 2 removes.

**Update the locked single-region tests.** The split contradicts existing locked
assertions that must be enumerated as deliberate changes:
`Toaster.test.tsx:132-137` asserts the single viewport is `role="status"
aria-live="polite"`, and `:152` asserts exactly one status region. Rewrite both to
assert the new structure (one polite `role="status"` region + one assertive
`role="alert"` region), and reconcile the board's single-region query in
`KanbanBoard.test.tsx` similarly.

**Live-region overlap (note).** On the kanban route the two toast regions coexist
with the board's own polite announcement region (`KanbanBoard.tsx:190`) and
dnd-kit's injected announcements (`KanbanBoard.tsx:174`). For a keyboard move,
dnd-kit's "Moved … to …" and the `ok` success toast both narrate the same event.
These are intentionally distinct (the toast is the persistent visual record; the
dnd-kit string is the transient move narration) — accept the mild redundancy, and
confirm via the screen-reader manual step that they read coherently rather than as
confusing duplication.

#### 3. Variant CSS via property indirection

**File**: `frontend/src/components/Toaster/Toaster.module.css`
**Changes**: Introduce a `--toast-accent` indirection defaulting to
`var(--ac-accent)` so `info` stays byte-identical; override per `data-kind`.

```css
.toast {
  --toast-accent: var(--ac-accent);
  /* …existing… */
  border-left: 3px solid var(--toast-accent);
}
.toast[data-kind='ok']    { --toast-accent: var(--ac-ok); }
.toast[data-kind='error'] { --toast-accent: var(--ac-err); }
.icon { color: var(--toast-accent); }
```

#### 4. Update the intentionally-locked CSS test

**File**: `frontend/src/components/Toaster/Toaster.test.tsx`
**Changes**: The lock at `:168-175` asserts `.icon` binds `--ac-accent` and *not*
`--ac-ok`/`--ac-warn`. Rewrite to assert the indirection: `.icon` and
`border-left` bind `var(--toast-accent)`, `--toast-accent` defaults to
`var(--ac-accent)`, and `data-kind='ok'`/`'error'` remap it to `--ac-ok`/`--ac-err`.

### Test-first additions:

- New `use-toast` test: `showToast({ heading, message })` yields a toast with
  `kind: 'info'`; `showToast({ …, kind: 'ok' })` yields `kind: 'ok'`.
- New `Toaster` render test: a toast with `kind: 'error'` renders
  `data-kind="error"` on the `.toast` element and the error icon.
- New a11y test: an `error` toast is rendered in the assertive region
  (`role="alert"`/`aria-live="assertive"`) while `info`/`ok` render in the polite
  region (`role="status"`/`aria-live="polite"`).
- New severity-prefix test: each kind renders its visually-hidden severity prefix
  ("Error:"/"Success:") for assistive tech.
- New dismiss test: an `error` toast does **not** auto-dismiss (advance fake
  timers past `TOAST_AUTO_DISMISS_MS`, assert still present) and **is dismissable
  via both the close button and Escape**; `info`/`ok` still auto-dismiss on the
  timer.
- New eviction test: with `MAX_TOASTS` auto-dismissing toasts already shown, a
  pre-existing `error` toast **survives** a burst of new `info`/`ok` toasts (errors
  exempt from the cap), while the oldest `info`/`ok` is evicted.
- New coexistence test: an `error` toast in the assertive region and an `ok` toast
  in the polite region are present simultaneously in correct order.
- Reconciliation: update the existing single-region assertions at
  `Toaster.test.tsx:132-137,152` and the board's single-region query in
  `KanbanBoard.test.tsx` to the two-region structure.
- Backward-compat: `use-external-edit-toast` tests
  (`toHaveBeenCalledWith({ heading, message })`) stay green untouched.

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type-checking passes: `mise run typecheck`
- [x] New variant tests assert `kind` default `'info'` and `ok`/`error` mapping
- [x] Updated CSS-source test asserts the `--toast-accent` indirection
- [x] `error` toast renders in the assertive region; `info`/`ok` in the polite
      region; each kind renders its sr-only severity prefix
- [x] `error` toast does not auto-dismiss; `info`/`ok` do

#### Manual Verification:

- [ ] An `info` toast is visually identical to before (accent border + icon)
- [ ] `ok` toast shows the success colour; `error` toast shows the error colour
- [ ] A screen reader announces an `error` toast assertively (interrupts) and
      reads its severity prefix
- [ ] Existing external-edit toast is unchanged

---

## Phase 2: Board toast-confirmation loop (B1 + B2)

### Overview

Wire the board's move outcome into the Toaster: a success toast on `204`, an
error toast + revert on failure/`412`, and remove the inline conflict banner.
Depends on Phase 1's variants.

### Changes Required:

#### 1. Raise success / error toasts from the move mutation

**File**: `frontend/src/routes/kanban/KanbanBoard.tsx`
**Changes**: **Only in the `'move'` branch** of `handleDragEnd` (i.e. guarded by
`outcome.kind === 'move'`), call `useToast()`'s `showToast` in
`onSuccess`/`onError`. The three no-op outcome branches (same-column,
other-rejected, unknown) keep their existing announcement-only behaviour and must
raise **no** toast and trigger **no** focus change — preserving "Release on same
column: no toast, no move." Revert is already handled by `useMoveWorkItem`'s
`onError` (`use-move-work-item.ts:40-42`); the board no longer renders a banner.

The success toast names the card and the target **label** (resolve via
`columns.find(c => c.key === outcome.toStatus)?.label`; assert the resolved label
is defined — never fall back to the raw key) and uses **plain user-facing copy**
in the existing toast voice (see Resolved decision 3 below), not the prototype's
raw HTTP/ETag line.

**Empty-body rendering.** Because the success toast is heading-only
(`message: ''`), update the Toaster to **omit** the `<p className={styles.message}>`
node entirely when `message` is empty (`Toaster.tsx:73`) — otherwise an empty `<p>`
still consumes the heading/body flex gap and success toasts look mis-padded next
to variants that have a body. Assert the empty-message toast renders no message
paragraph.

**Guard the source entry (consistency with the overlay null-check).**
`handleDragEnd` resolves `const source = entriesByRelPath.get(cardId)` and today
passes `move.mutate({ entry: source!, … })` (`KanbanBoard.tsx:108-113`). This has
the **same** concurrent-delete hazard the Phase 3 overlay null-guard addresses, so
apply the same convention: if `source` is missing (entry deleted mid-drag),
early-return to a no-op outcome rather than asserting non-null. Both the overlay
and the move path then follow one guarding standard.

```ts
// success: plain, user-facing copy (no transport jargon). The heading carries
// the confirmation; the body is omitted (heading-only toast):
showToast({
  kind: 'ok',
  heading: `${describeEntry(entry)} moved to ${targetLabel}`,
  message: '', // heading-only — see empty-body rendering note below
})
// error: assertive + persistent (Phase 1 routes kind:'error' to the assertive
// region with no auto-dismiss). Reuse the conflict copy, reviewed for the toast
// context (see #1a below):
showToast({
  kind: 'error',
  heading: 'Move failed',
  message: errorToastMessageFor(err),
})
```

#### 1a. Shared card-naming helper + toast error copy

**Files**: `frontend/src/routes/kanban/announcements.ts` (or a new shared
`describe-entry.ts`), `frontend/src/routes/kanban/KanbanBoard.tsx`
**Changes**: The success heading needs the same card-naming logic the
announcements use, but `describe` in `announcements.ts` is **module-private** with
signature `describe(id, entries)`. Extract a single exported `describeEntry`
helper consumed by **both** `announcements.ts` and the board toast, so the toast
heading and the screen-reader announcements cannot drift apart. **Specify the
signature as `describeEntry(entry: IndexEntry | undefined): string`** operating on
a *resolved* entry — `announcements.ts` keeps doing its existing `entries.get(id)`
lookup before calling it, and the board passes the `source`/`entry` it already
holds. Because the current `describe()` derives the work-item number from the
relPath, `describeEntry` must derive it from `entry.relPath`/`entry.workItemId`
(both present on `IndexEntry`), and the **`undefined` branch returns the existing
fallback string (e.g. "work item")** so the missing-entry wording is preserved
rather than degrading. Assert the missing-entry case as well as the happy path.

Likewise, review the reused `conflictMessageFor` copy (it currently embeds
banner-specific wording — "the card has been returned to its original column") for
the transient-toast context; introduce `errorToastMessageFor(err)` so the copy
reads correctly in a toast and distinguishes the `412`/`ConflictError` case from a
generic `FetchError`. **Dividing line:** transport/error-class-derived copy
(`errorToastMessageFor`) lives in `src/api/` alongside the fetch/error types; the
board-load-failure copy (`errorMessageFor`) stays in-route. The deleted banner's
`conflictMessageFor` is **superseded** by `errorToastMessageFor` (not retained).
To avoid two drifting error-class mappers, factor the `ConflictError`/`FetchError`
discrimination into a single shared predicate consumed by both copy functions, so
the branch logic lives once and only the user-facing strings differ.

#### 2. Remove the inline conflict banner

**File**: `frontend/src/routes/kanban/KanbanBoard.tsx`
**Changes**: Delete the `conflict` state, `showConflict`, `conflictTimerRef`, the
30s timer, and the banner JSX (`:177-189`). Keep the no-op `aria-live` region
(`:190-192`) and the board load-failure alert (`:149-160`). Focus restoration on
error moves to Phase 5 (it currently lives in this `onError`).

#### 3. Remove dead banner CSS

**File**: `frontend/src/routes/kanban/KanbanBoard.module.css`
**Changes**: Remove `.conflictBanner`, `.conflictMessage`, `.conflictDismiss`
(`:41-61`). Leave `.alert*` (load failure) and `.announcement`.

### Test-first additions:

The board's toast/revert logic lives inside `handleDragEnd`, reachable only
through dnd-kit's `DndContext`, and the existing jsdom board tests deliberately
drive state via `queryClient.invalidateQueries` rather than real drag events — so
there is **no faithful jsdom path** to fire a drop outcome and observe the
resulting `showToast`. Split the coverage accordingly:

- **Unit (pure mapping)**: extract a pure `moveToastFor(outcome, targetLabel,
  resultOrError)` function that maps a resolved outcome + the **already-resolved**
  target label + success/error into a `ShowToastInput` (or `null` for
  non-`move`/no-op outcomes). The caller resolves the label (`columns.find(c =>
  c.key === outcome.toStatus)?.label`, asserting it is defined, never the raw key)
  and passes it in, so the function stays a pure mapping with no board/config
  lookup of its own. Unit-test it exhaustively:
  - `204` success → `ok` toast whose heading contains the **exact human target
    label** (e.g. "In progress", never the raw `in_progress` key, never
    `undefined`).
  - `412`/`ConflictError` vs generic `FetchError` → `error` toast with the
    respective `errorToastMessageFor` copy (covered **separately** so swapping the
    branches fails).
  - same-column / any non-`move` outcome → returns `null` (no toast).
  - missing `source` entry (deleted mid-drag) → resolves to the no-op (no toast,
    no `move.mutate`), matching the overlay null-guard convention.
  This makes the assertions meaningful without faking the `DndContext`.
- **E2E (integrated)**: extend `kanban*.spec.ts` to assert the integrated path —
  a real drag to a new column shows the `ok` toast and the card stays; a forced
  `412` shows the assertive, persistent `error` toast and the card **reverts**.
- **Migrate `frontend/e2e/kanban-conflict.spec.ts`**: it currently asserts the
  removed `role="alert" aria-atomic="true"` banner after a 412. Rewrite it to
  assert the new assertive error toast (`role="alert"`/`aria-live="assertive"`,
  persistent) **plus** the card revert — otherwise Phase 2 turns a green E2E red
  and the only 412/revert E2E guard is lost.
- Empty-message success toast renders **no** message paragraph (see empty-body
  rendering above).
- The shared `describeEntry` helper is asserted to produce the same card name used
  by `announcements.ts` (one source of truth).

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type-checking passes: `mise run typecheck`
- [x] Unit `moveToastFor` covers: `204`→`ok` with exact label; `ConflictError` vs
      `FetchError`→`error` (separately); non-`move`/same-column→`null` (no toast)
- [x] **E2E** asserts the integrated success (toast + card stays) and `412`
      (assertive persistent error toast + revert) paths
- [x] `kanban-conflict.spec.ts` migrated from the banner to the assertive error
      toast + revert
- [x] Empty-message success toast renders no message paragraph
- [x] `describeEntry` is shared between the toast and `announcements.ts`

#### Manual Verification:

- [ ] Dragging a card to a new column shows a success toast (plain copy) and the
      card stays
- [ ] Forcing a `412` (concurrent edit) shows an assertive, persistent error
      toast and the card returns
- [ ] No inline conflict banner appears anywhere

---

## Phase 3: Drag affordance, defer-to-drop & design parity (A1, A3, aspects 6–7)

### Overview

Add a `DragOverlay` rendering a 0.6-opacity clone that follows the cursor, apply
`isDragging` styling to the source card, and bring the cursor and empty-column
copy into parity with the prototype. Fixes A1 and removes the A3 rendering
artefact together. The drag-state visual-regression baseline is A1's oracle.

### Changes Required:

#### 1. Render a DragOverlay clone

**File**: `frontend/src/routes/kanban/KanbanBoard.tsx`
**Changes**: Track the active drag id in `onDragStart`; render
`<DragOverlay>` (from `@dnd-kit/core`) containing a presentational clone of the
active card. Clear the id in `onDragEnd` **and** `onDragCancel`.

Guard the overlay lookup with a **real null check**, not a non-null assertion:
the board is SSE-live, so a concurrent external delete (whose queued
invalidation flushes when `setDragInProgress(false)` fires) can remove the
in-flight entry from the map mid-drag; `get(activeId)!` would then render
`<WorkItemCard entry={undefined}>` and crash.

```tsx
const [activeId, setActiveId] = useState<string | null>(null)
// onDragStart: setActiveId(event.active.id as string)
// onDragEnd / onDragCancel: setActiveId(null)
<DragOverlay>
  {(() => {
    const active = activeId ? entriesByRelPath.get(activeId) : null
    return active ? <WorkItemCardPresentation entry={active} overlay /> : null
  })()}
</DragOverlay>
```

**Add an `onDragCancel` handler** to the `DndContext`. dnd-kit invokes
`onDragCancel` (on Escape / interrupted drag) **instead of** `onDragEnd`, so
without it the drop-time teardown never runs. The teardown `onDragEnd` actually
owns is `docEvents.setDragInProgress(false)` (`KanbanBoard.tsx:105`) plus the new
`activeId` clear — **not** an announcement-timer reset (that happens on the *next*
`handleDragStart`, lines 96-102; `handleDragEnd` does not touch the timers, and
the dnd-kit cancel announcement is already wired in `announcements.ts:46-48`).

Extract a single **`endDrag()`** teardown — `setDragInProgress(false)` + clear
`activeId` — called by **both** `onDragEnd` and `onDragCancel`, leaving only
outcome-specific logic in `handleDragEnd`. This gives the gate-clearing invariant
one home, so a future edit to one handler can't reintroduce the stuck-gate bug
(which would otherwise leave the SSE drag-gate `true` and silently freeze live
board updates for the session).

**Ordering matters.** `endDrag()` must run **unconditionally at the start** of
`onDragEnd` — mirroring the current `setDragInProgress(false)` placement
(`KanbanBoard.tsx:105`), before outcome resolution and before the `move`-branch
and missing-`source` early-returns — so no drop path can skip the gate-clear. In
particular, keep the `setDragInProgress(false)` flush ordered **before**
`move.mutate(...)` (as today): the flush drains any SSE invalidations queued
mid-drag, and running it after the optimistic `onMutate` write could let a queued
external-edit refetch clobber the optimistic move. If folding both into `endDrag()`
would reorder the flush after the mutation dispatch, instead keep the flush at the
top and have `endDrag()` additionally clear only `activeId`.

#### 2. Extract a shared presentation + `isDragging` styling on the source card

**Files**: `frontend/src/routes/kanban/WorkItemCard.tsx`, new
`frontend/src/routes/kanban/WorkItemCardPresentation.tsx`
**Changes**: Rather than overloading one component with an `overlay` flag that
conditionally bypasses `useSortable` (a rules-of-hooks hazard, since the hook must
stay called) and the `<li>`/`<Link>` structure, **extract the shared inner visual
layout** into a presentational `WorkItemCardPresentation` that renders the card's
visuals from `entry` plus a small set of presentation props (e.g.
`overlay`/`dragging`) and **no** sortable or navigation wiring.

- `WorkItemCard` stays the sortable list item: it calls `useSortable`
  unconditionally, destructures `isDragging`, wraps the `<li data-relpath>` +
  `<Link>`, and renders `WorkItemCardPresentation` for the visuals — applying the
  dragging class when `isDragging`.
- The `DragOverlay` clone renders `WorkItemCardPresentation` directly with
  `overlay` (no `<li>`, no `useSortable`, no `<Link>` navigation) — the "lifted"
  copy.

This keeps each entry point single-responsibility and the sortable hook
unconditional, and avoids the dual-structure branching (it also helps the 0040
pipeline work that shares this file).

#### 3. Drag-state + cursor CSS

**File**: `frontend/src/routes/kanban/WorkItemCard.module.css`
**Changes**: Add the prototype values and cursor states.

```css
.card { cursor: grab; }                 /* aspect 7: grab at rest */
.cardDragging {                          /* source card while dragging (A1) */
  transform: rotate(1.5deg) scale(1.02);
  box-shadow: var(--ac-shadow-lift);
  border-color: var(--ac-accent);
}
.cardOverlay { opacity: 0.6; cursor: grabbing; }  /* clone following cursor */
```

#### 4. Empty-column copy to match prototype (aspect 6)

**File**: `frontend/src/routes/kanban/KanbanColumn.tsx` (+ `KanbanColumn.module.css`)
**Changes**: Replace the `aria-hidden` "No work items" line with the prototype's
two-line empty panel — title "Nothing here", body "Move a work item here to set
its status to {label}." Keep it a static placeholder (not hover-only).

- **Mechanism-neutral copy**: use "Move a work item here…" rather than the
  pointer-only "Drop a work item…", since keyboard users place cards via
  Space/arrows — the only instructional copy on the board should not imply a
  mouse-only gesture.
- **ARIA**: keep the panel `aria-hidden="true"` (matching the current
  placeholder). The column header already exposes the item count via its
  `aria-label`, so the count remains the single announced source of truth and the
  prose does not produce redundant/contradictory announcements. Assert the
  `aria-hidden` treatment in the unit test.
- **Reconcile the existing test**: `KanbanColumn.test.tsx:42-48` asserts the empty
  state via `getByText(/no work/i)`; rewrite it to the new "Nothing here" / "Move
  a work item here…" copy (keeping the `aria-hidden` check) so the copy change does
  not turn a green test red unannounced.

#### 5. New drag-state visual-regression baseline (A1 oracle)

**File**: `frontend/tests/visual-regression/…` (new spec) + `__screenshots__/…`
**Changes**: Add a spec that puts a card into the dragging state and captures it,
producing `drag-{dark,light}-…-{darwin,linux}.png` baselines.

**Cross-platform determinism (important).** Sub-pixel `rotate(1.5deg)
scale(1.02)`, `opacity: 0.6` compositing, and antialiased rotated text are exactly
what diverges most between the darwin and linux rasterizers, and existing
baselines are all static/axis-aligned — so this is a step-change in platform
sensitivity that can drift past `maxDiffPixelRatio`. Mitigate:

- **Capture from a static showcase surface, not a live drag.** `isDragging` only
  holds during a held pointer drag, which is not a deterministic screenshot frame.
  Following the existing `/glyph-showcase`/`/chip-showcase` pattern, render
  `WorkItemCardPresentation` with the `dragging`/`overlay` props applied
  statically (no live drag) on a showcase surface, and capture that — so the
  baseline frame is reproducible.
- **Constrain the capture to the card element** (element-level `toHaveScreenshot`,
  as `glyph-showcase.spec.ts`/`chip-showcase.spec.ts` already do), not a full-
  route capture, to minimise the rotated-text region under pixel comparison.
- **Disable transitions/animations** (or assert a settled state) before the
  screenshot so the capture frame is deterministic — the base transition (~140ms)
  could otherwise still be settling.
- **Prefer resolved-style probes for the exact values**: assert the source card's
  computed `transform`/`box-shadow`/`border-color` and the overlay's computed
  `opacity: 0.6` and `cursor` via `getComputedStyle` (the repo's
  `*-resolved-*.spec.ts` pattern), reserving the cross-platform **pixel** oracle
  for the overall affordance rather than the precise sub-pixel rotation. (The
  `grab`/`grabbing` cursor is OS-rendered and never appears in a screenshot —
  verify it only via computed CSS.) **Assert with tolerance, not string
  equality**: computed `transform` serializes to a float `matrix(…)` and
  `box-shadow` expands `var(--ac-shadow-lift)` into length+colour components, both
  of which can carry sub-pixel rounding across engines — parse and compare the
  rotation/scale within an epsilon. `opacity` (`0.6`), `cursor`, and
  `border-color` (integer rgb) serialize deterministically and can stay exact.

**Baseline generation & CI gate.** Generate darwin locally with
`--update-snapshots`; trigger the "Update visual regression baselines"
`workflow_dispatch` for linux. Per project memory, linux baselines drift behind
darwin and a GITHUB_TOKEN-pushed baseline commit does **not** re-trigger Main CI —
so a fresh CI run can lack/stale the `drag-…-linux.png` oracle while darwin is
present. **Success criteria must require the linux baseline to exist and the spec
to pass on the linux CI runner before merge** (not just `mise run
test:e2e:visualiser` locally on darwin), and the manual re-trigger step must be
performed after the baseline commit lands.

### Test-first additions:

- **A3 (integrated, E2E)**: the authoritative source-persistence assertion is an
  **E2E test** — drive the `dndDrag` pointer sequence, pause mid-drag (between
  move and release), and assert the source card is still rendered in its column
  while the overlay clone is present. jsdom cannot enter dnd-kit's active-drag
  state faithfully, so this is not a unit test.
- **Unit (styling branches)**: `WorkItemCardPresentation.test.tsx` — the
  `dragging` prop applies the dragging class; `overlay` renders the clone variant
  with no sortable attributes and no navigation `<Link>`. `WorkItemCard.test.tsx`
  — `useSortable`'s `isDragging` drives the dragging class on the sortable card.
  These test the prop wiring, explicitly **not** the integrated A3 behaviour.
- **Resolved-style probes**: assert the source card's computed
  `transform`/`box-shadow`/`border-color`, the overlay's computed `opacity: 0.6`,
  and the `grab`/`grabbing` `cursor` via `getComputedStyle` (with tolerance for
  `transform`/`box-shadow`).
- **`onDragCancel` SSE-gate regression**: Escape mid-drag, then assert a
  subsequent SSE/invalidation-driven update still renders (proving `endDrag()`
  cleared `setDragInProgress(false)`) — guarding the stuck-gate failure mode the
  prose calls out. (E2E, or a unit test asserting the cancel handler calls
  `setDragInProgress(false)`.)
- **Overlay null-guard regression**: when `entriesByRelPath.get(activeId)` is
  `undefined` (entry deleted mid-drag), the `DragOverlay` renders `null` rather
  than crashing — guards against the `!` assertion creeping back.
- The visual-regression baseline is the oracle for the overall drag affordance
  (captured from the static showcase surface with transitions disabled and the
  capture constrained to the card element).

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type-checking passes: `mise run typecheck`
- [x] E2E asserts source card persists mid-drag (A3) and the overlay renders (A1)
- [x] Resolved-style probes assert opacity 0.6, the rotation/lift/border, and the
      grab/grabbing cursor
- [~] Drag-state visual-regression baseline matches on **both** darwin and the
      **linux CI runner**: `mise run test:e2e:visualiser` _(darwin baselines
      generated and passing; linux baselines still to be generated via the
      "Update visual regression baselines" workflow + Main CI re-trigger before
      merge — see the convergence record)_

#### Manual Verification:

- [ ] A translucent (0.6) clone follows the cursor during drag
- [ ] The source card stays in its column, rotated/lifted with accent border
- [ ] Cursor is `grab` at rest and `grabbing` while dragging
- [ ] Empty columns show the "Nothing here" / "Move a work item here…" panel

---

## Phase 4: Drag-vs-click suppression (A2)

### Overview

Ensure starting a drag never triggers the card's `<Link>` navigation while a
genuine click still navigates, via a **card-local** drag guard that swallows the
post-drag synthetic click.

### Changes Required:

#### 1. Card-local "just dragged" guard

**File**: `frontend/src/routes/kanban/WorkItemCard.tsx`
**Changes**: `WorkItemCard` owns a short-lived `draggedRef` toggled by the
sortable's drag lifecycle (chosen over a board-passed signal: it keeps the card
independently unit-testable and avoids prop-drilling through `KanbanColumn`). On
the card's `onClickCapture`, if a real drag was in progress or just ended,
`preventDefault()`/`stopPropagation()` so navigation does not fire; otherwise
allow it.

```tsx
const draggedRef = useRef(false)
// set true when a real drag actually starts (activation threshold crossed),
// e.g. from useSortable's isDragging transition — NOT on mere pointerdown:
//   useEffect(() => { if (isDragging) draggedRef.current = true }, [isDragging])
// cleared on a boundary provably LATER than the synthetic click:
//   onDragEnd → setTimeout(() => { draggedRef.current = false }, 0)
const onClickCapture = (e: React.MouseEvent) => {
  if (isDragging || draggedRef.current) {
    e.preventDefault()
    e.stopPropagation()
  }
}
```

**Timing (correctness).** dnd-kit dispatches the suppressing synthetic click
during the same `pointerup` task. The clear must therefore run on a boundary
provably **later** than that click, and **not** `requestAnimationFrame` (a
coalesced synthetic click can fire after the next paint, clearing the guard too
early and re-introducing the A2 navigation). **Prefer clearing on the next
`pointerdown`** over a bare `setTimeout(…, 0)`: with a shared card-local ref, a
rapid second drag can start before the first drag's queued macrotask runs, and the
first drag's pending timer would then clear the guard mid-second-drag (a narrow
race where the first drag's synthetic click could leak through). The next
`pointerdown` of a new interaction naturally supersedes the prior guard window; if
a timeout is used instead, store and cancel the pending handle on the next drag
start. Key the guard off a **real drag having started** (the activation threshold
crossed / `isDragging` having gone true), not a bare press, so a genuine
sub-threshold click is never swallowed.

**Relationship to existing state.** This guard is deliberately **separate** from
the board's `docEvents.setDragInProgress` (which is cleared synchronously in
`handleDragEnd` and gates SSE invalidation) and from Phase 3's `activeId`. It is
the only one of the three whose clear is intentionally one tick late; document
this so the three drag signals are not conflated.

### Test-first additions:

- **A2 (authoritative, E2E)**: the headline assertion is an **E2E test** using the
  existing `dndDrag` helper — drag a card and release, assert the URL did **not**
  change; then click a card with no drag, assert navigation occurs. jsdom does not
  faithfully reproduce the PointerSensor's 5px activation or the synthetic
  post-drag click, so the unit level cannot prove the real behaviour.
- **Unit (supplementary)**: frame the test around the **pure suppress/allow
  decision** — given the guard set vs. not, the click is suppressed vs. passes
  through — without asserting the ref toggling or the clear timing (those couple to
  `useSortable` internals; the timing contract is the E2E's job). Documented as a
  decision check, not the integrated drag-then-click oracle.

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type-checking passes: `mise run typecheck`
- [x] **E2E**: drag-then-release does not navigate; plain click navigates
- [x] Unit: `onClickCapture` suppresses when the guard is set, passes through
      otherwise

#### Manual Verification:

- [ ] Dragging a card and releasing does not open the library page
- [ ] Clicking a card (no drag) opens its work-item library page

---

## Phase 5: Keyboard a11y verify + focus hardening (C1, C2, C3)

### Overview

Record verification of keyboard moves and announcements, and harden focus return
so it lands on the card `<Link>` in its final resting column on both success and
revert.

### Changes Required:

#### 1. Focus management on success and revert (C3)

**File**: `frontend/src/routes/kanban/KanbanBoard.tsx`
**Changes**: After a move settles, return focus to the dragged card's focusable
**anchor** — not the `<li>` wrapper. On success the card rests in the target
column; on revert it rests in the source column. Apply this **only** in the
`outcome.kind === 'move'` case (no focus churn on no-op drops, per Phase 2), to
both the move `onSuccess` and `onError` paths (replacing the error-only restore
removed in Phase 2).

**Timing (correctness — do not use a bare rAF).** The moved card's DOM node is
recreated by at least two asynchronous re-renders that a single
`requestAnimationFrame` will not out-wait: the optimistic `setQueryData` in
`onMutate`, and the `onSettled` `invalidateQueries` refetch
(`use-move-work-item.ts:44-46`) which resolves on a network/microtask boundary —
plus the DocEvents deferred-invalidation flush when `setDragInProgress(false)`
fires. A bare rAF can focus a node that the later refetch then unmounts, dropping
focus to `<body>`. Instead, tie restoration to **actual render completion**:

- Expose an **explicit focus contract** from the card over a DOM query: define a
  small named seam — e.g. a typed `focus()` method or a ref-registration callback
  prop on `WorkItemCard` keyed by relPath — owned by the card, so the board
  depends on that seam rather than the card's DOM structure, and focus survives the
  `WorkItemCardPresentation` split / markup refactors (no `querySelector` +
  `CSS.escape` + rAF chain). Cover the contract with the C3 test.
- Gate restoration on a **single-use pending-move token that is armed on
  settle, not on entries-list identity**. The naïve approach — set a token in the
  `move` branch and fire a `useEffect` keyed on entries identity — is **wrong**:
  `useMoveWorkItem.onMutate` writes the new status *optimistically*, so the card is
  already in the target column (anchor present) on the optimistic render; the
  effect would fire and self-clear there, and then the `onSettled`
  `invalidateQueries` refetch (`use-move-work-item.ts:44-46`) remounts the node and
  drops focus to `<body>` — the exact post-settle failure this is meant to prevent.
  (On the error path the optimistic render shows the card in the *target* column,
  so "focus the resting column" could also resolve to the wrong column before
  revert.)
- Instead, **arm the token in the mutation's `onSettled`** (which runs after both
  success-commit and error-revert have been applied and the refetch is in flight),
  carrying the *final resting* relPath. A declarative `useEffect` then consumes the
  token — focus the registered anchor, clear the token — when the entries list
  next renders with that anchor present. Because the token is armed only on settle,
  the effect is inert for the optimistic render and for any unrelated SSE-driven
  refetch, and fires exactly once on the post-settle render. Use this declarative
  effect, **not** an rAF poll loop (the fragile timing code this revision moves
  away from).

#### 2. Verify keyboard move (C1) — recorded E2E test

**File**: `frontend/e2e/kanban*.spec.ts` (E2E, not jsdom)
**Changes**: Drive the `KeyboardSensor` in a **real browser** via `page.keyboard`:
Space/Enter to pick up, arrow keys to move across columns, Space/Enter to drop,
and assert the card landed in the target column. This must be an E2E test:
dnd-kit's `sortableKeyboardCoordinates` computes geometry from real layout boxes
that jsdom does not provide (no `getBoundingClientRect` layout), so a jsdom
keyboard-move resolves to empty coordinates and silently fails to move the card —
producing a passing-but-vacuous "verification." Fix only if a defect surfaces;
otherwise the passing E2E test is the deliverable.

**Feasibility spike first (no existing precedent).** `page.keyboard` is used
nowhere in the current e2e suite and there is no existing dnd-kit keyboard-move
pattern to copy, so treat C1 as exploratory before committing it as a guaranteed
deliverable: spike that focusing the draggable's listeners element (the `<Link>`,
which also navigates) and pressing Space/arrows/Space actually drives
`sortableKeyboardCoordinates` and completes the cross-column move in the real
browser. Confirm the keyboard activation does not collide with the `<Link>`
navigation (the A2 guard is pointer-oriented). Record the spike outcome; if the
keyboard sensor needs configuration to make this work, that becomes the C1 fix.

#### 3. Verify announcements (C2) — assert verbatim strings

**File**: `frontend/src/routes/kanban/announcements.test.ts` (+ board-level check)
**Changes**: Assert the exact `announcements.ts` strings fire on pick-up, column
change, drop, and cancel, each naming the affected card and the relevant column:
- `Picked up ${describe}.`
- `${describe} is over ${label}.`
- `Moved ${describe} to ${label}.` / `Drop of ${describe} cancelled, no target.`
- `Drag of ${describe} cancelled.`

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Type-checking passes: `mise run typecheck`
- [x] **E2E** keyboard-move test completes a cross-column move (C1)
- [x] Announcement assertions match `announcements.ts` strings verbatim (C2)
- [x] Focus-return test (authoritative at **E2E**; any unit test uses
      deterministic rAF/timer flushing or `waitFor` on `activeElement`): focus
      lands on the card `<Link>` anchor in the target column on success and the
      source column on revert, **after** the `onSettled` invalidation resolves (C3)
- [x] Focus single-fire isolation: after a move settles and focus is applied, an
      **unrelated** entries-list change (e.g. an external SSE edit) does **not**
      re-apply focus (asserts the on-settle token is single-use); the
      relPath-keyed focus contract resolves to the live anchor after a refetch
      remount _(token armed only in onSettled; relPath-keyed registry covered by
      kanban-focus-registry.test.ts)_

#### Manual Verification:

- [ ] Keyboard-only: pick up (Space/Enter), move (arrows), drop (Space/Enter)
      completes a move; Escape cancels and returns the card
- [ ] A screen reader announces pick-up, column change, drop, and cancel
- [ ] After a move or revert, keyboard focus is on the moved card

---

## Phase 6: Convergence record + cross-config verification

### Overview

Produce the recorded side-by-side comparison that is section A's exit condition,
and verify every behaviour against the two representative live configs. This
phase records verdicts and logs any out-of-scope discrepancy as a follow-up work
item rather than absorbing it.

### Changes Required:

#### 1. Recorded side-by-side comparison

**File**: a comparison record (e.g.
`meta/research/design-gaps/2026-06-06-0086-kanban-convergence-record.md` or
appended to the work item)
**Changes**: For each section-A interaction aspect, record an explicit
**parity / fixed / follow-up** verdict against `view-kanban.jsx`:
1. Drag affordance (A1) — *fixed* (Phase 3)
2. Click-vs-drag activation (A2) — *fixed* (Phase 4)
3. Defer-to-drop (A3) — *fixed* (Phase 3)
4. Drop settle / animation — *parity* (prototype has none)
5. Same-column drop no-op — *parity* (`resolve-drop-outcome.ts:28,39`)
6. Empty-column hover/drop copy — *fixed* (Phase 3) or *follow-up* if larger
7. Cursor grab/grabbing — *fixed* (Phase 3)

Extend the checklist if the comparison reveals more aspects; anything not
fixable within the `KanbanBoard`/`KanbanColumn`/`resolve-drop-outcome` surface is
logged as a follow-on work item referencing 0086 as its origin.

#### 2. Cross-config verification (two live configs)

**File**: E2E specs under `frontend/e2e/` (extend `kanban*.spec.ts` /
custom-column specs)
**Changes**: Exercise drag, drop, toast, and keyboard behaviours against (a) a
three-column config and (b) a five-column config including at least one label of
≥30 characters that wraps to two or more lines at the board's column width.
Assert every behaviour holds in both.

**Assert the wrap behaviourally, and keep it font-robust.** The wrapping label is
the specific risk being guarded (layout shift breaking drop targets / label
rendering), so add an explicit assertion that the long label actually occupies
≥2 lines — e.g. its measured `clientHeight` exceeds a single-line height, or a
non-ellipsis check — not merely that drag/drop/toast/keyboard still function.
Because line-break position depends on resolved glyph widths, choose a label
length comfortably beyond a single line at the fixed viewport column width (not
marginally over). Note the font stack is **self-hosted woff2** served from
`/fonts/` (the same glyph binaries load identically on darwin and linux;
`fonts.test.ts` forbids any Google Fonts origin), so the determinism risk is
**not** third-party availability — it is the `font-display: swap` fallback not yet
having resolved at measurement time. Wait for `document.fonts.ready` before taking
the `clientHeight` measurement so the wrap is measured against the real glyphs, not
the swap fallback.

### Success Criteria:

#### Automated Verification:

- [x] E2E passes for both configs: `mise run test:e2e:visualiser`
- [x] Cross-config specs cover drag, drop, toast, and keyboard in 3- and 5-column
      sets (with a ≥30-char wrapping label)

#### Manual Verification:

- [x] Recorded comparison exists with a parity/fixed/follow-up verdict for every
      section-A aspect; A1–A3 are at least *fixed*
- [x] Any discrepancy too large for this story is logged as a follow-up work item
      referencing 0086 (none required — all aspects resolved in-surface)
- [ ] All behaviours verified by eye against both column configurations

---

## Testing Strategy

Test levels are chosen by what each behaviour can be faithfully exercised at:
dnd-kit's pointer/keyboard sensors and the synthetic post-drag click depend on
real layout geometry and pointer sequences that jsdom cannot reproduce, so the
integrated drag/keyboard behaviours (A2, A3, C1, C3) are **E2E-authoritative**,
with unit tests confined to pure logic, styling branches, and isolated handlers.

### Unit Tests:

- Toast `kind` default + variant mapping; assertive-vs-polite region routing;
  sr-only severity prefix; `error` no-auto-dismiss; **errors exempt from the
  `MAX_TOASTS` cap** while `info`/`ok` are capped; error dismissable via close
  button + Escape; error/ok coexistence ordering (`use-toast.test.tsx`,
  `Toaster.test.tsx` — incl. the reconciled `:132-137,152` region assertions).
- Pure **`moveToastFor(outcome, resultOrError)`** mapping: `204`→`ok` with exact
  label; `ConflictError` vs `FetchError`→`error` separately; non-`move`→`null`.
  Empty-message success toast renders no message paragraph.
- `WorkItemCardPresentation` `dragging`/`overlay` styling branches; `WorkItemCard`
  `isDragging` class; `onClickCapture` pure suppress/allow decision.
- Resolved-style probes (with tolerance for transform/box-shadow): overlay opacity
  0.6, source rotation/lift/border, grab/grabbing cursor.
- `describeEntry` shared helper produces the same name as `announcements.ts`;
  announcement strings verbatim (C2).
- Existing `resolve-drop-outcome`/`announcements`/`use-move-work-item` suites stay
  green (no behavioural change to those modules).

### Integration / E2E Tests:

- A2 — drag-then-release does not navigate; plain click navigates (needs a
  decomposable/shared drag helper; see note below).
- A3 — source card persists mid-drag while the overlay clone is present (requires
  a mid-drag inspection point, i.e. the decomposed drag helper).
- C1 — keyboard cross-column move completes (`page.keyboard`) — **gated on the
  feasibility spike**; no existing precedent.
- C3 — focus returns to the card anchor in the resting column on success/revert,
  after invalidation settles, via the card focus contract.
- Integrated move toasts: success (toast + card stays) and `412` (assertive
  persistent error toast + revert); **`kanban-conflict.spec.ts` migrated** off the
  removed banner.
- `onDragCancel` SSE-gate: Escape mid-drag, then a later update still renders.
- `frontend/e2e/kanban*.spec.ts`: drag, drop, toast, keyboard against the 3- and
  5-column configs, asserting the ≥30-char label wraps to ≥2 lines behaviourally
  (after `document.fonts.ready`).
- Drag-state visual-regression baseline (new spec, static showcase surface,
  transitions disabled, element-level capture) as A1's overall-affordance oracle,
  gated on **both** darwin and the linux CI runner.

**Drag E2E helper.** The current `dndDrag` is duplicated inline in two specs and
runs an uninterruptible down→move→up sequence. Extract a shared, **decomposed**
drag helper (separate down / move / inspect / up steps) so A2/A3 can assert state
mid-drag (between move and `mouse.up()`); the single-call form remains for
drop-only tests.

### Manual Testing Steps:

1. Drag a card: confirm 0.6 clone follows cursor, source stays rotated/lifted.
2. Release on a new column: success toast (card + label + body); card stays.
3. Release on same column: no toast, no move.
4. Force a concurrent edit, then drag: error toast; card reverts; no banner.
5. Keyboard: Space → arrows → Space moves; Escape cancels; focus on moved card.
6. Repeat 1–5 on both the 3-column and 5-column (wrapping-label) configs.

## Performance Considerations

Negligible. `DragOverlay` renders one extra presentational card during a drag;
toast variants are CSS/attribute changes. No new network calls or data-shape
changes.

## Migration Notes

No data migration. The `kind` field is optional and defaults to `info`, so the
one existing `showToast` caller (`use-external-edit-toast.ts`) is unaffected.
Removing the conflict banner deletes board-local state/CSS only.

## References

- Original work item: `meta/work/0086-kanban-drag-and-drop.md`
- Related research: `meta/research/codebase/2026-06-06-0086-kanban-drag-and-drop.md`
- Design target (authoritative): `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-kanban.jsx`
- Prototype CSS: `.../prototype-full/src/app.css:961-965,1352-1361`
- Board: `frontend/src/routes/kanban/KanbanBoard.tsx`, `KanbanColumn.tsx`,
  `WorkItemCard.tsx`, `resolve-drop-outcome.ts`, `announcements.ts`
- Write path: `frontend/src/api/fetch.ts:32-57`,
  `frontend/src/api/use-move-work-item.ts:15-48`, `server/src/api/docs.rs:127-289`
- Toaster: `frontend/src/components/Toaster/Toaster.tsx`,
  `frontend/src/api/use-toast.ts`, `Toaster.module.css`,
  `Toaster.test.tsx:168-175`, `frontend/src/api/use-external-edit-toast.ts:45-48`
- Tokens: `frontend/src/styles/global.css:99-101,361-363`
- ADR-0024 (configurable kanban columns)
- Related: 0039 (Toaster — done), 0040 (Pipeline Visualisation Overhaul — shares
  `WorkItemCard.tsx`)
