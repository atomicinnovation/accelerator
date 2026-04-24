import type {
  DocType, DocTypeKey, DocsListResponse, IndexEntry,
  TemplateSummaryListResponse, TemplateDetail,
} from './types'

export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch('/api/types')
  if (!r.ok) throw new Error(`GET /api/types: ${r.status}`)
  return r.json()
}

export async function fetchDocs(type: DocTypeKey): Promise<IndexEntry[]> {
  const r = await fetch(`/api/docs?type=${encodeURIComponent(type)}`)
  if (!r.ok) throw new Error(`GET /api/docs?type=${type}: ${r.status}`)
  const body: DocsListResponse = await r.json()
  return body.docs
}

export async function fetchDocContent(relPath: string): Promise<{ content: string; etag: string }> {
  // Encode per-segment so filenames containing '#', '?', '%', or
  // non-ASCII are transmitted correctly. '/' separators between segments
  // stay literal since the server accepts them as path structure.
  const encodedPath = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/docs/${encodedPath}`)
  if (!r.ok) throw new Error(`GET /api/docs/${relPath}: ${r.status}`)
  const content = await r.text()
  const etag = r.headers.get('etag') ?? ''
  return { content, etag }
}

export async function fetchTemplates(): Promise<TemplateSummaryListResponse> {
  const r = await fetch('/api/templates')
  if (!r.ok) throw new Error(`GET /api/templates: ${r.status}`)
  return r.json()
}

export async function fetchTemplateDetail(name: string): Promise<TemplateDetail> {
  const r = await fetch(`/api/templates/${encodeURIComponent(name)}`)
  if (!r.ok) throw new Error(`GET /api/templates/${name}: ${r.status}`)
  return r.json()
}
