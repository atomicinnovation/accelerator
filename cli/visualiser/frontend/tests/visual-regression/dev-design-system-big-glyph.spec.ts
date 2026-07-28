import { expect, test } from "@playwright/test";
import { DOC_TYPE_KEYS } from "../../src/api/types";
import { DARK_COLOR_TOKENS } from "../../src/styles/tokens";
import { hexToRgb } from "../lib/expected-colours";

const VIEWPORT = { width: 1024, height: 768 };

for (const theme of ["light", "dark"] as const) {
  test.describe(`big-glyph-showcase (${theme})`, () => {
    test.describe.configure({ mode: "parallel" });

    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT);
      await page.goto("/dev#bigglyphs");
      if (theme === "dark") {
        // Confirm the dark cascade has COMMITTED by polling a cell's resolved
        // background-color until it equals the dark `--ac-bg-card`. Critical:
        // getComputedStyle().backgroundColor returns an `rgb(...)` string (here
        // `rgb(19, 21, 36)`), never the hex token `#131524` — so we compare
        // against the hex converted to rgb via `hexToRgb`, NOT the raw hex
        // (which would never match and hang to timeout). This deliberately does
        // NOT reuse the tautological glyph-showcase predicate (`colour !== ''`),
        // which confirms only that *a* colour resolved, never the *dark* one.
        const expectedRgb = hexToRgb(DARK_COLOR_TOKENS["ac-bg-card"]);
        await page.evaluate(() => {
          document.documentElement.dataset.theme = "dark";
        });
        await page.waitForFunction((expected) => {
          const cell = document.querySelector(
            '[data-testid="big-glyph-cell-decisions"]',
          );
          if (!cell) return false;
          const norm = (s: string) => s.replace(/\s+/g, "");
          return (
            norm(getComputedStyle(cell).backgroundColor) === norm(expected)
          );
        }, expectedRgb);
      }
    });

    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        // Per-cell clipped capture so a per-illustration regression can't hide
        // under a viewport-wide diff budget.
        const cell = page.locator(`[data-testid="big-glyph-cell-${docType}"]`);
        await expect(cell).toHaveScreenshot(`${docType}-${theme}.png`, {
          maxDiffPixelRatio: 0.05,
          animations: "disabled",
        });
      });
    }
  });
}
