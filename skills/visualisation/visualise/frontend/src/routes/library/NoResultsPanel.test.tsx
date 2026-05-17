import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { NoResultsPanel } from './NoResultsPanel'
import type { LibraryFacet } from '../../api/types'

const FACETS: LibraryFacet[] = [
  {
    id: 'status',
    label: 'Status',
    options: [
      { id: 'open', label: 'Open', count: 0 },
      { id: 'blocked', label: 'Blocked', count: 0 },
    ],
  },
  {
    id: 'clusterSlug',
    label: 'Cluster',
    options: [{ id: 'foo', label: 'foo', count: 0 }],
  },
]

describe('NoResultsPanel', () => {
  it('renders the headline', () => {
    render(<NoResultsPanel selection={{}} facets={FACETS} onClear={() => {}} />)
    expect(
      screen.getByRole('heading', { name: /no results match your filter/i }),
    ).toBeInTheDocument()
  })

  it('summarises the active selection with human labels', () => {
    render(
      <NoResultsPanel
        selection={{ status: ['open', 'blocked'], clusterSlug: ['foo'] }}
        facets={FACETS}
        onClear={() => {}}
      />,
    )
    expect(
      screen.getByText(/active filters: status: open, blocked · cluster: foo/i),
    ).toBeInTheDocument()
  })

  it('Clear filters button fires the onClear callback', async () => {
    const user = userEvent.setup()
    const onClear = vi.fn()
    render(
      <NoResultsPanel
        selection={{ status: ['open'] }}
        facets={FACETS}
        onClear={onClear}
      />,
    )
    await user.click(screen.getByRole('button', { name: /clear filters/i }))
    expect(onClear).toHaveBeenCalled()
  })
})
