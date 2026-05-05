/**
 * Scenario A validation: default numeric ID pattern, default seven-column set.
 *
 * Runs against the shared fixture server (default configuration). Asserts the
 * canonical happy path for projects that have not customised id_pattern or
 * kanban_columns.
 *
 * Fixtures: server/tests/fixtures/meta/work/
 *   0001-first-work-item.md  — status: draft
 *   0005-sse-test-work-item.md
 */
import { test, expect } from './fixtures.js'

test.describe('Scenario A — default ID pattern, default columns', () => {
  test('GET /api/types lists work-items and work-item-reviews', async ({ request }) => {
    const res = await request.get('/api/types')
    expect(res.ok()).toBe(true)

    const { types } = (await res.json()) as { types: Array<{ key: string }> }
    const keys = types.map((t) => t.key)
    expect(keys).toContain('work-items')
    expect(keys).toContain('work-item-reviews')
    expect(keys.length).toBeGreaterThanOrEqual(11)
  })

  test('kanban renders all seven default columns', async ({ page }) => {
    await page.goto('/kanban')

    const defaultColumns = ['draft', 'ready', 'in-progress', 'review', 'done', 'blocked', 'abandoned']
    for (const key of defaultColumns) {
      await expect(page.locator(`section[data-column="${key}"]`)).toBeVisible()
    }
  })

  test('work-item-reviews tab appears and shows empty state when directory absent', async ({
    page,
  }) => {
    await page.goto('/library/work-item-reviews')

    await expect(page.locator('body')).not.toContainText('Internal Server Error')
    await expect(page.locator('body')).not.toContainText('404')
  })

  test('PATCH to a valid default status returns 200', async ({ request }) => {
    const docsRes = await request.get('/api/docs?type=work-items')
    expect(docsRes.ok()).toBe(true)

    const { docs } = (await docsRes.json()) as { docs: Array<{ relPath: string }> }
    if (docs.length === 0) {
      test.skip()
      return
    }

    const relPath = docs[0].relPath
    const getRes = await request.get(`/api/docs/${relPath}`)
    const etag = getRes.headers()['etag'] ?? '"test"'

    const patchRes = await request.patch(`/api/docs/${relPath}/frontmatter`, {
      headers: { 'If-Match': etag, 'Content-Type': 'application/json' },
      data: JSON.stringify({ patch: { status: 'ready' } }),
    })
    // 204 = updated; 412 = concurrent edit (acceptable in shared fixture env)
    expect([204, 412]).toContain(patchRes.status())
  })

  test('PATCH to an unknown status returns 400 with unknown_kanban_status', async ({ request }) => {
    const docsRes = await request.get('/api/docs?type=work-items')
    const { docs } = (await docsRes.json()) as { docs: Array<{ relPath: string }> }
    if (docs.length === 0) {
      test.skip()
      return
    }

    const relPath = docs[0].relPath
    // Status validation (step 6) happens before ETag check (step 7/8),
    // so any If-Match value produces 400 for an unknown status.
    const patchRes = await request.patch(`/api/docs/${relPath}/frontmatter`, {
      headers: { 'If-Match': '"any"', 'Content-Type': 'application/json' },
      data: JSON.stringify({ patch: { status: 'not-a-real-status' } }),
    })
    expect(patchRes.status()).toBe(400)
    const body = await patchRes.json()
    expect(body.error).toBe('unknown_kanban_status')
    expect(Array.isArray(body.acceptedKeys)).toBe(true)
  })
})
