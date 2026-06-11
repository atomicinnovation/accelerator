import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Link, Outlet, useParams } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { fetchDocs, fetchLibraryStructure } from "../../api/fetch";
import { formatDate, formatMtime } from "../../api/format";
import { fileSlugFromRelPath } from "../../api/path-utils";
import { queryKeys } from "../../api/query-keys";
import { statusToVariant } from "../../api/status-variant";
import type {
  DocTypeKey,
  IndexEntry,
  LibraryDocType,
  LibrarySelectionPerType,
} from "../../api/types";
import { DOC_TYPE_LABELS, isDocTypeKey } from "../../api/types";
import { useMarkDocTypeSeen } from "../../api/use-unseen-doc-types";
import { Chip } from "../../components/Chip/Chip";
import { EyebrowLabel } from "../../components/EyebrowLabel/EyebrowLabel";
import { FilterPill } from "../../components/FilterPill/FilterPill";
import { Page } from "../../components/Page/Page";
import { type SortOption, SortPill } from "../../components/SortPill/SortPill";
import { formatDocId } from "./doc-type-id";
import { EmptyState } from "./EmptyState";
import styles from "./LibraryTypeView.module.css";
import { NoResultsPanel } from "./NoResultsPanel";

function statusValue(entry: IndexEntry): string {
  const fm = entry.frontmatter as Record<string, unknown> | null;
  const status = fm?.status;
  return typeof status === "string" ? status : "";
}

function firstColumnContent(
  entry: IndexEntry,
):
  | { kind: "id"; value: string }
  | { kind: "date"; value: string }
  | { kind: "empty" } {
  if (entry.workItemId) {
    return { kind: "id", value: formatDocId(entry.workItemId) };
  }
  const dateRaw = entry.frontmatter?.date;
  if (typeof dateRaw === "string" && !Number.isNaN(Date.parse(dateRaw))) {
    return { kind: "date", value: formatDate(dateRaw) };
  }
  return { kind: "empty" };
}

function compareEntries(
  a: IndexEntry,
  b: IndexEntry,
  option: SortOption,
): number {
  function tieBreak(): number {
    const ai = a.workItemId ?? "";
    const bi = b.workItemId ?? "";
    if (ai !== bi) return ai < bi ? -1 : 1;
    return a.relPath < b.relPath ? -1 : a.relPath > b.relPath ? 1 : 0;
  }
  switch (option) {
    case "recently-modified": {
      if (a.mtimeMs !== b.mtimeMs) return b.mtimeMs - a.mtimeMs;
      return tieBreak();
    }
    case "oldest-first": {
      if (a.mtimeMs !== b.mtimeMs) return a.mtimeMs - b.mtimeMs;
      return tieBreak();
    }
    case "title-asc": {
      if (a.title !== b.title) return a.title < b.title ? -1 : 1;
      return tieBreak();
    }
    case "title-desc": {
      if (a.title !== b.title) return a.title < b.title ? 1 : -1;
      return tieBreak();
    }
    case "id-asc": {
      const ai = a.workItemId ?? "";
      const bi = b.workItemId ?? "";
      if (ai !== bi) return ai < bi ? -1 : 1;
      return a.relPath < b.relPath ? -1 : a.relPath > b.relPath ? 1 : 0;
    }
  }
}

function matchesSelection(
  entry: IndexEntry,
  selection: LibrarySelectionPerType,
): boolean {
  for (const [facetId, options] of Object.entries(selection)) {
    if (!options || options.length === 0) continue;
    let entryValue: string | null = null;
    if (facetId === "status") entryValue = statusValue(entry) || null;
    else if (facetId === "clusterSlug") entryValue = entry.slug;
    else if (facetId === "project") {
      if (entry.workItemId) {
        const idx = entry.workItemId.indexOf("-");
        entryValue = idx > 0 ? entry.workItemId.slice(0, idx) : null;
      }
    }
    if (entryValue === null) return false;
    if (!options.includes(entryValue)) return false;
  }
  return true;
}

interface Props {
  type?: DocTypeKey;
}

export function LibraryTypeView({ type: propType }: Props) {
  const params = useParams({ strict: false }) as {
    type?: string;
    fileSlug?: string;
  };
  const rawType = propType ?? params.type;
  const type: DocTypeKey | undefined =
    rawType && isDocTypeKey(rawType) ? rawType : undefined;
  const hasFileSlug = Boolean(params.fileSlug);

  useMarkDocTypeSeen(hasFileSlug ? undefined : type);

  const [sortOption, setSortOption] = useState<SortOption>("recently-modified");
  const [selection, setSelection] = useState<LibrarySelectionPerType>({});

  const {
    data: entries = [],
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: type ? queryKeys.docs(type) : (["docs", "__invalid__"] as const),
    // biome-ignore lint/style/noNonNullAssertion: queryFn only runs while `enabled: type !== undefined`, so `type` is guaranteed defined here
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  });

  const structureSelection = useMemo(
    () => (type ? { [type]: selection } : undefined),
    [type, selection],
  );
  const structureQuery = useQuery({
    queryKey: queryKeys.libraryStructure(structureSelection),
    queryFn: () => fetchLibraryStructure(structureSelection),
    enabled: type !== undefined,
    placeholderData: keepPreviousData,
  });

  const currentTypeData: LibraryDocType | undefined = useMemo(() => {
    const data = structureQuery.data;
    if (!data || !type) return undefined;
    for (const phase of data.phases) {
      const found = phase.docTypes.find((dt) => dt.id === type);
      if (found) return found;
    }
    if (data.templates.id === type) return data.templates;
    return undefined;
  }, [structureQuery.data, type]);

  const filteredEntries = useMemo(
    () => entries.filter((e) => matchesSelection(e, selection)),
    [entries, selection],
  );
  const sorted = useMemo(
    () => [...filteredEntries].sort((a, b) => compareEntries(a, b, sortOption)),
    [filteredEntries, sortOption],
  );

  if (params.fileSlug) return <Outlet />;

  if (type === undefined) {
    return <p role="alert">Unknown doc type: {String(rawType)}</p>;
  }
  if (isLoading) {
    return (
      <Page
        eyebrow={<EyebrowLabel type={type} />}
        title={DOC_TYPE_LABELS[type]}
        subtitle="Loading…"
      >
        <p>Loading…</p>
      </Page>
    );
  }
  if (isError) {
    return (
      <Page
        eyebrow={<EyebrowLabel type={type} />}
        title={DOC_TYPE_LABELS[type]}
      >
        <p role="alert" className={styles.error}>
          Failed to load documents:{" "}
          {error instanceof Error ? error.message : String(error)}
        </p>
      </Page>
    );
  }

  const totalCount = entries.length;
  const filteredCount = filteredEntries.length;
  const isDocTypeEmpty = totalCount === 0;
  const isFilterEmpty = filteredCount === 0 && totalCount > 0;

  if (isDocTypeEmpty) {
    return (
      <Page
        eyebrow={<EyebrowLabel type={type} />}
        title={DOC_TYPE_LABELS[type]}
      >
        <EmptyState docType={type} />
      </Page>
    );
  }

  const facets = currentTypeData?.filterFacets ?? [];

  return (
    <Page
      eyebrow={<EyebrowLabel type={type} />}
      title={DOC_TYPE_LABELS[type]}
      subtitle={`${filteredCount} ${filteredCount === 1 ? "document" : "documents"}`}
      actions={
        <>
          <SortPill value={sortOption} onChange={setSortOption} />
          <FilterPill
            facets={facets}
            selection={selection}
            onChange={setSelection}
            isFetching={structureQuery.isFetching}
          />
        </>
      }
    >
      {isFilterEmpty ? (
        <NoResultsPanel
          selection={selection}
          facets={facets}
          onClear={() => setSelection({})}
        />
      ) : (
        // biome-ignore lint/a11y/useSemanticElements: intentional ARIA grid on div/span to preserve the existing flex/grid layout and styling; native <table> migration is deferred to a dedicated a11y pass
        <div className={styles.rows} role="table">
          {/* biome-ignore lint/a11y/useSemanticElements: see role="table" above — div/span grid preserved on purpose */}
          {/* biome-ignore lint/a11y/useFocusableInteractive: table-structure rows are presentation of tabular data, not keyboard tab-stops; only the data-row links are focusable */}
          <div className={styles.headerRow} role="row">
            {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid header cell — div/span layout preserved */}
            {/* biome-ignore lint/a11y/useFocusableInteractive: column headers are not focusable tab-stops */}
            <span role="columnheader">ID / DATE</span>
            {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid header cell — div/span layout preserved */}
            {/* biome-ignore lint/a11y/useFocusableInteractive: column headers are not focusable tab-stops */}
            <span role="columnheader">TITLE</span>
            {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid header cell — div/span layout preserved */}
            {/* biome-ignore lint/a11y/useFocusableInteractive: column headers are not focusable tab-stops */}
            <span role="columnheader">STATUS</span>
            {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid header cell — div/span layout preserved */}
            {/* biome-ignore lint/a11y/useFocusableInteractive: column headers are not focusable tab-stops */}
            <span role="columnheader">SLUG</span>
            {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid header cell — div/span layout preserved */}
            {/* biome-ignore lint/a11y/useFocusableInteractive: column headers are not focusable tab-stops */}
            <span role="columnheader">MODIFIED</span>
          </div>
          {sorted.map((entry) => {
            const first = firstColumnContent(entry);
            return (
              <Link
                key={entry.relPath}
                to="/library/$type/$fileSlug"
                params={{
                  type,
                  fileSlug: entry.slug ?? fileSlugFromRelPath(entry.relPath),
                }}
                role="row"
                className={styles.row}
              >
                {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid cell — div/span layout preserved */}
                <span role="cell" className={styles.firstCol}>
                  {first.kind === "empty" ? "—" : first.value}
                </span>
                {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid cell — div/span layout preserved */}
                <span role="cell" className={styles.titleCell}>
                  {entry.title}
                </span>
                {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid cell — div/span layout preserved */}
                <span role="cell">
                  {statusValue(entry) ? (
                    <Chip variant={statusToVariant(statusValue(entry))}>
                      {statusValue(entry)}
                    </Chip>
                  ) : (
                    "—"
                  )}
                </span>
                {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid cell — div/span layout preserved */}
                <span role="cell" className={styles.slug}>
                  {entry.slug ?? "—"}
                </span>
                {/* biome-ignore lint/a11y/useSemanticElements: ARIA grid cell — div/span layout preserved */}
                <span role="cell" className={styles.mtime}>
                  {formatMtime(entry.mtimeMs)}
                </span>
              </Link>
            );
          })}
        </div>
      )}
    </Page>
  );
}
