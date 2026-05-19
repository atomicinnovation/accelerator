import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createHash } from 'node:crypto'
import React from 'react'
import { LibraryTemplatesView } from './LibraryTemplatesView'
import * as fetchModule from '../../api/fetch'
import type { TemplateDetail } from '../../api/types'
import { dispatchSseEvent } from '../../api/use-doc-events'
import { MemoryRouter } from '../../test/router-helpers'
import templatesCss from './LibraryTemplatesView.module.css?raw'

function digestForContent(content: string): string {
  return `sha256-${createHash('sha256').update(content).digest('hex')}`
}

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

  it('renders an indigo "active" Chip on the active tier panel', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText('active')
    expect(container.querySelector('[data-variant="indigo"]')).not.toBeNull()
  })

  it('renders neutral "absent" Chips on absent tier panels', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText('active')
    const neutralChips = container.querySelectorAll('[data-variant="neutral"]')
    expect(neutralChips.length).toBeGreaterThanOrEqual(2)
  })

  it('CSS module no longer defines the legacy .activeBadge rule', () => {
    expect(templatesCss).not.toMatch(/\.activeBadge\b/)
  })

  it('renders a two-column grid container for the detail layout', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText('active')
    expect(screen.getByTestId('templates-detail-layout')).toBeInTheDocument()
  })

  it('CSS module declares a two-column grid for the layout container', () => {
    expect(templatesCss).toMatch(/\.twoColumn\s*\{[^}]*grid-template-columns:\s*minmax/m)
  })

  it('applies the accent-ring class to the winning tier card only', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText('active')
    const ringed = container.querySelectorAll('[data-active="true"]')
    expect(ringed.length).toBe(1)
    expect(templatesCss).toMatch(/\.panel\[data-active="true"\]\s*\{[^}]*outline:/m)
  })

  it('retains the indigo "active" Chip alongside the ring', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText('active')
    expect(container.querySelector('[data-variant="indigo"]')).not.toBeNull()
  })

  it('renders the content-hash label with the digest computed from the winning content (AC11)', async () => {
    const winningContent = '# ADR\nBody.'
    const expectedDigest = digestForContent(winningContent)
    const mockWithSha: TemplateDetail = { ...mockDetail, sha256: expectedDigest }
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockWithSha)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText(expectedDigest)).toBeInTheDocument()
  })

  it('renders the content-hash label as the first row of the preview pane', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    const mockWithSha: TemplateDetail = { ...mockDetail, sha256: expectedDigest }
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockWithSha)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    const label = await screen.findByText(expectedDigest)
    const body = await screen.findByText('Body.')
    // eslint-disable-next-line no-bitwise
    expect(
      label.compareDocumentPosition(body) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy()
  })

  it('renders the winning-tier path alongside the content-hash label', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    const mockWithSha: TemplateDetail = { ...mockDetail, sha256: expectedDigest }
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockWithSha)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText(expectedDigest)
    expect(screen.getAllByText('/plugin/templates/adr.md').length).toBeGreaterThanOrEqual(1)
  })

  it('omits the content-hash label when sha256 is absent', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText('active')
    expect(screen.queryByText(/^sha256-/)).toBeNull()
  })

  it('content-hash label is non-interactive (AC13)', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    const mockWithSha: TemplateDetail = { ...mockDetail, sha256: expectedDigest }
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockWithSha)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    const label = await screen.findByText(expectedDigest)
    expect(label.getAttribute('role')).toBeNull()
    expect(label.getAttribute('tabindex')).toBeNull()
    expect(label.getAttribute('title')).toBeNull()
    await userEvent.click(label)
    // No side-effects observable.
    expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*pointer/m)
    expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*copy/m)
    expect(templatesCss).not.toMatch(/\.contentHashLabel:hover/)
  })

  it('updates the content-hash label end-to-end via dispatchSseEvent (AC12)', async () => {
    const firstDigest = digestForContent('# ADR\nBody.')
    const secondDigest = digestForContent('# ADR\nBody. v2.')
    const first: TemplateDetail = { ...mockDetail, sha256: firstDigest }
    const second: TemplateDetail = { ...mockDetail, sha256: secondDigest }
    const spy = vi
      .spyOn(fetchModule, 'fetchTemplateDetail')
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second)

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

    await waitFor(() => expect(screen.queryByText(secondDigest)).not.toBeNull(), {
      timeout: 1_000,
    })
    expect(spy).toHaveBeenCalledTimes(2)
  })

  it('omits the content-hash label when an SSE event clears sha256 (AC10 live path)', async () => {
    const firstDigest = digestForContent('# ADR\nBody.')
    const first: TemplateDetail = { ...mockDetail, sha256: firstDigest }
    const cleared: TemplateDetail = { ...mockDetail, sha256: undefined }
    vi.spyOn(fetchModule, 'fetchTemplateDetail')
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(cleared)

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

    await waitFor(() => expect(screen.queryByText(/^sha256-/)).toBeNull(), {
      timeout: 1_000,
    })
  })
})
