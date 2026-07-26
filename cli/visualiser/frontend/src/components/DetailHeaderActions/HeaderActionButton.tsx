import type { ReactNode } from "react";
import styles from "./HeaderActionButton.module.css";

interface BaseProps {
  /** Decorative glyph (wrapped `aria-hidden` here); the visible label carries
   *  the accessible name. */
  icon: ReactNode;
  /** Visible text label — also the control's accessible name. */
  label: string;
  /** Native tooltip — also the disabled hint surfaced on hover. */
  title?: string;
}

interface ButtonVariant extends BaseProps {
  as?: "button";
  /** Omit for a disabled / no-op control (a disabled control never fires). */
  onClick?: () => void;
  /** Renders an inert but still-focusable control (see note in the CSS). */
  disabled?: boolean;
  /** id of the visible / SR-only text describing why the control is disabled. */
  ariaDescribedBy?: string;
}

interface AnchorVariant extends BaseProps {
  as: "a";
  href: string;
  /** Defaults to `noopener noreferrer`; editor links may resolve to navigable
   *  http(s) targets via a custom template. */
  rel?: string;
}

export type HeaderActionButtonProps = ButtonVariant | AnchorVariant;

export function HeaderActionButton(props: HeaderActionButtonProps) {
  const common = {
    className: styles.btn,
    ...(props.title !== undefined ? { title: props.title } : {}),
  };
  const inner = (
    <>
      <span className={styles.icon} aria-hidden="true">
        {props.icon}
      </span>
      <span>{props.label}</span>
    </>
  );
  if (props.as === "a") {
    return (
      <a {...common} href={props.href} rel={props.rel ?? "noopener noreferrer"}>
        {inner}
      </a>
    );
  }
  const isDisabled = props.disabled === true;
  return (
    <button
      {...common}
      type="button"
      aria-disabled={isDisabled || undefined}
      {...(props.ariaDescribedBy !== undefined
        ? { "aria-describedby": props.ariaDescribedBy }
        : {})}
      onClick={isDisabled ? undefined : props.onClick}
    >
      {inner}
    </button>
  );
}
