import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useDebouncedValue } from './use-debounced-value'

describe('useDebouncedValue', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('returns initial value synchronously', () => {
    const { result } = renderHook(() => useDebouncedValue('a', 200))
    expect(result.current).toBe('a')
  })

  it('updates after delay with no intervening changes', () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) => useDebouncedValue(value, 200),
      { initialProps: { value: 'a' } },
    )
    rerender({ value: 'b' })
    expect(result.current).toBe('a')
    act(() => {
      vi.advanceTimersByTime(200)
    })
    expect(result.current).toBe('b')
  })

  it('resets timer on change within delay window', () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) => useDebouncedValue(value, 200),
      { initialProps: { value: 'ab' } },
    )
    rerender({ value: 'abc' })
    act(() => {
      vi.advanceTimersByTime(50)
    })
    rerender({ value: 'ab' })
    act(() => {
      vi.advanceTimersByTime(50)
    })
    expect(result.current).toBe('ab')
    act(() => {
      vi.advanceTimersByTime(200)
    })
    expect(result.current).toBe('ab')
  })

  it('respects custom delayMs', () => {
    const { result, rerender } = renderHook(
      ({ value }: { value: string }) => useDebouncedValue(value, 50),
      { initialProps: { value: 'a' } },
    )
    rerender({ value: 'b' })
    act(() => {
      vi.advanceTimersByTime(50)
    })
    expect(result.current).toBe('b')
  })

  it('cleans up pending timer on unmount', () => {
    const { rerender, unmount } = renderHook(
      ({ value }: { value: string }) => useDebouncedValue(value, 200),
      { initialProps: { value: 'a' } },
    )
    rerender({ value: 'b' })
    expect(() => {
      unmount()
      vi.advanceTimersByTime(500)
    }).not.toThrow()
  })
})
