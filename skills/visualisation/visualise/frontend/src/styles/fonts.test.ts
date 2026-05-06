import { describe, it, expect } from 'vitest'
import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
import globalCss from './global.css?raw'
import indexHtml from '../../index.html?raw'
import { TYPOGRAPHY_TOKENS } from './tokens'

// `frontend/package.json` declares `"type": "module"`, so `__dirname` is
// not defined. Vite's `import.meta.glob` deliberately excludes `public/`
// from the module graph (those files are served as static assets, not
// processed by the bundler), so a glob over `/public/fonts/*.woff2` would
// silently return an empty record and pass-vacuously. Vitest runs with
// process.cwd() set to the frontend root (where package.json lives), so
// we resolve the fonts directory relative to cwd rather than via
// import.meta.url (which jsdom transforms to a non-file:// URL).
const fontsDir = resolve(process.cwd(), 'public/fonts') + '/'
const fontFiles = readdirSync(fontsDir)

const EXPECTED_FONT_FILES = [
  'Sora-SemiBold.woff2',
  'Sora-Bold.woff2',
  'Inter-Regular.woff2',
  'Inter-Medium.woff2',
  'Inter-SemiBold.woff2',
  'Inter-Bold.woff2',
  'FiraCode-Regular.woff2',
  'FiraCode-Medium.woff2',
]

describe('AC2: self-hosted font wiring', () => {
  for (const file of EXPECTED_FONT_FILES) {
    it(`public/fonts/${file} exists`, () => {
      expect(fontFiles).toContain(file)
    })
  }

  for (const family of ['Sora', 'Inter', 'Fira Code']) {
    it(`global.css declares @font-face for "${family}"`, () => {
      expect(globalCss).toMatch(
        new RegExp(`@font-face\\s*\\{[^}]*font-family:\\s*"${family}"`, 'i'),
      )
    })
  }

  // AC2's second clause: each family is referenced *via a typography token*.
  for (const [family, tokenKey] of [
    ['Sora', 'ac-font-display'],
    ['Inter', 'ac-font-body'],
    ['Fira Code', 'ac-font-mono'],
  ] as const) {
    it(`TYPOGRAPHY_TOKENS["${tokenKey}"] references "${family}"`, () => {
      expect(TYPOGRAPHY_TOKENS[tokenKey]).toMatch(new RegExp(`"${family}"`))
    })
  }

  it('global.css uses font-display: swap on every @font-face', () => {
    const blocks = globalCss.match(/@font-face\s*\{[^}]+\}/g) ?? []
    expect(blocks.length).toBeGreaterThanOrEqual(8)
    for (const block of blocks) {
      expect(block).toMatch(/font-display:\s*swap/)
    }
  })

  it('index.html preloads at least one critical-path font', () => {
    expect(indexHtml).toMatch(
      /<link[^>]*rel="preload"[^>]*as="font"[^>]*type="font\/woff2"[^>]*\/fonts\/[^"]+\.woff2/,
    )
  })

  it('no third-party font origins are referenced', () => {
    expect(indexHtml).not.toMatch(/fonts\.googleapis\.com|fonts\.gstatic\.com/)
    expect(globalCss).not.toMatch(/fonts\.googleapis\.com|fonts\.gstatic\.com/)
  })

  // Supply-chain integrity: compute SHA-256 of each woff2 file and compare
  // against `public/fonts/SHA256SUMS`. The test runs unconditionally —
  // missing SUMS is a hard failure, not a silent skip.
  it('woff2 binary checksums match public/fonts/SHA256SUMS', () => {
    const sumsPath = resolve(process.cwd(), 'public/fonts/SHA256SUMS')
    const sums = readFileSync(sumsPath, 'utf8')
    const expected = new Map<string, string>()
    for (const line of sums.split('\n').filter(Boolean)) {
      const [hex, file] = line.trim().split(/\s+/)
      expected.set(file, hex)
    }
    for (const file of EXPECTED_FONT_FILES) {
      const bytes = readFileSync(`${fontsDir}${file}`)
      const actual = createHash('sha256').update(bytes).digest('hex')
      expect(actual, `checksum mismatch for ${file}`).toBe(expected.get(file))
    }
  })
})
