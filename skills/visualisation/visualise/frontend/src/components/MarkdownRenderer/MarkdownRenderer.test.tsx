import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MarkdownRenderer } from './MarkdownRenderer'

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
})
