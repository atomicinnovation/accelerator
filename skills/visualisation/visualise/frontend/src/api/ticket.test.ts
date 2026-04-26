import { describe, it, expect } from 'vitest'
import { parseTicketNumber } from './ticket'

describe('parseTicketNumber', () => {
  it('returns the integer parsed from a four-digit prefix', () => {
    expect(parseTicketNumber('meta/tickets/0001-foo.md')).toBe(1)
    expect(parseTicketNumber('meta/tickets/0029-bar-baz.md')).toBe(29)
  })

  it('returns the integer when the path has no directory component', () => {
    expect(parseTicketNumber('0042-bare.md')).toBe(42)
  })

  it('returns null when the leading segment is non-numeric', () => {
    expect(parseTicketNumber('meta/tickets/foo-bar.md')).toBeNull()
    expect(parseTicketNumber('meta/tickets/ADR-0001-foo.md')).toBeNull()
  })

  it('returns null when there is no leading digit run', () => {
    expect(parseTicketNumber('meta/tickets/-foo.md')).toBeNull()
    expect(parseTicketNumber('')).toBeNull()
  })

  it('returns null when the dash separator is missing', () => {
    expect(parseTicketNumber('meta/tickets/0001.md')).toBeNull()
  })

  it('parses ticket numbers with arbitrary digit count (no upper bound)', () => {
    expect(parseTicketNumber('meta/tickets/12345-foo.md')).toBe(12345)
  })
})
