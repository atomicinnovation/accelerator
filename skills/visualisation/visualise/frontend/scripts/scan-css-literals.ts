// Walks src/ for *.module.css and *.global.css; emits a TypeScript snippet of
// { file, literal, count, kind: 'to-migrate', reason: 'TODO' } entries
// suitable for pasting into the EXCEPTIONS constant in migration.test.ts.
//
// Usage (from frontend root):
//   npx tsx scripts/scan-css-literals.ts
//   npx tsx scripts/scan-css-literals.ts --irreducible-only   # only entries marked irreducible
//
// Re-run after each migration wave to confirm EXCEPTIONS is shrinking.
// The "irreducible-only" flag produces the PR-description excerpt.

import { readdirSync, readFileSync } from 'node:fs'
import { join, relative } from 'node:path'

const SRC_DIR = join(process.cwd(), 'src')

// Same patterns as migration.test.ts — keep in sync.
const HEX_RE = /#[0-9a-fA-F]{3,8}\b/g
const PX_REM_EM_RE = /\b(?!0(?:px|rem|em)\b)\d+(?:\.\d+)?(?:px|rem|em)\b/g

function walk(dir: string, files: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name)
    if (entry.isDirectory()) {
      walk(full, files)
    } else if (entry.isFile() && (entry.name.endsWith('.module.css') || entry.name.endsWith('.global.css'))) {
      files.push(full)
    }
  }
  return files
}

function countMatches(css: string, re: RegExp): Map<string, number> {
  const counts = new Map<string, number>()
  for (const m of css.matchAll(re)) {
    const lit = m[0]
    counts.set(lit, (counts.get(lit) ?? 0) + 1)
  }
  return counts
}

interface Entry {
  file: string
  literal: string
  count: number
  kind: 'to-migrate' | 'irreducible'
  reason: string
}

const irreducibleOnly = process.argv.includes('--irreducible-only')

const entries: Entry[] = []

for (const absPath of walk(SRC_DIR).sort()) {
  const srcRel = relative(SRC_DIR, absPath)
  const css = readFileSync(absPath, 'utf8')

  const hexCounts = countMatches(css, new RegExp(HEX_RE.source, 'g'))
  const pxCounts = countMatches(css, new RegExp(PX_REM_EM_RE.source, 'g'))

  for (const [lit, count] of [...hexCounts, ...pxCounts]) {
    entries.push({ file: srcRel, literal: lit, count, kind: 'to-migrate', reason: 'TODO' })
  }
}

if (irreducibleOnly) {
  // Filter to only entries already marked irreducible in migration.test.ts.
  // (This flag is more useful after hand-editing kinds; here it's a no-op
  // since all emitted entries are 'to-migrate'.)
  const filtered = entries.filter((e) => e.kind === 'irreducible')
  console.log(JSON.stringify(filtered, null, 2))
} else {
  // Emit as a TypeScript snippet ready to paste into EXCEPTIONS.
  const lines: string[] = []
  let currentFile = ''
  for (const e of entries) {
    if (e.file !== currentFile) {
      if (currentFile) lines.push('')
      lines.push(`  // ${e.file}`)
      currentFile = e.file
    }
    lines.push(
      `  { file: ${JSON.stringify(e.file)}, literal: ${JSON.stringify(e.literal)}, count: ${e.count}, kind: 'to-migrate', reason: 'TODO' },`,
    )
  }
  console.log(lines.join('\n'))
  console.log(`\n// Total entries: ${entries.length}`)
}
