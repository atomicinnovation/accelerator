import { expect, test } from "./fixtures.js";

const RESEARCH_SETS = [
  "ac2-coverage",
  "mcp-transport-layer",
  "model-context-protocol",
  "prompt-caching-economics",
];

test("the topic-research listing links each set to its own id", async ({
  page,
}) => {
  await page.goto("/library/topic-research");
  const listing = page.locator('[role="table"]');
  await expect(listing).toBeVisible();

  for (const set of RESEARCH_SETS) {
    await expect(
      listing.locator(`a[href="/library/topic-research/${set}"]`),
    ).toHaveCount(1);
  }

  await listing
    .locator('a[href="/library/topic-research/mcp-transport-layer"]')
    .click();
  await expect(page).toHaveURL(
    /\/library\/topic-research\/mcp-transport-layer$/,
  );
  await expect(
    page
      .getByRole("heading", { level: 1, name: "MCP Transport Layer" })
      .first(),
  ).toBeVisible();
});

test("a design-inventory search result links to the inventory's id", async ({
  page,
}) => {
  await page.goto("/library");
  await page.getByRole("searchbox", { name: "Search" }).fill("example");

  const results = page.getByRole("region", { name: "Search results" });
  await expect(
    results.getByRole("option").filter({ hasText: "Example Design Inventory" }),
  ).toHaveAttribute("href", "/library/design-inventories/2026-05-26-example");
});

test("plan reviews sharing a slug each link to their own filename stem", async ({
  page,
}) => {
  await page.goto("/library/plan-reviews");
  const listing = page.locator('[role="table"]');
  await expect(listing).toBeVisible();

  for (const n of [1, 2]) {
    await expect(
      listing.locator(
        `a[href="/library/plan-reviews/2026-01-01-first-plan-review-${n}"]`,
      ),
    ).toHaveCount(1);
  }

  await listing
    .locator('a[href="/library/plan-reviews/2026-01-01-first-plan-review-2"]')
    .click();
  await expect(page.locator("article")).toContainText(
    "Second review of the first plan.",
  );
});
