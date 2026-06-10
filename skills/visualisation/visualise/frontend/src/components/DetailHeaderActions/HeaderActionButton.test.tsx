import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { HeaderActionButton } from './HeaderActionButton'

const icon = <svg data-testid="glyph" />

describe('HeaderActionButton', () => {
  it('renders a button with a visible label as its accessible name', () => {
    render(<HeaderActionButton icon={icon} label="Copy path" onClick={vi.fn()} />)
    const btn = screen.getByRole('button', { name: 'Copy path' })
    expect(btn).toHaveTextContent('Copy path')
  })

  it('marks the icon decorative (aria-hidden)', () => {
    const { container } = render(
      <HeaderActionButton icon={icon} label="Copy path" onClick={vi.fn()} />,
    )
    const iconWrap = container.querySelector('[aria-hidden="true"]')
    expect(iconWrap).not.toBeNull()
    expect(iconWrap!.querySelector('[data-testid="glyph"]')).not.toBeNull()
  })

  it('invokes onClick when the button variant is clicked', () => {
    const onClick = vi.fn()
    render(<HeaderActionButton icon={icon} label="Copy path" onClick={onClick} />)
    fireEvent.click(screen.getByRole('button', { name: 'Copy path' }))
    expect(onClick).toHaveBeenCalledTimes(1)
  })

  it('renders the anchor variant as <a href> with a default rel', () => {
    render(
      <HeaderActionButton as="a" href="vscode://file/a.md" icon={icon} label="Open in editor" />,
    )
    const link = screen.getByRole('link', { name: 'Open in editor' })
    expect(link).toHaveAttribute('href', 'vscode://file/a.md')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })

  it('honours a custom rel on the anchor variant', () => {
    render(
      <HeaderActionButton as="a" href="https://x.test" icon={icon} label="Y" rel="noopener" />,
    )
    expect(screen.getByRole('link', { name: 'Y' })).toHaveAttribute('rel', 'noopener')
  })

  it('disabled variant uses aria-disabled (not native disabled), stays focusable, keeps title, fires no onClick, wires aria-describedby', () => {
    const onClick = vi.fn()
    render(
      <HeaderActionButton
        icon={icon}
        label="Open in editor"
        disabled
        title="Set visualiser.editor to enable"
        ariaDescribedBy="hint-1"
        onClick={onClick}
      />,
    )
    const btn = screen.getByRole('button', { name: 'Open in editor' })
    expect(btn).toHaveAttribute('aria-disabled', 'true')
    expect(btn).not.toBeDisabled()
    expect(btn).toHaveAttribute('title', 'Set visualiser.editor to enable')
    expect(btn).toHaveAttribute('aria-describedby', 'hint-1')
    btn.focus()
    expect(btn).toHaveFocus()
    fireEvent.click(btn)
    expect(onClick).not.toHaveBeenCalled()
  })

  it('CSS module gives aria-disabled an inactive affordance and a forced-colors border', async () => {
    const css = await import('./HeaderActionButton.module.css?raw')
    expect(css.default).toMatch(
      /\.btn\[aria-disabled='true'\]\s*\{[^}]*cursor\s*:\s*default[^}]*\}/s,
    )
    expect(css.default).toMatch(
      /\.btn\[aria-disabled='true'\]\s*\{[^}]*color\s*:\s*var\(--ac-[^)]+\)[^}]*\}/s,
    )
    expect(css.default).toMatch(
      /@media\s*\(\s*forced-colors:\s*active\s*\)[^}]*\{[^}]*border\s*:\s*1px solid\s+ButtonText/s,
    )
  })
})
