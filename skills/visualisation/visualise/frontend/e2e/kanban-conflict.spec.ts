import { test, expect } from './fixtures.js'
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dndDrag } from './dnd.js'

const WORK_ITEM_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../server/tests/fixtures/meta/work/0006-conflict-test-work-item.md',
)

test('stale ETag produces an assertive error toast and reverts the card to draft', async ({ page }) => {
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

    // An assertive, persistent error toast should appear (replacing the old
    // inline conflict banner).
    const errorRegion = page.getByTestId('toaster-region-assertive')
    await expect(errorRegion.getByText('Move failed')).toBeVisible({ timeout: 8000 })
    await expect(errorRegion.getByText(/updated by another editor/i)).toBeVisible()

    // Card should have snapped back to draft (optimistic rollback)
    await expect(
      page.locator(
        'section[data-column="draft"] li[data-relpath="tests/fixtures/meta/work/0006-conflict-test-work-item.md"]',
      ),
    ).toBeVisible({ timeout: 5000 })

    // The error toast is persistent: it survives past the info/ok auto-dismiss
    // window (5s) rather than disappearing on its own.
    await page.waitForTimeout(5_500)
    await expect(errorRegion.getByText('Move failed')).toBeVisible()
  } finally {
    writeFileSync(WORK_ITEM_PATH, original)
  }
})
