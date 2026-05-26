import { test, expect } from '@playwright/test'
import { DOC_TYPE_KEYS } from '../../src/api/types'
import {
  DETAIL_ROUTE_SLUGS,
  DETAIL_ROUTE_RENDERS_ARTICLE,
} from './lib/detail-route-slugs'

for (const docType of DOC_TYPE_KEYS) {
  test(`listing route renders for ${docType}`, async ({ page }) => {
    await page.goto(`/library/${docType}`)
    // Listing routes render a Page with eyebrow + heading; <article>
    // is only on detail pages. Use a heading-level-1 probe to assert
    // the page rendered something meaningful (not just the shell).
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  })

  test(`detail route renders for ${docType}`, async ({ page }) => {
    const slug = DETAIL_ROUTE_SLUGS[docType]
    await page.goto(`/library/${docType}/${slug}`)
    if (DETAIL_ROUTE_RENDERS_ARTICLE[docType]) {
      await expect(page.locator('article')).toBeVisible()
    } else {
      // templates detail (LibraryTemplatesView) renders a tiers/preview
      // layout rather than an <article>.
      await expect(page.getByTestId('templates-detail-layout')).toBeVisible()
    }
  })
}
