import { test, expect } from './fixtures.js'
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const WORK_ITEM_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../server/tests/fixtures/meta/work/0006-conflict-test-work-item.md',
)

async function dndDrag(
  page: import('@playwright/test').Page,
  sourceSelector: string,
  targetSelector: string,
) {
  const source = page.locator(sourceSelector)
  const target = page.locator(targetSelector)
  const srcBox = (await source.boundingBox())!
  const tgtBox = (await target.boundingBox())!
  const srcX = srcBox.x + srcBox.width / 2
  const srcY = srcBox.y + srcBox.height / 2
  const tgtX = tgtBox.x + tgtBox.width / 2
  const tgtY = tgtBox.y + tgtBox.height / 2
  await page.mouse.move(srcX, srcY)
  await page.mouse.down()
  await page.mouse.move(srcX + 30, srcY, { steps: 10 })
  await page.mouse.move(tgtX, tgtY, { steps: 20 })
  await page.mouse.up()
}

test('stale ETag produces conflict banner, card stays in draft', async ({ page }) => {
  const original = readFileSync(WORK_ITEM_PATH, 'utf-8')

  // Intercept PATCH: mutate the file on disk before the request completes
  // so the server sees a changed ETag and returns 412.
  // route.fetch() awaits the full server response (unlike route.continue()
  // which is fire-and-forget). We restore the file before fulfilling so
  // the post-conflict invalidation refetch sees todo and the card stays put.
  await page.route('**/api/docs/**/frontmatter', async (route) => {
    if (route.request().method() !== 'PATCH') {
      await route.continue()
      return
    }
    writeFileSync(WORK_ITEM_PATH, original.replace('status: draft', 'status: done'))
    // Short pause so the server's watcher can detect the change
    await new Promise((r) => setTimeout(r, 300))
    const response = await route.fetch()
    writeFileSync(WORK_ITEM_PATH, original)
    await route.fulfill({ response })
  })

  try {
    await page.goto('/kanban')
    await expect(
      page.locator('li[data-relpath="tests/fixtures/meta/work/0006-conflict-test-work-item.md"]'),
    ).toBeVisible()

    await dndDrag(
      page,
      'li[data-relpath="tests/fixtures/meta/work/0006-conflict-test-work-item.md"] a',
      'section[data-column="in-progress"]',
    )

    // Conflict banner should appear
    await expect(
      page.locator('[role="alert"][aria-atomic="true"]'),
    ).toBeVisible({ timeout: 8000 })

    // Card should have snapped back to draft (optimistic rollback)
    await expect(
      page.locator(
        'section[data-column="draft"] li[data-relpath="tests/fixtures/meta/work/0006-conflict-test-work-item.md"]',
      ),
    ).toBeVisible({ timeout: 5000 })
  } finally {
    writeFileSync(WORK_ITEM_PATH, original)
  }
})
