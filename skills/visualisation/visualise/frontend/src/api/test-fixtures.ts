import type { IndexEntry } from './types'

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
    ...overrides,
  }
}
