import type { ReactElement } from 'react'

export function PrsIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="6" cy="5.5" r="2" />
      <circle cx="6" cy="18.5" r="2" />
      <circle cx="18" cy="12" r="2" />
      <path d="M6 7.5v9" />
      <path d="M6 9.5c0 4 4.5 5 10 2.5" />
      <path d="M14.2 10.8 16.3 12.3 14.4 14" />
    </g>
  )
}
