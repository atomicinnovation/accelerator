import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, cleanup } from '@testing-library/react'
import { FontModeToggle } from './FontModeToggle'

vi.mock('../../api/use-font-mode', () => ({
  useFontModeContext: vi.fn(),
}))

import { useFontModeContext } from '../../api/use-font-mode'

function mountWith(fontMode: 'display' | 'mono', toggle = vi.fn()) {
  vi.mocked(useFontModeContext).mockReturnValue({
    fontMode,
    setFontMode: vi.fn(),
    toggleFontMode: toggle,
  })
  return render(<FontModeToggle />)
}

describe('FontModeToggle', () => {
  it('renders a button with a function-describing accessible name', () => {
    mountWith('display')
    expect(screen.getByRole('button', { name: /mono font/i })).toBeInTheDocument()
  })

  it('sets data-icon="display" in display mode', () => {
    mountWith('display')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'display')
  })

  it('sets data-icon="mono" in mono mode', () => {
    mountWith('mono')
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'mono')
  })

  it('exposes aria-pressed reflecting whether mono is active', () => {
    mountWith('display')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false')
    cleanup()
    mountWith('mono')
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true')
  })

  it('calls toggleFontMode on click', () => {
    const toggle = vi.fn()
    mountWith('display', toggle)
    fireEvent.click(screen.getByRole('button'))
    expect(toggle).toHaveBeenCalledTimes(1)
  })
})
