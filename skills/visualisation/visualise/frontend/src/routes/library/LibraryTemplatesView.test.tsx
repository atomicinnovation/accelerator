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
import codeSyntaxCss from '../../styles/code-syntax.global.css?raw'
import { assertSelectorColorIs } from '../../styles/testing/cssRules'

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
    expect(await screen.findByRole('heading', { name: /TIERS · ADR\.MD/i })).toBeInTheDocument()
  })

  it('marks the selected row with aria-current="page"', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    const row = await screen.findByRole('link', { name: /adr\.md/i })
    expect(row.getAttribute('aria-current')).toBe('page')
  })

  it('renders a TIER 1 / TIER 2 / TIER 3 numbered card per tier in priority order', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    // Tier 1 is highest priority (config-override), Tier 3 lowest (plugin-default).
    expect(screen.getByText(/^TIER 1$/)).toBeInTheDocument()
    expect(screen.getByText(/^TIER 2$/)).toBeInTheDocument()
    expect(screen.getByText(/^TIER 3$/)).toBeInTheDocument()
  })

  it('labels Tier 1 as "highest priority" and Tier 3 as "plugin-default"', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    expect(screen.getByText(/highest priority/i)).toBeInTheDocument()
    expect(screen.getByText(/plugin-default · always present/i)).toBeInTheDocument()
  })

  it('uses the user-override parent directory (not the file) as the Tier 2 description copy', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    // Path: /meta/templates/adr.md → description: "/meta/templates/ in this repo".
    expect(screen.getByText(/\/meta\/templates\/ in this repo/i)).toBeInTheDocument()
    // And the prior "file in this repo" copy must be gone.
    expect(screen.queryByText(/\/meta\/templates\/adr\.md in this repo/i)).toBeNull()
  })

  it('includes the configSource path in the Tier 1 description when present', async () => {
    const detailWithSource: TemplateDetail = {
      ...mockDetail,
      activeTier: 'config-override',
      tiers: [
        { source: 'config-override', path: '.accelerator/templates/adr.md',
          present: true, active: true, configSource: '.accelerator/config.md',
          content: '# from config', etag: 'sha256-x' },
        { source: 'user-override',   path: '/meta/templates/adr.md',
          present: false, active: false },
        { source: 'plugin-default',  path: '/plugin/templates/adr.md',
          present: true, active: false, content: '# plugin', etag: 'sha256-y' },
      ],
    }
    mockListAndDetail(detailWithSource)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    expect(
      screen.getByText(/highest priority · \.accelerator\/config\.md/),
    ).toBeInTheDocument()
  })

  it('renders an indigo "active" Chip on the winning tier card', async () => {
    mockListAndDetail()
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    expect(container.querySelector('[data-variant="indigo"]')).not.toBeNull()
  })

  it('applies the accent-ring data-active attribute to the winning tier card only', async () => {
    mockListAndDetail()
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    const ringed = container.querySelectorAll('[data-active="true"]')
    expect(ringed.length).toBe(1)
    // Active ring is implemented as border + box-shadow halo on
    // `.panel[data-active="true"]`, matching the prototype.
    expect(templatesCss).toMatch(/\.panel\[data-active="true"\]\s*\{[^}]*box-shadow:/m)
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
    await screen.findByRole('heading', { name: /TIERS · /i })
    expect(screen.getByTestId('templates-detail-layout')).toBeInTheDocument()
  })

  it('CSS module declares a two-column grid for the layout container', () => {
    expect(templatesCss).toMatch(/\.twoColumn\s*\{[^}]*grid-template-columns:\s*minmax/m)
  })

  it('renders the winning-tier template source verbatim (no markdown rendering)', async () => {
    mockListAndDetail()
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    const pane = screen.getByTestId('template-preview-pane')
    // The preview body must NOT render the markdown into <h1>/<p>. The
    // entire source is emitted as plain text (with token spans) line by
    // line inside the preview body container.
    expect(pane.querySelector('h1')).toBeNull()
    expect(pane.textContent ?? '').toContain('# ADR')
    expect(pane.textContent ?? '').toContain('Body.')
    // Only the page title <h1> should exist at the document level.
    expect(container.querySelectorAll('h1').length).toBe(1)
  })

  it('emits highlight.js token classes for frontmatter YAML, markdown body, and template variables', async () => {
    // The highlighter uses highlight.js (already a project dep) with YAML
    // and markdown sub-grammars, and wraps `{{var}}` in a custom
    // `hljs-template-variable` span. The CSS module theme-colours these
    // via `:global(.hljs-*)` rules, reusable wherever hljs renders.
    const detailWithFm: TemplateDetail = {
      ...mockDetail,
      tiers: [
        { source: 'config-override', path: '/no-config', present: false, active: false },
        { source: 'user-override',   path: '/meta/templates/adr.md', present: false, active: false },
        { source: 'plugin-default',  path: '/plugin/templates/adr.md', present: true, active: true,
          content: '---\ntitle: "{{title}}"\n---\n\n# Body {{author}}', etag: 'sha256-x' },
      ],
    }
    mockListAndDetail(detailWithFm)
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    // YAML frontmatter delimiter `---` is wrapped in `.hljs-meta`.
    expect(container.querySelector('.hljs-meta')).not.toBeNull()
    // Markdown heading marker → `.hljs-section`.
    expect(container.querySelector('.hljs-section')).not.toBeNull()
    // Both `{{title}}` and `{{author}}` are wrapped in `.hljs-template-variable`.
    expect(container.querySelectorAll('.hljs-template-variable').length).toBeGreaterThanOrEqual(2)
    // After 0076: hljs class colours come exclusively from the shared
    // `code-syntax.global.css` layer; the templates preview module must
    // not declare any `:global(.hljs-` colour rule. Broader than the
    // prior `.previewBody :global(.hljs-` check — catches selector
    // renames, @media wraps, and nested re-introductions.
    expect(templatesCss).not.toMatch(/:global\(\.hljs-/)
  })

  it('preserves empty lines in the template body via .tpl-line wrappers', async () => {
    const detailWithBlank: TemplateDetail = {
      ...mockDetail,
      tiers: [
        { source: 'config-override', path: '/no-config', present: false, active: false },
        { source: 'user-override',   path: '/meta/templates/adr.md', present: false, active: false },
        { source: 'plugin-default',  path: '/plugin/templates/adr.md', present: true, active: true,
          // Two blank lines between the heading and the body — the
          // prototype renders these as min-height: 1em rows so the
          // visual spacing is preserved.
          content: '# Title\n\n\nBody after two blanks.', etag: 'sha256-x' },
      ],
    }
    mockListAndDetail(detailWithBlank)
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    const lines = container.querySelectorAll('.tpl-line')
    // 4 source lines → 4 tpl-line wrappers (no collapse).
    expect(lines.length).toBe(4)
    // Empty lines get a &nbsp; placeholder so they preserve height.
    expect(lines[1]?.textContent ?? '').toMatch(/ |^\s*$/)
    expect(lines[2]?.textContent ?? '').toMatch(/ |^\s*$/)
  })

  it('renders the content-hash label truncated (~12 chars + ellipsis), with the full digest in a title attribute', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    mockListAndDetail({ ...mockDetail, sha256: expectedDigest })
    const { container } = render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    // The visible label is the truncated form.
    const truncated = `${expectedDigest.slice(0, 12)}…`
    expect(await screen.findByText(truncated)).toBeInTheDocument()
    // The full hash is recoverable via the title attribute on hover and via
    // a data-* attribute, so the AC11 contract (UI value == backend value)
    // still holds without dumping a 70-character string into the layout.
    const label = container.querySelector('[data-full-sha]')
    expect(label?.getAttribute('data-full-sha')).toBe(expectedDigest)
    expect(label?.getAttribute('title')).toBe(expectedDigest)
  })

  it('renders the winning-tier path alongside the content-hash label', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    mockListAndDetail({ ...mockDetail, sha256: expectedDigest })
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    // The path is rendered both in the tier card and in the preview header,
    // so use getAllByText.
    expect(screen.getAllByText('/plugin/templates/adr.md').length).toBeGreaterThanOrEqual(1)
  })

  it('omits the content-hash label when sha256 is absent', async () => {
    mockListAndDetail()
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: /TIERS · /i })
    expect(screen.queryByText(/^sha256-/)).toBeNull()
  })

  it('content-hash label is non-interactive (AC13)', async () => {
    const expectedDigest = digestForContent('# ADR\nBody.')
    mockListAndDetail({ ...mockDetail, sha256: expectedDigest })
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    const truncated = `${expectedDigest.slice(0, 12)}…`
    const label = await screen.findByText(truncated)
    expect(label.getAttribute('role')).toBeNull()
    expect(label.getAttribute('tabindex')).toBeNull()
    // `title` is intentionally set to surface the full digest on hover —
    // that's a static tooltip, not interactivity. Click still has no
    // observable side-effect.
    expect(label.getAttribute('title')).toBe(expectedDigest)
    await userEvent.click(label)
    expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*pointer/m)
    expect(templatesCss).not.toMatch(/\.contentHashLabel\s*\{[^}]*cursor:\s*copy/m)
    expect(templatesCss).not.toMatch(/\.contentHashLabel:hover/)
  })

  it('updates the content-hash label end-to-end via dispatchSseEvent (AC12)', async () => {
    const firstDigest = digestForContent('# ADR\nBody.')
    const secondDigest = digestForContent('# ADR\nBody. v2.')
    const truncatedFirst = `${firstDigest.slice(0, 12)}…`
    const truncatedSecond = `${secondDigest.slice(0, 12)}…`
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
    await screen.findByText(truncatedFirst)

    dispatchSseEvent(
      {
        type: 'template-changed',
        template: 'adr',
        sha256: secondDigest,
        timestamp: '2026-05-18T00:00:00Z',
      },
      qc,
    )

    await waitFor(() => expect(screen.queryByText(truncatedSecond)).not.toBeNull(), { timeout: 1_000 })
    expect(spy).toHaveBeenCalledTimes(2)
  })

  it('omits the content-hash label when an SSE event clears sha256 (AC10 live path)', async () => {
    const firstDigest = digestForContent('# ADR\nBody.')
    const truncated = `${firstDigest.slice(0, 12)}…`
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
    await screen.findByText(truncated)

    dispatchSseEvent(
      { type: 'template-changed', template: 'adr', timestamp: '2026-05-18T00:00:00Z' },
      qc,
    )

    await waitFor(() => expect(screen.queryByText(truncated)).toBeNull(), { timeout: 1_000 })
    expect(screen.queryByText(/sha256-/)).toBeNull()
  })

  it('CSS module no longer defines the legacy .activeBadge rule', () => {
    expect(templatesCss).not.toMatch(/\.activeBadge\b/)
  })
})

describe('LibraryTemplatesView: hljs class colouring delegated to shared layer', () => {
  // Subset of REQUIRED_MAPPINGS in src/styles/code-syntax.test.ts
  // covering classes previously declared locally in
  // LibraryTemplatesView.module.css prior to 0076. If the shared
  // layer mapping changes, update BOTH this array and
  // REQUIRED_MAPPINGS in lock-step.
  const PREVIOUSLY_LOCAL_MAPPINGS: ReadonlyArray<[selector: string, token: string]> = [
    ['.hljs-attr',              'tk-attr'],
    ['.hljs-meta',              'tk-deco'],
    ['.hljs-string',            'tk-str'],
    ['.hljs-number',            'tk-num'],
    ['.hljs-literal',           'tk-lit'],
    ['.hljs-section',           'tk-header'],
    ['.hljs-bullet',            'tk-com'],
    ['.hljs-code',              'tk-com'],
    ['.hljs-quote',             'tk-com'],
    ['.hljs-link',              'tk-anchor'],
    ['.hljs-symbol',            'tk-anchor'],
    ['.hljs-template-variable', 'tk-var'],
  ]

  for (const [selector, token] of PREVIOUSLY_LOCAL_MAPPINGS) {
    it(`shared layer maps ${selector} → var(--${token})`, () => {
      assertSelectorColorIs(codeSyntaxCss, selector, token)
    })
  }

  it('shared layer preserves font-weight: 600 on .hljs-section (was local)', () => {
    expect(codeSyntaxCss).toMatch(
      /\.hljs-section[^{]*\{[^}]*font-weight:\s*600/,
    )
  })

  it('shared layer carries emphasis/strong styling for markdown emphasis', () => {
    expect(codeSyntaxCss).toMatch(
      /\.hljs-emphasis\s*\{[^}]*font-style:\s*italic/,
    )
    expect(codeSyntaxCss).toMatch(
      /\.hljs-strong\s*\{[^}]*font-weight:\s*600/,
    )
  })

  it('templates preview module no longer declares any local hljs colour rules', () => {
    // Broader than the prior `.previewBody :global(.hljs-` check —
    // catches @media wraps, selector renames, and any nested
    // re-introduction.
    expect(templatesCss).not.toMatch(/:global\(\.hljs-/)
  })
})
