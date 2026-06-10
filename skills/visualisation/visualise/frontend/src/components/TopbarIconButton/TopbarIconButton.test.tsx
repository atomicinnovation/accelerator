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
})
