import { describe, it, expect } from 'vitest'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { BOOT_SCRIPT_SOURCE } from './boot-theme'

const distHtmlPath = resolve(process.cwd(), 'dist/index.html')

function stripLeadingCommentsAndWhitespace(s: string): string {
  let out = s
  while (true) {
    const t = out.replace(/^\s+/, '')
    if (t.startsWith('<!--')) {
      const end = t.indexOf('-->')
      if (end === -1) return t
      out = t.slice(end + 3)
      continue
    }
    return t
  }
}

describe.skipIf(!existsSync(distHtmlPath))(
  'dist/index.html boot script structure',
  () => {
    const html = existsSync(distHtmlPath) ? readFileSync(distHtmlPath, 'utf8') : ''

    it('the first <head> child is a classic <script> tag', () => {
      const headBody = /<head[^>]*>([\s\S]*?)<\/head>/.exec(html)?.[1] ?? ''
      const cleaned = stripLeadingCommentsAndWhitespace(headBody)
      const firstTag = /<\s*([a-zA-Z][\w-]*)/.exec(cleaned)?.[1]
      expect(firstTag).toBe('script')
      const firstScript = /<script[^>]*>/.exec(cleaned)?.[0] ?? ''
      expect(firstScript).not.toMatch(/type\s*=\s*["']module["']/)
      expect(firstScript).not.toMatch(/\bdefer\b/)
      expect(firstScript).not.toMatch(/\basync\b/)
    })

    it('the boot script precedes any <link rel="stylesheet">', () => {
      const head = /<head[^>]*>([\s\S]*?)<\/head>/.exec(html)?.[1] ?? ''
      const scriptIdx = head.indexOf('<script')
      const linkIdx = head.search(/<link[^>]*rel\s*=\s*["']stylesheet["']/)
      expect(scriptIdx).toBeGreaterThanOrEqual(0)
      if (linkIdx !== -1) {
        expect(scriptIdx).toBeLessThan(linkIdx)
      }
    })

    it('the inlined script body equals BOOT_SCRIPT_SOURCE', () => {
      expect(html).toContain(BOOT_SCRIPT_SOURCE)
    })
  },
)
