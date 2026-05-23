import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { verdictToVariant } from '../../api/verdict-variant'

export interface VerdictBadgeProps {
  value: unknown
}

export function VerdictBadge({ value }: VerdictBadgeProps) {
  return (
    <FrontmatterChip
      name="verdict"
      value={value}
      variant={verdictToVariant(value)}
      testId="verdict-badge"
    />
  )
}
