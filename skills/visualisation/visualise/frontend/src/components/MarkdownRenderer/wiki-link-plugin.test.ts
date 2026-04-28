import { describe, expect, it } from 'vitest'
import { unified } from 'unified'
import remarkParse from 'remark-parse'
import { remarkWikiLinks, type Resolver } from './wiki-link-plugin'
import type { Root } from 'mdast'

/** Run the plugin over a parsed markdown source. Returns the
 *  transformed mdast root. */
function transform(source: string, resolver: Resolver): Root {
  const tree = unified().use(remarkParse).parse(source) as Root
  unified().use(remarkWikiLinks, resolver).runSync(tree)
  return tree
}

const resolveAlwaysResolved: Resolver = (prefix, n) => ({
  kind: 'resolved',
  href: `/library/decisions/ADR-${String(n).padStart(4, '0')}-foo`,
  title: prefix === 'ADR' ? 'Example decision' : 'Example ticket',
})

const resolveAlwaysUnresolved: Resolver = () => ({ kind: 'unresolved' })
const resolveAlwaysPending: Resolver = () => ({ kind: 'pending' })

// ── Step 4.1 ────────────────────────────────────────────────────────────────
describe('remarkWikiLinks: leaves text without wiki-links unchanged', () => {
  it('produces a structurally identical tree for plain prose', () => {
    const source = 'Just some plain prose here.\n'
    const before = unified().use(remarkParse).parse(source) as Root
    const after = transform(source, resolveAlwaysResolved)
    expect(after).toEqual(before)
  })
})

// ── Step 4.2 ────────────────────────────────────────────────────────────────
describe('remarkWikiLinks: rewrites resolved ref to a link node with title as display text', () => {
  it('produces [Text("see "), Link(title-as-text, hProperties.title=bracketForm)]', () => {
    const tree = transform('see [[ADR-0001]]\n', resolveAlwaysResolved)
    const para = tree.children[0]
    expect(para.type).toBe('paragraph')
    if (para.type !== 'paragraph') return
    expect(para.children).toHaveLength(2)
    expect(para.children[0]).toMatchObject({ type: 'text', value: 'see ' })
    const link = para.children[1] as { type: string; url: string; children: unknown; data?: { hProperties?: { title?: string } } }
    expect(link.type).toBe('link')
    expect(link.url).toBe('/library/decisions/ADR-0001-foo')
    expect(link.children).toEqual([{ type: 'text', value: 'Example decision' }])
    expect(link.data?.hProperties?.title).toBe('[[ADR-0001]]')
  })
})

// ── Step 4.3 ────────────────────────────────────────────────────────────────
describe('remarkWikiLinks: emits unresolved marker when resolver returns kind=unresolved', () => {
  it('produces [Text("see "), MarkerNode(unresolved-wiki-link, diagnostic title)]', () => {
    const tree = transform('see [[ADR-9999]]\n', resolveAlwaysUnresolved)
    const para = tree.children[0]
    if (para.type !== 'paragraph') throw new Error('expected paragraph')
    const marker = para.children[1] as {
      type: string
      data: { hName: string; hProperties: { className: string; title: string }; hChildren: Array<{ type: string; value: string }> }
    }
    expect(marker.type).toBe('wikiLinkMarker')
    expect(marker.data.hName).toBe('span')
    expect(marker.data.hProperties.className).toBe('unresolved-wiki-link')
    expect(marker.data.hProperties.title).toBe('No matching ADR found for ID 9999')
    expect(marker.data.hChildren).toEqual([{ type: 'text', value: '[[ADR-9999]]' }])
  })
})

// ── Step 4.3b ───────────────────────────────────────────────────────────────
describe('remarkWikiLinks: emits pending marker when resolver returns kind=pending', () => {
  it('produces [Text("see "), MarkerNode(wiki-link-pending, "Loading reference…")]', () => {
    const tree = transform('see [[ADR-0001]]\n', resolveAlwaysPending)
    const para = tree.children[0]
    if (para.type !== 'paragraph') throw new Error('expected paragraph')
    const marker = para.children[1] as unknown as {
      data: { hProperties: { className: string; title: string } }
    }
    expect(marker.data.hProperties.className).toBe('wiki-link-pending')
    expect(marker.data.hProperties.title).toBe('Loading reference…')
  })
})

// ── Step 4.4 ────────────────────────────────────────────────────────────────
describe('remarkWikiLinks: handles multiple matches in one text node', () => {
  it('produces interleaved Link/Text/Link when both resolve', () => {
    const tree = transform('[[ADR-0001]] and [[TICKET-1]]\n', resolveAlwaysResolved)
    const para = tree.children[0]
    if (para.type !== 'paragraph') throw new Error('expected paragraph')
    expect(para.children.map((c) => c.type)).toEqual(['link', 'text', 'link'])
    const middle = para.children[1] as { type: string; value: string }
    expect(middle.value).toBe(' and ')
  })

  it('mixes Link with marker variants when resolver returns mixed kinds', () => {
    const mixed: Resolver = (prefix) =>
      prefix === 'TICKET'
        ? {
            kind: 'resolved',
            href: '/library/tickets/0001-foo',
            title: 'Foo ticket',
          }
        : { kind: 'unresolved' }
    const tree = transform('[[ADR-0001]] and [[TICKET-1]]\n', mixed)
    const para = tree.children[0]
    if (para.type !== 'paragraph') throw new Error('expected paragraph')
    // [marker (ADR unresolved), text(' and '), link (TICKET resolved)]
    expect(para.children[0].type).toBe('wikiLinkMarker')
    expect(para.children[1].type).toBe('text')
    expect(para.children[2].type).toBe('link')
  })
})

// ── Step 4.5 ────────────────────────────────────────────────────────────────
describe('remarkWikiLinks: does not visit inline code', () => {
  it('rewrites only the un-coded reference', () => {
    const tree = transform(
      'plain [[ADR-0001]] and `[[ADR-0002]]`\n',
      resolveAlwaysResolved,
    )
    const para = tree.children[0]
    if (para.type !== 'paragraph') throw new Error('expected paragraph')
    // Expected children: text("plain "), link, text(" and "), inlineCode
    const types = para.children.map((c) => c.type)
    expect(types.filter((t) => t === 'link')).toHaveLength(1)
    expect(types).toContain('inlineCode')
  })
})

// ── Step 4.5b ───────────────────────────────────────────────────────────────
describe('remarkWikiLinks: does not visit fenced code blocks', () => {
  it('emits no Link inside a triple-backtick block', () => {
    const tree = transform(
      '```\n[[ADR-0001]]\n```\n',
      resolveAlwaysResolved,
    )
    // The tree's first child is the code block.
    const code = tree.children[0] as { type: string; value: string }
    expect(code.type).toBe('code')
    expect(code.value).toContain('[[ADR-0001]]')
    // There should be no link nodes anywhere.
    const flat = JSON.stringify(tree)
    expect(flat).not.toContain('"type":"link"')
  })
})

// ── Step 4.5c ───────────────────────────────────────────────────────────────
describe('remarkWikiLinks: SKIP prevents double rewrite of inserted children', () => {
  it('emits exactly one Link when the resolved title contains a bracket-form', () => {
    // Contrived collision: the resolved title itself contains `[[ADR-0002]]`
    // text. Without the SKIP guard, the visitor would re-enter the new
    // Link's child Text node and rewrite again.
    const collidingResolver: Resolver = () => ({
      kind: 'resolved',
      href: '/library/decisions/ADR-0001-foo',
      title: '[[ADR-0002]]',
    })
    const tree = transform('see [[ADR-0001]]\n', collidingResolver)
    const flat = JSON.stringify(tree)
    const linkCount = (flat.match(/"type":"link"/g) ?? []).length
    expect(linkCount).toBe(1)
  })
})
