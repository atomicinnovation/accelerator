import { useMemo } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeHighlight from 'rehype-highlight'
import styles from './MarkdownRenderer.module.css'
import { remarkWikiLinks, type Resolver } from './wiki-link-plugin'

interface Props {
  content: string
  /** Optional. When provided, body text matching the wiki-link
   *  bracket-shape (`[[ADR-NNNN]]` / `[[WORK-ITEM-NNNN]]`) is rewritten
   *  to anchors / pending-markers / unresolved-markers per the
   *  resolver's return kind. When omitted, no rewriting happens — the
   *  pre-Phase-9 behaviour. */
  resolveWikiLink?: Resolver
}

export function MarkdownRenderer({ content, resolveWikiLink }: Props) {
  // Memoise the plugin tuple keyed on the resolver's identity. The
  // resolver from `useWikiLinkResolver` is itself memoised (stable
  // across renders that don't change docs-cache state), so this tuple
  // is stable too — react-markdown short-circuits its pipeline re-run
  // for content-unchanged renders. When docs caches settle and the
  // resolver reference rotates, the tuple identity changes and the
  // pipeline re-runs, flipping pending markers to anchors.
  const remarkPlugins = useMemo(
    () =>
      resolveWikiLink
        ? ([remarkGfm, [remarkWikiLinks, resolveWikiLink]] as const)
        : ([remarkGfm] as const),
    [resolveWikiLink],
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
