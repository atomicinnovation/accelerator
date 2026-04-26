import { describe, it, expect } from 'vitest'
import { parseTicketNumber, groupTicketsByStatus } from './ticket'
import { makeIndexEntry } from './test-fixtures'
import { OTHER_COLUMN_KEY } from './types'

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

describe('groupTicketsByStatus', () => {
  it('groups by canonical status values', () => {
    const a = makeIndexEntry({ relPath: 'meta/tickets/0001-a.md', frontmatter: { status: 'todo' } })
    const b = makeIndexEntry({ relPath: 'meta/tickets/0002-b.md', frontmatter: { status: 'in-progress' } })
    const c = makeIndexEntry({ relPath: 'meta/tickets/0003-c.md', frontmatter: { status: 'done' } })

    const groups = groupTicketsByStatus([a, b, c])
    expect(groups.get('todo')).toEqual([a])
    expect(groups.get('in-progress')).toEqual([b])
    expect(groups.get('done')).toEqual([c])
    expect(groups.get(OTHER_COLUMN_KEY) ?? []).toEqual([])
  })

  it('places exotic status values in the "other" group', () => {
    const blocked = makeIndexEntry({
      relPath: 'meta/tickets/0001-x.md',
      frontmatter: { status: 'blocked' },
    })
    const groups = groupTicketsByStatus([blocked])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([blocked])
    expect(groups.get('todo') ?? []).toEqual([])
  })

  it('places tickets with missing status in "other"', () => {
    const noStatus = makeIndexEntry({ frontmatter: {} })
    const groups = groupTicketsByStatus([noStatus])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([noStatus])
  })

  it('places tickets with non-string status in "other"', () => {
    const numeric = makeIndexEntry({ frontmatter: { status: 42 } })
    const groups = groupTicketsByStatus([numeric])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([numeric])
  })

  it('places tickets with absent or malformed frontmatter in "other"', () => {
    const absent = makeIndexEntry({ frontmatterState: 'absent', frontmatter: {} })
    const malformed = makeIndexEntry({ frontmatterState: 'malformed', frontmatter: {} })
    const groups = groupTicketsByStatus([absent, malformed])
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([absent, malformed])
  })

  it('sorts each group by mtimeMs descending', () => {
    const old = makeIndexEntry({ relPath: 'meta/tickets/0001-old.md', frontmatter: { status: 'todo' }, mtimeMs: 100 })
    const mid = makeIndexEntry({ relPath: 'meta/tickets/0002-mid.md', frontmatter: { status: 'todo' }, mtimeMs: 200 })
    const newest = makeIndexEntry({ relPath: 'meta/tickets/0003-new.md', frontmatter: { status: 'todo' }, mtimeMs: 300 })
    const groups = groupTicketsByStatus([old, newest, mid])
    expect(groups.get('todo')).toEqual([newest, mid, old])
  })

  it('breaks mtime ties deterministically by relPath ascending', () => {
    const beta = makeIndexEntry({
      relPath: 'meta/tickets/0002-beta.md',
      frontmatter: { status: 'todo' }, mtimeMs: 500,
    })
    const alpha = makeIndexEntry({
      relPath: 'meta/tickets/0001-alpha.md',
      frontmatter: { status: 'todo' }, mtimeMs: 500,
    })
    const groups = groupTicketsByStatus([beta, alpha])
    expect(groups.get('todo')).toEqual([alpha, beta])
  })

  it('omits the "other" key entirely when no exotic tickets exist', () => {
    const todo = makeIndexEntry({
      relPath: 'meta/tickets/0001-x.md',
      frontmatter: { status: 'todo' },
    })
    const groups = groupTicketsByStatus([todo])
    expect(groups.has(OTHER_COLUMN_KEY)).toBe(false)
  })

  it('returns empty arrays for known columns when no tickets match', () => {
    const groups = groupTicketsByStatus([])
    expect(groups.get('todo')).toEqual([])
    expect(groups.get('in-progress')).toEqual([])
    expect(groups.get('done')).toEqual([])
    expect(groups.has(OTHER_COLUMN_KEY)).toBe(false)
  })
})
