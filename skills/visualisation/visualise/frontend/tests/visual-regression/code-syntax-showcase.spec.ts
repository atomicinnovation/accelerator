import { expect, test } from "@playwright/test";
import { applyTheme } from "./helpers";

const ROUTES = [["code-syntax-showcase", "/code-syntax-showcase"]] as const;

const VIEWPORT = { width: 1440, height: 900 };

for (const [id, path] of ROUTES) {
  for (const theme of ["light", "dark"] as const) {
    test(`${id} (${theme})`, async ({ page }) => {
      await page.setViewportSize(VIEWPORT);
      await page.goto(path);
      await applyTheme(page, theme);
      await page.locator(".hljs-keyword").first().waitFor();
      await page.evaluate(() => document.fonts.ready.then(() => undefined));
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: "disabled",
      });
    });
  }
}
