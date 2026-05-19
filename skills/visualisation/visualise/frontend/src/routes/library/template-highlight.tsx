import type { ReactElement } from 'react'

/**
 * Tiny template-specific syntax highlighter, ported from the design
 * prototype's `syntaxHighlight()` in view-templates.jsx. Recognises:
 *
 * - `---` frontmatter delimiters
 * - `key: value` pairs inside the frontmatter block
 * - `{{variable}}` placeholders (both in frontmatter values and in body)
 *
 * Returns one `<div>` per source line so the resulting layout matches
 * the prototype's preview pane (`white-space: pre` plus per-line rows).
 * The output is plain `<span>`-tagged text — no `dangerouslySetInnerHTML`,
 * no third-party highlight engine — which keeps the result deterministic
 * and predictable for template content.
 */
export function highlightTemplate(src: string): ReactElement[] {
  const out: ReactElement[] = []
  const lines = src.split('\n')
  let inFrontmatter = false
  lines.forEach((line, i) => {
    if (line === '---') {
      inFrontmatter = !inFrontmatter
      out.push(
        <div key={i}>
          <span className="fm-delim">---</span>
        </div>,
      )
      return
    }
    if (inFrontmatter) {
      const m = line.match(/^([a-zA-Z_][\w-]*):(.*)$/)
      if (m) {
        const key = m[1]
        const value = m[2]
        out.push(
          <div key={i}>
            <span className="fm-key">{key}</span>
            <span className="fm-delim">:</span>
            {splitOnVars(value).map((part, j) =>
              part.kind === 'var' ? (
                <span key={j} className="tpl-var">{part.text}</span>
              ) : (
                <span key={j}>{part.text}</span>
              ),
            )}
          </div>,
        )
        return
      }
    }
    // Body (or non-key frontmatter line): highlight {{vars}} only.
    out.push(
      <div key={i}>
        {splitOnVars(line).map((part, j) =>
          part.kind === 'var' ? (
            <span key={j} className="tpl-var">{part.text}</span>
          ) : (
            <span key={j}>{part.text}</span>
          ),
        )}
      </div>,
    )
  })
  return out
}

interface Part {
  kind: 'text' | 'var'
  text: string
}

function splitOnVars(s: string): Part[] {
  const out: Part[] = []
  const re = /\{\{[^}]+\}\}/g
  let lastIndex = 0
  for (let m = re.exec(s); m !== null; m = re.exec(s)) {
    if (m.index > lastIndex) {
      out.push({ kind: 'text', text: s.slice(lastIndex, m.index) })
    }
    out.push({ kind: 'var', text: m[0] })
    lastIndex = m.index + m[0].length
  }
  if (lastIndex < s.length) {
    out.push({ kind: 'text', text: s.slice(lastIndex) })
  }
  return out
}
