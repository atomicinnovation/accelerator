---
date: "2026-06-06T22:35:00+00:00"
type: design-gap
work_item_id: "work-item:0086"
target_inventory: "meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-kanban.jsx"
author: "Toby Clemson"
status: accepted
tags: [design, gap-analysis, kanban, drag-and-drop, convergence]
---

# Kanban Drag-and-Drop Convergence Record (work item 0086)

## Overview

Section A of work item 0086 required a recorded side-by-side comparison of the
live kanban board's drag interaction against the prototype (`view-kanban.jsx`,
with CSS in `app.css:961-965,1352-1361`), giving every section-A interaction
aspect a **parity / fixed / follow-up** verdict, with A1–A3 at least *fixed*.

This record is the exit condition for section A. Each verdict below was
established by the automated coverage landed across the implementation (unit,
resolved-style probes, and E2E in a real browser) plus manual review against the
prototype.

## Aspect-by-aspect verdict

| # | Aspect | Verdict | Evidence |
|---|--------|---------|----------|
| 1 | Drag affordance (A1) — lifted clone follows cursor; source card lifts | **fixed** | `DragOverlay` renders a 0.6-opacity `WorkItemCardPresentation` clone; the source card shows `rotate(1.5deg) scale(1.02)` + `--ac-shadow-lift` + accent border. Oracle: `tests/visual-regression/kanban-card-showcase.spec.ts` (darwin baselines generated) + computed-style probes in `kanban-card-resolved-styles.spec.ts`. |
| 2 | Click-vs-drag activation (A2) — a drag never navigates; a click always does | **fixed** | Card-local drag guard suppresses the synthetic post-drag click (`WorkItemCard.tsx` `shouldSuppressClick` + `onClickCapture`/`onPointerDownCapture`). Oracle: `e2e/kanban-click-vs-drag.spec.ts`. |
| 3 | Defer-to-drop (A3) — the source card stays in its slot mid-drag | **fixed** | A3 was a *rendering* artefact of the un-overlaid node, not a data-model bug; the `DragOverlay` resolves it. The data model already defers the cross-column move to `handleDragEnd` → `resolveDropOutcome` → `move.mutate` (no `onDragOver` mutation). Oracle: `e2e/kanban-drag-overlay.spec.ts` asserts the source persists while the clone renders. |
| 4 | Drop settle / animation | **parity** | The prototype has no drop-settle animation; the live board matches (no bespoke settle animation introduced). |
| 5 | Same-column drop no-op | **parity** | `resolve-drop-outcome.ts:28,39` returns `no-op-same-column`; the board raises no toast and triggers no focus change. Matches the prototype's "release on same column → nothing happens". |
| 6 | Empty-column copy | **fixed** | Static two-line panel — "Nothing here" / "Move a work item here to set its status to {label}." (`KanbanColumn.tsx`). Copy is mechanism-neutral ("Move … here", not the prototype's pointer-only "Drop … ") so keyboard placement is not implied to be mouse-only; the panel stays `aria-hidden` (the header count is the single announced source of truth). |
| 7 | Cursor grab/grabbing | **fixed** | `.card { cursor: grab }` at rest; the overlay clone uses `cursor: grabbing`. Verified via computed CSS (`kanban-card-resolved-styles.spec.ts`) — the OS-rendered cursor never appears in a screenshot. |

All section-A aspects are **parity** or **fixed**; A1–A3 are **fixed**, satisfying
the exit condition. No discrepancy required a follow-on work item — every aspect
was resolvable within the `KanbanBoard` / `KanbanColumn` / `WorkItemCard` /
`resolve-drop-outcome` surface.

## Beyond section A (B and C)

- **B (toasts):** a successful move raises a heading-only `ok` toast naming the
  card and the target column's human label; a failed/`412` move raises an
  assertive, persistent `error` toast and reverts the card. The inline conflict
  banner is removed. Covered by `move-toast.test.ts`, `Toaster.test.tsx`,
  `kanban.spec.ts`, and `kanban-conflict.spec.ts`.
- **C (keyboard a11y):** keyboard pick-up / cross-column move / drop completes
  (`kanban-keyboard.spec.ts`, C1); the four `announcements.ts` lifecycle strings
  are asserted verbatim (`announcements.test.ts`, C2); focus returns to the card
  anchor in its resting column on both success and revert via a relPath-keyed
  focus registry + on-settle token (C3).

## Cross-config verification

`e2e/kanban-cross-config.spec.ts` exercises drag, drop, toast, and keyboard
against both a 3-column and a 5-column config, the latter with a ≥30-char label
("Awaiting downstream review and final sign-off") asserted to wrap to ≥2 lines
(measured after `document.fonts.ready`) without breaking the column's drop
target.

## Outstanding pre-merge step (not a code gap)

The drag-state visual-regression **darwin** baselines were generated locally
(`tests/visual-regression/__screenshots__/kanban-card-showcase.spec.ts-snapshots/drag-*-darwin.png`).
Per project memory, linux baselines drift behind darwin and a GITHUB_TOKEN-pushed
baseline commit does not re-trigger Main CI. Before merge: run the "Update visual
regression baselines" `workflow_dispatch` to generate the `drag-*-linux.png`
baselines, then manually re-trigger Main CI so the spec runs against them on the
linux runner.
