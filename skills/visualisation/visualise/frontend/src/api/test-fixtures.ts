import type { Completeness, IndexEntry } from './types'

/** Test-only `Completeness` factory. Defaults to all-false with empty
 *  `present`; callers override only what they care about via
 *  `Partial<Completeness>`. */
export function makeCompleteness(
  overrides: Partial<Completeness> = {},
): Completeness {
  return {
    hasWorkItem: false,
    hasResearch: false,
    hasPlan: false,
    hasPlanReview: false,
    hasValidation: false,
    hasPrDescription: false,
    hasPrReview: false,
    hasDecision: false,
    hasNotes: false,
    hasDesignInventory: false,
    hasDesignGap: false,
    present: [],
    ...overrides,
  }
}

/** Test-only `IndexEntry` factory. New required fields default here
 *  in one place; callers override only what they care about via
 *  `Partial<IndexEntry>`. */
export function makeIndexEntry(overrides: Partial<IndexEntry> = {}): IndexEntry {
  return {
    type: 'plans',
    path: '/x/foo.md',
    relPath: 'foo.md',
    slug: 'foo',
    workItemId: null,
    title: 'Foo',
    frontmatter: {},
    frontmatterState: 'parsed',
    workItemRefs: [],
    mtimeMs: 0,
    size: 0,
    etag: 'sha256-x',
    bodyPreview: '',
    completeness: null,
    ...overrides,
  }
}
