import type {
  DocType, DocTypeKey, DocsListResponse, IndexEntry,
  TemplateSummaryListResponse, TemplateDetail,
  LifecycleCluster, LifecycleListResponse, KanbanColumnKey,
  RelatedArtifactsResponse,
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

export class ConflictError extends FetchError {
  constructor(status: number, message: string, public readonly currentEtag: string) {
    super(status, message)
    this.name = 'ConflictError'
  }
}

export interface PatchResult {
  etag: string
}

export async function patchWorkItemFrontmatter(
  relPath: string,
  patch: { status: KanbanColumnKey },
  etag: string,
): Promise<PatchResult> {
  const encodedPath = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/docs/${encodedPath}/frontmatter`, {
    method: 'PATCH',
    headers: {
      'If-Match': `"${etag}"`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ patch }),
  })
  if (r.status === 412) {
    const body = (await r.json()) as { currentEtag: string }
    throw new ConflictError(412, `PATCH /api/docs/${relPath}/frontmatter: 412`, body.currentEtag)
  }
  if (!r.ok) {
    throw new FetchError(r.status, `PATCH /api/docs/${relPath}/frontmatter: ${r.status}`)
  }
  const rawEtag = r.headers.get('etag') ?? ''
  const parsedEtag =
    rawEtag.startsWith('"') && rawEtag.endsWith('"') ? rawEtag.slice(1, -1) : rawEtag
  return { etag: parsedEtag }
}

export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch('/api/types')
  if (!r.ok) throw new FetchError(r.status, `GET /api/types: ${r.status}`)
  const body: { types: DocType[] } = await r.json()
  return body.types
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

export async function fetchRelated(relPath: string): Promise<RelatedArtifactsResponse> {
  const encodedPath = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/related/${encodedPath}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/related/${relPath}: ${r.status}`)
  return r.json()
}
