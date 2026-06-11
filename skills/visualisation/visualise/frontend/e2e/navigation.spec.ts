import { expect, test } from "./fixtures.js";

test("library → lifecycle → library deep-link round trip", async ({ page }) => {
  // Start at the plans list. The list view used to be a `<table>` but is
  // now a CSS grid with `role="table"` (the entire row is a single Link),
  // so we wait on the ARIA role rather than the element name.
  await page.goto("/library/plans");
  await expect(page.locator('[role="table"]')).toBeVisible();

  // Click into first-plan
  await page.locator("a").filter({ hasText: "First Plan" }).first().click();
  await expect(page).toHaveURL(/\/library\/plans\/first-plan/);

  // Navigate to lifecycle via the sidebar
  await page.locator('nav a[href="/lifecycle"]').click();
  await expect(page).toHaveURL(/\/lifecycle/);

  // Find the first-plan cluster card and click it
  await page.locator('a[href="/lifecycle/first-plan"]').click();
  await expect(page).toHaveURL(/\/lifecycle\/first-plan/);

  // The cluster view's timeline carries one <li data-stage="<doc-type>">
  // per present (or missing) stage. Scope to the timeline (top-level <ol>)
  // to avoid matching the pipeline-panel tiles, which also carry
  // data-stage attributes but live inside the panel's nested markup.
  await expect(
    page.locator('ol > li[data-stage="plans"]').first(),
  ).toBeVisible();

  // Click the plan entry link to return to the library
  await page
    .locator('ol > li[data-stage="plans"] a[href*="/library/plans/"]')
    .first()
    .click();
  await expect(page).toHaveURL(/\/library\/plans\//);
});
