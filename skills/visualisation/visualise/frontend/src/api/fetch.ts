import type {
  DocType, DocTypeKey, DocsListResponse, IndexEntry,
  TemplateSummaryListResponse, TemplateDetail,
  LifecycleCluster, LifecycleListResponse,
} from './types'

/** Typed error thrown by fetch helpers on non-2xx responses, so
 *  callers can branch on `err instanceof FetchError && err.status === 404`
 *  rather than substring-matching the message. */
export class FetchError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message)
    this.name = 'FetchError'
  }
}

export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch('/api/types')
  if (!r.ok) throw new FetchError(r.status, `GET /api/types: ${r.status}`)
  return r.json()
}

export async function fetchDocs(type: DocTypeKey): Promise<IndexEntry[]> {
  const r = await fetch(`/api/docs?type=${encodeURIComponent(type)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/docs?type=${type}: ${r.status}`)
  const body: DocsListResponse = await r.json()
  return body.docs
}

export async function fetchDocContent(relPath: string): Promise<{ content: string; etag: string }> {
  // Encode per-segment so filenames containing '#', '?', '%', or
  // non-ASCII are transmitted correctly. '/' separators between segments
  // stay literal since the server accepts them as path structure.
  const encodedPath = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/docs/${encodedPath}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/docs/${relPath}: ${r.status}`)
  const content = await r.text()
  const etag = r.headers.get('etag') ?? ''
  return { content, etag }
}

export async function fetchTemplates(): Promise<TemplateSummaryListResponse> {
  const r = await fetch('/api/templates')
  if (!r.ok) throw new FetchError(r.status, `GET /api/templates: ${r.status}`)
  return r.json()
}

export async function fetchTemplateDetail(name: string): Promise<TemplateDetail> {
  const r = await fetch(`/api/templates/${encodeURIComponent(name)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/templates/${name}: ${r.status}`)
  return r.json()
}

export async function fetchLifecycleClusters(): Promise<LifecycleCluster[]> {
  const r = await fetch('/api/lifecycle')
  if (!r.ok) throw new FetchError(r.status, `GET /api/lifecycle: ${r.status}`)
  const body: LifecycleListResponse = await r.json()
  return body.clusters
}

export async function fetchLifecycleCluster(slug: string): Promise<LifecycleCluster> {
  const r = await fetch(`/api/lifecycle/${encodeURIComponent(slug)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/lifecycle/${slug}: ${r.status}`)
  return r.json()
}
