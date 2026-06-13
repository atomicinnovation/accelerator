import { Link } from "@tanstack/react-router";
import { DOC_TYPE_LABELS_SINGULAR, type DocTypeKey } from "../../../api/types";
import { Glyph } from "../../../components/Glyph/Glyph";
import { DOC_TYPE_COLOR_VAR } from "../../../components/Glyph/Glyph.constants";
import { RecoverySurface } from "./RecoverySurface";
import styles from "./RecoverySurface.module.css";
import { useDelayedFlag } from "./use-delayed-flag";
import { useNearbySlugSuggestions } from "./use-nearby-slug-suggestions";

interface NotFoundSurfaceProps {
  /** The missing document slug, when one is present (unknown-slug 404).
   *  Absent on the router-level catch-all. Drives the H1, the mono-quoted
   *  query, and suggestion generation. */
  missingSlug?: string;
  /** The valid doc type from the URL's first /library/ segment, when it
   *  passes isDocTypeKey. Drives the eyebrow, the per-type hero hue, and the
   *  `Back to {type} list` link. Absent ⇒ catch-all (DefaultBigGlyph, no
   *  eyebrow, no back-to-type). */
  knownType?: DocTypeKey;
}

export function NotFoundSurface({
  missingSlug,
  knownType,
}: NotFoundSurfaceProps) {
  const { suggestions, isPending } = useNearbySlugSuggestions(
    missingSlug ?? "",
  );
  // Defer the working hint so the warm-cache path (fan-out settles near-
  // instantly under staleTime: Infinity) goes straight to the list with no
  // hint flash.
  const showHint = useDelayedFlag(isPending);

  const title = missingSlug ? "Document not found" : "Page not found";

  // Concise status string announced to assistive tech only (the <h2> + links
  // are navigable content rendered outside the live region below).
  const statusText = isPending
    ? showHint
      ? "Looking for similar documents…"
      : ""
    : suggestions.length > 0
      ? `${suggestions.length} similar document${
          suggestions.length === 1 ? "" : "s"
        } found`
      : "";

  return (
    <RecoverySurface title={title} knownType={knownType}>
      <p className={styles.copy} data-testid="not-found-body">
        {missingSlug ? (
          <>
            We couldn&rsquo;t find a document with the slug{" "}
            <code className={styles.mono}>{missingSlug}</code> in this library.
          </>
        ) : (
          <>That page doesn&rsquo;t exist in this workspace.</>
        )}
      </p>

      {/* Visually-hidden scoped live region: announces the deferred working
          hint while the fan-out is pending and a short summary on settle. The
          suggestion <h2> + links are deliberately rendered OUTSIDE this region
          so a screen reader summarises the outcome rather than reading all five
          link rows aloud. */}
      <span className="srOnly" role="status" aria-live="polite">
        {statusText}
      </span>

      {!isPending && suggestions.length > 0 && (
        <div className={styles.suggestions}>
          <h2 className={styles.suggestHeading}>Did you mean…</h2>
          <ul className={styles.suggestList}>
            {suggestions.map((s) => (
              <li key={`${s.type}/${s.slug}`}>
                <Link
                  to="/library/$type/$fileSlug"
                  params={{ type: s.type, fileSlug: s.slug }}
                  className={styles.suggestRowLink}
                >
                  <Glyph docType={s.type} size={24} />
                  <div className={styles.suggestRowBody}>
                    <div className={styles.suggestRowTitle}>{s.title}</div>
                    <div className={styles.suggestRowSub}>
                      <span
                        className={styles.suggestRowType}
                        style={{ color: DOC_TYPE_COLOR_VAR[s.type] }}
                      >
                        {DOC_TYPE_LABELS_SINGULAR[s.type]}
                      </span>
                      <span className={styles.suggestRowSep}>·</span>
                      <span className={styles.suggestRowPath}>
                        {s.type}/{s.slug}
                      </span>
                    </div>
                  </div>
                </Link>
              </li>
            ))}
          </ul>
        </div>
      )}
    </RecoverySurface>
  );
}
