import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { PageSubtitle } from './PageSubtitle'
import { Chip } from '../Chip/Chip'
import pageSubtitleCss from './PageSubtitle.module.css?raw'

describe('PageSubtitle', () => {
  it('renders the title', () => {
    render(<PageSubtitle title="Kanban" />)
    expect(screen.getByRole('heading', { name: 'Kanban', level: 1 })).toBeInTheDocument()
  })

  it('renders children alongside the title when provided', () => {
    render(
      <PageSubtitle title="Kanban">
        <Chip variant="indigo">live</Chip>
      </PageSubtitle>,
    )
    expect(screen.getByText('live').closest('[data-variant="indigo"]')).not.toBeNull()
  })

  it.each([
    ['undefined', undefined],
    ['null', null],
    ['false', false],
  ] as const)('does not render the subtitle slot when children=%s', (_label, value) => {
    const { container } = render(<PageSubtitle title="Kanban">{value as never}</PageSubtitle>)
    expect(container.querySelector('[data-slot="subtitle"]')).toBeNull()
  })

  it('binds title typography to a --size-* token (CSS source assertion)', () => {
    expect(pageSubtitleCss).toMatch(/\.title\s*\{[^}]*font-size:\s*var\(--size-/)
  })
})
