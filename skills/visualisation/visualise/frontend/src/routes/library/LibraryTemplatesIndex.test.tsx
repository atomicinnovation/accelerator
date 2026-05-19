import { describe, it, expect, vi } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryTemplatesIndex } from './LibraryTemplatesIndex'
import { glyphKeyForTemplate } from './template-tier'
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

describe('glyphKeyForTemplate', () => {
  it('maps exact template names to their doc-type glyph', () => {
    expect(glyphKeyForTemplate('adr')).toBe('decisions')
    expect(glyphKeyForTemplate('research')).toBe('research')
    expect(glyphKeyForTemplate('plan')).toBe('plans')
    expect(glyphKeyForTemplate('validation')).toBe('validations')
    expect(glyphKeyForTemplate('pr-description')).toBe('pr-descriptions')
  })

  it('falls back to a matching stem inside compound template names', () => {
    expect(glyphKeyForTemplate('codebase-research')).toBe('research')
    expect(glyphKeyForTemplate('feature-plan')).toBe('plans')
    expect(glyphKeyForTemplate('something-decision')).toBe('decisions')
  })

  it('returns null when no stem matches', () => {
    expect(glyphKeyForTemplate('totally-unknown')).toBeNull()
  })
})

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

  it('renders a framed doc-type glyph svg per row', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    const { container } = render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: /adr\.md/i })
    // Framed glyph wraps the <svg> in a <span data-doc-type="..."> with a
    // tinted background; verify both that the wrapper exists and the svg
    // is inside it.
    const frame = container.querySelector('span[data-doc-type="decisions"]')
    expect(frame).not.toBeNull()
    expect(frame?.querySelector('svg[data-doc-type="decisions"]')).not.toBeNull()
    expect(container.querySelector('span[data-doc-type="plans"]')).not.toBeNull()
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

  it('renders three tier pills per row in the fixed order default → user → config', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: /adr\.md/i })
    for (const name of ['adr', 'plan', 'research']) {
      const row = screen.getByRole('link', { name: new RegExp(`${name}\\.md`) })
      const labels = within(row).getAllByText(/^(default|user|config)$/)
      expect(labels.map((c) => c.textContent)).toEqual(['default', 'user', 'config'])
    }
  })

  it('uses right-chevron "›" separators between adjacent tier pills', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    const row = await screen.findByRole('link', { name: /adr\.md/i })
    // Three pills produce two inter-pill separators + one trailing row chevron.
    const chevrons = within(row).getAllByText('›')
    expect(chevrons.length).toBeGreaterThanOrEqual(3)
    // No "→" arrow remnants from the prior iteration.
    expect(within(row).queryAllByText('→')).toEqual([])
  })

  it('uses "•" as the leading icon on each tier pill', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    const row = await screen.findByRole('link', { name: /adr\.md/i })
    expect(within(row).getAllByText('•').length).toBe(3)
    expect(within(row).queryAllByText('+')).toEqual([])
  })

  it('maps tier (active, present, absent) state to data-state on the pill', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockWithVariety })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: /adr\.md/i })
    const stateFor = (rowName: string, label: 'default' | 'user' | 'config') => {
      const row = screen.getByRole('link', { name: new RegExp(`${rowName}\\.md`) })
      return within(row).getByText(label).closest('[data-state]')!.getAttribute('data-state')
    }
    expect(stateFor('adr',      'default')).toBe('active')
    expect(stateFor('adr',      'user')).toBe('absent')
    expect(stateFor('adr',      'config')).toBe('absent')
    expect(stateFor('plan',     'default')).toBe('present')
    expect(stateFor('plan',     'user')).toBe('active')
    expect(stateFor('plan',     'config')).toBe('absent')
    expect(stateFor('research', 'default')).toBe('present')
    expect(stateFor('research', 'user')).toBe('present')
    expect(stateFor('research', 'config')).toBe('active')
  })

  it('rows in the list share borders rather than gap-separated cards', () => {
    // The connected-table look-and-feel is driven by `border-top` on each
    // row in a shared container, not per-row `border` + `gap`. Anchor a
    // CSS regression test so a future refactor that re-introduces gap
    // styling fails loudly.
    expect(indexCss).toMatch(/\.row\s*\{[^}]*border-top:/m)
    expect(indexCss).not.toMatch(/\.list\s*\{[^}]*gap:/m)
  })

  it('CSS module no longer defines legacy .winning or .active rules', () => {
    expect(indexCss).not.toMatch(/\.winning\b/)
    expect(indexCss).not.toMatch(/\.active\b/)
  })
})
