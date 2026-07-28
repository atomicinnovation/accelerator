import { expect, test } from "./fixtures.js";

test("library → RCA listing → RCA detail with status colour", async ({
  page,
}) => {
  // The Operate category's RCA listing reuses the shared list-view layout.
  await page.goto("/library/root-cause-analyses");
  await expect(page.locator('[role="table"]')).toBeVisible();

  // AC #3: the resolved RCA row colours its status cell green
  // (resolved→green), not neutral grey. The whole row is a single Link, so
  // the status chip lives inside the row anchor.
  const row = page.locator("a").filter({ hasText: "Example RCA" }).first();
  await expect(row.locator('[data-variant="green"]')).toBeVisible();

  // Click into the RCA detail page (dated stem → date-stripped slug).
  await row.click();
  await expect(page).toHaveURL(/\/library\/root-cause-analyses\/example-rca/);
  await expect(page.locator("article")).toBeVisible();
  // AC #5's related-artifacts routing is covered by the server `api_related`
  // suite and the `RelatedArtifacts.test.tsx` unit case rather than here: an
  // E2E inbound-link fixture would have to carry a cross-ref, which would pull
  // an RCA into the balanced `ac2-coverage` cluster the aside-row VR spec
  // counts on.
});

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
