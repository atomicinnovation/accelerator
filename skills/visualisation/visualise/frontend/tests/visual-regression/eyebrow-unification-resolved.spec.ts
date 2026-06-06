import { test, expect, type Locator } from '@playwright/test'
import { hexToRgb, setTheme } from './lib/expected-colours'
import { DETAIL_ROUTE_SLUGS } from './lib/detail-route-slugs'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'

// 0079: the page eyebrow, the aside <h3> section labels and the inactive
// panel-variant Pipeline rail label must resolve identically for SIX
// properties — the five typographic ones plus font-weight (added so the
// "drop the explicit weight" decision is machine-checked, not assumed).
//
// Notes:
//  - PipelineMini has no .label (dots only), so that clause is vacuous.
//  - The rail check targets an INACTIVE (data-active='false') PANEL-variant
//    stage label; the active override (--ac-fg) is intentional and untested.
//  - The lifecycle-index CARD-variant labels are out of scope (unchanged) and
//    deliberately not asserted here.

const THEMES = ['light', 'dark'] as const

const SIX_PROPS = [
  'fontFamily',
  'fontSize',
  'fontWeight',
  'letterSpacing',
  'textTransform',
  'color',
] as const
type Resolved = Record<(typeof SIX_PROPS)[number], string>

async function readRecipe(locator: Locator): Promise<Resolved> {
  return locator.evaluate((el) => {
    const cs = getComputedStyle(el)
    return {
      fontFamily: cs.fontFamily,
      fontSize: cs.fontSize,
      fontWeight: cs.fontWeight,
      letterSpacing: cs.letterSpacing,
      textTransform: cs.textTransform,
      color: cs.color,
    }
  })
}

function assertCanonical(recipe: Resolved, faintRgb: string) {
  expect(recipe.fontFamily).toContain('Fira Code')
  expect(recipe.fontSize).toBe('11px')
  // 0.12em (--tracking-caps) at 11px → 1.32px computed.
  expect(recipe.letterSpacing).toBe('1.32px')
  expect(recipe.textTransform).toBe('uppercase')
  expect(recipe.color).toBe(faintRgb)
}

for (const theme of THEMES) {
  const faintRgb = hexToRgb(
    (theme === 'light' ? LIGHT_COLOR_TOKENS : DARK_COLOR_TOKENS)['ac-fg-faint'],
  )

  test.describe(`eyebrow unification — ${theme}`, () => {
    test('page eyebrow and aside section label share the recipe', async ({
      page,
    }) => {
      await page.goto(`/library/plans/${DETAIL_ROUTE_SLUGS.plans}`)
      await setTheme(page, theme)

      const eyebrow = await readRecipe(page.locator('[data-slot="eyebrow"]'))
      assertCanonical(eyebrow, faintRgb)

      const h3 = page.getByRole('heading', {
        level: 3,
        name: 'Related artifacts',
      })
      await expect(h3).toBeVisible()
      const asideRecipe = await readRecipe(h3)
      assertCanonical(asideRecipe, faintRgb)

      // Cross-element identity: the aside label matches the eyebrow on every
      // one of the six properties (incl. font-weight).
      expect(asideRecipe).toEqual(eyebrow)
    })

    test('inactive panel rail label shares the recipe', async ({ page }) => {
      await page.goto('/lifecycle/first-plan')
      await setTheme(page, theme)

      // Guard against a vacuous pass: the inactive panel-label locator must
      // match ≥1 element before reading computed styles. Stable hooks only —
      // the module class .label is hashed and unselectable in the bundle.
      const railLabel = page
        .locator(
          ".ac-stagechain[data-variant='panel'] " +
            ".ac-stagechain__stage[data-active='false'] " +
            '.ac-stagechain__label',
        )
        .first()
      await expect(railLabel).toBeVisible()

      const railRecipe = await readRecipe(railLabel)
      assertCanonical(railRecipe, faintRgb)

      // Cross-element identity against the same-page eyebrow (same .eyebrow
      // rule as the detail page), tying all three call sites together.
      const eyebrow = await readRecipe(page.locator('[data-slot="eyebrow"]'))
      expect(railRecipe).toEqual(eyebrow)
    })
  })
}
