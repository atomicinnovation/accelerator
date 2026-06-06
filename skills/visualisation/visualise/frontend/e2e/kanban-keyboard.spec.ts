/**
 * C1 (keyboard cross-column move) and C3 (focus returns to the moved card's
 * anchor after the move settles). dnd-kit's sortableKeyboardCoordinates needs
 * real layout boxes jsdom does not provide, so these are E2E.
 *
 * C1 has no existing page.keyboard precedent in this suite — treat a failure
 * here as the feasibility-spike outcome (e.g. the keyboard sensor needs
 * configuration), not as a flaky test to retry.
 */
import { test, expect } from './fixtures.js'
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dndDrag } from './dnd.js'

const WORK_ITEM_0001_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../server/tests/fixtures/meta/work/0001-first-work-item.md',
)
const CARD_0001 =
  'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"]'

test('keyboard: Space → arrows → Space completes a cross-column move (C1)', async ({ page }) => {
  const original = readFileSync(WORK_ITEM_0001_PATH, 'utf-8')
  try {
    await page.goto('/kanban')
    await expect(page.locator(`section[data-column="draft"] ${CARD_0001}`)).toBeVisible()

    const patchDone = page.waitForResponse(
      (r) => r.url().includes('/api/docs/') && r.request().method() === 'PATCH',
    )

    // Focus the draggable's listeners element (the card anchor) and drive the
    // KeyboardSensor: Space to pick up, ArrowRight to cross to the next column,
    // Space to drop. Space (not Enter) avoids the anchor's native activation.
    await page.locator(`${CARD_0001} a`).focus()
    await page.keyboard.press('Space')
    await page.keyboard.press('ArrowRight')
    await page.keyboard.press('Space')

    // The card left the draft column (optimistic move) and the PATCH fires.
    await expect(page.locator(`section[data-column="draft"] ${CARD_0001}`)).toHaveCount(0, {
      timeout: 5000,
    })
    await expect(page.locator(CARD_0001)).toBeVisible()
    await patchDone
  } finally {
    writeFileSync(WORK_ITEM_0001_PATH, original)
  }
})

test('focus returns to the moved card anchor in its resting column after settle (C3)', async ({ page }) => {
  const original = readFileSync(WORK_ITEM_0001_PATH, 'utf-8')
  try {
    await page.goto('/kanban')
    await expect(page.locator(CARD_0001)).toBeVisible()

    const patchDone = page.waitForResponse(
      (r) => r.url().includes('/api/docs/') && r.request().method() === 'PATCH',
    )
    await dndDrag(page, `${CARD_0001} a`, 'section[data-column="ready"]')
    await patchDone

    // After the onSettled invalidation resolves and the node remounts, focus is
    // restored to the card's <Link> anchor (relPath-keyed, so it resolves to the
    // live node in its resting column).
    await expect(page.locator(`${CARD_0001} a`)).toBeFocused({ timeout: 5000 })
  } finally {
    writeFileSync(WORK_ITEM_0001_PATH, original)
  }
})
