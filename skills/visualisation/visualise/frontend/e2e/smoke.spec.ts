import { test, expect } from './fixtures.js'

test('app loads and redirects to /library', async ({ page }) => {
  await page.goto('/')
  await expect(page).toHaveURL(/\/library/)
  await expect(page.getByRole('navigation', { name: /site navigation/i })).toBeVisible()
})
