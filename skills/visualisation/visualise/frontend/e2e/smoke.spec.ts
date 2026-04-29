import { test, expect } from './fixtures.js'

test('app loads and redirects to /library', async ({ page }) => {
  await page.goto('/')
  await expect(page).toHaveURL(/\/library/)
  await expect(page.locator('nav')).toBeVisible()
})
