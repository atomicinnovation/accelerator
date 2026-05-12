import type { ReactElement } from 'react'

export function DesignInventoriesIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3.5" y="4.5" width="7" height="7" rx="1" />
      <rect x="13.5" y="4.5" width="7" height="7" rx="1" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1" />
      <circle cx="7" cy="8" r="1.2" />
      <path d="m4.5 11 2-2 1.5 1.5L9.5 9l1 1" strokeOpacity="0.5" />
      <path d="M15 7h4M15 9h3" strokeOpacity="0.55" />
      <path d="M15 16h4M15 18h2.5" strokeOpacity="0.55" />
      <circle cx="17" cy="16.5" r="1" strokeOpacity="0.6" />
    </g>
  )
}
