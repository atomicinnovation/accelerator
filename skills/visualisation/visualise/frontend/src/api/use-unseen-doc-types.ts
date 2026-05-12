import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'
import { safeGetItem, safeSetItem } from './safe-storage'
import { SEEN_DOC_TYPES_STORAGE_KEY } from './storage-keys'
import { DOC_TYPE_KEYS, isDocTypeKey, type DocTypeKey, type SseEvent } from './types'

type SeenMap = Partial<Record<DocTypeKey, number>>

export interface UnseenDocTypesHandle {
  unseenSet: ReadonlySet<DocTypeKey>
  markSeen: (type: DocTypeKey) => void
  onEvent: (event: SseEvent) => void
  onReconnect: () => void
}

/** Exported for testing. Reads and sanitises the persisted SeenMap. */
export function parseStored(): SeenMap {
  const raw = safeGetItem(SEEN_DOC_TYPES_STORAGE_KEY)
  if (raw === null) return {}
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    return {}
  }
  if (
    parsed === null ||
    typeof parsed !== 'object' ||
    Array.isArray(parsed)
  ) {
    return {}
  }
  const out: SeenMap = {}
  for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
    if (!isDocTypeKey(key)) continue
    if (typeof value !== 'number' || !Number.isFinite(value)) continue
    out[key] = value
  }
  return out
}

/**
 * OWNING hook — call EXACTLY ONCE at the RootLayout level. Returns a
 * fresh handle whose value must be supplied to
 * <UnseenDocTypesContext.Provider value={...}>. Leaf components must NOT
 * call this directly — use the consumer hooks below.
 */
export function useUnseenDocTypes(): UnseenDocTypesHandle {
  // Persisted: per-type last-seen epoch ms. Survives across mounts.
  // Updated only by markSeen.
  const seenRef = useRef<SeenMap>(parseStored())

  // Transient: which types have unseen activity right now. Empty on
  // mount; never persisted, so N events do not produce N storage writes.
  const [unseenSet, setUnseenSet] = useState<Set<DocTypeKey>>(
    () => new Set(),
  )

  const onEvent = useCallback((event: SseEvent) => {
    // doc-invalid bypasses the self-cause guard (no etag); ignore it
    // entirely — it signals an operational issue, not new content.
    if (event.type !== 'doc-changed') return
    if (!isDocTypeKey(event.docType)) return

    const type = event.docType
    const stored = seenRef.current[type]

    // First event for a never-visited type is silently absorbed: the
    // user has no T to compare against; "since last visit" semantics,
    // not "new to you".
    if (stored === undefined) return

    // Strict greater-than: same-ms markSeen+event resolves as 'seen'.
    if (Date.now() > stored) {
      setUnseenSet((prev) => {
        if (prev.has(type)) return prev
        const next = new Set(prev)
        next.add(type)
        return next
      })
    }
  }, [])

  const markSeen = useCallback((type: DocTypeKey) => {
    seenRef.current = { ...seenRef.current, [type]: Date.now() }
    safeSetItem(SEEN_DOC_TYPES_STORAGE_KEY, JSON.stringify(seenRef.current))
    setUnseenSet((prev) => {
      if (!prev.has(type)) return prev
      const next = new Set(prev)
      next.delete(type)
      return next
    })
  }, [])

  const onReconnect = useCallback(() => {
    // No-op. Real changes during a disconnect are reflected by replayed
    // doc-changed events; onEvent classifies them correctly.
  }, [])

  return { unseenSet, markSeen, onEvent, onReconnect }
}

const noopHandle: UnseenDocTypesHandle = {
  unseenSet: new Set(),
  markSeen: () => {},
  onEvent: () => {},
  onReconnect: () => {},
}

export const UnseenDocTypesContext =
  createContext<UnseenDocTypesHandle>(noopHandle)

/** CONSUMER hook — reads the UnseenDocTypesContext. */
export function useUnseenDocTypesContext(): UnseenDocTypesHandle {
  return useContext(UnseenDocTypesContext)
}

/**
 * CONSUMER hook — bumps T for `type` on mount and whenever `type`
 * changes. Pass `undefined` to opt out (e.g. on child-doc routes
 * that must NOT silently clear the parent type's unseen dot).
 */
export function useMarkDocTypeSeen(type: DocTypeKey | undefined): void {
  const { markSeen } = useUnseenDocTypesContext()
  useEffect(() => {
    if (type) markSeen(type)
  }, [type, markSeen])
}

// Re-export so consumers do not need to import storage-keys directly.
export { SEEN_DOC_TYPES_STORAGE_KEY, DOC_TYPE_KEYS }
