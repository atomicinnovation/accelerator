import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useDeferredFetchingHint } from './use-deferred-fetching-hint'

describe('useDeferredFetchingHint', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  // ── Step 6.5d ───────────────────────────────────────────────────────
  it('stays false for sub-250ms refetches', () => {
    const { result, rerender } = renderHook(
      ({ q }: { q: { isFetching: boolean; isPending: boolean } }) =>
        useDeferredFetchingHint(q),
      { initialProps: { q: { isFetching: false, isPending: false } } },
    )
    expect(result.current).toBe(false)

    // Refetch starts.
    rerender({ q: { isFetching: true, isPending: false } })
    expect(result.current).toBe(false)

    // Advance 100ms — still under the threshold.
    act(() => {
      vi.advanceTimersByTime(100)
    })
    expect(result.current).toBe(false)

    // Refetch ends before 250ms.
    rerender({ q: { isFetching: false, isPending: false } })
    act(() => {
      vi.advanceTimersByTime(200)
    })
    expect(result.current).toBe(false)
  })

  it('flips to true after the 250ms threshold for a long-running refetch', () => {
    const { result, rerender } = renderHook(
      ({ q }: { q: { isFetching: boolean; isPending: boolean } }) =>
        useDeferredFetchingHint(q),
      { initialProps: { q: { isFetching: false, isPending: false } } },
    )
    rerender({ q: { isFetching: true, isPending: false } })

    act(() => {
      vi.advanceTimersByTime(250)
    })
    expect(result.current).toBe(true)
  })

  // ── Step 6.5e ───────────────────────────────────────────────────────
  it('hint resets to false on pending (initial load is not a refetch)', () => {
    const { result } = renderHook(() =>
      useDeferredFetchingHint({ isFetching: true, isPending: true }),
    )
    act(() => {
      vi.advanceTimersByTime(500)
    })
    expect(result.current).toBe(false)
  })
})
