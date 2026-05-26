---
date: "2026-05-27T00:00:00+01:00"
type: plan
skill: create-plan
work_item_id: "0039"
status: approved
---

# Toaster and External-Edit Notifications Implementation Plan

## Overview

Add an ephemeral **Toaster** notification system to the visualiser frontend and
its first consumer, an **external-edit toast**. The Toaster is a context-backed
toast stack with a `useToast()` dispatcher, auto-dismiss after 5 s, and manual
dismissal. The external-edit feature subscribes to SSE `doc-changed` events and,
when a non-self-caused change targets the document the user is currently
viewing, raises a toast naming the affected relative path and the action verb.
React Query already invalidates the open document's content on every event, so
the displayed body refreshes with no extra work — the toast is purely additive.

This is almost entirely a composition/wiring exercise on top of mature SSE,
self-cause, and React Query infrastructure, plus one greenfield UI component
(`Toaster`) and three small `src/api/` modules.

## Current State Analysis

All paths below are relative to `skills/visualisation/visualise/frontend/`
(the `workspaces/` checkouts are jj workspaces, not source).

- **No toast / dialog / overlay / portal component exists.** `Toaster` is
  greenfield.
- **SSE plumbing is complete.** `use-doc-events.ts` runs a single
  `EventSource`. In `onmessage` (`use-doc-events.ts:202-232`): `subscribe()`
  listeners fire **first, before** the self-cause drop (`:210-219`); then the
  drop (`if doc-changed && registry.has(etag) return`, `:220`); then `onEvent`
  (`:221`, already taken by `useUnseenDocTypes`); then `dispatchSseEvent`
  (`:227`).
- **`dispatchSseEvent` already invalidates `docContent(event.path)`**
  (`use-doc-events.ts:106`), and `event.path === entry.relPath`
  (`fetchDocContent`'s key), so the open detail view refreshes automatically.
  The "content refreshes without reload" AC is satisfied by existing code.
- **Self-cause registry** (`self-cause.ts`): `has(etag)` returns `false` for
  `undefined`, non-consuming, 5 s TTL. `RootLayout` does **not** provide
  `SelfCauseContext`, so every consumer shares `defaultSelfCauseRegistry` — a
  subscriber's `has()` agrees with the dispatcher's drop decision.
- **Route → relPath** is resolved inline in `LibraryDocView.tsx:37-57`:
  `useParams({ strict: false })` → `type` + `fileSlug` → docs-list query
  (`queryKeys.docs(type)`) → `entries.find(e => e.slug === fileSlug ||
  fileSlugFromRelPath(e.relPath) === fileSlug)` → `entry.relPath`.
- **RootLayout** (`RootLayout.tsx:16-62`) owns the handles
  (`useUnseenDocTypes`, `useDocEvents`, `useTheme`, `useFontMode`) and nests
  providers outer→inner: `Theme` → `FontMode` → `DocEvents` → `UnseenDocTypes`
  → `<div styles.root>` (`Topbar` + body with `Sidebar` + `<main><Outlet/></main>`).

### Key Discoveries:

- **Context/hook template** — `use-theme.ts:8-78` and `use-unseen-doc-types.ts:63-131`:
  handle interface → owning hook (called once at RootLayout, held in
  `useState`/`useRef`) → `createContext` seeded with a **no-op default handle**
  → consumer `useXContext()`. **This codebase never throws "must be used within
  provider"** — match the no-op default so leaf tests render without a provider.
- **Immutable collection state** — `use-unseen-doc-types.ts:90-107` shows the
  `useState<Set>` + `setX(prev => new Set(prev))` immutable-mutator pattern; the
  toast stack (`Toast[]`) follows the same shape.
- **Auto-dismiss timer** — `use-deferred-fetching-hint.ts:16-31` is the
  canonical `setTimeout` + `useEffect` + `clearTimeout` transient; the
  `justReconnected` reset-on-retrigger edge case is in `use-doc-events.ts:176-197`
  (also the `ReturnType<typeof setTimeout>` id typing).
- **Fake-timer test** — `use-deferred-fetching-hint.test.tsx` is a direct
  template: `beforeEach(vi.useFakeTimers)` / `afterEach(vi.useRealTimers)`,
  `act(() => vi.advanceTimersByTime(...))`.
- **Component trio** — `SseIndicator.{tsx,module.css,test.tsx}`: **named
  export** (house convention, no barrel `index`), CSS module imported as
  `styles`, visual state via `data-*` attributes, decorative inline SVG
  `aria-hidden="true"`. The test has a `describe('CSS source assertions')` block
  importing the CSS via `?raw` and regex-asserting `var(--ac-*)` bindings
  (`SseIndicator.test.tsx:77-104`).
- **No generic notification glyph** — `Glyph` is strictly per-doc-type
  (12 keys). Inline a small purpose-built `<svg viewBox="0 0 24 24"
  aria-hidden="true">` per the `SseIndicator` precedent.
- **Tokens** (`global.css`): `--ac-bg-card` (:85), `--ac-fg-muted` (:90),
  `--ac-ok` (:99), `--ac-warn` (:100), `--ac-err` (:101),
  `--ac-shadow-lift` (:202), `--radius-md`/`--radius-lg` (:193-194), spacing
  `--sp-*`, sizes `--size-*`. Dark + `prefers-color-scheme` mirrors at
  :319-423.
- **Route-aware test harness** — `src/test/router-helpers.tsx`
  `renderWithRouterAt(ui, atUrl)` (memory history; routes `/`,
  `/library/$type`, `/library/$type/$fileSlug` already defined).
- **Build/test commands** run from `frontend/` via npm (no Makefile):
  `npm run typecheck`, `npm test` (vitest run), `npm run build`.

## Desired End State

- A `Toaster` portal mounted at `RootLayout` renders a stack of toasts; each has
  an icon, heading, message, and close button. The stack lives inside a single
  persistent `aria-live="polite"` viewport container (announcements are reliable
  because the live region pre-exists the inserted cards) and is capped at
  `MAX_TOASTS` (5) — when full, the oldest toast is dropped (its timer cleared).
- `useToast()` (consumer hook reading `ToastContext`) exposes
  `showToast({ heading, message })` (options object — self-documenting at call
  sites, transposition-proof, extensible to a future `variant`/`duration` field);
  toasts auto-dismiss at 5 s (±0.5 s) and can be dismissed early via the close
  button (which cancels that toast's timer) or by pressing `Escape` (dismisses
  the most-recent toast). Auto-dismiss **pauses on hover/focus-within** and
  resumes on leave/blur (WCAG 2.2.1 — the timer never expires while the user is
  reading or interacting with the toast).
- When the user views document at relPath `X` and a non-self-caused
  `doc-changed` event reports a change to `X`, a toast appears with heading
  "External edit detected" and message ``\`X\` was {verb} while you were looking
  at it.`` (`created`→"created", `edited`→"updated", `deleted`→"deleted"). The
  rendered body reflects the post-change content without a reload.
- An event for a different path `Y`, or a self-caused change to `X`, raises no
  toast.

### Verification

`npm run typecheck` passes; `npm test` passes (new unit + integration suites);
`npm run build` succeeds; manual browser check (Manual Verification, Phase 5).

## What We're NOT Doing

- **No actor identity in copy.** SSE carries no actor; copy is actor-generic.
  (The prototype's `WORK-0007`/"A reviewer agent" copy is design intent only and
  not buildable.)
- **No stable document ID.** Correlation and display use `relPath`. Switching to
  a real doc ID is deferred to a separate, future work item
  (`meta/notes/2026-05-26-toast-correlation-should-use-document-id.md`).
- **No "Query invalidated" text** — dropped as an implementation detail.
- **No new SSE invalidation logic** — `dispatchSseEvent` already refreshes
  `docContent(event.path)`; we ride on it, never duplicate or fight it.
- **No `SelfCauseContext` provider added to RootLayout** — the shared
  `defaultSelfCauseRegistry` is the intended single source.
- **No toast positioning animation library.** No swipe-to-dismiss. (A simple
  `MAX_TOASTS` cap and pause-on-hover/focus ARE in scope — they are accessibility
  requirements, not feature creep; see Desired End State and Phase 1/2.)
- **No explicit post-dismiss focus restoration** — accepted limitation. Toasts
  are non-focus-stealing polite notifications; after clicking close, focus
  returns to `document.body` (the toast was ephemeral). The close button is
  keyboard-operable, pause-on-focus gives keyboard users reading grace, and
  `Escape` dismisses the most-recent toast — together these cover the
  substantive WCAG concerns without focus-trap complexity.
- **No second `EventSource`** — the subscriber uses
  `useDocEventsContext().subscribe`, never a second `useDocEvents()`.

## Implementation Approach

Five phases, sequenced so each is independently testable with TDD (write the
failing test(s) first, then the implementation, until green). Phase 1 defines
the toast contract (the `ToastHandle` interface) that Phases 2 and 3 depend on
**at the type level only** — both are unit-testable in isolation via mocks, so
the phases can be implemented and reviewed independently. Phase 4 is a tiny
params-only route→relPath resolver hook (also independent). Phase 5 wires everything into
`RootLayout` and adds the end-to-end integration tests that bind the acceptance
criteria to real route resolution.

Per-phase TDD loop: add the test file (or cases) → run `npm test <file>` and see
it fail → implement → see it pass → `npm run typecheck`.

```
Phase 1  use-toast.ts            (stack + dispatcher + 5s auto-dismiss)   ─┐
Phase 2  Toaster/ component      (presentational, consumes ToastContext)   │ independent
Phase 4  use-active-doc-relpath  (params-only route→relPath resolver)      ─┘
Phase 3  use-external-edit-toast (subscriber: depends on Phase 1 + 4 types)
Phase 5  RootLayout wiring + integration tests (depends on all)
```

---

## Phase 1: Toast context, stack, and dispatcher

### Overview

Greenfield `src/api/use-toast.ts`: the toast data model, owning hook (stack +
auto-dismiss timers), context with no-op default, and consumer hook. No UI, no
SSE. Pure, fully unit-testable with fake timers.

### Changes Required:

#### 1. Toast model + hook

**File**: `src/api/use-toast.ts` (new)
**Changes**: Define the `Toast` record and `ToastHandle`; implement the owning
hook `useToastDispatcher()` (called once at RootLayout), the `ToastContext` with
a no-op default, and the consumer hook `useToast()`.

```ts
import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'

export interface Toast {
  id: number
  heading: string
  message: string
}

export interface ShowToastInput {
  heading: string
  message: string
}

export interface ToastHandle {
  toasts: ReadonlyArray<Toast>
  showToast(input: ShowToastInput): number
  dismissToast(id: number): void
  pauseToast(id: number): void
  resumeToast(id: number): void
}

export const TOAST_AUTO_DISMISS_MS = 5_000
export const MAX_TOASTS = 5

/**
 * OWNING hook — call EXACTLY ONCE at the RootLayout level. Returns a fresh
 * handle whose value must be supplied to <ToastContext.Provider value={...}>.
 * Leaf components must NOT call this — use `useToast()` instead.
 *
 * NOTE: the owning/consumer naming is deliberately the inverse of the house
 * convention (useTheme owns / useThemeContext consumes). The work item mandates
 * `useToast()` as the dispatcher-facing API, so the bare name is the consumer;
 * the owning hook takes the explicit `useToastDispatcher` name + this docstring.
 */
export function useToastDispatcher(
  autoDismissMs = TOAST_AUTO_DISMISS_MS,
): ToastHandle {
  const [toasts, setToasts] = useState<Toast[]>([])
  const toastsRef = useRef<Toast[]>(toasts)
  toastsRef.current = toasts // mirror for side-effect-free reads in event handlers
  const nextIdRef = useRef(1)
  const timersRef = useRef(new Map<number, ReturnType<typeof setTimeout>>())

  const clearTimer = useCallback((id: number) => {
    const timer = timersRef.current.get(id)
    if (timer !== undefined) {
      clearTimeout(timer)
      timersRef.current.delete(id)
    }
  }, [])

  const dismissToast = useCallback(
    (id: number) => {
      clearTimer(id)
      setToasts((prev) => prev.filter((t) => t.id !== id))
    },
    [clearTimer],
  )

  const arm = useCallback(
    (id: number) => {
      timersRef.current.set(id, setTimeout(() => dismissToast(id), autoDismissMs))
    },
    [autoDismissMs, dismissToast],
  )

  const pauseToast = useCallback((id: number) => clearTimer(id), [clearTimer])

  // Resume restarts a fresh full window (acceptable for a 5 s toast; avoids
  // tracking elapsed remaining time). Reads the mirror ref (NOT a state updater)
  // so it stays a pure event handler; no-op if the toast is gone or still armed.
  const resumeToast = useCallback(
    (id: number) => {
      if (toastsRef.current.some((t) => t.id === id) && !timersRef.current.has(id)) {
        arm(id)
      }
    },
    [arm],
  )

  const showToast = useCallback(
    ({ heading, message }: ShowToastInput): number => {
      const id = nextIdRef.current++
      // Pure updater: append, then cap by keeping the newest MAX_TOASTS. Timer
      // bookkeeping for any dropped toast is handled by the reconcile effect
      // below (so the updater has no side effects and is StrictMode-safe).
      setToasts((prev) => [...prev, { id, heading, message }].slice(-MAX_TOASTS))
      arm(id)
      return id
    },
    [arm],
  )

  // Reconcile timers to the live toast list: any toast that has left the stack
  // (dropped by the MAX_TOASTS cap) gets its orphaned timer cleared. Arming is
  // owned by showToast/resumeToast (so a deliberately-paused toast — present but
  // unarmed — is never re-armed here).
  useEffect(() => {
    const live = new Set(toasts.map((t) => t.id))
    for (const id of [...timersRef.current.keys()]) {
      if (!live.has(id)) clearTimer(id)
    }
  }, [toasts, clearTimer])

  // Clear all outstanding timers on unmount — matches the clearTimeout-on-
  // teardown convention (use-deferred-fetching-hint.ts, use-doc-events.ts).
  useEffect(() => {
    const timers = timersRef.current
    return () => {
      for (const t of timers.values()) clearTimeout(t)
      timers.clear()
    }
  }, [])

  return { toasts, showToast, dismissToast, pauseToast, resumeToast }
}

const noopHandle: ToastHandle = {
  toasts: [],
  showToast: () => 0,
  dismissToast: () => {},
  pauseToast: () => {},
  resumeToast: () => {},
}

export const ToastContext = createContext<ToastHandle>(noopHandle)

/** CONSUMER hook — reads the ToastContext provided by RootLayout. */
export function useToast(): ToastHandle {
  return useContext(ToastContext)
}
```

Notes:
- `showToast` takes an **options object** (`{ heading, message }`) so call sites
  are self-documenting and the two same-typed strings can't be transposed; it
  returns the `id` so a caller could pre-emptively dismiss (the external-edit
  subscriber ignores it). The shape is extensible to a future `variant`/`duration`.
- Auto-dismiss timeout is injectable (`autoDismissMs`) so tests can keep the
  real 5 s default while production uses the constant; the AC tolerance test
  uses the default. Per-toast duration is intentionally out of scope (YAGNI) —
  the single injection seam covers all current needs.
- Manual dismiss clears that toast's timer (resolves the "manual-dismiss vs
  auto-dismiss race" — no double-removal, no leaked timer).
- **Unmount cleanup**: a teardown `useEffect` clears every outstanding timer, so
  no auto-dismiss callback fires `setToasts` after unmount (avoids React unmount
  warnings and the leak the Performance section promises to avoid).
- **Pure updaters + reconcile effect**: `showToast`'s `setToasts` updater is a
  pure function of `prev` (append + `slice(-MAX_TOASTS)`); all timer side effects
  live in plain event handlers (`arm`/`clearTimer`) or the reconcile `useEffect`,
  never inside an updater — so StrictMode's double-invocation of updaters cannot
  double-schedule or leak timers. The reconcile effect clears the timer of any
  toast that has left the stack (i.e. one the cap dropped); it deliberately does
  NOT arm, so a paused toast (present but unarmed) is left alone.
- **Pause/resume**: `pauseToast` clears the timer (without removing the toast);
  `resumeToast` re-arms a fresh window if the toast still exists and isn't armed,
  reading the `toastsRef` mirror (not a state updater). The presentational
  component wires these to hover/focus-within (Phase 2). A toast that is paused
  and never resumed (e.g. a leave/blur event is missed) stays in the stack with
  no timer until manual dismiss — an accepted terminal state, not a leak (the
  unmount teardown still clears it).
- **Stack cap** (`MAX_TOASTS` = 5): keeping only the newest `MAX_TOASTS` means a
  rapid burst (> 5 events in < 5 s) drops the **oldest** still-visible toast
  before its timer fires. Accepted information-loss tradeoff: the toast is only a
  notification — React Query invalidation refreshes the affected document content
  independently of whether its toast survived, so no document state is lost. A
  future "N files changed" coalesced toast could reduce churn if it proves
  noisy.
- The "call exactly once" owning-hook contract is enforced by docstring +
  emphatic naming only (not a runtime guard) — consistent with the existing
  `useTheme`/`useUnseenDocTypes` owning hooks. A misuse (calling
  `useToastDispatcher` in a leaf) creates a disconnected stack whose toasts never
  appear; accepted as a house-convention tradeoff. A dev-only "instantiated more
  than once" `console.warn` is a possible future hardening if it bites.

### Test-First:

**File**: `src/api/use-toast.test.tsx` (new) — model on
`use-deferred-fetching-hint.test.tsx` (fake timers).

- `beforeEach(vi.useFakeTimers)` / `afterEach(vi.useRealTimers)`;
  `renderHook(() => useToastDispatcher())`.
- `showToast({ heading, message })` appends a toast with the given
  heading/message and a unique id; two calls yield two stacked toasts with
  distinct ids.
- **Auto-dismiss timing (AC)**: after `showToast`, toast present;
  `advanceTimersByTime(4000)` → still present; `advanceTimersByTime(1500)`
  (total 5500) → removed. (Asserts the 5 s ±0.5 s window via the default.)
- `dismissToast(id)` removes that toast immediately and, after a subsequent
  `advanceTimersByTime(5000)`, does not throw / double-remove (timer cleared).
- Dismissing one toast in a stack of two leaves the other intact and still
  auto-dismissing on its own timer.
- **Staggered independent timers**: `showToast` A; `advanceTimersByTime(3000)`;
  `showToast` B; `advanceTimersByTime(2000)` (A at 5 s) → A gone, B present;
  `advanceTimersByTime(3000)` (B at 5 s) → B gone. (Pins per-toast timers, not a
  shared/reset one.)
- **Pause/resume**: `showToast`; `pauseToast(id)`; `advanceTimersByTime(10000)`
  → still present (timer cleared); `resumeToast(id)`; `advanceTimersByTime(4000)`
  → present; `advanceTimersByTime(1500)` → removed (fresh window from resume).
  `resumeToast` on an unknown/already-dismissed id is a no-op (does not throw,
  does not resurrect).
- **Stack cap (`MAX_TOASTS`)**: issue `MAX_TOASTS + 1` `showToast` calls → only
  `MAX_TOASTS` toasts present, and the **oldest** id is the one dropped; advancing
  5 s does not throw (dropped toast's timer was cleared).
- **Unmount cleanup**: `showToast`; `unmount()`; `advanceTimersByTime(5000)`
  produces no error / no "state update on unmounted component" warning (spy on
  `console.error`, or rely on the timer map being cleared).
- `useToast()` with no provider returns the no-op handle (calling `showToast`,
  `pauseToast`, `resumeToast` does not throw).

### Success Criteria:

#### Automated Verification:
- [ ] New unit suite passes: `npm test src/api/use-toast.test.tsx` (run from `frontend/`)
- [ ] Type checking passes: `npm run typecheck`

#### Manual Verification:
- [ ] None (no UI in this phase).

---

## Phase 2: Toaster presentational component

### Overview

Greenfield `src/components/Toaster/` trio. Pure presentational: consumes
`useToast()` for the stack and renders each toast (icon, heading, message, close
button). State and dismissal come from the Phase 1 handle; this component owns
no timers.

### Changes Required:

#### 1. Component

**File**: `src/components/Toaster/Toaster.tsx` (new)
**Changes**: Named export `Toaster`; maps `toasts` to cards. The **viewport is a
single persistent `role="status"` `aria-live="polite"` region** (always rendered
so the live region pre-exists inserted cards — more reliable SR announcement than
a per-card region; mirrors KanbanBoard's persistent-region pattern). Each card is
a plain `<div>` (no per-card live region). Each card carries
`onMouseEnter`/`onFocus` → `pauseToast(t.id)` and `onMouseLeave`/`onBlur` →
`resumeToast(t.id)` (WCAG 2.2.1 pause-on-hover/focus). Inline decorative info
glyph SVG with `aria-hidden="true"` and `data-testid="toaster-icon"`; heading,
message, and a close `<button aria-label="Dismiss notification">` calling
`dismissToast(t.id)`. A document-level `keydown` listener dismisses the
most-recent toast on `Escape` (keyboard-only quick dismiss; only attached while
the stack is non-empty). Render to a portal at `document.body`. Use a single
consistent info glyph for every toast.

Because the viewport is now always mounted (the live region must pre-exist), it
renders empty when the stack is empty rather than returning `null`.

```tsx
import { useEffect } from 'react'
import { createPortal } from 'react-dom'
import { useToast } from '../../api/use-toast'
import styles from './Toaster.module.css'

export function Toaster() {
  const { toasts, dismissToast, pauseToast, resumeToast } = useToast()

  // Escape dismisses the most-recent toast (the visually-topmost / last in the
  // stack). Document-level listener because toasts are non-focus-stealing.
  useEffect(() => {
    if (toasts.length === 0) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && toasts.length > 0) {
        dismissToast(toasts[toasts.length - 1].id)
      }
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [toasts, dismissToast])

  return createPortal(
    <div
      className={styles.viewport}
      data-testid="toaster-viewport"
      role="status"
      aria-live="polite"
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          className={styles.toast}
          onMouseEnter={() => pauseToast(t.id)}
          onMouseLeave={() => resumeToast(t.id)}
          onFocus={() => pauseToast(t.id)}
          onBlur={() => resumeToast(t.id)}
        >
          <svg
            className={styles.icon}
            data-testid="toaster-icon"
            viewBox="0 0 24 24"
            width="20"
            height="20"
            fill="none"
            stroke="currentColor"
            aria-hidden="true"
          >
            {/* Info glyph: circle + i (stroke round). Concrete, not a placeholder. */}
            <circle cx="12" cy="12" r="9" strokeWidth="2" />
            <path d="M12 11v5M12 8h.01" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          <div className={styles.text}>
            <p className={styles.heading}>{t.heading}</p>
            <p className={styles.message}>{t.message}</p>
          </div>
          <button
            type="button"
            className={styles.close}
            aria-label="Dismiss notification"
            onClick={() => dismissToast(t.id)}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" aria-hidden="true">
              <path d="M6 6l12 12M18 6L6 18" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      ))}
    </div>,
    document.body,
  )
}
```

Note: `onFocus`/`onBlur` on the card bubble from the close button, so focusing
the close button (keyboard) also pauses the timer — keyboard users get the same
reading grace as hover.

#### 2. Styles

**File**: `src/components/Toaster/Toaster.module.css` (new)
**Changes**: `.viewport` fixed bottom-right (`position: fixed`, `inset` /
`bottom`+`right` via `--sp-*`, `z-index` above chrome, column flex, gap;
`pointer-events: none` on the empty viewport so the always-mounted container
doesn't block clicks, with `pointer-events: auto` restored on `.toast`). `.toast`
uses `--ac-bg-card`, `--ac-shadow-lift`, `--radius-lg`/`--radius-md`,
border/`--ac-fg-muted` text. `.icon` colour is `var(--ac-fg-muted)` — a neutral,
informational accent; **not** `var(--ac-ok)` (green would read as "success" for
what is an informational "external edit detected" notice) and not `--ac-warn`
(amber would over-escalate a benign content refresh). Respect
`@media (prefers-reduced-motion: reduce)` if any
entrance animation is added (mirror `SseIndicator.module.css:19-23`).

### Test-First:

**File**: `src/components/Toaster/Toaster.test.tsx` (new) — model on
`SseIndicator.test.tsx` (mock the context module, `?raw` CSS assertions).

```tsx
vi.mock('../../api/use-toast', () => ({ useToast: vi.fn() }))
import { useToast } from '../../api/use-toast'
import toasterCss from './Toaster.module.css?raw'
```

- With one toast in the handle, renders heading, message, an element with
  `data-testid="toaster-icon"` that is an actual `<svg>`, and a button named
  "Dismiss notification" (AC: icon + heading + message + close button rendered).
  Assert both the testid AND `card.querySelector('svg')` so a testid rename or an
  empty SVG doesn't silently pass.
- Clicking the close button calls `dismissToast` with that toast's id (AC:
  close click dismisses). (Removal itself is Phase 1's job; assert the callback
  fires.)
- **Pause/resume wiring**: `mouseEnter` on the card calls `pauseToast(id)`;
  `mouseLeave` calls `resumeToast(id)`; focusing the close button (which bubbles
  `onFocus` to the card) calls `pauseToast(id)`, blur calls `resumeToast(id)`.
- **Escape-to-dismiss**: with a stack of two, dispatching `keydown` with
  `key: 'Escape'` calls `dismissToast` with the **last** toast's id (most-recent
  / topmost). With an empty stack, `Escape` is a no-op (no listener attached).
  The listener is removed on unmount (no leaked document listener).
- Empty stack: the viewport is still rendered (the persistent live region) but
  contains no `.toast` cards (`toasts: []` → no cards, `toaster-icon` absent).
- A stack of two renders two cards inside the single `role="status"` viewport.
- CSS source assertions: `.toast` binds `var(--ac-bg-card)` and
  `var(--ac-shadow-lift)`; viewport is `position: fixed`; `.icon` binds
  `var(--ac-fg-muted)` and **not** `--ac-ok`. (Mirror SseIndicator's `?raw`
  regex style.)

### Success Criteria:

#### Automated Verification:
- [ ] New component suite passes: `npm test src/components/Toaster/Toaster.test.tsx`
- [ ] Type checking passes: `npm run typecheck`

#### Manual Verification:
- [ ] None yet (component not mounted until Phase 5).

---

## Phase 4: Shared active-document relPath resolver

### Overview

Add a small, **deliberately params-only** route→relPath resolver hook
`src/api/use-active-doc-relpath.ts` for the RootLayout-mounted subscriber
(Phase 3) to learn which document is currently being viewed. (Numbered Phase 4
but independent of Phases 1–3; can be built in parallel.)

**Scope note — this is NOT a shared "single source of truth" with
`LibraryDocView`.** `LibraryDocView` resolves identity from `propType ??
params.type` / `propSlug ?? params.fileSlug` (`:38-39`) and surfaces docs-list
`isError`/`error` to the user (`:44`); this hook reads `useParams` only and
surfaces no error (returning `undefined` off-route / unloaded is exactly the
"no toast" behaviour). They have different input contracts and different
responsibilities, so they are intentionally two resolvers, not one. The
`LibraryDocView` refactor is **out of scope** (see below) — attempting to unify
them would change the view's prop-driven and error-surfacing behaviour.

### Changes Required:

#### 1. Resolver hook

**File**: `src/api/use-active-doc-relpath.ts` (new)
**Changes**: Read `useParams({ strict: false })`, narrow `type` via
`isDocTypeKey`, run the docs-list query (`queryKeys.docs(type)`, `enabled: type
!== undefined`), find the entry by `slug` or `fileSlugFromRelPath(relPath)`,
return `entry?.relPath` (`string | undefined`). Returns `undefined` whenever not
on a doc route or the list is unloaded/unmatched — which is exactly the
"no toast off-route" behaviour.

```ts
import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs } from './fetch'
import { queryKeys } from './query-keys'
import { isDocTypeKey } from './types'
import { fileSlugFromRelPath } from './path-utils'

export function useActiveDocRelPath(): string | undefined {
  const params = useParams({ strict: false }) as { type?: string; fileSlug?: string }
  const type = params.type && isDocTypeKey(params.type) ? params.type : undefined
  const fileSlug = params.fileSlug ?? ''
  const { data: entries = [] } = useQuery({
    queryKey: type ? queryKeys.docs(type) : queryKeys.disabled('docs'),
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  })
  if (!fileSlug) return undefined
  return entries.find(
    (e) => e.slug === fileSlug || fileSlugFromRelPath(e.relPath) === fileSlug,
  )?.relPath
}
```

#### 2. LibraryDocView is left untouched (refactor explicitly out of scope)

**File**: `src/routes/library/LibraryDocView.tsx` — **no change.**
`LibraryDocView` keeps its inline prop-or-param resolution and its
`isError`/`error` surfacing. The new hook stands alone as a params-only resolver
for the subscriber. This is a deliberate decision (not an implementation-time
coin flip): unifying the two would require the hook to grow prop overrides and
error exposure, changing the view's behaviour for no functional gain here. If a
future doc-ID migration wants one resolver, that is its own work item.

### Test-First:

**File**: `src/api/use-active-doc-relpath.test.tsx` (new) — model on
`use-related.test.tsx` / `use-doc-page-data.test.tsx` (QueryClient + router
wrapper) and `router-helpers.tsx`.

- At `/library/work-items/0007-foo` with a docs-list query seeded (mock
  `fetchDocs`) containing an entry whose `slug`/relPath matches `0007-foo`,
  the hook returns that `entry.relPath`.
- Match via `fileSlugFromRelPath` fallback (slug differs from filename).
- Off a doc route (`/`), returns `undefined`.
- On a doc route but docs list still loading / no match → `undefined`.
- Unknown/`invalid` type → `undefined` (query disabled).

### Success Criteria:

#### Automated Verification:
- [ ] New suite passes: `npm test src/api/use-active-doc-relpath.test.tsx`
- [ ] `LibraryDocView` untouched: `npm test src/routes/library` still green
- [ ] Type checking passes: `npm run typecheck`

#### Manual Verification:
- [ ] None.

---

## Phase 3: External-edit subscriber

### Overview

Greenfield `src/api/use-external-edit-toast.ts`: a hook that, on mount,
subscribes to SSE events via `useDocEventsContext().subscribe`, and for each
`doc-changed` event compares `event.path` to the active relPath
(`useActiveDocRelPath`, Phase 4), excludes self-caused events
(`useSelfCauseRegistry().has(event.etag)`), maps the action to a verb, and calls
`useToast().showToast(...)`. Depends on Phases 1 and 4 at the type level; tested
in isolation by mocking all three context hooks.

### Changes Required:

#### 1. Action→verb map + subscriber hook

**File**: `src/api/use-external-edit-toast.ts` (new)
**Changes**:

```ts
import { useEffect, useRef } from 'react'
import { useDocEventsContext } from './use-doc-events'
import { useSelfCauseRegistry } from './self-cause'
import { useActiveDocRelPath } from './use-active-doc-relpath'
import { useToast } from './use-toast'
import type { ActionKind, SseEvent } from './types'

/** Normative action→verb mapping (work item 0039). */
const ACTION_VERB: Record<ActionKind, string> = {
  created: 'created',
  edited: 'updated',
  deleted: 'deleted',
}

export const EXTERNAL_EDIT_HEADING = 'External edit detected'

export function externalEditMessage(relPath: string, action: ActionKind): string {
  return `\`${relPath}\` was ${ACTION_VERB[action]} while you were looking at it.`
}

/**
 * Headless subscriber — mount once inside the Toast + DocEvents providers.
 * Raises an external-edit toast when a non-self-caused doc-changed event hits
 * the document currently being viewed.
 */
export function useExternalEditToast(): void {
  const { subscribe } = useDocEventsContext()
  const registry = useSelfCauseRegistry()
  const { showToast } = useToast()
  const relPath = useActiveDocRelPath()

  // Latest values read inside the long-lived listener without re-subscribing.
  const ref = useRef({ relPath, registry, showToast })
  ref.current = { relPath, registry, showToast }

  useEffect(() => {
    const unsubscribe = subscribe((event: SseEvent) => {
      if (event.type !== 'doc-changed') return
      const { relPath, registry, showToast } = ref.current
      if (relPath === undefined) return
      // Correlation is exact string equality: event.path must be byte-identical
      // to the indexer's relPath (same separators, no leading './', same .md
      // casing — server contract, see use-doc-events.ts:84-86). A format
      // divergence fails silently here (no toast), so the Phase 5 integration
      // test pins it with a real server-shaped event.path fixture.
      if (event.path !== relPath) return
      if (registry.has(event.etag)) return // self-caused — not external
      // Stay in the pre-drop `subscribe` slot (use-doc-events.ts:210), NOT the
      // post-drop `onEvent` slot: subscribe sees ALL events so this hook can own
      // its own self-cause check above; onEvent only fires for events that
      // survived the drop, which is the wrong tier for this decision. (relPath
      // comes from the cached docs query, not synchronous SSE state, so it
      // resolves the doc regardless of ordering — incl. the deleted case, where
      // the docs-list refetch flips it to undefined only on a later render.)
      showToast({
        heading: EXTERNAL_EDIT_HEADING,
        message: externalEditMessage(relPath, event.action),
      })
    })
    return unsubscribe
  }, [subscribe])
}
```

Notes:
- `subscribe()` fires **before** the self-cause drop (`use-doc-events.ts:210`),
  so this hook owns its `registry.has(event.etag)` check explicitly — by design.
- **Shared-registry invariant**: this correctness depends on `useSelfCauseRegistry()`
  resolving the **same** instance the dispatcher's drop consults — true today
  because no `SelfCauseContext` provider is mounted, so both default to the
  module singleton `defaultSelfCauseRegistry`. Any future `SelfCauseContext`
  provider MUST wrap both the dispatcher and this subscriber, or external-edit
  toasts will desync (phantom or missing).
- `registry.has(undefined)` is `false`, so a `doc-changed` event with no `etag`
  is always treated as external (a toast fires) — intentional.
- The `ref` pattern (matching `use-doc-events.ts:148-151`) keeps the
  subscription stable across re-renders (route changes update `relPath` without
  re-subscribing). `subscribe` is `useCallback([])`-stable. Of the three reffed
  values only `relPath` actually varies; `registry`/`showToast` are stable
  context handles bundled in solely to keep the `[subscribe]` effect dep list
  minimal (not because they are reactive).
- **5 s coincidence is benign**: the toast auto-dismiss window
  (`TOAST_AUTO_DISMISS_MS`) and the self-cause registry TTL (`self-cause.ts`) are
  both 5 s but **independent** — they need not track each other. Suppression
  depends only on whether the event's etag is in the registry at event-arrival
  time; the toast's own lifetime is unrelated. Tuning one does not require
  tuning the other.
- Headless: returns `void`; a wrapper component (Phase 5) calls it and renders
  `null`.

### Test-First:

**File**: `src/api/use-external-edit-toast.test.tsx` (new). Mock
`useDocEventsContext`, `useSelfCauseRegistry`, `useActiveDocRelPath`, `useToast`
(the SseIndicator-style module mock). Capture the listener passed to
`subscribe`, then invoke it with crafted events.

- Pure helpers first: `externalEditMessage('a/b.md', 'created')` ===
  ``\`a/b.md\` was created while you were looking at it.``; `'edited'`→"updated";
  `'deleted'`→"deleted" (each action verified independently, per AC).
- Active relPath `X`, event `doc-changed` path `X`, etag not in registry →
  `showToast({ heading: 'External edit detected', message: <message for that
  action> })` called once; one case per action verb.
- Event path `Y !== X` → `showToast` not called.
- Self-caused (`registry.has` returns `true` for the etag) → `showToast` not
  called.
- **Undefined etag** (`event.etag` omitted; `registry.has(undefined) === false`)
  with matching path → `showToast` IS called (treated as external). Pins the
  documented boundary at the subscriber.
- `relPath === undefined` (off-route) → `showToast` not called.
- **Route change without re-subscribe**: mount with relPath `X` (capture the
  single listener passed to `subscribe`); rerender with relPath `Y`; fire an
  event for `Y` → `showToast` fires for `Y`, AND `subscribe` was called exactly
  once (no re-subscribe / no leaked listener). Guards the ref pattern.
- Non-`doc-changed` events (`doc-invalid`, `template-changed`) → ignored.
- Unmount calls the returned unsubscribe.

### Success Criteria:

#### Automated Verification:
- [ ] New suite passes: `npm test src/api/use-external-edit-toast.test.tsx`
- [ ] Type checking passes: `npm run typecheck`

#### Manual Verification:
- [ ] None yet (mounted in Phase 5).

---

## Phase 5: RootLayout wiring + end-to-end integration

### Overview

Mount the toast provider, the presentational `Toaster`, and a headless
subscriber wrapper in `RootLayout`, then add an integration test that exercises
the full path with real route resolution and the real `dispatchSseEvent`
invalidation (asserting the content-refresh AC).

### Changes Required:

#### 1. Headless subscriber wrapper

**File**: `src/components/Toaster/ExternalEditToast.tsx` (new)
**Changes**: Tiny component that calls `useExternalEditToast()` and returns
`null`. Keeps `Toaster` purely presentational and the subscriber out of
`RootLayout`'s body (so it sits inside the providers).

```tsx
import { useExternalEditToast } from '../../api/use-external-edit-toast'
export function ExternalEditToast() {
  useExternalEditToast()
  return null
}
```

#### 2. RootLayout wiring

**File**: `src/components/RootLayout/RootLayout.tsx`
**Changes**: Call `useToastDispatcher()` alongside the other owning hooks
(`:16-23`); nest `<ToastContext.Provider value={toast}>` inside
`UnseenDocTypesContext.Provider` (after `:43`); render `<ExternalEditToast />`
and `<Toaster />` inside that provider (the `Toaster` portals to
`document.body`, so position in the tree only matters for context access).

```tsx
const toast = useToastDispatcher()
// …
<UnseenDocTypesContext.Provider value={unseen}>
  <ToastContext.Provider value={toast}>
    <div className={styles.root}>
      {/* Topbar + body as before */}
    </div>
    {/* INVARIANT: <ExternalEditToast/> and <Toaster/> must stay inside
        <ToastContext.Provider>. Toaster portals to document.body, so its DOM
        position is irrelevant, but if it falls outside this provider it
        silently reads the no-op handle and all toasts vanish with no type
        error. Keep these two adjacent and inside the provider. */}
    <ExternalEditToast />
    <Toaster />
  </ToastContext.Provider>
</UnseenDocTypesContext.Provider>
```

### Test-First:

**File**: `src/components/RootLayout/RootLayout.externalEdit.test.tsx` (new) —
model on `LibraryDocView.dispatch.test.tsx` and `use-move-work-item.test.tsx`
(SSE + router + QueryClient).

**Test seam (decided, not left to implementation).** `RootLayout` instantiates
the production `useDocEvents` singleton (`makeUseDocEvents((url) => new
EventSource(url))`, `:18`) and sets `DocEventsContext` to it itself, so an outer
provider is shadowed and there is no real-onmessage path under jsdom. There is no
way to both deliver an event AND run the real `dispatchSseEvent` through that
singleton. Therefore split coverage into two complementary test shapes:

1. **Correlation + toast (captured-listener shape)** — render a harness that
   provides a test `DocEventsHandle` (a real `subscribe` registry, e.g. from a
   `makeUseDocEvents(fakeFactory)` instance, or a hand-built handle exposing
   `subscribe`) plus `ToastContext`/router/QueryClient, with a seeded docs-list
   query so `/library/work-items/<slug>` resolves to a real `entry.relPath = X`
   (real resolution via `useActiveDocRelPath`, not two literals). Capture the
   listener registered with `subscribe` and invoke it with crafted events. This
   shape proves the toast appears/doesn't, but does NOT assert invalidation.
   Because `RootLayout` self-provides the production `useDocEvents` singleton
   (`:18`), this shape renders a **substitute tree** (`ToastContext.Provider
   value={useToastDispatcher()}` + `<ExternalEditToast/>` + `<Toaster/>` + test
   `DocEventsContext`/`SelfCauseContext`/router/QueryClient), NOT the real
   `RootLayout` — so it exercises subscriber + presentational wiring but not
   `RootLayout`'s provider nesting. To guard the RootLayout nesting INVARIANT,
   add one small assertion that renders the real `<RootLayout>` and confirms a
   toast dispatched through the provider it sets up actually renders in the
   `toaster-viewport` region — so moving `<Toaster/>`/`<ExternalEditToast/>`
   outside `ToastContext.Provider` fails a test, not just a manual check.
2. **Content refresh (direct-dispatch shape)** — call `dispatchSseEvent(event,
   queryClient)` directly against a `QueryClient` with a seeded `docContent(X)`
   query (`fetchDocContent` returning v1 then v2), asserting the cache for
   `docContent(X)` is invalidated and the rendered body shows v2 / drops v1 with
   no navigation. This proves the content-refresh AC against the real
   invalidation code path without needing the singleton's onmessage.

**Timer regime split (avoids the fake-timer × async-query flake).** Run the
correlation and content-refresh shapes on **real timers** (`findBy*` / `waitFor`
for query settling). Enable `vi.useFakeTimers()` ONLY in the dedicated
auto-dismiss/pause case below — or scope it with `vi.useFakeTimers({ toFake:
[...] })` leaving the microtask queue real if a single test needs both.

**Registry isolation (no shared-singleton pollution).** Construct a fresh
`createSelfCauseRegistry()` per test and inject it via
`SelfCauseContext.Provider` (the `use-move-work-item.test.tsx` pattern) for BOTH
the subscriber and the dispatch path, rather than mutating the module-level
`defaultSelfCauseRegistry`. Call `registry.reset()` in `beforeEach`. If fake
timers are used in a case that also exercises TTL, ensure the registry's clock is
the same mocked clock (mock `Date.now`).

Cases:

- **Positive correlation, per verb (shape 1)**: with relPath resolved to `X`,
  deliver `doc-changed` `path: X`, fresh etag not in the injected registry,
  `action: 'edited'` → a toast with heading "External edit detected" and message
  naming `X` with verb "updated" appears (queried via the `toaster-viewport`
  `role="status"` region / `data-testid="toaster-icon"`). Repeat for `created`
  ("created") and **`deleted`** ("deleted") — the deleted case explicitly
  verifies the pre-drop ordering still resolves `X` (regression guard for the
  delete-ordering invariant).
- **Content refresh AC (shape 2)**: per the direct-dispatch shape above — body
  shows a known v2 value, v1 gone, URL unchanged.
- **Different path (shape 1)**: event `path: Y` → no toast.
- **Self-caused (shape 1)**: register the event's etag in the **injected**
  registry first → no toast (and assert the same registry's `has(etag)` is
  `true`, confirming the dispatcher would also have dropped it).
- **Auto-dismiss + pause in context (fake timers)**: toast visible at 4 s, gone
  by 5.5 s; clicking close removes it earlier; hovering the toast
  (`mouseEnter`) before 5 s and advancing past 5 s keeps it visible, and
  `mouseLeave` then advancing 5 s removes it (pause-on-hover end-to-end).

### Success Criteria:

#### Automated Verification:
- [ ] Integration suite passes: `npm test src/components/RootLayout`
- [ ] Full suite passes: `npm test` (from `frontend/`)
- [ ] Type checking passes: `npm run typecheck`
- [ ] Production build succeeds: `npm run build`

#### Manual Verification:
- [ ] `npm run dev`, open a document detail page; from another process/tab edit
      that document's file → a bottom-right toast "External edit detected" with
      the correct relPath and verb appears, the body updates without reload, and
      the toast auto-dismisses ~5 s later.
- [ ] Editing the document **from this same browser session** (a self-caused
      write) raises **no** toast.
- [ ] Editing a **different** document raises no toast on the current page.
- [ ] Clicking the toast's close button dismisses it immediately; the close
      button is reachable and operable by keyboard (Tab + Enter/Space).
- [ ] Pressing `Escape` dismisses the most-recent toast; with multiple toasts,
      repeated `Escape` presses dismiss them in reverse order; with an empty
      stack `Escape` has no effect on other page state.
- [ ] Toast is legible in both light and dark themes (tokens resolve correctly),
      and the icon reads as informational/neutral (not success-green).
- [ ] Multiple rapid external edits stack and each dismisses on its own timer;
      beyond `MAX_TOASTS` (5) the oldest is dropped rather than the stack growing
      unbounded.
- [ ] Hovering a toast (or keyboard-focusing its close button) pauses its
      auto-dismiss; moving away / blurring resumes it (the toast does not vanish
      while being read or interacted with).
- [ ] Screen reader announces the toast text when it appears (single persistent
      live region), and a burst of stacked toasts does not garble or drop
      announcements. Confirm the announcement reads the heading + message
      cleanly and the close button's role isn't appended verbosely; if it is,
      move the close `<button>` out of the announced subtree (keep it in the
      card visually) rather than inside the `aria-live` region.

---

## Testing Strategy

### Unit Tests:
- `use-toast.test.tsx` — stack append/dismiss, unique ids, 5 s auto-dismiss
  window (4 s present / 5.5 s gone), staggered independent per-toast timers,
  pause/resume, `MAX_TOASTS` cap (oldest dropped), unmount cleanup (no
  post-unmount state update), manual-dismiss timer cancellation, no-op default.
- `Toaster.test.tsx` — icon (testid + actual `<svg>`)/heading/message/close
  rendering, close callback, pause/resume hover+focus wiring, Escape-to-dismiss
  most-recent (and listener removed on unmount), empty-stack (persistent
  viewport, no cards), stack of two, CSS token bindings incl. neutral icon token
  (not `--ac-ok`) (`?raw`).
- `use-active-doc-relpath.test.tsx` — slug + fileSlug-fallback match, off-route
  `undefined`, unloaded/unmatched `undefined`, invalid type.
- `use-external-edit-toast.test.tsx` — verb map per action, positive match,
  path mismatch, self-cause exclusion, undefined-etag → external, off-route,
  route-change-without-resubscribe, non-doc-changed ignored, unsubscribe on
  unmount.

### Integration Tests:
- `RootLayout.externalEdit.test.tsx` — two complementary shapes (see Phase 5):
  (1) captured-listener correlation with real route→relPath resolution —
  positive correlation per verb incl. `deleted`, path-mismatch and self-cause
  negatives, auto-dismiss + pause-on-hover + manual dismiss (fake timers,
  isolated); (2) direct `dispatchSseEvent` content-refresh to post-change value
  (real timers). Self-cause uses a per-test injected `createSelfCauseRegistry()`,
  not the shared singleton.

### Manual Testing Steps:
See Phase 5 Manual Verification.

## Performance Considerations

- The subscribe listener runs synchronously inside `onmessage`; it does only
  cheap comparisons and a `showToast` — negligible. The `ref` pattern avoids
  re-subscribing on every render/route change.
- Auto-dismiss timers are per-toast and cleared on manual dismiss, on stack-cap
  eviction, and on unmount (teardown `useEffect` clears the whole timer map) —
  no leaks, no post-unmount `setToasts`.
- The `Toaster` viewport stays mounted (it is the persistent `aria-live` region)
  but renders no cards when the stack is empty — a single empty container, not a
  growing DOM. `pointer-events: none` on the empty viewport keeps it
  click-through.
- The stack is capped at `MAX_TOASTS` (5), so rapid edit bursts cannot grow the
  rendered DOM (or the live-region announcement queue) without bound.

## Migration Notes

None — purely additive. No schema, storage, or API changes. No existing
behaviour changes (`LibraryDocView` is left untouched; the Phase 4 resolver is
new code).

## References

- Work item: `meta/work/0039-toaster-and-external-edit-notifications.md`
- Research: `meta/research/codebase/2026-05-27-0039-toaster-and-external-edit-notifications.md`
- Deferral note (doc ID): `meta/notes/2026-05-26-toast-correlation-should-use-document-id.md`
- Review (APPROVE): `meta/reviews/work/0039-...-review-1.md`
- Context/hook template: `src/api/use-theme.ts:8-78`, `src/api/use-unseen-doc-types.ts:63-131`
- Auto-dismiss + fake-timer template: `src/api/use-deferred-fetching-hint.ts:16-31` + `.test.tsx`
- Component trio + `?raw` token test: `src/components/SseIndicator/SseIndicator.{tsx,module.css,test.tsx}`
- SSE flow / dispatch: `src/api/use-doc-events.ts:202-232,89-126`
- Self-cause: `src/api/self-cause.ts:39-56`
- Route→relPath: `src/routes/library/LibraryDocView.tsx:37-57`
- Test harness: `src/test/router-helpers.tsx:32`
- Tokens: `src/styles/global.css` (`--ac-bg-card`:85, `--ac-shadow-lift`:202, status:99-101)
