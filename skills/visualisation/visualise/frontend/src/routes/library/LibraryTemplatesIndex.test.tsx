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
    name: 'plan',
    activeTier: 'plugin-default',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: false, active: false },
      { source: 'plugin-default',  path: '/z', present: true,  active: true  },
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
    name: 'plan',
    activeTier: 'user-override',
    tiers: [
      { source: 'config-override', path: '/x', present: false, active: false },
      { source: 'user-override',   path: '/y', present: true,  active: true  },
      { source: 'plugin-default',  path: '/z', present: true,  active: false },
    ],
  },
  {
    name: 'research',
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
  it('renders an "Authoring templates" page title', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByRole('heading', { name: /Authoring templates/i })).toBeInTheDocument()
  })

  it('renders a clickable row for each template, with the filename in monospace', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByRole('link', { name: /adr\.md/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /plan\.md/i })).toBeInTheDocument()
  })

  it('renders a glyph svg for templates with a doc-type mapping', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    const { container } = render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: /adr\.md/i })
    // adr → decisions doc-type. Glyph emits an <svg data-doc-type="decisions">.
    expect(container.querySelector('svg[data-doc-type="decisions"]')).not.toBeNull()
    expect(container.querySelector('svg[data-doc-type="plans"]')).not.toBeNull()
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
    await screen.findByRole('link', { name: /adr\.md/i })
    for (const name of ['adr', 'plan', 'research']) {
      const row = screen.getByRole('link', { name: new RegExp(`${name}\\.md`) })
      const chips = within(row).getAllByText(/^(default|user|config)$/)
      expect(chips.map((c) => c.textContent)).toEqual(['default', 'user', 'config'])
    }
  })

  it('places "→" arrow separators between adjacent tier chips', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    const row = await screen.findByRole('link', { name: /adr\.md/i })
    // Two arrows per row (one between each pair of three chips).
    expect(within(row).getAllByText('→')).toHaveLength(2)
  })

  it('renders a chevron disclosure marker on each row', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    const row = await screen.findByRole('link', { name: /adr\.md/i })
    expect(within(row).getByText('›')).toBeInTheDocument()
  })

  it('maps tier presence/active to neutral/indigo/green Chip variants', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: /adr\.md/i })
    const variantFor = (rowName: string, label: 'default' | 'user' | 'config') => {
      const row = screen.getByRole('link', { name: new RegExp(`${rowName}\\.md`) })
      return within(row)
        .getByText(label)
        .closest('[data-variant]')!
        .getAttribute('data-variant')
    }
    expect(variantFor('adr',      'default')).toBe('green')
    expect(variantFor('adr',      'user')).toBe('neutral')
    expect(variantFor('adr',      'config')).toBe('neutral')
    expect(variantFor('plan',     'default')).toBe('indigo')
    expect(variantFor('plan',     'user')).toBe('green')
    expect(variantFor('plan',     'config')).toBe('neutral')
    expect(variantFor('research', 'default')).toBe('indigo')
    expect(variantFor('research', 'user')).toBe('indigo')
    expect(variantFor('research', 'config')).toBe('green')
  })

  it('CSS module no longer defines legacy .winning or .active rules', () => {
    expect(indexCss).not.toMatch(/\.winning\b/)
    expect(indexCss).not.toMatch(/\.active\b/)
  })
})
