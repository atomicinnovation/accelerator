import type { LibraryFacet, LibrarySelectionPerType } from '../../api/types'
import styles from './NoResultsPanel.module.css'

export interface NoResultsPanelProps {
  selection: LibrarySelectionPerType
  facets: LibraryFacet[]
  onClear: () => void
}

export function NoResultsPanel({ selection, facets, onClear }: NoResultsPanelProps) {
  const summary = summariseSelection(selection, facets)
  return (
    <div className={styles.panel} role="status">
      <h2 className={styles.headline}>no results match your filter</h2>
      <p className={styles.description}>
        Try removing a filter or broadening your selection.
      </p>
      {summary.length > 0 && (
        <p className={styles.activeFilters}>
          Active filters: {summary}
        </p>
      )}
      <button type="button" className={styles.clearButton} onClick={onClear}>
        Clear filters
      </button>
    </div>
  )
}

function summariseSelection(
  selection: LibrarySelectionPerType,
  facets: LibraryFacet[],
): string {
  const parts: string[] = []
  for (const facet of facets) {
    const selected = selection[facet.id]
    if (!selected || selected.length === 0) continue
    const labels = selected.map((id) => {
      const opt = facet.options.find((o) => o.id === id)
      return opt?.label ?? id
    })
    parts.push(`${facet.label}: ${labels.join(', ')}`)
  }
  return parts.join(' · ')
}
