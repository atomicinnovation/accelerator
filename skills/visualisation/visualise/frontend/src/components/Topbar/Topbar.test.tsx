import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import topbarCss from './Topbar.module.css?raw'
import { Topbar } from './Topbar'

vi.mock('../../api/use-doc-events', () => ({
  useDocEventsContext: vi.fn(),
}))

vi.mock('../../api/use-origin', () => ({
  useOrigin: vi.fn(),
}))

vi.mock('../Breadcrumbs/Breadcrumbs', () => ({
  Breadcrumbs: () => <nav aria-label="Breadcrumb"><ol /></nav>,
}))

import { useDocEventsContext } from '../../api/use-doc-events'
import { useOrigin } from '../../api/use-origin'

function mountTopbar(connectionState = 'open', justReconnected = false) {
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState,
    justReconnected,
    setDragInProgress: vi.fn(),
  } as any)
  vi.mocked(useOrigin).mockReturnValue('127.0.0.1:5173')
  return render(<Topbar />)
}

describe('Topbar', () => {
  it('renders brand text "Accelerator"', () => {
    mountTopbar()
    expect(screen.getByText('Accelerator')).toBeInTheDocument()
  })

  it('renders brand text "VISUALISER"', () => {
    mountTopbar()
    expect(screen.getByText('VISUALISER')).toBeInTheDocument()
  })

  it('renders a <nav aria-label="Breadcrumb">', () => {
    mountTopbar()
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toBeInTheDocument()
  })

  it('renders origin pill with mocked host', () => {
    mountTopbar()
    expect(screen.getByText('127.0.0.1:5173')).toBeInTheDocument()
  })

  it('renders SSE indicator with data-state="open"', () => {
    mountTopbar('open')
    expect(document.querySelector('[data-state="open"]')).not.toBeNull()
  })

  it('renders an empty data-slot="theme-toggle" div', () => {
    mountTopbar()
    const slot = document.querySelector('[data-slot="theme-toggle"]')
    expect(slot).not.toBeNull()
    expect(slot?.children).toHaveLength(0)
  })

  it('renders an empty data-slot="font-mode-toggle" div', () => {
    mountTopbar()
    const slot = document.querySelector('[data-slot="font-mode-toggle"]')
    expect(slot).not.toBeNull()
    expect(slot?.children).toHaveLength(0)
  })

  it('does NOT render "Reconnected — refreshing" text even when justReconnected is true', () => {
    mountTopbar('open', true)
    expect(screen.queryByText(/Reconnected — refreshing/)).toBeNull()
  })

  describe('CSS source assertions', () => {
    it('.slot:empty collapses the slot', () => {
      expect(topbarCss).toMatch(/\.slot:empty\s*\{[^}]*width:\s*0/)
    })
  })
})
