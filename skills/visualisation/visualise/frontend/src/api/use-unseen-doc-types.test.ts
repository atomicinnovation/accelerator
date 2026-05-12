import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import React from 'react'
import {
  useUnseenDocTypes,
  useUnseenDocTypesContext,
  UnseenDocTypesContext,
  SEEN_DOC_TYPES_STORAGE_KEY,
  type UnseenDocTypesHandle,
} from './use-unseen-doc-types'
import type { SseEvent } from './types'

function resetDom(): void {
  localStorage.clear()
}

const changed = (docType: string, etag = 'sha256-abc'): SseEvent =>
  ({ type: 'doc-changed', docType, path: `/x/${docType}`, etag } as SseEvent)

const invalid = (docType: string): SseEvent =>
  ({ type: 'doc-invalid', docType, path: `/x/${docType}` } as SseEvent)

describe('useUnseenDocTypes', () => {
  beforeEach(() => {
    resetDom()
    vi.useFakeTimers()
    vi.setSystemTime(new Date(1000))
  })
  afterEach(() => {
    vi.useRealTimers()
    resetDom()
  })

  it('initial render with empty storage has empty unseenSet and no write', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    expect(result.current.unseenSet.size).toBe(0)
    expect(localStorage.getItem(SEEN_DOC_TYPES_STORAGE_KEY)).toBeNull()
  })

  it('first doc-changed for a never-visited type: no dot, no write', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.size).toBe(0)
    expect(localStorage.getItem(SEEN_DOC_TYPES_STORAGE_KEY)).toBeNull()
  })

  it('event after markSeen and time advance raises the dot', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('work-items'))
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(true)
  })

  it('equal-T event does not raise the dot (strict gt)', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('work-items'))
    // System time still at 1000
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(false)
  })

  it('markSeen bumps T, clears the dot, and writes once', () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem')
    const { result } = renderHook(() => useUnseenDocTypes())

    // Raise the dot first
    act(() => result.current.markSeen('work-items'))
    setItemSpy.mockClear()
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(true)

    // Now clear it
    act(() => { vi.setSystemTime(new Date(3000)) })
    act(() => result.current.markSeen('work-items'))
    expect(result.current.unseenSet.has('work-items')).toBe(false)

    const raw = localStorage.getItem(SEEN_DOC_TYPES_STORAGE_KEY)
    expect(raw).not.toBeNull()
    const parsed = JSON.parse(raw!)
    expect(parsed['work-items']).toBe(3000)
    expect(typeof parsed['work-items']).toBe('number')
    expect(setItemSpy).toHaveBeenCalledTimes(1)
    setItemSpy.mockRestore()
  })

  it('onEvent does not write to storage under any condition', () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem')
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('work-items'))
    setItemSpy.mockClear()
    for (let i = 0; i < 50; i++) {
      const types = ['decisions', 'plans', 'research', 'work-items']
      act(() => result.current.onEvent(changed(types[i % types.length])))
    }
    expect(setItemSpy).not.toHaveBeenCalled()
    setItemSpy.mockRestore()
  })

  it('doc-invalid events are ignored', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('work-items'))
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onEvent(invalid('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(false)
  })

  it('onReconnect is a no-op', () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem')
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('work-items'))
    setItemSpy.mockClear()
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onReconnect())
    expect(result.current.unseenSet.size).toBe(0)
    expect(setItemSpy).not.toHaveBeenCalled()
    setItemSpy.mockRestore()
  })

  it('persistence round-trip of markSeen values, transient state does not survive remount', () => {
    const { result, unmount } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('decisions'))
    // Raise a dot too
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onEvent(changed('decisions')))
    expect(result.current.unseenSet.has('decisions')).toBe(true)
    const raw = localStorage.getItem(SEEN_DOC_TYPES_STORAGE_KEY)
    expect(JSON.parse(raw!)['decisions']).toBe(1000)
    unmount()

    // Remount via fresh renderHook
    const { result: result2 } = renderHook(() => useUnseenDocTypes())
    expect(result2.current.unseenSet.size).toBe(0)
    // Strict gt: event at t=999 → no dot
    act(() => { vi.setSystemTime(new Date(999)) })
    act(() => result2.current.onEvent(changed('decisions')))
    expect(result2.current.unseenSet.has('decisions')).toBe(false)
    // Event at t=1001 → dot
    act(() => { vi.setSystemTime(new Date(1001)) })
    act(() => result2.current.onEvent(changed('decisions')))
    expect(result2.current.unseenSet.has('decisions')).toBe(true)
  })

  it('malformed storage — "not json" — is treated as empty', () => {
    localStorage.setItem(SEEN_DOC_TYPES_STORAGE_KEY, 'not json')
    const { result } = renderHook(() => useUnseenDocTypes())
    expect(result.current.unseenSet.size).toBe(0)
  })

  it('malformed storage — JSON array — is treated as empty', () => {
    localStorage.setItem(SEEN_DOC_TYPES_STORAGE_KEY, '[1,2,3]')
    const { result } = renderHook(() => useUnseenDocTypes())
    expect(result.current.unseenSet.size).toBe(0)
  })

  it('malformed storage — non-numeric values dropped', () => {
    localStorage.setItem(
      SEEN_DOC_TYPES_STORAGE_KEY,
      JSON.stringify({ 'work-items': 'banana' }),
    )
    const { result } = renderHook(() => useUnseenDocTypes())
    // 'work-items' is treated as never-seen, so first event absorbs silently
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(false)
  })

  it('malformed storage — unknown DocTypeKey is filtered, valid entry survives', () => {
    localStorage.setItem(
      SEEN_DOC_TYPES_STORAGE_KEY,
      JSON.stringify({ 'made-up-type': 1000, 'work-items': 1000 }),
    )
    const { result } = renderHook(() => useUnseenDocTypes())
    // Strict gt: event at t=999 → no dot
    act(() => { vi.setSystemTime(new Date(999)) })
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(false)
    // Event at t=1001 → dot
    act(() => { vi.setSystemTime(new Date(1001)) })
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(true)
  })

  it('safeSetItem failure does not propagate; in-memory state still updates', () => {
    const setItemSpy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    const { result } = renderHook(() => useUnseenDocTypes())
    // Raise the dot first
    act(() => result.current.markSeen('work-items'))
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onEvent(changed('work-items')))
    expect(result.current.unseenSet.has('work-items')).toBe(true)

    expect(() => act(() => result.current.markSeen('work-items'))).not.toThrow()
    expect(result.current.unseenSet.has('work-items')).toBe(false)
    setItemSpy.mockRestore()
  })

  it('unseenSet identity stable across repeat events for already-unseen type', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('decisions'))
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onEvent(changed('decisions')))
    const setA = result.current.unseenSet
    act(() => result.current.onEvent(changed('decisions')))
    const setB = result.current.unseenSet
    expect(Object.is(setA, setB)).toBe(true)
  })

  it('reactivity propagates through Context', () => {
    function Child() {
      const { unseenSet } = useUnseenDocTypesContext()
      return React.createElement('span', null, unseenSet.has('research') ? 'yes' : 'no')
    }
    let handle: UnseenDocTypesHandle | null = null
    function Host() {
      handle = useUnseenDocTypes()
      return React.createElement(
        UnseenDocTypesContext.Provider,
        { value: handle },
        React.createElement(Child),
      )
    }
    const { container } = require('@testing-library/react').render(
      React.createElement(Host),
    )
    expect(container.textContent).toBe('no')
    act(() => handle!.markSeen('research'))
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => handle!.onEvent(changed('research')))
    expect(container.textContent).toBe('yes')
  })

  it('markSeen then synchronous onEvent sees the updated T (strict-gt boundary)', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => {
      result.current.markSeen('decisions')
      // Same JS turn, same system time = 1000
      result.current.onEvent(changed('decisions'))
    })
    expect(result.current.unseenSet.has('decisions')).toBe(false)
  })

  it('reconnect then replayed doc-changed raises the dot', () => {
    const { result } = renderHook(() => useUnseenDocTypes())
    act(() => result.current.markSeen('decisions'))
    act(() => { vi.setSystemTime(new Date(2000)) })
    act(() => result.current.onReconnect())
    act(() => result.current.onEvent(changed('decisions')))
    expect(result.current.unseenSet.has('decisions')).toBe(true)
  })
})
