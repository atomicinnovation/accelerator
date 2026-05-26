import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { statusToVariant } from '../../api/status-variant'

export interface StatusBadgeProps {
  value: unknown
}

// The prototype always renders status labels in sentence case
// (`accepted` → `Accepted`), regardless of how the source frontmatter
// cases the value. Tone selection still keys off the raw value via
// `statusToVariant`, which normalises casing and separators separately.
function sentenceCase(value: unknown): unknown {
  if (typeof value !== 'string' || value.length === 0) return value
  return value.charAt(0).toUpperCase() + value.slice(1).toLowerCase()
}

export function StatusBadge({ value }: StatusBadgeProps) {
  return (
    <FrontmatterChip
      name="status"
      value={sentenceCase(value)}
      variant={statusToVariant(value)}
      testId="status-badge"
    />
  )
}
