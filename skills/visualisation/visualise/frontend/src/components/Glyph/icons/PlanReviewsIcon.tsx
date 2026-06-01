import type { ReactElement } from 'react'

export function PlanReviewsIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5.5 3.5h8L18 8v9.5A1.5 1.5 0 0 1 16.5 19h-11A1.5 1.5 0 0 1 4 17.5V5A1.5 1.5 0 0 1 5.5 3.5z" />
      <path d="M13.5 3.5V8H18" />
      <path d="M7 11.5h4.5M7 14.5h3.5" />
      {/* Badge ring + check. The ring's fill stays transparent so the
          underlying tile/background bleeds through; otherwise on coloured
          backgrounds (e.g. Pipeline's filled active tile) the previously
          hard-coded `--ac-bg-raised` fill matched the foreground colour
          and the entire badge disappeared. */}
      <circle cx="17" cy="17" r="3.8" fill="none" stroke="currentColor" />
      <path d="m15.3 17.1 1.3 1.3 2.2-2.5" strokeWidth="1.4" />
    </g>
  )
}
