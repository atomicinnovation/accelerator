import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PipelineDots } from './PipelineDots'
import type { Completeness } from '../../api/types'

const empty: Completeness = {
  hasTicket: false, hasResearch: false, hasPlan: false,
  hasPlanReview: false, hasValidation: false, hasPr: false,
  hasPrReview: false, hasDecision: false, hasNotes: false,
}

describe('PipelineDots', () => {
  it('renders all 8 pipeline stage dots', () => {
    render(<PipelineDots completeness={empty} />)
    expect(screen.getAllByRole('listitem')).toHaveLength(8)
  })

  it('marks present stages as filled and absent as unfilled via data-present', () => {
    const c: Completeness = { ...empty, hasTicket: true, hasPlan: true }
    const { container } = render(<PipelineDots completeness={c} />)
    const ticket = container.querySelector('[data-stage="hasTicket"]')!
    const plan = container.querySelector('[data-stage="hasPlan"]')!
    const research = container.querySelector('[data-stage="hasResearch"]')!
    expect(ticket.getAttribute('data-present')).toBe('true')
    expect(plan.getAttribute('data-present')).toBe('true')
    expect(research.getAttribute('data-present')).toBe('false')
  })

  it('exposes each stage label via accessible title', () => {
    const c: Completeness = { ...empty, hasPlan: true }
    render(<PipelineDots completeness={c} />)
    expect(screen.getByTitle(/^Plan$/)).toBeInTheDocument()
    expect(screen.getByTitle(/^Plan review$/)).toBeInTheDocument()
  })

  it('exposes presence state via aria-label per dot', () => {
    const c: Completeness = { ...empty, hasPlan: true }
    render(<PipelineDots completeness={c} />)
    expect(screen.getByLabelText('Plan: present')).toBeInTheDocument()
    expect(screen.getByLabelText('Ticket: missing')).toBeInTheDocument()
  })
})
