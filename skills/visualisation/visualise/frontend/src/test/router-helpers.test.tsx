import { describe, it, expect } from 'vitest'
import { screen } from '@testing-library/react'
import { Link } from '@tanstack/react-router'
import { renderWithRouterAt } from './router-helpers'

describe('renderWithRouterAt', () => {
  it('renders the supplied children at /', async () => {
    renderWithRouterAt(<p>Hello kanban</p>)
    expect(await screen.findByText('Hello kanban')).toBeInTheDocument()
  })

  it('resolves <Link> targets at the library doc route', async () => {
    renderWithRouterAt(
      <Link to="/library/$type/$fileSlug" params={{ type: 'tickets', fileSlug: '0001-x' }}>
        ticket link
      </Link>,
    )
    const link = await screen.findByRole('link', { name: /ticket link/i })
    expect(link.getAttribute('href')).toBe('/library/tickets/0001-x')
  })
})
