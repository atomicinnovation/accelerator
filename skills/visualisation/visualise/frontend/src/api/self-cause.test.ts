import { describe, it, expect } from 'vitest'
import { createSelfCauseRegistry } from './self-cause'

describe('createSelfCauseRegistry', () => {
  it('has returns true for registered etag repeatedly (non-consuming)', () => {
    let clock = 0
    const r = createSelfCauseRegistry({ now: () => clock })
    r.register('sha256-X')
    expect(r.has('sha256-X')).toBe(true)
    expect(r.has('sha256-X')).toBe(true)
  })

  it('has returns false for unknown etag', () => {
    const r = createSelfCauseRegistry()
    expect(r.has('sha256-Y')).toBe(false)
  })

  it('has returns false for undefined', () => {
    const r = createSelfCauseRegistry()
    expect(r.has(undefined)).toBe(false)
  })

  it('expired etag is no longer present', () => {
    let clock = 0
    const r = createSelfCauseRegistry({ ttlMs: 5000, now: () => clock })
    r.register('sha256-A')
    clock = 5001
    expect(r.has('sha256-A')).toBe(false)
    // Register a second etag after expiry — the first is pruned
    r.register('sha256-B')
    expect(r.has('sha256-A')).toBe(false)
    expect(r.has('sha256-B')).toBe(true)
  })

  it('FIFO eviction drops oldest when over cap', () => {
    const r = createSelfCauseRegistry({ maxEntries: 3 })
    r.register('sha256-A')
    r.register('sha256-B')
    r.register('sha256-C')
    r.register('sha256-D')
    expect(r.has('sha256-A')).toBe(false)
    expect(r.has('sha256-B')).toBe(true)
    expect(r.has('sha256-C')).toBe(true)
    expect(r.has('sha256-D')).toBe(true)
  })

  it('reset drops all entries', () => {
    const r = createSelfCauseRegistry()
    r.register('sha256-A')
    r.register('sha256-B')
    r.reset()
    expect(r.has('sha256-A')).toBe(false)
    expect(r.has('sha256-B')).toBe(false)
  })
})
