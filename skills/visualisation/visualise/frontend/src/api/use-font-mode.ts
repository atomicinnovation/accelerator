import { createContext, useCallback, useContext, useEffect, useState } from 'react'
import { safeGetItem, safeSetItem } from './safe-storage'
import { FONT_MODE_STORAGE_KEY } from './storage-keys'

export type FontMode = 'display' | 'mono'
export { FONT_MODE_STORAGE_KEY }

export interface FontModeHandle {
  fontMode: FontMode
  setFontMode(m: FontMode): void
  toggleFontMode(): void
}

export function isFontMode(v: unknown): v is FontMode {
  return v === 'display' || v === 'mono'
}

function readInitial(): FontMode {
  const attr = document.documentElement.getAttribute('data-font')
  if (isFontMode(attr)) return attr
  const stored = safeGetItem(FONT_MODE_STORAGE_KEY)
  if (isFontMode(stored)) return stored
  return 'display'
}

/**
 * OWNING hook — call EXACTLY ONCE at the RootLayout level. Returns
 * a fresh FontModeHandle whose value must be supplied to
 * <FontModeContext.Provider value={...}>. Leaf components must NOT
 * call this directly — use `useFontModeContext()`.
 */
export function useFontMode(): FontModeHandle {
  const [fontMode, setState] = useState<FontMode>(() => readInitial())

  useEffect(() => {
    document.documentElement.setAttribute('data-font', fontMode)
  }, [fontMode])

  const setFontMode = useCallback((m: FontMode) => {
    setState(m)
    safeSetItem(FONT_MODE_STORAGE_KEY, m)
  }, [])

  // toggleFontMode routes through setFontMode so persistence happens
  // outside the state-updater function — see use-theme.ts for
  // rationale (React purity / StrictMode double-invoke).
  const toggleFontMode = useCallback(() => {
    setFontMode(fontMode === 'display' ? 'mono' : 'display')
  }, [fontMode, setFontMode])

  return { fontMode, setFontMode, toggleFontMode }
}

const _defaultHandle: FontModeHandle = {
  fontMode: 'display',
  setFontMode: () => {},
  toggleFontMode: () => {},
}

export const FontModeContext = createContext<FontModeHandle>(_defaultHandle)

/**
 * CONSUMER hook — reads the FontModeContext provided by RootLayout.
 * Use this from any component that needs to read or change the
 * current font mode.
 */
export function useFontModeContext(): FontModeHandle {
  return useContext(FontModeContext)
}
