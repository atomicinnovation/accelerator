import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { TopbarIconButton } from './TopbarIconButton'

describe('TopbarIconButton', () => {
  it('exposes the accessible name from ariaLabel', () => {
    render(
      <TopbarIconButton ariaLabel="Test mode" ariaPressed={false} dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button', { name: 'Test mode' })).toBeInTheDocument()
  })

  it('reflects ariaPressed state', () => {
    const { rerender } = render(
      <TopbarIconButton ariaLabel="X" ariaPressed={false} dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false')
    rerender(
      <TopbarIconButton ariaLabel="X" ariaPressed={true} dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true')
  })

  it('writes data-icon for CSS/test targeting', () => {
    render(
      <TopbarIconButton ariaLabel="X" ariaPressed={false} dataIcon="custom-icon" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).toHaveAttribute('data-icon', 'custom-icon')
  })

  it('invokes onClick when clicked', () => {
    const onClick = vi.fn()
    render(
      <TopbarIconButton ariaLabel="X" ariaPressed={false} dataIcon="x" onClick={onClick}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    fireEvent.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledTimes(1)
  })

  it('CSS module includes a @media (forced-colors: active) block', async () => {
    const css = await import('./TopbarIconButton.module.css?raw')
    expect(css.default).toMatch(
      /@media\s*\(\s*forced-colors:\s*active\s*\)[^}]*\{[^}]*border\s*:\s*1px solid\s+ButtonText/s,
    )
  })

  it('omits aria-pressed on the button variant when not provided', () => {
    render(
      <TopbarIconButton ariaLabel="X" dataIcon="x" onClick={vi.fn()}>
        <span aria-hidden="true">X</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('button')).not.toHaveAttribute('aria-pressed')
  })

  it('renders the anchor variant as <a href> with default rel and no aria-pressed', () => {
    render(
      <TopbarIconButton as="a" href="vscode://file/a.md" ariaLabel="Open in editor" dataIcon="edit">
        <span aria-hidden="true">E</span>
      </TopbarIconButton>,
    )
    const link = screen.getByRole('link', { name: 'Open in editor' })
    expect(link).toHaveAttribute('href', 'vscode://file/a.md')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(link).not.toHaveAttribute('aria-pressed')
  })

  it('honours a custom rel on the anchor variant', () => {
    render(
      <TopbarIconButton as="a" href="https://x.test" ariaLabel="Y" dataIcon="edit" rel="noopener">
        <span aria-hidden="true">Y</span>
      </TopbarIconButton>,
    )
    expect(screen.getByRole('link')).toHaveAttribute('rel', 'noopener')
  })

  it('disabled variant uses aria-disabled (not native disabled), stays focusable, keeps title, fires no onClick', () => {
    const onClick = vi.fn()
    render(
      <TopbarIconButton
        ariaLabel="Open in editor"
        dataIcon="edit"
        disabled
        title="Set visualiser.editor to enable"
        ariaDescribedBy="hint-1"
        onClick={onClick}
      >
        <span aria-hidden="true">E</span>
      </TopbarIconButton>,
    )
    const btn = screen.getByRole('button', { name: 'Open in editor' })
    expect(btn).toHaveAttribute('aria-disabled', 'true')
    // NOT the native disabled attribute — must stay in the tab order.
    expect(btn).not.toBeDisabled()
    expect(btn).toHaveAttribute('title', 'Set visualiser.editor to enable')
    expect(btn).toHaveAttribute('aria-describedby', 'hint-1')
    btn.focus()
    expect(btn).toHaveFocus()
    fireEvent.click(btn)
    expect(onClick).not.toHaveBeenCalled()
  })

  it('CSS module gives aria-disabled an inactive affordance (default cursor, muted token, no hover/active change)', async () => {
    const css = await import('./TopbarIconButton.module.css?raw')
    // Base disabled rule: default cursor + a muted --ac-* token colour.
    expect(css.default).toMatch(
      /\.toggle\[aria-disabled="true"\]\s*\{[^}]*cursor\s*:\s*default[^}]*\}/s,
    )
    expect(css.default).toMatch(
      /\.toggle\[aria-disabled="true"\]\s*\{[^}]*color\s*:\s*var\(--ac-[^)]+\)[^}]*\}/s,
    )
    // Hover/active feedback suppressed for the disabled state.
    expect(css.default).toMatch(
      /\.toggle\[aria-disabled="true"\]:hover[^{]*,[^{]*\.toggle\[aria-disabled="true"\]:active\s*\{[^}]*background\s*:\s*transparent/s,
    )
  })
})
