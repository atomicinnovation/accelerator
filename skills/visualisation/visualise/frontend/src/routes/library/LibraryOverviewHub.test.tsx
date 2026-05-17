import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import React from 'react'
import { MemoryRouter } from '../../test/router-helpers'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { LibraryOverviewHub } from './LibraryOverviewHub'
import * as fetchModule from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { LibraryStructureResponse } from '../../api/types'

const baseStructure: LibraryStructureResponse = {
  phases: [
    {
      id: 'define',
      label: 'Define',
      docTypes: [
        {
          id: 'decisions',
          label: 'Decisions',
          count: 3,
          filteredCount: 3,
          latest: { title: 'Foo ADR', slug: 'foo', modifiedAt: 1_700_000_000_000 },
          filterFacets: [],
        },
      ],
    },
    {
      id: 'discover',
      label: 'Discover',
      docTypes: [
        {
          id: 'research',
          label: 'Research',
          count: 0,
          filteredCount: 0,
          latest: null,
          filterFacets: [],
        },
      ],
    },
  ],
  templates: {
    id: 'templates',
    label: 'Templates',
    count: 4,
    filteredCount: 4,
    latest: { title: 'adr', slug: null, modifiedAt: 1_700_100_000_000 },
    filterFacets: [],
  },
}

function makeWrapper(qc?: QueryClient) {
  const client =
    qc ??
    new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={client}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    )
  }
}

describe('LibraryOverviewHub', () => {
  it('renders the eyebrow, title and subtitle', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockResolvedValue(baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    expect(await screen.findByText('LIBRARY')).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: 'All artifacts in meta/' }),
    ).toBeInTheDocument()
  })

  it('renders phase groupings driven by the server response', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockResolvedValue(baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    expect(await screen.findByText('DEFINE')).toBeInTheDocument()
    expect(screen.getByText('DISCOVER')).toBeInTheDocument()
  })

  it('does NOT render a META/Templates section on the hub', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockResolvedValue(baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    await screen.findByText('DEFINE')
    expect(screen.queryByText('META')).toBeNull()
    expect(screen.queryByText('Templates')).toBeNull()
  })

  it('renders a non-zero card as a link', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockResolvedValue(baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    await screen.findByText('Decisions')
    const link = screen.getByRole('link', { name: /decisions/i })
    expect(link).toHaveAttribute('href', '/library/decisions')
  })

  it('renders a zero-count card as a link with "no docs yet" subtitle', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockResolvedValue(baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    await screen.findByText('Research')
    // Empty cards remain clickable (navigating to the list-view empty state)
    // but get a pinstripe pattern + altered hover via the `cardEmpty` class.
    const link = screen.getByRole('link', { name: /research \(no documents yet\)/i })
    expect(link).toHaveAttribute('href', '/library/research')
    expect(screen.getByText('no docs yet')).toBeInTheDocument()
  })

  it('renders the latest preview line for non-empty cards', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockResolvedValue(baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    expect(await screen.findByText(/Foo ADR/)).toBeInTheDocument()
  })

  it('renders a loading placeholder while fetching', async () => {
    vi.spyOn(fetchModule, 'fetchLibraryStructure').mockReturnValue(
      new Promise(() => {}),
    )
    render(<LibraryOverviewHub />, { wrapper: makeWrapper() })
    expect(await screen.findByText(/loading/i)).toBeInTheDocument()
  })

  it('reads from the canonical libraryStructure cache key (no selection arg)', async () => {
    // Pre-warm cache with the exact key the hub should subscribe to. The hub
    // must NOT introduce a selection-scoped key here — that would break the
    // dedup with RootLayout. We assert the rendered output matches the cache
    // entry, which only works if the hub subscribed to the same key.
    const fetchSpy = vi
      .spyOn(fetchModule, 'fetchLibraryStructure')
      .mockImplementation(() => new Promise(() => {}))
    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false, staleTime: Infinity } },
    })
    qc.setQueryData(queryKeys.libraryStructure(), baseStructure)
    render(<LibraryOverviewHub />, { wrapper: makeWrapper(qc) })
    await waitFor(() => screen.getByText('DEFINE'))
    expect(fetchSpy).not.toHaveBeenCalled()
  })
})
