import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EmptyState } from './EmptyState'
import { EMPTY_DESCRIPTIONS, EMPTY_TYPE_PLURALS } from './empty-descriptions'
import { DOC_TYPE_KEYS } from '../../api/types'

describe('EmptyState', () => {
  it('renders the path heading from dirPath', () => {
    render(<EmptyState docType="pr-descriptions" dirPath="meta/prs/" />)
    expect(screen.getByText('meta/prs/')).toBeInTheDocument()
  })

  it('renders the no-{plural}-yet headline', () => {
    render(<EmptyState docType="decisions" dirPath="meta/decisions" />)
    expect(
      screen.getByRole('heading', { name: /no decisions yet/i }),
    ).toBeInTheDocument()
  })

  it('renders the per-doc-type description', () => {
    render(<EmptyState docType="plans" dirPath="meta/plans" />)
    expect(screen.getByText(EMPTY_DESCRIPTIONS['plans'])).toBeInTheDocument()
  })

  it('renders the indexer-aware footer with the dirPath inline', () => {
    render(<EmptyState docType="plans" dirPath="meta/plans" />)
    expect(
      screen.getByText(/new files added to meta\/plans are picked up live/i),
    ).toBeInTheDocument()
  })
})

describe('empty-descriptions table completeness', () => {
  it('declares a non-empty description and plural for every DocTypeKey', () => {
    for (const key of DOC_TYPE_KEYS) {
      expect(EMPTY_DESCRIPTIONS[key], `${key} description`).toBeTruthy()
      expect(EMPTY_TYPE_PLURALS[key], `${key} plural`).toBeTruthy()
    }
  })
})
