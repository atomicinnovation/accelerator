import { describe, expect, it } from 'vitest'
import { normaliseValue } from './normalise-value'

describe('normaliseValue', () => {
  it('lowercases', () => {
    expect(normaliseValue('Accepted')).toBe('accepted')
  })

  it('strips leading/trailing whitespace', () => {
    expect(normaliseValue('  Accepted  ')).toBe('accepted')
  })

  it.each([
    ['in progress', 'inprogress'],
    ['in_progress', 'inprogress'],
    ['in-progress', 'inprogress'],
    ['approve w/ changes', 'approvewchanges'],
    ['REQUEST_CHANGES', 'requestchanges'],
  ])('treats whitespace, underscore, hyphen, slash equivalently (%s)', (input, expected) => {
    expect(normaliseValue(input)).toBe(expected)
  })

  it.each([null, undefined, 42, true, ['a', 'b'], { x: 1 }])(
    'returns empty string for non-string inputs (%s)', (input) => {
      expect(normaliseValue(input as unknown)).toBe('')
    },
  )

  describe('unicode scope (documented limitation)', () => {
    it.each([
      ['en–dash', 'en–dash'],
      ['em—dash', 'em—dash'],
      ['unicode‐hyphen', 'unicode‐hyphen'],
    ])('does not collapse typographic separator in %s', (input, expected) => {
      expect(normaliseValue(input)).toBe(expected)
    })
  })
})
