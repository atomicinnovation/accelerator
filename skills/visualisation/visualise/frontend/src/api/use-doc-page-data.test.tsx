import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { useDocPageData } from './use-doc-page-data'
import * as fetchModule from './fetch'

function makeWrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: qc }, children)
  }
}

describe('useDocPageData', () => {
  beforeEach(() => vi.restoreAllMocks())

  // ── Step 5.9 ────────────────────────────────────────────────────────
  it('gates both children on relPath; fires both once a value arrives', async () => {
    const qc = new QueryClient()
    const contentSpy = vi
      .spyOn(fetchModule, 'fetchDocContent')
      .mockResolvedValue({ content: '# Body', etag: 'sha256-abc' })
    const relatedSpy = vi
      .spyOn(fetchModule, 'fetchRelated')
      .mockResolvedValue({
        inferredCluster: [],
        declaredOutbound: [],
        declaredInbound: [],
      })

    const initialProps: { p: string | undefined } = { p: undefined }
    const { rerender } = renderHook(
      ({ p }: { p: string | undefined }) => useDocPageData(p),
      { wrapper: makeWrapper(qc), initialProps },
    )

    // Neither fires while relPath is undefined.
    expect(contentSpy).not.toHaveBeenCalled()
    expect(relatedSpy).not.toHaveBeenCalled()

    rerender({ p: 'meta/plans/foo.md' })

    await waitFor(() => {
      expect(contentSpy).toHaveBeenCalledTimes(1)
      expect(relatedSpy).toHaveBeenCalledTimes(1)
    })
    // The two queries don't collide on a shared cache key — both
    // produce distinct cached data.
    expect(qc.getQueryData(['doc-content', 'meta/plans/foo.md'])).toBeDefined()
    expect(qc.getQueryData(['related', 'meta/plans/foo.md'])).toBeDefined()
  })
})
