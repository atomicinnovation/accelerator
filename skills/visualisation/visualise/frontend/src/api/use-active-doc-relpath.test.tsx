import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import {
  createRootRoute, createRoute, createRouter,
  createMemoryHistory, RouterProvider, Outlet,
} from '@tanstack/react-router'
import * as fetchModule from './fetch'
import { useActiveDocRelPath } from './use-active-doc-relpath'
import type { IndexEntry } from './types'

function entry(overrides: Partial<IndexEntry>): IndexEntry {
  return {
    type: 'work-items',
    path: 'meta/work/0007-foo.md',
    relPath: 'meta/work/0007-foo.md',
    slug: '0007-foo',
    workItemId: '0007',
    title: 'Foo',
    frontmatter: {},
    frontmatterState: 'parsed',
    workItemRefs: [],
    mtimeMs: 0,
    size: 0,
    etag: 'e',
    bodyPreview: '',
    completeness: null,
    linkedCount: 0,
    clusterKey: null,
    ...overrides,
  }
}

function makeRouterWrapper(atUrl: string) {
  const root = createRootRoute({ component: () => <Outlet /> })
  const Capture = ({
    captureRef,
  }: {
    captureRef: { current: string | undefined }
  }) => {
    captureRef.current = useActiveDocRelPath()
    return null
  }
  const captureRef: { current: string | undefined } = { current: undefined }
  const indexRoute = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: () => <Capture captureRef={captureRef} />,
  })
  const typeRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type',
    component: () => <Capture captureRef={captureRef} />,
  })
  const docRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type/$fileSlug',
    component: () => <Capture captureRef={captureRef} />,
  })
  const tree = root.addChildren([indexRoute, typeRoute, docRoute])
  const router = createRouter({
    routeTree: tree,
    history: createMemoryHistory({ initialEntries: [atUrl] }),
  })
  return { router, captureRef }
}

function makeWrapper(qc: QueryClient, atUrl: string) {
  const { router, captureRef } = makeRouterWrapper(atUrl)
  function Wrapper({ children: _children }: { children?: React.ReactNode }) {
    return (
      <QueryClientProvider client={qc}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )
  }
  return { Wrapper, captureRef }
}

function renderAt(atUrl: string) {
  const qc = new QueryClient()
  const { Wrapper, captureRef } = makeWrapper(qc, atUrl)
  const utils = renderHook(() => null, { wrapper: Wrapper as any })
  return { captureRef, utils }
}

describe('useActiveDocRelPath', () => {
  beforeEach(() => vi.restoreAllMocks())

  it('returns entry.relPath when slug matches', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([
      entry({ slug: '0007-foo', relPath: 'meta/work/0007-foo.md' }),
    ])
    const { captureRef } = renderAt('/library/work-items/0007-foo')
    await waitFor(() => expect(captureRef.current).toBe('meta/work/0007-foo.md'))
  })

  it('matches via fileSlugFromRelPath fallback when slug differs', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([
      entry({ slug: 'something-else', relPath: 'meta/work/0007-foo.md' }),
    ])
    const { captureRef } = renderAt('/library/work-items/0007-foo')
    await waitFor(() => expect(captureRef.current).toBe('meta/work/0007-foo.md'))
  })

  it('returns undefined off a doc route', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const { captureRef } = renderAt('/')
    // Give effects a tick.
    await waitFor(() => {
      // No assertion failure expected — value remains undefined
      expect(captureRef.current).toBeUndefined()
    })
  })

  it('returns undefined when docs list unloaded or unmatched', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([
      entry({ slug: 'other', relPath: 'meta/work/other.md' }),
    ])
    const { captureRef } = renderAt('/library/work-items/0007-foo')
    await waitFor(() => {
      // After settling, no match → undefined
      expect(captureRef.current).toBeUndefined()
    })
  })

  it('returns undefined for an invalid type (query disabled)', async () => {
    const spy = vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const { captureRef } = renderAt('/library/not-a-real-type/abc')
    await waitFor(() => {
      expect(captureRef.current).toBeUndefined()
    })
    expect(spy).not.toHaveBeenCalled()
  })
})
