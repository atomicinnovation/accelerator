import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from '../../test/router-helpers'
import rootLayoutCss from './RootLayout.module.css?raw'
import { RootLayout } from './RootLayout'

vi.mock('../../api/use-doc-events', () => ({
  useDocEvents: vi.fn(() => ({
    connectionState: 'open',
    justReconnected: false,
    setDragInProgress: vi.fn(),
  })),
  useDocEventsContext: vi.fn(() => ({
    connectionState: 'open',
    justReconnected: false,
    setDragInProgress: vi.fn(),
  })),
  DocEventsContext: { Provider: ({ children }: any) => children },
}))

vi.mock('../../api/use-origin', () => ({
  useOrigin: vi.fn(() => 'localhost'),
}))

vi.mock('@tanstack/react-query', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-query')>()
  return {
    ...actual,
    useQuery: vi.fn(() => ({ data: [] })),
  }
})

describe('RootLayout', () => {
  it('renders a <main> element', async () => {
    render(<MemoryRouter><RootLayout /></MemoryRouter>)
    expect(await screen.findByRole('main')).toBeInTheDocument()
  })

  it('renders a <nav> (sidebar)', async () => {
    render(<MemoryRouter><RootLayout /></MemoryRouter>)
    // The sidebar renders as <nav> and the breadcrumbs also render as <nav>
    // Confirm sidebar exists by looking for the nav with section headings
    expect(await screen.findByRole('navigation')).toBeInTheDocument()
  })

  it('renders a <header> (Topbar) above the body row in DOM order', async () => {
    const { container } = render(<MemoryRouter><RootLayout /></MemoryRouter>)
    await screen.findByRole('main')
    const root = container.firstChild as HTMLElement
    const header = root?.querySelector('header')
    const body = header?.nextElementSibling
    expect(header?.tagName).toBe('HEADER')
    expect(body).not.toBeNull()
  })

  describe('CSS source assertions', () => {
    it('.root declares flex-direction: column', () => {
      expect(rootLayoutCss).toMatch(/\.root\s*\{[^}]*flex-direction:\s*column/)
    })

    it('.root declares min-height: 100vh', () => {
      expect(rootLayoutCss).toMatch(/\.root\s*\{[^}]*min-height:\s*100vh/)
    })

    it('.body declares flex: 1', () => {
      expect(rootLayoutCss).toMatch(/\.body\s*\{[^}]*flex:\s*1/)
    })
  })
})
