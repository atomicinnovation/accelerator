// All fields use camelCase to match the server's
// `#[serde(rename_all = "camelCase")]` output.

export type DocTypeKey =
  | 'decisions' | 'work-items' | 'plans' | 'research'
  | 'plan-reviews' | 'pr-reviews' | 'work-item-reviews'
  | 'validations' | 'notes' | 'pr-descriptions' | 'design-gaps' | 'design-inventories'
  | 'templates'

/** Single source of truth for the DocTypeKey union at runtime. Drives both
 *  the `isDocTypeKey` type guard and the router's `parseParams` validators,
 *  so URL params are narrowed at the routing boundary rather than inside
 *  each view component. */
export const DOC_TYPE_KEYS: readonly DocTypeKey[] = [
  'decisions', 'work-items', 'plans', 'research',
  'plan-reviews', 'pr-reviews', 'work-item-reviews',
  'validations', 'notes', 'pr-descriptions', 'design-gaps', 'design-inventories',
  'templates',
] as const

/** Type guard: narrows a string to `DocTypeKey` when valid. */
export function isDocTypeKey(s: string): s is DocTypeKey {
  return (DOC_TYPE_KEYS as readonly string[]).includes(s)
}

/** Doc-type keys that are virtual (not backed by on-disk documents). Static
 *  mirror of the server's `virtual: true` flag on `DocType`. Keep in lock-step
 *  with the server-side `DocType.virtual` flag (see `RootLayout.tsx` useQuery).
 *  Used by `Glyph` to filter `DocTypeKey` down to the renderable subset. */
export const VIRTUAL_DOC_TYPE_KEYS = ['templates'] as const satisfies
  readonly DocTypeKey[]

/** The virtual `DocTypeKey` literal union — `'templates'` today, broadening
 *  automatically if `VIRTUAL_DOC_TYPE_KEYS` grows. */
export type VirtualDocTypeKey = (typeof VIRTUAL_DOC_TYPE_KEYS)[number]

/** Type guard: true for doc types backed by on-disk documents (i.e. not
 *  virtual). Narrows away the virtual keys so callers can rely on a real
 *  per-doc colour palette and fixture existing. */
export function isPhysicalDocTypeKey(
  key: DocTypeKey,
): key is Exclude<DocTypeKey, VirtualDocTypeKey> {
  return !(VIRTUAL_DOC_TYPE_KEYS as readonly DocTypeKey[]).includes(key)
}

/** Static, human-friendly labels for each `DocTypeKey`. Mirrors the
 *  server-emitted `DocType.label` field; used in dev-only routes (e.g.
 *  `/glyph-showcase`) and tests where a runtime `useQuery` is undesirable. */
export const DOC_TYPE_LABELS: Readonly<Record<DocTypeKey, string>> = {
  'decisions': 'Decisions',
  'work-items': 'Work items',
  'plans': 'Plans',
  'research': 'Research',
  'plan-reviews': 'Plan reviews',
  'pr-reviews': 'PR reviews',
  'work-item-reviews': 'Work item reviews',
  'validations': 'Validations',
  'notes': 'Notes',
  'pr-descriptions': 'PR descriptions',
  'design-gaps': 'Design gaps',
  'design-inventories': 'Design inventories',
  'templates': 'Templates',
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
  count?: number
}

export interface IndexEntry {
  type: DocTypeKey
  path: string
  relPath: string
  slug: string | null
  /** Filename-derived work-item ID (regex-extracted). Present for work-item
   *  entries whose filename matches the configured scan pattern; null otherwise. */
  workItemId: string | null
  title: string
  frontmatter: Record<string, unknown>
  frontmatterState: 'parsed' | 'absent' | 'malformed'
  workItemRefs: string[]
  mtimeMs: number
  size: number
  etag: string
  bodyPreview: string
  /** Cluster-level Completeness back-filled by the server. `null` for orphan
   *  entries (no cluster slug) — kanban cards switch to orphan rendering on
   *  this signal. Older servers that omit the field are normalised to `null`
   *  at the API client boundary. */
  completeness: Completeness | null
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
  /** Project-root-relative path of the config file (e.g.
   *  `.accelerator/config.md` or `.accelerator/config.local.md`) in which
   *  the config-override for this template is declared. Only meaningful
   *  for the `config-override` tier; absent for the user-override /
   *  plugin-default tiers and for config-override tiers whose source
   *  file is unknown to the launcher. */
  configSource?: string
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
  sha256?: string
}

export type ActionKind = 'created' | 'edited' | 'deleted'

export interface SseDocChangedEvent {
  type: 'doc-changed'
  action: ActionKind
  docType: DocTypeKey
  path: string
  etag?: string
  timestamp: string
}

export interface ActivityEvent {
  action: ActionKind
  docType: DocTypeKey
  path: string
  timestamp: string
}

export interface ActivityResponse {
  events: ActivityEvent[]
}

export interface SseDocInvalidEvent {
  type: 'doc-invalid'
  docType: DocTypeKey
  path: string
}

export interface SseTemplateChangedEvent {
  type: 'template-changed'
  template: string
  sha256?: string
  timestamp: string
}

export type SseEvent =
  | SseDocChangedEvent
  | SseDocInvalidEvent
  | SseTemplateChangedEvent

export interface Completeness {
  hasWorkItem: boolean
  hasResearch: boolean
  hasPlan: boolean
  hasPlanReview: boolean
  hasValidation: boolean
  hasPrDescription: boolean
  hasPrReview: boolean
  hasDecision: boolean
  hasNotes: boolean
  hasDesignInventory: boolean
  hasDesignGap: boolean
  /** Kebab-case `DocTypeKey` strings for every stage whose corresponding
   *  `has*` flag is true, in canonical order (workflow then long-tail).
   *  See `LIFECYCLE_PIPELINE_STEPS` for the canonical ordering. */
  present: string[]
}

export interface LifecycleCluster {
  slug: string
  title: string
  entries: IndexEntry[]
  completeness: Completeness
  lastChangedMs: number
}

export interface LifecycleListResponse {
  clusters: LifecycleCluster[]
}

export interface RelatedArtifactsResponse {
  inferredCluster: IndexEntry[]
  declaredOutbound: IndexEntry[]
  declaredInbound: IndexEntry[]
}

type PipelineStepKey =
  | 'hasWorkItem' | 'hasResearch' | 'hasPlan' | 'hasPlanReview'
  | 'hasValidation' | 'hasPrDescription' | 'hasPrReview' | 'hasDecision'
  | 'hasNotes' | 'hasDesignInventory' | 'hasDesignGap'

export const LIFECYCLE_PIPELINE_STEPS: ReadonlyArray<{
  key: PipelineStepKey
  docType: DocTypeKey
  label: string
  placeholder: string
  longTail?: boolean
}> = [
  { key: 'hasWorkItem', docType: 'work-items', label: 'Work item', placeholder: 'no work item yet' },
  { key: 'hasResearch', docType: 'research', label: 'Research', placeholder: 'no research yet' },
  { key: 'hasPlan', docType: 'plans', label: 'Plan', placeholder: 'no plan yet' },
  { key: 'hasPlanReview', docType: 'plan-reviews', label: 'Plan review', placeholder: 'no plan review yet' },
  { key: 'hasValidation', docType: 'validations', label: 'Validation', placeholder: 'no validation yet' },
  { key: 'hasPrDescription', docType: 'pr-descriptions', label: 'PR descriptions', placeholder: 'no PR description yet' },
  { key: 'hasPrReview', docType: 'pr-reviews', label: 'PR review', placeholder: 'no PR review yet' },
  { key: 'hasDecision', docType: 'decisions', label: 'Decision', placeholder: 'no decision yet' },
  { key: 'hasNotes', docType: 'notes', label: 'Notes', placeholder: 'no notes yet', longTail: true },
  {
    key: 'hasDesignInventory',
    docType: 'design-inventories',
    label: 'Design inventory',
    placeholder: 'no design inventory yet',
    longTail: true
  },
  {
    key: 'hasDesignGap',
    docType: 'design-gaps',
    label: 'Design gap',
    placeholder: 'no design gap yet',
    longTail: true
  },
] as const

export const WORKFLOW_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(
  s => !s.longTail,
)

export const LONG_TAIL_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(
  s => s.longTail,
)

export interface LibraryStructureResponse {
  phases: LibraryPhase[]
  templates: LibraryDocType
}

export interface LibraryPhase {
  id: string
  label: string
  docTypes: LibraryDocType[]
}

export interface LibraryDocType {
  id: DocTypeKey
  label: string
  /** Total entries (selection-unaware). */
  count: number
  /** Entries matching the active selection. */
  filteredCount: number
  latest: LatestPreview | null
  filterFacets: LibraryFacet[]
}

export interface LatestPreview {
  title: string
  slug: string | null
  modifiedAt: number
}

export interface LibraryFacet {
  /** camelCase facet id: "status" | "clusterSlug" | "project". */
  id: string
  label: string
  options: LibraryFacetOption[]
}

export interface LibraryFacetOption {
  id: string
  label: string
  count: number
}

/** Selection state for one or more doc types. Keyed by doc type, then by
 *  facet id; values are arrays of selected option ids (OR within a facet,
 *  AND across facets). Empty arrays / missing keys ⇒ no filter for that
 *  facet. */
export type LibrarySelection = Partial<Record<DocTypeKey, LibrarySelectionPerType>>

/** Per-doc-type selection slice — what `FilterPill` consumes. */
export type LibrarySelectionPerType = Record<string, string[]>

export interface KanbanColumn {
  key: string
  label: string
}

export const OTHER_COLUMN_KEY = 'other' as const

export const OTHER_COLUMN: { key: typeof OTHER_COLUMN_KEY; label: string } = {
  key: OTHER_COLUMN_KEY,
  label: 'Other',
}
