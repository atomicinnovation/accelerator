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
}

const OPTIONS: OptionMeta[] = [
  { id: 'recently-modified', label: 'Recently modified' },
  { id: 'oldest-first', label: 'Oldest first' },
  { id: 'title-asc', label: 'Title (A → Z)' },
  { id: 'title-desc', label: 'Title (Z → A)' },
  { id: 'id-asc', label: 'ID (ascending)' },
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
          className={`${styles.trigger} ${open ? styles.triggerOpen : ''}`}
        >
          <SortIcon />
          <span>{current.label}</span>
          <ChevronDownIcon />
        </button>
      )}
    >
      <div className={styles.menu}>
        <div className={styles.menuHeader}>Sort by</div>
        <ul className={styles.menuList}>
          {OPTIONS.map(opt => {
            const selected = opt.id === value
            return (
              <li
                key={opt.id}
                role="menuitem"
                tabIndex={-1}
                className={`${styles.menuItem} ${selected ? styles.menuItemActive : ''}`}
                aria-checked={selected}
                onClick={() => {
                  onChange(opt.id)
                  setOpen(false)
                }}
              >
                <span>{opt.label}</span>
                {selected && <CheckIcon />}
              </li>
            )
          })}
        </ul>
      </div>
    </Popover>
  )
}

function SortIcon() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M7 4v16" />
      <path d="m3 8 4-4 4 4" />
      <path d="M17 20V4" />
      <path d="m13 16 4 4 4-4" />
    </svg>
  )
}

function ChevronDownIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  )
}

function CheckIcon() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m5 12 5 5L20 7" />
    </svg>
  )
}
