import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/react'
import { StatusBadge } from './StatusBadge'

describe('StatusBadge', () => {
  describe('observable hook', () => {
    it('renders with data-testid="status-badge"', () => {
      const { container } = render(
        <StatusBadge value="Accepted" />,
      )
      expect(container.querySelector('[data-testid="status-badge"]')).not.toBeNull()
    })
  })

  describe('aria-label (inherited via composition)', () => {
    it('renders aria-label of "${key}: ${value}"', () => {
      const { container } = render(
        <StatusBadge value="Accepted" />,
      )
      expect(container.querySelector('[aria-label="status: Accepted"]')).not.toBeNull()
    })
  })

  describe('status vocabulary — full live mapping preserved', () => {
    it.each([
      ['Accepted', 'green'], ['Done', 'green'], ['complete', 'green'],
      ['approved', 'green'], ['implemented', 'green'], ['final', 'green'], ['shipped', 'green'],
      ['In progress', 'indigo'], ['Proposed', 'indigo'], ['live', 'indigo'],
      ['active', 'indigo'], ['reviewed', 'indigo'], ['ready', 'indigo'],
      ['Approve w/ changes', 'amber'], ['review', 'amber'], ['revised', 'amber'],
      ['blocked', 'red'], ['rejected', 'red'], ['deprecated', 'red'],
      ['superseded', 'red'], ['abandoned', 'red'],
    ])('status %s → %s', (value, expected) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('normalisation reach (separator insensitivity)', () => {
    it.each([
      ['IN_PROGRESS', 'indigo'], ['in-progress', 'indigo'], ['in_progress', 'indigo'],
    ])('status %s → %s', (value, expected) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })

  describe('neutral fallback', () => {
    it.each([
      'Todo', 'absent', 'SomeUnknownValue', '2026-05-21', '',
    ])('status %s → neutral', (value) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it.each([null, undefined, 42, true, ['a'], { x: 1 }] as const)(
      'non-string value → neutral', (value) => {
        const { container } = render(<StatusBadge value={value} />)
        expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
      },
    )
  })

  describe('vocabulary isolation', () => {
    it.each([
      ['approve', 'neutral'], ['pass', 'neutral'], ['fail', 'neutral'],
      ['REVISE', 'neutral'], ['REQUEST_CHANGES', 'neutral'], ['partial', 'neutral'],
    ])('verdict-shaped value %s under status → %s (cross-leakage prevented)', (value, expected) => {
      const { container } = render(<StatusBadge value={value} />)
      expect(container.querySelector(`[data-variant="${expected}"]`)).not.toBeNull()
    })
  })
})
