import { describe, it, expect } from 'vitest'
import { resolveCrumb } from './router'

describe('resolveCrumb()', () => {
  it('returns the static string when given a string crumb', () => {
    expect(resolveCrumb('Static', {})).toEqual({ crumb: 'Static' })
  })

  it('calls the resolver and returns its result', () => {
    expect(resolveCrumb(({ params }) => params.x, { x: 'foo' })).toEqual({ crumb: 'foo' })
  })

  it('invokes the resolver function (not just param spread)', () => {
    expect(
      resolveCrumb(({ params }) => params.x.toUpperCase(), { x: 'foo' }),
    ).toEqual({ crumb: 'FOO' })
  })
})
