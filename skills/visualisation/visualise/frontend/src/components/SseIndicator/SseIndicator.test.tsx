import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import sseCss from './SseIndicator.module.css?raw'
import { SseIndicator } from './SseIndicator'

vi.mock('../../api/use-doc-events', () => ({
  useDocEventsContext: vi.fn(),
}))

import { useDocEventsContext } from '../../api/use-doc-events'

function mockState(connectionState: string) {
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState,
    justReconnected: false,
    setDragInProgress: vi.fn(),
  } as any)
}

describe('SseIndicator', () => {
  it('open state: data-state="open", no data-animated', () => {
    mockState('open')
    render(<SseIndicator />)
    const indicator = document.querySelector('[data-state="open"]')
    expect(indicator).not.toBeNull()
    expect(indicator?.getAttribute('data-animated')).toBeNull()
  })

  it('reconnecting state: data-state="reconnecting" and data-animated="true"', () => {
    mockState('reconnecting')
    render(<SseIndicator />)
    const indicator = document.querySelector('[data-state="reconnecting"]')
    expect(indicator).not.toBeNull()
    expect(indicator?.getAttribute('data-animated')).toBe('true')
  })

  it('connecting state: data-state="connecting", no data-animated', () => {
    mockState('connecting')
    render(<SseIndicator />)
    const indicator = document.querySelector('[data-state="connecting"]')
    expect(indicator).not.toBeNull()
    expect(indicator?.getAttribute('data-animated')).toBeNull()
  })

  it('closed state: data-state="closed", no data-animated', () => {
    mockState('closed')
    render(<SseIndicator />)
    const indicator = document.querySelector('[data-state="closed"]')
    expect(indicator).not.toBeNull()
    expect(indicator?.getAttribute('data-animated')).toBeNull()
  })

  it('open: aria-label is "SSE connection: open"', () => {
    mockState('open')
    render(<SseIndicator />)
    expect(screen.getByLabelText('SSE connection: open')).toBeInTheDocument()
  })

  it('reconnecting: aria-label is "SSE connection: reconnecting"', () => {
    mockState('reconnecting')
    render(<SseIndicator />)
    expect(screen.getByLabelText('SSE connection: reconnecting')).toBeInTheDocument()
  })

  it('connecting: aria-label is "SSE connection: connecting"', () => {
    mockState('connecting')
    render(<SseIndicator />)
    expect(screen.getByLabelText('SSE connection: connecting')).toBeInTheDocument()
  })

  it('closed: aria-label is "SSE connection: closed"', () => {
    mockState('closed')
    render(<SseIndicator />)
    expect(screen.getByLabelText('SSE connection: closed')).toBeInTheDocument()
  })

  describe('CSS source assertions', () => {
    it("[data-state='open'] binds --ac-ok color", () => {
      expect(sseCss).toContain("[data-state='open']")
      expect(sseCss).toMatch(/\[data-state='open'\][^{]*\{[^}]*color:\s*var\(--ac-ok\)/)
    })

    it("[data-state='reconnecting'] binds --ac-warn color", () => {
      expect(sseCss).toMatch(/\[data-state='reconnecting'\][^{]*\{[^}]*color:\s*var\(--ac-warn\)/)
    })

    it("[data-state='connecting'] binds --ac-fg-faint color", () => {
      expect(sseCss).toMatch(/\[data-state='connecting'\][^{]*\{[^}]*color:\s*var\(--ac-fg-faint\)/)
    })

    it("[data-state='closed'] binds --ac-err color", () => {
      expect(sseCss).toMatch(/\[data-state='closed'\][^{]*\{[^}]*color:\s*var\(--ac-err\)/)
    })

    it("[data-animated='true'] binds animation: ac-pulse", () => {
      expect(sseCss).toMatch(/\[data-animated='true'\][^{]*\{[^}]*animation:\s*ac-pulse/)
    })

    it('has @media (prefers-reduced-motion: reduce) block disabling animation', () => {
      expect(sseCss).toMatch(
        /@media\s*\(prefers-reduced-motion:\s*reduce\)[^}]*\{[^}]*animation:\s*none/s,
      )
    })
  })
})
