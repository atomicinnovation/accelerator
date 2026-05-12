import type { ReactElement } from 'react'

export function WorkItemsIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3.5 7.5h11l2 2v5l-2 2h-11z" />
      <path d="M14.5 7.5v9" strokeDasharray="1.2 1.4" />
      <path d="M6 11h5.5M6 13.5h4" />
      <circle cx="18.5" cy="9" r="0.55" fill="currentColor" stroke="none" />
      <circle cx="18.5" cy="12" r="0.55" fill="currentColor" stroke="none" />
      <circle cx="18.5" cy="15" r="0.55" fill="currentColor" stroke="none" />
    </g>
  )
}
