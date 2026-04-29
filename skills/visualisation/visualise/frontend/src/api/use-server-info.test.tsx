import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { ReactNode } from 'react'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useServerInfo } from './use-server-info'

function makeWrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
}

describe('useServerInfo', () => {
  beforeEach(() => {
    vi.unstubAllGlobals()
  })

  it('returns name and version from /api/info', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) => {
        if (url.endsWith('/api/info')) {
          return new Response(
            JSON.stringify({ name: 'accelerator-visualiser', version: '0.1.0' }),
            { status: 200 },
          )
        }
        return new Response('not found', { status: 404 })
      }),
    )
    const { result } = renderHook(() => useServerInfo(), { wrapper: makeWrapper() })
    await waitFor(() => expect(result.current.data?.version).toBe('0.1.0'))
    expect(result.current.data?.name).toBe('accelerator-visualiser')
  })

  it('surfaces an error when /api/info returns 500', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('boom', { status: 500 })))
    const { result } = renderHook(() => useServerInfo(), { wrapper: makeWrapper() })
    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(result.current.data).toBeUndefined()
  })

  it('does not throw when the response body is missing the version field', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({}), { status: 200 })),
    )
    const { result } = renderHook(() => useServerInfo(), { wrapper: makeWrapper() })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.version).toBeUndefined()
  })
})
