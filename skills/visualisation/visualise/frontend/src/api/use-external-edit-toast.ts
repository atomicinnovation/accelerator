import { useEffect, useRef } from 'react'
import { useDocEventsContext } from './use-doc-events'
import { useSelfCauseRegistry } from './self-cause'
import { useActiveDocRelPath } from './use-active-doc-relpath'
import { useToast } from './use-toast'
import type { ActionKind, SseEvent } from './types'

const ACTION_VERB: Record<ActionKind, string> = {
  created: 'created',
  edited: 'updated',
  deleted: 'deleted',
}

export const EXTERNAL_EDIT_HEADING = 'External edit detected'

export function externalEditMessage(relPath: string, action: ActionKind): string {
  return `\`${relPath}\` was ${ACTION_VERB[action]} while you were looking at it.`
}

/**
 * Headless subscriber — mount once inside the Toast + DocEvents providers.
 * Raises an external-edit toast when a non-self-caused doc-changed event
 * targets the document currently being viewed.
 *
 * Sits in the pre-drop `subscribe` slot (use-doc-events.ts) so it can do its
 * own self-cause check; the shared `defaultSelfCauseRegistry` makes the
 * dispatcher's drop and this hook's check agree byte-for-byte.
 */
export function useExternalEditToast(): void {
  const { subscribe } = useDocEventsContext()
  const registry = useSelfCauseRegistry()
  const { showToast } = useToast()
  const relPath = useActiveDocRelPath()

  const ref = useRef({ relPath, registry, showToast })
  ref.current = { relPath, registry, showToast }

  useEffect(() => {
    const unsubscribe = subscribe((event: SseEvent) => {
      if (event.type !== 'doc-changed') return
      const { relPath, registry, showToast } = ref.current
      if (relPath === undefined) return
      if (event.path !== relPath) return
      if (registry.has(event.etag)) return
      showToast({
        heading: EXTERNAL_EDIT_HEADING,
        message: externalEditMessage(relPath, event.action),
      })
    })
    return unsubscribe
  }, [subscribe])
}
