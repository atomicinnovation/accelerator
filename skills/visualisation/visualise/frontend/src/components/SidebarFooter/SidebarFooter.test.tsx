import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SidebarFooter } from './SidebarFooter'

vi.mock('../../api/use-server-info', () => ({
  useServerInfo: vi.fn(),
}))

vi.mock('../../api/use-doc-events', () => ({
  useDocEvents: vi.fn(),
}))

import { useServerInfo } from '../../api/use-server-info'
import { useDocEvents } from '../../api/use-doc-events'

function mockDefaults(overrides: {
  serverInfo?: { name?: string; version?: string }
  connectionState?: string
  justReconnected?: boolean
} = {}) {
  vi.mocked(useServerInfo).mockReturnValue({
    data: overrides.serverInfo,
  } as any)
  vi.mocked(useDocEvents).mockReturnValue({
    setDragInProgress: vi.fn(),
    connectionState: overrides.connectionState ?? 'open',
    justReconnected: overrides.justReconnected ?? false,
  } as any)
}

describe('SidebarFooter', () => {
  it('renders the version label when /api/info has resolved', () => {
    mockDefaults({ serverInfo: { name: 'accelerator-visualiser', version: '0.1.0' } })
    render(<SidebarFooter />)
    expect(screen.getByText('Visualiser v0.1.0')).toBeInTheDocument()
  })

  it('renders nothing in the footer when data is not yet loaded', () => {
    mockDefaults({ connectionState: 'connecting' })
    render(<SidebarFooter />)
    expect(screen.queryByText(/Visualiser v/)).toBeNull()
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('renders nothing when version field is missing', () => {
    mockDefaults({ serverInfo: { name: 'accelerator-visualiser' } })
    render(<SidebarFooter />)
    expect(screen.queryByText(/Visualiser v/)).toBeNull()
  })

  it('shows "Reconnecting…" when connectionState is reconnecting', () => {
    mockDefaults({ connectionState: 'reconnecting' })
    render(<SidebarFooter />)
    expect(screen.getByRole('status')).toHaveTextContent('Reconnecting…')
  })

  it('shows "Reconnected — refreshing" when justReconnected is true and open', () => {
    mockDefaults({ connectionState: 'open', justReconnected: true })
    render(<SidebarFooter />)
    expect(screen.getByRole('status')).toHaveTextContent('Reconnected — refreshing')
  })

  it('shows only "Reconnecting…" when both reconnecting and justReconnected', () => {
    mockDefaults({ connectionState: 'reconnecting', justReconnected: true })
    render(<SidebarFooter />)
    const statuses = screen.getAllByRole('status')
    expect(statuses).toHaveLength(1)
    expect(statuses[0]).toHaveTextContent('Reconnecting…')
  })
})
