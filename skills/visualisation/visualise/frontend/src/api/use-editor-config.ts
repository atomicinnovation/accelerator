import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "./query-keys";
import type { EditorConfig } from "./types";

async function fetchEditorConfig(): Promise<EditorConfig> {
  const resp = await fetch("/api/editor/config");
  if (!resp.ok) throw new Error(`/api/editor/config returned ${resp.status}`);
  return resp.json() as Promise<EditorConfig>;
}

export function useEditorConfig() {
  return useQuery({
    queryKey: queryKeys.editor(),
    queryFn: fetchEditorConfig,
    staleTime: Infinity,
  });
}
