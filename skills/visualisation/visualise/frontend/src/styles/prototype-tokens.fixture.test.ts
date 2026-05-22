import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import prototypeTokens from './fixtures/prototype-tokens.json'

// Drift detector: the committed fixture must remain byte-equivalent
// (case- and whitespace-normalised) to the prototype's .ac-codeblock
// declaration block. If the prototype changes a colour, this spec
// surfaces the drift instead of silently accepting it. Four `..`
// walks from `skills/visualisation/visualise/frontend/` reach the
// repo root; mirrors the cwd-relative pattern in fonts.test.ts.
const PROTOTYPE_PATH = resolve(
  process.cwd(),
  '..', '..', '..', '..',
  'meta', 'research', 'design-inventories',
  '2026-05-21-015231-claude-design-prototype',
  'prototype-standalone.html',
)
const source = readFileSync(PROTOTYPE_PATH, 'utf-8')

// Extract the `.ac-codeblock { ... }` block via brace-balanced
// scanning so nested rules (if any) do not truncate the match.
function extractAcCodeblockBlock(html: string): string {
  const sel = '.ac-codeblock'
  let i = 0
  while (i < html.length) {
    const idx = html.indexOf(sel, i)
    if (idx === -1) break
    // The matched substring must be followed by whitespace then `{`
    // (not by `__head` or another suffix).
    const after = html.slice(idx + sel.length)
    const m = /^\s*\{/.exec(after)
    if (m) {
      const open = idx + sel.length + m.index + m[0].length - 1
      let depth = 1
      for (let j = open + 1; j < html.length; j++) {
        if (html[j] === '{') depth++
        else if (html[j] === '}') {
          depth--
          if (depth === 0) return html.slice(open + 1, j)
        }
      }
    }
    i = idx + sel.length
  }
  throw new Error('extractAcCodeblockBlock: could not locate .ac-codeblock rule')
}

// Normalise a CSS value for comparison: strip ALL whitespace and
// lowercase. So `rgba(255,255,255,0.07)` and `rgba(255, 255, 255, 0.07)`
// compare equal, as do `#0E1320` and `#0e1320`.
const canonical = (v: string): string => v.toLowerCase().replace(/\s+/g, '')

// Parse the block body into a flat name→value map for the --code-*
// and --tk-* tokens only.
function declarationsOf(block: string): Map<string, string> {
  const out = new Map<string, string>()
  // The block contains `\n` literally in the embedded HTML string —
  // treat both real newlines and escaped sequences as separators.
  const normalised = block.replace(/\\n/g, '\n')
  for (const m of normalised.matchAll(/--((?:code|tk)-[\w-]+):\s*([^;]+);/g)) {
    out.set(`--${m[1]}`, m[2].trim())
  }
  return out
}

const block = extractAcCodeblockBlock(source)
const protoMap = declarationsOf(block)
const fixtureMap = new Map<string, string>(
  Object.entries(prototypeTokens) as ReadonlyArray<[string, string]>,
)

describe('prototype-tokens.json ↔ prototype-standalone.html drift detector', () => {
  it('every prototype token is captured in the fixture', () => {
    const missing: string[] = []
    for (const name of protoMap.keys()) {
      if (!fixtureMap.has(name)) missing.push(name)
    }
    expect(missing).toEqual([])
  })

  it('fixture introduces no token absent from the prototype', () => {
    const extra: string[] = []
    for (const name of fixtureMap.keys()) {
      if (!protoMap.has(name)) extra.push(name)
    }
    expect(extra).toEqual([])
  })

  for (const [name, value] of fixtureMap) {
    it(`${name}: fixture value matches prototype source`, () => {
      const proto = protoMap.get(name)
      expect(proto, `prototype source missing ${name}`).toBeDefined()
      expect(canonical(value)).toBe(canonical(proto!))
    })
  }
})
