import { Link, useRouterState } from "@tanstack/react-router";
import { type RefObject, useRef, useState } from "react";
import type { DocType, LibraryDocType, LibraryPhase } from "../../api/types";
import { useServerInfo } from "../../api/use-server-info";
import { useUnseenDocTypesContext } from "../../api/use-unseen-doc-types";
import { ActivityFeed } from "../ActivityFeed/ActivityFeed";
import { useDevActivationContext } from "../DevDesignSystem/use-dev-activation";
import { Icon } from "../Icon/Icon";
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
          <Icon name="search" size={14} className={styles.searchIcon} />
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
              <Icon name="close" size={11} />
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
              <Icon name="kanban" size={16} className={styles.viewIcon} />
              <span className={styles.label}>Kanban</span>
            </Link>
          </li>
          <li>
            <Link
              to="/lifecycle"
              className={`${styles.link} ${pathname.startsWith("/lifecycle") ? styles.active : ""}`}
            >
              <Icon name="lifecycle" size={16} className={styles.viewIcon} />
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

      <SidebarFoot />
    </nav>
  );
}

/** Sidebar-foot version label that doubles as the (least-discoverable) third
 *  DevDesignSystem trigger: three clicks within a rolling 600 ms window open
 *  `/dev`. Counter ported from the prototype `app-shell.jsx:7-20`. A subtle
 *  hover affordance + `title` is the only hint — an accepted dev-only tradeoff.
 *  Shows the running server's version (from `/api/info`); a full-width dashed
 *  rule (the `.foot` border-top) separates it from the nav above. */
function SidebarFoot() {
  const dev = useDevActivationContext();
  const { data } = useServerInfo();
  const version = data?.version;
  const clickCountRef = useRef(0);
  const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const onFootClick = () => {
    const next = clickCountRef.current + 1;
    if (next >= 3) {
      clickCountRef.current = 0;
      if (clickTimerRef.current) clearTimeout(clickTimerRef.current);
      dev?.enterDev();
      return;
    }
    clickCountRef.current = next;
    if (clickTimerRef.current) clearTimeout(clickTimerRef.current);
    clickTimerRef.current = setTimeout(() => {
      clickCountRef.current = 0;
    }, 600);
  };

  return (
    <button
      type="button"
      className={styles.foot}
      onClick={onFootClick}
      title="triple-click for the design-system reference"
    >
      {version ? `v${version}` : "—"}
    </button>
  );
}
