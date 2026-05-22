import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MarkdownRenderer } from './MarkdownRenderer'
import type { Resolver } from './wiki-link-plugin'

describe('MarkdownRenderer', () => {
  it('renders headings', () => {
    render(<MarkdownRenderer content="# Hello World" />)
    expect(screen.getByRole('heading', { level: 1, name: 'Hello World' })).toBeInTheDocument()
  })

  it('renders GFM tables', async () => {
    // JSX attribute strings don't process \n escape sequences; use a JS
    // string expression so remark-gfm receives actual newline separators.
    render(<MarkdownRenderer content={'| A | B |\n|---|---|\n| x | y |'} />)
    expect(await screen.findByRole('table')).toBeInTheDocument()
  })

  it('renders a code block', () => {
    render(<MarkdownRenderer content="```js\nconsole.log('hi')\n```" />)
    expect(screen.getByText(/console\.log/)).toBeInTheDocument()
  })

  it('renders paragraphs', () => {
    render(<MarkdownRenderer content="Hello paragraph." />)
    expect(screen.getByText('Hello paragraph.')).toBeInTheDocument()
  })

  it('does not render raw HTML (XSS regression guard)', () => {
    // react-markdown defaults to escaping HTML. This test locks in that
    // default so enabling `rehype-raw` or `allowDangerousHtml` in future
    // requires deliberately breaking this test — at which point the
    // contributor is forced to add a sanitiser (e.g. rehype-sanitize).
    const { container } = render(
      <MarkdownRenderer content="<script>alert('xss')</script>" />,
    )
    expect(container.querySelector('script')).toBeNull()
    // The raw text survives as content; it's just not parsed as HTML.
    expect(container.textContent).toContain("<script>alert('xss')</script>")
  })

  it('does not render javascript: URLs in links (XSS regression guard)', () => {
    const { container } = render(
      <MarkdownRenderer content="[click]( javascript:alert(1) )" />,
    )
    const anchor = container.querySelector('a')
    // react-markdown's default urlTransform strips/rewrites dangerous schemes.
    // We assert no anchor with a javascript: href makes it into the DOM.
    expect(anchor?.getAttribute('href') ?? '').not.toMatch(/^\s*javascript:/i)
  })

  // ── Step 4.6 ───────────────────────────────────────────────────────────
  it('renders wiki-link as anchor when resolver returns kind=resolved', () => {
    const resolver: Resolver = () => ({
      kind: 'resolved',
      href: '/library/decisions/ADR-0001-foo',
      title: 'Example decision',
    })
    render(<MarkdownRenderer content="[[ADR-0001]]" resolveWikiLink={resolver} />)
    const anchor = screen.getByRole('link', { name: 'Example decision' })
    expect(anchor.getAttribute('href')).toBe('/library/decisions/ADR-0001-foo')
    expect(anchor.getAttribute('title')).toBe('[[ADR-0001]]')
  })

  // ── Step 4.7 ───────────────────────────────────────────────────────────
  it('renders unresolved-wiki-link span when resolver returns kind=unresolved', () => {
    const resolver: Resolver = () => ({ kind: 'unresolved' })
    const { container } = render(
      <MarkdownRenderer content="[[ADR-9999]]" resolveWikiLink={resolver} />,
    )
    const span = container.querySelector('span.unresolved-wiki-link')
    expect(span).not.toBeNull()
    expect(span?.getAttribute('title')).toBe('No matching ADR found for ID 9999')
    expect(span?.textContent).toBe('[[ADR-9999]]')
    expect(container.querySelector('a')).toBeNull()
  })

  // ── Step 4.7b ──────────────────────────────────────────────────────────
  it('renders wiki-link-pending span when resolver returns kind=pending', () => {
    const resolver: Resolver = () => ({ kind: 'pending' })
    const { container } = render(
      <MarkdownRenderer content="[[ADR-0001]]" resolveWikiLink={resolver} />,
    )
    const span = container.querySelector('span.wiki-link-pending')
    expect(span).not.toBeNull()
    expect(span?.getAttribute('title')).toBe('Loading reference…')
    expect(span?.textContent).toBe('[[ADR-0001]]')
    expect(container.querySelector('a')).toBeNull()
  })

  // ── Step 4.8 ───────────────────────────────────────────────────────────
  it('omits the plugin when resolveWikiLink is not provided', () => {
    const { container } = render(<MarkdownRenderer content="[[ADR-0001]]" />)
    expect(container.textContent).toContain('[[ADR-0001]]')
    expect(container.querySelector('a')).toBeNull()
    expect(container.querySelector('span.unresolved-wiki-link')).toBeNull()
    expect(container.querySelector('span.wiki-link-pending')).toBeNull()
  })

  // ── Step 4.10 ──────────────────────────────────────────────────────────
  it('sanitises resolver-supplied dangerous URL via urlTransform', () => {
    const resolver: Resolver = () => ({
      kind: 'resolved',
      href: 'javascript:alert(1)',
      title: 'evil',
    })
    const { container } = render(
      <MarkdownRenderer content="[[ADR-0001]]" resolveWikiLink={resolver} />,
    )
    const anchor = container.querySelector('a')
    expect(anchor?.getAttribute('href') ?? '').not.toMatch(/^\s*javascript:/i)
  })

  describe('Story 0076 AC4 — markdown pipeline behaviours', () => {
    it('renders a GFM task list with interactive checkboxes', () => {
      const { container } = render(
        <MarkdownRenderer content={'- [x] done\n- [ ] todo\n'} />,
      )
      const checkboxes = container.querySelectorAll('input[type="checkbox"]')
      expect(checkboxes.length).toBe(2)
      expect((checkboxes[0] as HTMLInputElement).checked).toBe(true)
      expect((checkboxes[1] as HTMLInputElement).checked).toBe(false)
    })

    it('renders a GFM table with thead/tbody/tr/td structure', () => {
      const { container } = render(
        <MarkdownRenderer
          content={'| H1 | H2 |\n|----|----|\n| a  | b  |\n'}
        />,
      )
      expect(container.querySelector('table thead tr th')?.textContent).toBe(
        'H1',
      )
      expect(container.querySelector('table tbody tr td')?.textContent).toBe(
        'a',
      )
    })

    it('routes a [[WORK-ITEM-NNNN]] wiki-link in body prose through the resolver', () => {
      const resolver: Resolver = (_prefix, id) => ({
        kind: 'resolved',
        href: `/library/work-items/${id}`,
        title: `Work item ${id}`,
      })
      const { container } = render(
        <MarkdownRenderer
          content={'See [[WORK-ITEM-0042]] for context.'}
          resolveWikiLink={resolver}
        />,
      )
      const anchor = container.querySelector('a')
      expect(anchor?.getAttribute('href')).toBe('/library/work-items/0042')
    })

    it('emits .hljs-keyword spans for an explicit language-python fenced code block', () => {
      const { container } = render(
        <MarkdownRenderer content={'```python\ndef foo():\n    return 1\n```'} />,
      )
      expect(
        container.querySelectorAll('.hljs-keyword').length,
      ).toBeGreaterThanOrEqual(1)
    })

    it('emits .hljs-keyword spans for an explicit language-typescript fenced code block', () => {
      const { container } = render(
        <MarkdownRenderer content={'```typescript\nconst x: number = 1\n```'} />,
      )
      expect(
        container.querySelectorAll('.hljs-keyword').length,
      ).toBeGreaterThanOrEqual(1)
    })

    it('renders a fenced code block and a [[WORK-ITEM-NNNN]] wiki-link in the same document without regression', () => {
      const resolver: Resolver = (_prefix, id) => ({
        kind: 'resolved',
        href: `/library/work-items/${id}`,
        title: `Work item ${id}`,
      })
      const { container } = render(
        <MarkdownRenderer
          content={'See [[WORK-ITEM-0042]].\n\n```python\nx = 1\n```\n'}
          resolveWikiLink={resolver}
        />,
      )
      expect(
        container.querySelector('a[href="/library/work-items/0042"]'),
      ).not.toBeNull()
      expect(container.querySelector('pre code')).not.toBeNull()
    })

    it('does NOT resolve [[WORK-ITEM-NNNN]] inside an inline code span (verbatim pass-through)', () => {
      const resolver: Resolver = () => ({
        kind: 'resolved',
        href: '/x',
        title: 'x',
      })
      const { container } = render(
        <MarkdownRenderer
          content={'inline `[[WORK-ITEM-0042]]` should not resolve'}
          resolveWikiLink={resolver}
        />,
      )
      expect(container.querySelector('a[href="/x"]')).toBeNull()
      expect(container.textContent).toContain('[[WORK-ITEM-0042]]')
    })

    it('renders an unknown-language fence with the base .hljs class (no thrown error)', () => {
      const { container } = render(
        <MarkdownRenderer
          content={"```klingon\nbatlh Daqawlu'taH\n```"}
        />,
      )
      const code = container.querySelector('pre code')
      expect(code).not.toBeNull()
      expect(code!.className).toMatch(/\bhljs\b/)
    })
  })

  // ── Step 4.9 ───────────────────────────────────────────────────────────
  describe('XSS regression guards still pass with plugin enabled', () => {
    const resolver: Resolver = () => ({
      kind: 'resolved',
      href: '/safe',
      title: 'safe',
    })

    it('does not render raw HTML', () => {
      const { container } = render(
        <MarkdownRenderer
          content="<script>alert('xss')</script>"
          resolveWikiLink={resolver}
        />,
      )
      expect(container.querySelector('script')).toBeNull()
    })

    it('does not render javascript: URLs in links', () => {
      const { container } = render(
        <MarkdownRenderer
          content="[click]( javascript:alert(1) )"
          resolveWikiLink={resolver}
        />,
      )
      const anchor = container.querySelector('a')
      expect(anchor?.getAttribute('href') ?? '').not.toMatch(/^\s*javascript:/i)
    })
  })
})
