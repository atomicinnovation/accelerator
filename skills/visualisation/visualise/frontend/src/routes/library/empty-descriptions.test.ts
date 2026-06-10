import { describe, it, expect } from 'vitest'
import { TYPE_COPY } from './empty-descriptions'
import { DOC_TYPE_HUE } from '../../styles/tokens'
import { DOC_TYPE_KEYS } from '../../api/types'

describe('empty-descriptions hue single-sourcing', () => {
  // TYPE_COPY.hue is sourced from DOC_TYPE_HUE (styles/tokens). This pins the
  // refactor's value-preservation as an enforced invariant — a transcription
  // slip in the lifted hue map would fail here rather than silently drifting
  // the empty-state gradient panel from the BigGlyph hero.
  it.each(DOC_TYPE_KEYS)('TYPE_COPY[%s].hue equals DOC_TYPE_HUE', (key) => {
    expect(TYPE_COPY[key].hue).toBe(DOC_TYPE_HUE[key])
  })
})
