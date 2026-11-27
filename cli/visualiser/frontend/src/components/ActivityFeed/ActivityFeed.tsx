import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { fetchActivity, fetchTypes } from "../../api/fetch";
import { formatRelative } from "../../api/format";
import { libraryLink } from "../../api/library-document-address";
import { queryKeys } from "../../api/query-keys";
import {
  type ActivityEvent,
  DOC_TYPE_LABELS,
  type DocTypeKey,
  isPhysicalDocTypeKey,
} from "../../api/types";
import { useDocEventsContext } from "../../api/use-doc-events";
import { Glyph } from "../Glyph/Glyph";
import styles from "./ActivityFeed.module.css";
import { useActivityRowDocuments } from "./use-activity-row-documents";

const LIMIT = 5;

function basename(path: string): string {
  const ix = path.lastIndexOf("/");
  return ix >= 0 ? path.slice(ix + 1) : path;
}

function activityRowId(event: ActivityEvent): string {
  return `${event.timestamp}|${event.path}|${event.action}`;
}

function ActivityRowContent({
  row,
  label,
  file,
}: {
  row: ActivityEvent;
  label: string;
  file: string;
}) {
  return (
    <>
      {isPhysicalDocTypeKey(row.docType) ? (
        <Glyph docType={row.docType} size={16} />
      ) : (
        <span className={styles.glyphSlot} />
      )}
      <div className={styles.body}>
        <div className={styles.line1}>
          <span className={styles.docType}>{label}</span>
          <span className={styles.separator}> · </span>
          <span className={styles.action}>{row.action}</span>
        </div>
        <div className={styles.line2}>
          {file}
          <span className={styles.separator}> · </span>
          {formatRelative(Date.parse(row.timestamp))}
        </div>
      </div>
    </>
  );
}

export function ActivityFeed() {
  const { data: initial, isSuccess } = useQuery({
    queryKey: queryKeys.activity(LIMIT),
    queryFn: () => fetchActivity(LIMIT),
  });
  // Reuse the cached /api/types result that Sidebar/RootLayout already
  // primed, so doc-type labels match the rest of the UI (plural, server-
  // provided). Falls back to DOC_TYPE_LABELS if the cache is cold or the
  // key is unknown.
  const { data: docTypes } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  });
  const labelFor = (key: DocTypeKey): string =>
    docTypes?.find((t) => t.key === key)?.label ?? DOC_TYPE_LABELS[key] ?? key;

  const { connectionState, subscribe } = useDocEventsContext();
  const queryClient = useQueryClient();
  const [, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 60_000);
    return () => clearInterval(id);
  }, []);

  useEffect(
    () =>
      subscribe((event) => {
        if (event.type !== "doc-changed") return;
        void queryClient.invalidateQueries({
          queryKey: queryKeys.activity(LIMIT),
        });
      }),
    [subscribe, queryClient],
  );

  const rows = initial ?? [];
  const documentOf = useActivityRowDocuments(rows);
  const isLive = connectionState === "open";
  const isEmptyHistory = isSuccess && rows.length === 0;

  return (
    <section aria-labelledby="activity-heading" className={styles.section}>
      <h2 id="activity-heading" className={styles.heading}>
        <span>ACTIVITY</span>
        {isLive && (
          <span data-testid="activity-live-badge" className={styles.liveBadge}>
            LIVE
          </span>
        )}
      </h2>
      {isEmptyHistory ? (
        <p data-testid="activity-empty" className={styles.empty}>
          No recent activity
        </p>
      ) : (
        <ul className={styles.list}>
          {rows.map((r) => {
            const entry = documentOf(r);
            const label = labelFor(r.docType);
            const file = basename(r.path);
            const content = (
              <ActivityRowContent row={r} label={label} file={file} />
            );
            return (
              <li key={activityRowId(r)}>
                {entry ? (
                  <Link
                    {...libraryLink(entry)}
                    className={`${styles.row} ${styles.linkedRow}`}
                    aria-label={`${label}, ${r.action}, ${file}`}
                  >
                    {content}
                  </Link>
                ) : (
                  <div className={styles.row}>{content}</div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
