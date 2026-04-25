import styles from './FrontmatterChips.module.css'

interface Props {
  frontmatter: Record<string, unknown>
  state: 'parsed' | 'absent' | 'malformed'
}

export function FrontmatterChips({ frontmatter, state }: Props) {
  if (state === 'absent') return null

  if (state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = Object.entries(frontmatter).filter(
    ([, v]) => v !== null && v !== undefined,
  )

  if (entries.length === 0) return null

  return (
    <dl className={styles.chips}>
      {entries.map(([k, v]) => (
        <div key={k} className={styles.chip}>
          <dt className={styles.key}>{k}</dt>
          <dd className={styles.value}>{formatChipValue(v)}</dd>
        </div>
      ))}
    </dl>
  )
}

/** Render arbitrary YAML frontmatter values as readable text. Arrays
 *  join with commas; nested objects JSON-stringify rather than showing
 *  the useless "[object Object]" default from `String(obj)`. */
function formatChipValue(v: unknown): string {
  if (Array.isArray(v)) return v.join(', ')
  if (v !== null && typeof v === 'object') return JSON.stringify(v)
  return String(v)
}
