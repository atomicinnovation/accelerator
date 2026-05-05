/**
 * Scenario B validation: project-prefixed ID pattern + custom column set.
 *
 * Mocks /api/kanban/config (four custom columns) and /api/docs?type=work-items
 * (PROJ-prefixed work items) via page.route() so no separately-configured
 * server instance is needed.
 *
 * Assertions cover:
 *   - Only the four configured columns render (not the seven defaults)
 *   - PROJ-prefixed work-item cards render in the correct columns
 *   - PATCH with a status outside the configured values returns 400
 *   - Case sensitivity: label-cased status ("In Progress") is rejected
 */
import { test, expect } from './fixtures.js'

const CUSTOM_COLUMNS = [
  { key: 'ready', label: 'Ready' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'review', label: 'Review' },
  { key: 'done', label: 'Done' },
]

function makeWorkItem(
  relPath: string,
  slug: string,
  workItemId: string,
  title: string,
  status: string,
) {
  return {
    type: 'work-items',
    path: `/fixtures/${relPath}`,
    relPath,
    slug,
    workItemId,
    title,
    frontmatter: { status },
    frontmatterState: 'parsed',
    workItemRefs: [],
    mtimeMs: 0,
    size: 100,
    etag: 'e2e-mock',
    bodyPreview: '',
  }
}

const PROJ_WORK_ITEMS = [
  makeWorkItem('meta/work/PROJ-0001-first-story.md', 'first-story', 'PROJ-0001', 'First Story', 'ready'),
  makeWorkItem('meta/work/PROJ-0002-second-story.md', 'second-story', 'PROJ-0002', 'Second Story', 'in-progress'),
  makeWorkItem('meta/work/PROJ-0007-seventh-story.md', 'seventh-story', 'PROJ-0007', 'Seventh Story', 'done'),
]

const mockKanbanConfig = (page: import('@playwright/test').Page) =>
  page.route('**/api/kanban/config', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({ columns: CUSTOM_COLUMNS }),
    }),
  )

// /api/docs?type=work-items — glob '?' is a single-char wildcard so we use a
// URL predicate instead of a string or RegExp to match query parameters exactly.
const mockWorkItems = (page: import('@playwright/test').Page) =>
  page.route(
    (url) => url.pathname.endsWith('/api/docs') && url.searchParams.get('type') === 'work-items',
    (route) =>
      route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify({ docs: PROJ_WORK_ITEMS }),
      }),
  )

test.describe('Scenario B — project ID pattern + custom columns', () => {
  test('renders only the four configured columns', async ({ page }) => {
    // Only mock the column config; work-items come from the real server.
    await mockKanbanConfig(page)

    await page.goto('/kanban')

    for (const { key } of CUSTOM_COLUMNS) {
      await expect(page.locator(`section[data-column="${key}"]`)).toBeVisible()
    }
    for (const key of ['draft', 'blocked', 'abandoned']) {
      await expect(page.locator(`section[data-column="${key}"]`)).not.toBeVisible()
    }
  })

  test('PROJ-prefixed work-item cards render in the correct columns', async ({ page }) => {
    await mockKanbanConfig(page)
    await mockWorkItems(page)

    await page.goto('/kanban')

    await expect(page.locator('section[data-column="ready"]').getByText('First Story')).toBeVisible()
    await expect(page.locator('section[data-column="in-progress"]').getByText('Second Story')).toBeVisible()
    await expect(page.locator('section[data-column="done"]').getByText('Seventh Story')).toBeVisible()
  })

  test('PATCH with status outside the seven defaults returns 400', async ({ request }) => {
    // Uses the real server (request context bypasses page.route() mocks).
    const docsRes = await request.get('/api/docs?type=work-items')
    const { docs } = (await docsRes.json()) as { docs: Array<{ relPath: string }> }
    if (docs.length === 0) {
      test.skip()
      return
    }

    const relPath = docs[0].relPath
    // Status validation happens before ETag check, so any If-Match value returns 400.
    const patchRes = await request.patch(`/api/docs/${relPath}/frontmatter`, {
      headers: { 'If-Match': '"any"', 'Content-Type': 'application/json' },
      data: JSON.stringify({ patch: { status: 'not-configured' } }),
    })
    expect(patchRes.status()).toBe(400)
    const body = await patchRes.json()
    expect(body.error).toBe('unknown_kanban_status')
    expect(Array.isArray(body.acceptedKeys)).toBe(true)
  })

  test('PATCH with label-cased status returns 400 (case sensitive)', async ({ request }) => {
    const docsRes = await request.get('/api/docs?type=work-items')
    const { docs } = (await docsRes.json()) as { docs: Array<{ relPath: string }> }
    if (docs.length === 0) {
      test.skip()
      return
    }

    const relPath = docs[0].relPath
    const patchRes = await request.patch(`/api/docs/${relPath}/frontmatter`, {
      headers: { 'If-Match': '"any"', 'Content-Type': 'application/json' },
      data: JSON.stringify({ patch: { status: 'In Progress' } }),
    })
    expect(patchRes.status()).toBe(400)
  })
})
