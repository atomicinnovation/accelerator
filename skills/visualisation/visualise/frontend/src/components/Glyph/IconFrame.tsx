import type { ReactNode } from "react";
import styles from "./Glyph.module.css";

interface IconFrameProps {
  /** Outer tile dimension in pixels. Padding scales at ~14%, same as
   *  framed Glyph, so the inner SVG draws on the prototype's
   *  size-16/24/32 grid. */
  size?: number;
  /** A 24×24-viewBox stroke SVG, sized down via the framed padding. */
  children: ReactNode;
}

/** Inner content size for a framed tile of the given outer `size`. Mirrors
 *  the ~14% padding convention below so callers that drop a fixed-size `Icon`
 *  (rather than a `width="100%"` SVG) into the frame render at the exact same
 *  inner dimension. Single source of truth for the pad/inner maths. */
export function iconFrameInner(size: number): number {
  const pad = Math.round(size * 0.14);
  return size - 2 * pad;
}

/** Generic chrome-glyph frame — the same tinted square the per-doc-type
 *  framed `Glyph` renders, available for chrome icons (sidebar nav,
 *  page eyebrow, etc.) that aren't tied to a `DocTypeKey`. Background
 *  falls through to `--ac-bg-sunken` from the shared `.frame` rule. */
export function IconFrame({ size = 16, children }: IconFrameProps) {
  const pad = Math.round(size * 0.14);
  const inner = iconFrameInner(size);
  return (
    <span
      className={styles.frame}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        padding: `${pad}px`,
        color: "var(--ac-fg-muted)",
      }}
    >
      <span
        style={{
          display: "inline-flex",
          width: `${inner}px`,
          height: `${inner}px`,
        }}
        aria-hidden="true"
      >
        {children}
      </span>
    </span>
  );
}
