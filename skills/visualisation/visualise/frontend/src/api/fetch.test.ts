import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  fetchTypes, fetchDocs, fetchDocContent,
  fetchTemplates, fetchTemplateDetail,
} from './fetch'

const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

beforeEach(() => mockFetch.mockReset())

describe('fetchTypes', () => {
  it('returns parsed JSON on 200', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => [{ key: 'decisions', label: 'Decisions', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false }],
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
