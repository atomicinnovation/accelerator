// CSS-free constants for `Glyph`. Kept in a separate module so consumers
// that only need the type/runtime list (notably the Playwright visual-
// regression spec, whose TS transformer can't parse CSS modules) can
// import them without pulling `Glyph.module.css` into the import graph.

import { type DocTypeKey, DOC_TYPE_KEYS, VIRTUAL_DOC_TYPE_KEYS } from '../../api/types'

/**
 * The 12 non-virtual `DocTypeKey` values Glyph renders.
 *
 * INVARIANT: Glyph is for real document types only. Virtual keys (currently
 * `templates`) are excluded by construction. The exclusion is data-driven: the
 * runtime `GLYPH_DOC_TYPE_KEYS` below filters by `VIRTUAL_DOC_TYPE_KEYS`, so
 * adding a future virtual key in `api/types.ts` automatically removes it from
 * Glyph's set. The type alias must be updated in lock-step (extend the
 * `Exclude` to cover the new virtual key) — caught at unit-test time by the
 * exhaustiveness assertion on `ICON_COMPONENTS`.
 */
export type GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>

/** Runtime mirror of `GlyphDocTypeKey`. Derived from `VIRTUAL_DOC_TYPE_KEYS`
 *  at module load — assumes the virtual-keys list is statically resolvable. */
export const GLYPH_DOC_TYPE_KEYS: readonly GlyphDocTypeKey[] = DOC_TYPE_KEYS.filter(
  (k): k is GlyphDocTypeKey => !VIRTUAL_DOC_TYPE_KEYS.includes(k),
)

/** Narrow `DocTypeKey` to `GlyphDocTypeKey`. Use in data-driven consumers. */
export function isGlyphDocTypeKey(k: DocTypeKey): k is GlyphDocTypeKey {
  return GLYPH_DOC_TYPE_KEYS.includes(k as GlyphDocTypeKey)
}
