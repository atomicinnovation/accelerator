import { describe, it, expect } from 'vitest'
import { buildKanbanAnnouncements, ticketNumberFromRelPath } from './announcements'
import type { IndexEntry } from '../../api/types'

const fooEntry = {
  type: 'tickets' as const,
  path: '/abs/meta/tickets/0001-foo.md',
  relPath: 'meta/tickets/0001-foo.md',
  slug: 'foo',
  frontmatter: {},
  frontmatterState: 'parsed' as const,
  ticket: null,
  title: 'Foo',
  mtimeMs: 0,
  size: 0,
  etag: 'sha256-x',
  bodyPreview: '',
} as unknown as IndexEntry

describe('ticketNumberFromRelPath', () => {
  it('extracts the NNNN- prefix', () => {
    expect(ticketNumberFromRelPath('meta/tickets/0001-foo.md')).toBe('0001')
    expect(ticketNumberFromRelPath('meta/tickets/0042-bar.md')).toBe('0042')
  })

  it('returns null when the prefix is missing', () => {
    expect(ticketNumberFromRelPath('meta/tickets/foo.md')).toBeNull()
  })
})

describe('buildKanbanAnnouncements', () => {
  const entriesMap = new Map([[fooEntry.relPath, fooEntry]])
  const a = buildKanbanAnnouncements({ entries: () => entriesMap })

  it('onDragStart includes the ticket number and title (colon separator)', () => {
    const msg = a.onDragStart!({ active: { id: 'meta/tickets/0001-foo.md' } } as any)
    expect(msg).toBe('Picked up ticket 0001: Foo.')
  })

  it('onDragEnd maps column id to its display label', () => {
    const msg = a.onDragEnd!({
      active: { id: 'meta/tickets/0001-foo.md' },
      over: { id: 'in-progress' },
    } as any)
    expect(msg).toBe('Moved ticket 0001: Foo to In progress.')
  })

  it('onDragOver omits announcement when there is no over target', () => {
    const msg = a.onDragOver!({
      active: { id: 'meta/tickets/0001-foo.md' },
      over: null,
    } as any)
    expect(msg).toBeUndefined()
  })

  it('onDragCancel labels the cancellation', () => {
    const msg = a.onDragCancel!({ active: { id: 'meta/tickets/0001-foo.md' } } as any)
    expect(msg).toBe('Drag of ticket 0001: Foo cancelled.')
  })
})
