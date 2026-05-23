import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChip } from './FrontmatterChip'

describe('FrontmatterChip', () => {
  describe('default rendering', () => {
    it('renders the value text', () => {
      render(<FrontmatterChip name="status" value="Accepted" />)
      expect(screen.getByText('Accepted')).toBeInTheDocument()
    })

    it('does not render the key in visible content', () => {
      render(<FrontmatterChip name="status" value="Accepted" />)
      expect(screen.queryByText(/^status:/i)).toBeNull()
    })

    it('attaches aria-label of "${key}: ${value}"', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="Accepted" />,
      )
      expect(container.querySelector('[aria-label="status: Accepted"]')).not.toBeNull()
    })

    it('defaults to variant="neutral" regardless of key', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="Accepted" />,
      )
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it('renders with data-testid="frontmatter-chip" by default', () => {
      const { container } = render(
        <FrontmatterChip name="date" value="2026-05-22" />,
      )
      expect(container.querySelector('[data-testid="frontmatter-chip"]')).not.toBeNull()
    })
  })

  describe('variant override', () => {
    it('uses an explicitly-passed variant', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="x" variant="green" />,
      )
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })
  })

  describe('testId override', () => {
    it('renders a caller-provided data-testid value', () => {
      const { container } = render(
        <FrontmatterChip name="status" value="x" testId="custom-badge" />,
      )
      expect(container.querySelector('[data-testid="custom-badge"]')).not.toBeNull()
    })
  })

  describe('value formatting', () => {
    it('joins array values with ", " (and the join appears in aria-label)', () => {
      const { container } = render(
        <FrontmatterChip name="tags" value={['design', 'frontend']} />,
      )
      expect(screen.getByText('design, frontend')).toBeInTheDocument()
      expect(container.querySelector('[aria-label="tags: design, frontend"]')).not.toBeNull()
    })

    it('JSON-stringifies plain objects (visible text and aria-label parity)', () => {
      const { container } = render(<FrontmatterChip name="meta" value={{ x: 1 }} />)
      expect(screen.getByText('{"x":1}')).toBeInTheDocument()
      expect(container.querySelector('[aria-label="meta: {\\"x\\":1}"]')).not.toBeNull()
    })

    it('coerces booleans to strings', () => {
      render(<FrontmatterChip name="archived" value={false} />)
      expect(screen.getByText('false')).toBeInTheDocument()
    })

    it('coerces numbers to strings', () => {
      render(<FrontmatterChip name="version" value={0} />)
      expect(screen.getByText('0')).toBeInTheDocument()
    })
  })
})
