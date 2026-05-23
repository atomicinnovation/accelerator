import { Chip } from '../Chip/Chip'
import { statusToVariant, isStatusKey } from '../../api/status-variant'
import styles from './FrontmatterChips.module.css'

type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }

function formatChipValue(value: unknown): string {
  if (Array.isArray(value)) return value.join(', ')
  if (typeof value === 'object' && value !== null) return JSON.stringify(value)
  return String(value)
}

export function FrontmatterChips(props: FrontmatterChipsProps) {
  if (props.state === 'absent') return null
  if (props.state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = Object.entries(props.frontmatter).filter(([, v]) => {
    if (v === null || v === undefined) return false
    if (typeof v === 'string' && v === '') return false
    return true
  })

  if (entries.length === 0) return null

  return (
    <div className={styles.chips}>
      {entries.map(([key, value]) => {
        const text = formatChipValue(value)
        const variant = isStatusKey(key) ? statusToVariant(value) : 'neutral'
        return (
          <Chip key={key} variant={variant} aria-label={`${key}: ${text}`}>
            {text}
          </Chip>
        )
      })}
    </div>
  )
}
