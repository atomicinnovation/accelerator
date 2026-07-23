import { expect, test } from "@playwright/test";
import { applyTheme } from "../lib/helpers";

// Clips the DevDesignSystem Markdown section (`#markdown`) in light + dark.
// Guards the M1 rounded-table treatment (wrapper border + radius, recessed
// uppercase header, top-border row separators, wide-table clip) and the M3
// muted horizontal rule, neither of which had VR coverage before. Modelled on
// dev-design-system-code-syntax.spec.ts.
const VIEWPORT = { width: 1440, height: 900 };

for (const theme of ["light", "dark"] as const) {
  test.describe(`dev-design-system-markdown (${theme})`, () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT);
      await page.goto("/dev#markdown");
      await applyTheme(page, theme);
      // Wait for the rendered table + hr, then for fonts so measured text
      // and the Sora header are stable before clipping.
      await page.locator('[data-testid="ds-markdown"] table').waitFor();
      await page.locator('[data-testid="ds-markdown"] hr').waitFor();
      await page.evaluate(() => document.fonts.ready.then(() => undefined));
    });

    test("markdown section", async ({ page }) => {
      const section = page.locator('[data-testid="ds-markdown"]');
      await expect(section).toHaveScreenshot(`markdown-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: "disabled",
      });
    });
  });
}
