import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { RelatedArtifacts } from './RelatedArtifacts'
import { makeIndexEntry } from '../../api/test-fixtures'
import type { RelatedArtifactsResponse } from '../../api/types'

const empty: RelatedArtifactsResponse = {
  inferredCluster: [],
  declaredOutbound: [],
  declaredInbound: [],
}

const examplePlan = makeIndexEntry({
  type: 'plans',
  relPath: 'meta/plans/2026-04-18-foo.md',
  title: 'Foo Plan',
})
const exampleReview = makeIndexEntry({
  type: 'plan-reviews',
  relPath: 'meta/reviews/plans/2026-04-18-foo-review-1.md',
  title: 'Foo review',
})
const exampleAdr = makeIndexEntry({
  type: 'decisions',
  relPath: 'meta/decisions/ADR-0001-example.md',
  title: 'Example decision',
})

describe('RelatedArtifacts', () => {
  // ── Step 6.1 ────────────────────────────────────────────────────────
  it('shows all-empty message when all three arrays are empty', () => {
    render(<RelatedArtifacts related={empty} />)
    expect(
      screen.getByText('This document has no declared or inferred relations.'),
    ).toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 4 })).toBeNull()
  })

  // ── Step 6.2 ────────────────────────────────────────────────────────
  it('renders Targets group as h4 for declaredOutbound', () => {
    render(
      <RelatedArtifacts
        related={{ ...empty, declaredOutbound: [examplePlan] }}
      />,
    )
    expect(screen.getByRole('heading', { level: 4, name: 'Targets' })).toBeInTheDocument()
    const link = screen.getByRole('link', { name: 'Foo Plan' })
    expect(link.getAttribute('href')).toBe('/library/plans/2026-04-18-foo')
  })

  // ── Step 6.3 ────────────────────────────────────────────────────────
  it('renders Inbound reviews group as h4 for declaredInbound', () => {
    render(
      <RelatedArtifacts
        related={{ ...empty, declaredInbound: [exampleReview] }}
      />,
    )
    expect(
      screen.getByRole('heading', { level: 4, name: 'Inbound reviews' }),
    ).toBeInTheDocument()
    const link = screen.getByRole('link', { name: 'Foo review' })
    expect(link.getAttribute('href')).toBe(
      '/library/plan-reviews/2026-04-18-foo-review-1',
    )
  })

  // ── Step 6.4 ────────────────────────────────────────────────────────
  it('renders Same lifecycle group as h4 for inferredCluster', () => {
    render(
      <RelatedArtifacts
        related={{ ...empty, inferredCluster: [exampleAdr] }}
      />,
    )
    expect(
      screen.getByRole('heading', { level: 4, name: 'Same lifecycle' }),
    ).toBeInTheDocument()
    const link = screen.getByRole('link', { name: 'Example decision' })
    expect(link.getAttribute('href')).toBe('/library/decisions/ADR-0001-example')
  })

  // ── Step 6.5 ────────────────────────────────────────────────────────
  it('declared and inferred groups carry distinct element-named CSS modifier classes', () => {
    const { container } = render(
      <RelatedArtifacts
        related={{
          inferredCluster: [exampleAdr],
          declaredOutbound: [examplePlan],
          declaredInbound: [],
        }}
      />,
    )
    // Modifier classes are CSS-module-hashed, so we match on the
    // unhashed prefix that vite-style classnames preserve.
    const declaredGroup = container.querySelector('[class*="groupDeclared"]')
    const inferredGroup = container.querySelector('[class*="groupInferred"]')
    expect(declaredGroup).not.toBeNull()
    expect(inferredGroup).not.toBeNull()
  })

  // ── Step 6.5b ───────────────────────────────────────────────────────
  it('legend explains declared vs inferred whenever any group renders', () => {
    render(
      <RelatedArtifacts
        related={{ ...empty, declaredOutbound: [examplePlan] }}
      />,
    )
    expect(screen.getByText('Declared')).toBeInTheDocument()
    expect(screen.getByText('Inferred')).toBeInTheDocument()
  })

  // ── Step 6.5c ───────────────────────────────────────────────────────
  it('shows Updating hint only when showUpdatingHint is true', () => {
    const populated = { ...empty, declaredOutbound: [examplePlan] }
    const { rerender } = render(
      <RelatedArtifacts related={populated} showUpdatingHint />,
    )
    const hint = screen.getByText('Updating…')
    expect(hint).toBeInTheDocument()
    expect(hint.getAttribute('aria-live')).toBe('polite')

    rerender(<RelatedArtifacts related={populated} showUpdatingHint={false} />)
    expect(screen.queryByText('Updating…')).toBeNull()
  })
})
