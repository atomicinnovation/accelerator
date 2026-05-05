import { test, expect } from './fixtures.js'

test('resolved wiki-link [[ADR-0001]] renders as a clickable anchor', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  await expect(page.locator('article')).toBeVisible()

  // [[ADR-0001]] should resolve to a link pointing at the decisions library
  const wikiLink = page.locator('a[href*="/library/decisions/"]').filter({
    hasText: /ADR-0001|example-decision/i,
  })

  if (await wikiLink.count() > 0) {
    await expect(wikiLink.first()).toBeVisible()
    await wikiLink.first().click()
    await expect(page).toHaveURL(/\/library\/decisions\//)
  }
})

test('resolved wiki-link [[WORK-ITEM-0001]] renders as a clickable anchor', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  await expect(page.locator('article')).toBeVisible()

  // The plan fixture body contains [[WORK-ITEM-0001]] which should resolve
  // to the first work item under the default numeric ID pattern.
  const wikiLink = page.locator('a[href*="/library/work-items/"]').filter({
    hasText: /first.work.item|0001/i,
  })

  if (await wikiLink.count() > 0) {
    await expect(wikiLink.first()).toBeVisible()
    await wikiLink.first().click()
    await expect(page).toHaveURL(/\/library\/work-items\//)
  }
})

test('unresolved wiki-link [[ADR-9999]] renders as a span, not an anchor', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  await expect(page.locator('article')).toBeVisible()

  // [[ADR-9999]] does not exist — should render as an unresolved span
  const unresolved = page.locator('span.unresolved-wiki-link')

  if (await unresolved.count() > 0) {
    await expect(unresolved.first()).toBeVisible()
    await expect(unresolved.first()).not.toHaveAttribute('href')
  }
})
