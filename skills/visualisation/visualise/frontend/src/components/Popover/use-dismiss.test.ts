import { renderHook, act } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { createRef } from 'react'
import { useDismiss } from './use-dismiss'

function dispatchMouseDown(target: EventTarget) {
  const event = new MouseEvent('mousedown', { bubbles: true })
  Object.defineProperty(event, 'target', { value: target, writable: false })
  document.dispatchEvent(event)
}

function dispatchKey(key: string) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true })
  document.dispatchEvent(event)
}

describe('useDismiss', () => {
  it('does not bind listeners when open is false', () => {
    const onDismiss = vi.fn()
    const ref = createRef<HTMLDivElement>()
    Object.defineProperty(ref, 'current', { value: document.createElement('div'), writable: true })
    renderHook(() => useDismiss(false, ref, onDismiss))
    act(() => dispatchMouseDown(document.body))
    act(() => dispatchKey('Escape'))
    expect(onDismiss).not.toHaveBeenCalled()
  })

  it('fires onDismiss on mousedown outside the referenced element', () => {
    const onDismiss = vi.fn()
    const inside = document.createElement('div')
    const outside = document.createElement('div')
    document.body.append(inside, outside)
    const ref = { current: inside } as React.RefObject<HTMLElement>
    renderHook(() => useDismiss(true, ref, onDismiss))
    act(() => dispatchMouseDown(outside))
    expect(onDismiss).toHaveBeenCalledTimes(1)
    inside.remove()
    outside.remove()
  })

  it('does not fire onDismiss on mousedown inside the referenced element', () => {
    const onDismiss = vi.fn()
    const inside = document.createElement('div')
    const child = document.createElement('span')
    inside.append(child)
    document.body.append(inside)
    const ref = { current: inside } as React.RefObject<HTMLElement>
    renderHook(() => useDismiss(true, ref, onDismiss))
    act(() => dispatchMouseDown(child))
    expect(onDismiss).not.toHaveBeenCalled()
    inside.remove()
  })

  it('fires onDismiss on Escape', () => {
    const onDismiss = vi.fn()
    const ref = { current: document.createElement('div') } as React.RefObject<HTMLElement>
    renderHook(() => useDismiss(true, ref, onDismiss))
    act(() => dispatchKey('Escape'))
    expect(onDismiss).toHaveBeenCalledTimes(1)
  })

  it('ignores non-Escape keys', () => {
    const onDismiss = vi.fn()
    const ref = { current: document.createElement('div') } as React.RefObject<HTMLElement>
    renderHook(() => useDismiss(true, ref, onDismiss))
    act(() => dispatchKey('a'))
    act(() => dispatchKey('Enter'))
    expect(onDismiss).not.toHaveBeenCalled()
  })

  it('removes listeners on unmount', () => {
    const onDismiss = vi.fn()
    const ref = { current: document.createElement('div') } as React.RefObject<HTMLElement>
    const { unmount } = renderHook(() => useDismiss(true, ref, onDismiss))
    unmount()
    act(() => dispatchKey('Escape'))
    expect(onDismiss).not.toHaveBeenCalled()
  })

  it('removes listeners when open flips to false', () => {
    const onDismiss = vi.fn()
    const ref = { current: document.createElement('div') } as React.RefObject<HTMLElement>
    const { rerender } = renderHook(({ open }: { open: boolean }) => useDismiss(open, ref, onDismiss), {
      initialProps: { open: true },
    })
    rerender({ open: false })
    act(() => dispatchKey('Escape'))
    expect(onDismiss).not.toHaveBeenCalled()
  })

  it('stacks multiple instances independently', () => {
    const onDismissA = vi.fn()
    const onDismissB = vi.fn()
    const refA = { current: document.createElement('div') } as React.RefObject<HTMLElement>
    const refB = { current: document.createElement('div') } as React.RefObject<HTMLElement>
    renderHook(() => useDismiss(true, refA, onDismissA))
    renderHook(() => useDismiss(true, refB, onDismissB))
    act(() => dispatchKey('Escape'))
    expect(onDismissA).toHaveBeenCalledTimes(1)
    expect(onDismissB).toHaveBeenCalledTimes(1)
  })
})
