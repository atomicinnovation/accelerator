import { Link } from "@tanstack/react-router";
import { DOC_TYPE_LABELS_SINGULAR } from "../../api/types";
import { useSearch } from "../../api/use-search";
import { Glyph } from "../Glyph/Glyph";
import { DOC_TYPE_COLOR_VAR } from "../Glyph/Glyph.constants";
import styles from "./Sidebar.module.css";

/**
 * Render state machine (rows = mutually exclusive branches, top-down
 * precedence; each row evaluated only if all above are false):
 *
 *   query.trim().length < 2                            → render nothing (panel hidden)
 *   search.isError                                     → render nothing (panel cleared; fetch.ts logged)
 *   search.isFetching && !data                         → render loading bar
 *   search.isSuccess && data.length === 0              → render expanded "No matches" status
 *   data && data.length > 0                            → render meta row + results list
 *   otherwise                                          → render nothing (first-load pending)
 */
export function SearchResultsPanel({ query }: { query: string }) {
  const search = useSearch(query);
  const trimmed = query.trim();
  if (trimmed.length < 2) return null;
  if (search.isError) return null;

  const showLoading =
    search.isFetching && (!search.data || search.data.length === 0);
  const showEmpty =
    search.isSuccess && search.data?.length === 0 && !search.isPlaceholderData;
  const showResults = !!(search.data && search.data.length > 0);
  // Narrowed view of the results: empty array when absent, so the results
  // branch (only rendered when showResults) needs no non-null assertions.
  const results = search.data ?? [];

  if (!showLoading && !showEmpty && !showResults) return null;

  return (
    // biome-ignore lint/a11y/useSemanticElements: intentional div+role="region" to preserve the existing search-panel layout/styling; native <section> migration is deferred to a dedicated a11y pass
    <div
      className={styles.searchPanel}
      role="region"
      aria-label="Search results"
    >
      {showLoading && (
        <div className={styles.searchLoading} aria-hidden="true">
          <span className={styles.searchLoadbar} />
          <span className={styles.searchLoadhint}>
            Searching meta/ for <span className={styles.mono}>{trimmed}</span>…
          </span>
        </div>
      )}
      {showResults && (
        <>
          <div className={styles.searchMeta}>
            <span>
              <b>{results.length}</b>{" "}
              {results.length === 1 ? "match" : "matches"} ·{" "}
              <span className={styles.mono}>{trimmed}</span>
            </span>
            <span className={styles.searchHint} title="Enter opens, Esc clears">
              <kbd className={styles.searchHintKbd}>↵</kbd>
              <kbd className={styles.searchHintKbd}>esc</kbd>
            </span>
          </div>
          <div className={styles.searchList} role="listbox">
            {results.map((r) => (
              <Link
                key={`${r.docType}/${r.slug}`}
                to="/library/$type/$fileSlug"
                params={{ type: r.docType, fileSlug: r.slug }}
                className={styles.searchRowLink}
                role="option"
                tabIndex={0}
              >
                <Glyph docType={r.docType} size={24} />
                <div className={styles.searchRowBody}>
                  <div className={styles.searchRowTitle}>
                    <Highlight text={r.title} q={trimmed} />
                  </div>
                  <div className={styles.searchRowSub}>
                    <span
                      className={styles.searchRowType}
                      style={{ color: DOC_TYPE_COLOR_VAR[r.docType] }}
                    >
                      {DOC_TYPE_LABELS_SINGULAR[r.docType]}
                    </span>
                    <span className={styles.searchRowSep}>·</span>
                    <span className={`${styles.mono} ${styles.searchRowPath}`}>
                      {r.docType}/{r.slug}
                    </span>
                  </div>
                </div>
                <ChevronRight />
              </Link>
            ))}
          </div>
        </>
      )}
      {showEmpty && (
        <div className={styles.searchEmpty} role="status" aria-live="polite">
          <div className={styles.searchEmptyTitle}>No matches</div>
          <div className={styles.searchEmptyBody}>
            Nothing in <span className={styles.mono}>meta/</span> matches{" "}
            <span className={styles.mono}>"{trimmed}"</span>. Try a slug, a
            fragment of a title, or a doc id.
          </div>
        </div>
      )}
    </div>
  );
}

function Highlight({ text, q }: { text: string; q: string }) {
  if (!q || q.length < 2) return <>{text}</>;
  const lower = text.toLowerCase();
  const needle = q.toLowerCase();
  const i = lower.indexOf(needle);
  if (i < 0) return <>{text}</>;
  return (
    <>
      {text.slice(0, i)}
      <mark className={styles.searchMark}>
        {text.slice(i, i + needle.length)}
      </mark>
      {text.slice(i + needle.length)}
    </>
  );
}

function ChevronRight() {
  return (
    <svg
      className={styles.searchRowChev}
      width="12"
      height="12"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        d="M9 6 L15 12 L9 18"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
