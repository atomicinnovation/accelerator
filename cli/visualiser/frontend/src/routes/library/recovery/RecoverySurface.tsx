import { Link } from "@tanstack/react-router";
import type { CSSProperties, ReactNode } from "react";
import { DOC_TYPE_LABELS_SINGULAR, type DocTypeKey } from "../../../api/types";
import { BigGlyph } from "../../../components/BigGlyph/BigGlyph";
import btnStyles from "../../../components/DetailHeaderActions/HeaderActionButton.module.css";
import { EyebrowLabel } from "../../../components/EyebrowLabel/EyebrowLabel";
import { Page } from "../../../components/Page/Page";
import { DOC_TYPE_HUE } from "../../../styles/tokens";
import styles from "./RecoverySurface.module.css";

/** Neutral hue for the no-type hero panel — mirrors `BigGlyph`'s
 *  `DEFAULT_BIG_HUE` so the gradient panel and the fallback illustration share
 *  one identity on the catch-all / load-error surfaces. */
const DEFAULT_HUE = 215;

interface RecoverySurfaceProps {
  /** Rendered as the Page H1. */
  title: ReactNode;
  /** Valid doc type from the URL, when present. Drives the eyebrow, the
   *  per-type hero hue, and the `Back to {type} list` link. Absent ⇒ catch-all
   *  / no type (default hero hue 215, no eyebrow, no back-to-type). */
  knownType?: DocTypeKey;
  /** Body copy (sentence case, terminal period) and any surface-specific block
   *  (e.g. the 404 `Did you mean…` suggestions). */
  children: ReactNode;
}

export function RecoverySurface({
  title,
  knownType,
  children,
}: RecoverySurfaceProps) {
  const hue = knownType ? DOC_TYPE_HUE[knownType] : DEFAULT_HUE;
  const cssVars = {
    ["--ac-empty-page-hue" as never]: String(hue),
  } satisfies CSSProperties;

  return (
    <Page
      eyebrow={knownType ? <EyebrowLabel type={knownType} /> : undefined}
      title={title}
    >
      <div className={styles.card} style={cssVars}>
        <div className={styles.hero}>
          <BigGlyph docType={knownType} size={96} />
        </div>
        <div className={styles.body}>
          {children}
          <div className={styles.actions}>
            <Link to="/library" className={btnStyles.btn}>
              Back to library
            </Link>
            {knownType && (
              <Link
                to="/library/$type"
                params={{ type: knownType }}
                className={btnStyles.btn}
              >
                Back to {DOC_TYPE_LABELS_SINGULAR[knownType].toLowerCase()} list
              </Link>
            )}
          </div>
        </div>
      </div>
    </Page>
  );
}
