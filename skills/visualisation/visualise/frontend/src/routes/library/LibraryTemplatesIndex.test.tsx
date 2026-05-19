import { describe, it, expect, vi } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryTemplatesIndex } from './LibraryTemplatesIndex'
import * as fetchModule from '../../api/fetch'
import type { TemplateSummary } from '../../api/types'
import { MemoryRouter } from '../../test/router-helpers'
import indexCss from './LibraryTemplatesIndex.module.css?raw'

const mockTemplates: TemplateSummary[] = [
  {
    name: 'adr',
    activeTier: 'plugin-default',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: false, active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: true  },
    ],
  },
  {
    name: 'ticket',
    activeTier: 'user-override',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: true,  active: true  },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
]

const mockWithVariety: TemplateSummary[] = [
  {
    name: 'adr',
    activeTier: 'plugin-default',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: false, active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: true  },
    ],
  },
  {
    name: 'log',
    activeTier: 'user-override',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: true,  active: true  },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
  {
    name: 'ticket',
    activeTier: 'config-override',
    tiers: [
      { source: 'config-override', path: '/x', present: true,  active: true  },
      { source: 'user-override',   path: '/y', present: true,  active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
  {
    name: 'review',
    activeTier: 'config-override',
    tiers: [
      { source: 'config-override', path: '/x', present: true,  active: true  },
      { source: 'user-override',   path: '/y', present: true,  active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
]

function Wrapper({ children }: { children: React.ReactNode }) {
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

  it('renders three tier chips per row in the fixed order default → user → config', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: 'adr' })
    for (const name of ['adr', 'log', 'ticket', 'review']) {
      const row = screen.getByRole('link', { name }).closest('li')!
      const chips = within(row).getAllByText(/^(default|user|config)$/)
      expect(chips.map((c) => c.textContent)).toEqual(['default', 'user', 'config'])
    }
  })

  it('maps tier presence/active to neutral/indigo/green Chip variants', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: 'adr' })
    const variantFor = (rowName: string, label: 'default' | 'user' | 'config') => {
      const row = screen.getByRole('link', { name: rowName }).closest('li')!
      return within(row)
        .getByText(label)
        .closest('[data-variant]')!
        .getAttribute('data-variant')
    }
    expect(variantFor('adr',    'default')).toBe('green')
    expect(variantFor('adr',    'user')).toBe('neutral')
    expect(variantFor('adr',    'config')).toBe('neutral')
    expect(variantFor('log',    'default')).toBe('indigo')
    expect(variantFor('log',    'user')).toBe('green')
    expect(variantFor('log',    'config')).toBe('neutral')
    expect(variantFor('ticket', 'default')).toBe('indigo')
    expect(variantFor('ticket', 'user')).toBe('indigo')
    expect(variantFor('ticket', 'config')).toBe('green')
    expect(variantFor('review', 'default')).toBe('indigo')
    expect(variantFor('review', 'user')).toBe('indigo')
    expect(variantFor('review', 'config')).toBe('green')
  })

  it('CSS module does not define legacy .winning or .active rules', () => {
    expect(indexCss).not.toMatch(/\.winning\b/)
    expect(indexCss).not.toMatch(/\.active\b/)
  })
})
