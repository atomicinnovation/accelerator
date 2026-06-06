/**
 * Phase 6 cross-config verification: drag, drop, toast, and keyboard behaviours
 * hold against both a 3-column and a 5-column config, the latter including a
 * ≥30-char label that wraps to ≥2 lines (the specific layout risk: a wrapping
 * header must not break the column's drop target).
 *
 * The whole data path is mocked (installMockBoard) so each config runs from a
 * deterministic, isolated state — no shared on-disk fixture / file-watcher race.
 */
import { test, expect } from './fixtures.js'
import { dndDrag } from './dnd.js'
import { installMockBoard } from './mock-board.js'

const RELPATH = 'meta/work/0001-cross-config.md'
const CARD = `li[data-relpath="${RELPATH}"]`
const CARD_FIXTURE = {
  relPath: RELPATH,
  workItemId: '0001',
  title: 'Cross-config card',
  status: 'draft',
}

const THREE_COLUMNS = [
  { key: 'draft', label: 'Draft' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'done', label: 'Done' },
]

// ≥30 chars; long enough to wrap to ≥2 lines at a 16rem column width.
const LONG_LABEL = 'Awaiting downstream review and final sign-off'
const FIVE_COLUMNS = [
  { key: 'draft', label: 'Draft' },
  { key: 'ready', label: 'Ready' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'review', label: LONG_LABEL },
  { key: 'done', label: 'Done' },
]

const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

for (const { name, columns, target, targetLabel } of [
  { name: '3-column', columns: THREE_COLUMNS, target: 'in-progress', targetLabel: 'In progress' },
  { name: '5-column (wrapping label)', columns: FIVE_COLUMNS, target: 'review', targetLabel: LONG_LABEL },
] as const) {
  test.describe(`kanban ${name} config`, () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: 1024, height: 800 })
      await installMockBoard(page, { columns: [...columns], cards: [CARD_FIXTURE] })
    })

    test('renders exactly the configured columns', async ({ page }) => {
      await page.goto('/kanban')
      for (const { key } of columns) {
        await expect(page.locator(`section[data-column="${key}"]`)).toBeVisible()
      }
    })

    test('drag + drop raises a success toast and the card stays', async ({ page }) => {
      await page.goto('/kanban')
      await expect(page.locator(`section[data-column="draft"] ${CARD}`)).toBeVisible()

      await dndDrag(page, `${CARD} a`, `section[data-column="${target}"]`)

      await expect(page.locator(`section[data-column="${target}"] ${CARD}`)).toBeVisible({
        timeout: 5000,
      })
      await expect(
        page
          .getByTestId('toaster-region-polite')
          .getByText(new RegExp(`moved to ${escapeRe(targetLabel)}`, 'i')),
      ).toBeVisible({ timeout: 5000 })
    })

    test('keyboard move completes across columns', async ({ page }) => {
      await page.goto('/kanban')
      await expect(page.locator(`section[data-column="draft"] ${CARD}`)).toBeVisible()

      const anchor = page.locator(`${CARD} a`)
      await anchor.focus()
      await expect(anchor).toBeFocused()
      await page.keyboard.press('Space')
      await page.waitForTimeout(150)
      await page.keyboard.press('ArrowRight')
      await page.waitForTimeout(150)
      await page.keyboard.press('Space')

      await expect(page.locator(`section[data-column="draft"] ${CARD}`)).toHaveCount(0, {
        timeout: 5000,
      })
      await expect(page.locator(CARD)).toBeVisible()
    })
  })
}

test('the ≥30-char column label wraps to ≥2 lines without breaking the drop target', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1024, height: 800 })
  await installMockBoard(page, { columns: [...FIVE_COLUMNS], cards: [CARD_FIXTURE] })
  await page.goto('/kanban')

  const reviewColumn = page.locator('section[data-column="review"]')
  await expect(reviewColumn).toBeVisible()

  // Measure against the real glyphs, not the font-display: swap fallback.
  await page.evaluate(() => document.fonts.ready)

  const reviewHeadingHeight = await reviewColumn
    .locator('h2')
    .evaluate((el) => el.clientHeight)
  const draftHeadingHeight = await page
    .locator('section[data-column="draft"] h2')
    .evaluate((el) => el.clientHeight)

  // The long label occupies clearly more than the single-line short label.
  expect(reviewHeadingHeight).toBeGreaterThan(draftHeadingHeight * 1.5)

  // The drop target is intact despite the wrap: the column still renders as a
  // droppable section with its full label.
  await expect(reviewColumn.getByRole('heading', { name: LONG_LABEL })).toBeVisible()
})
