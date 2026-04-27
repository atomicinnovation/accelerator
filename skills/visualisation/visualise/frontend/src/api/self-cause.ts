import { createContext, useContext } from 'react'

export interface SelfCauseRegistry {
  register(etag: string): void
  has(etag: string | undefined): boolean
  reset(): void
}

export interface SelfCauseOptions {
  ttlMs?: number
  maxEntries?: number
  now?: () => number
}

export function createSelfCauseRegistry(opts?: SelfCauseOptions): SelfCauseRegistry {
  const ttlMs = opts?.ttlMs ?? 5_000
  const maxEntries = opts?.maxEntries ?? 256
  const now = opts?.now ?? (() => Date.now())

  // Map from etag → timestamp of registration (insertion-ordered for FIFO)
  const entries = new Map<string, number>()

  function pruneExpired(): void {
    const t = now()
    for (const [k, ts] of entries) {
      if (t - ts >= ttlMs) entries.delete(k)
    }
  }

  return {
    register(etag: string): void {
      pruneExpired()
      if (entries.size >= maxEntries) {
        const oldest = entries.keys().next().value
        if (oldest !== undefined) entries.delete(oldest)
      }
      entries.set(etag, now())
    },
    has(etag: string | undefined): boolean {
      if (etag === undefined) return false
      pruneExpired()
      return entries.has(etag)
    },
    reset(): void {
      entries.clear()
    },
  }
}

export const defaultSelfCauseRegistry: SelfCauseRegistry = createSelfCauseRegistry()

export const SelfCauseContext = createContext<SelfCauseRegistry>(defaultSelfCauseRegistry)

export function useSelfCauseRegistry(): SelfCauseRegistry {
  return useContext(SelfCauseContext)
}
