import { describe, it, expect } from 'vitest'
import { renderHighlightedTemplate } from './template-highlight'

describe('renderHighlightedTemplate', () => {
  it('wraps each line in <div class="tpl-line">', () => {
    const html = renderHighlightedTemplate('alpha\nbeta\ngamma')
    const lines = html.match(/<div class="tpl-line">/g)
    expect(lines?.length).toBe(3)
  })

  it('renders empty lines with a non-breaking space so they keep their height', () => {
    const html = renderHighlightedTemplate('first\n\nthird')
    // Three lines total — the second should be a "&nbsp;"-filled placeholder.
    expect(html.match(/<div class="tpl-line">/g)?.length).toBe(3)
    expect(html).toContain('<div class="tpl-line">&nbsp;</div>')
  })

  it('highlights leading YAML frontmatter with the yaml grammar', () => {
    const html = renderHighlightedTemplate('---\ntitle: hi\n---\nbody')
    // `title:` is an attr in YAML hljs grammar.
    expect(html).toContain('hljs-attr')
    // `---` fences are wrapped in our own hljs-meta span.
    expect(html).toContain('hljs-meta">---</span>')
  })

  it('wraps {{variable}} occurrences in a template-variable span', () => {
    const html = renderHighlightedTemplate('Hello {{author}} on {{date}}.')
    const matches = html.match(/hljs-template-variable">{{[^<]+}}<\/span>/g)
    expect(matches?.length).toBe(2)
  })

  it('escapes HTML metacharacters inside template-variable contents (via hljs)', () => {
    const html = renderHighlightedTemplate('Look: {{<not-html>}}')
    // The raw `<not-html>` must never appear literally in the output —
    // hljs HTML-escapes it before our wrapper runs.
    expect(html).not.toContain('<not-html>')
    expect(html).toContain('&lt;not-html&gt;')
    // And the wrapper still wraps the escaped tokens.
    expect(html).toMatch(/hljs-template-variable">{{&lt;not-html&gt;}}<\/span>/)
  })

  it('highlights markdown sections in the body', () => {
    const html = renderHighlightedTemplate('# Heading\n\nBody.')
    // The `#` line is a markdown section.
    expect(html).toContain('hljs-section')
  })

  it('handles content with no frontmatter cleanly', () => {
    const html = renderHighlightedTemplate('just a markdown body')
    expect(html).toContain('<div class="tpl-line">')
    // No frontmatter → no leading hljs-meta `---` span.
    expect(html).not.toContain('hljs-meta">---</span>')
  })
})
