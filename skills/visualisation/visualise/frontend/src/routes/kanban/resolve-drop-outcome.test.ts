import { describe, it, expect } from 'vitest'
import type { Active, Over } from '@dnd-kit/core'
import { resolveDropOutcome } from './resolve-drop-outcome'
import type { IndexEntry } from '../../api/types'

function makeEntry(relPath: string, status: string): IndexEntry {
  return {
    type: 'tickets',
    path: `/tmp/${relPath}`,
    relPath,
    slug: relPath.split('/').pop()!.replace('.md', ''),
    title: 'Test',
    frontmatter: { status },
    frontmatterState: 'parsed',
    ticket: null,
    mtimeMs: 0,
    size: 100,
    etag: 'sha256-abc',
    bodyPreview: '',
  }
}

function makeMap(...entries: IndexEntry[]): Map<string, IndexEntry> {
  return new Map(entries.map(e => [e.relPath, e]))
}

function active(id: string): Active { return { id } as unknown as Active }
function over(id: string): Over { return { id } as unknown as Over }

describe('resolveDropOutcome', () => {
  it('column_id_returns_move_outcome', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), over('column:done'), makeMap(src)))
      .toEqual({ kind: 'move', toStatus: 'done' })
  })

  it('card_id_returns_target_cards_column_as_move', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    const tgt = makeEntry('meta/tickets/0002-bar.md', 'done')
    expect(resolveDropOutcome(active(src.relPath), over(tgt.relPath), makeMap(src, tgt)))
      .toEqual({ kind: 'move', toStatus: 'done' })
  })

  it('other_column_id_returns_no_op_other_rejected', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), over('column:other'), makeMap(src)))
      .toEqual({ kind: 'no-op-other-rejected' })
  })

  it('card_id_in_other_returns_no_op_other_rejected', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    const tgt = makeEntry('meta/tickets/0002-bar.md', 'blocked')
    expect(resolveDropOutcome(active(src.relPath), over(tgt.relPath), makeMap(src, tgt)))
      .toEqual({ kind: 'no-op-other-rejected' })
  })

  it('same_column_returns_no_op_same_column', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), over('column:todo'), makeMap(src)))
      .toEqual({ kind: 'no-op-same-column' })
  })

  it('card_on_card_in_same_column_returns_no_op_same_column', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    const tgt = makeEntry('meta/tickets/0002-bar.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), over(tgt.relPath), makeMap(src, tgt)))
      .toEqual({ kind: 'no-op-same-column' })
  })

  it('unknown_column_id_returns_no_op_unknown', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), over('column:unknown'), makeMap(src)))
      .toEqual({ kind: 'no-op-unknown' })
  })

  it('unknown_card_id_returns_no_op_unknown', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), over('meta/tickets/ghost.md'), makeMap(src)))
      .toEqual({ kind: 'no-op-unknown' })
  })

  it('null_over_returns_no_op_unknown', () => {
    const src = makeEntry('meta/tickets/0001-foo.md', 'todo')
    expect(resolveDropOutcome(active(src.relPath), null, makeMap(src)))
      .toEqual({ kind: 'no-op-unknown' })
  })

  it('unknown_active_returns_no_op_unknown', () => {
    expect(resolveDropOutcome(active('meta/tickets/ghost.md'), over('column:done'), new Map()))
      .toEqual({ kind: 'no-op-unknown' })
  })
})
