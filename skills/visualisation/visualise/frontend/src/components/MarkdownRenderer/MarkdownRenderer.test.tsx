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
