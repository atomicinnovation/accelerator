---
type: work-item
id: "0086"
title: "Kanban Drag-and-Drop with Toast Confirmations"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: story
priority: medium
relates_to: ["work-item:0040"]
tags: [design, frontend, kanban, accessibility]
last_updated: "2026-06-06T12:54:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0086: Kanban Drag-and-Drop with Toast Confirmations

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a user managing work items on the kanban board, I want drag-and-drop that
is reliable, matches the prototype's interaction design, and confirms each move
with a toast — so I can re-status work directly on the board without bugs or
ambiguity.

The board already ships drag-and-drop (via dnd-kit) backed by a mature status
write-back path, but the interaction is buggy and off-design, and successful
moves are not confirmed through the Toaster. This story is a quality pass: fix
the interaction defects, bring the kanban page into line with the prototype
design, complete the toast-confirmation loop, and verify/harden keyboard
accessibility.

## Context

Contrary to the original draft, there is only one application (the visualiser:
React frontend + Rust/axum server). The "prototype" is a static design
reference, not a separate app — its intended kanban interaction lives in
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-kanban.jsx`,
which is the **authoritative** design reference for this story's design-match
work (confirmed current; no newer reference supersedes it).

The following already exist in the live app and are **not** to be rebuilt:

- **Drag-and-drop** between columns via `@dnd-kit/core` in
  `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.tsx`
  (pointer + keyboard sensors, `resolve-drop-outcome.ts`, `announcements.ts`).
- **Status write-back** on drop via `PATCH /api/docs/{path}/frontmatter`
  (`server/src/api/docs.rs`), with `If-Match`/ETag optimistic concurrency,
  work-item-only + accepted-column validation, and a `204` + fresh ETag on
  success / `412` on conflict. Wired client-side through
  `patchWorkItemFrontmatter` (`frontend/src/api/fetch.ts`) and
  `useMoveWorkItem` (optimistic update + rollback).
- **Configurable columns** per ADR-0024, served by `GET /api/kanban/config`
  and consumed via `useKanbanConfig()`.
- **The Toaster** (work item 0039, done) at
  `frontend/src/components/Toaster/Toaster.tsx` — but it is **info-only** (no
  success/error variants) and is **not currently wired into the board**, which
  instead shows an inline `role="alert"` conflict banner.

The prototype's `view-kanban.jsx` establishes the interaction target: a
translucent card follows the cursor while the source card shows a rotated
dragging state; the card only enters the target column on release; a
same-column drop is a no-op; and a successful move emits a success toast
(`kind: "ok"`) naming the item and target column with a technical body line
(`PATCH … → 204 · fresh ETag received`). The prototype models no error path —
error/rollback behaviour is a real-app concern.

## Requirements

### A. Drag-interaction fixes (against the prototype design)

The three issues below are **known examples**, not an exhaustive list. The work
in this section is an **iterative convergence loop**, not a fixed checklist:
repeatedly compare the live kanban page against the prototype design
(`view-kanban.jsx`), fix the most significant interaction discrepancy found,
and repeat — converging the live interaction towards the prototype. A1–A3 are
the seed discrepancies the loop must address at minimum.

The loop's **exit condition** — what makes this section "done" — is a recorded
side-by-side pass that gives an explicit verdict for *each* of the interaction
aspects enumerated below — **parity** (already matches the prototype, no change
needed), **fixed** (required a change, now matches), or **follow-up** (too large
for this story, logged as a separate work item) — so closure is a repeatable
procedure rather than a "looks done to me" judgement. The pass is satisfied when
every aspect is parity, fixed, or follow-up; A1–A3 must be at least *fixed*. The
aspect checklist (extend it if the comparison reveals more):

1. Drag affordance — translucent clone + rotated source card (A1).
2. Click-vs-drag activation — drag does not navigate; a genuine click does (A2).
3. Defer-to-drop — card stays in source column until release (A3).
4. Drop settle / animation as the card enters the target column.
5. Same-column drop is a no-op.
6. Empty-column hover and drop-target affordance / copy.
7. Cursor state during drag (grab / grabbing).

"Localised enough to fix within this story" means fixable within the existing
`KanbanBoard` / `KanbanColumn` / `resolve-drop-outcome` surface without new
components or API changes; anything larger is split into a follow-on work item
(referencing 0086 as its origin) rather than absorbed here.

- **A1 — Drag affordance**: while a card is being dragged, render a translucent
  copy that follows the cursor; the original card remains in its column with a
  slight rotation and dragging styling. The prototype (`view-kanban.jsx`) is the
  source of truth for the exact opacity and rotation values. The concrete oracle
  is a **visual-regression snapshot** of the dragging state checked into the
  project's existing baseline set; the snapshot test passes when it matches the
  approved baseline (which is itself derived from the prototype's affordance),
  rather than a subjective eyeball comparison. (Currently no drag affordance is
  shown.)
- **A2 — Drag must not navigate**: starting a drag must not trigger the card's
  click handler (which navigates to the work-item library page). A genuine
  click with no drag must still navigate.
- **A3 — Card persists until drop**: the dragged card stays rendered in its
  source column for the entire drag and only enters the target column on mouse
  release — not on drag-over. (Currently the card disappears while hovering a
  target column.)

### B. Toast-confirmation loop

- **B1 — Success toast**: on a successful move (write-back `204`), emit a
  success Toaster naming the card and target column, with a technical body line
  reporting the write outcome (the prototype's form is
  `PATCH … → 204 · fresh ETag received`).
- **B2 — Error toast + revert**: on write failure or `412` conflict, emit an
  error Toaster and revert the card to its source column. This **replaces** the
  current inline conflict banner — the banner is removed.
- **B3 — Toaster variants**: extend the Toaster component with success and
  error variants (it is info-only today).

### C. Keyboard accessibility (verify-and-harden)

**The Library decision in Open Questions is a hard precondition to this
section**: it must be resolved before section C is worked, so the concrete
keyboard mechanism is fixed by the time the keyboard work begins. The *required
behaviour* is mechanism-agnostic and is what C1–C3 specify; the dnd-kit artefacts
named below (keyboard sensor + `announcements.ts`) are the concrete instantiation
under the **dnd-kit-retention** outcome — the one the decision's rule favours, and
the expected outcome. If the decision instead selects raw HTML5 DnD, the same
behaviour is verified against that approach's equivalent keyboard-move and
announcement source, substituting those artefacts for the dnd-kit ones.

- **C1**: verify a card can be picked up, moved across columns, and dropped
  using the keyboard via the existing dnd-kit keyboard sensor — i.e. the
  sensor's activation keys (Space/Enter to pick up and to drop, arrow keys to
  move between columns, Escape to cancel) complete a cross-column move. The
  deliverable is a recorded verification (e.g. a passing keyboard test); if a
  defect is found it is fixed, otherwise the passing record is the outcome.
- **C2**: verify screen-reader announcements fire on pick-up, column change,
  drop, and cancel, and that each announcement names the affected card and (for
  column-change and drop) the target column, and (for cancel) the return to the
  source column. The exact announcement strings are those produced by
  `announcements.ts` — assert against those strings rather than that *some* event
  fired. As with C1, the deliverable is a recorded verification.
- **C3**: harden focus management — after a successful move or a failed-write
  revert, focus returns to the card that was being dragged (in its final resting
  column: the target on success, the source on revert).

### Out of scope

- Column-set configuration (owned by ADR-0024; already dynamic).
- Reordering cards within a column (not in the prototype; cross-column status
  moves only).
- Mutating frontmatter fields other than `status`.

## Acceptance Criteria

- [ ] Given a card is being dragged, when the pointer moves, then a translucent
  clone follows the cursor and the source card shows the rotated dragging state,
  matching the approved drag-affordance visual-regression baseline.
- [ ] Given a user begins dragging a card, when the drag starts, then the card's
  click navigation does not fire; and given a user clicks a card without
  dragging, then the work-item library page opens.
- [ ] Given a card is dragged over a different column, when the pointer has not
  been released, then the card remains rendered in its source column and only
  moves on release.
- [ ] Given the drag-interaction convergence loop, when this story is closed,
  then a recorded side-by-side comparison against the prototype
  (`view-kanban.jsx`) exists giving each interaction aspect in the section-A
  checklist an explicit parity / fixed / follow-up verdict, with every aspect at
  parity, fixed, or logged as a follow-up work item, and at minimum A1–A3 fixed.
- [ ] Given a drop whose write-back returns `204`, when it settles, then a
  success toast appears naming the card and the new column, with a technical body
  line reporting the write outcome (status and ETag).
- [ ] Given a drop whose write-back fails or returns `412`, then an error toast
  appears and the card returns to its source column, and no inline conflict
  banner is shown.
- [ ] Given the Toaster component, when invoked, then it supports distinct
  success and error variants.
- [ ] Given keyboard-only operation, when a user picks up (Space/Enter), moves
  across columns (arrow keys), and drops (Space/Enter) a card, then the move
  completes; and screen-reader announcements fire on pick-up, column change,
  drop, and cancel (Escape), each announcement naming the affected card and the
  relevant column, with the strings matching those defined in `announcements.ts`.
- [ ] Given a move completes or a failed write reverts, then keyboard focus
  returns to the card that was being dragged, in its final resting column (target
  on success, source on revert).
- [ ] Given the drag, drop, toast, and keyboard behaviours above, when they are
  exercised against two representative **live-board column configurations**
  supplied by `GET /api/kanban/config` — (a) a three-column set and (b) a
  five-column set including at least one label of ≥30 characters that wraps to two
  or more lines at the board's column width — then every behaviour holds in both.
  (Column labels and count are driven entirely by config per ADR-0024 and are out
  of scope to change; the prototype's Todo / In progress / Done columns are a
  design reference for *interaction*, not a target column set to reproduce — the
  live board's own configured columns are authoritative. These two configurations
  stand in for the "any configured column set" guarantee.)

## Open Questions

- **Library decision** (resolve during the research/planning phase): retain
  dnd-kit or move to raw HTML5 drag-and-drop? Decision rule: prefer raw HTML5
  DnD only if it concisely expresses *all* desired behaviour including keyboard
  accessibility and rollback; otherwise retain a library, since re-implementing
  keyboard accessibility (a11y) by hand amounts to owning a custom DnD
  implementation. (The
  prototype uses raw HTML5 DnD but is a non-persistent simulation with no
  keyboard a11y or rollback; the live app already uses dnd-kit with both.)
- _(Resolved)_ The prototype (`view-kanban.jsx`) is confirmed as the
  authoritative design reference for the iterate-against-the-design work; no
  newer reference supersedes it. See Context and References.

## Dependencies

- Blocked by: none. (0039 Toaster — **done**; the 0044 column-set spike was
  **abandoned** and is no longer a dependency.)
- Precondition (soft ordering): 0040's `routes/kanban/WorkItemCard.tsx` changes
  are confirmed merged before this story starts. Its work-item status is not yet
  updated, so the "complete in the codebase" claim is an out-of-band assumption;
  if it proves false, the shared-surface merge-conflict risk re-materialises.
- Blocks: none. The B3 Toaster-variant work extends the shared, globally-mounted
  Toaster (0039, dispatched via `use-toast.ts`); the addition is intended to be
  purely additive — existing info-only call sites keep their current behaviour
  and a new variant field defaults to info — so no current consumer is broken. If
  the variant API turns out not to be backward-compatible, those consumers become
  affected and must be captured here.
- Related: 0040 (Pipeline Visualisation Overhaul) — shares the
  `routes/kanban/WorkItemCard.tsx` surface; that work is complete in the
  codebase though its work-item status is not yet updated, so the previously
  flagged merge-conflict risk is effectively resolved (subject to the precondition
  above).

## Assumptions

- The prototype's three columns (Todo / In progress / Done) are illustrative;
  the live board's configurable column set (ADR-0024) is authoritative, so all
  interaction behaviour must hold for any configured columns.

## Technical Notes

- Live board: `frontend/src/routes/kanban/KanbanBoard.tsx` (+ `KanbanColumn.tsx`,
  `resolve-drop-outcome.ts`, `announcements.ts`).
- Write path: `patchWorkItemFrontmatter` (`frontend/src/api/fetch.ts`) →
  `useMoveWorkItem` (`frontend/src/api/use-move-work-item.ts`) →
  `PATCH /api/docs/{path}/frontmatter` (`server/src/api/docs.rs`), ETag via
  `If-Match`, byte-level YAML edit in `server/src/patcher.rs`.
- Toaster: `frontend/src/components/Toaster/Toaster.tsx`, dispatched via
  `frontend/src/api/use-toast.ts` (`showToast({ heading, message })`); mounted
  once in `RootLayout.tsx`. Adding success/error variants is net-new.
- A2 (drag vs. click) is a click-vs-drag activation problem — under dnd-kit this
  is typically a pointer activation constraint (distance/delay); under raw HTML5
  DnD it requires suppressing the click that follows a drag.
- A3 (card disappears on drag-over) indicates optimistic state is mutating on
  drag-over rather than deferring the list change to drop.

## Drafting Notes

- Enrichment found the feature largely shipped (dnd-kit DnD + a mature PATCH
  write-back endpoint + configurable columns). Scope therefore pivoted from
  "build drag-and-drop" to "fix interaction defects + match the design +
  complete the toast loop + verify keyboard a11y". A reviewer who expected a
  greenfield build should note this.
- The original "no third-party library / raw HTML5 DnD" requirement was relaxed
  to a decision rule (see Open Questions) because the live app already uses
  dnd-kit and keyboard a11y is now in scope.
- The original "a status-mutation endpoint exists or can be added trivially"
  assumption was retired — it is now a confirmed fact recorded in Context.
- The known issue list is intentionally non-exhaustive; matching the prototype
  is an iterative convergence activity (iterate against the design and fix
  inconsistencies), bounded by a recorded side-by-side exit condition rather
  than an enumerated up-front checklist. The recorded comparison is a
  lightweight by-product of the convergence loop, not a separate audit phase.
- Frontmatter was migrated from the legacy shape (`work_item_id:` / `type:`) to
  the unified shape (`id:` / `kind:` + `type: work-item` discriminator and
  schema fields) during this enrichment.
- 0040 is treated as done-but-unmarked, hence Related rather than a blocker.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Design target (authoritative reference): `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-kanban.jsx`
- Related: 0039 (Toaster — done), 0040 (Pipeline Visualisation Overhaul)
- ADR-0024 (configurable kanban columns)
