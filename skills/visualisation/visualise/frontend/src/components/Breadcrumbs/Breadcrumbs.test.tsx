import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { rootRouteId } from '@tanstack/router-core'
import breadcrumbsCss from './Breadcrumbs.module.css?raw'
import { Breadcrumbs } from './Breadcrumbs'

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return {
    ...actual,
    useMatches: vi.fn(),
    useRouter: vi.fn(() => ({ navigate: vi.fn() })),
  }
})

import { useMatches, useRouter } from '@tanstack/react-router'

type PartialMatch = {
  id: string
  routeId: string
  pathname: string
  status: 'success' | 'pending' | 'error' | 'redirected' | 'notFound'
  loaderData: unknown
}

function makeMatch(overrides: Partial<PartialMatch>): PartialMatch {
  return {
    id: overrides.routeId ?? 'test',
    routeId: '__root__',
    pathname: '/',
    status: 'success',
    loaderData: undefined,
    ...overrides,
  }
}

const rootMatch = makeMatch({ routeId: rootRouteId, id: rootRouteId })

afterEach(() => {
  vi.clearAllMocks()
})

describe('URL matrix', () => {
  it('/library/templates shows Library → Templates trail', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', loaderData: { crumb: 'Library' } }),
      makeMatch({ id: '/library/templates', routeId: '/library/templates', pathname: '/library/templates', loaderData: { crumb: 'Templates' } }),
    ] as any)
    render(<Breadcrumbs />)

    const nav = screen.getByRole('navigation', { name: 'Breadcrumb' })
    expect(nav).toBeInTheDocument()
    const libraryLink = screen.getByRole('link', { name: 'Library' })
    expect(libraryLink).toHaveAttribute('href', '/library')
    const currentCrumb = screen.getByText('Templates')
    expect(currentCrumb).toHaveAttribute('aria-current', 'page')
  })

  it('/library/templates/adr shows Library → Templates → adr trail', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', loaderData: { crumb: 'Library' } }),
      makeMatch({ id: '/library/templates', routeId: '/library/templates', pathname: '/library/templates', loaderData: { crumb: 'Templates' } }),
      makeMatch({ id: '/library/templates/adr', routeId: '/library/templates/$name', pathname: '/library/templates/adr', loaderData: { crumb: 'adr' } }),
    ] as any)
    render(<Breadcrumbs />)

    expect(screen.getByRole('link', { name: 'Library' })).toHaveAttribute('href', '/library')
    expect(screen.getByRole('link', { name: 'Templates' })).toHaveAttribute('href', '/library/templates')
    expect(screen.getByText('adr')).toHaveAttribute('aria-current', 'page')
  })

  it('/library/decisions shows Library → decisions trail', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', loaderData: { crumb: 'Library' } }),
      makeMatch({ id: '/library/decisions', routeId: '/library/$type', pathname: '/library/decisions', loaderData: { crumb: 'decisions' } }),
    ] as any)
    render(<Breadcrumbs />)

    expect(screen.getByRole('link', { name: 'Library' })).toHaveAttribute('href', '/library')
    expect(screen.getByText('decisions')).toHaveAttribute('aria-current', 'page')
  })

  it('/library/decisions/foo-slug shows Library → decisions → foo-slug trail', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', loaderData: { crumb: 'Library' } }),
      makeMatch({ id: '/library/decisions', routeId: '/library/$type', pathname: '/library/decisions', loaderData: { crumb: 'decisions' } }),
      makeMatch({ id: '/library/decisions/foo-slug', routeId: '/library/$type/$fileSlug', pathname: '/library/decisions/foo-slug', loaderData: { crumb: 'foo-slug' } }),
    ] as any)
    render(<Breadcrumbs />)

    expect(screen.getByRole('link', { name: 'Library' })).toHaveAttribute('href', '/library')
    expect(screen.getByRole('link', { name: 'decisions' })).toHaveAttribute('href', '/library/decisions')
    expect(screen.getByText('foo-slug')).toHaveAttribute('aria-current', 'page')
  })

  it('/lifecycle/cluster-x shows Lifecycle → cluster-x trail', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/lifecycle', routeId: '/lifecycle', pathname: '/lifecycle', loaderData: { crumb: 'Lifecycle' } }),
      makeMatch({ id: '/lifecycle/cluster-x', routeId: '/lifecycle/$slug', pathname: '/lifecycle/cluster-x', loaderData: { crumb: 'cluster-x' } }),
    ] as any)
    render(<Breadcrumbs />)

    expect(screen.getByRole('link', { name: 'Lifecycle' })).toHaveAttribute('href', '/lifecycle')
    expect(screen.getByText('cluster-x')).toHaveAttribute('aria-current', 'page')
  })

  it('/kanban shows single Kanban crumb with no ancestor links', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/kanban', routeId: '/kanban', pathname: '/kanban', loaderData: { crumb: 'Kanban' } }),
    ] as any)
    render(<Breadcrumbs />)

    expect(screen.getByText('Kanban')).toHaveAttribute('aria-current', 'page')
    const nav = screen.getByRole('navigation', { name: 'Breadcrumb' })
    expect(nav.querySelectorAll('a')).toHaveLength(0)
  })

  it('renders root nav element wrapping an ol', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/kanban', routeId: '/kanban', pathname: '/kanban', loaderData: { crumb: 'Kanban' } }),
    ] as any)
    render(<Breadcrumbs />)

    const nav = screen.getByRole('navigation', { name: 'Breadcrumb' })
    expect(nav.querySelector('ol')).not.toBeNull()
  })
})

describe('click handler', () => {
  it('calls router.navigate on plain left-click', () => {
    const navigate = vi.fn()
    vi.mocked(useRouter).mockReturnValue({ navigate } as any)
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', loaderData: { crumb: 'Library' } }),
      makeMatch({ id: '/library/decisions', routeId: '/library/$type', pathname: '/library/decisions', loaderData: { crumb: 'decisions' } }),
    ] as any)
    render(<Breadcrumbs />)

    fireEvent.click(screen.getByRole('link', { name: 'Library' }))
    expect(navigate).toHaveBeenCalledWith({ to: '/library' })
  })

  it('does NOT call router.navigate on cmd-click', () => {
    const navigate = vi.fn()
    vi.mocked(useRouter).mockReturnValue({ navigate } as any)
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', loaderData: { crumb: 'Library' } }),
      makeMatch({ id: '/library/decisions', routeId: '/library/$type', pathname: '/library/decisions', loaderData: { crumb: 'decisions' } }),
    ] as any)
    render(<Breadcrumbs />)

    fireEvent.click(screen.getByRole('link', { name: 'Library' }), { metaKey: true })
    expect(navigate).not.toHaveBeenCalled()
  })
})

describe('pending and empty states', () => {
  it('excludes matches with status: pending', () => {
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', pathname: '/library', status: 'pending', loaderData: undefined }),
    ] as any)
    const { container } = render(<Breadcrumbs />)
    expect(container.firstChild).toBeNull()
  })

  it('returns null when no crumb matches exist', () => {
    vi.mocked(useMatches).mockReturnValue([rootMatch] as any)
    const { container } = render(<Breadcrumbs />)
    expect(container.firstChild).toBeNull()
  })
})

describe('dev-warn predicate', () => {
  it('warns when a success non-root match is missing crumb', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/some/path', routeId: '/some/path', status: 'success', loaderData: {} }),
    ] as any)
    render(<Breadcrumbs />)
    expect(spy).toHaveBeenCalledOnce()
  })

  it('does NOT warn for the root route', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.mocked(useMatches).mockReturnValue([
      makeMatch({ id: rootRouteId, routeId: rootRouteId, status: 'success', loaderData: {} }),
    ] as any)
    render(<Breadcrumbs />)
    expect(spy).not.toHaveBeenCalled()
  })

  it('does NOT warn for redirected matches', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library/', routeId: '/library/', status: 'redirected', loaderData: undefined }),
    ] as any)
    render(<Breadcrumbs />)
    expect(spy).not.toHaveBeenCalled()
  })

  it('does NOT warn for pending matches', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library/$type', routeId: '/library/$type', status: 'pending', loaderData: undefined }),
    ] as any)
    render(<Breadcrumbs />)
    expect(spy).not.toHaveBeenCalled()
  })

  it('does NOT warn for error matches', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library/$type', routeId: '/library/$type', status: 'error', loaderData: undefined }),
    ] as any)
    render(<Breadcrumbs />)
    expect(spy).not.toHaveBeenCalled()
  })

  it('does NOT warn when crumb contract is fulfilled', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.mocked(useMatches).mockReturnValue([
      rootMatch,
      makeMatch({ id: '/library', routeId: '/library', status: 'success', loaderData: { crumb: 'Library' } }),
    ] as any)
    render(<Breadcrumbs />)
    expect(spy).not.toHaveBeenCalled()
  })
})

describe('CSS source assertions', () => {
  it('.breadcrumbs has margin-left: 0 (selector-bound)', () => {
    expect(breadcrumbsCss).toMatch(/\.breadcrumbs\s*\{[^}]*margin-left:\s*0/)
  })

  it('.link declares :focus-visible', () => {
    expect(breadcrumbsCss).toContain(':focus-visible')
  })
})
