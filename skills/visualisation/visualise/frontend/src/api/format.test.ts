import { describe, it, expect } from 'vitest'
import { formatMtime, formatRelative, formatChipDate } from './format'

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

describe('formatRelative', () => {
  const now = 10_000_000_000

  it('renders seconds for elapsed < 60s', () => {
    expect(formatRelative(now - 30_000, now)).toBe('30s ago')
  })

  it('renders minutes for 60s <= elapsed < 3600s', () => {
    expect(formatRelative(now - 90_000, now)).toBe('1m ago')
    expect(formatRelative(now - 59_000, now)).toBe('59s ago')
  })

  it('renders hours for 3600s <= elapsed < 86400s', () => {
    expect(formatRelative(now - 3_700_000, now)).toBe('1h ago')
  })

  it('renders days for elapsed >= 86400s', () => {
    expect(formatRelative(now - 90_000_000, now)).toBe('1d ago')
    expect(formatRelative(now - 8 * 86_400_000, now)).toBe('8d ago')
  })

  it('clamps negative elapsed to 0s ago', () => {
    expect(formatRelative(now + 1000, now)).toBe('0s ago')
  })

  it('renders boundary inputs precisely', () => {
    expect(formatRelative(now, now)).toBe('0s ago')
    expect(formatRelative(now - 60_000, now)).toBe('1m ago')
    expect(formatRelative(now - 3_600_000, now)).toBe('1h ago')
    expect(formatRelative(now - 86_400_000, now)).toBe('1d ago')
  })
})

describe('formatChipDate', () => {
  const NOW = Date.parse('2026-04-08T00:00:00Z')

  it('formats a date-only string as short-form relative time', () => {
    expect(formatChipDate('2026-04-05', NOW)).toBe('3d ago')
  })

  it('formats an ISO datetime string as short-form relative time', () => {
    expect(formatChipDate('2026-04-08T00:00:00Z', NOW)).toBe('0s ago')
    expect(formatChipDate('2026-04-07T21:00:00Z', NOW)).toBe('3h ago')
  })

  it('formats a Date object as short-form relative time', () => {
    expect(formatChipDate(new Date('2026-04-05T00:00:00Z'), NOW)).toBe('3d ago')
  })

  it('flips to a locale date string past 30 days', () => {
    const result = formatChipDate('2026-01-01', NOW)
    expect(result).not.toMatch(/ago$/)
  })

  it('returns the raw string when unparseable', () => {
    expect(formatChipDate('not a date', NOW)).toBe('not a date')
  })

  it('stringifies non-string, non-Date values', () => {
    expect(formatChipDate(2026, NOW)).toBe('2026')
  })
})
