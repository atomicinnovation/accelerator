import type { ReactElement } from 'react'

export function PlanReviewsIcon(): ReactElement {
  return (
    <>
      <path
        d="M4 3 H13 L17 7 V14 H4 Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <path d="M13 3 V7 H17" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
      <line x1="6.5" y1="10" x2="13" y2="10" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <circle
        cx="16"
        cy="16"
        r="5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <path
        d="M13.5 16 L15.2 17.7 L18.5 14.4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </>
  )
}
