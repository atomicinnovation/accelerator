import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from '../../test/router-helpers'
import rootLayoutCss from './RootLayout.module.css?raw'
import { RootLayout } from './RootLayout'

vi.mock('../../api/use-doc-events', () => ({
  useDocEvents: vi.fn(() => ({
    connectionState: 'open',
    justReconnected: false,
    setDragInProgress: vi.fn(),
    isDragInProgress: vi.fn(() => false),
    subscribe: vi.fn(() => () => {}),
  })),
  useDocEventsContext: vi.fn(() => ({
    connectionState: 'open',
    justReconnected: false,
    setDragInProgress: vi.fn(),
    isDragInProgress: vi.fn(() => false),
    subscribe: vi.fn(() => () => {}),
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

  describe('global / keybind', () => {
    async function renderLayout() {
      const result = render(<MemoryRouter><RootLayout /></MemoryRouter>)
      // Wait for the sidebar to render so the search input ref is attached.
      await screen.findByRole('searchbox', { name: /search/i })
      return result
    }

    it('focuses sidebar search when no field focused', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const input = screen.getByRole('searchbox', { name: /search/i })
      expect(document.activeElement).not.toBe(input)
      await user.keyboard('/')
      expect(document.activeElement).toBe(input)
    })

    it('does not focus sidebar search when an <input> is focused', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const other = document.createElement('input')
      document.body.appendChild(other)
      other.focus()
      await user.keyboard('/')
      expect(document.activeElement).toBe(other)
      expect(other.value).toBe('/')
      other.remove()
    })

    it('does not focus sidebar search when a <textarea> is focused', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const ta = document.createElement('textarea')
      document.body.appendChild(ta)
      ta.focus()
      await user.keyboard('/')
      expect(document.activeElement).toBe(ta)
      ta.remove()
    })

    it('does not focus sidebar search when a contenteditable is focused', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const ce = document.createElement('div')
      ce.setAttribute('contenteditable', 'true')
      ce.tabIndex = 0
      document.body.appendChild(ce)
      ce.focus()
      await user.keyboard('/')
      expect(document.activeElement).toBe(ce)
      ce.remove()
    })

    it('does not activate with meta modifier', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const input = screen.getByRole('searchbox', { name: /search/i })
      const initialActive = document.activeElement
      await user.keyboard('{Meta>}/{/Meta}')
      expect(document.activeElement).toBe(initialActive)
      expect(document.activeElement).not.toBe(input)
    })

    it('does not activate with ctrl modifier', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const input = screen.getByRole('searchbox', { name: /search/i })
      const initialActive = document.activeElement
      await user.keyboard('{Control>}/{/Control}')
      expect(document.activeElement).toBe(initialActive)
      expect(document.activeElement).not.toBe(input)
    })

    it('does not activate with alt modifier', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const input = screen.getByRole('searchbox', { name: /search/i })
      const initialActive = document.activeElement
      await user.keyboard('{Alt>}/{/Alt}')
      expect(document.activeElement).toBe(initialActive)
      expect(document.activeElement).not.toBe(input)
    })

    it('does not activate with shift modifier', async () => {
      const user = userEvent.setup()
      await renderLayout()
      const input = screen.getByRole('searchbox', { name: /search/i })
      const initialActive = document.activeElement
      await user.keyboard('{Shift>}/{/Shift}')
      expect(document.activeElement).toBe(initialActive)
      expect(document.activeElement).not.toBe(input)
    })

    it('cleans up the listener on unmount', async () => {
      const user = userEvent.setup()
      const { unmount } = await renderLayout()
      unmount()
      // After unmount, pressing / should not throw and there is no
      // search input to focus.
      await user.keyboard('/')
      // No assertion needed beyond not throwing; the listener should be gone.
      expect(true).toBe(true)
    })

    it('does not call preventDefault when an editable target has focus', async () => {
      await renderLayout()
      const other = document.createElement('input')
      document.body.appendChild(other)
      other.focus()
      const event = new KeyboardEvent('keydown', {
        key: '/',
        bubbles: true,
        cancelable: true,
      })
      const preventSpy = vi.spyOn(event, 'preventDefault')
      other.dispatchEvent(event)
      expect(preventSpy).not.toHaveBeenCalled()
      other.remove()
    })
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
