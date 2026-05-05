/**
 * End-to-end tests for work-item cross-reference display in the library view.
 *
 * Fixtures: server/tests/fixtures/meta/work/
 *   0007-parent-epic.md          — no parent
 *   0008-child-story-1.md        — parent: "0007"
 *   0009-child-story-2.md        — parent: "0007"
 *   0010-child-story-3.md        — parent: "0007"
 *
 * The reverse index built by the server connects children → parent so that:
 *   - The parent's library view shows all three children under "Referenced by"
 *     (declared_inbound via work_item_refs_by_id)
 *   - Each child's library view shows the parent under "Targets"
 *     (declared_outbound via work_item_by_id lookup)
 */
import { test, expect } from './fixtures.js'

// Server-side slug strips the numeric prefix; fileSlugFromRelPath keeps it.
const PARENT_SLUG_URL = '/library/work-items/parent-epic'
const CHILD_SLUG_URL = '/library/work-items/child-story-1'

// RelatedArtifacts links are built with fileSlugFromRelPath (full filename
// minus .md), so the parent link from a child's page uses the filename form.
const PARENT_FILE_SLUG = '0007-parent-epic'

test.describe('work-item parent/child cross-references in library view', () => {
  test('parent epic shows all three children in Referenced by', async ({ page }) => {
    await page.goto(PARENT_SLUG_URL)

    // Wait for the related-artifacts panel to load and display declared inbound refs.
    const relatedSection = page.locator('section', {
      has: page.locator('h3', { hasText: 'Related artifacts' }),
    })
    const referencedByHeading = relatedSection.getByRole('heading', {
      level: 4,
      name: 'Referenced by',
    })
    await expect(referencedByHeading).toBeVisible({ timeout: 10_000 })

    // All three children must appear as navigable links.
    await expect(relatedSection.getByRole('link', { name: 'Child Story 1' })).toBeVisible()
    await expect(relatedSection.getByRole('link', { name: 'Child Story 2' })).toBeVisible()
    await expect(relatedSection.getByRole('link', { name: 'Child Story 3' })).toBeVisible()
  })

  test('child story shows parent in Targets', async ({ page }) => {
    await page.goto(CHILD_SLUG_URL)

    const relatedSection = page.locator('section', {
      has: page.locator('h3', { hasText: 'Related artifacts' }),
    })
    const targetsHeading = relatedSection.getByRole('heading', {
      level: 4,
      name: 'Targets',
    })
    await expect(targetsHeading).toBeVisible({ timeout: 10_000 })

    // The parent link is present and points to the correct library URL.
    const parentLink = relatedSection.getByRole('link', { name: 'Parent Epic' })
    await expect(parentLink).toBeVisible()
    await expect(parentLink).toHaveAttribute(
      'href',
      `/library/work-items/${PARENT_FILE_SLUG}`,
    )
  })
})
