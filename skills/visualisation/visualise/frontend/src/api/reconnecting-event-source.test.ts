import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  ReconnectingEventSource,
  computeBackoff,
} from './reconnecting-event-source'

describe('computeBackoff', () => {
  it('starts at 1s and doubles', () => {
    expect(computeBackoff(0, 0.5)).toBeCloseTo(1000, -2)
    expect(computeBackoff(1, 0.5)).toBeCloseTo(2000, -2)
    expect(computeBackoff(2, 0.5)).toBeCloseTo(4000, -2)
  })

  it('caps at 30s', () => {
    expect(computeBackoff(99, 0.5)).toBeCloseTo(30000, -2)
  })

  it('caps cleanly even at extreme attempt values', () => {
    const v = computeBackoff(1000, 0.5)
    expect(Number.isFinite(v)).toBe(true)
    expect(v).toBeCloseTo(30000, -2)
  })

  it('applies +/-20% jitter across the seed range', () => {
    const samples = Array.from({ length: 100 }, (_, i) => computeBackoff(2, i / 100))
    const min = Math.min(...samples)
    const max = Math.max(...samples)
    expect(min).toBeGreaterThanOrEqual(4000 * 0.8)
    expect(max).toBeLessThanOrEqual(4000 * 1.2)
  })
})

describe('ReconnectingEventSource', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  function makeFakeFactory() {
    const fakes: Array<{
      onopen: ((e: any) => void) | null
      onerror: ((e: any) => void) | null
      onmessage: ((e: any) => void) | null
      close: ReturnType<typeof vi.fn>
    }> = []
    const factory = vi.fn(() => {
      const fake = {
        onopen: null as any,
        onerror: null as any,
        onmessage: null as any,
        close: vi.fn(),
      }
      fakes.push(fake)
      return fake as unknown as EventSource
    })
    return { fakes, factory }
  }

  it('opens, errors, then reconnects after the deterministic backoff', () => {
    const { fakes, factory } = makeFakeFactory()
    const onReconnect = vi.fn()
    new ReconnectingEventSource('/api/events', {
      factory,
      onReconnect,
      random: () => 0,
    })
    expect(factory).toHaveBeenCalledTimes(1)
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    expect(fakes[0].close).toHaveBeenCalled()

    vi.advanceTimersByTime(799)
    expect(factory).toHaveBeenCalledTimes(1)

    vi.advanceTimersByTime(2)
    expect(factory).toHaveBeenCalledTimes(2)

    fakes[1].onopen?.({})
    expect(onReconnect).toHaveBeenCalledTimes(1)
  })

  it('does not call onReconnect on the very first open', () => {
    const { fakes, factory } = makeFakeFactory()
    const onReconnect = vi.fn()
    new ReconnectingEventSource('/api/events', { factory, onReconnect, random: () => 0 })
    fakes[0].onopen?.({})
    expect(onReconnect).not.toHaveBeenCalled()
  })

  it('fires onReconnect on first-ever-open after a from-boot error', () => {
    const { fakes, factory } = makeFakeFactory()
    const onReconnect = vi.fn()
    new ReconnectingEventSource('/api/events', { factory, onReconnect, random: () => 0 })
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(801)
    fakes[1].onopen?.({})
    expect(onReconnect).toHaveBeenCalledTimes(1)
  })

  it('reports connecting from the constructor before the first open', () => {
    const { factory } = makeFakeFactory()
    const states: string[] = []
    new ReconnectingEventSource('/api/events', {
      factory,
      random: () => 0,
      onStateChange: s => states.push(s),
    })
    expect(states).toEqual(['connecting'])
  })

  it('exposes a state observable: connecting → open → reconnecting → open → closed', () => {
    const { fakes, factory } = makeFakeFactory()
    const states: string[] = []
    const r = new ReconnectingEventSource('/api/events', {
      factory,
      random: () => 0,
      onStateChange: s => states.push(s),
    })
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(801)
    fakes[1].onopen?.({})
    r.close()
    expect(states).toEqual(['connecting', 'open', 'reconnecting', 'open', 'closed'])
  })

  it('reports connecting then reconnecting on a from-boot error', () => {
    const { fakes, factory } = makeFakeFactory()
    const states: string[] = []
    new ReconnectingEventSource('/api/events', {
      factory,
      random: () => 0,
      onStateChange: s => states.push(s),
    })
    fakes[0].onerror?.({})
    expect(states).toEqual(['connecting', 'reconnecting'])
  })

  it('surfaces a constructor-time factory throw via onerror', () => {
    const seen: Event[] = []
    new ReconnectingEventSource('/api/events', {
      factory: () => {
        throw new Error('CSP block')
      },
      random: () => 0,
      onerror: (e) => seen.push(e),
    })
    expect(seen.length).toBe(1)
  })

  it('re-entrant close() from inside onerror leaves wrapper closed', () => {
    const { fakes, factory } = makeFakeFactory()
    const r = new ReconnectingEventSource('/api/events', {
      factory,
      random: () => 0,
      onerror: () => r.close(),
    })
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    expect(r.connectionState).toBe('closed')
    vi.advanceTimersByTime(60_000)
    expect(factory).toHaveBeenCalledTimes(1)
  })

  it('repeated browser errors during reconnect do not reset the timer', () => {
    const { fakes, factory } = makeFakeFactory()
    new ReconnectingEventSource('/api/events', { factory, random: () => 0 })
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(400)
    fakes[0].onerror?.({})
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(401)
    expect(factory).toHaveBeenCalledTimes(2)
  })

  it('close() prevents resurrection by post-close error events', () => {
    const { fakes, factory } = makeFakeFactory()
    const r = new ReconnectingEventSource('/api/events', { factory, random: () => 0 })
    fakes[0].onopen?.({})
    r.close()
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(60_000)
    expect(factory).toHaveBeenCalledTimes(1)
  })

  it('factory throwing during reconnect is caught and re-tried', () => {
    let throwOnce = true
    const { fakes, factory } = makeFakeFactory()
    const wrapped = vi.fn((url: string) => {
      if (throwOnce) {
        throwOnce = false
        throw new Error('CSP block')
      }
      return factory(url)
    })
    new ReconnectingEventSource('/api/events', { factory: wrapped, random: () => 0 })
    vi.advanceTimersByTime(801)
    expect(wrapped).toHaveBeenCalledTimes(2)
    expect(fakes.length).toBe(1)
  })
})
