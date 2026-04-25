import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryTemplatesView } from './LibraryTemplatesView'
import * as fetchModule from '../../api/fetch'
import type { TemplateDetail } from '../../api/types'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'

const mockDetail: TemplateDetail = {
  name: 'adr',
  activeTier: 'plugin-default',
  tiers: [
    { source: 'config-override', path: '/no-config', present: false, active: false },
    { source: 'user-override',   path: '/meta/templates/adr.md', present: false, active: false },
    { source: 'plugin-default',  path: '/plugin/templates/adr.md', present: true, active: true,
      content: '# ADR\nBody.', etag: 'sha256-x' },
  ],
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

describe('LibraryTemplatesView', () => {
  it('renders a panel for each tier', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText(/plugin.default/i)).toBeInTheDocument()
    expect(screen.getByText(/config.override/i)).toBeInTheDocument()
    expect(screen.getByText(/user.override/i)).toBeInTheDocument()
  })

  it('marks the active tier', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText('active')).toBeInTheDocument()
  })

  it('renders absent tiers as greyed-out cards with a note', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText(/plugin.default/i)
    expect(screen.getAllByText(/not currently configured/i).length).toBeGreaterThanOrEqual(1)
  })

  it('renders the markdown content of the active tier', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText('Body.')).toBeInTheDocument()
  })

  it('renders an error alert when fetchTemplateDetail rejects', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockRejectedValue(new Error('boom'))
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load template/i)
  })
})
