import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useFontMode } from './use-font-mode'

function resetDom(): void {
  document.documentElement.removeAttribute('data-font')
  localStorage.clear()
}

describe('useFontMode', () => {
  beforeEach(resetDom)
  afterEach(resetDom)

  it('initial state reflects pre-existing data-font attribute', () => {
    document.documentElement.setAttribute('data-font', 'mono')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('mono')
  })

  it('initial state reads localStorage when no attribute is present', () => {
    localStorage.setItem('ac-font-mode', 'mono')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('mono')
  })

  it('defaults to "display" when no attribute or storage', () => {
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('display')
  })

  it('attribute takes precedence over conflicting localStorage', () => {
    document.documentElement.setAttribute('data-font', 'mono')
    localStorage.setItem('ac-font-mode', 'display')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('mono')
  })

  it('setFontMode writes the attribute on <html>', () => {
    const { result } = renderHook(() => useFontMode())
    act(() => result.current.setFontMode('mono'))
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
  })

  it('setFontMode persists to localStorage under "ac-font-mode"', () => {
    const { result } = renderHook(() => useFontMode())
    act(() => result.current.setFontMode('mono'))
    expect(localStorage.getItem('ac-font-mode')).toBe('mono')
  })

  it('toggleFontMode flips display → mono and back', () => {
    const { result } = renderHook(() => useFontMode())
    act(() => result.current.setFontMode('display'))
    act(() => result.current.toggleFontMode())
    expect(result.current.fontMode).toBe('mono')
    act(() => result.current.toggleFontMode())
    expect(result.current.fontMode).toBe('display')
  })

  it('does not throw when localStorage.setItem throws', () => {
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const { result } = renderHook(() => useFontMode())
    expect(() => act(() => result.current.setFontMode('mono'))).not.toThrow()
    expect(document.documentElement.getAttribute('data-font')).toBe('mono')
    setItemSpy.mockRestore()
  })

  it('rejects invalid stored values and falls back to "display"', () => {
    localStorage.setItem('ac-font-mode', 'serif')
    const { result } = renderHook(() => useFontMode())
    expect(result.current.fontMode).toBe('display')
  })
})
