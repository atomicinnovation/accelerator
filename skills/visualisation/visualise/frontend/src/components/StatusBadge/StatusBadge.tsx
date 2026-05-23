import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { statusToVariant } from '../../api/status-variant'

export interface StatusBadgeProps {
  value: unknown
}

export function StatusBadge({ value }: StatusBadgeProps) {
  return (
    <FrontmatterChip
      name="status"
      value={value}
      variant={statusToVariant(value)}
      testId="status-badge"
    />
  )
}
