import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { makeUseTheme } from './use-theme'

function resetDom(): void {
  document.documentElement.removeAttribute('data-theme')
  localStorage.clear()
}

describe('makeUseTheme', () => {
  beforeEach(resetDom)
  afterEach(resetDom)

  it('initial state reflects pre-existing data-theme attribute', () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('attribute takes precedence over conflicting localStorage', () => {
    document.documentElement.setAttribute('data-theme', 'dark')
    localStorage.setItem('ac-theme', 'light')
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('localStorage takes precedence over OS preference when no attribute', () => {
    localStorage.setItem('ac-theme', 'light')
    const useTheme = makeUseTheme(() => true) // OS prefers dark
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('light')
  })

  it('initial state reads localStorage when no attribute is present', () => {
    localStorage.setItem('ac-theme', 'dark')
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('initial state falls back to OS preference when no attribute or storage', () => {
    const useTheme = makeUseTheme(() => true) // OS prefers dark
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('setTheme writes the attribute on <html>', () => {
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    act(() => result.current.setTheme('dark'))
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('setTheme persists to localStorage under "ac-theme"', () => {
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    act(() => result.current.setTheme('dark'))
    expect(localStorage.getItem('ac-theme')).toBe('dark')
  })

  it('toggleTheme flips light → dark and back', () => {
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    act(() => result.current.setTheme('light'))
    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('dark')
    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('light')
  })

  it('does not throw when localStorage.setItem throws (private mode)', () => {
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const useTheme = makeUseTheme(() => false)
    const { result } = renderHook(() => useTheme())
    expect(() => act(() => result.current.setTheme('dark'))).not.toThrow()
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    setItemSpy.mockRestore()
  })

  it('does not throw when localStorage.getItem throws on init', () => {
    const getItemSpy = vi
      .spyOn(Storage.prototype, 'getItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const useTheme = makeUseTheme(() => true) // OS prefers dark
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
    getItemSpy.mockRestore()
  })

  it('rejects invalid stored values and falls back to OS preference', () => {
    localStorage.setItem('ac-theme', 'midnight') // not a valid Theme
    const useTheme = makeUseTheme(() => true)
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })
})
