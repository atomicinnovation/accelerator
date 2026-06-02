import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { useSearch } from './use-search'
import * as fetchModule from './fetch'

function makeWrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: qc }, children)
  }
}

function makeClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: Infinity,
        gcTime: Infinity,
      },
    },
  })
}

describe('useSearch', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.restoreAllMocks()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('does not request below 2 chars', async () => {
    const qc = makeClient()
    const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    renderHook(({ q }: { q: string }) => useSearch(q), {
      wrapper: makeWrapper(qc),
      initialProps: { q: 'a' },
    })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).not.toHaveBeenCalled()
  })

  it('requests once after settle for 2+ chars', async () => {
    const qc = makeClient()
    const spy = vi
      .spyOn(fetchModule, 'fetchSearch')
      .mockResolvedValue([])
    renderHook(({ q }: { q: string }) => useSearch(q), {
      wrapper: makeWrapper(qc),
      initialProps: { q: 'ab' },
    })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith('ab', expect.anything())
  })

  it('dedupes via react-query cache within gcTime', async () => {
    const qc = makeClient()
    const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    const { rerender } = renderHook(
      ({ q }: { q: string }) => useSearch(q),
      { wrapper: makeWrapper(qc), initialProps: { q: 'ab' } },
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    // Let the resolved promise flush through React Query.
    await act(async () => {
      await Promise.resolve()
      await Promise.resolve()
    })
    rerender({ q: 'a' })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    rerender({ q: 'ab' })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).toHaveBeenCalledTimes(1)
  })

  it('intermediate keystrokes do not settle', async () => {
    const qc = makeClient()
    const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    const { rerender } = renderHook(
      ({ q }: { q: string }) => useSearch(q),
      { wrapper: makeWrapper(qc), initialProps: { q: 'ab' } },
    )
    await act(async () => {
      vi.advanceTimersByTime(100)
    })
    rerender({ q: 'abc' })
    await act(async () => {
      vi.advanceTimersByTime(100)
    })
    rerender({ q: 'ab' })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith('ab', expect.anything())
    expect(spy).not.toHaveBeenCalledWith('abc', expect.anything())
  })

  it('trims input before debounce', async () => {
    const qc = makeClient()
    const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    renderHook(({ q }: { q: string }) => useSearch(q), {
      wrapper: makeWrapper(qc),
      initialProps: { q: '  ab  ' },
    })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).toHaveBeenCalledWith('ab', expect.anything())
  })

  it('query key uses settled trimmed value', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    renderHook(({ q }: { q: string }) => useSearch(q), {
      wrapper: makeWrapper(qc),
      initialProps: { q: '  ab  ' },
    })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(qc.getQueryState(['search', 'ab'])).toBeDefined()
    expect(qc.getQueryState(['search', '  ab  '])).toBeUndefined()
  })

  it('aborts in-flight request on key change', async () => {
    const qc = makeClient()
    let capturedSignal: AbortSignal | undefined
    const neverResolves = new Promise<fetchModule.SearchResult[]>(() => {
      /* never */
    })
    vi.spyOn(fetchModule, 'fetchSearch').mockImplementation(
      (_q: string, signal?: AbortSignal) => {
        if (capturedSignal === undefined) capturedSignal = signal
        return neverResolves
      },
    )
    const { rerender } = renderHook(
      ({ q }: { q: string }) => useSearch(q),
      { wrapper: makeWrapper(qc), initialProps: { q: 'abcd' } },
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(capturedSignal).toBeDefined()
    expect(capturedSignal?.aborted).toBe(false)
    rerender({ q: 'abcde' })
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(capturedSignal?.aborted).toBe(true)
  })
})
