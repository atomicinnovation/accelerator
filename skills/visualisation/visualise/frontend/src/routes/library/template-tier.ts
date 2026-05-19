import type { TemplateTierSource } from '../../api/types'

export const TIER_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default': 'plugin default',
  'user-override': 'user override',
  'config-override': 'config override',
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
