import { describe, it, expect } from 'vitest'
import { formatMtime } from './format'

describe('formatMtime', () => {
  const NOW = 1_700_000_000_000

  it('returns em-dash for zero or negative input', () => {
    expect(formatMtime(0,   NOW)).toBe('—')
    expect(formatMtime(-1,  NOW)).toBe('—')
  })

  it('returns "just now" for future timestamps (clock skew)', () => {
    expect(formatMtime(NOW + 5_000, NOW)).toBe('just now')
  })

  it('uses seconds, minutes, hours under a day', () => {
    expect(formatMtime(NOW - 30  * 1000, NOW)).toBe('30s ago')
    expect(formatMtime(NOW - 5   * 60_000, NOW)).toBe('5m ago')
    expect(formatMtime(NOW - 3   * 3_600_000, NOW)).toBe('3h ago')
  })

  it('uses days under a week and weeks under a month', () => {
    expect(formatMtime(NOW - 2  * 86_400_000, NOW)).toBe('2d ago')
    expect(formatMtime(NOW - 10 * 86_400_000, NOW)).toBe('1w ago')
    expect(formatMtime(NOW - 21 * 86_400_000, NOW)).toBe('3w ago')
  })

  it('falls back to a date string past 30 days', () => {
    const result = formatMtime(NOW - 60 * 86_400_000, NOW)
    expect(result).not.toMatch(/:/)
    expect(result).not.toMatch(/ago$/)
  })
})
