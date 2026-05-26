import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { StatusBadge } from '../StatusBadge/StatusBadge'
import styles from './FrontmatterChips.module.css'

type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }

// The chip whitelist. Drawn from ADR-0033's unified base frontmatter
// schema (status / date / author are base fields shared across all
// doc kinds). If the base schema gains or loses a chip-worthy field,
// update this list and the ADR together.
const CANONICAL_KEYS = ['status', 'date', 'author'] as const

function pickCanonical(
  frontmatter: Record<string, unknown>,
): Array<[string, unknown]> {
  // Build a case-folded view, ignoring null / undefined / empty /
  // whitespace-only values during the fold. A skipped value never
  // claims the canonical slot, so `{ Status: null, status: 'draft' }`
  // correctly resolves to `'draft'` rather than being silently dropped
  // by a first-match-wins collision.
  const folded = new Map<string, unknown>()
  for (const [k, v] of Object.entries(frontmatter)) {
    if (v === null || v === undefined) continue
    if (typeof v === 'string' && v.trim() === '') continue
    const lk = k.trim().toLowerCase()
    if (!folded.has(lk)) folded.set(lk, v)
  }
  const picked: Array<[string, unknown]> = []
  for (const key of CANONICAL_KEYS) {
    if (folded.has(key)) picked.push([key, folded.get(key)])
  }
  return picked
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

  const entries = pickCanonical(props.frontmatter)

  if (entries.length === 0) return null

  return (
    <div className={styles.chips} data-testid="frontmatter-chips">
      {entries.map(([key, value]) =>
        key === 'status'
          ? <StatusBadge key={key} value={value} />
          : <FrontmatterChip key={key} name={key} value={value} />
      )}
    </div>
  )
}
