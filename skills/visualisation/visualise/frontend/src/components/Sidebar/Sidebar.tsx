import { Link, useRouterState } from "@tanstack/react-router";
import { type RefObject, useState } from "react";
import type { DocType, LibraryDocType, LibraryPhase } from "../../api/types";
import { useUnseenDocTypesContext } from "../../api/use-unseen-doc-types";
import { ActivityFeed } from "../ActivityFeed/ActivityFeed";
import { SearchResultsPanel } from "./SearchResultsPanel";
import styles from "./Sidebar.module.css";

interface Props {
  docTypes: DocType[];
  phases: LibraryPhase[];
  templates: LibraryDocType | null;
  searchInputRef: RefObject<HTMLInputElement | null>;
}

export function Sidebar({
  docTypes,
  phases,
  templates,
  searchInputRef,
}: Props) {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const { unseenSet } = useUnseenDocTypesContext();
  const [query, setQuery] = useState("");
  // docTypes is kept around for affordances that need dirPath / inLifecycle,
  // but phase grouping comes from the server-driven `phases` prop.
  void docTypes;

  return (
    <nav className={styles.sidebar} aria-label="Site navigation">
      <div className={styles.search}>
        <div className={styles.searchInputWrap}>
          <SearchIcon />
          <input
            ref={searchInputRef}
            type="search"
            aria-label="Search"
            placeholder="Search meta/…"
            className={styles.searchInput}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setQuery("");
                e.currentTarget.blur();
              }
            }}
          />
          {query ? (
            <button
              type="button"
              className={styles.searchClear}
              aria-label="Clear search"
              title="Clear (Esc)"
              onClick={() => {
                setQuery("");
                searchInputRef.current?.focus();
              }}
            >
              <CloseIcon />
            </button>
          ) : (
            <kbd className={styles.kbd}>/</kbd>
          )}
        </div>
        <SearchResultsPanel query={query} />
      </div>

      <section aria-labelledby="library-heading" className={styles.section}>
        <Link
          to="/library"
          id="library-heading"
          className={`${styles.libraryHeading} ${styles.libraryHeadingClickable} ${pathname === "/library" ? styles.libraryHeadingActive : ""}`}
        >
          <span>LIBRARY</span>
          <span className={styles.libraryHeadingHint} aria-hidden="true">
            All
          </span>
        </Link>
        {phases.map((phase) => (
          <section key={phase.id} className={styles.phase}>
            <h3 className={styles.phaseHeading}>{phase.label.toUpperCase()}</h3>
            <ul className={styles.list}>
              {phase.docTypes.map((dt) => {
                const active =
                  pathname === `/library/${dt.id}` ||
                  pathname.startsWith(`/library/${dt.id}/`);
                const hasUnseen = unseenSet.has(dt.id);
                const linkLabel = hasUnseen
                  ? `${dt.label} (unseen changes)`
                  : dt.label;
                return (
                  <li key={dt.id}>
                    <Link
                      to="/library/$type"
                      params={{ type: dt.id }}
                      aria-label={linkLabel}
                      title={
                        hasUnseen
                          ? "Unseen changes since your last visit"
                          : undefined
                      }
                      className={`${styles.link} ${active ? styles.active : ""}`}
                    >
                      <span className={styles.label}>{dt.label}</span>
                      {hasUnseen && (
                        <span className={styles.dot} aria-hidden="true" />
                      )}
                      {dt.count > 0 && (
                        <span className={styles.count}>{dt.count}</span>
                      )}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </section>

      <section aria-labelledby="views-heading" className={styles.section}>
        <h2 id="views-heading" className={styles.sectionHeading}>
          VIEWS
        </h2>
        <ul className={styles.list}>
          <li>
            <Link
              to="/kanban"
              className={`${styles.link} ${pathname === "/kanban" ? styles.active : ""}`}
            >
              <KanbanIcon />
              <span className={styles.label}>Kanban</span>
            </Link>
          </li>
          <li>
            <Link
              to="/lifecycle"
              className={`${styles.link} ${pathname.startsWith("/lifecycle") ? styles.active : ""}`}
            >
              <LifecycleIcon />
              <span className={styles.label}>Lifecycle</span>
            </Link>
          </li>
        </ul>
      </section>

      <ActivityFeed />

      {templates && (
        <section aria-labelledby="meta-heading" className={styles.section}>
          <h2 id="meta-heading" className={styles.sectionHeading}>
            META
          </h2>
          <ul className={styles.list}>
            <li>
              <Link
                to="/library/$type"
                params={{ type: "templates" }}
                className={`${styles.link} ${
                  pathname === "/library/templates" ||
                  pathname.startsWith("/library/templates/")
                    ? styles.active
                    : ""
                }`}
              >
                <span className={styles.label}>{templates.label}</span>
              </Link>
            </li>
          </ul>
        </section>
      )}
    </nav>
  );
}

function SearchIcon() {
  return (
    <svg
      className={styles.searchIcon}
      width="14"
      height="14"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <circle
        cx="10.5"
        cy="10.5"
        r="6.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <path
        d="M15.2 15.2 L20 20"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M6 6 L18 18 M18 6 L6 18"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function KanbanIcon() {
  return (
    <svg
      className={styles.viewIcon}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <rect
        x="3"
        y="5"
        width="4"
        height="12"
        rx="1"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <rect
        x="10"
        y="5"
        width="4"
        height="8"
        rx="1"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <rect
        x="17"
        y="5"
        width="4"
        height="14"
        rx="1"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
    </svg>
  );
}

/* Lifecycle nav icon — mirrors the prototype's `lifecycle` Icon (four
   dots in a square frame with cross-bars). Shared with the lifecycle
   pages' eyebrow icon (`routes/lifecycle/icons.tsx`); the SVG is
   inlined here too so the sidebar doesn't reach across route folders
   for a chrome-level glyph. */
function LifecycleIcon() {
  return (
    <svg
      className={styles.viewIcon}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="6" cy="6" r="2" />
      <circle cx="18" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <circle cx="18" cy="18" r="2" />
      <path d="M8 6h8M6 8v8M18 8v8M8 18h8" />
    </svg>
  );
}
