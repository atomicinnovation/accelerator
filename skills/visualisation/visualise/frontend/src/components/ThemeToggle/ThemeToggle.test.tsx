import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, cleanup } from '@testing-library/react'
import { ThemeToggle } from './ThemeToggle'

vi.mock('../../api/use-theme', () => ({
  useThemeContext: vi.fn(),
}))

import { useThemeContext } from '../../api/use-theme'

function mountWith(theme: 'light' | 'dark', toggle: () => void = vi.fn()) {
  vi.mocked(useThemeContext).mockReturnValue({
    theme,
    setTheme: vi.fn(),
    toggleTheme: toggle,
  })
  return render(<ThemeToggle />)
}

describe('ThemeToggle', () => {
  it('renders a button with a function-describing accessible name', () => {
    mountWith('light')
    expect(screen.getByRole('button', { name: /dark theme/i })).toBeInTheDocument()
  })

  it('renders the current-state glyph in light mode', () => {
    mountWith('light')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'sun')
    expect(screen.getByRole('button').textContent).toContain('☀︎')
  })

  it('renders the current-state glyph in dark mode', () => {
    mountWith('dark')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'moon')
    expect(screen.getByRole('button').textContent).toContain('☽︎')
  })

  it('exposes aria-pressed reflecting whether dark is active', () => {
    mountWith('light')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false')
    cleanup()
    mountWith('dark')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true')
  })

  it('calls toggleTheme on click', () => {
    const toggle = vi.fn()
    mountWith('light', toggle)
    fireEvent.click(screen.getByRole('button'))
    expect(toggle).toHaveBeenCalledTimes(1)
  })
})
