import { test, expect } from './fixtures.js'

test('library → lifecycle → library deep-link round trip', async ({ page }) => {
  // Start at the plans list
  await page.goto('/library/plans')
  await expect(page.locator('table')).toBeVisible()

  // Click into first-plan
  await page.locator('a').filter({ hasText: 'First Plan' }).first().click()
  await expect(page).toHaveURL(/\/library\/plans\/first-plan/)

  // Navigate to lifecycle via the sidebar
  await page.locator('nav a[href="/lifecycle"]').click()
  await expect(page).toHaveURL(/\/lifecycle/)

  // Find the first-plan cluster card and click it
  await page.locator('a[href="/lifecycle/first-plan"]').click()
  await expect(page).toHaveURL(/\/lifecycle\/first-plan/)

  // The cluster view should show the plans stage as present
  await expect(
    page.locator('li[data-stage="hasPlan"][data-present="true"]'),
  ).toBeVisible()

  // Click the plan entry link to return to the library
  await page
    .locator('li[data-stage="hasPlan"] a[href*="/library/plans/"]')
    .first()
    .click()
  await expect(page).toHaveURL(/\/library\/plans\//)
})
