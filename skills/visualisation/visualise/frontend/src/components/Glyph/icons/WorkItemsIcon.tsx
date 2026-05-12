import type { ReactElement } from 'react'

export function WorkItemsIcon(): ReactElement {
  return (
    <>
      <rect
        x="2.5"
        y="6.5"
        width="14"
        height="11"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <line x1="5" y1="10" x2="12" y2="10" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <line x1="5" y1="13" x2="10" y2="13" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <circle cx="19.5" cy="9" r="1" fill="currentColor" />
      <circle cx="19.5" cy="12" r="1" fill="currentColor" />
      <circle cx="19.5" cy="15" r="1" fill="currentColor" />
    </>
  )
}
