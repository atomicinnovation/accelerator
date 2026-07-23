import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { fetchSearch } from "./fetch";
import { queryKeys } from "./query-keys";
import { useDebouncedValue } from "./use-debounced-value";

export function useSearch(query: string) {
  const debounced = useDebouncedValue(query.trim(), 200);
  const enabled = debounced.length >= 2;
  return useQuery({
    queryKey: enabled
      ? queryKeys.search(debounced)
      : queryKeys.disabled("search"),
    queryFn: ({ signal }) => fetchSearch(debounced, signal),
    enabled,
    placeholderData: keepPreviousData,
  });
}
