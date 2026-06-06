/**
 * A1/A3 (overlay + source persistence) and the onDragCancel SSE-gate
 * regression. These need dnd-kit's real active-drag state, which jsdom cannot
 * reproduce — so they are E2E.
 */
import { test, expect } from './fixtures.js'
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { startDrag } from './dnd.js'

const FIXTURES_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../server/tests/fixtures/meta/work',
)
const CARD_0001 =
  'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"]'
const CARD_0005 =
  'li[data-relpath="tests/fixtures/meta/work/0005-sse-test-work-item.md"]'
const WORK_ITEM_0005_PATH = resolve(FIXTURES_DIR, '0005-sse-test-work-item.md')

test('source card persists in its column while the lifted overlay clone renders (A1/A3)', async ({ page }) => {
  await page.goto('/kanban')
  await expect(page.locator(CARD_0001)).toBeVisible()

  // Drag within the same column (no status change) and pause mid-drag.
  const drag = await startDrag(page, `${CARD_0001} a`)
  await drag.moveBy(0, 40)

  // The lifted overlay clone is present AND the source card still occupies its
  // slot — A3 (no source displacement) is a rendering question, not a data move.
  await expect(page.locator('[data-overlay]')).toBeVisible()
  await expect(page.locator(CARD_0001)).toBeVisible()

  await drag.drop()
})

test('Escape-cancelling a drag clears the SSE gate so later updates still render', async ({
  page,
}) => {
  const original = readFileSync(WORK_ITEM_0005_PATH, 'utf-8')
  try {
    await page.goto('/kanban')
    await expect(page.locator('[data-sse-state="open"]')).toBeVisible({ timeout: 5000 })
    await expect(page.locator(CARD_0005)).toBeVisible()

    // Begin a drag, then cancel it with Escape (dnd-kit fires onDragCancel
    // INSTEAD OF onDragEnd — endDrag must still clear setDragInProgress).
    const drag = await startDrag(page, `${CARD_0001} a`)
    await drag.moveBy(0, 40)
    await page.keyboard.press('Escape')
    await drag.drop()

    // An external edit now arrives via SSE. If the cancel path left the gate
    // stuck true, this invalidation would be queued forever and the card would
    // never move — the stuck-gate failure mode this guards.
    writeFileSync(WORK_ITEM_0005_PATH, original.replace(/^status:.*$/m, 'status: done'))
    await expect(
      page.locator(`section[data-column="done"] ${CARD_0005}`),
    ).toBeVisible({ timeout: 10000 })
  } finally {
    writeFileSync(WORK_ITEM_0005_PATH, original)
  }
})
