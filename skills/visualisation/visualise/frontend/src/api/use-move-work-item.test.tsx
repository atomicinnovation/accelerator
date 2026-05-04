import { describe, it, expect, vi, beforeEach } from 'vitest'
import React from 'react'
import { renderHook, act, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useMoveWorkItem } from './use-move-work-item'
import { createSelfCauseRegistry, SelfCauseContext } from './self-cause'
import { queryKeys } from './query-keys'
import * as fetchModule from './fetch'
import { ConflictError } from './fetch'
import type { IndexEntry } from './types'

function makeEntry(overrides: Partial<IndexEntry> = {}): IndexEntry {
  return {
    type: 'work-items',
    path: '/tmp/meta/work/0001-foo.md',
    relPath: 'meta/work/0001-foo.md',
    slug: '0001-foo',
    title: 'Foo',
    frontmatter: { status: 'todo' },
    frontmatterState: 'parsed',
    workItemRefs: ['0001'],
    mtimeMs: 0,
    size: 100,
    etag: 'sha256-OLD',
    bodyPreview: '',
    ...overrides,
  }
}

describe('useMoveWorkItem', () => {
  let queryClient: QueryClient
  let registry: ReturnType<typeof createSelfCauseRegistry>

  beforeEach(() => {
    queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    registry = createSelfCauseRegistry()
    vi.restoreAllMocks()
  })

  function wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <SelfCauseContext.Provider value={registry}>
          {children}
        </SelfCauseContext.Provider>
      </QueryClientProvider>
    )
  }

  it('optimistically updates status in cache', async () => {
    const entry = makeEntry()
    queryClient.setQueryData(queryKeys.docs('work-items'), [entry])

    vi.spyOn(fetchModule, 'patchTicketFrontmatter').mockReturnValue(new Promise(() => {}))

    const { result } = renderHook(() => useMoveWorkItem(), { wrapper })
    act(() => { result.current.mutate({ entry, toStatus: 'in-progress' }) })

    await waitFor(() => {
      const cached = queryClient.getQueryData<IndexEntry[]>(queryKeys.docs('work-items'))!
      expect(cached[0].frontmatter.status).toBe('in-progress')
    })
  })

  it('rolls back on error', async () => {
    const entry = makeEntry()
    queryClient.setQueryData(queryKeys.docs('work-items'), [entry])

    vi.spyOn(fetchModule, 'patchTicketFrontmatter').mockRejectedValue(
      new ConflictError(412, 'conflict', 'sha256-LATEST'),
    )

    const { result } = renderHook(() => useMoveWorkItem(), { wrapper })
    act(() => { result.current.mutate({ entry, toStatus: 'in-progress' }) })

    await waitFor(() => expect(result.current.isError).toBe(true))

    const cached = queryClient.getQueryData<IndexEntry[]>(queryKeys.docs('work-items'))!
    expect(cached[0].frontmatter.status).toBe('todo')
  })

  it('registers self etag on success', async () => {
    const entry = makeEntry()
    queryClient.setQueryData(queryKeys.docs('work-items'), [entry])

    vi.spyOn(fetchModule, 'patchTicketFrontmatter').mockResolvedValue({ etag: 'sha256-NEW' })

    const { result } = renderHook(() => useMoveWorkItem(), { wrapper })
    act(() => { result.current.mutate({ entry, toStatus: 'in-progress' }) })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    expect(registry.has('sha256-NEW')).toBe(true)
  })

  it('invalidates work-items query on settle', async () => {
    const entry = makeEntry()
    queryClient.setQueryData(queryKeys.docs('work-items'), [entry])

    vi.spyOn(fetchModule, 'patchTicketFrontmatter').mockResolvedValue({ etag: 'sha256-NEW' })
    vi.spyOn(queryClient, 'invalidateQueries')

    const { result } = renderHook(() => useMoveWorkItem(), { wrapper })
    act(() => { result.current.mutate({ entry, toStatus: 'in-progress' }) })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.docs('work-items') }),
    )
  })

  it('does not modify other entries in cache', async () => {
    const entryA = makeEntry({ relPath: 'meta/work/A.md', etag: 'sha256-A' })
    const entryB = makeEntry({ relPath: 'meta/work/B.md', etag: 'sha256-B' })
    queryClient.setQueryData(queryKeys.docs('work-items'), [entryA, entryB])

    vi.spyOn(fetchModule, 'patchTicketFrontmatter').mockReturnValue(new Promise(() => {}))

    const { result } = renderHook(() => useMoveWorkItem(), { wrapper })
    act(() => { result.current.mutate({ entry: entryA, toStatus: 'in-progress' }) })

    await waitFor(() => {
      const cached = queryClient.getQueryData<IndexEntry[]>(queryKeys.docs('work-items'))!
      expect(cached[0].frontmatter.status).toBe('in-progress')
    })

    const cached = queryClient.getQueryData<IndexEntry[]>(queryKeys.docs('work-items'))!
    expect(cached[1]).toBe(entryB)
  })
})
