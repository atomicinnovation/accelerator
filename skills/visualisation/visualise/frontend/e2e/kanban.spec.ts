import { test, expect } from './fixtures.js'
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const FIXTURES_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../server/tests/fixtures/meta/tickets',
)
const TICKET_0001_PATH = resolve(FIXTURES_DIR, '0001-first-ticket.md')
const TICKET_0005_PATH = resolve(FIXTURES_DIR, '0005-sse-test-ticket.md')

// dnd-kit's PointerSensor requires 5px of movement before activating.
// We simulate a full pointer-down → move → move → up sequence so the
// sensor fires, rather than using page.dragTo() which is a single step.
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
  // Move well past the 5px activation threshold so dnd-kit fires before the anchor click
  await page.mouse.move(srcX + 30, srcY, { steps: 10 })
  await page.mouse.move(tgtX, tgtY, { steps: 20 })
  await page.mouse.up()
}

test('drag todo card to in-progress column', async ({ page }) => {
  const original = readFileSync(TICKET_0001_PATH, 'utf-8')
  try {
    await page.goto('/kanban')

    const todoCard = page.locator(
      'li[data-relpath="tests/fixtures/meta/tickets/0001-first-ticket.md"]',
    )
    const inProgressColumn = page.locator('section[data-column="in-progress"]')

    await expect(todoCard).toBeVisible()

    await dndDrag(
      page,
      'li[data-relpath="tests/fixtures/meta/tickets/0001-first-ticket.md"] a',
      'section[data-column="in-progress"]',
    )

    // Card should now appear in the in-progress column
    await expect(
      inProgressColumn.locator(
        'li[data-relpath="tests/fixtures/meta/tickets/0001-first-ticket.md"]',
      ),
    ).toBeVisible({ timeout: 5000 })
  } finally {
    writeFileSync(TICKET_0001_PATH, original)
  }
})

test('second tab receives SSE update after drag', async ({ page, context }) => {
  test.setTimeout(60_000)
  const original = readFileSync(TICKET_0005_PATH, 'utf-8')
  try {
    const page2 = await context.newPage()
    await page.goto('/kanban')
    await page2.goto('/kanban')

    // Wait until both pages have established their SSE connections so neither
    // misses the doc-changed broadcast.
    await expect(page.locator('[data-sse-state="open"]')).toBeVisible({ timeout: 5000 })
    await expect(page2.locator('[data-sse-state="open"]')).toBeVisible({ timeout: 5000 })

    // Arm the PATCH response interceptor before the drag so we don't race.
    const patchDone = page.waitForResponse(
      (r) => r.url().includes('/api/docs/') && r.request().method() === 'PATCH',
    )

    await dndDrag(
      page,
      'li[data-relpath="tests/fixtures/meta/tickets/0005-sse-test-ticket.md"] a',
      'section[data-column="in-progress"]',
    )

    // Confirm optimistic update on page 1.
    await expect(
      page.locator(
        'section[data-column="in-progress"] li[data-relpath="tests/fixtures/meta/tickets/0005-sse-test-ticket.md"]',
      ),
    ).toBeVisible({ timeout: 5000 })

    await patchDone

    // Bring page 2 to the front before asserting to avoid background-tab throttling.
    await page2.bringToFront()

    // Second tab should receive the SSE event and move the card.
    await expect(
      page2.locator(
        'section[data-column="in-progress"] li[data-relpath="tests/fixtures/meta/tickets/0005-sse-test-ticket.md"]',
      ),
    ).toBeVisible({ timeout: 10000 })
  } finally {
    writeFileSync(TICKET_0005_PATH, original)
  }
})
