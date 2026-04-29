import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SidebarFooter } from './SidebarFooter'

vi.mock('../../api/use-server-info', () => ({
  useServerInfo: vi.fn(),
}))

import { useServerInfo } from '../../api/use-server-info'

describe('SidebarFooter', () => {
  it('renders the version label when /api/info has resolved', () => {
    vi.mocked(useServerInfo).mockReturnValue({
      data: { name: 'accelerator-visualiser', version: '0.1.0' },
    } as any)

    render(<SidebarFooter />)
    expect(screen.getByText('Visualiser v0.1.0')).toBeInTheDocument()
  })

  it('renders nothing in the footer when data is not yet loaded', () => {
    vi.mocked(useServerInfo).mockReturnValue({ data: undefined } as any)

    render(<SidebarFooter />)
    expect(screen.queryByText(/Visualiser v/)).toBeNull()
  })

  it('renders nothing when version field is missing', () => {
    vi.mocked(useServerInfo).mockReturnValue({
      data: { name: 'accelerator-visualiser' },
    } as any)

    render(<SidebarFooter />)
    expect(screen.queryByText(/Visualiser v/)).toBeNull()
  })
})
