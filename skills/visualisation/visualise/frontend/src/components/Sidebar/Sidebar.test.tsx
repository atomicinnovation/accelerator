import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, renderWithRouterAt } from '../../test/router-helpers'
import { Sidebar } from './Sidebar'
import type { DocType, DocTypeKey } from '../../api/types'
import {
  UnseenDocTypesContext,
  type UnseenDocTypesHandle,
} from '../../api/use-unseen-doc-types'
import * as fetchModule from '../../api/fetch'

vi.mock('../../api/fetch', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api/fetch')>()
  return {
    ...actual,
    fetchActivity: vi.fn(),
  }
})

beforeEach(() => {
  vi.mocked(fetchModule.fetchActivity).mockResolvedValue([])
})

function makeUnseenHandle(unseenSet: Set<DocTypeKey> = new Set()): UnseenDocTypesHandle {
  return {
    unseenSet,
    markSeen: vi.fn(),
    onEvent: vi.fn(),
    onReconnect: vi.fn(),
  }
}

const allDocTypes: DocType[] = [
  { key: 'decisions',          label: 'Decisions',          dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'work-items',         label: 'Work items',         dirPath: '/p', inLifecycle: true,  inKanban: true,  virtual: false },
  { key: 'plans',              label: 'Plans',              dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'research',           label: 'Research',           dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'plan-reviews',       label: 'Plan reviews',       dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'pr-reviews',         label: 'PR reviews',         dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'work-item-reviews',  label: 'Work item reviews',  dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'validations',        label: 'Validations',        dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'notes',              label: 'Notes',              dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'prs',                label: 'PRs',                dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'design-gaps',        label: 'Design gaps',        dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'design-inventories', label: 'Design inventories', dirPath: '/p', inLifecycle: true,  inKanban: false, virtual: false },
  { key: 'templates',          label: 'Templates',          dirPath: null, inLifecycle: false, inKanban: false, virtual: true  },
]

function renderSidebar(
  docTypes: DocType[] = allDocTypes,
  unseen: UnseenDocTypesHandle = makeUnseenHandle(),
) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={qc}>
      <UnseenDocTypesContext.Provider value={unseen}>
        <MemoryRouter><Sidebar docTypes={docTypes} /></MemoryRouter>
      </UnseenDocTypesContext.Provider>
    </QueryClientProvider>,
  )
}

describe('Sidebar', () => {
  it('renders the LIBRARY heading', async () => {
    renderSidebar()
    expect(await screen.findByText('LIBRARY')).toBeInTheDocument()
  })

  it('renders five phase subheadings in canonical order', async () => {
    renderSidebar()
    await screen.findByText('LIBRARY')
    const headings = screen.getAllByRole('heading', { level: 3 })
    expect(headings.map(h => h.textContent)).toEqual([
      'DEFINE', 'DISCOVER', 'BUILD', 'SHIP', 'REMEMBER',
    ])
  })

  it('renders each phase\'s doc types in canonical display order', async () => {
    renderSidebar()
    await screen.findByText('LIBRARY')
    const links = screen.getAllByRole('link').map(a => a.textContent ?? '')
    // Define: work-items, work-item-reviews
    expect(links.findIndex(t => t.includes('Work items'))).toBeLessThan(
      links.findIndex(t => t.includes('Work item reviews')),
    )
    // Discover: design-inventories, design-gaps, research
    expect(links.findIndex(t => t.includes('Design inventories'))).toBeLessThan(
      links.findIndex(t => t.includes('Design gaps')),
    )
    expect(links.findIndex(t => t.includes('Design gaps'))).toBeLessThan(
      links.findIndex(t => t.includes('Research')),
    )
    // Remember: decisions, notes
    expect(links.findIndex(t => t.includes('Decisions'))).toBeLessThan(
      links.findIndex(t => t.includes('Notes')),
    )
  })

  it('Templates appears under META section, not in LIBRARY', async () => {
    renderSidebar()
    await screen.findByText('LIBRARY')
    expect(screen.getByText('META')).toBeInTheDocument()
    expect(screen.getByText('Templates')).toBeInTheDocument()
    // Templates should be linked from /library/templates
    const tpl = screen.getByRole('link', { name: /Templates/i })
    expect(tpl.getAttribute('href')).toBe('/library/templates')
  })

  it('does not render glyphs in LIBRARY nav items', async () => {
    const { container } = renderSidebar()
    await screen.findByText('LIBRARY')
    // Only the VIEWS section carries icon SVGs (Kanban + Lifecycle = 2).
    const svgs = container.querySelectorAll('section[aria-labelledby="library-heading"] svg')
    expect(svgs.length).toBe(0)
  })

  it('VIEWS section renders Kanban and Lifecycle with icons', async () => {
    const { container } = renderSidebar()
    await screen.findByText('VIEWS')
    expect(screen.getByRole('link', { name: /Kanban/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Lifecycle/i })).toBeInTheDocument()
    const svgs = container.querySelectorAll('section[aria-labelledby="views-heading"] svg')
    expect(svgs.length).toBe(2)
  })

  it('count badge present when count > 0', async () => {
    const withCount = allDocTypes.map(t =>
      t.key === 'decisions' ? { ...t, count: 12 } : t,
    )
    renderSidebar(withCount)
    await screen.findByText('LIBRARY')
    expect(screen.getByText('12')).toBeInTheDocument()
  })

  it('count badge absent when count === 0', async () => {
    const withZero = allDocTypes.map(t =>
      t.key === 'decisions' ? { ...t, count: 0 } : t,
    )
    renderSidebar(withZero)
    await screen.findByText('LIBRARY')
    expect(screen.queryByText('0')).toBeNull()
  })

  it('count badge absent when count missing (undefined)', async () => {
    renderSidebar() // no counts at all
    await screen.findByText('LIBRARY')
    // No badge for any item
    expect(screen.queryByText(/^\d+$/)).toBeNull()
  })

  it('unseen dot present when context flags the type', async () => {
    const handle = makeUnseenHandle(new Set(['research']))
    renderSidebar(allDocTypes, handle)
    await screen.findByText('LIBRARY')
    const researchLink = screen.getByLabelText('Research (unseen changes)')
    // The dot is a <span aria-hidden="true">. Glyph renders an <svg>, so
    // a <span aria-hidden> uniquely identifies the dot inside this link.
    expect(researchLink.querySelectorAll('span[aria-hidden="true"]').length).toBe(1)
    expect(researchLink.getAttribute('title')).toBe('Unseen changes since your last visit')
    const decisionsLink = screen.getByLabelText('Decisions')
    expect(decisionsLink.querySelectorAll('span[aria-hidden="true"]').length).toBe(0)
  })

  it('link aria-label reflects unseen state', async () => {
    const handle = makeUnseenHandle(new Set(['research']))
    renderSidebar(allDocTypes, handle)
    await screen.findByText('LIBRARY')
    expect(screen.getByLabelText('Research (unseen changes)')).toBeInTheDocument()
    // Decisions: bare label, no title
    const decisionsLink = screen.getByLabelText('Decisions')
    expect(decisionsLink.getAttribute('title')).toBeNull()
  })

  it('dot and count co-exist; dot precedes count in DOM order', async () => {
    const handle = makeUnseenHandle(new Set(['decisions']))
    const docs = allDocTypes.map(t =>
      t.key === 'decisions' ? { ...t, count: 12 } : t,
    )
    renderSidebar(docs, handle)
    await screen.findByText('LIBRARY')
    const decisionsLink = screen.getByLabelText('Decisions (unseen changes)')
    const count = decisionsLink.querySelector('span:last-child')
    const dot = decisionsLink.querySelector('[aria-hidden="true"]')
    expect(count?.textContent).toBe('12')
    expect(dot).not.toBeNull()
    if (dot && count) {
      expect(
        dot.compareDocumentPosition(count) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy()
    }
  })

  it('search row is rendered (temporary; 0054 will wire behaviour)', async () => {
    const { container } = renderSidebar()
    await screen.findByText('LIBRARY')
    expect(container.querySelector('input[type="search"]')).not.toBeNull()
    expect(container.querySelector('kbd')?.textContent).toBe('/')
  })

  it('active state for /library/<type>', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    renderWithRouterAt(
      <QueryClientProvider client={qc}>
        <UnseenDocTypesContext.Provider value={makeUnseenHandle()}>
          <Sidebar docTypes={allDocTypes} />
        </UnseenDocTypesContext.Provider>
      </QueryClientProvider>,
      '/library/work-items',
    )
    await waitFor(() => {
      const link = screen.getByRole('link', { name: /Work items/i })
      expect(link.className).toMatch(/active/)
    })
  })

  it('active state for child-doc URL /library/<type>/<slug>', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    renderWithRouterAt(
      <QueryClientProvider client={qc}>
        <UnseenDocTypesContext.Provider value={makeUnseenHandle()}>
          <Sidebar docTypes={allDocTypes} />
        </UnseenDocTypesContext.Provider>
      </QueryClientProvider>,
      '/library/work-items/0099',
    )
    await waitFor(() => {
      const link = screen.getByRole('link', { name: /Work items/i })
      expect(link.className).toMatch(/active/)
    })
  })

  it('active state does NOT collide across prefix-sharing keys', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    renderWithRouterAt(
      <QueryClientProvider client={qc}>
        <UnseenDocTypesContext.Provider value={makeUnseenHandle()}>
          <Sidebar docTypes={allDocTypes} />
        </UnseenDocTypesContext.Provider>
      </QueryClientProvider>,
      '/library/plan-reviews',
    )
    await waitFor(() => {
      const planReviews = screen.getByRole('link', { name: /Plan reviews/i })
      expect(planReviews.className).toMatch(/active/)
    })
    const plansLink = screen.getByRole('link', { name: /^Plans$/i })
    expect(plansLink.className).not.toMatch(/active/)
  })

  it('empty docTypes still renders headings and empty LIBRARY lists', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    renderSidebar([])
    await screen.findByText('LIBRARY')
    expect(screen.getAllByRole('heading', { level: 3 })).toHaveLength(5)
    // Library section has no doc-type links when docTypes is empty.
    // VIEWS section still renders Kanban + Lifecycle (those don't depend
    // on docTypes); META is hidden when templates is missing.
    const libraryLinks = document.querySelectorAll(
      'section[aria-labelledby="library-heading"] a',
    )
    expect(libraryLinks.length).toBe(0)
    expect(screen.queryByText('META')).toBeNull()
    warn.mockRestore()
  })

  it('warns and skips when a PHASE_DOC_TYPES key is missing from payload', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const withoutPlans = allDocTypes.filter(t => t.key !== 'plans')
    renderSidebar(withoutPlans)
    await screen.findByText('LIBRARY')
    expect(screen.queryByRole('link', { name: /^Plans$/i })).toBeNull()
    expect(screen.getByText('BUILD')).toBeInTheDocument()
    expect(warn).toHaveBeenCalled()
    expect(warn.mock.calls.some(call => String(call[0]).includes('plans'))).toBe(true)
    warn.mockRestore()
  })

  it('orphan DocTypeKey not in PHASE_DOC_TYPES does not appear', async () => {
    const docs: DocType[] = [
      ...allDocTypes,
      { key: 'orphan-type' as DocTypeKey, label: 'Orphan', dirPath: '/p', inLifecycle: false, inKanban: false, virtual: false },
    ]
    renderSidebar(docs)
    await screen.findByText('LIBRARY')
    expect(screen.queryByText('Orphan')).toBeNull()
  })

  describe('Activity slot', () => {
    it('renders the ACTIVITY heading inside the Sidebar', async () => {
      renderSidebar()
      await screen.findByText('LIBRARY')
      expect(screen.getByText('ACTIVITY')).toBeInTheDocument()
    })

    it('renders the ACTIVITY section between VIEWS and META in DOM order', async () => {
      renderSidebar()
      await screen.findByText('LIBRARY')
      const views = screen.getByText('VIEWS')
      const activity = screen.getByText('ACTIVITY')
      const meta = screen.getByText('META')
      expect(
        views.compareDocumentPosition(activity) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy()
      expect(
        activity.compareDocumentPosition(meta) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy()
    })

    it('existing LIBRARY / VIEWS / META headings still render unchanged', async () => {
      renderSidebar()
      await screen.findByText('LIBRARY')
      expect(screen.getByText('LIBRARY')).toBeInTheDocument()
      expect(screen.getByText('VIEWS')).toBeInTheDocument()
      expect(screen.getByText('META')).toBeInTheDocument()
    })
  })
})
