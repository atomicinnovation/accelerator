import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import type { IndexEntry } from '../../api/types'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'

const mockEntry: IndexEntry = {
  type: 'plans', path: '/p/meta/plans/2026-01-01-foo.md',
  relPath: 'meta/plans/2026-01-01-foo.md',
  slug: 'foo', title: 'Foo Plan',
  frontmatter: { status: 'draft' }, frontmatterState: 'parsed', ticket: null,
  mtimeMs: 1_700_000_000_000, size: 100, etag: 'sha256-a',
}

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

describe('LibraryDocView', () => {
  it('renders the doc title', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    // Use content that doesn't repeat the title to avoid duplicate h1s in DOM
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body text.',
      etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo Plan')).toBeInTheDocument()
  })

  it('renders frontmatter chips', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Title', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('draft')).toBeInTheDocument()
  })

  it('renders the markdown body', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Foo Plan\nBody text here.', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Body text here.')).toBeInTheDocument()
  })

  it('shows Document not found when the slug does not match any entry', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    render(<LibraryDocView type="plans" fileSlug="nonexistent" />, { wrapper: Wrapper })
    expect(await screen.findByText(/Document not found/i)).toBeInTheDocument()
  })

  it('renders Loading while the content fetch is pending', async () => {
    // fetchDocs resolves so the entry is found, but fetchDocContent never
    // resolves — keeps the component in isLoading state indefinitely.
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    // findByText waits for RouterProvider to settle and fetchDocs to resolve
    expect(await screen.findByText(/Loading…/i)).toBeInTheDocument()
  })

  it('renders an error alert when fetchDocs rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('list-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load document list/i)
  })

  it('renders an error alert when fetchDocContent rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockRejectedValue(new Error('content-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load document content/i)
  })
})
