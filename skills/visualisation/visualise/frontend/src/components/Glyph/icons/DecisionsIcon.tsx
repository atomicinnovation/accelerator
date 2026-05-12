import type { ReactElement } from 'react'

export function DecisionsIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3.5v17" />
      <path d="M12 8 6.5 6.5v4L12 12" strokeOpacity="0.5" />
      <path d="M12 11 18 9v4.5L12 15" strokeWidth="1.5" />
      <circle cx="12" cy="20.5" r="1" fill="currentColor" stroke="none" />
    </g>
  )
}
