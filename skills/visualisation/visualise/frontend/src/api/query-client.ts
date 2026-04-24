import { QueryClient } from '@tanstack/react-query'

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // SSE (useDocEvents) is the authoritative invalidator for every
      // server-backed cache — file edits trigger doc-changed events that
      // invalidate docs/docContent/lifecycle/kanban. A time-based staleness
      // threshold would only cause redundant refetches on focus / remount.
      staleTime: Infinity,
      retry: 1,
    },
  },
})
