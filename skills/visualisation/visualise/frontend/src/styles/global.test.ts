import { describe, it, expect } from 'vitest'
import globalCss from './global.css?raw'

describe('global focus rings', () => {
  it('declares :focus-visible with an outline', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline:[^;]+;/)
  })

  it('declares an outline-offset for breathing room', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline-offset:[^;]+;/)
  })

  it('overrides the focus-ring colour under forced-colors mode', () => {
    expect(globalCss).toMatch(
      /@media\s*\(forced-colors:\s*active\)\s*\{[^}]*:focus-visible[^}]*outline-color:\s*Highlight/i,
    )
  })
})
