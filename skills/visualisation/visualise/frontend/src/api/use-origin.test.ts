import { describe, it, expect, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useOrigin } from './use-origin'

describe('useOrigin()', () => {
  it('returns window.location.host by default', () => {
    const { result } = renderHook(() => useOrigin())
    expect(result.current).toBe(window.location.host)
  })

  it('reads the injected reader exactly once across multiple rerenders', () => {
    const reader = vi.fn().mockReturnValue('initial.example')
    const { rerender } = renderHook(() => useOrigin(reader))
    rerender()
    rerender()
    expect(reader).toHaveBeenCalledTimes(1)
  })

  it('returns the value read at mount even when reader changes its return value', () => {
    const reader = vi.fn().mockReturnValue('initial.example')
    const { result, rerender } = renderHook(() => useOrigin(reader))
    expect(result.current).toBe('initial.example')
    reader.mockReturnValue('changed.example')
    rerender()
    expect(result.current).toBe('initial.example')
  })
})
