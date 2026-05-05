import { describe, it, expect } from 'vitest'
import { parseWorkItemId, groupWorkItemsByStatus } from './work-item'
import { makeIndexEntry } from './test-fixtures'
import { OTHER_COLUMN_KEY } from './types'

const COLS = [
  { key: 'todo', label: 'Todo' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'done', label: 'Done' },
]

describe('parseWorkItemId', () => {
  it('returns the integer parsed from a four-digit prefix', () => {
    expect(parseWorkItemId('meta/work/0001-foo.md')).toBe(1)
    expect(parseWorkItemId('meta/work/0029-bar-baz.md')).toBe(29)
  })

  it('returns the integer when the path has no directory component', () => {
    expect(parseWorkItemId('0042-bare.md')).toBe(42)
  })

  it('returns null when the leading segment is non-numeric', () => {
    expect(parseWorkItemId('meta/work/foo-bar.md')).toBeNull()
    expect(parseWorkItemId('meta/work/ADR-0001-foo.md')).toBeNull()
  })

  it('returns null when there is no leading digit run', () => {
    expect(parseWorkItemId('meta/work/-foo.md')).toBeNull()
    expect(parseWorkItemId('')).toBeNull()
  })

  it('returns null when the dash separator is missing', () => {
    expect(parseWorkItemId('meta/work/0001.md')).toBeNull()
  })

  it('parses work item ids with arbitrary digit count (no upper bound)', () => {
    expect(parseWorkItemId('meta/work/12345-foo.md')).toBe(12345)
  })
})

describe('groupWorkItemsByStatus', () => {
  it('groups by canonical status values', () => {
    const a = makeIndexEntry({ relPath: 'meta/work/0001-a.md', frontmatter: { status: 'todo' } })
    const b = makeIndexEntry({ relPath: 'meta/work/0002-b.md', frontmatter: { status: 'in-progress' } })
    const c = makeIndexEntry({ relPath: 'meta/work/0003-c.md', frontmatter: { status: 'done' } })

    const groups = groupWorkItemsByStatus([a, b, c], COLS)
    expect(groups.get('todo')).toEqual([a])
    expect(groups.get('in-progress')).toEqual([b])
    expect(groups.get('done')).toEqual([c])
    expect(groups.get(OTHER_COLUMN_KEY) ?? []).toEqual([])
  })

  it('places exotic status values in the "other" group', () => {
    const blocked = makeIndexEntry({
      relPath: 'meta/work/0001-x.md',
      frontmatter: { status: 'blocked' },
    })
    const groups = groupWorkItemsByStatus([blocked], COLS)
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([blocked])
    expect(groups.get('todo') ?? []).toEqual([])
  })

  it('places work items with missing status in "other"', () => {
    const noStatus = makeIndexEntry({ frontmatter: {} })
    const groups = groupWorkItemsByStatus([noStatus], COLS)
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([noStatus])
  })

  it('places work items with non-string status in "other"', () => {
    const numeric = makeIndexEntry({ frontmatter: { status: 42 } })
    const groups = groupWorkItemsByStatus([numeric], COLS)
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([numeric])
  })

  it('places work items with absent or malformed frontmatter in "other"', () => {
    const absent = makeIndexEntry({ frontmatterState: 'absent', frontmatter: {} })
    const malformed = makeIndexEntry({ frontmatterState: 'malformed', frontmatter: {} })
    const groups = groupWorkItemsByStatus([absent, malformed], COLS)
    expect(groups.get(OTHER_COLUMN_KEY)).toEqual([absent, malformed])
  })

  it('sorts each group by mtimeMs descending', () => {
    const old = makeIndexEntry({ relPath: 'meta/work/0001-old.md', frontmatter: { status: 'todo' }, mtimeMs: 100 })
    const mid = makeIndexEntry({ relPath: 'meta/work/0002-mid.md', frontmatter: { status: 'todo' }, mtimeMs: 200 })
    const newest = makeIndexEntry({ relPath: 'meta/work/0003-new.md', frontmatter: { status: 'todo' }, mtimeMs: 300 })
    const groups = groupWorkItemsByStatus([old, newest, mid], COLS)
    expect(groups.get('todo')).toEqual([newest, mid, old])
  })

  it('breaks mtime ties deterministically by relPath ascending', () => {
    const beta = makeIndexEntry({
      relPath: 'meta/work/0002-beta.md',
      frontmatter: { status: 'todo' }, mtimeMs: 500,
    })
    const alpha = makeIndexEntry({
      relPath: 'meta/work/0001-alpha.md',
      frontmatter: { status: 'todo' }, mtimeMs: 500,
    })
    const groups = groupWorkItemsByStatus([beta, alpha], COLS)
    expect(groups.get('todo')).toEqual([alpha, beta])
  })

  it('omits the "other" key entirely when no exotic work items exist', () => {
    const todo = makeIndexEntry({
      relPath: 'meta/work/0001-x.md',
      frontmatter: { status: 'todo' },
    })
    const groups = groupWorkItemsByStatus([todo], COLS)
    expect(groups.has(OTHER_COLUMN_KEY)).toBe(false)
  })

  it('returns empty arrays for known columns when no work items match', () => {
    const groups = groupWorkItemsByStatus([], COLS)
    expect(groups.get('todo')).toEqual([])
    expect(groups.get('in-progress')).toEqual([])
    expect(groups.get('done')).toEqual([])
    expect(groups.has(OTHER_COLUMN_KEY)).toBe(false)
  })
})
