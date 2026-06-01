import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { Pipeline } from './Pipeline'
import { makeCompleteness } from '../../api/test-fixtures'
import { WORKFLOW_PIPELINE_STEPS } from '../../api/types'

describe('Pipeline', () => {
  it('renders exactly eight stage tiles in canonical order', () => {
    const { container } = render(
      <Pipeline completeness={makeCompleteness()} />,
    )
    const tiles = container.querySelectorAll('[data-stage]')
    expect(tiles).toHaveLength(8)
    const order = Array.from(tiles).map(t => t.getAttribute('data-stage'))
    expect(order).toEqual(WORKFLOW_PIPELINE_STEPS.map(s => s.docType))
  })

  it('marks present stages active and absent stages inactive via data-active', () => {
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({
          hasWorkItem: true,
          hasPlan: true,
          present: ['work-items', 'plans'],
        })}
      />,
    )
    const workItems = container.querySelector('[data-stage="work-items"]')!
    const plans = container.querySelector('[data-stage="plans"]')!
    const research = container.querySelector('[data-stage="research"]')!
    expect(workItems).toHaveAttribute('data-active', 'true')
    expect(plans).toHaveAttribute('data-active', 'true')
    expect(research).toHaveAttribute('data-active', 'false')
  })

  it('renders the canonical label for each step', () => {
    const { container } = render(
      <Pipeline completeness={makeCompleteness()} />,
    )
    for (const step of WORKFLOW_PIPELINE_STEPS) {
      const stage = container.querySelector(`[data-stage="${step.docType}"]`)!
      expect(stage.textContent).toContain(step.label)
    }
  })

  it('contains no anchor elements (non-interactive)', () => {
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({ hasWorkItem: true, present: ['work-items'] })}
      />,
    )
    expect(container.querySelector('a')).toBeNull()
  })

  it('updates data-active when completeness prop changes between renders', () => {
    const { container, rerender } = render(
      <Pipeline completeness={makeCompleteness()} />,
    )
    expect(
      container
        .querySelector('[data-stage="work-items"]')!
        .getAttribute('data-active'),
    ).toBe('false')
    rerender(
      <Pipeline
        completeness={makeCompleteness({
          hasWorkItem: true,
          present: ['work-items'],
        })}
      />,
    )
    expect(
      container
        .querySelector('[data-stage="work-items"]')!
        .getAttribute('data-active'),
    ).toBe('true')
  })

  it('marks connector active only when both adjacent stages are present', () => {
    // Non-adjacent: work-items + plans — no connector should be active
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({
          hasWorkItem: true,
          hasPlan: true,
          present: ['work-items', 'plans'],
        })}
      />,
    )
    const connectors = container.querySelectorAll('[data-stage] > [data-active]')
    for (const c of connectors) {
      expect(c.getAttribute('data-active')).toBe('false')
    }
  })

  it('marks connector active when adjacent stages are both present', () => {
    // Adjacent: work-items + research — work-items connector is active
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({
          hasWorkItem: true,
          hasResearch: true,
          present: ['work-items', 'research'],
        })}
      />,
    )
    const workItems = container.querySelector('[data-stage="work-items"]')!
    // Connector is the inner span carrying its own data-active distinct from
    // the parent's data-active.
    const workItemsConnector = workItems.querySelector(
      ':scope > span[data-active]',
    )
    expect(workItemsConnector?.getAttribute('data-active')).toBe('true')

    const research = container.querySelector('[data-stage="research"]')!
    const researchConnector = research.querySelector(':scope > span[data-active]')
    // research → plans: research is active, plans is not → false
    expect(researchConnector?.getAttribute('data-active')).toBe('false')
  })

  it('uses --ac-stage-* token (not hard-coded hsl()) for active stage colour', () => {
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({
          hasWorkItem: true,
          present: ['work-items'],
        })}
      />,
    )
    const stage = container.querySelector(
      '[data-stage="work-items"]',
    ) as HTMLElement
    const cssText = stage.style.cssText
    expect(cssText).toContain('var(--ac-stage-')
    expect(cssText).not.toMatch(/hsl\(/)
  })

  it('renders panel variant with data-variant="panel" and 24px glyph', () => {
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({ hasWorkItem: true, present: ['work-items'] })}
        variant="panel"
      />,
    )
    const root = container.querySelector('.ac-stagechain')!
    expect(root.getAttribute('data-variant')).toBe('panel')
    const svg = root.querySelector('[data-stage="work-items"] svg')!
    expect(svg.getAttribute('width')).toBe('24')
  })

  it('renders card variant by default with 16px glyph', () => {
    const { container } = render(
      <Pipeline
        completeness={makeCompleteness({ hasWorkItem: true, present: ['work-items'] })}
      />,
    )
    const root = container.querySelector('.ac-stagechain')!
    expect(root.getAttribute('data-variant')).toBe('card')
    const svg = root.querySelector('[data-stage="work-items"] svg')!
    expect(svg.getAttribute('width')).toBe('16')
  })
})
