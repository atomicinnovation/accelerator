import type { ReactNode } from 'react'
import styles from './TopbarIconButton.module.css'

interface BaseProps {
  /** Function-describing accessible name (e.g. "Dark theme"). */
  ariaLabel: string
  /** State key written to `data-icon` for CSS targeting and tests. */
  dataIcon: string
  /** Glyph / SVG content rendered inside the button. Should be
   *  decorative — wrap in `aria-hidden="true"` at the call site. */
  children: ReactNode
  /** Native tooltip — also the disabled `Open in editor` hint. */
  title?: string
}

interface ButtonProps extends BaseProps {
  as?: 'button'
  /** Omit for a disabled / no-op control (a disabled button never fires). */
  onClick?: () => void
  /** Toggle-pressed state. Lives on the button variant only — `aria-pressed` is
   *  invalid on a link, so the anchor variant cannot accept it. */
  ariaPressed?: boolean
  /** Renders an inert but still-focusable control (see note below). */
  disabled?: boolean
  /** id of visible / SR-only text describing why the control is disabled. */
  ariaDescribedBy?: string
}

interface AnchorProps extends BaseProps {
  as: 'a'
  href: string
  /** Defaults to `noopener noreferrer`; editor links may resolve to navigable
   *  http(s) targets via a custom template. */
  rel?: string
}

export type TopbarIconButtonProps = ButtonProps | AnchorProps

export function TopbarIconButton(props: TopbarIconButtonProps) {
  const common = {
    className: styles.toggle,
    'data-icon': props.dataIcon,
    'aria-label': props.ariaLabel,
    ...(props.title !== undefined ? { title: props.title } : {}),
  }
  if (props.as === 'a') {
    return (
      <a {...common} href={props.href} rel={props.rel ?? 'noopener noreferrer'}>
        {props.children}
      </a>
    )
  }
  const isDisabled = props.disabled === true
  return (
    <button
      {...common}
      type="button"
      // `aria-disabled` (NOT the native `disabled` attribute) keeps the control
      // in the tab order so its title / description stay reachable by keyboard
      // and screen readers — the disabled `Open in editor` hint is the user's
      // only path to enabling the feature.
      aria-disabled={isDisabled || undefined}
      {...(props.ariaPressed !== undefined ? { 'aria-pressed': props.ariaPressed } : {})}
      {...(props.ariaDescribedBy !== undefined ? { 'aria-describedby': props.ariaDescribedBy } : {})}
      onClick={isDisabled ? undefined : props.onClick}
    >
      {props.children}
    </button>
  )
}
