import type { TemplateTierSource } from '../../api/types'
import type { GlyphDocTypeKey } from '../../components/Glyph/Glyph.constants'

export const TIER_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default': 'Plugin default',
  'user-override': 'User override',
  'config-override': 'Config override',
}

export const TIER_SHORT_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default': 'default',
  'user-override': 'user',
  'config-override': 'config',
}

/** Fixed left-to-right render order for the index tier-presence row
 *  (resolution order, lowest priority first). */
export const TIER_ORDER: readonly TemplateTierSource[] = [
  'plugin-default',
  'user-override',
  'config-override',
] as const

/** Map a template basename to the doc-type whose glyph best represents it.
 *  Unknown template names get `null` and the caller falls back to a neutral
 *  rendering (no glyph). The keys mirror the names emitted by the resolver
 *  (e.g. seeded_cfg() in server/tests/common/mod.rs). */
export const TEMPLATE_NAME_TO_GLYPH_KEY: Readonly<Record<string, GlyphDocTypeKey>> = {
  'adr': 'decisions',
  'plan': 'plans',
  'research': 'research',
  'validation': 'validations',
  'pr-description': 'pr-descriptions',
  'work-item': 'work-items',
  'design-gap': 'design-gaps',
  'design-inventory': 'design-inventories',
}

export function glyphKeyForTemplate(name: string): GlyphDocTypeKey | null {
  return TEMPLATE_NAME_TO_GLYPH_KEY[name] ?? null
}
