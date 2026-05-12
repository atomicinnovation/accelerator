import type { ReactElement } from 'react'

export function DesignGapsIcon(): ReactElement {
  return (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="5" width="7" height="14" rx="1" />
      <rect x="14" y="5" width="7" height="14" rx="1" />
      <path d="M10.5 12h3" strokeDasharray="1.2 1.4" />
      <path d="m12 9.5 1.5 2.5L12 14.5" strokeWidth="1.3" />
      <path d="M5 9h3M5 11.5h2.5M5 14h3" strokeOpacity="0.55" />
      <path d="M16 9h3M16 11.5h2.5M16 14h3" strokeOpacity="0.55" />
    </g>
  )
}
