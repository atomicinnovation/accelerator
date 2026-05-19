import { useEffect, useRef } from 'react'
import hljs from 'highlight.js/lib/core'
import markdown from 'highlight.js/lib/languages/markdown'
import yaml from 'highlight.js/lib/languages/yaml'

// Register the two sub-grammars we need for templates. Both languages
// are already available via the `highlight.js` package the project
// already depends on (used by rehype-highlight in MarkdownRenderer).
// Registering them on the core engine keeps the bundle hit minimal —
// we never load the full 190-language pack just to render templates.
hljs.registerLanguage('markdown', markdown)
hljs.registerLanguage('yaml', yaml)

/**
 * Renders a template file with semantic highlighting:
 *
 * - Leading YAML frontmatter block (between two `---` fences) is
 *   highlighted with the YAML grammar.
 * - The body is highlighted with the markdown grammar.
 * - Template variables `{{var}}` are wrapped in a dedicated
 *   `template-variable` token span on top of the underlying highlight
 *   so we can style them distinctly from regular markdown.
 *
 * The output is wrapped per-line in `<div class="tpl-line">` so empty
 * lines preserve their height (without this the lines collapse and the
 * preview becomes visually compressed). `white-space: pre` on the
 * container preserves intra-line whitespace.
 */
export function TemplateHighlight({ content }: { content: string }) {
  const ref = useRef<HTMLPreElement | null>(null)

  useEffect(() => {
    if (!ref.current) return
    ref.current.innerHTML = renderHighlightedTemplate(content)
  }, [content])

  return (
    <pre
      ref={ref}
      className="tpl-highlight hljs"
      data-testid="template-preview-code"
    />
  )
}

/** Pure, testable HTML renderer. Exposed for unit tests. */
export function renderHighlightedTemplate(content: string): string {
  // 1. Split frontmatter from body so each chunk gets the right grammar.
  const { frontmatterRaw, frontmatterHtml, body } = splitFrontmatter(content)

  let bodyHtml = body.length > 0
    ? hljs.highlight(body, { language: 'markdown', ignoreIllegals: true }).value
    : ''

  // 2. Wrap template variables in both the frontmatter and the body.
  //    Runs AFTER hljs so the wrapper sees the highlighted (already
  //    HTML-escaped) source. The inner contents are passed through
  //    verbatim — escapeHtml() here would double-escape what hljs
  //    already escaped. Anything matched by `\{\{…\}\}` is guaranteed
  //    not to span an hljs-emitted `<span …>` boundary in practice,
  //    because neither the YAML nor the markdown grammar tokenises
  //    `{{…}}` patterns.
  const wrap = (s: string) =>
    s.replace(
      /\{\{([^}]+)\}\}/g,
      (_m, inner: string) =>
        `<span class="hljs-template-variable">{{${inner}}}</span>`,
    )

  const fmWrapped = wrap(frontmatterHtml)
  const bodyWrapped = wrap(bodyHtml)

  // 3. Concatenate, preserving the original number of lines (so a
  //    single trailing newline between frontmatter and body stays
  //    visible).
  const combined = frontmatterRaw.length > 0
    ? `${fmWrapped}\n${bodyWrapped}`
    : bodyWrapped

  // 4. Wrap each line in `.tpl-line` so empty lines keep their height
  //    via the `min-height: 1em` CSS rule. We split on `\n` AFTER
  //    highlighting because hljs's tokens never span line boundaries
  //    for the grammars we use here, so a naive split is safe — the
  //    only thing we need to balance is open/close `<span>` tags
  //    spanning lines, which hljs never emits.
  const lines = combined.split('\n')
  return lines
    .map((l) => `<div class="tpl-line">${l.length === 0 ? '&nbsp;' : l}</div>`)
    .join('')
}

function splitFrontmatter(content: string): {
  frontmatterRaw: string
  frontmatterHtml: string
  body: string
} {
  // Match a leading frontmatter block: `---\n…\n---\n` (or end-of-string).
  // Body starts on the line after the closing fence.
  const m = content.match(/^---\n([\s\S]*?)\n---(?:\n([\s\S]*))?$/)
  if (!m) {
    return { frontmatterRaw: '', frontmatterHtml: '', body: content }
  }
  const yamlSource = m[1]
  const body = m[2] ?? ''
  const yamlHtml = yamlSource.length > 0
    ? hljs.highlight(yamlSource, { language: 'yaml', ignoreIllegals: true }).value
    : ''
  // Surround with the two `---` fences (highlight them ourselves so
  // they're visually consistent regardless of the yaml grammar's choice).
  const frontmatterHtml = `<span class="hljs-meta">---</span>\n${yamlHtml}\n<span class="hljs-meta">---</span>`
  return {
    frontmatterRaw: `---\n${yamlSource}\n---`,
    frontmatterHtml,
    body,
  }
}

