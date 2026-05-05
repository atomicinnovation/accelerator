import { useMemo } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeHighlight from 'rehype-highlight'
import styles from './MarkdownRenderer.module.css'
import { remarkWikiLinks, type Resolver } from './wiki-link-plugin'
import { buildWikiLinkPattern } from '../../api/wiki-links'

const DEFAULT_WIKI_LINK_PATTERN = buildWikiLinkPattern(null)

interface Props {
  content: string
  /** Optional. When provided, body text matching the wiki-link
   *  bracket-shape (`[[ADR-NNNN]]` / `[[WORK-ITEM-NNNN]]`) is rewritten
   *  to anchors / pending-markers / unresolved-markers per the
   *  resolver's return kind. When omitted, no rewriting happens. */
  resolveWikiLink?: Resolver
  /** The compiled wiki-link pattern to match against. When omitted and
   *  `resolveWikiLink` is provided, defaults to the default numeric-only
   *  pattern. Callers with a project-prefixed workspace should supply the
   *  pattern from `useWikiLinkResolver`. */
  wikiLinkPattern?: RegExp
}

export function MarkdownRenderer({ content, resolveWikiLink, wikiLinkPattern }: Props) {
  // Memoise the plugin tuple keyed on the resolver's and pattern's identity.
  // The resolver from `useWikiLinkResolver` is itself memoised (stable
  // across renders that don't change docs-cache state), so this tuple
  // is stable too — react-markdown short-circuits its pipeline re-run
  // for content-unchanged renders. When docs caches settle and the
  // resolver reference rotates, the tuple identity changes and the
  // pipeline re-runs, flipping pending markers to anchors.
  const effectivePattern = wikiLinkPattern ?? DEFAULT_WIKI_LINK_PATTERN
  const remarkPlugins = useMemo(
    () =>
      resolveWikiLink
        ? ([remarkGfm, [remarkWikiLinks, effectivePattern, resolveWikiLink]] as const)
        : ([remarkGfm] as const),
    [resolveWikiLink, effectivePattern],
  )
  return (
    <div className={styles.markdown}>
      <ReactMarkdown
        remarkPlugins={remarkPlugins as never}
        rehypePlugins={[rehypeHighlight]}
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}
