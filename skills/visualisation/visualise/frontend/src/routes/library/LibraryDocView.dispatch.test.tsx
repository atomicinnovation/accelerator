import { describe, it, expect, vi } from 'vitest'
import { render, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import type { IndexEntry } from '../../api/types'
import { MemoryRouter } from '../../test/router-helpers'

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

const baseEntry: IndexEntry = {
  type: 'plan-reviews',
  path: '/p/meta/reviews/plans/2026-01-01-foo-review-1.md',
  relPath: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
  slug: '2026-01-01-foo-review-1',
  workItemId: null,
  title: 'Foo review',
  frontmatter: {},
  frontmatterState: 'parsed',
  workItemRefs: [],
  mtimeMs: 1_700_000_000_000,
  size: 100,
  etag: 'sha256-a',
  bodyPreview: '',
}

function mockFetches(entry: IndexEntry) {
  vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
  vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
    content: 'Body.',
    etag: '"sha256-a"',
  })
  vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
    inferredCluster: [],
    declaredOutbound: [],
    declaredInbound: [],
  })
}

describe('LibraryDocView chip dispatch (surface ACs for 0081)', () => {
  it('plan-review document: verdict chip is coloured per plan-review vocabulary', async () => {
    mockFetches({
      ...baseEntry,
      type: 'plan-reviews',
      frontmatter: { verdict: 'APPROVE' },
    })
    const { container } = render(
      <LibraryDocView type="plan-reviews" fileSlug="2026-01-01-foo-review-1" />,
      { wrapper: Wrapper },
    )
    await waitFor(() => {
      const chip = container.querySelector('[data-testid="verdict-badge"]')
      expect(chip).not.toBeNull()
      expect(chip?.getAttribute('data-variant')).toBe('green')
    })
  })

  it('work-item-review document: verdict chip is coloured per plan-review vocabulary', async () => {
    mockFetches({
      ...baseEntry,
      type: 'work-item-reviews',
      relPath: 'meta/reviews/work-items/0042-review-1.md',
      slug: '0042-review-1',
      frontmatter: { verdict: 'REVISE' },
    })
    const { container } = render(
      <LibraryDocView type="work-item-reviews" fileSlug="0042-review-1" />,
      { wrapper: Wrapper },
    )
    await waitFor(() => {
      const chip = container.querySelector('[data-testid="verdict-badge"]')
      expect(chip).not.toBeNull()
      expect(chip?.getAttribute('data-variant')).toBe('amber')
    })
  })

  it('validation document: result chip is coloured per validation vocabulary', async () => {
    mockFetches({
      ...baseEntry,
      type: 'validations',
      relPath: 'meta/validations/2026-01-01-foo-validation-1.md',
      slug: '2026-01-01-foo-validation-1',
      frontmatter: { result: 'pass' },
    })
    const { container } = render(
      <LibraryDocView type="validations" fileSlug="2026-01-01-foo-validation-1" />,
      { wrapper: Wrapper },
    )
    await waitFor(() => {
      const chip = container.querySelector('[data-testid="result-badge"]')
      expect(chip).not.toBeNull()
      expect(chip?.getAttribute('data-variant')).toBe('green')
    })
  })
})
