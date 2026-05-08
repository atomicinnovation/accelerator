import { createContext, useCallback, useContext, useEffect, useState } from 'react'
import { safeGetItem, safeSetItem } from './safe-storage'
import { THEME_STORAGE_KEY } from './storage-keys'

export type Theme = 'light' | 'dark'
export { THEME_STORAGE_KEY }

export interface ThemeHandle {
  theme: Theme
  setTheme(t: Theme): void
  toggleTheme(): void
}

export function isTheme(v: unknown): v is Theme {
  return v === 'light' || v === 'dark'
}

function readInitial(prefersDark: () => boolean): Theme {
  const attr = document.documentElement.getAttribute('data-theme')
  if (isTheme(attr)) return attr
  const stored = safeGetItem(THEME_STORAGE_KEY)
  if (isTheme(stored)) return stored
  return prefersDark() ? 'dark' : 'light'
}

export function makeUseTheme(prefersDark: () => boolean) {
  return function useTheme(): ThemeHandle {
    const [theme, setThemeState] = useState<Theme>(() => readInitial(prefersDark))

    useEffect(() => {
      document.documentElement.setAttribute('data-theme', theme)
    }, [theme])

    const setTheme = useCallback((t: Theme) => {
      setThemeState(t)
      safeSetItem(THEME_STORAGE_KEY, t)
    }, [])

    // toggleTheme routes through setTheme so persistence happens at a
    // single call site, outside the state-updater function. Calling
    // safeSetItem inside a setState updater would violate React's
    // purity contract and double-fire under StrictMode dev /
    // concurrent rendering.
    const toggleTheme = useCallback(() => {
      setTheme(theme === 'light' ? 'dark' : 'light')
    }, [theme, setTheme])

    return { theme, setTheme, toggleTheme }
  }
}

/**
 * OWNING hook — call EXACTLY ONCE at the RootLayout level. Returns
 * a fresh ThemeHandle whose value must be supplied to
 * <ThemeContext.Provider value={...}>. Leaf components must NOT
 * call this — calling it creates a parallel state machine that
 * does not observe the Provider. Use `useThemeContext()` instead.
 */
export const useTheme = makeUseTheme(
  () => window.matchMedia('(prefers-color-scheme: dark)').matches,
)

const _defaultHandle: ThemeHandle = {
  theme: 'light',
  setTheme: () => {},
  toggleTheme: () => {},
}

export const ThemeContext = createContext<ThemeHandle>(_defaultHandle)

/**
 * CONSUMER hook — reads the ThemeContext provided by RootLayout.
 * This is the hook every component should use to read or change
 * the current theme.
 */
export function useThemeContext(): ThemeHandle {
  return useContext(ThemeContext)
}
