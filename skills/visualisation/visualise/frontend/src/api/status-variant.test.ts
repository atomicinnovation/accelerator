import { describe, expect, it } from 'vitest'
import { statusToVariant, isStatusKey, __SETS_FOR_TEST } from './status-variant'
import { normaliseValue } from './normalise-value'

describe('statusToVariant', () => {
  describe('green (terminal success)', () => {
    it.each([
      'done', 'complete', 'accepted', 'approved', 'implemented', 'final', 'shipped',
    ])('maps %s → green', (s) => {
      expect(statusToVariant(s)).toBe('green')
    })
  })

  describe('indigo (in-flight / active)', () => {
    it.each(['in-progress', 'in_progress', 'reviewed', 'ready', 'active', 'proposed', 'live'])(
      'maps %s → indigo',
      (s) => expect(statusToVariant(s)).toBe('indigo'),
    )
  })

  describe('amber (needs attention)', () => {
    it.each(['approve-with-changes', 'approve w/ changes', 'Approve w/ changes', 'review', 'revised'])(
      'maps %s → amber',
      (s) => expect(statusToVariant(s)).toBe('amber'),
    )
  })

  describe('red (blocked / terminal failure)', () => {
    it.each(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])(
      'maps %s → red',
      (s) => expect(statusToVariant(s)).toBe('red'),
    )
  })

  describe('neutral (default)', () => {
    it.each(['draft', 'todo', 'absent'])('maps %s → neutral', (s) => {
      expect(statusToVariant(s)).toBe('neutral')
    })

    it('returns neutral for unknown strings', () => {
      expect(statusToVariant('whatever')).toBe('neutral')
    })

    it('returns neutral for ISO date strings (fallback used by LibraryTypeView)', () => {
      expect(statusToVariant('2026-04-05')).toBe('neutral')
    })

    it('returns neutral for undefined / null / empty / non-string', () => {
      expect(statusToVariant(undefined)).toBe('neutral')
      expect(statusToVariant(null)).toBe('neutral')
      expect(statusToVariant('')).toBe('neutral')
      expect(statusToVariant(42)).toBe('neutral')
      expect(statusToVariant(true)).toBe('neutral')
      expect(statusToVariant(['accepted'])).toBe('neutral')
      expect(statusToVariant({ status: 'accepted' })).toBe('neutral')
    })
  })

  describe('case and separator insensitivity', () => {
    it('maps "Accepted" (capitalised) → green', () => {
      expect(statusToVariant('Accepted')).toBe('green')
    })
    it('maps "  In Progress  " → indigo', () => {
      expect(statusToVariant('  In Progress  ')).toBe('indigo')
    })
    it('treats hyphen, space, underscore, and slash equivalently', () => {
      expect(statusToVariant('in progress')).toBe('indigo')
      expect(statusToVariant('in_progress')).toBe('indigo')
      expect(statusToVariant('in-progress')).toBe('indigo')
      expect(statusToVariant('approve w/ changes')).toBe('amber')
    })
  })

  describe('internal invariants', () => {
    it('all Set keys are in normalised form', () => {
      expect(__SETS_FOR_TEST).toBeDefined()
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0)
        for (const k of s) {
          expect(normaliseValue(k)).toBe(k)
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
