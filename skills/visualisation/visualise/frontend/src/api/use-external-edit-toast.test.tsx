import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import type { SseEvent, ActionKind } from './types'

vi.mock('./use-doc-events', () => ({ useDocEventsContext: vi.fn() }))
vi.mock('./self-cause', () => ({ useSelfCauseRegistry: vi.fn() }))
vi.mock('./use-active-doc-relpath', () => ({ useActiveDocRelPath: vi.fn() }))
vi.mock('./use-toast', () => ({ useToast: vi.fn() }))

import { useDocEventsContext } from './use-doc-events'
import { useSelfCauseRegistry } from './self-cause'
import { useActiveDocRelPath } from './use-active-doc-relpath'
import { useToast } from './use-toast'
import {
  externalEditMessage,
  EXTERNAL_EDIT_HEADING,
  useExternalEditToast,
} from './use-external-edit-toast'

function setupMocks(opts: {
  relPath?: string
  hasEtag?: boolean
} = {}) {
  const subscribe = vi.fn().mockImplementation((_listener: (e: SseEvent) => void) => {
    return () => {}
  })
  const showToast = vi.fn()
  vi.mocked(useDocEventsContext).mockReturnValue({
    subscribe,
    setDragInProgress: vi.fn(),
    connectionState: 'open',
    justReconnected: false,
  } as any)
  vi.mocked(useSelfCauseRegistry).mockReturnValue({
    has: vi.fn().mockReturnValue(opts.hasEtag ?? false),
    register: vi.fn(),
    reset: vi.fn(),
  })
  vi.mocked(useActiveDocRelPath).mockReturnValue(opts.relPath)
  vi.mocked(useToast).mockReturnValue({
    toasts: [],
    showToast,
    dismissToast: vi.fn(),
    pauseToast: vi.fn(),
    resumeToast: vi.fn(),
  })
  return { subscribe, showToast }
}

function captureListener(subscribe: ReturnType<typeof vi.fn>) {
  expect(subscribe).toHaveBeenCalledTimes(1)
  return subscribe.mock.calls[0][0] as (e: SseEvent) => void
}

function makeDocChanged(
  path: string,
  action: ActionKind,
  etag?: string,
): SseEvent {
  return {
    type: 'doc-changed',
    action,
    docType: 'work-items',
    path,
    etag,
    timestamp: '2026-05-30T00:00:00Z',
  }
}

describe('externalEditMessage', () => {
  it('formats created', () => {
    expect(externalEditMessage('a/b.md', 'created')).toBe(
      '`a/b.md` was created while you were looking at it.',
    )
  })
  it('formats edited as "updated"', () => {
    expect(externalEditMessage('a/b.md', 'edited')).toBe(
      '`a/b.md` was updated while you were looking at it.',
    )
  })
  it('formats deleted', () => {
    expect(externalEditMessage('a/b.md', 'deleted')).toBe(
      '`a/b.md` was deleted while you were looking at it.',
    )
  })
})

describe('useExternalEditToast', () => {
  beforeEach(() => vi.clearAllMocks())

  it('raises toast when event matches active relPath (edited → updated)', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    renderHook(() => useExternalEditToast())
    const listener = captureListener(subscribe)
    listener(makeDocChanged('X', 'edited', 'etag-1'))
    expect(showToast).toHaveBeenCalledTimes(1)
    expect(showToast).toHaveBeenCalledWith({
      heading: EXTERNAL_EDIT_HEADING,
      message: '`X` was updated while you were looking at it.',
    })
  })

  it('raises toast for created → created', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    renderHook(() => useExternalEditToast())
    captureListener(subscribe)(makeDocChanged('X', 'created', 'e'))
    expect(showToast).toHaveBeenCalledWith({
      heading: EXTERNAL_EDIT_HEADING,
      message: '`X` was created while you were looking at it.',
    })
  })

  it('raises toast for deleted → deleted', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    renderHook(() => useExternalEditToast())
    captureListener(subscribe)(makeDocChanged('X', 'deleted', 'e'))
    expect(showToast).toHaveBeenCalledWith({
      heading: EXTERNAL_EDIT_HEADING,
      message: '`X` was deleted while you were looking at it.',
    })
  })

  it('does nothing when event path differs from active relPath', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    renderHook(() => useExternalEditToast())
    captureListener(subscribe)(makeDocChanged('Y', 'edited', 'e'))
    expect(showToast).not.toHaveBeenCalled()
  })

  it('does nothing for self-caused events (etag in registry)', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X', hasEtag: true })
    renderHook(() => useExternalEditToast())
    captureListener(subscribe)(makeDocChanged('X', 'edited', 'self-etag'))
    expect(showToast).not.toHaveBeenCalled()
  })

  it('treats event without etag as external (registry.has(undefined) === false)', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    renderHook(() => useExternalEditToast())
    captureListener(subscribe)(makeDocChanged('X', 'edited', undefined))
    expect(showToast).toHaveBeenCalledTimes(1)
  })

  it('does nothing when off-route (relPath undefined)', () => {
    const { subscribe, showToast } = setupMocks({ relPath: undefined })
    renderHook(() => useExternalEditToast())
    captureListener(subscribe)(makeDocChanged('X', 'edited', 'e'))
    expect(showToast).not.toHaveBeenCalled()
  })

  it('ignores non-doc-changed events', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    renderHook(() => useExternalEditToast())
    const listener = captureListener(subscribe)
    listener({ type: 'doc-invalid', docType: 'work-items', path: 'X' })
    listener({
      type: 'template-changed',
      template: 'foo',
      timestamp: '2026-05-30T00:00:00Z',
    })
    expect(showToast).not.toHaveBeenCalled()
  })

  it('route change does not re-subscribe (subscribe called exactly once)', () => {
    const { subscribe, showToast } = setupMocks({ relPath: 'X' })
    const { rerender } = renderHook(() => useExternalEditToast())
    expect(subscribe).toHaveBeenCalledTimes(1)
    const listener = subscribe.mock.calls[0][0] as (e: SseEvent) => void
    vi.mocked(useActiveDocRelPath).mockReturnValue('Y')
    rerender()
    expect(subscribe).toHaveBeenCalledTimes(1)
    listener(makeDocChanged('Y', 'edited', 'e'))
    expect(showToast).toHaveBeenCalledWith({
      heading: EXTERNAL_EDIT_HEADING,
      message: '`Y` was updated while you were looking at it.',
    })
  })

  it('calls returned unsubscribe on unmount', () => {
    const unsubscribe = vi.fn()
    setupMocks({ relPath: 'X' })
    vi.mocked(useDocEventsContext).mockReturnValue({
      subscribe: vi.fn().mockReturnValue(unsubscribe),
      setDragInProgress: vi.fn(),
      connectionState: 'open',
      justReconnected: false,
    } as any)
    const { unmount } = renderHook(() => useExternalEditToast())
    unmount()
    expect(unsubscribe).toHaveBeenCalledTimes(1)
  })
})
