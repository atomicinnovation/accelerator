import type { ReactNode } from 'react'
import styles from './TopbarIconButton.module.css'

export interface TopbarIconButtonProps {
  /** Function-describing accessible name (e.g. "Dark theme"). */
  ariaLabel: string
  /** Pressed state for binary toggle buttons. */
  ariaPressed: boolean
  /** State key written to `data-icon` for CSS targeting and tests. */
  dataIcon: string
  /** Glyph / SVG content rendered inside the button. Should be
   *  decorative — wrap in `aria-hidden="true"` at the call site. */
  children: ReactNode
  onClick: () => void
}

export function TopbarIconButton(props: TopbarIconButtonProps) {
  return (
    <button
      type="button"
      className={styles.toggle}
      data-icon={props.dataIcon}
      aria-label={props.ariaLabel}
      aria-pressed={props.ariaPressed}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  )
}
