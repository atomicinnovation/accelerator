import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'
import { LibraryTypeView } from './LibraryTypeView'
import * as fetchModule from '../../api/fetch'
import type { IndexEntry } from '../../api/types'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'

const mockEntries: IndexEntry[] = [
  {
    type: 'plans', path: '/p/meta/plans/2026-01-01-foo.md',
    relPath: 'meta/plans/2026-01-01-foo.md',
    slug: 'foo', title: 'Foo Plan',
    frontmatter: { status: 'draft', date: '2026-01-01' },
    frontmatterState: 'parsed', ticket: null,
    mtimeMs: 1_700_000_000_000, size: 100, etag: 'sha256-a',
    bodyPreview: '',
  },
  {
    type: 'plans', path: '/p/meta/plans/2026-02-01-bar.md',
    relPath: 'meta/plans/2026-02-01-bar.md',
    slug: 'bar', title: 'Bar Plan',
    frontmatter: { status: 'complete', date: '2026-02-01' },
    frontmatterState: 'parsed', ticket: null,
    mtimeMs: 1_700_100_000_000, size: 200, etag: 'sha256-b',
    bodyPreview: '',
  },
]

function Wrapper({ children }: { children: React.ReactNode }) {
  // Disable retries in tests so rejected fetches surface as error state
  // immediately instead of re-firing the mock and slowing the suite.
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LibraryTypeView', () => {
  it('renders a row for each doc', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue(mockEntries)
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo Plan')).toBeInTheDocument()
    expect(screen.getByText('Bar Plan')).toBeInTheDocument()
  })

  it('clicking a column header sorts the table', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue(mockEntries)
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    await screen.findByText('Foo Plan')

    fireEvent.click(screen.getByRole('button', { name: /title/i }))
    const rows = screen.getAllByRole('row').slice(1) // skip header
    expect(rows[0]).toHaveTextContent('Bar Plan')
  })

  it('toggles sort direction on repeated clicks of the same column', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue(mockEntries)
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    await screen.findByText('Foo Plan')

    const titleButton = screen.getByRole('button', { name: /title/i })
    fireEvent.click(titleButton)  // first click: ascending
    let rows = screen.getAllByRole('row').slice(1)
    expect(rows[0]).toHaveTextContent('Bar Plan')

    fireEvent.click(titleButton)  // second click: descending
    rows = screen.getAllByRole('row').slice(1)
    expect(rows[0]).toHaveTextContent('Foo Plan')
  })

  it('shows empty-state message when no docs', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByText(/No documents found/i)).toBeInTheDocument()
  })

  it('shows loading state while fetching', async () => {
    // Never-resolving promise keeps isLoading=true for the lifetime of the test.
    // RouterProvider initialises asynchronously, so we wait for the loading
    // text to appear rather than asserting synchronously.
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByText(/Loading…/i)).toBeInTheDocument()
  })

  it('renders an error branch for an unknown doc type', async () => {
    // Bypass the DocTypeKey type with a cast — the point is to exercise
    // the runtime narrowing introduced by Finding 11.
    render(<LibraryTypeView type={'bogus' as never} />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Unknown doc type/i)
  })

  it('renders a fetch-error alert when fetchDocs rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('boom'))
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load documents/i)
  })
})
