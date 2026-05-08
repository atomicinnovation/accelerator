import { THEME_STORAGE_KEY, FONT_MODE_STORAGE_KEY } from './storage-keys'

export { BOOT_SCRIPT_SOURCE } from './storage-keys'

export interface BootDeps {
  doc: Document
  storage: Storage | null
  matchPrefersDark: () => boolean
}

/**
 * Apply persisted theme/font-mode attributes to <html>. Called both
 * by the inlined boot script (before React mounts) and by tests
 * (with stubbed deps).
 *
 * If `localStorage` is unavailable or the read throws (private-mode
 * SecurityError), the corresponding attribute is left unset and the
 * CSS prefers-color-scheme mirror handles the paint.
 */
export function applyBootAttributes(deps: BootDeps): void {
  const root = deps.doc.documentElement
  // Theme: only write when storage has a valid entry. Otherwise let
  // CSS prefers-color-scheme govern the paint.
  try {
    const t = deps.storage?.getItem(THEME_STORAGE_KEY)
    if (t === 'light' || t === 'dark') {
      root.setAttribute('data-theme', t)
    }
  } catch { /* SecurityError in private mode — fall through */ }
  // Font-mode: same shape, separate try block so a font-mode failure
  // never overwrites an already-applied data-theme.
  try {
    const f = deps.storage?.getItem(FONT_MODE_STORAGE_KEY)
    if (f === 'display' || f === 'mono') {
      root.setAttribute('data-font', f)
    }
  } catch { /* SecurityError — fall through */ }
}
