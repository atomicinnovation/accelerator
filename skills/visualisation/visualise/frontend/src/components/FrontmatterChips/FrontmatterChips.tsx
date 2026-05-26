import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { StatusBadge } from '../StatusBadge/StatusBadge'
import { formatChipDate } from '../../api/format'
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
  if (props.state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = props.state === 'parsed'
    ? pickCanonical(props.frontmatter)
    : []

  // The empty container is a deliberate spacer reserving subtitle
  // height when no canonical chips qualify (see .chips min-height in
  // the module CSS). Mark it aria-hidden so screen readers don't
  // announce an undifferentiated landmark inside the subtitle slot.
  const isEmpty = entries.length === 0

  return (
    <div
      className={styles.chips}
      data-testid="frontmatter-chips"
      aria-hidden={isEmpty ? true : undefined}
    >
      {entries.map(([key, value]) =>
        key === 'status'
          ? <StatusBadge key={key} value={value} />
          : key === 'date'
            ? <FrontmatterChip key={key} name={key} value={formatChipDate(value)} />
            : <FrontmatterChip key={key} name={key} value={value} />
      )}
    </div>
  )
}
