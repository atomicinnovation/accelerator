import type { ReactElement } from 'react'

export function DecisionsIcon(): ReactElement {
  return (
    <>
      <line x1="12" y1="3" x2="12" y2="21" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <path
        d="M12 6 L7 9 L12 12"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M12 12 L17 15 L12 18"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="12" cy="21" r="1.2" fill="currentColor" />
    </>
  )
}
