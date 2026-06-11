import type { ReactNode } from "react";
import styles from "./Chip.module.css";

export type ChipVariant =
  | "neutral"
  | "indigo"
  | "green"
  | "amber"
  | "red"
  | "violet";

export type ChipSize = "sm" | "md";

export interface ChipProps {
  variant: ChipVariant;
  size?: ChipSize;
  leading?: ReactNode;
  "aria-label"?: string;
  "data-testid"?: string;
  children: ReactNode;
}

export function Chip({
  variant,
  size = "sm",
  leading,
  children,
  ...rest
}: ChipProps) {
  const hasLeading =
    leading !== undefined && leading !== null && leading !== false;
  const ariaLabel = rest["aria-label"];
  return (
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: role="img" is applied exactly when (and only when) aria-label is supplied — see the conditional `role` below — so aria-label is never present without a supporting role at runtime; Biome cannot see this static-conditional relationship
    <span
      className={styles.chip}
      data-variant={variant}
      data-size={size}
      // aria-label is only valid on an element with a role; apply role="img"
      // (labelled-as-a-unit) only when a label is actually supplied, so plain
      // chips keep their visible text in the accessibility tree.
      role={ariaLabel !== undefined ? "img" : undefined}
      aria-label={ariaLabel}
      data-testid={rest["data-testid"]}
    >
      {hasLeading && (
        <span className={styles.leading} data-slot="leading">
          {leading}
        </span>
      )}
      <span className={styles.label}>{children}</span>
    </span>
  );
}
