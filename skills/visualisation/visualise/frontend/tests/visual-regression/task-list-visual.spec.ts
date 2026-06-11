import { expect, test } from "@playwright/test";
import { applyTheme } from "./helpers";

const LISTS = [
  ["tight", 0],
  ["loose", 1],
] as const;

for (const theme of ["light", "dark"] as const) {
  for (const [kind, nth] of LISTS) {
    test(`task-list ${kind} (${theme})`, async ({ page }) => {
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto("/library/plans/first-plan");
      await applyTheme(page, theme);
      await expect(page.getByText("Loading…")).toHaveCount(0);
      await page.evaluate(() => document.fonts.ready.then(() => undefined));
      const list = page.locator('[class*="tasklist"]').nth(nth);
      await expect(list).toHaveScreenshot(`task-list-${kind}-${theme}.png`, {
        maxDiffPixelRatio: 0.01,
        animations: "disabled",
      });
    });
  }
}
