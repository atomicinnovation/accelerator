import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import topbarCss from './Topbar.module.css?raw'
import { Topbar } from './Topbar'

vi.mock('../../api/use-doc-events', () => ({
  useDocEventsContext: vi.fn(),
}))

vi.mock('../../api/use-origin', () => ({
  useOrigin: vi.fn(),
}))

vi.mock('../Breadcrumbs/Breadcrumbs', () => ({
  Breadcrumbs: () => <nav aria-label="Breadcrumb"><ol /></nav>,
}))

vi.mock('../../api/use-theme', () => ({
  useThemeContext: vi.fn(),
}))

vi.mock('../../api/use-font-mode', () => ({
  useFontModeContext: vi.fn(),
}))

import { useDocEventsContext } from '../../api/use-doc-events'
import { useOrigin } from '../../api/use-origin'
import { useThemeContext } from '../../api/use-theme'
import { useFontModeContext } from '../../api/use-font-mode'

function mountTopbar(connectionState = 'open', justReconnected = false) {
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState,
    justReconnected,
    setDragInProgress: vi.fn(),
  } as any)
  vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
  vi.mocked(useThemeContext).mockReturnValue({
    theme: 'light',
    setTheme: vi.fn(),
    toggleTheme: vi.fn(),
  })
  vi.mocked(useFontModeContext).mockReturnValue({
    fontMode: 'display',
    setFontMode: vi.fn(),
    toggleFontMode: vi.fn(),
  })
  return render(<Topbar />)
}

describe('Topbar', () => {
  it('renders brand text "Accelerator"', () => {
    mountTopbar()
    expect(screen.getByText('Accelerator')).toBeInTheDocument()
  })

  it('renders brand text "VISUALISER"', () => {
    mountTopbar()
    expect(screen.getByText('VISUALISER')).toBeInTheDocument()
  })

  it('renders a <nav aria-label="Breadcrumb">', () => {
    mountTopbar()
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toBeInTheDocument()
  })

  it('renders origin pill with mocked host', () => {
    mountTopbar()
    expect(screen.getByText('127.0.0.1:5173')).toBeInTheDocument()
  })

  it('renders SSE indicator with data-state="open"', () => {
    mountTopbar('open')
    expect(document.querySelector('[data-state="open"]')).not.toBeNull()
  })

  it('renders a theme toggle inside the data-slot="theme-toggle" div', () => {
    mountTopbar()
    const slot = document.querySelector('[data-slot="theme-toggle"]')
    expect(slot).not.toBeNull()
    expect(slot?.querySelector('button')).not.toBeNull()
  })

  it('renders a font-mode toggle inside the data-slot="font-mode-toggle" div', () => {
    mountTopbar()
    const slot = document.querySelector('[data-slot="font-mode-toggle"]')
    expect(slot).not.toBeNull()
    expect(slot?.querySelector('button')).not.toBeNull()
  })

  it('the theme toggle button has an accessible name', () => {
    mountTopbar()
    expect(screen.getByRole('button', { name: /dark theme/i })).toBeInTheDocument()
  })

  it('the font-mode toggle button has an accessible name', () => {
    mountTopbar()
    expect(screen.getByRole('button', { name: /mono font/i })).toBeInTheDocument()
  })

  it('clicking the theme toggle invokes toggleTheme from the context', () => {
    const toggleTheme = vi.fn()
    vi.mocked(useThemeContext).mockReturnValue({
      theme: 'light',
      setTheme: vi.fn(),
      toggleTheme,
    })
    vi.mocked(useFontModeContext).mockReturnValue({
      fontMode: 'display',
      setFontMode: vi.fn(),
      toggleFontMode: vi.fn(),
    })
    vi.mocked(useDocEventsContext).mockReturnValue({
      connectionState: 'open',
      justReconnected: false,
      setDragInProgress: vi.fn(),
    } as any)
    vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
    render(<Topbar />)
    fireEvent.click(screen.getByRole('button', { name: /dark theme/i }))
    expect(toggleTheme).toHaveBeenCalledTimes(1)
  })

  it('clicking the font-mode toggle invokes toggleFontMode from the context', () => {
    const toggleFontMode = vi.fn()
    vi.mocked(useThemeContext).mockReturnValue({
      theme: 'light',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    })
    vi.mocked(useFontModeContext).mockReturnValue({
      fontMode: 'display',
      setFontMode: vi.fn(),
      toggleFontMode,
    })
    vi.mocked(useDocEventsContext).mockReturnValue({
      connectionState: 'open',
      justReconnected: false,
      setDragInProgress: vi.fn(),
    } as any)
    vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
    render(<Topbar />)
    fireEvent.click(screen.getByRole('button', { name: /mono font/i }))
    expect(toggleFontMode).toHaveBeenCalledTimes(1)
  })

  it('does NOT render "Reconnected — refreshing" text even when justReconnected is true', () => {
    mountTopbar('open', true)
    expect(screen.queryByText(/Reconnected — refreshing/)).toBeNull()
  })

  describe('CSS source assertions', () => {
    it('.slot:empty collapses the slot', () => {
      expect(topbarCss).toMatch(/\.slot:empty\s*\{[^}]*width:\s*0/)
    })
  })
})
