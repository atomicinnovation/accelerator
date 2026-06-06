/**
 * A2: a drag must never trigger the card's <Link> navigation, while a genuine
 * click always navigates. jsdom cannot reproduce the PointerSensor 5px
 * activation or the synthetic post-drag click, so this is the authoritative
 * oracle for the click-vs-drag guard.
 */
import { test, expect } from './fixtures.js'
import { startDrag } from './dnd.js'

const CARD =
  'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"]'

test('dragging a card and releasing does NOT navigate', async ({ page }) => {
  await page.goto('/kanban')
  await expect(page.locator(CARD)).toBeVisible()

  // Drag within the same column (no status change, no file mutation) and drop.
  // Without the guard the synthetic post-drag click would follow the anchor.
  const drag = await startDrag(page, `${CARD} a`)
  await drag.moveBy(0, 40)
  await drag.drop()

  // Still on the board — navigation was suppressed.
  await expect(page).toHaveURL(/\/kanban$/)
  await expect(page.locator(CARD)).toBeVisible()
})

test('clicking a card (no drag) navigates to its library page', async ({ page }) => {
  await page.goto('/kanban')
  await expect(page.locator(CARD)).toBeVisible()

  await page.locator(`${CARD} a`).click()

  await expect(page).toHaveURL(/\/library\/work-items\/0001-first-work-item$/)
})
