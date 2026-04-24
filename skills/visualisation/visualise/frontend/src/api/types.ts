// All fields use camelCase to match the server's
// `#[serde(rename_all = "camelCase")]` output.

export type DocTypeKey =
  | 'decisions' | 'tickets' | 'plans' | 'research'
  | 'plan-reviews' | 'pr-reviews'
  | 'validations' | 'notes' | 'prs' | 'templates'

/** Single source of truth for the DocTypeKey union at runtime. Drives both
 *  the `isDocTypeKey` type guard and the router's `parseParams` validators,
 *  so URL params are narrowed at the routing boundary rather than inside
 *  each view component. */
export const DOC_TYPE_KEYS: readonly DocTypeKey[] = [
  'decisions', 'tickets', 'plans', 'research',
  'plan-reviews', 'pr-reviews',
  'validations', 'notes', 'prs', 'templates',
] as const

/** Type guard: narrows a string to `DocTypeKey` when valid. */
export function isDocTypeKey(s: string): s is DocTypeKey {
  return (DOC_TYPE_KEYS as readonly string[]).includes(s)
}

export interface DocType {
  key: DocTypeKey
  label: string
  dirPath: string | null
  inLifecycle: boolean
  inKanban: boolean
  // Required: the server always emits this field (see Step 1b). Templates
  // and any future virtual/derived types set `virtual: true`; real
  // document types set `virtual: false`. Sidebar partitions on this flag.
  virtual: boolean
}

export interface IndexEntry {
  type: DocTypeKey
  path: string
  relPath: string
  slug: string | null
  title: string
  frontmatter: Record<string, unknown>
  frontmatterState: 'parsed' | 'absent' | 'malformed'
  ticket: string | null
  mtimeMs: number
  size: number
  etag: string
}

export interface DocsListResponse {
  docs: IndexEntry[]
}

export type TemplateTierSource = 'config-override' | 'user-override' | 'plugin-default'

export interface TemplateTier {
  source: TemplateTierSource
  path: string
  present: boolean
  active: boolean
  content?: string
  etag?: string
}

export interface TemplateSummary {
  name: string
  tiers: TemplateTier[]
  activeTier: TemplateTierSource
}

export interface TemplateSummaryListResponse {
  templates: TemplateSummary[]
}

export interface TemplateDetail {
  name: string
  tiers: TemplateTier[]
  activeTier: TemplateTierSource
}

export interface SseDocChangedEvent {
  type: 'doc-changed'
  docType: DocTypeKey
  path: string
  etag?: string
}

export interface SseDocInvalidEvent {
  type: 'doc-invalid'
  docType: DocTypeKey
  path: string
}

export type SseEvent = SseDocChangedEvent | SseDocInvalidEvent
