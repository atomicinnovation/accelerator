import { Fragment, type ReactNode, useEffect } from "react";
import { createPortal } from "react-dom";
import { useDocEventsContext } from "../../api/use-doc-events";
import { type Toast, type ToastKind, useToast } from "../../api/use-toast";
import { Icon } from "../Icon/Icon";
import styles from "./Toaster.module.css";

/** Render `text`, replacing paired-backtick runs with inline <code> spans.
 *  Mirrors the markdown body's `inline code` convention so a notification
 *  saying ``\`path/to/file.md\` was updated`` renders the path as monospace.
 *  Lone (unpaired) backticks are passed through as literal characters. */
function renderMessage(text: string): ReactNode {
  const parts = text.split("`");
  // An even number of segments => unpaired backtick; render literally.
  if (parts.length % 2 === 0) return text;
  return parts.map((part, i) =>
    i % 2 === 1 ? (
      // biome-ignore lint/suspicious/noArrayIndexKey: positional split() segments of one string — no stable id; order is fixed and the list never reorders
      <code key={i} className={styles.code}>
        {part}
      </code>
    ) : (
      // biome-ignore lint/suspicious/noArrayIndexKey: positional split() segments of one string — no stable id; order is fixed and the list never reorders
      <Fragment key={i}>{part}</Fragment>
    ),
  );
}

// Visually-hidden severity prefix, announced first so the variant is
// perceivable to assistive tech and colour-blind users independent of the
// colour/icon. `info` carries no prefix (its heading already reads plainly).
const SEVERITY_PREFIX: Record<ToastKind, string> = {
  info: "",
  ok: "Success: ",
  error: "Error: ",
};

/** Per-kind icon. The icon is decorative (`aria-hidden`); the severity prefix
 *  carries the meaning for assistive tech. */
function ToastIcon({ kind }: { kind: ToastKind }) {
  const common = {
    className: styles.icon,
    "data-testid": "toaster-icon",
    viewBox: "0 0 24 24",
    width: 20,
    height: 20,
    fill: "none",
    stroke: "currentColor",
    "aria-hidden": true,
  } as const;
  if (kind === "ok") {
    return (
      <svg {...common}>
        <title>Success</title>
        <circle cx="12" cy="12" r="9" strokeWidth="2" />
        <path
          d="M8 12l3 3 5-6"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }
  if (kind === "error") {
    return (
      <svg {...common}>
        <title>Error</title>
        <path
          d="M12 3.5L21 19H3L12 3.5z"
          strokeWidth="2"
          strokeLinejoin="round"
        />
        <path
          d="M12 10v4M12 17h.01"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }
  return (
    <svg {...common}>
      <title>Information</title>
      <circle cx="12" cy="12" r="9" strokeWidth="2" />
      <path
        d="M12 11v5M12 8h.01"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

interface ToastCardProps {
  toast: Toast;
  onDismiss: (id: number) => void;
  onPause: (id: number) => void;
  onResume: (id: number) => void;
}

function ToastCard({ toast, onDismiss, onPause, onResume }: ToastCardProps) {
  const prefix = SEVERITY_PREFIX[toast.kind];
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: hover/focus only pause the auto-dismiss timer (a non-essential affordance); the card already sits inside a labelled role=alert/status live region, and giving it its own interactive role would duplicate that region role and break the single-live-region a11y contract the unit tests assert
    <div
      className={styles.toast}
      data-kind={toast.kind}
      onMouseEnter={() => onPause(toast.id)}
      onMouseLeave={() => onResume(toast.id)}
      onFocus={() => onPause(toast.id)}
      onBlur={() => onResume(toast.id)}
    >
      {/* Announced first, before the heading, so heading-only success toasts
          still read "Success: <heading>". */}
      {prefix !== "" && <span className="srOnly">{prefix}</span>}
      <ToastIcon kind={toast.kind} />
      <div className={styles.text}>
        <p className={styles.heading}>{toast.heading}</p>
        {/* Omit the body node entirely for heading-only toasts so the
            heading/body flex gap does not mis-pad them. */}
        {toast.message !== "" && (
          <p className={styles.message}>{renderMessage(toast.message)}</p>
        )}
      </div>
      <button
        type="button"
        className={styles.close}
        aria-label="Dismiss notification"
        onClick={() => onDismiss(toast.id)}
      >
        <Icon name="close" size={16} />
      </button>
    </div>
  );
}

export function Toaster() {
  const { toasts, dismissToast, pauseToast, resumeToast } = useToast();
  const docEvents = useDocEventsContext();

  useEffect(() => {
    if (toasts.length === 0) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || toasts.length === 0) return;
      // Escape during an active drag belongs to dnd-kit's drag-cancel — do not
      // also dismiss a lingering (now persistent) error toast on the same key.
      if (docEvents.isDragInProgress()) return;
      dismissToast(toasts[toasts.length - 1].id);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [toasts, dismissToast, docEvents]);

  // Both regions render from the same ordered `toasts` array filtered by kind:
  // `info`/`ok` are polite; `error` is assertive (interrupts) — preserving the
  // assertiveness of the old conflict banner.
  const assertive = toasts.filter((t) => t.kind === "error");
  const polite = toasts.filter((t) => t.kind !== "error");

  return createPortal(
    <div className={styles.viewport} data-testid="toaster-viewport">
      <div
        className={styles.region}
        data-testid="toaster-region-assertive"
        role="alert"
        aria-live="assertive"
      >
        {assertive.map((t) => (
          <ToastCard
            key={t.id}
            toast={t}
            onDismiss={dismissToast}
            onPause={pauseToast}
            onResume={resumeToast}
          />
        ))}
      </div>
      <div
        className={styles.region}
        data-testid="toaster-region-polite"
        role="status"
        aria-live="polite"
      >
        {polite.map((t) => (
          <ToastCard
            key={t.id}
            toast={t}
            onDismiss={dismissToast}
            onPause={pauseToast}
            onResume={resumeToast}
          />
        ))}
      </div>
    </div>,
    document.body,
  );
}
