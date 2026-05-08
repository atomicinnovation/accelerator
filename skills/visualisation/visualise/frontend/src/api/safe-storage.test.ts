import { describe, it, expect, afterEach, vi } from 'vitest'
import { safeGetItem, safeSetItem } from './safe-storage'

afterEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
})

describe('safeGetItem / safeSetItem', () => {
  it('round-trips a value through real localStorage', () => {
    safeSetItem('test-key', 'test-value')
    expect(safeGetItem('test-key')).toBe('test-value')
  })

  it('returns null when the key is missing', () => {
    expect(safeGetItem('missing-key')).toBeNull()
  })

  it('safeGetItem returns null when getItem throws SecurityError', () => {
    const spy = vi
      .spyOn(Storage.prototype, 'getItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    expect(safeGetItem('any-key')).toBeNull()
    expect(spy).toHaveBeenCalled()
  })

  it('safeSetItem does not throw when setItem throws SecurityError', () => {
    const spy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new DOMException('private mode', 'SecurityError')
      })
    expect(() => safeSetItem('any-key', 'any-value')).not.toThrow()
    expect(spy).toHaveBeenCalled()
  })
})
