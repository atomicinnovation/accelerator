import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import toasterCss from './Toaster.module.css?raw'

vi.mock('../../api/use-toast', () => ({ useToast: vi.fn() }))

import { useToast } from '../../api/use-toast'
import { Toaster } from './Toaster'

function mockToastState(overrides: Partial<ReturnType<typeof useToast>> = {}) {
  const handle = {
    toasts: [],
    showToast: vi.fn(),
    dismissToast: vi.fn(),
    pauseToast: vi.fn(),
    resumeToast: vi.fn(),
    ...overrides,
  } as unknown as ReturnType<typeof useToast>
  vi.mocked(useToast).mockReturnValue(handle)
  return handle
}

describe('Toaster', () => {
  beforeEach(() => vi.clearAllMocks())

  it('renders heading, message, icon, and dismiss button for a single toast', () => {
    mockToastState({
      toasts: [{ id: 1, heading: 'External edit detected', message: 'hello' }],
    })
    render(<Toaster />)

    expect(screen.getByText('External edit detected')).toBeInTheDocument()
    expect(screen.getByText('hello')).toBeInTheDocument()
  })

  it('renders backtick-delimited segments inside the message as inline <code>', () => {
    mockToastState({
      toasts: [
        {
          id: 1,
          heading: 'External edit detected',
          message: '`meta/work/0007-foo.md` was updated while you were looking at it.',
        },
      ],
    })
    render(<Toaster />)
    const viewport = screen.getByTestId('toaster-viewport')
    const code = viewport.querySelector('code')
    expect(code).not.toBeNull()
    expect(code!.textContent).toBe('meta/work/0007-foo.md')
    // The literal backticks must not appear in the rendered DOM.
    expect(viewport.textContent).not.toContain('`')

    const icon = screen.getByTestId('toaster-icon')
    expect(icon.tagName.toLowerCase()).toBe('svg')

    expect(
      screen.getByRole('button', { name: 'Dismiss notification' }),
    ).toBeInTheDocument()
  })

  it('clicking the dismiss button calls dismissToast with that toast id', () => {
    const handle = mockToastState({
      toasts: [{ id: 42, heading: 'h', message: 'm' }],
    })
    render(<Toaster />)
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }))
    expect(handle.dismissToast).toHaveBeenCalledWith(42)
  })

  it('mouseEnter pauses and mouseLeave resumes the toast timer', () => {
    const handle = mockToastState({
      toasts: [{ id: 7, heading: 'h', message: 'm' }],
    })
    render(<Toaster />)
    const card = screen.getByText('h').closest('div')!.parentElement!
    fireEvent.mouseEnter(card)
    expect(handle.pauseToast).toHaveBeenCalledWith(7)
    fireEvent.mouseLeave(card)
    expect(handle.resumeToast).toHaveBeenCalledWith(7)
  })

  it('focusing the close button bubbles to card and pauses; blur resumes', () => {
    const handle = mockToastState({
      toasts: [{ id: 3, heading: 'h', message: 'm' }],
    })
    render(<Toaster />)
    const close = screen.getByRole('button', { name: 'Dismiss notification' })
    fireEvent.focus(close)
    expect(handle.pauseToast).toHaveBeenCalledWith(3)
    fireEvent.blur(close)
    expect(handle.resumeToast).toHaveBeenCalledWith(3)
  })

  it('Escape dismisses the most-recent (last) toast', () => {
    const handle = mockToastState({
      toasts: [
        { id: 1, heading: 'first', message: 'a' },
        { id: 2, heading: 'last', message: 'b' },
      ],
    })
    render(<Toaster />)
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).toHaveBeenCalledWith(2)
  })

  it('Escape is a no-op (no listener attached) when the stack is empty', () => {
    const handle = mockToastState({ toasts: [] })
    render(<Toaster />)
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).not.toHaveBeenCalled()
  })

  it('removes the document keydown listener on unmount', () => {
    const handle = mockToastState({
      toasts: [{ id: 1, heading: 'h', message: 'm' }],
    })
    const { unmount } = render(<Toaster />)
    unmount()
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).not.toHaveBeenCalled()
  })

  it('empty stack: viewport rendered but no toast cards', () => {
    mockToastState({ toasts: [] })
    render(<Toaster />)
    const viewport = screen.getByTestId('toaster-viewport')
    expect(viewport).toBeInTheDocument()
    expect(screen.queryByTestId('toaster-icon')).toBeNull()
  })

  it('viewport is role="status" aria-live="polite"', () => {
    mockToastState({ toasts: [] })
    render(<Toaster />)
    const viewport = screen.getByTestId('toaster-viewport')
    expect(viewport.getAttribute('role')).toBe('status')
    expect(viewport.getAttribute('aria-live')).toBe('polite')
  })

  it('renders two cards in the single viewport for a stack of two', () => {
    mockToastState({
      toasts: [
        { id: 1, heading: 'one', message: 'a' },
        { id: 2, heading: 'two', message: 'b' },
      ],
    })
    render(<Toaster />)
    expect(screen.getByText('one')).toBeInTheDocument()
    expect(screen.getByText('two')).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: 'Dismiss notification' })).toHaveLength(2)
    // Single status region
    expect(screen.getAllByRole('status')).toHaveLength(1)
  })

  describe('CSS source assertions', () => {
    it('.toast binds var(--ac-bg-card)', () => {
      expect(toasterCss).toMatch(/\.toast[^{]*\{[^}]*background[^}]*var\(--ac-bg-card\)/)
    })

    it('.toast binds var(--ac-shadow-lift)', () => {
      expect(toasterCss).toMatch(/\.toast[^{]*\{[^}]*box-shadow[^}]*var\(--ac-shadow-lift\)/)
    })

    it('viewport is position: fixed', () => {
      expect(toasterCss).toMatch(/\.viewport[^{]*\{[^}]*position:\s*fixed/)
    })

    it('.icon binds var(--ac-accent) and NOT --ac-ok / --ac-warn', () => {
      expect(toasterCss).toMatch(/\.icon[^{]*\{[^}]*color:\s*var\(--ac-accent\)/)
      expect(toasterCss).not.toMatch(/\.icon[^{]*\{[^}]*color:\s*var\(--ac-ok\)/)
      expect(toasterCss).not.toMatch(/\.icon[^{]*\{[^}]*color:\s*var\(--ac-warn\)/)
    })

    it('.toast binds an accent border-left (prototype "category bar")', () => {
      expect(toasterCss).toMatch(/\.toast[^{]*\{[^}]*border-left[^;]*var\(--ac-accent\)/)
    })
  })
})
