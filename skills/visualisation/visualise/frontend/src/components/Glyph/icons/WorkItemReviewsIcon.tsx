import type { ReactElement } from 'react'

export function WorkItemReviewsIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3.5 7.5h10l2 2v5l-2 2h-10z" />
      <path d="M6 11h5M6 13.5h3.5" />
      {/* Badge ring + check; transparent fill so the underlying surface
          shows through. See PlanReviewsIcon for the rationale. */}
      <circle cx="17.5" cy="16.5" r="3.8" fill="none" stroke="currentColor" />
      <path d="m15.8 16.6 1.3 1.3 2.2-2.5" strokeWidth="1.4" />
    </g>
  )
}
