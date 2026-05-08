import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import originPillCss from './OriginPill.module.css?raw'
import { OriginPill } from './OriginPill'

vi.mock('../../api/use-origin', () => ({
  useOrigin: vi.fn(),
}))

import { useOrigin } from '../../api/use-origin'

describe('OriginPill', () => {
  it('renders the host string from useOrigin (127.0.0.1:5173)', () => {
    vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
    render(<OriginPill />)
    expect(screen.getByText('127.0.0.1:5173')).toBeInTheDocument()
  })

  it('renders the host string from useOrigin (localhost:3000)', () => {
    vi.mocked(useOrigin).mockReturnValue('localhost:3000')
    render(<OriginPill />)
    expect(screen.getByText('localhost:3000')).toBeInTheDocument()
  })

  describe('CSS source assertions', () => {
    it('.pulseDot has animation referencing ac-pulse', () => {
      expect(originPillCss).toMatch(/\.pulseDot\s*\{[^}]*animation:\s*ac-pulse/)
    })

    it('has @media (prefers-reduced-motion: reduce) block disabling animation', () => {
      expect(originPillCss).toMatch(
        /@media\s*\(prefers-reduced-motion:\s*reduce\)[^}]*\{[^}]*animation:\s*none/s,
      )
    })
  })
})
