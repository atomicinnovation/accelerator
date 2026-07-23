import type { ReactElement } from "react";
import { DOC_TYPE_LABELS, type DocTypeKey } from "../../api/types";
import { Glyph } from "../Glyph/Glyph";
import styles from "./EyebrowLabel.module.css";

interface Props {
  type: DocTypeKey;
}

export function EyebrowLabel({ type }: Props): ReactElement {
  return (
    <span className={styles.root} data-testid="eyebrow-label">
      <Glyph docType={type} size={16} framed />
      {DOC_TYPE_LABELS[type].toUpperCase()}
    </span>
  );
}
