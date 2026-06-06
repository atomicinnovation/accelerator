---
type: plan-review
id: "2026-06-06-0086-kanban-drag-and-drop-review-1"
title: "Plan Review: Kanban Drag-and-Drop with Toast Confirmations"
date: "2026-06-06T16:53:04+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-06-0086-kanban-drag-and-drop"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, usability, standards, portability]
review_number: 1
review_pass: 4
tags: [plan-review, frontend, kanban, drag-and-drop, dnd-kit, toaster, accessibility]
last_updated: "2026-06-06T19:15:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Kanban Drag-and-Drop with Toast Confirmations

**Verdict:** REVISE

This is a well-scoped, research-grounded defect-and-polish plan that correctly
avoids rebuilding shipped machinery, diagnoses A1/A3 as a single rendering-layer
root cause, and models the Toaster variant change as a clean additive
CSS-custom-property indirection. The plan is unusually test-conscious and
sequences its six phases by dependency direction. It needs revision before
implementation, however: several load-bearing mechanisms are left under-specified
in ways that touch correctness (the A2 click-suppression timing, the C3
focus-restore timing against async re-renders), the interaction tests are pinned
at a jsdom level where dnd-kit's sensors cannot be faithfully driven, and the
error-feedback path is downgraded from an assertive `role="alert"` banner to a
polite, auto-dismissing toast without acknowledgement.

### Cross-Cutting Themes

- **Duplicate drag-in-progress state** (flagged by: Architecture, Code Quality,
  Correctness, Standards) — Phase 4's new click-suppression flag overlaps the
  board's existing `docEvents.setDragInProgress` signal (KanbanBoard.tsx:97,105)
  *and* Phase 3's new `activeId`. Three parallel representations of "a drag is
  happening" with different lifetimes (synchronous SSE-gate vs. one-tick-late
  click guard) is the single most-flagged structural risk.

- **Error feedback is silently downgraded** (flagged by: Usability, Standards,
  Architecture, Correctness) — removing the `role="alert" aria-atomic="true"`
  conflict banner (30s) and routing failures through the Toaster's
  `role="status" aria-live="polite"` region (5s auto-dismiss) makes a
  data-loss-adjacent event (a reverted move) *less* assertive and shorter-lived.
  No lens disputes removing the banner; all four object to the loss of
  assertiveness/persistence without acknowledgement.

- **Focus restoration is timing-fragile** (flagged by: Correctness,
  Architecture, Test Coverage, Code Quality, Usability) — a single
  `requestAnimationFrame` does not reliably out-wait the optimistic
  `setQueryData` *and* the `onSettled` `invalidateQueries` refetch (plus the
  deferred SSE-invalidation queue), so focus can land on a node about to be
  unmounted, or on `<body>`. The DOM-`querySelector` approach compounds the
  fragility.

- **Interaction tests pinned at the wrong level** (flagged by: Test Coverage,
  with Correctness/Usability echoes) — A2 (drag-then-click), A3 (mid-drag source
  persistence), C1 (keyboard cross-column move) and C3 (focus return) are
  specified as jsdom unit tests, but dnd-kit's PointerSensor/KeyboardSensor and
  `sortableKeyboardCoordinates` depend on real layout geometry and pointer
  sequences jsdom cannot reproduce. Today's `KanbanBoard.test.tsx` never drives a
  drag-end at all.

- **A2 click-suppression mechanism undecided and timing-sensitive** (flagged by:
  Code Quality, Correctness, Usability) — the plan offers two ownership models
  ("board passes a `dragJustEnded` signal down, or the card owns a local ref")
  without choosing, and the "cleared a tick after `onDragEnd`" timing is asserted
  rather than proven against dnd-kit's synthetic-click dispatch.

- **Visual-regression baseline portability** (flagged by: Portability, Test
  Coverage) — the net-new drag-state baseline freezes OS-dependent rendering
  (sub-pixel `rotate(1.5deg) scale(1.02)`, `opacity: 0.6` compositing,
  antialiased rotated text) into per-platform PNGs, and the linux-via-
  `workflow_dispatch` generation path inherits the known hazard that a
  GITHUB_TOKEN-pushed baseline commit does not re-trigger Main CI.

### Tradeoff Analysis

- **Prototype parity vs. user-facing copy quality**: The success-toast body
  `PATCH \`…/frontmatter\` → 204 · fresh ETag received` is lifted verbatim from
  the prototype, which is a developer-facing design simulation. Usability and
  Architecture both flag this as raw transport jargon inconsistent with the only
  other production toast (`use-external-edit-toast`, plain prose). Recommendation:
  decide explicitly whether the board's audience is technical; if not, replace
  with plain confirmation copy (the heading already carries the message) or
  source the status/ETag fragment from the `PatchResult` the mutation returns
  rather than hard-coding it.

- **Toast assertiveness vs. consistency**: Unifying all feedback into one toast
  surface is a usability win for consistency, but errors specifically warrant an
  assertive live region and longer persistence. Recommendation: make politeness
  and auto-dismiss `kind`-aware (`error` → `role="alert"`/`aria-live="assertive"`,
  no/longer auto-dismiss) rather than abandoning the unified surface.

### Findings

#### Critical

None.

#### Major

- 🟡 **Architecture / Code Quality / Correctness / Standards**: Duplicate
  drag-in-progress state across three mechanisms
  **Location**: Phase 4 §1 (vs. Phase 3 §1 and existing `setDragInProgress`)
  Phase 4 introduces a new card-local click-suppression flag while the board
  already owns `docEvents.setDragInProgress` (gates SSE invalidation, cleared
  synchronously in `handleDragEnd`) and Phase 3 adds `activeId`. Pick one
  ownership model, state that the click guard is a deliberately separate
  card-local flag whose clear is one tick later than `setDragInProgress(false)`,
  and justify the separation in the plan.

- 🟡 **Code Quality / Correctness / Usability**: A2 click-suppression design
  undecided and its timing is asserted, not guaranteed
  **Location**: Phase 4 §1
  The plan offers two implementations without choosing, and the "cleared a tick
  after `onDragEnd`" guard ordering relative to dnd-kit's synthetic click is
  frame-rate-dependent: an rAF clear can run *before* a coalesced synthetic click
  on a slow frame, re-introducing the unwanted navigation. Commit to the
  card-local ref, clear it on a provably-later boundary (next `pointerdown` or
  `setTimeout(…,0)`, not rAF), and key suppression off whether a real drag
  actually started (distance threshold crossed).

- 🟡 **Architecture / Correctness / Code Quality / Usability**: Focus
  restoration competes with `onSettled` invalidation / async re-renders
  **Location**: Phase 5 §1
  A single rAF does not out-wait `onMutate`'s optimistic `setQueryData`,
  `onSettled`'s `invalidateQueries` refetch (use-move-work-item.ts:44-46), and
  the DocEvents deferred-invalidation flush on `setDragInProgress(false)`. Focus
  can resolve to `null` or a stale node. Tie restoration to a render-completion
  signal (effect keyed on entries-list identity, or poll across rAFs until the
  anchor exists) and prefer an exposed ref/focus handle over
  `querySelector('[data-relpath="…"] a')`.

- 🟡 **Correctness**: Toast emission and focus restoration must be scoped to the
  `move` outcome branch only
  **Location**: Phase 2 §1 / Phase 5 §1
  `outcome` carries `toStatus` only in the `move` case; the three no-op branches
  (same-column, other-rejected, unknown) must continue to raise no toast and
  cause no focus churn — otherwise a same-column drop raises a spurious success
  toast and steals focus, contradicting "Release on same column: no toast, no
  move." State the scoping explicitly and add a same-column-drop test asserting
  zero `showToast` calls.

- 🟡 **Architecture / Correctness / Code Quality**: Overlay non-null assertion
  crashes on concurrent external delete
  **Location**: Phase 3 §1
  `entriesByRelPath.get(activeId)!` throws if an SSE-driven delete removes the
  in-flight card mid-drag (the board is live; `setDragInProgress(false)` flushes
  queued invalidations on drop). Guard with a real null check
  (`const active = …get(activeId); return active ? <WorkItemCard entry={active}
  overlay/> : null`) and add a test where the active entry disappears mid-drag.

- 🟡 **Architecture / Code Quality**: `WorkItemCard` asked to serve two
  structural roles without a clean seam
  **Location**: Phase 3 §1–§2 (+ Phase 4)
  The `overlay?: boolean` flag forces one component to be both an `<li>`
  sortable `<Link>` (with `useSortable` called unconditionally) and a
  presentational clone (no `<li>`, no sortable, no navigation). Extract a shared
  `WorkItemCardBody`/`WorkItemCardPresentation` consumed by both a sortable
  `WorkItemCard` and a thin overlay component, keeping the sortable hook
  unconditional and each entry point single-purpose (0040 also shares this file).

- 🟡 **Code Quality**: Phase 2 reuses the module-private `describe()` helper
  **Location**: Phase 2 §1
  `describe` in `announcements.ts:21` is module-private with signature
  `describe(id, entries)`; the board has no equivalent. Extract a shared exported
  card-naming helper consumed by both `announcements.ts` and the board toast, or
  the toast heading and the SR announcements will drift. Name the extraction as
  an explicit Phase 2 step.

- 🟡 **Usability / Architecture**: Success-toast body is raw developer jargon
  **Location**: Phase 2 §1
  `PATCH \`…/frontmatter\` → 204 · fresh ETag received` is HTTP/ETag jargon lifted
  from a developer-facing prototype, inconsistent with the plain-prose
  `use-external-edit-toast` body. Replace with plain confirmation copy (or drop
  the body), or — if a technical line is genuinely wanted — confirm the audience
  is technical and source the status/ETag from the mutation's `PatchResult`
  rather than a hard-coded constant.

- 🟡 **Usability / Standards / Architecture / Correctness**: Error toasts inherit
  polite live region and 5s auto-dismiss
  **Location**: Phase 1 §2 / Phase 2 §2
  Removing the `role="alert"` banner (30s) and routing failures through
  `role="status" aria-live="polite"` (5s) under-announces a reverted move (WCAG
  4.1.3) and shrinks the failure-notice window 6×. Make politeness and dismissal
  `kind`-aware: `error` → assertive `role="alert"`, no/longer auto-dismiss; add a
  test asserting the error variant is announced assertively.

- 🟡 **Test Coverage**: A2 drag-then-click regression specified at jsdom level
  **Location**: Phase 4, Test-first additions
  dnd-kit's PointerSensor + the 5px threshold + synthetic post-drag click are not
  faithfully reproducible in jsdom (the E2E suite uses a multi-step `page.mouse`
  `dndDrag` helper precisely for this). Make the authoritative A2 test an E2E
  test (drag → assert URL unchanged; click → assert navigation); treat any unit
  test as a supplementary check on the handler.

- 🟡 **Test Coverage**: A3 mid-drag source-persistence has no stated jsdom harness
  **Location**: Phase 3, Test-first additions
  Entering the active-drag state needs real sensor activation. Without a stated
  mechanism the assertion risks degrading to rendering `WorkItemCard
  overlay={true}` directly (prop wiring, not the integrated A3 behaviour).
  Specify an E2E mid-drag DOM assertion for the integrated behaviour plus focused
  unit tests for the `isDragging`/`overlay` styling branches as separate
  concerns.

- 🟡 **Test Coverage**: C1 keyboard cross-column move cannot complete in jsdom
  **Location**: Phase 5 §2
  `sortableKeyboardCoordinates` computes geometry from real layout boxes jsdom
  lacks, so a jsdom keyboard-move commonly resolves to zero coordinates and
  produces a passing-but-vacuous "recorded verification." Commit C1 to an E2E
  test using `page.keyboard` and assert the card landed in the target column.

- 🟡 **Test Coverage**: C3 focus-return test is timing-sensitive and may be
  always-fail/flaky
  **Location**: Phase 5, Success Criteria
  A naive `expect(document.activeElement).toBe(link)` immediately after the move
  is flaky (rAF not flushed) or always-fails (focus lost on re-render). Specify
  deterministic flushing (fake-timer/rAF advance or `waitFor` on activeElement),
  assert the anchor in the correct resting column for both success and revert,
  and consider an E2E focus assertion as the authoritative oracle.

- 🟡 **Portability**: Drag-state baseline freezes OS-dependent
  rotation/opacity/antialiasing into per-platform PNGs
  **Location**: Phase 3 §5
  Sub-pixel affine transforms, opacity compositing, and antialiased rotated text
  are exactly what diverges between the darwin and linux rasterizers; existing
  baselines are static and axis-aligned, so this is a step-change in platform
  sensitivity that can drift past `maxDiffPixelRatio`. Constrain the capture to
  the card element (as `glyph-showcase.spec.ts` does), and consider asserting the
  transform/opacity via resolved-style probes (`*-resolved-*.spec.ts` pattern)
  instead of a cross-platform pixel oracle.

- 🟡 **Portability**: Linux baseline generation depends on a workflow whose commit
  does not re-trigger CI
  **Location**: Phase 3 §5
  Per recorded project context, linux baselines drift behind darwin and a
  GITHUB_TOKEN-pushed baseline commit does not re-run Main CI, so a fresh CI run
  may lack/stale the `drag-…-linux.png` oracle while darwin is present. Make the
  success criteria explicit that the linux baseline must exist and the spec must
  pass on the linux CI runner before merge, and document the manual re-trigger.

#### Minor

- 🔵 **Correctness**: `onDragCancel` must mirror the full `onDragEnd` teardown
  **Location**: Phase 3 §1 / Phase 5
  The DndContext wires only `onDragStart`/`onDragEnd` today. Adding
  `onDragCancel` solely to clear `activeId` would leave `setDragInProgress(true)`
  stuck on the Escape-cancel path, permanently queuing SSE invalidations and
  freezing live updates. Specify that `onDragCancel` also calls
  `setDragInProgress(false)` and resets announcement timers; add a cancel-path
  test.

- 🔵 **Correctness / Architecture**: Error-notice window shrinks from 30s to 5s
  **Location**: Phase 2 §2
  Not a logic error, but combined with focus landing on the reverted source card,
  a user who looks away may not realise the move failed. Confirm 5s is acceptable
  for `error`, or allow a longer auto-dismiss for that kind (covered by the
  cross-cutting error-feedback theme).

- 🔵 **Architecture**: Toast body couples board presentation to HTTP/ETag
  transport details
  **Location**: Phase 2 §1
  Source the status/ETag fragment from the mutation's `PatchResult` rather than a
  duplicated literal so the body reflects actual transport facts.

- 🔵 **Architecture**: dnd-kit retention recorded only in plan prose, not a
  durable decision record
  **Location**: Resolved decisions #5 / What We're NOT Doing
  A load-bearing dependency choice (owns keyboard a11y + rollback) lives only in
  one story's plan. Reconsider a short ADR or decisions-log entry, or note why
  prose is sufficient.

- 🔵 **Code Quality**: Reused `conflictMessageFor` copy embeds banner-specific
  wording in a toast
  **Location**: Phase 2 §1
  The copy says "the card has been returned to its original column" — written for
  the persistent banner. Review the strings for the transient-toast context or
  note they are intentionally kept.

- 🔵 **Code Quality / Usability**: `WorkItemCard` accumulating flag-like props /
  dual-mode rendering
  **Location**: Phase 3 §2 / Phase 4
  Subsumed by the "two structural roles" major; the presentational-extraction fix
  addresses both.

- 🔵 **Test Coverage**: Success-path label assertion is loose
  **Location**: Phase 2, Test-first additions
  A `toMatch(/204/)` on the static body passes even if
  `columns.find(c => c.key === outcome.toStatus)?.label` silently returns
  `undefined`. Assert the heading contains the exact human label (resolved
  decision #3), so a key-vs-label regression fails.

- 🔵 **Test Coverage**: Error-path per-type message not asserted
  **Location**: Phase 2, Test-first additions
  Cover `ConflictError` (412) and `FetchError` as separate cases asserting their
  respective `conflictMessageFor` copy, so collapsing/swapping the branches
  fails.

- 🔵 **Test Coverage**: Wrapping-label config does not assert the label actually
  wraps
  **Location**: Phase 6 §2
  Add an explicit assertion that the ≥30-char label renders across ≥2 lines
  (client height exceeds single-line height / non-ellipsis), not just that
  behaviour holds.

- 🔵 **Test Coverage**: Drag-state baseline capture frame is non-deterministic
  **Location**: Phase 3 §5
  Specify transition/animation disabling (or an explicit settle wait) before the
  screenshot so the baseline captures a deterministic frame (the 140ms transition
  may still be settling).

- 🔵 **Standards**: ARIA treatment of the new empty-column panel is unspecified
  **Location**: Phase 3 §4
  The current placeholder is `aria-hidden="true"` (count is exposed via the
  header `aria-label`). State whether the new two-line panel stays `aria-hidden`
  to avoid redundant/contradictory announcements; assert it in the unit test.

- 🔵 **Usability**: Variant meaning conveyed only visually, not to assistive tech
  **Location**: Phase 1 §2
  The icon is `aria-hidden`; severity is colour/icon-only. Add a visually-hidden
  severity prefix keyed off `kind` (sr-only "Error:"/"Success:") so the variant
  is perceivable independent of colour.

- 🔵 **Usability**: Empty-column copy is pointer-centric
  **Location**: Phase 3 §4
  "Drop a work item…" describes a pointer-only gesture; keyboard users pick
  up/place. Consider mechanism-neutral wording ("Move a work item here…") or
  ensure the keyboard affordance is discoverable elsewhere.

- 🔵 **Portability**: Wrapping-label assertion depends on platform font metrics
  **Location**: Phase 6 §2 / Desired End State
  Line-break position depends on resolved glyph widths (Google-Fonts stack, not
  self-hosted). Choose a length comfortably beyond one line, assert wrapping
  behaviourally, and confirm the font stack is deterministically available on CI.

- 🔵 **Portability**: Cursor `grab`/`grabbing` parity is OS-rendered, not
  screenshot-capturable
  **Location**: Phase 3 §3 / Phase 6
  Verify the cursor via computed CSS (`getComputedStyle(card).cursor`) rather than
  the pixel baseline, and note in the convergence record that cursor appearance
  is OS-delegated.

#### Suggestions

- 🔵 **Standards**: `.cardDragging` switches the card border to the `--ac-accent`
  token family (the resting border uses `--ac-stroke-soft`). Acceptable given the
  prototype's explicit accent-border-on-drag intent — just confirm it is the
  deliberate prototype value so the token-family switch reads as intentional.

### Strengths

- ✅ Strong respect for existing module boundaries: explicitly does not rebuild
  drag-and-drop, the PATCH write-back, optimistic update/rollback, or column
  config (ADR-0024), confining changes to a tight surface.
- ✅ The Toaster variant change is genuinely additive and open-closed: optional
  `kind` defaulting to `info` plus a `--toast-accent` custom-property indirection
  keeps the single existing caller byte-identical.
- ✅ Correctly diagnoses A1/A3 as a single rendering-layer root cause (missing
  `DragOverlay`) rather than a data-model bug, avoiding an over-engineered state
  change; membership already defers to drop.
- ✅ Phase sequencing follows dependency direction (variants → board loop →
  rendering → click separation → a11y → convergence), each phase independently
  mergeable and shippable.
- ✅ Unusually test-conscious: every phase has explicit test-first additions and
  an automated/manual split; A1's oracle is correctly a checked-in
  visual-regression baseline, not an eyeball comparison.
- ✅ C2 mandates asserting the four `announcements.ts` strings verbatim (the
  strings in the plan match `announcements.ts:35-48` exactly), and C3 correctly
  fixes the focus target from the non-focusable `<li>` to the `<Link>` anchor.
- ✅ Reuses already-themed, contrast-tested `--ac-ok`/`--ac-err` tokens; the
  border/icon uses are already covered by WCAG 1.4.11 (≥3:1) tests.
- ✅ The error-path Phase 2 test asserts a multi-condition oracle (error toast +
  optimistic rollback + absence of the old banner).

### Recommended Changes

1. **Unify drag state and commit the A2 mechanism** (addresses: Duplicate
   drag-in-progress state; A2 design undecided; A2 timing). In Phase 4, choose
   the card-local ref, document its relationship to `setDragInProgress` and
   `activeId`, clear the guard on a provably-later boundary than the synthetic
   click (not rAF), and key suppression off a real drag having started.

2. **Make error feedback assertive and persistent** (addresses: error toasts
   polite/5s; role=alert removal; error-notice window). Make the Toaster's live
   region and auto-dismiss `kind`-aware: `error` → `role="alert"`/`aria-live=
   "assertive"` and no/longer dismiss; add a test asserting assertive
   announcement. Note the contract change in the plan.

3. **Harden focus restoration against async re-renders** (addresses: focus vs
   `onSettled`; rAF fragility; DOM-query fragility; C3 test). Tie restoration to a
   render-completion signal (effect keyed on entries identity / poll across rAFs),
   prefer an exposed focus ref over `querySelector`, and scope toast+focus to the
   `move` branch only (with a same-column no-op test).

4. **Re-level the interaction tests to E2E** (addresses: A2/A3/C1/C3 jsdom).
   Specify E2E as the authoritative level for drag-then-click (A2), mid-drag
   source persistence (A3), keyboard cross-column move (C1), and focus return
   (C3), using the existing `dndDrag`/`page.keyboard` helpers; keep unit tests as
   supplementary checks on isolated handlers/styling branches.

5. **Extract shared helpers/components** (addresses: `describe()` private;
   WorkItemCard two roles; overlay non-null assertion). Export a shared
   card-naming helper for both announcements and the toast; extract a
   presentational card body shared by the sortable card and the overlay clone
   (keeping `useSortable` unconditional); guard the overlay lookup with a real
   null check.

6. **Tighten the success-toast copy/coupling** (addresses: raw jargon body;
   transport coupling; loose label assertion). Decide the audience; replace or
   mute the technical body or source status/ETag from `PatchResult`; assert the
   exact human target label in the success test.

7. **De-risk the visual-regression baseline cross-platform** (addresses: baseline
   OS-dependence; linux workflow CI; capture determinism; cursor; wrapping).
   Constrain the capture to the card element or assert transform/opacity via
   resolved-style probes; disable transitions before capture; require the linux
   baseline to exist and pass on the linux runner before merge; verify cursor via
   computed CSS and the wrapping-label behaviourally.

8. **Specify `onDragCancel` teardown and empty-column ARIA** (addresses: cancel
   path freezes SSE; empty-column ARIA). Have `onDragCancel` mirror the full
   `onDragEnd` teardown; state whether the new empty-column panel stays
   `aria-hidden`.

9. **(Optional) Capture the dnd-kit retention as an ADR/decisions entry**
   (addresses: prose-only library decision), or note why prose suffices.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: A well-scoped defect-and-polish plan that respects existing module
boundaries: it reuses the shipped DnD machinery, the ETag write-back path, and
the optimistic-rollback hook rather than rebuilding them, and the Toaster variant
change is genuinely additive via a CSS custom-property indirection. The phasing
is sound and each phase independently mergeable. The main architectural risks are
localized: an unacknowledged second drag-in-progress signal that already exists on
the DocEvents handle, the WorkItemCard component serving two structural roles
without a clean seam, and an unexamined timing/ownership interaction between the
new focus-restoration and the existing deferred-SSE-invalidation queue.

**Strengths**:
- Strong respect for existing module boundaries (no rebuild of DnD, PATCH,
  rollback, ADR-0024 column config).
- Toaster variant change is additive and open-closed via optional `kind` + a
  `--toast-accent` indirection, keeping the single existing caller byte-identical.
- Phase sequencing follows dependency direction cleanly; each phase shippable.
- Correctly diagnoses A1/A3 as a single rendering-layer root cause, avoiding an
  over-engineered state change.
- Reuses already-themed, contrast-tested `--ac-ok`/`--ac-err` tokens.

**Findings**:
- 🟡 (major, high) New card-local drag flag duplicates an existing board-level
  drag-in-progress signal — Phase 4 §1. The board already calls
  `docEvents.setDragInProgress(true/false)` (KanbanBoard.tsx:97,105), gating a
  deferred SSE-invalidation queue. Two independent representations of "a drag is
  happening" invite drift. Decide on a single source or document why the click
  suppression is intentionally card-local.
- 🟡 (major, high) WorkItemCard asked to serve two structural roles without a
  clean seam — Phase 3 §1–§2. The `overlay` clone must not be an `<li>`, must not
  call `useSortable`, must not be a `<Link>`; conditionally bypassing the hook
  risks rules-of-hooks violations. Extract a shared `WorkItemCardBody`.
- 🟡 (major, medium) Focus restoration competes with the existing `onSettled`
  query invalidation / deferred-SSE re-render — Phase 5 §1. A single rAF is not a
  robust ordering guarantee against async invalidation; focus may land on a
  soon-unmounted node. Tie focus to actual settled/re-rendered state.
- 🔵 (minor, high) Failure-surfacing contract shifts from assertive `role=alert`
  to polite `role=status` — Phase 2 §1–§2. Acknowledge the
  assertiveness/persistence tradeoff for a data-loss-adjacent event.
- 🔵 (minor, medium) Toast body couples board presentation to HTTP/ETag transport
  details — Phase 2 §1. Source the status/ETag fragment from the `PatchResult`.
- 🔵 (minor, medium) Library-retention decision recorded only in plan prose, not a
  durable architectural record — Resolved decisions #5. Reconsider a short ADR.

### Code Quality

**Summary**: A well-scoped plan that builds additively on mature, well-structured
code and correctly avoids rebuilding shipped machinery. The Toaster variant work
is cleanly designed via CSS custom-property indirection, and most phases reuse
existing helpers. The main maintainability concerns are around the A2
click-suppression design (Phase 4 leaves the implementation undecided and
introduces a second "drag in progress" notion that overlaps existing state),
reuse of the module-private `describe` helper across files, and message text that
embeds banner-specific copy now reused in toasts.

**Strengths**:
- Phase 1's `--toast-accent` indirection is a clean, DRY variant mechanism that
  keeps `info` byte-identical.
- Honest scope with an explicit "What We're NOT Doing", keeping phases small and
  independently mergeable.
- Strong reuse of existing infrastructure (rollback in `useMoveWorkItem.onError`,
  existing tokens, `kind` defaulting to `info`).
- Each phase leaves the board shippable and sequences dependencies forward.

**Findings**:
- 🟡 (major, high) A2 click-suppression design left undecided between two
  implementations — Phase 4. Commit to the card-local approach and state it.
- 🟡 (major, high) New drag flag overlaps the existing `setDragInProgress` notion
  — Phase 4 vs KanbanBoard.tsx:97,105. Three overlapping drag-state
  representations; acknowledge and justify or derive from `isDragging` + one ref.
- 🟡 (major, high) Phase 2 reuses the module-private `describe()` helper —
  Phase 2 §1. `describe` (announcements.ts:21) is private with a different
  signature; extract a shared exported helper to avoid card-naming drift.
- 🔵 (minor, medium) Reused error copy embeds banner-specific wording in a toast —
  Phase 2 §1 (`conflictMessageFor`, KanbanBoard.tsx:34). Review for the toast
  context.
- 🔵 (minor, medium) WorkItemCard accumulating flag-like props and dual-mode
  rendering — Phase 3 §2 / Phase 4. Prefer a shared presentational component.
- 🔵 (minor, medium) Non-null assertion on overlay entry lookup hides a latent
  failure mode — Phase 3 §1. Guard the lookup instead of `!`.
- 🔵 (minor, medium) Focus restoration via DOM query and CSS.escape adds fragile
  imperative coupling — Phase 5 §1. Prefer an exposed ref/focus target.

### Test Coverage

**Summary**: Unusually test-conscious for a frontend polish pass: every phase has
explicit test-first additions, automated/manual splits, and the A1 oracle is
correctly a visual-regression baseline. The main risk is that the most
defect-prone behaviours (A2, A3, C1, C3) are specified as jsdom unit tests, where
dnd-kit's sensors and the synthetic post-drag click are hard or impossible to
exercise faithfully — and today's `KanbanBoard.test.tsx` never drives a drag-end
(only SSE-driven moves). Several assertions are also specified loosely enough that
a mutation could survive.

**Strengths**:
- A1's oracle is a checked-in baseline derived from the prototype, with correct
  darwin/linux regen guidance.
- C2 mandates verbatim `announcements.ts` strings; an existing
  `announcements.test.ts` proves the pattern.
- Phase 1 preserves backward compatibility and updates the one intentionally
  locked CSS test.
- Test-first ordering keeps each phase's tests meaningful in isolation.
- The Phase 2 error-path test is a thorough multi-condition assertion.

**Findings**:
- 🟡 (major, medium) A2 regression test specified in jsdom — Phase 4. dnd-kit's
  PointerSensor/synthetic click are not faithfully reproducible in jsdom; make
  the authoritative test E2E via `dndDrag`.
- 🟡 (major, medium) A3 active-drag assertion has no stated jsdom harness —
  Phase 3. Risks degrading to direct `overlay={true}` rendering; specify E2E
  mid-drag assertion plus focused unit tests for styling branches.
- 🟡 (major, medium) C1 keyboard move cannot complete in jsdom — Phase 5.
  `sortableKeyboardCoordinates` needs real layout; commit C1 to E2E.
- 🟡 (major, medium) C3 focus-return test timing-sensitive — Phase 5. Specify
  deterministic flushing and assert the resting-column anchor for both branches.
- 🔵 (minor, high) Success-path label assertion loose — Phase 2. Assert the exact
  human label so a key-vs-label regression fails.
- 🔵 (minor, medium) Cross-config wrapping-label not asserted to wrap — Phase 6.
  Assert ≥2 lines explicitly.
- 🔵 (minor, medium) Error-path per-type message not asserted — Phase 2. Cover
  ConflictError (412) and FetchError separately.
- 🔵 (minor, low) Drag-state baseline capture frame non-deterministic — Phase 3.
  Disable transitions / settle before capture.

### Correctness

**Summary**: Built on an accurate, well-traced reading of the existing code; most
claims verify against source. The principal correctness risks cluster in Phase 4
(the drag-vs-click suppression flag) and Phase 5 (rAF focus restore), where the
timing relationships between dnd-kit's synthetic click, React re-renders, the
optimistic-update-then-invalidate cycle, and the existing SSE drag-gating flag are
under-specified and can leave the click guard armed or the focus query targeting a
stale/absent node. The same-column no-op and rollback paths are correctly
identified but the focus/toast wiring does not fully cover every outcome branch.

**Strengths**:
- Correctly identifies A3 as a rendering artefact, not a data-model bug (verified
  against KanbanBoard.tsx:104-139 and resolve-drop-outcome.ts).
- Correctly understands optimistic update/rollback (use-move-work-item.ts:40-45).
- Toast-variant change correctly characterised as additive, preserving the single
  caller.
- The four announcement strings match announcements.ts:35-48 exactly.

**Findings**:
- 🟡 (major, high; body marked 🔴) A2 click-suppression flag clear ordering vs the
  synthetic click is asserted, not guaranteed — Phase 4. An rAF clear can run
  before a coalesced synthetic click; specify a provably-later boundary and test
  through the real pointer sequence.
- 🟡 (major, high) Focus restore single rAF does not out-wait optimistic
  `setQueryData` + `onSettled` invalidate refetch — Phase 5 §1. Tie restoration to
  a render-completion signal; test after invalidation resolves.
- 🟡 (major, medium) Toast/focus must be scoped strictly to `outcome.kind ===
  'move'` — Phase 2 §1. The three no-op branches must raise no toast / no focus
  churn; add a same-column-drop test asserting zero `showToast` calls.
- 🟡 (major, medium) Overlay non-null assertion crashes on concurrent external
  delete mid-drag — Phase 3 §1. Guard with a real null check; test the
  entry-disappears scenario.
- 🔵 (minor, medium) `onDragCancel` must mirror full `onDragEnd` teardown —
  Phase 3 §1/Phase 5. Escape-cancel would otherwise leave `setDragInProgress`
  stuck true, freezing live updates; add a cancel-path test.
- 🔵 (minor, medium) Click-guard vs `setDragInProgress` conflation — Phase 4. Pick
  one ownership model; the click guard's clear is deliberately one tick later.
- 🔵 (minor, high) Error toast 5s auto-dismiss vs the removed banner's 30s —
  Phase 2 §2. Confirm 5s acceptable for `error` or allow a longer timeout.

### Usability

**Summary**: Well-scoped, research-grounded, with sensible UX choices (human
column labels over raw keys, prototype-matched drag affordance, removing the
inline banner for a consistent toast loop, keyboard focus return to the focusable
anchor). The main concerns are user-facing copy and feedback semantics: the
success-toast body is raw developer jargon inconsistent with the existing toast
voice, error toasts share the success toast's polite/5s behaviour so failures may
be under-announced and vanish before they can be read, and drag-state semantics
are conveyed visually but not to assistive tech beyond the announcement strings.

**Strengths**:
- Resolved decision #3 prefers the human column label over the raw key — correct
  for a label-driven board.
- C3 fixes the focus target to the focusable `<Link>` anchor and extends focus
  return to the success path.
- Empty-column copy becomes a helpful, action-oriented static two-line panel.
- Replacing the inline banner with the toast loop unifies move feedback.
- The variant change is additive (optional `kind` defaulting to `info`).

**Findings**:
- 🟡 (major, high) Success toast body is raw developer jargon, not user-facing
  copy — Phase 2 §1. Inconsistent with `use-external-edit-toast`'s plain prose;
  replace/mute or confirm a technical audience.
- 🟡 (major, high) Error toasts inherit polite live region and 5s auto-dismiss —
  Phase 1 §2 / Phase 2. Give error toasts `role="alert"`/`aria-live="assertive"`
  and persistence.
- 🔵 (minor, medium) Toast variant meaning conveyed only visually, not to
  assistive tech — Phase 1 §2. Add a visually-hidden severity prefix keyed off
  `kind`.
- 🔵 (minor, medium) Click-suppression timing flag is a fragile ergonomic boundary
  — Phase 4. Key suppression off a real drag actually starting.
- 🔵 (minor, medium) Empty-column drop instruction is pointer-centric — Phase 3
  §4. Consider mechanism-neutral wording.
- 🔵 (minor, low) Focus return relies on a DOM query after re-render rather than a
  stable handle — Phase 5 §1. Prefer a React ref/imperative focus handle.

### Standards

**Summary**: Strongly aligned with the project's frontend conventions: CSS-module
+ design-token styling, the `data-kind` + `--toast-accent` indirection, backtick
inline-code toast bodies, and the existing visual-regression workflow are all
idiomatic, and the chosen `--ac-ok`/`--ac-err` tokens are already covered by WCAG
1.4.11 (≥3:1) contrast tests for the border/icon uses proposed. The most
significant gap is an ARIA live-region regression: routing error toasts through
the Toaster's polite region while deleting the `role="alert"` banner downgrades
how assertively a failed/412 move is announced. Smaller items: a duplicate
drag-in-progress flag and unspecified ARIA treatment of the new empty-column copy.

**Strengths**:
- Toast variants follow the CSS-module + token idiom exactly; `info` stays
  byte-identical.
- `--ac-ok`/`--ac-err` used only for the 3px border-left and icon currentColor,
  both already contrast-tested for light and dark.
- Success-toast body reuses the established backtick inline-code convention.
- Phase 5 fixes the focus target to the `<Link>` anchor (WCAG 2.4.3).
- Optional `kind` defaulting to `info` preserves the single caller and its tests.
- Baseline approach matches the documented process.

**Findings**:
- 🟡 (major, high) Error toasts downgrade from assertive to polite live-region
  announcement — Phase 1 §2 / Phase 2 §2. Make the live region `kind`-aware
  (`error` assertive); add a test.
- 🔵 (minor, medium) New drag flag duplicates the existing `setDragInProgress`
  mechanism — Phase 4 §1. Reuse/extend it or document the deliberate departure.
- 🔵 (minor, medium) ARIA treatment of the new empty-column panel unspecified —
  Phase 3 §4. State whether it stays `aria-hidden`; assert in the unit test.
- 🔵 (suggestion, medium) `.cardDragging` hard-codes `--ac-accent` rather than the
  card's usual `--ac-stroke-soft` border token — Phase 3 §3. Confirm it is the
  deliberate prototype value.

### Portability

**Summary**: Overwhelmingly a frontend interaction-polish pass with no
infrastructure, deployment, or vendor-coupling surface; dnd-kit and TanStack
Router are existing in-repo dependencies, not new lock-in. The one material
concern is the net-new drag-state visual-regression baseline, which freezes
OS-dependent rendering of a transient interaction (sub-pixel rotation/scale,
opacity compositing, antialiased rotated text, and a ≥30-char wrapping label whose
line geometry depends on platform font metrics) into per-platform PNGs. The
darwin-local + linux-via-`workflow_dispatch` path inherits a known hazard
(GITHUB_TOKEN-pushed baseline commits do not re-trigger Main CI), threatening
cross-platform CI determinism rather than runtime portability.

**Strengths**:
- No new vendor/cloud coupling; reuses already-committed dependencies.
- Configuration stays externalised (runtime `GET /api/kanban/config`, runtime CSS
  tokens).
- The plan already acknowledges the darwin/linux baseline split and cites project
  memory on linux drift.

**Findings**:
- 🟡 (major, high; body marked 🔴) Drag-state baseline freezes OS-dependent
  rotation/opacity/antialiasing into per-platform PNGs — Phase 3 §5. Constrain to
  element-level capture or assert via resolved-style probes.
- 🟡 (major, high) Linux baseline generation depends on a workflow whose commit
  does not re-trigger CI — Phase 3 §5. Require the linux baseline to exist and
  pass on the linux runner before merge; document the manual re-trigger.
- 🔵 (minor, medium) Wrapping-label assertion depends on platform font metrics —
  Phase 6 §2. Choose a length comfortably beyond one line, assert behaviourally,
  confirm font availability on CI.
- 🔵 (minor, medium) Cursor `grab`/`grabbing` parity is OS-rendered, not
  screenshot-capturable — Phase 3 §3. Verify via computed CSS; note OS-delegation
  in the convergence record.

## Re-Review (Pass 2) — 2026-06-06T17:51:46+00:00

**Verdict:** REVISE

The revision is a clear, substantial improvement: **all 38 findings from the
initial review are resolved or deliberately accepted.** Every prior major was
addressed at the specification level — the A2 mechanism is committed and its
timing fixed, focus restoration is tied to render-completion, the overlay is
null-guarded, `onDragCancel` is added, `WorkItemCardPresentation` and
`describeEntry` are extracted, toast copy is plain, error toasts are assertive +
persistent, the interaction tests are re-levelled to E2E, and the visual baseline
is constrained and CI-gated. However, the re-review surfaced **9 new major**
second-order issues introduced by the edits, clustering in two areas: (a) the
kind-aware two-region toast change is more invasive than the plan captures
(eviction/ordering, locked-test breakage, sr-only utility, live-region
multiplication), and (b) several test-level claims assume harness capabilities
that do not exist (the `dndDrag` helper can't pause mid-drag and isn't shared,
`page.keyboard` has no precedent, the banner removal breaks an unlisted existing
spec, and some Phase 2 "unit" tests can't reach `handleDragEnd`). These are worth
one more focused pass before implementation.

### Previously Identified Issues

**Architecture** — all resolved:
- 🟡 Duplicate drag-in-progress state — **Resolved** (Phase 4 names all three
  signals and justifies the card-local ref).
- 🟡 WorkItemCard two structural roles — **Resolved** (`WorkItemCardPresentation`
  extraction, `useSortable` unconditional).
- 🟡 Focus vs `onSettled` invalidation — **Resolved** (render-completion approach).
- 🔵 role=alert→role=status contract shift — **Resolved** (assertive region).
- 🔵 Toast body transport coupling — **Resolved** (plain copy + `errorToastMessageFor`).
- 🔵 dnd-kit ADR prose-only — **Resolved** (accepted by author; not re-raised).

**Code Quality** — all resolved:
- 🟡 A2 design undecided — **Resolved** (committed to card-local ref).
- 🟡 Drag flag overlap — **Resolved** ("Relationship to existing state" note).
- 🟡 Module-private `describe()` reuse — **Resolved** (`describeEntry` §1a).
- 🔵 Banner-specific error copy — **Resolved** (`errorToastMessageFor`).
- 🔵 WorkItemCard dual-mode props — **Resolved** (presentation extraction).
- 🔵 Overlay non-null assertion — **Resolved** (real null check).
- 🔵 Focus DOM-query fragility — **Resolved** (exposed focus ref).

**Test Coverage** — all resolved at the specification level:
- 🟡 A2 / 🟡 A3 / 🟡 C1 / 🟡 C3 jsdom-level — **Resolved** (re-levelled to E2E).
- 🔵 Loose label assertion — **Resolved** (exact label). 🔵 Per-type error message
  — **Resolved** (ConflictError vs FetchError split). 🔵 Wrapping not asserted —
  **Resolved** (behavioural ≥2 lines). 🔵 Baseline capture frame — **Resolved**
  (transitions disabled). *(But see new harness-reality findings below.)*

**Correctness** — all resolved:
- 🟡 A2 clear ordering — **Resolved** (macrotask/next-pointerdown, not rAF).
- 🟡 Focus rAF vs async re-render — **Resolved** (render-completion after settle).
- 🟡 Toast/focus scope — **Resolved** (gated on `kind === 'move'` + no-op test).
- 🟡 Overlay crash — **Resolved** (null guard).
- 🔵 `onDragCancel` teardown — **Resolved** (handler added). 🔵 Guard vs
  `setDragInProgress` — **Resolved** (documented distinct). 🔵 5s vs 30s window —
  **Resolved** (error persists).

**Usability** — all resolved:
- 🟡 Jargon body — **Resolved** (plain copy). 🟡 Error polite/5s — **Resolved**
  (assertive + persistent). 🔵 Variant visual-only — **Resolved** (sr prefix). 🔵
  Click timing — **Resolved** (keys off real drag). 🔵 Pointer-centric copy —
  **Resolved** ("Move a work item here"). 🔵 Focus DOM query — **Resolved** (ref).

**Standards** — resolved:
- 🟡 Error assertiveness — **Resolved** (kind-aware region). 🔵 Drag flag dup —
  **Resolved**. 🔵 Empty-column ARIA — **Resolved** (`aria-hidden` specified). 🔵
  `.cardDragging` accent token — **Accepted** (deliberate prototype value).

**Portability** — all resolved:
- 🟡 OS-dependent baseline — **Resolved** (element capture + resolved-style
  probes). 🟡 Linux baseline CI gate — **Resolved** (success criteria + manual
  re-trigger). 🔵 Wrapping font metrics — **Resolved** (behavioural). 🔵 Cursor —
  **Resolved** (computed CSS).

### New Issues Introduced

#### Major

- 🟡 **Architecture / Correctness**: Two-region toast split fractures the single
  ordered/capped model — a persistent (no-dismiss) `error` toast still counts
  against `MAX_TOASTS` (`use-toast.ts:84` `.slice(-MAX_TOASTS)`), so a burst of
  later info/ok (or repeated failures) can **silently evict** the very error the
  persistence was meant to preserve. Decide eviction policy (exempt `error`, or
  cap/coalesce per-kind) and assert it. *(Phase 1 §1/§2)*
- 🟡 **Architecture**: `onDragCancel` teardown specified as a duplicated invariant
  rather than a shared seam — extract one `endDrag()` (gate + `activeId`) called
  by both `onDragEnd` and `onDragCancel`, so the gate-clearing invariant has one
  home. *(Phase 3 §1)*
- 🟡 **Correctness**: Render-completion focus effect keyed on entries-list identity
  will fire on **unrelated SSE-driven list updates** (the work-items query is
  invalidated by any external edit), stealing focus after the move. Gate on a
  single-use pending-move token cleared the instant focus is applied. *(Phase 5 §1)*
- 🟡 **Test Coverage**: Phase 2's banner removal **breaks the existing
  `frontend/e2e/kanban-conflict.spec.ts`** (it asserts the `role="alert"
  aria-atomic="true"` banner after a 412), but no migration step is listed —
  the only 412/revert E2E guard is left asserting a deleted element. Add a step to
  migrate it to the assertive error toast + revert. *(Phase 2 §2)*
- 🟡 **Test Coverage**: The `dndDrag` helper is **duplicated inline** in two specs
  (not shared) and runs an uninterruptible down→move→up sequence — it **cannot
  "pause mid-drag"** as the A3 test requires, so A3 as specified would only ever
  observe the post-drop state. Specify extracting a decomposed drag helper with a
  mid-drag inspection point. *(Phase 3 test-first / Testing Strategy)*
- 🟡 **Test Coverage**: **No `page.keyboard` / dnd-kit keyboard-move precedent**
  exists anywhere in the e2e suite — the "existing helpers" claim is inaccurate
  and C1 is a first-of-kind build with real feasibility/flake risk. Add a
  spike/feasibility step (confirm focus target + that Space/arrows drive
  `sortableKeyboardCoordinates` in-browser) before treating C1 as a guaranteed
  deliverable. *(Phase 5 §2)*
- 🟡 **Test Coverage**: Several Phase 2 assertions (success/error toast + revert,
  ConflictError-vs-FetchError, same-column no-op) are listed as **unit tests in
  `KanbanBoard.test.tsx`**, but that logic lives inside `handleDragEnd` reachable
  only through `DndContext`; the existing jsdom board tests deliberately avoid
  drag events. Route through E2E, or extract a pure outcome→toast mapping function
  to assert at unit level. *(Phase 2 test-first / Testing Strategy)*
- 🟡 **Correctness**: Persistent error toasts **accumulate and silently evict**
  under the FIFO cap (same root as the Architecture eviction finding, from the
  state-machine angle) — coalesce repeated move-failure toasts or exempt errors
  from eviction. *(Phase 1 §1)*
- 🟡 **Standards**: The dual-region design **contradicts locked Toaster tests not
  listed for update** — `Toaster.test.tsx:132-137` asserts the single viewport is
  `role="status" aria-live="polite"` and `:152` asserts exactly one status region.
  Add these to Phase 1's explicit test-update list alongside the `:168-175`
  rewrite. *(Phase 1 §4)*

#### Minor

- 🔵 **Code Quality / Correctness**: Overlay null-guard rationale contradicts the
  surviving `source!` non-null assertion on the same move path
  (`KanbanBoard.tsx:108-113`) — apply the same guard to `source`. *(Phase 2/3)*
- 🔵 **Code Quality**: Error-toast dismissal still an unresolved either/or
  (no-dismiss vs 30s); commit to one and assert it. *(Phase 1 §1)*
- 🔵 **Code Quality**: `describeEntry` signature left implicit — specify
  `describeEntry(entry): string` on a resolved entry so both callers pass a clean
  shape. *(Phase 2 §1a)*
- 🔵 **Code Quality**: Focus-restoration trigger still offered as effect-vs-rAF-poll
  — commit to the declarative effect. *(Phase 5 §1)*
- 🔵 **Correctness / Standards**: `onDragCancel` description is inaccurate —
  `handleDragEnd` does **not** reset announcement timers (only `handleDragStart`
  does); the load-bearing teardown is `setDragInProgress(false)` + clear
  `activeId`. Correct the prose. *(Phase 3 §1)*
- 🔵 **Correctness**: `setTimeout(0)`-cleared guard can be cross-cleared by a prior
  drag's pending timer on rapid successive drags — prefer next-pointerdown clear
  or store/cancel the timeout handle. *(Phase 4 §1)*
- 🔵 **Test Coverage**: Two-region split also breaks existing single-region
  assertions in `Toaster.test.tsx`/`KanbanBoard.test.tsx` — list them for
  reconciliation. *(Phase 1)*
- 🔵 **Test Coverage**: Drag-state baseline has no settled-state route to hold
  `isDragging` — specify a static showcase surface rendering
  `WorkItemCardPresentation` with `dragging`/`overlay` props. *(Phase 3 §5)*
- 🔵 **Test Coverage**: `onDragCancel` SSE-gate reset has no regression test — add
  one (Escape mid-drag, assert a later invalidation still renders). *(Phase 3)*
- 🔵 **Test Coverage**: Supplementary `onClickCapture` unit test risks coupling to
  `useSortable` internals — frame it around the pure suppress/allow decision.
  *(Phase 4)*
- 🔵 **Usability**: `message: ''` renders a stray empty `<p>` consuming the flex
  gap — omit the message node when empty, or commit to a short body. *(Phase 2 §1)*
- 🔵 **Usability**: Stacked persistent error toasts clutter the viewport with only
  per-toast dismissal — de-dup or use a long fixed timeout. *(Phase 1 §1)*
- 🔵 **Usability / Standards**: The manual-dismiss affordance (close button +
  Escape) **already exists** (`Toaster.tsx:75-95,28-34`) — resolve the plan's
  conditional to a stated fact and add a dismissal test + verify close-control
  contrast. *(Phase 1 §1/§2)*
- 🔵 **Standards**: No `sr-only`/visually-hidden utility exists in the frontend —
  specify adding a shared `.srOnly` (clip-rect recipe, announced) for the severity
  prefix. *(Phase 1 §2)*
- 🔵 **Standards**: Live-region multiplication on the kanban view — the two new
  toast regions coexist with the board's own polite region and dnd-kit's
  announcements (`KanbanBoard.tsx:174,190`); decide whether the `ok` toast and the
  dnd-kit drop announcement should double-narrate a keyboard move. *(Phase 1/2)*
- 🔵 **Portability**: The Phase 6 font note is **factually inverted** — the stack
  is **self-hosted woff2** from `/fonts/` (`fonts.test.ts:72-75` forbids any
  Google Fonts origin), not Google-Fonts-loaded. Reframe the determinism guard
  around `document.fonts.ready` / swap-fallback resolution, not third-party
  availability. *(Phase 6 §2)*
- 🔵 **Portability**: Computed `transform` serializes as a float `matrix(…)` and
  `box-shadow` expands `var(--ac-shadow-lift)` — assert these resolved-style
  probes with **tolerance**, not string equality (opacity/cursor can stay exact).
  *(Phase 3 §5)*

### Assessment

The plan moved from a broad first-pass REVISE to a much tighter draft — every
original finding is closed. The remaining work is a focused second revision around
two coherent themes rather than scattered fixes:

1. **The kind-aware two-region toast change needs to be treated as a real
   refactor of a shipped component**, not an additive tweak: the `MAX_TOASTS`
   eviction policy for persistent errors, the locked single-region tests, the
   missing `.srOnly` utility, the empty-body render, and live-region
   multiplication all stem from it. Consider whether a simpler approach (one
   region whose `aria-live` is escalated, or mirroring only errors to an assertive
   region) achieves the assertiveness with less blast radius.
2. **The E2E test claims need to be reconciled with the actual harness**: extract
   a decomposable/shared drag helper, spike the keyboard-move feasibility, migrate
   `kanban-conflict.spec.ts`, and decide where the Phase 2 toast/revert assertions
   genuinely live.

Plus the focus one-shot-token correctness fix and a handful of small
commit-to-one-option and accuracy corrections. None are structural; a third pass
should converge quickly. Suggested verdict: **REVISE**.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-06-06T18:41:42+00:00

**Verdict:** COMMENT

The plan has converged: **all 30 pass-2 findings are resolved** (Portability
returned **zero** new findings this pass), and the major count has fallen 19 → 9
→ **1**. The one remaining major is a genuine flaw in the pass-2 focus fix itself
— the single-use pending-move token, gated on a declarative effect keyed on the
entries list, will fire and self-clear on the **optimistic** render (the card is
already in the target column) and so lose focus when the `onSettled` refetch
remounts the node. It is worth correcting, but it is localised (set the token in
the mutation's `onSettled` / pair it with a settled flag, rather than keying on
bare entries identity). The remaining items are minor consistency/reconciliation
tidy-ups and two small accessibility-placement specifics. The plan is acceptable
to proceed; folding in the focus-token fix and the test-reconciliation list is
recommended but not blocking.

### Previously Identified Issues (pass-2 set)

All resolved:
- **Architecture** — two-region eviction model (errors exempt from cap) ✅;
  `onDragCancel` shared `endDrag()` ✅; card focus contract ✅; `errorToastMessageFor`
  placement ✅.
- **Code Quality** — `source!` guarded ✅; error dismissal committed ✅;
  `describeEntry` signature ✅; focus mechanism committed (declarative effect) ✅.
- **Test Coverage** — `kanban-conflict.spec.ts` migration ✅; decomposed/shared
  drag helper ✅; C1 feasibility spike ✅; pure `moveToastFor` + E2E split ✅; locked
  Toaster region tests listed ✅; static-showcase baseline ✅; `onDragCancel`
  regression test ✅; `onClickCapture` reframed ✅.
- **Correctness** — pending-move token (gate intent) ✅; error eviction exemption
  ✅; `onDragCancel` prose corrected ✅; rapid-drag cross-clear → next-pointerdown
  ✅; two-region ordering ✅. *(But see the new token-timing major below.)*
- **Usability** — empty-body `<p>` omitted ✅; persistent-error stacking accepted ✅;
  dismissal stated as fact + test ✅; assertive interruption accepted ✅.
- **Standards** — locked single-region tests listed ✅; `.srOnly` utility ✅;
  live-region overlap accepted ✅; keyboard dismissal path ✅.
- **Portability** — font note corrected (self-hosted woff2) ✅; resolved-style
  tolerance ✅.

### New Issues Introduced

#### Major

- 🟡 **Correctness**: Pending-move focus token **self-clears on the optimistic
  render**, before `onSettled`. `useMoveWorkItem.onMutate` optimistically writes
  the new status, so the card is already in the target column (anchor present) on
  the optimistic entries update; a token-guarded effect keyed on entries identity
  fires and clears there, then the `onSettled` refetch remounts the node and focus
  drops to `<body>` — the exact post-settle failure the token was meant to prevent.
  On the error path the optimistic render shows the card in the *target* column, so
  "focus the resting column" can momentarily resolve to the wrong column before
  revert. **Fix:** set/consume the token on the mutation's `onSettled` (or pair it
  with a settled flag) rather than on bare entries identity. *(Phase 5 §1)*

#### Minor

- 🔵 **Architecture / Correctness**: `endDrag()` must run **unconditionally at the
  start** of `onDragEnd` (mirroring the current line-105 `setDragInProgress(false)`
  placement), before outcome resolution and the new early-returns — not "after
  mutation dispatch" — or an early-return path skips the gate-clear and reinstates
  the stuck-gate bug. Relatedly, keep the SSE flush ordered **before**
  `move.mutate` so the queued-invalidation flush can't clobber the optimistic
  update; have `endDrag()` additionally clear only `activeId`. *(Phase 3 §1)*
- 🔵 **Code Quality**: `describeEntry(entry)` drops the relPath the current
  `describe()` uses to derive the work-item number and the missing-entry fallback —
  specify it derives the number from `entry.relPath`/`entry.workItemId` and define
  the `undefined` return, and assert the missing-entry case. *(Phase 2 §1a)*
- 🔵 **Architecture / Code Quality**: Error-copy mapping now spans `src/api`
  (`errorToastMessageFor`) and the in-route `conflictMessageFor`/`errorMessageFor`
  — state the dividing line (transport/error-class copy in `src/api`; board-load
  copy in-route) and whether the residual in-route `conflictMessageFor` is
  superseded, to avoid two drifting error-class mappers. *(Phase 2 §1a)*
- 🔵 **Correctness / Usability**: A single Escape now both cancels an in-flight
  keyboard drag **and** dismisses a (now-persistent) error toast — the Toaster's
  document-level Escape handler has no `stopPropagation` and errors linger. Gate
  toast Escape-dismiss on no-active-drag (or scope it to a focused toast). *(Phase
  1 §2 vs Phase 3)*
- 🔵 **Test Coverage**: `KanbanColumn.test.tsx:42-48` (`getByText(/no work/i)`) is
  not in the reconciliation list despite the empty-column copy change — add it
  (rewrite to "Nothing here" / "Move a work item here…", keep the `aria-hidden`
  check). *(Phase 3 §4)*
- 🔵 **Test Coverage**: The focus-contract seam has no **single-fire / re-register
  isolation** test (an unrelated SSE refetch must not re-apply focus; the
  relPath-keyed registration must resolve after a refetch remount), and the
  concurrent-delete null-guards (overlay → null; move → no-op) have no regression
  test. Add both. *(Phase 5 §1 / Phase 2 §1 + Phase 3 §1)*
- 🔵 **Standards**: The close-glyph colour (`--ac-fg-faint`) is the dismissal
  affordance for now-persistent errors but is **not** in the automated contrast
  suite — extend `contrast.test.ts` to cover it at 3:1, or state that the
  focus-visible outline (`--ac-accent`, already tested) is the conformance-bearing
  affordance. *(Phase 1 §1)*
- 🔵 **Standards / Usability**: Specify the **sr-only severity prefix is the first
  announced child** (preceding the heading in DOM order) so "Success:" survives the
  empty-message omission on heading-only success toasts; assert the polite region's
  text begins with the prefix. *(Phase 1 §2 / Phase 2 §1)*

#### Suggestions

- 🔵 **Architecture**: Keep `moveToastFor` pure by passing the **resolved target
  label as a parameter** (caller resolves + asserts defined), rather than looking
  it up inside the function. *(Phase 2)*
- 🔵 **Code Quality**: Consider a single error-classification predicate shared by
  `errorToastMessageFor` and the load-failure mapper so the branch logic lives
  once. *(Phase 2 §1a)*
- 🔵 **Usability**: Align the Phase 3 manual-verification checkbox ("Drop a work
  item…") with the chosen mechanism-neutral copy ("Move a work item here…"). *(Phase
  3 §4)*
- 🔵 **Test Coverage**: Add an automated close-glyph contrast probe or explicitly
  mark it manual-only. *(Phase 1 §1)*

### Assessment

The plan is in good shape and ready to implement. Three review passes drove the
major count from 19 to 1, and that last major is a contained timing fix in the
focus restoration (consume the token on `onSettled`, not on the optimistic
render). The remaining minors are almost all "add this existing test to the
reconciliation list" or "specify DOM placement/parameter" tidy-ups that can be
folded in now or handled during implementation. No structural concerns remain.
Suggested verdict: **COMMENT** (acceptable as-is; the focus-token fix and the
test-reconciliation additions are the highest-value follow-ups).

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 4) — 2026-06-06T19:15:20+00:00

**Verdict:** APPROVE

All pass-3 items have been folded into the plan: the focus restoration now arms
its single-use token in the mutation's `onSettled` (not on the optimistic render),
`endDrag()` is specified to run unconditionally at the start of `onDragEnd` with
the SSE flush kept before `move.mutate`, `describeEntry` derives the work-item
number from the entry with a defined missing-entry fallback, the error-copy
dividing line and a shared error-class predicate are specified, the Escape
drag-cancel/toast-dismiss collision is gated, the `KanbanColumn` empty-state test
and the concurrent-delete null-guards have reconciliation/regression tests, the
close-glyph contrast and the sr-only-prefix DOM placement are pinned, and
`moveToastFor` is kept pure via a passed-in label.

Across four passes the major count fell 19 → 9 → 1 → 0, with every finding either
resolved or recorded as a deliberately-accepted tradeoff (persistent-error
stacking; assertive-region interruption; dnd-kit prose-only library decision). No
outstanding findings remain. The plan is **approved** for implementation.

---
*Re-review generated by /accelerator:review-plan*
