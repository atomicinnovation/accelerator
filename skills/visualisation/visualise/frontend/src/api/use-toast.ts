import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'

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
  autoDismissMs: number = TOAST_AUTO_DISMISS_MS,
): ToastHandle {
  const [toasts, setToasts] = useState<Toast[]>([])
  const toastsRef = useRef<Toast[]>(toasts)
  toastsRef.current = toasts
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
    ({ heading, message, kind = 'info' }: ShowToastInput): number => {
      const id = nextIdRef.current++
      // Eviction policy: `error` toasts persist and are EXEMPT from the cap —
      // only the auto-dismissing kinds (`info`/`ok`) are capped at MAX_TOASTS
      // (oldest dropped first). This stops a burst of later toasts silently
      // evicting a persistent error the user has not acknowledged.
      setToasts((prev) => {
        const next = [...prev, { id, heading, message, kind }]
        const capped = new Set(next.filter((t) => t.kind !== 'error').slice(-MAX_TOASTS))
        return next.filter((t) => t.kind === 'error' || capped.has(t))
      })
      // Kind-aware auto-dismiss: `info`/`ok` auto-dismiss after the configured
      // window; `error` toasts persist (no timer armed).
      if (kind !== 'error') arm(id)
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
