import type { ReactNode } from 'react'
import styles from './Glyph.module.css'

interface IconFrameProps {
  /** Outer tile dimension in pixels. Padding scales at ~14%, same as
   *  framed Glyph, so the inner SVG draws on the prototype's
   *  size-16/24/32 grid. */
  size?: number
  /** A 24×24-viewBox stroke SVG, sized down via the framed padding. */
  children: ReactNode
}

/** Generic chrome-glyph frame — the same tinted square the per-doc-type
 *  framed `Glyph` renders, available for chrome icons (sidebar nav,
 *  page eyebrow, etc.) that aren't tied to a `DocTypeKey`. Background
 *  falls through to `--ac-bg-sunken` from the shared `.frame` rule. */
export function IconFrame({ size = 16, children }: IconFrameProps) {
  const pad = Math.round(size * 0.14)
  const inner = size - 2 * pad
  return (
    <span
      className={styles.frame}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        padding: `${pad}px`,
        color: 'var(--ac-fg-muted)',
      }}
    >
      <span
        style={{
          display: 'inline-flex',
          width: `${inner}px`,
          height: `${inner}px`,
        }}
        aria-hidden="true"
      >
        {children}
      </span>
    </span>
  )
}
