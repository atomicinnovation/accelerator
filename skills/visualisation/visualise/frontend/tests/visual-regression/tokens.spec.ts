import { test, expect } from '@playwright/test'

const ROUTES = [
  ['kanban', '/kanban'],
  ['library', '/library'],
  ['library-type', '/library/plans'],
  ['library-decisions', '/library/decisions'],
  ['library-templates', '/library/templates'],
  ['lifecycle-cluster', '/lifecycle/first-plan'],
] as const

const VIEWPORT = { width: 1440, height: 900 }

for (const [id, path] of ROUTES) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${id} (${theme})`, async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto(path)
      if (theme === 'dark') {
        await page.evaluate(() => {
          document.documentElement.dataset.theme = 'dark'
        })
      }
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: 'disabled',
      })
    })
  }
}

test('lifecycle-cluster-after-click (light)', async ({ page }) => {
  await page.setViewportSize(VIEWPORT)
  await page.goto('/lifecycle/first-plan')
  const preClickUrl = page.url()
  await page.getByRole('link').first().click()
  // Wait for SPA navigation away from the cluster page (SSE keeps the
  // network busy so 'networkidle' never fires; URL change is reliable).
  await page.waitForURL(url => url.href !== preClickUrl, { timeout: 10000 })
  await expect(page).toHaveScreenshot('lifecycle-cluster-after-click-light.png', {
    maxDiffPixelRatio: 0.05,
    animations: 'disabled',
  })
})

test('lifecycle-cluster-after-click (dark)', async ({ page }) => {
  await page.setViewportSize(VIEWPORT)
  await page.goto('/lifecycle/first-plan')
  await page.evaluate(() => {
    document.documentElement.dataset.theme = 'dark'
  })
  const preClickUrl = page.url()
  await page.getByRole('link').first().click()
  await page.waitForURL(url => url.href !== preClickUrl, { timeout: 10000 })
  await expect(page).toHaveScreenshot('lifecycle-cluster-after-click-dark.png', {
    maxDiffPixelRatio: 0.05,
    animations: 'disabled',
  })
})

// Sanity check for the @media (prefers-color-scheme: dark) path: global.test.ts
// asserts the two dark blocks are byte-equivalent, but this exercises the OS-
// preference route visually so a selector-engine bug in the @media block
// can't escape both checks. Asserts the same baseline as library (dark).
test('library (prefers-color-scheme: dark, no data-theme attribute)', async ({ page }) => {
  await page.setViewportSize(VIEWPORT)
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/library')
  await expect(page).toHaveScreenshot('library-dark.png', {
    maxDiffPixelRatio: 0.05,
    animations: 'disabled',
  })
})
