import type { ReactElement } from 'react'

export function PlansIcon(): ReactElement {
  return (
    <>
      <rect
        x="3"
        y="3"
        width="18"
        height="18"
        rx="2.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <polyline
        points="6,17 10,12 13,15 18,8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M17 5 L17.6 6.4 L19 7 L17.6 7.6 L17 9 L16.4 7.6 L15 7 L16.4 6.4 Z"
        fill="currentColor"
      />
    </>
  )
}
