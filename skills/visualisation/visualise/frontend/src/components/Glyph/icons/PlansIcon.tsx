import type { ReactElement } from 'react'

export function PlansIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3.5" y="4.5" width="17" height="15" rx="1.4" />
      <path d="M3.5 9.5h17M3.5 14.5h17M8.5 4.5v15M13.5 4.5v15" strokeOpacity="0.3" />
      <path d="M6 17 10.5 12 13 14 17.5 7" strokeWidth="1.5" />
      <circle cx="6" cy="17" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="17.5" cy="7" r="1.1" fill="currentColor" stroke="none" />
    </g>
  )
}
