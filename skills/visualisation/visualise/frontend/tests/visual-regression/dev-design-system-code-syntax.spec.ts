import { expect, test } from "@playwright/test";
import { applyTheme } from "../lib/helpers";

// Migrated from the retired /code-syntax-showcase full-page screenshot to a
// per-language clipped locator on the DevDesignSystem Code-blocks section
// (`#code`). Clipping each `code-syntax-cell-<lang>` keeps a per-language
// regression from hiding under a viewport-wide diff budget and drops the
// dependency on the old 1440×900 full-page frame. `bash` is net-new coverage.
const LANGS = [
  "python",
  "typescript",
  "yaml",
  "json",
  "css",
  "html",
  "diff",
  "markdown",
  "bash",
] as const;

const VIEWPORT = { width: 1440, height: 900 };

for (const theme of ["light", "dark"] as const) {
  test.describe(`dev-design-system-code-syntax (${theme})`, () => {
    test.describe.configure({ mode: "parallel" });

    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT);
      await page.goto("/dev#code");
      await applyTheme(page, theme);
      // Wait for highlight.js to have tokenised at least one cell before
      // clipping, then for fonts so wrapped/measured text is stable.
      await page.locator(".hljs-keyword").first().waitFor();
      await page.evaluate(() => document.fonts.ready.then(() => undefined));
    });

    for (const lang of LANGS) {
      test(`${lang}`, async ({ page }) => {
        const cell = page.locator(`[data-testid="code-syntax-cell-${lang}"]`);
        await expect(cell).toHaveScreenshot(`${lang}-${theme}.png`, {
          maxDiffPixelRatio: 0.05,
          animations: "disabled",
        });
      });
    }
  });
}
