import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { PipelineMini } from './PipelineMini'
import { makeCompleteness } from '../../api/test-fixtures'
import { WORKFLOW_PIPELINE_STEPS } from '../../api/types'

describe('PipelineMini', () => {
  it('renders eight <li> dots inside an <ol> root in canonical order', () => {
    const { container } = render(
      <PipelineMini completeness={makeCompleteness()} />,
    )
    const root = container.querySelector('ol.ac-stagedots')!
    expect(root).toBeInTheDocument()
    const dots = root.querySelectorAll('li[data-stage]')
    expect(dots).toHaveLength(8)
    const order = Array.from(dots).map(d => d.getAttribute('data-stage'))
    expect(order).toEqual(WORKFLOW_PIPELINE_STEPS.map(s => s.docType))
  })

  it('marks present stages active and absent stages inactive', () => {
    const { container } = render(
      <PipelineMini
        completeness={makeCompleteness({
          hasWorkItem: true,
          hasPlan: true,
          present: ['work-items', 'plans'],
        })}
      />,
    )
    expect(
      container
        .querySelector('[data-stage="work-items"]')!
        .getAttribute('data-active'),
    ).toBe('true')
    expect(
      container
        .querySelector('[data-stage="plans"]')!
        .getAttribute('data-active'),
    ).toBe('true')
    expect(
      container
        .querySelector('[data-stage="research"]')!
        .getAttribute('data-active'),
    ).toBe('false')
  })

  it('renders no label text or anchors', () => {
    const { container } = render(
      <PipelineMini
        completeness={makeCompleteness({ hasWorkItem: true, present: ['work-items'] })}
      />,
    )
    const dots = container.querySelectorAll('li[data-stage]')
    for (const d of dots) {
      expect(d.textContent).toBe('')
    }
    expect(container.querySelector('a')).toBeNull()
  })

  it('uses --ac-stage-* token (not hard-coded hsl()) for active dot colour', () => {
    const { container } = render(
      <PipelineMini
        completeness={makeCompleteness({
          hasWorkItem: true,
          present: ['work-items'],
        })}
      />,
    )
    const dot = container.querySelector(
      '[data-stage="work-items"]',
    ) as HTMLElement
    expect(dot.style.cssText).toContain('var(--ac-stage-')
    expect(dot.style.cssText).not.toMatch(/hsl\(/)
  })
})
