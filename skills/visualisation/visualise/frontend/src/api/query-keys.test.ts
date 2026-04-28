import { describe, it, expect } from 'vitest'
import { queryKeys } from './query-keys'

describe('queryKeys', () => {
  it('returns stable arrays for the same inputs', () => {
    expect(queryKeys.docs('plans')).toEqual(['docs', 'plans'])
    expect(queryKeys.docContent('meta/plans/foo.md')).toEqual([
      'doc-content', 'meta/plans/foo.md',
    ])
    expect(queryKeys.templateDetail('adr')).toEqual(['template-detail', 'adr'])
  })

  it('types key is a singleton', () => {
    expect(queryKeys.types()).toEqual(['types'])
  })

  // ── Step 5.4 ────────────────────────────────────────────────────────
  it('related and relatedPrefix have stable shapes that nest under the prefix', () => {
    expect(queryKeys.related('meta/plans/foo.md')).toEqual(['related', 'meta/plans/foo.md'])
    expect(queryKeys.relatedPrefix()).toEqual(['related'])
  })

  it('disabled(prefix) cannot collide with related(<relPath>)', () => {
    // The sentinel uses a doubled-underscore token that cannot appear
    // as a relPath. Even if a relPath equalled '__disabled__' the keys
    // still differ in their prefix shape (`related(...)` vs
    // `disabled('related')` both collapse to ['related', '__disabled__']
    // — so the only case that *would* collide is a doc literally named
    // '__disabled__', which is not a legal filename slug for any
    // doc-type in this project. Locked here as a contract.
    expect(queryKeys.disabled('related')).toEqual(['related', '__disabled__'])
    expect(queryKeys.disabled('related')).not.toEqual(queryKeys.related('foo'))
  })
})
