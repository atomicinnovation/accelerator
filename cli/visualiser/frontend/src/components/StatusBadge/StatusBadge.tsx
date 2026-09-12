import { statusToVariant } from "../../api/status-variant";
import type { ChipVariant } from "../Chip/Chip";
import { FrontmatterChip } from "../FrontmatterChip/FrontmatterChip";

export interface StatusBadgeProps {
  value: unknown;
  /** Resolved chip variant override. When absent, the shared semantic
   *  `statusToVariant` lexicon applies. Supplied by a type-aware parent for
   *  doc types on a bespoke chip scale (e.g. topic-research's lifecycle ramp),
   *  keeping this presenter doc-type-agnostic. */
  variant?: ChipVariant;
}

// The prototype always renders status labels in sentence case
// (`accepted` → `Accepted`), regardless of how the source frontmatter
// cases the value. Tone selection still keys off the raw value via
// `statusToVariant`, which normalises casing and separators separately.
function sentenceCase(value: unknown): unknown {
  if (typeof value !== "string" || value.length === 0) return value;
  return value.charAt(0).toUpperCase() + value.slice(1).toLowerCase();
}

export function StatusBadge({ value, variant }: StatusBadgeProps) {
  return (
    <FrontmatterChip
      name="status"
      value={sentenceCase(value)}
      variant={variant ?? statusToVariant(value)}
      testId="status-badge"
    />
  );
}
