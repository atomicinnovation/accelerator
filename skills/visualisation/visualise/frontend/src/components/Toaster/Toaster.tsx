import { Fragment, useEffect, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { useToast } from '../../api/use-toast'
import styles from './Toaster.module.css'

/** Render `text`, replacing paired-backtick runs with inline <code> spans.
 *  Mirrors the markdown body's `inline code` convention so a notification
 *  saying ``\`path/to/file.md\` was updated`` renders the path as monospace.
 *  Lone (unpaired) backticks are passed through as literal characters. */
function renderMessage(text: string): ReactNode {
  const parts = text.split('`')
  // An even number of segments => unpaired backtick; render literally.
  if (parts.length % 2 === 0) return text
  return parts.map((part, i) =>
    i % 2 === 1 ? (
      <code key={i} className={styles.code}>{part}</code>
    ) : (
      <Fragment key={i}>{part}</Fragment>
    ),
  )
}

export function Toaster() {
  const { toasts, dismissToast, pauseToast, resumeToast } = useToast()

  useEffect(() => {
    if (toasts.length === 0) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && toasts.length > 0) {
        dismissToast(toasts[toasts.length - 1].id)
      }
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [toasts, dismissToast])

  return createPortal(
    <div
      className={styles.viewport}
      data-testid="toaster-viewport"
      role="status"
      aria-live="polite"
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          className={styles.toast}
          onMouseEnter={() => pauseToast(t.id)}
          onMouseLeave={() => resumeToast(t.id)}
          onFocus={() => pauseToast(t.id)}
          onBlur={() => resumeToast(t.id)}
        >
          <svg
            className={styles.icon}
            data-testid="toaster-icon"
            viewBox="0 0 24 24"
            width="20"
            height="20"
            fill="none"
            stroke="currentColor"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="9" strokeWidth="2" />
            <path
              d="M12 11v5M12 8h.01"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
          <div className={styles.text}>
            <p className={styles.heading}>{t.heading}</p>
            <p className={styles.message}>{renderMessage(t.message)}</p>
          </div>
          <button
            type="button"
            className={styles.close}
            aria-label="Dismiss notification"
            onClick={() => dismissToast(t.id)}
          >
            <svg
              viewBox="0 0 24 24"
              width="16"
              height="16"
              fill="none"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                d="M6 6l12 12M18 6L6 18"
                strokeWidth="2"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </div>
      ))}
    </div>,
    document.body,
  )
}
