import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { useWikiLinkResolver } from './use-wiki-link-resolver'
import * as fetchModule from './fetch'
import { makeIndexEntry } from './test-fixtures'
import { queryKeys } from './query-keys'

function makeWrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: qc }, children)
  }
}

const exampleAdr = makeIndexEntry({
  type: 'decisions',
  relPath: 'meta/decisions/ADR-0001-example.md',
  title: 'Example decision',
  frontmatter: { adr_id: 'ADR-0001' },
})

describe('useWikiLinkResolver', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    // Stub the work-item/config fetch so tests don't need a real server.
    vi.stubGlobal('fetch', vi.fn((url: string) => {
      if (url === '/api/work-item/config') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ }),
        })
      }
      return Promise.reject(new Error(`unexpected fetch: ${url}`))
    }))
  })

  // ── Step 5.7 ────────────────────────────────────────────────────────
  it('returns kind=resolved after both docs queries settle and ID matches', async () => {
    const qc = new QueryClient()
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) =>
      Promise.resolve(type === 'decisions' ? [exampleAdr] : []),
    )

    const { result } = renderHook(() => useWikiLinkResolver(), {
      wrapper: makeWrapper(qc),
    })

    await waitFor(() => {
      const r = result.current.resolver('ADR', '1')
      expect(r.kind).toBe('resolved')
    })
    const resolved = result.current.resolver('ADR', '1')
    expect(resolved).toEqual({
      kind: 'resolved',
      href: '/library/decisions/ADR-0001-example',
      title: 'Example decision',
    })
  })

  // ── Step 5.7b ───────────────────────────────────────────────────────
  it('returns kind=unresolved for unknown IDs after queries settle', async () => {
    const qc = new QueryClient()
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])

    const { result } = renderHook(() => useWikiLinkResolver(), {
      wrapper: makeWrapper(qc),
    })

    await waitFor(() => {
      const r = result.current.resolver('ADR', '9999')
      expect(r.kind).toBe('unresolved')
    })
  })

  // ── Step 5.8 ────────────────────────────────────────────────────────
  it('returns kind=pending while either query is pending', () => {
    const qc = new QueryClient()
    // Never resolves — query stays pending.
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation(
      () => new Promise(() => {}),
    )

    const { result } = renderHook(() => useWikiLinkResolver(), {
      wrapper: makeWrapper(qc),
    })

    expect(result.current.resolver('ADR', '1')).toEqual({ kind: 'pending' })
  })

  // ── Step 5.8b ───────────────────────────────────────────────────────
  it('resolver reference is memo-stable across re-renders with unchanged state', async () => {
    const qc = new QueryClient()
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) =>
      Promise.resolve(type === 'decisions' ? [exampleAdr] : []),
    )

    const { result, rerender } = renderHook(() => useWikiLinkResolver(), {
      wrapper: makeWrapper(qc),
    })

    // While both queries are still pending, the resolver should be
    // stable across re-renders.
    const pendingFirst = result.current.resolver
    rerender()
    expect(result.current.resolver).toBe(pendingFirst)

    // After settle, the resolver reference rotates exactly once.
    await waitFor(() => {
      expect(result.current.resolver('ADR', '1').kind).toBe('resolved')
    })
    const settledFirst = result.current.resolver
    expect(settledFirst).not.toBe(pendingFirst)

    // Subsequent re-renders keep the same settled reference.
    rerender()
    expect(result.current.resolver).toBe(settledFirst)
  })

  // ── Step 5.8c ───────────────────────────────────────────────────────
  it('refetch with cached data does NOT return resolver to pending', async () => {
    const qc = new QueryClient()
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) =>
      Promise.resolve(type === 'decisions' ? [exampleAdr] : []),
    )

    const { result } = renderHook(() => useWikiLinkResolver(), {
      wrapper: makeWrapper(qc),
    })

    await waitFor(() => {
      expect(result.current.resolver('ADR', '1').kind).toBe('resolved')
    })

    // Trigger a background refetch — cached data remains, so isPending
    // stays false. The resolver must keep returning resolved verdicts
    // for the duration of the refetch.
    await act(async () => {
      await qc.invalidateQueries({ queryKey: queryKeys.docs('decisions') })
    })

    expect(result.current.resolver('ADR', '1').kind).toBe('resolved')
  })
})
