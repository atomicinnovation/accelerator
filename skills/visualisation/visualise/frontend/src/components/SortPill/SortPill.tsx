import { useState } from 'react'
import { Popover } from '../Popover/Popover'
import styles from './SortPill.module.css'

export type SortOption =
  | 'recently-modified'
  | 'oldest-first'
  | 'title-asc'
  | 'title-desc'
  | 'id-asc'

interface OptionMeta {
  id: SortOption
  label: string
  direction: '↑' | '↓'
}

const OPTIONS: OptionMeta[] = [
  { id: 'recently-modified', label: 'Recently modified', direction: '↓' },
  { id: 'oldest-first', label: 'Oldest first', direction: '↑' },
  { id: 'title-asc', label: 'Title (A → Z)', direction: '↑' },
  { id: 'title-desc', label: 'Title (Z → A)', direction: '↓' },
  { id: 'id-asc', label: 'ID (ascending)', direction: '↑' },
]

export interface SortPillProps {
  value: SortOption
  onChange: (next: SortOption) => void
}

export function SortPill({ value, onChange }: SortPillProps) {
  const [open, setOpen] = useState(false)
  const current = OPTIONS.find(o => o.id === value) ?? OPTIONS[0]

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      ariaLabel="Sort options"
      trigger={(triggerProps) => (
        <button
          {...triggerProps}
          ref={triggerProps.ref as React.Ref<HTMLButtonElement>}
          className={styles.trigger}
        >
          <span>{current.label}</span>
          <span aria-hidden="true">{current.direction}</span>
        </button>
      )}
    >
      <div className={styles.menuHeader}>SORT BY</div>
      <ul className={styles.menuList}>
        {OPTIONS.map(opt => {
          const selected = opt.id === value
          return (
            <li
              key={opt.id}
              role="menuitem"
              tabIndex={-1}
              className={styles.menuItem}
              aria-checked={selected}
              onClick={() => {
                onChange(opt.id)
                setOpen(false)
              }}
            >
              <span>{opt.label}</span>
              {selected && <span className={styles.checkmark} aria-hidden="true">✓</span>}
            </li>
          )
        })}
      </ul>
    </Popover>
  )
}
