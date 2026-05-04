// All fields use camelCase to match the server's
// `#[serde(rename_all = "camelCase")]` output.

export type DocTypeKey =
  | 'decisions' | 'work-items' | 'plans' | 'research'
  | 'plan-reviews' | 'pr-reviews' | 'work-item-reviews'
  | 'validations' | 'notes' | 'prs' | 'templates'

/** Single source of truth for the DocTypeKey union at runtime. Drives both
 *  the `isDocTypeKey` type guard and the router's `parseParams` validators,
 *  so URL params are narrowed at the routing boundary rather than inside
 *  each view component. */
export const DOC_TYPE_KEYS: readonly DocTypeKey[] = [
  'decisions', 'work-items', 'plans', 'research',
  'plan-reviews', 'pr-reviews', 'work-item-reviews',
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
  workItemRefs: string[]
  mtimeMs: number
  size: number
  etag: string
  bodyPreview: string
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

export interface Completeness {
  hasWorkItem: boolean
  hasResearch: boolean
  hasPlan: boolean
  hasPlanReview: boolean
  hasValidation: boolean
  hasPr: boolean
  hasPrReview: boolean
  hasDecision: boolean
  hasNotes: boolean
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
  | 'hasValidation' | 'hasPr' | 'hasPrReview' | 'hasDecision'
  | 'hasNotes'

export const LIFECYCLE_PIPELINE_STEPS: ReadonlyArray<{
  key: PipelineStepKey
  docType: DocTypeKey
  label: string
  placeholder: string
  longTail?: boolean
}> = [
  { key: 'hasWorkItem',   docType: 'work-items',   label: 'Work item',   placeholder: 'no work item yet' },
  { key: 'hasResearch',   docType: 'research',     label: 'Research',    placeholder: 'no research yet' },
  { key: 'hasPlan',       docType: 'plans',        label: 'Plan',        placeholder: 'no plan yet' },
  { key: 'hasPlanReview', docType: 'plan-reviews', label: 'Plan review', placeholder: 'no plan review yet' },
  { key: 'hasValidation', docType: 'validations',  label: 'Validation',  placeholder: 'no validation yet' },
  { key: 'hasPr',         docType: 'prs',          label: 'PR',          placeholder: 'no PR yet' },
  { key: 'hasPrReview',   docType: 'pr-reviews',   label: 'PR review',   placeholder: 'no PR review yet' },
  { key: 'hasDecision',   docType: 'decisions',    label: 'Decision',    placeholder: 'no decision yet' },
  { key: 'hasNotes',      docType: 'notes',        label: 'Notes',       placeholder: 'no notes yet', longTail: true },
] as const

export const WORKFLOW_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(
  s => !s.longTail,
)

export const LONG_TAIL_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(
  s => s.longTail,
)

export type KanbanColumnKey = 'todo' | 'in-progress' | 'done'

export const OTHER_COLUMN_KEY = 'other' as const
export type KanbanGroupKey = KanbanColumnKey | typeof OTHER_COLUMN_KEY

export const STATUS_COLUMNS: ReadonlyArray<{
  key: KanbanColumnKey
  label: string
}> = [
  { key: 'todo',        label: 'Todo' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'done',        label: 'Done' },
] as const

export const OTHER_COLUMN: { key: typeof OTHER_COLUMN_KEY; label: string } = {
  key: OTHER_COLUMN_KEY,
  label: 'Other',
}
