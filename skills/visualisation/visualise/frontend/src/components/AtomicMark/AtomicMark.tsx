import { useId } from "react";

export interface AtomicMarkProps {
  /** Width/height in pixels (square). Default 28 — the topbar Brand size. */
  size?: number;
  className?: string;
}

/**
 * The Atomic hex mark: a gradient hexagon with a centred accent dot and a
 * faint orbit ring. Extracted from `Brand` so it can render at any size and on
 * a dark backdrop (the DevDesignSystem Atomic-mark section), while `Brand`
 * keeps composing it with the wordmark.
 *
 * Each instance mints a **unique gradient id** (via `useId()`, with the colons
 * stripped so the `url(#…)` fragment reference resolves) so multiple marks on
 * one page never collide on a shared `<linearGradient id>`. The app is pure CSR
 * (no SSR/hydration), so `useId()` carries no server/client id-mismatch risk.
 *
 * Fills stay `var(--ac-accent*)`, so the mark flips with `data-theme` for free.
 */
export function AtomicMark({ size = 28, className }: AtomicMarkProps) {
  const gradientId = `atomic-mark-${useId().replace(/:/g, "")}`;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 40 40"
      aria-hidden="true"
      className={className}
    >
      <defs>
        <linearGradient id={gradientId} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="var(--ac-accent-2)" stopOpacity="1" />
          <stop offset="100%" stopColor="var(--ac-accent)" stopOpacity="1" />
        </linearGradient>
      </defs>
      <path
        d="M20 2 36 11v18L20 38 4 29V11z"
        fill="none"
        stroke={`url(#${gradientId})`}
        strokeWidth="2"
      />
      <circle cx="20" cy="20" r="3" fill="var(--ac-accent-2)" />
      <circle
        cx="20"
        cy="20"
        r="7.5"
        fill="none"
        stroke="var(--ac-accent)"
        strokeWidth="1"
        strokeOpacity="0.5"
      />
    </svg>
  );
}
