import { useQueries } from "@tanstack/react-query";
import { fetchDocs } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import {
  type ActivityEvent,
  type IndexEntry,
  isPhysicalDocTypeKey,
} from "../../api/types";

export function useActivityRowDocuments(
  rows: readonly ActivityEvent[],
): (row: ActivityEvent) => IndexEntry | undefined {
  const docTypes = [...new Set(rows.map((r) => r.docType))].filter(
    isPhysicalDocTypeKey,
  );
  const listings = useQueries({
    queries: docTypes.map((type) => ({
      queryKey: queryKeys.docs(type),
      queryFn: () => fetchDocs(type),
    })),
  });
  const entriesByPath = new Map<string, IndexEntry>(
    listings.flatMap((listing) =>
      (listing.data ?? []).map((entry) => [entry.relPath, entry] as const),
    ),
  );
  return (row) => entriesByPath.get(row.path);
}
