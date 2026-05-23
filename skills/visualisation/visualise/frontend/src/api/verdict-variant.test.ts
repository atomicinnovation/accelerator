import { describe, expect, it } from 'vitest'
import { verdictToVariant, __SETS_FOR_TEST } from './verdict-variant'
import { normaliseValue } from './normalise-value'

describe('verdictToVariant', () => {
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


  describe('plan-review vocabulary', () => {
    it.each([
      ['APPROVE', 'green'],
      ['REVISE', 'amber'],
      ['REQUEST_CHANGES', 'red'],
      ['COMMENT', 'neutral'],
    ])('maps %s → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })

  describe('case insensitivity', () => {
    it.each([
      ['approve', 'green'], ['Approve', 'green'], ['APPROVE', 'green'],
      ['revise', 'amber'], ['Revise', 'amber'],
      ['request_changes', 'red'], ['Request_Changes', 'red'],
    ])('maps %s → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })

  describe('normalisation reach', () => {
    it.each([
      ['REQUEST_CHANGES', 'red'],
      ['request-changes', 'red'],
      ['request changes', 'red'],
      ['request/changes', 'red'],
    ])('maps %s → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })

  describe('neutral fallback', () => {
    it.each(['xyz', '', 'undecided', 'maybe'])(
      'unmapped %s → neutral', (v) => expect(verdictToVariant(v)).toBe('neutral'),
    )

    it.each([null, undefined, 42, true, ['a'], { x: 1 }] as const)(
      'non-string %s → neutral', (v) => expect(verdictToVariant(v as unknown)).toBe('neutral'),
    )
  })

  describe('vocabulary isolation', () => {
    it.each([
      ['done', 'neutral'], ['accepted', 'neutral'], ['blocked', 'neutral'],
      ['in progress', 'neutral'], ['rejected', 'neutral'],
    ])('status-shaped %s under verdict → %s', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })

    it.each([
      ['pass', 'neutral'], ['fail', 'neutral'], ['partial', 'neutral'],
    ])('result-shaped %s under verdict → %s (handled by resultToVariant only)', (v, expected) => {
      expect(verdictToVariant(v)).toBe(expected)
    })
  })
})
