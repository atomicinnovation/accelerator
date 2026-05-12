import type { ReactElement } from 'react'

export function PrsIcon(): ReactElement {
  return (
    <>
      <circle cx="6" cy="5" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="6" cy="19" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="18" cy="12" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <line x1="6" y1="7.5" x2="6" y2="16.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <path
        d="M8 5 Q15 5 15 12 Q15 19 8 19"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </>
  )
}
