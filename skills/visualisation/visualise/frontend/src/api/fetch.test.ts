import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  FetchError, ConflictError,
  fetchTypes, fetchDocs, fetchDocContent,
  fetchTemplates, fetchTemplateDetail,
  fetchLifecycleClusters, fetchLifecycleCluster,
  fetchRelated,
  patchTicketFrontmatter,
} from './fetch'

const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

beforeEach(() => mockFetch.mockReset())

describe('fetchTypes', () => {
  it('returns parsed JSON on 200', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ types: [{ key: 'decisions', label: 'Decisions', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false }] }),
    })
    const types = await fetchTypes()
    expect(types).toHaveLength(1)
    expect(types[0].key).toBe('decisions')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })
    await expect(fetchTypes()).rejects.toThrow('500')
  })
})

describe('fetchDocs', () => {
  it('unwraps the `docs` field from the response envelope', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ docs: [{ type: 'plans', path: '/p', relPath: 'r' }] }),
    })
    const docs = await fetchDocs('plans')
    expect(Array.isArray(docs)).toBe(true)
    expect(docs).toHaveLength(1)
  })

  it('url-encodes the type parameter', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ docs: [] }),
    })
    await fetchDocs('plan-reviews')
    expect(mockFetch).toHaveBeenCalledWith('/api/docs?type=plan-reviews')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    await expect(fetchDocs('plans')).rejects.toThrow('404')
  })
})

describe('fetchDocContent', () => {
  it('returns content and etag', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      text: async () => '# Hello',
      headers: { get: (h: string) => h === 'etag' ? '"sha256-abc"' : null },
    })
    const result = await fetchDocContent('meta/plans/foo.md')
    expect(result.content).toBe('# Hello')
    expect(result.etag).toBe('"sha256-abc"')
  })

  it('encodes path segments individually, preserving slash separators', async () => {
    // Locks in the per-segment encoding: spaces and special characters
    // get percent-encoded within a segment, but '/' between segments
    // stays literal so the server route `/api/docs/*path` receives the
    // right structure.
    mockFetch.mockResolvedValueOnce({
      ok: true, text: async () => '', headers: { get: () => null },
    })
    await fetchDocContent('meta/plans/with spaces/file#1.md')
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/docs/meta/plans/with%20spaces/file%231.md',
    )
  })

  it('falls back to empty etag when the header is missing', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, text: async () => 'x', headers: { get: () => null },
    })
    const result = await fetchDocContent('foo.md')
    expect(result.etag).toBe('')
  })
})

describe('fetchTemplates', () => {
  it('returns the full template-summary envelope', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ templates: [{ name: 'adr', activeTier: 'plugin-default', tiers: [] }] }),
    })
    const result = await fetchTemplates()
    expect(result.templates).toHaveLength(1)
    expect(result.templates[0].name).toBe('adr')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })
    await expect(fetchTemplates()).rejects.toThrow('500')
  })
})

describe('fetchTemplateDetail', () => {
  it('url-encodes the template name', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ name: 'adr', activeTier: 'plugin-default', tiers: [] }),
    })
    await fetchTemplateDetail('adr')
    expect(mockFetch).toHaveBeenCalledWith('/api/templates/adr')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    await expect(fetchTemplateDetail('missing')).rejects.toThrow('404')
  })
})

describe('fetchLifecycleClusters', () => {
  it('unwraps the `clusters` field from the response envelope', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        clusters: [
          {
            slug: 'foo',
            title: 'Foo',
            entries: [],
            completeness: {
              hasTicket: false, hasResearch: false, hasPlan: true,
              hasPlanReview: false, hasValidation: false, hasPr: false,
              hasPrReview: false, hasDecision: false, hasNotes: false,
            },
            lastChangedMs: 1_700_000_000_000,
          },
        ],
      }),
    })
    const clusters = await fetchLifecycleClusters()
    expect(clusters).toHaveLength(1)
    expect(clusters[0].slug).toBe('foo')
    expect(clusters[0].lastChangedMs).toBe(1_700_000_000_000)
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })
    await expect(fetchLifecycleClusters()).rejects.toThrow('500')
  })
})

describe('fetchRelated', () => {
  // ── Step 5.1 ─────────────────────────────────────────────────────────
  it('builds the right URL and decodes the payload', async () => {
    const payload = {
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    }
    mockFetch.mockResolvedValueOnce({ ok: true, json: async () => payload })
    const result = await fetchRelated('meta/plans/foo.md')
    expect(mockFetch).toHaveBeenCalledWith('/api/related/meta/plans/foo.md')
    expect(result).toEqual(payload)
  })

  // ── Step 5.2 ─────────────────────────────────────────────────────────
  it('throws FetchError on non-2xx', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    try {
      await fetchRelated('meta/plans/missing.md')
      throw new Error('expected throw')
    } catch (err) {
      expect(err).toBeInstanceOf(FetchError)
      expect((err as FetchError).status).toBe(404)
    }
  })

  // ── Step 5.3 ─────────────────────────────────────────────────────────
  it('encodes path segments individually, preserving slash separators', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        inferredCluster: [],
        declaredOutbound: [],
        declaredInbound: [],
      }),
    })
    await fetchRelated('meta/plans/with spaces/file#1.md')
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/related/meta/plans/with%20spaces/file%231.md',
    )
  })

  it('round-trips a literal % through encode/decode', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        inferredCluster: [],
        declaredOutbound: [],
        declaredInbound: [],
      }),
    })
    await fetchRelated('meta/plans/100%-coverage.md')
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/related/meta/plans/100%25-coverage.md',
    )
  })
})

describe('FetchError contract — all helpers throw FetchError on non-2xx', () => {
  it.each([
    ['fetchTypes',          () => fetchTypes()],
    ['fetchDocs',           () => fetchDocs('tickets')],
    ['fetchDocContent',     () => fetchDocContent('meta/tickets/0001-x.md')],
    ['fetchTemplates',      () => fetchTemplates()],
    ['fetchTemplateDetail', () => fetchTemplateDetail('foo')],
  ])('%s rejects with FetchError carrying the status', async (_name, call) => {
    mockFetch.mockResolvedValue({ ok: false, status: 503, headers: { get: () => null }, text: async () => '', json: async () => ({}) })
    await expect(call()).rejects.toBeInstanceOf(FetchError)
    try {
      await call()
    } catch (err) {
      expect(err).toBeInstanceOf(FetchError)
      expect((err as FetchError).status).toBe(503)
    }
  })

  it.each([
    ['fetchTypes',          () => fetchTypes(),                 404],
    ['fetchDocs',           () => fetchDocs('tickets'),         404],
    ['fetchDocContent',     () => fetchDocContent('foo.md'),    404],
    ['fetchTemplates',      () => fetchTemplates(),             404],
    ['fetchTemplateDetail', () => fetchTemplateDetail('foo'),   404],
  ])('%s rejects with FetchError(404)', async (_name, call, status) => {
    mockFetch.mockResolvedValue({ ok: false, status, headers: { get: () => null }, text: async () => '', json: async () => ({}) })
    await expect(call()).rejects.toMatchObject({
      name: 'FetchError',
      status,
    })
  })
})

describe('patchTicketFrontmatter', () => {
  it('sends PATCH with If-Match and JSON body', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 204,
      headers: { get: (h: string) => h === 'etag' ? '"sha256-NEW"' : null },
    })
    const result = await patchTicketFrontmatter(
      'meta/tickets/0001-foo.md',
      { status: 'in-progress' },
      'sha256-OLD',
    )
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/docs/meta/tickets/0001-foo.md/frontmatter',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.objectContaining({ 'If-Match': '"sha256-OLD"' }),
        body: JSON.stringify({ patch: { status: 'in-progress' } }),
      }),
    )
    expect(result).toEqual({ etag: 'sha256-NEW' })
  })

  it('unwraps quoted etag from response', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 204,
      headers: { get: (h: string) => h === 'etag' ? '"sha256-NEW"' : null },
    })
    const result = await patchTicketFrontmatter('meta/tickets/0001-foo.md', { status: 'todo' }, 'sha256-OLD')
    expect(result.etag).toBe('sha256-NEW')
  })

  it('throws ConflictError on 412 with currentEtag', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 412,
      json: async () => ({ currentEtag: 'sha256-LATEST' }),
    })
    await expect(
      patchTicketFrontmatter('meta/tickets/0001-foo.md', { status: 'done' }, 'sha256-OLD'),
    ).rejects.toSatisfy((e: unknown) => {
      return e instanceof ConflictError && e.status === 412 && e.currentEtag === 'sha256-LATEST'
    })
  })

  it('throws FetchError (not ConflictError) on other 4xx', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 400, json: async () => ({}) })
    const err = await patchTicketFrontmatter(
      'meta/tickets/0001-foo.md', { status: 'todo' }, 'sha256-OLD',
    ).catch((e: unknown) => e)
    expect(err).toBeInstanceOf(FetchError)
    expect(err).not.toBeInstanceOf(ConflictError)
    expect((err as FetchError).status).toBe(400)
  })

  it('encodes rel path segments', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 204,
      headers: { get: () => '"sha256-NEW"' },
    })
    await patchTicketFrontmatter('meta/tickets/0001 weird path.md', { status: 'todo' }, 'sha256-X')
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/docs/meta/tickets/0001%20weird%20path.md/frontmatter',
      expect.anything(),
    )
  })
})

describe('fetchLifecycleCluster', () => {
  it('returns the single-cluster payload directly', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        slug: 'foo', title: 'Foo', entries: [],
        completeness: {
          hasTicket: false, hasResearch: false, hasPlan: false,
          hasPlanReview: false, hasValidation: false, hasPr: false,
          hasPrReview: false, hasDecision: false, hasNotes: false,
        },
        lastChangedMs: 0,
      }),
    })
    const cluster = await fetchLifecycleCluster('foo')
    expect(cluster.slug).toBe('foo')
  })

  it('url-encodes the slug', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        slug: 'foo bar', title: '', entries: [],
        completeness: {
          hasTicket: false, hasResearch: false, hasPlan: false,
          hasPlanReview: false, hasValidation: false, hasPr: false,
          hasPrReview: false, hasDecision: false, hasNotes: false,
        },
        lastChangedMs: 0,
      }),
    })
    await fetchLifecycleCluster('foo bar')
    expect(mockFetch).toHaveBeenCalledWith('/api/lifecycle/foo%20bar')
  })

  it('throws on 404', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    await expect(fetchLifecycleCluster('missing')).rejects.toThrow('404')
  })
})
