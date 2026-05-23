import type { ComponentType } from 'react'
import { FrontmatterChip } from '../FrontmatterChip/FrontmatterChip'
import { StatusBadge } from '../StatusBadge/StatusBadge'
import { VerdictBadge } from '../VerdictBadge/VerdictBadge'
import { ResultBadge } from '../ResultBadge/ResultBadge'
import styles from './FrontmatterChips.module.css'

type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }

interface BadgeProps {
  value: unknown
}

// Keys MUST be lowercase: `badgeFor` lowercases the lookup key so any
// case variant in frontmatter (`status` / `Status` / `STATUS`) routes
// to the same badge.
const BADGE_FOR_KEY: Record<string, ComponentType<BadgeProps>> = {
  status: StatusBadge,
  verdict: VerdictBadge,
  result: ResultBadge,
}

function badgeFor(key: string): ComponentType<BadgeProps> | null {
  return BADGE_FOR_KEY[key.trim().toLowerCase()] ?? null
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
        const Badge = badgeFor(key)
        if (Badge) return <Badge key={key} value={value} />
        return <FrontmatterChip key={key} name={key} value={value} />
      })}
    </div>
  )
}
