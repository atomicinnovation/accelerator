import type { ReactElement } from 'react'

export function NotesIcon(): ReactElement {
  return (
    <>
      <path
        d="M5 3 H14 L19 8 V20 C19 20.55 18.55 21 18 21 H5 C4.45 21 4 20.55 4 20 V4 C4 3.45 4.45 3 5 3 Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <path
        d="M14 3 V8 H19"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <line x1="7" y1="12" x2="16" y2="12" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <line x1="7" y1="15" x2="16" y2="15" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <line x1="7" y1="18" x2="13" y2="18" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </>
  )
}
