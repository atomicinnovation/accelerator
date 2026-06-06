import { describe, it, expect, vi } from 'vitest'
import React from 'react'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useDocCluster } from './use-doc-cluster'
import * as fetchModule from './fetch'
import { makeIndexEntry, makeLifecycleCluster } from './test-fixtures'

function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  }
}

const docEntry = makeIndexEntry({
  type: 'plans',
  path: '/p/meta/plans/2026-01-01-foo.md',
  relPath: 'meta/plans/2026-01-01-foo.md',
})

describe('useDocCluster', () => {
  it('returns the cluster whose entries contain the doc by path', async () => {
    const cluster = makeLifecycleCluster({
      slug: '0001',
      title: 'Foo cluster',
      entries: [makeIndexEntry({ path: '/other.md' }), docEntry],
    })
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([
      makeLifecycleCluster({ slug: 'unrelated', entries: [makeIndexEntry({ path: '/nope.md' })] }),
      cluster,
    ])
    const { result } = renderHook(() => useDocCluster(docEntry), {
      wrapper: makeWrapper(),
    })
    await waitFor(() => expect(result.current.cluster).not.toBeNull())
    expect(result.current.cluster?.slug).toBe('0001')
    expect(result.current.isError).toBe(false)
  })

  it('returns cluster: null with isPending while the query is in flight', () => {
    // Enabled query (entry provided) + never-resolving promise: cluster is
    // null and isPending true. Driving an *enabled* query rules out the
    // disabled-idle path (isPending true / fetchStatus idle) passing this.
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    const { result } = renderHook(() => useDocCluster(docEntry), {
      wrapper: makeWrapper(),
    })
    expect(result.current.cluster).toBeNull()
    expect(result.current.isPending).toBe(true)
    expect(result.current.fetchStatus).toBe('fetching')
  })

  it('returns cluster: null with isError when the fetch rejects', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockRejectedValue(
      new Error('lifecycle-boom'),
    )
    const { result } = renderHook(() => useDocCluster(docEntry), {
      wrapper: makeWrapper(),
    })
    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(result.current.cluster).toBeNull()
  })

  it('returns cluster: null (settled) when no cluster contains the doc', async () => {
    // Negative case via a cluster list deliberately missing the doc's path —
    // exercises the hook's negative branch directly, independent of server
    // bucketing rules.
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([
      makeLifecycleCluster({ slug: 'a', entries: [makeIndexEntry({ path: '/x.md' })] }),
      makeLifecycleCluster({ slug: 'b', entries: [makeIndexEntry({ path: '/y.md' })] }),
    ])
    const { result } = renderHook(() => useDocCluster(docEntry), {
      wrapper: makeWrapper(),
    })
    await waitFor(() => expect(result.current.isPending).toBe(false))
    expect(result.current.cluster).toBeNull()
    expect(result.current.isError).toBe(false)
  })

  it('returns cluster: null (settled) for a slug-less lifecycle-participating doc', async () => {
    // A Plan with slug == null is the genuine server-side no-cluster case.
    const slugless = makeIndexEntry({
      type: 'plans',
      slug: null,
      path: '/p/meta/plans/orphan.md',
      relPath: 'meta/plans/orphan.md',
    })
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([
      makeLifecycleCluster({ slug: 'a', entries: [makeIndexEntry({ path: '/x.md' })] }),
    ])
    const { result } = renderHook(() => useDocCluster(slugless), {
      wrapper: makeWrapper(),
    })
    await waitFor(() => expect(result.current.isPending).toBe(false))
    expect(result.current.cluster).toBeNull()
  })
})
