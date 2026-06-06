import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fireEvent, render, screen, within } from '@testing-library/react'
import toasterCss from './Toaster.module.css?raw'

vi.mock('../../api/use-toast', () => ({ useToast: vi.fn() }))
vi.mock('../../api/use-doc-events', () => ({ useDocEventsContext: vi.fn() }))

import { useToast } from '../../api/use-toast'
import { useDocEventsContext } from '../../api/use-doc-events'
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

function mockDocEvents(isDragInProgress = false) {
  vi.mocked(useDocEventsContext).mockReturnValue({
    setDragInProgress: vi.fn(),
    isDragInProgress: vi.fn(() => isDragInProgress),
    connectionState: 'open',
    justReconnected: false,
    subscribe: () => () => {},
  })
}

describe('Toaster', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockDocEvents(false)
  })

  it('renders heading, message, icon, and dismiss button for a single toast', () => {
    mockToastState({
      toasts: [{ id: 1, heading: 'External edit detected', message: 'hello', kind: 'info' }],
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
          kind: 'info',
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
      toasts: [{ id: 42, heading: 'h', message: 'm', kind: 'info' }],
    })
    render(<Toaster />)
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }))
    expect(handle.dismissToast).toHaveBeenCalledWith(42)
  })

  it('mouseEnter pauses and mouseLeave resumes the toast timer', () => {
    const handle = mockToastState({
      toasts: [{ id: 7, heading: 'h', message: 'm', kind: 'info' }],
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
      toasts: [{ id: 3, heading: 'h', message: 'm', kind: 'info' }],
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
        { id: 1, heading: 'first', message: 'a', kind: 'info' },
        { id: 2, heading: 'last', message: 'b', kind: 'info' },
      ],
    })
    render(<Toaster />)
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).toHaveBeenCalledWith(2)
  })

  it('Escape during an active drag does NOT dismiss (dnd-kit owns drag-cancel)', () => {
    mockDocEvents(true)
    const handle = mockToastState({
      toasts: [{ id: 9, heading: 'Move failed', message: 'oops', kind: 'error' }],
    })
    render(<Toaster />)
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).not.toHaveBeenCalled()
  })

  it('Escape is a no-op (no listener attached) when the stack is empty', () => {
    const handle = mockToastState({ toasts: [] })
    render(<Toaster />)
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).not.toHaveBeenCalled()
  })

  it('removes the document keydown listener on unmount', () => {
    const handle = mockToastState({
      toasts: [{ id: 1, heading: 'h', message: 'm', kind: 'info' }],
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

  it('renders one polite (role=status) and one assertive (role=alert) live region', () => {
    mockToastState({ toasts: [] })
    render(<Toaster />)
    const polite = screen.getByTestId('toaster-region-polite')
    expect(polite.getAttribute('role')).toBe('status')
    expect(polite.getAttribute('aria-live')).toBe('polite')
    const assertive = screen.getByTestId('toaster-region-assertive')
    expect(assertive.getAttribute('role')).toBe('alert')
    expect(assertive.getAttribute('aria-live')).toBe('assertive')
    // Exactly one of each region.
    expect(screen.getAllByRole('status')).toHaveLength(1)
    expect(screen.getAllByRole('alert')).toHaveLength(1)
  })

  it('renders two cards for a stack of two info toasts in the polite region', () => {
    mockToastState({
      toasts: [
        { id: 1, heading: 'one', message: 'a', kind: 'info' },
        { id: 2, heading: 'two', message: 'b', kind: 'info' },
      ],
    })
    render(<Toaster />)
    const polite = screen.getByTestId('toaster-region-polite')
    expect(within(polite).getByText('one')).toBeInTheDocument()
    expect(within(polite).getByText('two')).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: 'Dismiss notification' })).toHaveLength(2)
  })

  it('routes an error toast to the assertive region with data-kind="error" and the error icon', () => {
    mockToastState({
      toasts: [{ id: 1, heading: 'Move failed', message: 'conflict', kind: 'error' }],
    })
    render(<Toaster />)
    const assertive = screen.getByTestId('toaster-region-assertive')
    const card = within(assertive).getByText('Move failed').closest('[data-kind]')!
    expect(card.getAttribute('data-kind')).toBe('error')
    // The error icon (alert triangle) lives in the assertive region.
    expect(within(assertive).getByTestId('toaster-icon')).toBeInTheDocument()
    // Nothing leaked into the polite region.
    expect(within(screen.getByTestId('toaster-region-polite')).queryByTestId('toaster-icon')).toBeNull()
  })

  it('routes info/ok to the polite region and error to the assertive region simultaneously', () => {
    mockToastState({
      toasts: [
        { id: 1, heading: 'saved', message: '', kind: 'ok' },
        { id: 2, heading: 'Move failed', message: 'conflict', kind: 'error' },
        { id: 3, heading: 'fyi', message: 'note', kind: 'info' },
      ],
    })
    render(<Toaster />)
    const polite = screen.getByTestId('toaster-region-polite')
    const assertive = screen.getByTestId('toaster-region-assertive')
    // Polite holds ok + info in insertion order; assertive holds the error.
    expect(within(polite).getByText('saved')).toBeInTheDocument()
    expect(within(polite).getByText('fyi')).toBeInTheDocument()
    expect(within(assertive).getByText('Move failed')).toBeInTheDocument()
    expect(within(assertive).queryByText('saved')).toBeNull()
  })

  it('renders the visually-hidden severity prefix per kind (announced first)', () => {
    mockToastState({
      toasts: [
        { id: 1, heading: 'all good', message: '', kind: 'ok' },
        { id: 2, heading: 'broken', message: 'why', kind: 'error' },
      ],
    })
    render(<Toaster />)
    const okCard = screen.getByText('all good').closest('[data-kind]')!
    expect(okCard.textContent?.startsWith('Success: ')).toBe(true)
    const errCard = screen.getByText('broken').closest('[data-kind]')!
    expect(errCard.textContent?.startsWith('Error: ')).toBe(true)
  })

  it('omits the message paragraph for a heading-only (empty message) toast', () => {
    mockToastState({
      toasts: [{ id: 1, heading: '0086 moved to In progress', message: '', kind: 'ok' }],
    })
    render(<Toaster />)
    const card = screen.getByText('0086 moved to In progress').closest('[data-kind]')!
    // The heading is the only paragraph; no empty message <p> consuming the gap.
    expect(card.querySelectorAll('p')).toHaveLength(1)
  })

  it('an error toast is dismissable via the close button and via Escape (no drag)', () => {
    const handle = mockToastState({
      toasts: [{ id: 5, heading: 'Move failed', message: 'conflict', kind: 'error' }],
    })
    render(<Toaster />)
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }))
    expect(handle.dismissToast).toHaveBeenCalledWith(5)
    vi.mocked(handle.dismissToast).mockClear()
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(handle.dismissToast).toHaveBeenCalledWith(5)
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

    it('.icon and border-left bind var(--toast-accent), defaulting to --ac-accent', () => {
      // The accent is now an indirection: `--toast-accent` defaults to
      // var(--ac-accent) (so `info` is byte-identical) and is remapped per kind.
      expect(toasterCss).toMatch(/\.icon[^{]*\{[^}]*color:\s*var\(--toast-accent\)/)
      expect(toasterCss).toMatch(/\.toast[^{]*\{[^}]*border-left[^;]*var\(--toast-accent\)/)
      expect(toasterCss).toMatch(/--toast-accent:\s*var\(--ac-accent\)/)
    })

    it("data-kind='ok'/'error' remap --toast-accent to --ac-ok / --ac-err", () => {
      expect(toasterCss).toMatch(/\[data-kind='ok'\][^{]*\{[^}]*--toast-accent:\s*var\(--ac-ok\)/)
      expect(toasterCss).toMatch(/\[data-kind='error'\][^{]*\{[^}]*--toast-accent:\s*var\(--ac-err\)/)
    })
  })
})
