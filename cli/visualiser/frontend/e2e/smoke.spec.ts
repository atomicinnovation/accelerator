import { expect, test } from "./fixtures.js";

test("app loads and redirects to /library", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/library/);
  await expect(
    page.getByRole("navigation", { name: /site navigation/i }),
  ).toBeVisible();
});

test("codebase research is served from its default path", async ({
  request,
}) => {
  const response = await request.get("/api/docs?type=codebase-research");
  expect(response.ok()).toBe(true);
  const { docs } = (await response.json()) as { docs: { relPath: string }[] };

  expect(docs.length).toBeGreaterThan(0);
  for (const { relPath } of docs) {
    expect(relPath).toMatch(/^meta\/research\/codebase\//);
  }
});
