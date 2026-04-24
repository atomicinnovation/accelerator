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
})
