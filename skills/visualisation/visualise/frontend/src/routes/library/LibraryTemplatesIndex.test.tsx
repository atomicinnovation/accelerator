import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryTemplatesIndex } from './LibraryTemplatesIndex'
import * as fetchModule from '../../api/fetch'
import type { TemplateSummary } from '../../api/types'
import { MemoryRouter } from '../../test/router-helpers'

const mockTemplates: TemplateSummary[] = [
  { name: 'adr',    activeTier: 'plugin-default', tiers: [] },
  { name: 'ticket', activeTier: 'user-override',  tiers: [] },
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

describe('LibraryTemplatesIndex', () => {
  it('renders a link for each template name', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates')
      .mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByRole('link', { name: 'adr' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'ticket' })).toBeInTheDocument()
  })

  it('renders the active tier (friendly label) beside each template name', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates')
      .mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: 'adr' })
    expect(screen.getByText('Plugin default')).toBeInTheDocument()
    expect(screen.getByText('User override')).toBeInTheDocument()
  })

  it('shows loading state while fetching', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByText(/Loading…/i)).toBeInTheDocument()
  })

  it('renders an error alert when fetchTemplates rejects', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockRejectedValue(new Error('boom'))
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load templates/i)
  })
})
