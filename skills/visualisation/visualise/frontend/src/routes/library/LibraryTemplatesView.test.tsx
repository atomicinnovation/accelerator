import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createHash } from 'node:crypto'
import React from 'react'
import { LibraryTemplatesView } from './LibraryTemplatesView'
import * as fetchModule from '../../api/fetch'
import type { TemplateDetail, TemplateSummary } from '../../api/types'
import { dispatchSseEvent } from '../../api/use-doc-events'
import { MemoryRouter } from '../../test/router-helpers'
import templatesCss from './LibraryTemplatesView.module.css?raw'

function digestForContent(content: string): string {
  return `sha256-${createHash('sha256').update(content).digest('hex')}`
}

const mockTemplates: TemplateSummary[] = [
  {
    name: 'adr',
    activeTier: 'plugin-default',
    tiers: [
      { source: 'config-override', path: '/no-config', present: false, active: false },
      { source: 'user-override',   path: '/meta/templates/adr.md', present: false, active: false },
      { source: 'plugin-default',  path: '/plugin/templates/adr.md', present: true, active: true },
    ],
  },
]

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
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

function mockListAndDetail(detail: TemplateDetail = mockDetail) {
  vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
  return vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(detail)
}

describe('LibraryTemplatesView', () => {
  it('renders the index list above the detail section', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    // Index row is rendered alongside the detail.
    expect(await screen.findByRole('link', { name: /adr\.md/i })).toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: /THREE TIERS · ADR\.MD/i })).toBeInTheDocument()
  })

  it('marks the selected row with aria-current="page"', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    const row = await screen.findByRole('link', { name: /adr\.md/i })
    expect(row.getAttribute('aria-current')).toBe('page')
  })

  it('renders a TIER 1 / TIER 2 / TIER 3 numbered card per tier', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /THREE TIERS/i })
    expect(screen.getByText(/^TIER 1$/)).toBeInTheDocument()
    expect(screen.getByText(/^TIER 2$/)).toBeInTheDocument()
    expect(screen.getByText(/^TIER 3$/)).toBeInTheDocument()
  })

  it('renders an indigo "active" Chip on the winning tier card', async () => {
    mockListAndDetail()
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /THREE TIERS/i })
    expect(container.querySelector('[data-variant="indigo"]')).not.toBeNull()
  })

  it('applies the accent-ring data-active attribute to the winning tier card only', async () => {
    mockListAndDetail()
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /THREE TIERS/i })
    const ringed = container.querySelectorAll('[data-active="true"]')
    expect(ringed.length).toBe(1)
    expect(templatesCss).toMatch(/\.panel\[data-active="true"\]\s*\{[^}]*outline:/m)
  })

  it('renders an error alert when fetchTemplateDetail rejects', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockRejectedValue(new Error('boom'))
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load template/i)
  })

  it('renders a two-column grid container for the detail layout', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /THREE TIERS/i })
    expect(screen.getByTestId('templates-detail-layout')).toBeInTheDocument()
  })

  it('CSS module declares a two-column grid for the layout container', () => {
    expect(templatesCss).toMatch(/\.twoColumn\s*\{[^}]*grid-template-columns:\s*minmax/m)
  })

  it('renders the winning-tier template source in a <pre><code> code block, not a rendered markdown body', async () => {
    mockListAndDetail()
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /THREE TIERS/i })
    // Inside the preview pane, the body is rendered as <pre><code> not as a <h1>.
    const pane = screen.getByTestId('template-preview-pane')
    expect(pane.querySelector('pre code')).not.toBeNull()
    expect(pane.querySelector('h1')).toBeNull()
    // The code element contains the literal markdown source text (verbatim).
    expect(pane.querySelector('pre code')?.textContent ?? '').toContain('# ADR')
    // Cover all matches at the document level too — there should not be a rendered <h1>
    // anywhere just from the preview pane's body.
    expect(container.querySelectorAll('h1').length).toBeLessThanOrEqual(1)  // Only the page title.
  })

  it('renders the content-hash label with the digest computed from the winning content (AC11)', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    mockListAndDetail({ ...mockDetail, sha256: expectedDigest })
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText(expectedDigest)).toBeInTheDocument()
  })

  it('renders the winning-tier path alongside the content-hash label', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    mockListAndDetail({ ...mockDetail, sha256: expectedDigest })
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText(expectedDigest)
    // The path is rendered both in the tier card and in the preview header,
    // so use getAllByText.
    expect(screen.getAllByText('/plugin/templates/adr.md').length).toBeGreaterThanOrEqual(1)
  })

  it('omits the content-hash label when sha256 is absent', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /THREE TIERS/i })
    expect(screen.queryByText(/^sha256-/)).toBeNull()
  })

  it('content-hash label is non-interactive (AC13)', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    mockListAndDetail({ ...mockDetail, sha256: expectedDigest })
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    const label = await screen.findByText(expectedDigest)
    expect(label.getAttribute('role')).toBeNull()
    expect(label.getAttribute('tabindex')).toBeNull()
    expect(label.getAttribute('title')).toBeNull()
    await userEvent.click(label)
    expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*pointer/m)
    expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*copy/m)
    expect(templatesCss).not.toMatch(/\.contentHashLabel:hover/)
  })

  it('updates the content-hash label end-to-end via dispatchSseEvent (AC12)', async () => {
    const firstDigest = digestForContent('# ADR\nBody.')
    const secondDigest = digestForContent('# ADR\nBody. v2.')
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    const spy = vi
      .spyOn(fetchModule, 'fetchTemplateDetail')
      .mockResolvedValueOnce({ ...mockDetail, sha256: firstDigest })
      .mockResolvedValueOnce({ ...mockDetail, sha256: secondDigest })

    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={qc}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    )
    render(<LibraryTemplatesView name="adr" />, { wrapper })
    await screen.findByText(firstDigest)

    dispatchSseEvent(
      {
        type: 'template-changed',
        template: 'adr',
        sha256: secondDigest,
        timestamp: '2026-05-18T00:00:00Z',
      },
      qc,
    )

    await waitFor(() => expect(screen.queryByText(secondDigest)).not.toBeNull(), { timeout: 1_000 })
    expect(spy).toHaveBeenCalledTimes(2)
  })

  it('omits the content-hash label when an SSE event clears sha256 (AC10 live path)', async () => {
    const firstDigest = digestForContent('# ADR\nBody.')
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: mockTemplates })
    vi.spyOn(fetchModule, 'fetchTemplateDetail')
      .mockResolvedValueOnce({ ...mockDetail, sha256: firstDigest })
      .mockResolvedValueOnce({ ...mockDetail, sha256: undefined })

    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={qc}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    )
    render(<LibraryTemplatesView name="adr" />, { wrapper })
    await screen.findByText(firstDigest)

    dispatchSseEvent(
      { type: 'template-changed', template: 'adr', timestamp: '2026-05-18T00:00:00Z' },
      qc,
    )

    await waitFor(() => expect(screen.queryByText(/^sha256-/)).toBeNull(), { timeout: 1_000 })
  })

  it('CSS module no longer defines the legacy .activeBadge rule', () => {
    expect(templatesCss).not.toMatch(/\.activeBadge\b/)
  })
})
