// Feather-style 24×24 stroke icons used by the lifecycle index +
// cluster detail pages. Exported as standalone components so each
// surface can drop them into its layout (eyebrow, meta row, breadcrumb)
// without going through Glyph (which is keyed off DocTypeKey).

import { IconFrame } from '../../components/Glyph/IconFrame'

interface IconProps {
  size?: number
}

/** Framed lifecycle eyebrow glyph — drop-in for `Page`'s eyebrow slot
 *  on lifecycle index/detail. Matches the tinted-square treatment that
 *  per-doc-type `Glyph framed` uses on library/templates pages. */
export function LifecycleEyebrowIcon({ size = 16 }: IconProps) {
  return (
    <IconFrame size={size}>
      <svg
        width="100%"
        height="100%"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <circle cx="6" cy="6" r="2" />
        <circle cx="18" cy="6" r="2" />
        <circle cx="6" cy="18" r="2" />
        <circle cx="18" cy="18" r="2" />
        <path d="M8 6h8M6 8v8M18 8v8M8 18h8" />
      </svg>
    </IconFrame>
  )
}

export function ClockIcon({ size = 11 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" />
    </svg>
  )
}

export function ChevronRightIcon({ size = 10 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m9 6 6 6-6 6" />
    </svg>
  )
}
