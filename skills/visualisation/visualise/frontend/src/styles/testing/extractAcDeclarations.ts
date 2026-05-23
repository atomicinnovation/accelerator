// Test-only helper. Consumers: global.test.ts.
//
// Do not import from production code.

import { extractBlockBody } from './cssBlocks'

export type AcBlockTag = 'root' | 'data-dark' | 'media-dark'

export interface AcDeclaration {
  name: string
  value: string
  block: AcBlockTag
}

const BLOCK_OPENERS: ReadonlyArray<[AcBlockTag, RegExp]> = [
  // `:root` requires whitespace before `{` so `:root:not(...)` inside
  // the @media mirror block does NOT match here.
  ['root',       /(?:^|\n)\s*:root\s*\{/],
  ['data-dark',  /\[data-theme="dark"\]\s*\{/],
  ['media-dark', /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/],
]

/**
 * Returns one entry per `--ac-*` declaration across the three
 * theme-related blocks (`:root`, `[data-theme="dark"]`, and the
 * `@media (prefers-color-scheme: dark)` mirror), tagged with the
 * originating block. Skips non-`--ac-*` declarations.
 */
export function extractAllAcDeclarations(css: string): AcDeclaration[] {
  const result: AcDeclaration[] = []
  for (const [tag, opener] of BLOCK_OPENERS) {
    const match = opener.exec(css)
    if (!match) continue
    const body = extractBlockBody(css, match.index)
    if (body === undefined) continue
    const declRe = /--(ac-[\w-]+):\s*([^;]+);/g
    for (const m of body.matchAll(declRe)) {
      result.push({ name: m[1], value: m[2].trim(), block: tag })
    }
  }
  return result
}
