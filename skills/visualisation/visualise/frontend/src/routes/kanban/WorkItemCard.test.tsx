import { describe, it, expect } from 'vitest'
import { screen } from '@testing-library/react'
import { DndContext } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { WorkItemCard } from './WorkItemCard'
import { makeCompleteness, makeIndexEntry } from '../../api/test-fixtures'
import { renderWithRouterAt } from '../../test/router-helpers'

const FROZEN_NOW = 1_700_000_000_000

function renderCard(entry: ReturnType<typeof makeIndexEntry>, now = FROZEN_NOW) {
  return renderWithRouterAt(
    <DndContext>
      <SortableContext items={[entry.relPath]} strategy={verticalListSortingStrategy}>
        <WorkItemCard entry={entry} now={now} />
      </SortableContext>
    </DndContext>,
  )
}

describe('WorkItemCard', () => {
  it('renders the work-item ID verbatim from entry.workItemId (prefixed form)', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: 'ENG-0042',
      title: 'Foo',
      frontmatter: { kind: 'adr-creation-task' },
    })
    const { container } = renderCard(entry)
    const id = await screen.findByText('ENG-0042')
    expect(id).toBeInTheDocument()
    expect(container.querySelector('.ac-kcard__id')).toBeInTheDocument()
    expect(container.querySelector('.ac-kcard__id')!.textContent).toBe('ENG-0042')
  })

  it('renders bare-digit workItemId verbatim (formatDocId passthrough)', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0042-foo.md',
      workItemId: '0042',
      title: 'Foo',
    })
    renderCard(entry)
    expect(await screen.findByText('0042')).toBeInTheDocument()
  })

  it('omits the .ac-kcard__id slot when workItemId is null', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/foo-without-number.md',
      workItemId: null,
      title: 'No id',
    })
    const { container } = renderCard(entry)
    await screen.findByText('No id')
    expect(container.querySelector('.ac-kcard__id')).toBeNull()
  })

  it('renders PipelineMini as first child of .ac-kcard__top when completeness is present', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
      title: 'Foo',
      completeness: makeCompleteness({
        hasWorkItem: true,
        hasPlan: true,
        present: ['work-items', 'plans'],
      }),
    })
    const { container } = renderCard(entry)
    await screen.findByText('Foo')
    const top = container.querySelector('.ac-kcard__top')!
    const firstChild = top.firstElementChild!
    expect(firstChild.classList.contains('ac-stagedots')).toBe(true)
  })

  it('passes the entry.completeness to PipelineMini (active stages reflect present)', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
      title: 'Foo',
      completeness: makeCompleteness({
        hasWorkItem: true,
        hasPlan: true,
        present: ['work-items', 'plans'],
      }),
    })
    const { container } = renderCard(entry)
    await screen.findByText('Foo')
    const top = container.querySelector('.ac-kcard__top')!
    expect(
      top.querySelector('[data-stage="work-items"]')!.getAttribute('data-active'),
    ).toBe('true')
    expect(
      top.querySelector('[data-stage="plans"]')!.getAttribute('data-active'),
    ).toBe('true')
    expect(
      top.querySelector('[data-stage="research"]')!.getAttribute('data-active'),
    ).toBe('false')
  })

  it('omits PipelineMini when completeness is null (orphan)', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
      title: 'Foo',
      completeness: null,
    })
    const { container } = renderCard(entry)
    await screen.findByText('Foo')
    expect(container.querySelector('.ac-stagedots')).toBeNull()
  })

  it('renders "{N} linked" inside .ac-kcard__foot when linkedCount > 0', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
      title: 'Foo',
      linkedCount: 3,
    })
    const { container } = renderCard(entry)
    await screen.findByText('Foo')
    const foot = container.querySelector('.ac-kcard__foot')!
    expect(foot.textContent).toContain('3 linked')
    expect(container.querySelector('.ac-kcard__links')!.textContent).toBe('3 linked')
  })

  it('omits the linked label when linkedCount is 0', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
      title: 'Foo',
      linkedCount: 0,
    })
    const { container } = renderCard(entry)
    await screen.findByText('Foo')
    expect(container.querySelector('.ac-kcard__links')).toBeNull()
  })

  it('renders entry.frontmatter.kind inside .ac-kcard__kind', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-foo.md',
      workItemId: '0001',
      title: 'Foo',
      frontmatter: { kind: 'adr-creation-task' },
    })
    const { container } = renderCard(entry)
    await screen.findByText('Foo')
    expect(container.querySelector('.ac-kcard__kind')!.textContent).toBe(
      'adr-creation-task',
    )
  })

  it('links to the library detail page using the canonical typed-route form', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-three-layer-review-system-architecture.md',
      workItemId: '0001',
      title: 'Three-layer review system architecture',
    })
    renderCard(entry)
    const link = await screen.findByRole('link', {
      name: /three-layer review system architecture/i,
    })
    expect(link.getAttribute('href')).toBe(
      '/library/work-items/0001-three-layer-review-system-architecture',
    )
  })

  it('preserves dnd-kit attributes: data-relpath and aria-roledescription', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-a.md',
      workItemId: '0001',
      title: 'Some work item',
    })
    renderCard(entry)
    const link = await screen.findByRole('link', { name: /some work item/i })
    expect(link.getAttribute('aria-roledescription')).toBe('sortable')
    const li = document.querySelector('[data-relpath="meta/work/0001-a.md"]')
    expect(li).not.toBeNull()
  })
})
