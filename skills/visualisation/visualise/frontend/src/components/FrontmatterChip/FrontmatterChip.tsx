import { Chip, type ChipVariant } from '../Chip/Chip'

export interface FrontmatterChipProps {
  name: string
  value: unknown
  variant?: ChipVariant
  testId?: string
}

function formatChipValue(value: unknown): string {
  if (Array.isArray(value)) return value.join(', ')
  if (typeof value === 'object' && value !== null) return JSON.stringify(value)
  return String(value)
}

export function FrontmatterChip({
  name,
  value,
  variant = 'neutral',
  testId = 'frontmatter-chip',
}: FrontmatterChipProps) {
  const text = formatChipValue(value)
  return (
    <Chip
      variant={variant}
      aria-label={`${name}: ${text}`}
      data-testid={testId}
    >
      {text}
    </Chip>
  )
}
