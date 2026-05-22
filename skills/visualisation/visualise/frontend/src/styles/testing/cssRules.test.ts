import { describe, it, expect } from 'vitest'
import {
  parseFlatCssRules,
  assertSelectorColorIs,
  selectorOffset,
} from './cssRules'

describe('parseFlatCssRules', () => {
  it('parses a single flat rule', () => {
    const rules = parseFlatCssRules('.a { color: red; }')
    expect(rules).toHaveLength(1)
    expect(rules[0].selectors).toEqual(['.a'])
    expect(rules[0].body).toContain('color: red')
  })

  it('splits multi-selector lists', () => {
    const rules = parseFlatCssRules('.a, .b, .c { color: red; }')
    expect(rules).toHaveLength(1)
    expect(rules[0].selectors).toEqual(['.a', '.b', '.c'])
  })

  it('returns offsets in ascending order for source-order arguments', () => {
    const rules = parseFlatCssRules('.a { color: red; }\n.b { color: blue; }')
    expect(rules[0].offset).toBeLessThan(rules[1].offset)
  })

  it('ignores comments in selector blocks', () => {
    const rules = parseFlatCssRules('/* hello */ .a { color: red; }')
    expect(rules).toHaveLength(1)
    expect(rules[0].selectors).toEqual(['.a'])
  })

  it('throws on top-level @media', () => {
    expect(() =>
      parseFlatCssRules('@media (min-width: 0) { .a { color: red; } }'),
    ).toThrow(/@-rule/)
  })

  it('throws on top-level @supports', () => {
    expect(() =>
      parseFlatCssRules('@supports (display: grid) { .a { color: red; } }'),
    ).toThrow(/@-rule/)
  })
})

describe('assertSelectorColorIs', () => {
  it('passes when selector exactly matches a rule with the right colour', () => {
    expect(() =>
      assertSelectorColorIs('.hljs-comment { color: var(--tk-com); }', '.hljs-comment', 'tk-com'),
    ).not.toThrow()
  })

  it('passes for multi-selector grouping (both members match)', () => {
    const css = '.hljs-comment, .hljs-quote { color: var(--tk-com); }'
    expect(() =>
      assertSelectorColorIs(css, '.hljs-comment', 'tk-com'),
    ).not.toThrow()
    expect(() =>
      assertSelectorColorIs(css, '.hljs-quote', 'tk-com'),
    ).not.toThrow()
  })

  it('rejects compound selector suffix (.hljs-meta vs .hljs-meta.doctype)', () => {
    expect(() =>
      assertSelectorColorIs(
        '.hljs-meta.doctype { color: var(--tk-dhdr); }',
        '.hljs-meta',
        'tk-dhdr',
      ),
    ).toThrow(/no rule declares selector "\.hljs-meta" exactly/)
  })

  it('rejects substring sibling (.hljs-attr vs .hljs-attribute)', () => {
    expect(() =>
      assertSelectorColorIs(
        '.hljs-attribute { color: var(--tk-attr); }',
        '.hljs-attr',
        'tk-attr',
      ),
    ).toThrow(/no rule declares selector "\.hljs-attr" exactly/)
  })

  it('rejects border-color (property-boundary anchor)', () => {
    expect(() =>
      assertSelectorColorIs(
        '.x { border-color: var(--tk-com); }',
        '.x',
        'tk-com',
      ),
    ).toThrow(/no matching rule declares color/)
  })

  it('rejects background-color', () => {
    expect(() =>
      assertSelectorColorIs(
        '.x { background-color: var(--tk-com); }',
        '.x',
        'tk-com',
      ),
    ).toThrow(/no matching rule declares color/)
  })

  it('throws with matched bodies on colour-mismatch diagnostic', () => {
    expect(() =>
      assertSelectorColorIs(
        '.hljs-comment { color: var(--tk-str); }',
        '.hljs-comment',
        'tk-com',
      ),
    ).toThrow(/Inspected bodies/)
  })
})

describe('selectorOffset', () => {
  it('returns ascending offsets for source-order arguments', () => {
    const css = '.a { color: red; }\n.b { color: blue; }'
    const a = selectorOffset(css, '.a')
    const b = selectorOffset(css, '.b')
    expect(a).not.toBeNull()
    expect(b).not.toBeNull()
    expect(b!).toBeGreaterThan(a!)
  })

  it('returns null when selector is not present', () => {
    expect(selectorOffset('.a { color: red; }', '.b')).toBeNull()
  })

  it('returns the offset of the FIRST occurrence when multiple rules share the selector', () => {
    const css = '.a { color: red; }\n.a { color: blue; }'
    const first = css.indexOf('{')
    expect(selectorOffset(css, '.a')).toBe(first)
  })
})
