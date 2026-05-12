import type { ReactElement } from 'react'

export function ResearchIcon(): ReactElement {
  return (
    <>
      <line x1="3" y1="6" x2="14" y2="6" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <line x1="3" y1="10" x2="11" y2="10" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <line x1="3" y1="14" x2="9" y2="14" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <circle
        cx="15.5"
        cy="15.5"
        r="4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <line
        x1="18.5"
        y1="18.5"
        x2="21"
        y2="21"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
    </>
  )
}
