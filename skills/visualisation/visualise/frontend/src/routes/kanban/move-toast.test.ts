import { describe, it, expect } from 'vitest'
import { moveToastFor } from './move-toast'
import { makeIndexEntry } from '../../api/test-fixtures'
import { ConflictError, FetchError } from '../../api/fetch'
import type { DropOutcome } from './resolve-drop-outcome'

const MOVE: DropOutcome = { kind: 'move', toStatus: 'in-progress' }

const entry = makeIndexEntry({
  type: 'work-items',
  relPath: 'meta/work/0086-kanban.md',
  title: 'Kanban DnD',
  frontmatter: { status: 'todo' },
})

describe('moveToastFor', () => {
  it('204 success → ok toast carrying the exact human target label, empty body', () => {
    const toast = moveToastFor(MOVE, entry, 'In progress', { ok: true })
    expect(toast).not.toBeNull()
    expect(toast!.kind).toBe('ok')
    // The human label, never the raw `in-progress` key, never `undefined`.
    expect(toast!.heading).toContain('In progress')
    expect(toast!.heading).not.toContain('in-progress')
    expect(toast!.heading).not.toContain('undefined')
    expect(toast!.message).toBe('')
  })

  it('heading names the card via the shared describeEntry helper', () => {
    const toast = moveToastFor(MOVE, entry, 'In progress', { ok: true })!
    expect(toast.heading).toBe('work item 0086: Kanban DnD moved to In progress')
  })

  it('ConflictError → error toast with the conflict copy', () => {
    const toast = moveToastFor(MOVE, entry, 'In progress', {
      ok: false,
      error: new ConflictError(412, 'PATCH …: 412', 'fresh-etag'),
    })
    expect(toast!.kind).toBe('error')
    expect(toast!.heading).toBe('Move failed')
    expect(toast!.message).toMatch(/updated by another editor/i)
  })

  it('generic FetchError → error toast whose copy differs from the conflict copy', () => {
    const conflict = moveToastFor(MOVE, entry, 'In progress', {
      ok: false,
      error: new ConflictError(412, 'x', 'etag'),
    })!
    const fetch = moveToastFor(MOVE, entry, 'In progress', {
      ok: false,
      error: new FetchError(500, 'PATCH …: 500'),
    })!
    expect(fetch.kind).toBe('error')
    expect(fetch.message).toMatch(/could not be saved/i)
    // Asserted separately so swapping the two branches fails the test.
    expect(fetch.message).not.toBe(conflict.message)
  })

  it('any non-move outcome returns null (no toast)', () => {
    expect(moveToastFor({ kind: 'no-op-same-column' }, entry, 'In progress', { ok: true })).toBeNull()
    expect(moveToastFor({ kind: 'no-op-other-rejected' }, entry, 'In progress', { ok: true })).toBeNull()
    expect(moveToastFor({ kind: 'no-op-unknown' }, entry, 'In progress', { ok: true })).toBeNull()
  })

  it('a missing source entry (deleted mid-drag) returns null even for a move', () => {
    expect(moveToastFor(MOVE, undefined, 'In progress', { ok: true })).toBeNull()
  })
})
