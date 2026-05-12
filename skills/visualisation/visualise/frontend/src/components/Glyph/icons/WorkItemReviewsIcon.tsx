import type { ReactElement } from 'react'

export function WorkItemReviewsIcon(): ReactElement {
  return (
    <>
      <rect
        x="2.5"
        y="6.5"
        width="12"
        height="11"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <line x1="5" y1="10" x2="11" y2="10" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <line x1="5" y1="13" x2="9" y2="13" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <circle
        cx="17"
        cy="15"
        r="5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <path
        d="M14.5 15 L16.2 16.7 L19.5 13.4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </>
  )
}
