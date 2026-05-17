import { useState } from 'react'
import { Popover } from '../Popover/Popover'
import { Chip } from '../Chip/Chip'
import { statusToChipVariant } from '../../api/status-variant'
import type {
  LibraryFacet,
  LibrarySelectionPerType,
} from '../../api/types'
import styles from './FilterPill.module.css'

export interface FilterPillProps {
  facets: LibraryFacet[]
  selection: LibrarySelectionPerType
  onChange: (next: LibrarySelectionPerType) => void
  isFetching?: boolean
}

const SEARCH_THRESHOLD = 8

export function FilterPill({
  facets,
  selection,
  onChange,
  isFetching,
}: FilterPillProps) {
  const [open, setOpen] = useState(false)
  const hasSelection = Object.values(selection).some(
    (arr) => arr && arr.length > 0,
  )

  function toggleOption(facetId: string, optionId: string) {
    const current = selection[facetId] ?? []
    const next = current.includes(optionId)
      ? current.filter((o) => o !== optionId)
      : [...current, optionId]
    const updated: LibrarySelectionPerType = { ...selection, [facetId]: next }
    if (next.length === 0) delete updated[facetId]
    onChange(updated)
  }

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      ariaLabel="Filter options"
      trigger={(triggerProps) => (
        <button
          {...triggerProps}
          ref={triggerProps.ref as React.Ref<HTMLButtonElement>}
          className={styles.trigger}
        >
          <span>▽ Filter</span>
          {isFetching && (
            <span
              className={styles.fetchingDot}
              data-testid="filter-pill-fetching"
              aria-hidden="true"
            />
          )}
        </button>
      )}
    >
      <div className={styles.menu}>
        <div className={styles.facetHeading}>FILTER</div>
        {facets.map((facet) => (
          <FacetSection
            key={facet.id}
            facet={facet}
            selected={selection[facet.id] ?? []}
            onToggle={(optionId) => toggleOption(facet.id, optionId)}
          />
        ))}
        {hasSelection && (
          <div className={styles.footer}>
            <button
              type="button"
              className={styles.clearButton}
              onClick={() => onChange({})}
            >
              Clear filters
            </button>
          </div>
        )}
      </div>
    </Popover>
  )
}

function FacetSection({
  facet,
  selected,
  onToggle,
}: {
  facet: LibraryFacet
  selected: string[]
  onToggle: (optionId: string) => void
}) {
  const [query, setQuery] = useState('')
  const showSearch = facet.options.length > SEARCH_THRESHOLD
  const filtered = showSearch && query
    ? facet.options.filter((o) =>
        o.label.toLowerCase().includes(query.toLowerCase()),
      )
    : facet.options

  return (
    <section className={styles.facetSection}>
      <div className={styles.facetHeading}>{facet.label}</div>
      {showSearch && (
        <input
          type="search"
          className={styles.search}
          placeholder={`Filter ${facet.label}…`}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            // Prevent printable characters bubbling up to the popover menu
            // key handler (which would trigger focus moves).
            e.stopPropagation()
          }}
        />
      )}
      <ul className={styles.optionList}>
        {filtered.map((option) => {
          const isSelected = selected.includes(option.id)
          return (
            <li
              key={option.id}
              role="menuitemcheckbox"
              tabIndex={-1}
              aria-checked={isSelected}
              className={styles.option}
              onClick={() => onToggle(option.id)}
            >
              <span className={styles.optionLabel}>
                {facet.id === 'status' ? (
                  <Chip variant={statusToChipVariant(option.id)}>
                    {option.label}
                  </Chip>
                ) : (
                  <>{option.label}</>
                )}
              </span>
              <span className={styles.optionCount}>{option.count}</span>
            </li>
          )
        })}
      </ul>
    </section>
  )
}
