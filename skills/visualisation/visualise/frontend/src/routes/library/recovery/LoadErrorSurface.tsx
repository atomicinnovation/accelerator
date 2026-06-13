import type { DocTypeKey } from "../../../api/types";
import { RecoverySurface } from "./RecoverySurface";
import styles from "./RecoverySurface.module.css";

interface LoadErrorSurfaceProps {
  /** Valid type from the URL, when present — same affordance rules as 404. */
  knownType?: DocTypeKey;
  /** Optional already-resolved error message, surfaced as supplementary detail
   *  (not the H1). The caller resolves the raw error to a string via the shared
   *  `errorMessage()` helper, so this component stays purely presentational and
   *  cannot throw on a non-Error value. */
  errorMessage?: string;
}

export function LoadErrorSurface({
  knownType,
  errorMessage,
}: LoadErrorSurfaceProps) {
  return (
    <RecoverySurface
      title="Something went wrong loading this document"
      knownType={knownType}
    >
      <p className={styles.copy} data-testid="load-error-body">
        We couldn&rsquo;t load this document right now.
      </p>
      {errorMessage !== undefined && (
        <p role="alert" className={styles.errorDetail}>
          {errorMessage}
        </p>
      )}
    </RecoverySurface>
  );
}
