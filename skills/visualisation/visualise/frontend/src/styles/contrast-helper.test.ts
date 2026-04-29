import { describe, it, expect } from 'vitest'
import { contrastRatio } from './contrast'

describe('contrastRatio helper (WebAIM reference values)', () => {
  it('black on white is ~21:1', () => {
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 0)
  })

  it('#777777 on white is ~4.48:1', () => {
    const ratio = contrastRatio('#777777', '#ffffff')
    expect(ratio).toBeGreaterThanOrEqual(4.47)
    expect(ratio).toBeLessThanOrEqual(4.49)
  })

  it('white on white is 1:1', () => {
    expect(contrastRatio('#ffffff', '#ffffff')).toBeCloseTo(1, 1)
  })

  it('#ff0000 on white is ~4:1', () => {
    const ratio = contrastRatio('#ff0000', '#ffffff')
    expect(ratio).toBeGreaterThanOrEqual(3.99)
    expect(ratio).toBeLessThanOrEqual(4.01)
  })
})
