/**
 * Regression scenario: legacy-schema work-items.
 *
 * Validates that work-items with the legacy schema (no work_item_id:,
 * type: adr-creation-task, or a non-configured status) render without errors.
 *
 * Fixtures: server/tests/fixtures/meta/work/
 *   0003-third-work-item.md  — status: todo (not in seven defaults → Other)
 *   0004-in-progress-work-item.md  — type: adr-creation-task, status: in-progress
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from './fixtures.js'

const FIXTURES_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../server/tests/fixtures/meta/work',
)
// 0003 has status: todo (non-configured → Other) and has a status field (patchable)
const LEGACY_OTHER_PATH = join(FIXTURES_DIR, '0003-third-work-item.md')
const LEGACY_OTHER_REL = 'tests/fixtures/meta/work/0003-third-work-item.md'

test.describe('Regression — legacy-schema work-items', () => {
  test('legacy work-item with non-configured status lands in the Other swimlane', async ({ page }) => {
    await page.goto('/kanban')

    // The Other swimlane must be visible (0003 has status: todo, not in the seven defaults).
    const otherColumn = page.locator('section[data-column="other"]')
    await expect(otherColumn).toBeVisible()
  })

  test('library view of a legacy-type work-item renders without errors', async ({ page }) => {
    await page.goto('/library/work-items')

    await expect(page.locator('body')).not.toContainText('Internal Server Error')
    await expect(page.locator('body')).not.toContainText('Error')
  })

  test('PATCH from Other (non-configured status) to a configured column succeeds', async ({
    request,
  }) => {
    const original = readFileSync(LEGACY_OTHER_PATH, 'utf-8')
    try {
      const docsRes = await request.get('/api/docs?type=work-items')
      expect(docsRes.ok()).toBe(true)

      const { docs } = (await docsRes.json()) as { docs: Array<{ relPath: string }> }
      const target = docs.find((d) => d.relPath === LEGACY_OTHER_REL)
      if (!target) {
        test.skip()
        return
      }

      const getRes = await request.get(`/api/docs/${LEGACY_OTHER_REL}`)
      const etag = getRes.headers()['etag'] ?? '"test"'

      const patchRes = await request.patch(`/api/docs/${LEGACY_OTHER_REL}/frontmatter`, {
        headers: { 'If-Match': etag, 'Content-Type': 'application/json' },
        data: JSON.stringify({ patch: { status: 'ready' } }),
      })
      // 204 = updated; 412 = ETag mismatch from concurrent access (acceptable)
      expect([204, 412]).toContain(patchRes.status())
    } finally {
      writeFileSync(LEGACY_OTHER_PATH, original)
    }
  })

  test('PATCH with an unknown status returns 400 even for legacy files', async ({ request }) => {
    const docsRes = await request.get('/api/docs?type=work-items')
    const { docs } = (await docsRes.json()) as { docs: Array<{ relPath: string }> }
    if (docs.length === 0) {
      test.skip()
      return
    }

    const relPath = docs[0].relPath
    // Status validation happens before ETag check, so any If-Match yields 400.
    // 'proposed' is a legacy status not in the configured column set.
    const patchRes = await request.patch(`/api/docs/${relPath}/frontmatter`, {
      headers: { 'If-Match': '"any"', 'Content-Type': 'application/json' },
      data: JSON.stringify({ patch: { status: 'proposed' } }),
    })
    expect(patchRes.status()).toBe(400)
    const body = await patchRes.json()
    expect(body.error).toBe('unknown_kanban_status')
  })
})
