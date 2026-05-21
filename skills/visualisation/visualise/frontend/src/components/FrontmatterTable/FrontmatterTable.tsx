import { Fragment, type ReactNode } from 'react'
import type { Resolver } from '../MarkdownRenderer/wiki-link-plugin'
import { splitByBareIds } from '../../api/wiki-links'
import styles from './FrontmatterTable.module.css'

export interface FrontmatterTableProps {
  frontmatter: Record<string, unknown>
  resolveWikiLink: Resolver
  bareIdPattern: RegExp
}

function isEmpty(value: unknown): boolean {
  if (value === null || value === undefined) return true
  if (typeof value === 'string' && value === '') return true
  if (Array.isArray(value) && value.length === 0) return true
  if (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    Object.keys(value as object).length === 0
  ) {
    return true
  }
  return false
}

function safeStringify(value: unknown): string {
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

function renderScalar(
  text: string,
  resolveWikiLink: Resolver,
  bareIdPattern: RegExp,
): ReactNode {
  const segments = splitByBareIds(text, bareIdPattern)
  return segments.map((seg, i) => {
    if (seg.kind === 'text') {
      return <Fragment key={i}>{seg.text}</Fragment>
    }
    const result = resolveWikiLink(seg.prefix, seg.id)
    if (result.kind === 'resolved') {
      return (
        <a key={i} href={result.href} title={result.title}>
          {seg.text}
        </a>
      )
    }
    if (result.kind === 'pending') {
      return (
        <span key={i} className={styles.pending}>
          {seg.text}
        </span>
      )
    }
    return <Fragment key={i}>{seg.text}</Fragment>
  })
}

function renderValue(
  value: unknown,
  resolveWikiLink: Resolver,
  bareIdPattern: RegExp,
): ReactNode {
  if (Array.isArray(value)) {
    return value.map((el, i) => (
      <Fragment key={i}>
        {i > 0 ? ', ' : ''}
        {renderScalar(
          typeof el === 'object' && el !== null
            ? safeStringify(el)
            : String(el),
          resolveWikiLink,
          bareIdPattern,
        )}
      </Fragment>
    ))
  }
  if (typeof value === 'object' && value !== null) {
    return renderScalar(safeStringify(value), resolveWikiLink, bareIdPattern)
  }
  return renderScalar(String(value), resolveWikiLink, bareIdPattern)
}

export function FrontmatterTable({
  frontmatter,
  resolveWikiLink,
  bareIdPattern,
}: FrontmatterTableProps) {
  const entries = Object.entries(frontmatter)
  if (entries.length === 0) return null

  return (
    <dl className={styles.table} aria-label="Document metadata">
      {entries.map(([key, value]) => (
        <div key={key} className={styles.row}>
          <dt className={styles.key}>{key}</dt>
          <dd
            className={styles.value}
            data-empty={isEmpty(value) || undefined}
          >
            {isEmpty(value) ? (
              <span className={styles.empty} aria-hidden="true">
                —
              </span>
            ) : (
              renderValue(value, resolveWikiLink, bareIdPattern)
            )}
          </dd>
        </div>
      ))}
    </dl>
  )
}
