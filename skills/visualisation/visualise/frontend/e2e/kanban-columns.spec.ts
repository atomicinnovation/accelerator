/**
 * Verifies that the kanban board renders exactly the column set returned by
 * GET /api/kanban/config. The server is started with its default seven-column
 * config; this spec mocks the config endpoint so the frontend sees the four
 * custom columns and we can assert the rendering without restarting the server.
 */
import { test, expect } from './fixtures.js'

const CUSTOM_COLUMNS = [
  { key: 'ready', label: 'Ready' },
  { key: 'in-progress', label: 'In progress' },
  { key: 'review', label: 'Review' },
  { key: 'done', label: 'Done' },
]

test('renders the four configured columns when kanban_columns is set', async ({ page }) => {
  await page.route('**/api/kanban/config', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({ columns: CUSTOM_COLUMNS }),
    }),
  )

  await page.goto('/kanban')

  // Each configured column renders as a section regardless of card count.
  for (const { key } of CUSTOM_COLUMNS) {
    await expect(page.locator(`section[data-column="${key}"]`)).toBeVisible()
  }

  // Columns that belong to the seven-default set but not to the configured
  // four must be absent — confirms the frontend uses the API response, not
  // a hardcoded fallback.
  for (const key of ['draft', 'blocked', 'abandoned']) {
    await expect(page.locator(`section[data-column="${key}"]`)).not.toBeVisible()
  }
})
