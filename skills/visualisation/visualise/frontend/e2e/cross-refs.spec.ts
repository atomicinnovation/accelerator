/**
 * End-to-end tests for work-item cross-reference display in the library view.
 *
 * Fixtures: server/tests/fixtures/meta/work/
 *   0007-parent-epic.md          — no parent
 *   0008-child-story-1.md        — parent: "0007"
 *   0009-child-story-2.md        — parent: "0007"
 *   0010-child-story-3.md        — parent: "0007"
 *
 * The reverse index built by the server connects children → parent so that
 * (under the Option B single-list aside):
 *   - The parent's library view shows all three children as `(declared)` rows
 *     (declared_inbound via work_item_refs_by_id)
 *   - Each child's library view shows the parent as a `(declared)` row
 *     (declared_outbound via work_item_by_id lookup)
 */
import { expect, test } from "./fixtures.js";

// Server-side slug strips the numeric prefix; fileSlugFromRelPath keeps it.
const PARENT_SLUG_URL = "/library/work-items/parent-epic";
const CHILD_SLUG_URL = "/library/work-items/child-story-1";

// RelatedArtifacts links are built with fileSlugFromRelPath (full filename
// minus .md), so the parent link from a child's page uses the filename form.
const PARENT_FILE_SLUG = "0007-parent-epic";

test.describe("work-item parent/child cross-references in library view", () => {
  test("parent epic shows all three children as declared rows", async ({
    page,
  }) => {
    await page.goto(PARENT_SLUG_URL);

    // Wait for the related-artifacts panel to load and display declared inbound refs.
    const relatedSection = page.locator("section", {
      has: page.locator("h3", { hasText: "Related artifacts" }),
    });
    // Option B: a single list with no sub-group headings.
    await expect(relatedSection.getByTestId("related-list")).toBeVisible({
      timeout: 10_000,
    });
    await expect(relatedSection.getByRole("heading", { level: 4 })).toHaveCount(
      0,
    );

    // All three children appear as navigable links, each in a (declared) row.
    for (const name of ["Child Story 1", "Child Story 2", "Child Story 3"]) {
      const row = relatedSection
        .locator('[data-testid="related-row"][data-kind="declared"]')
        .filter({ has: page.getByRole("link", { name }) });
      await expect(row).toHaveCount(1);
      await expect(row.getByText("(declared)")).toBeVisible();
    }
  });

  test("child story shows parent as a declared row", async ({ page }) => {
    await page.goto(CHILD_SLUG_URL);

    const relatedSection = page.locator("section", {
      has: page.locator("h3", { hasText: "Related artifacts" }),
    });
    await expect(relatedSection.getByTestId("related-list")).toBeVisible({
      timeout: 10_000,
    });

    // The parent appears as a (declared) row linking to the correct library URL.
    const parentRow = relatedSection
      .locator('[data-testid="related-row"][data-kind="declared"]')
      .filter({ has: page.getByRole("link", { name: "Parent Epic" }) });
    await expect(parentRow).toHaveCount(1);
    await expect(parentRow.getByText("(declared)")).toBeVisible();
    await expect(
      parentRow.getByRole("link", { name: "Parent Epic" }),
    ).toHaveAttribute("href", `/library/work-items/${PARENT_FILE_SLUG}`);
  });
});
