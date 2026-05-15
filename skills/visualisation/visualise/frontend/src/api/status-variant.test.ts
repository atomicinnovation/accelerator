import { describe, expect, it } from 'vitest'
import { statusToChipVariant, isStatusKey, __SETS_FOR_TEST } from './status-variant'

describe('statusToChipVariant', () => {
  describe('green (terminal success)', () => {
    it.each([
      'done', 'complete', 'accepted', 'approved', 'implemented', 'final', 'shipped',
    ])('maps %s → green', (s) => {
      expect(statusToChipVariant(s)).toBe('green')
    })
  })

  describe('indigo (in-flight / active)', () => {
    it.each(['in-progress', 'in_progress', 'reviewed', 'ready', 'active', 'proposed', 'live'])(
      'maps %s → indigo',
      (s) => expect(statusToChipVariant(s)).toBe('indigo'),
    )
  })

  describe('amber (needs attention)', () => {
    it.each(['approve-with-changes', 'approve w/ changes', 'Approve w/ changes', 'review', 'revised'])(
      'maps %s → amber',
      (s) => expect(statusToChipVariant(s)).toBe('amber'),
    )
  })

  describe('red (blocked / terminal failure)', () => {
    it.each(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])(
      'maps %s → red',
      (s) => expect(statusToChipVariant(s)).toBe('red'),
    )
  })

  describe('neutral (default)', () => {
    it.each(['draft', 'todo', 'absent'])('maps %s → neutral', (s) => {
      expect(statusToChipVariant(s)).toBe('neutral')
    })

    it('returns neutral for unknown strings', () => {
      expect(statusToChipVariant('whatever')).toBe('neutral')
    })

    it('returns neutral for ISO date strings (fallback used by LibraryTypeView)', () => {
      expect(statusToChipVariant('2026-04-05')).toBe('neutral')
    })

    it('returns neutral for undefined / null / empty / non-string', () => {
      expect(statusToChipVariant(undefined)).toBe('neutral')
      expect(statusToChipVariant(null)).toBe('neutral')
      expect(statusToChipVariant('')).toBe('neutral')
      expect(statusToChipVariant(42)).toBe('neutral')
      expect(statusToChipVariant(true)).toBe('neutral')
      expect(statusToChipVariant(['accepted'])).toBe('neutral')
      expect(statusToChipVariant({ status: 'accepted' })).toBe('neutral')
    })
  })

  describe('case and separator insensitivity', () => {
    it('maps "Accepted" (capitalised) → green', () => {
      expect(statusToChipVariant('Accepted')).toBe('green')
    })
    it('maps "  In Progress  " → indigo', () => {
      expect(statusToChipVariant('  In Progress  ')).toBe('indigo')
    })
    it('treats hyphen, space, underscore, and slash equivalently', () => {
      expect(statusToChipVariant('in progress')).toBe('indigo')
      expect(statusToChipVariant('in_progress')).toBe('indigo')
      expect(statusToChipVariant('in-progress')).toBe('indigo')
      expect(statusToChipVariant('approve w/ changes')).toBe('amber')
    })
  })

  describe('internal invariants', () => {
    it('all Set keys are separator-free lowercase (the normalised form)', () => {
      expect(__SETS_FOR_TEST).toBeDefined()
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0)
        for (const k of s) {
          expect(k).toMatch(/^[a-z]+$/)
        }
      }
    })
  })
})

describe('isStatusKey', () => {
  it.each(['status', 'Status', 'STATUS', '  status  '])('returns true for %s', (k) => {
    expect(isStatusKey(k)).toBe(true)
  })
  it.each(['state', 'lifecycle-status', 'StatusX', ''])('returns false for %s', (k) => {
    expect(isStatusKey(k)).toBe(false)
  })
})
