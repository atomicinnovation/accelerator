import { useState } from "react";
import { statusToVariant } from "../../api/status-variant";
import type { LibraryFacet, LibrarySelectionPerType } from "../../api/types";
import { Chip } from "../Chip/Chip";
import { Popover } from "../Popover/Popover";
import styles from "./FilterPill.module.css";

export interface FilterPillProps {
  facets: LibraryFacet[];
  selection: LibrarySelectionPerType;
  onChange: (next: LibrarySelectionPerType) => void;
  isFetching?: boolean;
}

const SEARCH_THRESHOLD = 8;

export function FilterPill({
  facets,
  selection,
  onChange,
  isFetching,
}: FilterPillProps) {
  const [open, setOpen] = useState(false);
  const activeCount = Object.values(selection).reduce(
    (n, arr) => n + (arr ? arr.length : 0),
    0,
  );

  function toggleOption(facetId: string, optionId: string) {
    const current = selection[facetId] ?? [];
    const next = current.includes(optionId)
      ? current.filter((o) => o !== optionId)
      : [...current, optionId];
    const updated: LibrarySelectionPerType = { ...selection, [facetId]: next };
    if (next.length === 0) delete updated[facetId];
    onChange(updated);
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
          className={`${styles.trigger} ${open ? styles.triggerOpen : ""} ${activeCount > 0 ? styles.triggerActive : ""}`}
          data-testid="filter-trigger"
        >
          <FilterIcon />
          <span>Filter</span>
          {activeCount > 0 && (
            <span className={styles.badge}>{activeCount}</span>
          )}
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
        <div className={styles.menuHeader}>
          <span>Filter</span>
          {activeCount > 0 && (
            <button
              type="button"
              className={styles.clearButton}
              onClick={() => onChange({})}
            >
              Clear all
            </button>
          )}
        </div>
        {facets.map((facet) => (
          <FacetSection
            key={facet.id}
            facet={facet}
            selected={selection[facet.id] ?? []}
            onToggle={(optionId) => toggleOption(facet.id, optionId)}
          />
        ))}
      </div>
    </Popover>
  );
}

function FacetSection({
  facet,
  selected,
  onToggle,
}: {
  facet: LibraryFacet;
  selected: string[];
  onToggle: (optionId: string) => void;
}) {
  const [query, setQuery] = useState("");
  const showSearch = facet.options.length > SEARCH_THRESHOLD;
  const filtered =
    showSearch && query
      ? facet.options.filter((o) =>
          o.label.toLowerCase().includes(query.toLowerCase()),
        )
      : facet.options;

  // Only the cluster-slug facet gets a scrolling, search-augmented list. The
  // status/project facets stay short and fit the panel height comfortably.
  const isLongFacet = showSearch;

  return (
    <section className={styles.facetSection}>
      <div className={styles.facetHeading}>{facet.label}</div>
      {showSearch && (
        <div className={styles.search}>
          <SearchIcon />
          <input
            type="search"
            placeholder={`Filter ${facet.label.toLowerCase()}…`}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              // Prevent printable chars bubbling up to the menu key handler.
              e.stopPropagation();
            }}
          />
        </div>
      )}
      <ul
        className={`${styles.optionList} ${isLongFacet ? styles.optionListScroll : ""}`}
      >
        {filtered.map((option) => {
          const isSelected = selected.includes(option.id);
          return (
            <li
              key={option.id}
              // biome-ignore lint/a11y/noNoninteractiveElementToInteractiveRole: <li role="menuitemcheckbox"> is the canonical multi-select menu-item markup; the unit tests query getAllByRole("menuitemcheckbox"), so the role cannot be downgraded
              role="menuitemcheckbox"
              tabIndex={-1}
              aria-checked={isSelected}
              className={styles.option}
              onClick={() => onToggle(option.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onToggle(option.id);
                }
              }}
            >
              <span
                className={`${styles.checkbox} ${isSelected ? styles.checkboxChecked : ""}`}
                aria-hidden="true"
              />
              <span className={styles.optionLabel}>
                {facet.id === "status" ? (
                  <Chip variant={statusToVariant(option.id)}>
                    {option.label}
                  </Chip>
                ) : (
                  option.label
                )}
              </span>
              <span className={styles.optionCount}>{option.count}</span>
            </li>
          );
        })}
        {filtered.length === 0 && (
          <li className={styles.noMatches}>No matches.</li>
        )}
      </ul>
    </section>
  );
}

function FilterIcon() {
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
      <path d="M4 4h16l-6 8v6l-4 2v-8z" />
    </svg>
  );
}

function SearchIcon() {
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
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </svg>
  );
}
