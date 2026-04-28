import type { Plugin } from 'unified'
import type { Link, Parents, Root, Text } from 'mdast'
import { SKIP, visit } from 'unist-util-visit'
import { WIKI_LINK_PATTERN } from '../../api/wiki-links'

/** The resolver's three return shapes drive three distinct visual
 *  treatments downstream. The hook in Phase 5 returns `pending` while
 *  either docs query is in flight, `resolved` when an entry is found,
 *  and `unresolved` when both queries have settled and no entry
 *  matches. */
export type ResolverResult =
  | { kind: 'resolved'; href: string; title: string }
  | { kind: 'unresolved' }
  | { kind: 'pending' }

export type Resolver = (
  prefix: 'ADR' | 'TICKET',
  n: number,
) => ResolverResult

/** A pseudo-mdast node rendered as a span with a class modifier. Two
 *  flavours: `wiki-link-pending` (cache warming) and
 *  `unresolved-wiki-link` (settled-but-no-match). The
 *  `data.hName` + `data.hProperties` shape is the unified-recommended
 *  way to emit a custom HTML element without enabling raw-HTML
 *  parsing. */
export interface MarkerNode {
  type: 'wikiLinkMarker'
  data: {
    hName: 'span'
    hProperties: {
      className: 'unresolved-wiki-link' | 'wiki-link-pending'
      title: string
    }
    hChildren: [{ type: 'text'; value: string }]
  }
}

type ReplacementNode = Text | Link | MarkerNode

/** remark plugin that rewrites `[[ADR-NNNN]]` / `[[TICKET-NNNN]]`
 *  occurrences inside `text` nodes only. mdast represents fenced and
 *  inline code as `code`/`inlineCode` nodes whose interior text is
 *  *not* `text`-typed children, so the visitor never enters them. */
export const remarkWikiLinks: Plugin<[Resolver], Root> = (resolve) => (tree) => {
  visit(tree, 'text', (node: Text, index, parent: Parents | undefined) => {
    if (!parent || index === undefined) return
    const replacement = splitTextNode(node, resolve)
    if (!replacement) return
    parent.children.splice(
      index,
      1,
      ...(replacement as Array<Text | Link>),
    )
    // Skip past the inserted nodes unconditionally. Inserted nodes are
    // either Text (already exhausted), Link (Text child is the entry
    // title — never the bracket-form), or marker spans (child Text
    // *is* bracket-form and would re-match if visited). SKIP prevents
    // double-rewrite for the marker case and is a no-op for the others.
    return [SKIP, index + replacement.length]
  })
}

/** Returns the replacement node sequence, or `null` when the input
 *  contains no bracket-shape matches at all (no allocation overhead
 *  for plain prose). */
function splitTextNode(node: Text, resolve: Resolver): ReplacementNode[] | null {
  const value = node.value
  // Reset lastIndex defensively — the WIKI_LINK_PATTERN regex is
  // module-scoped and uses the global flag.
  WIKI_LINK_PATTERN.lastIndex = 0
  const out: ReplacementNode[] = []
  let cursor = 0
  let match: RegExpExecArray | null = WIKI_LINK_PATTERN.exec(value)
  while (match !== null) {
    const [bracketForm, prefixRaw, digitsRaw] = match
    const prefix = prefixRaw as 'ADR' | 'TICKET'
    const n = parseInt(digitsRaw, 10)
    const start = match.index
    const end = start + bracketForm.length

    if (start > cursor) {
      out.push({ type: 'text', value: value.slice(cursor, start) })
    }

    const result = resolve(prefix, n)
    if (result.kind === 'resolved') {
      out.push(linkNode(result.href, result.title, bracketForm))
    } else if (result.kind === 'unresolved') {
      out.push(
        markerNode(
          bracketForm,
          'unresolved-wiki-link',
          `No matching ${prefix} found for ID ${n}`,
        ),
      )
    } else {
      out.push(markerNode(bracketForm, 'wiki-link-pending', 'Loading reference…'))
    }

    cursor = end
    match = WIKI_LINK_PATTERN.exec(value)
  }
  if (out.length === 0) return null
  if (cursor < value.length) {
    out.push({ type: 'text', value: value.slice(cursor) })
  }
  return out
}

function linkNode(href: string, title: string, bracketForm: string): Link {
  return {
    type: 'link',
    url: href,
    children: [{ type: 'text', value: title }],
    // The hover/source-form fallback. `data.hProperties.title` becomes
    // the rendered `<a>`'s `title` attribute via mdast-to-hast.
    data: { hProperties: { title: bracketForm } },
  }
}

function markerNode(
  bracketForm: string,
  className: 'unresolved-wiki-link' | 'wiki-link-pending',
  title: string,
): MarkerNode {
  return {
    type: 'wikiLinkMarker',
    data: {
      hName: 'span',
      hProperties: { className, title },
      hChildren: [{ type: 'text', value: bracketForm }],
    },
  }
}
