import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from '../../test/router-helpers'
import { Sidebar } from './Sidebar'
import type { DocType } from '../../api/types'

vi.mock('../../api/use-server-info', () => ({
  useServerInfo: vi.fn(() => ({ data: undefined })),
}))

const mockDocTypes: DocType[] = [
  { key: 'decisions', label: 'Decisions', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false },
  { key: 'tickets', label: 'Tickets', dirPath: '/p', inLifecycle: true, inKanban: true, virtual: false },
  { key: 'plans', label: 'Plans', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false },
  { key: 'templates', label: 'Templates', dirPath: null, inLifecycle: false, inKanban: false, virtual: true },
]

describe('Sidebar', () => {
  // RouterProvider initialises via React.startTransition, so assertions must
  // be async to wait for the router to settle before querying the DOM.
  it('renders all doc type labels', async () => {
    render(<MemoryRouter><Sidebar docTypes={mockDocTypes} /></MemoryRouter>)
    expect(await screen.findByText('Decisions')).toBeInTheDocument()
    expect(screen.getByText('Tickets')).toBeInTheDocument()
    expect(screen.getByText('Plans')).toBeInTheDocument()
  })

  it('renders Templates under a "Meta" heading', async () => {
    render(<MemoryRouter><Sidebar docTypes={mockDocTypes} /></MemoryRouter>)
    expect(await screen.findByText('Meta')).toBeInTheDocument()
    expect(screen.getByText('Templates')).toBeInTheDocument()
  })

  it('renders Lifecycle and Kanban nav items', async () => {
    render(<MemoryRouter><Sidebar docTypes={mockDocTypes} /></MemoryRouter>)
    expect(await screen.findByText('Lifecycle')).toBeInTheDocument()
    expect(screen.getByText('Kanban')).toBeInTheDocument()
  })
})
