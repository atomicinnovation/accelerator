import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { MemoryRouter } from '../../test/router-helpers'
import { SearchResultsPanel } from './SearchResultsPanel'
import * as fetchModule from '../../api/fetch'
import { FetchError } from '../../api/fetch'

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

function Wrap({ children, qc }: { children: React.ReactNode; qc: QueryClient }) {
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

async function flushMicrotasks() {
  await act(async () => {
    // React Query schedules data updates through both microtasks and timer
    // callbacks (e.g. requestIdleCallback shims). Drain both repeatedly so
    // resolved promises propagate to rendered DOM before assertions.
    for (let i = 0; i < 20; i++) {
      // eslint-disable-next-line no-await-in-loop
      await vi.advanceTimersByTimeAsync(1)
      // eslint-disable-next-line no-await-in-loop
      await Promise.resolve()
    }
  })
}

describe('SearchResultsPanel', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.restoreAllMocks()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('typing 2 chars issues one request after 200ms', async () => {
    const qc = makeClient()
    const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="ab" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith('ab', expect.anything())
  })

  it('does not request below 2 chars', async () => {
    const qc = makeClient()
    const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="a" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(spy).not.toHaveBeenCalled()
  })

  it('renders one row per result with the expected href in response order', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'First', slug: 'first', mtimeMs: 1 },
      { docType: 'decisions', title: 'Second', slug: 'second', mtimeMs: 2 },
      { docType: 'research', title: 'Third', slug: 'third', mtimeMs: 3 },
    ])
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    // Rows carry role="option" inside the listbox, but the element type
    // is still <a href> — native modifier-click / middle-click / Enter
    // semantics depend on that.
    const rows = screen.getAllByRole('option')
    expect(rows).toHaveLength(3)
    expect(rows[0].tagName).toBe('A')
    expect(rows[0].hasAttribute('href')).toBe(true)
    expect(rows[0].getAttribute('href')).toBe('/library/plans/first')
    expect(rows[1].getAttribute('href')).toBe('/library/decisions/second')
    expect(rows[2].getAttribute('href')).toBe('/library/research/third')
  })

  it('each result row renders title and sentence-case label', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Pumpkin pie plan', slug: 'pp', mtimeMs: 1 },
      { docType: 'work-items', title: 'Pumpkin work item', slug: 'pwi', mtimeMs: 2 },
    ])
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="xyzzyx" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    // Highlight does not match (query absent from titles), so titles render
    // as single text nodes.
    expect(container.textContent).toContain('Pumpkin pie plan')
    expect(screen.getByText('Plans')).toBeInTheDocument()
    expect(container.textContent).toContain('Pumpkin work item')
    expect(screen.getByText('Work items')).toBeInTheDocument()
  })

  it('renders a Glyph per result (data-doc-type element)', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    // Glyph (framed or unframed) carries data-doc-type on at least one element.
    const glyph = container.querySelector('[data-doc-type="plans"]')
    expect(glyph).not.toBeNull()
  })

  it('highlights the matched substring inside the title', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo Bar Baz', slug: 'foo-bar-baz', mtimeMs: 1 },
    ])
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="bar" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    const mark = container.querySelector('mark')
    expect(mark).not.toBeNull()
    expect(mark!.textContent).toBe('Bar')
  })

  it('shows a result-type label colored via DOC_TYPE color var', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    expect(screen.getByText('Plans')).toBeInTheDocument()
    // The path sub-row renders <docType>/<slug>.
    expect(container.textContent).toContain('plans/foo')
  })

  it('meta row shows match count and the settled query', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
      { docType: 'plans', title: 'Foo2', slug: 'foo2', mtimeMs: 2 },
    ])
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    // "2 matches · foo"
    expect(container.textContent).toContain('2')
    expect(container.textContent).toContain('matches')
    expect(container.textContent).toContain('foo')
    // ↵ and esc hint kbds
    expect(screen.getByText('↵')).toBeInTheDocument()
    expect(screen.getByText('esc')).toBeInTheDocument()
  })

  it('singular "match" wording when one result', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    expect(container.textContent).toContain('1 match')
    expect(container.textContent).not.toContain('1 matches')
  })

  it('result rows have role="option" inside listbox', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    expect(screen.getByRole('listbox')).toBeInTheDocument()
    expect(screen.getAllByRole('option')).toHaveLength(1)
  })

  it('loading state shows a loadbar and "Searching meta/ for X…" hint', async () => {
    const qc = makeClient()
    // Never-resolving — keeps the panel in the loading branch.
    vi.spyOn(fetchModule, 'fetchSearch').mockImplementation(
      () => new Promise<fetchModule.SearchResult[]>(() => {/* never */}),
    )
    const { container } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="abcd" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    // The loadhint mentions the trimmed query.
    expect(container.textContent).toContain('Searching meta/ for')
    expect(container.textContent).toContain('abcd')
  })

  it('empty results show expanded "No matches" status with aria-live', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="zzz" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    const status = screen.getByRole('status')
    // The status block contains the heading "No matches" plus a body
    // explaining the empty result; the title is its own element.
    expect(screen.getByText('No matches')).toBeInTheDocument()
    expect(status.getAttribute('aria-live')).toBe('polite')
    expect(status.textContent).toContain('zzz')
  })

  it('results region has accessible name "Search results"', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    expect(
      screen.getByRole('region', { name: /search results/i }),
    ).toBeInTheDocument()
  })

  it('below threshold clears the panel', async () => {
    const qc = makeClient()
    vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([
      { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
    ])
    const { rerender } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="abc" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    await flushMicrotasks()
    expect(screen.queryAllByRole('option').length).toBeGreaterThan(0)
    rerender(
      <Wrap qc={qc}>
        <SearchResultsPanel query="a" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(screen.queryAllByRole('option')).toHaveLength(0)
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('in-flight keeps prior results visible via placeholderData', async () => {
    const qc = makeClient()
    // First key 'ab' resolves with two rows; second key 'cd' never resolves.
    vi.spyOn(fetchModule, 'fetchSearch').mockImplementation(
      (q: string) => {
        if (q === 'ab') {
          return Promise.resolve([
            { docType: 'plans', title: 'Foo', slug: 'foo', mtimeMs: 1 },
            { docType: 'plans', title: 'Bar', slug: 'bar', mtimeMs: 2 },
          ] as fetchModule.SearchResult[])
        }
        return new Promise<fetchModule.SearchResult[]>(() => {/* never */})
      },
    )
    // Use a host component with internal state so React preserves the
    // <SearchResultsPanel> instance (and thus its useQuery observer) when
    // we trigger a state change. A rerender through <Wrap> would rebuild
    // the MemoryRouter and remount the panel — losing keepPreviousData's
    // prior data — which is a test-infra artefact, not real behaviour.
    let setQueryExternal: ((q: string) => void) | null = null
    function Host() {
      const [q, setQ] = React.useState('ab')
      setQueryExternal = setQ
      return <SearchResultsPanel query={q} />
    }
    render(
      <Wrap qc={qc}>
        <Host />
      </Wrap>,
    )
    await flushMicrotasks()
    expect(screen.getAllByRole('option')).toHaveLength(2)

    await act(async () => {
      setQueryExternal!('cd')
    })
    // Advance debounce so 'cd' is dispatched.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(250)
    })
    await flushMicrotasks()
    // The two 'ab' rows are still visible while 'cd' is in flight.
    expect(screen.getAllByRole('option')).toHaveLength(2)
    expect(screen.queryByRole('status')).toBeNull()
    // The 'cd' fetch is observed pending.
    const cdState = qc.getQueryState(['search', 'cd'])
    expect(cdState?.fetchStatus).toBe('fetching')
    expect(cdState?.data).toBeUndefined()
  })

  it('fetch error clears the panel', async () => {
    const qc = makeClient()
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const fetchSpy = vi.spyOn(fetchModule, 'fetchSearch').mockImplementation(
      async () => {
        // Mirror fetchSearch's catch path — log and re-throw.
        const err = new FetchError(500, 'GET /api/search: 500')
        console.error(err)
        throw err
      },
    )
    render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="foo" />
      </Wrap>,
    )
    await flushMicrotasks()
    expect(fetchSpy).toHaveBeenCalled()
    expect(screen.queryAllByRole('option')).toHaveLength(0)
    expect(screen.queryByRole('status')).toBeNull()
    expect(consoleSpy).toHaveBeenCalled()
    const firstArg = consoleSpy.mock.calls[0][0] as Error
    expect(firstArg.message).toContain('/api/search')
    consoleSpy.mockRestore()
  })

  it('aborts in-flight request on unmount', async () => {
    const qc = makeClient()
    let capturedSignal: AbortSignal | undefined
    vi.spyOn(fetchModule, 'fetchSearch').mockImplementation(
      (_q: string, signal?: AbortSignal) => {
        capturedSignal = signal
        return new Promise<fetchModule.SearchResult[]>(() => {/* never */})
      },
    )
    const { unmount } = render(
      <Wrap qc={qc}>
        <SearchResultsPanel query="abcd" />
      </Wrap>,
    )
    await act(async () => {
      vi.advanceTimersByTime(200)
    })
    expect(capturedSignal?.aborted).toBe(false)
    unmount()
    expect(capturedSignal?.aborted).toBe(true)
  })
})
