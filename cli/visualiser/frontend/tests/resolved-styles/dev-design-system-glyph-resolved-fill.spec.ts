import { expect, test } from "@playwright/test";
import { DOC_TYPE_KEYS } from "../../src/api/types";
import { EXPECTED_COLOR, hexToRgb, setTheme } from "../lib/expected-colours";

// Verifies AC #4's resolved-hex contract using a real Chromium engine: that
// `getComputedStyle(svg).color` substitutes each Glyph's colour var to the
// canonical hex. JSDOM does not reliably substitute `var()` in SVG inline
// styles, hence this is a Playwright spec rather than a Vitest one.
//
// Parametrised over all 13 doc types × {light, dark}; expected hex comes
// from the shared EXPECTED_COLOR table (templates → ac-fg-muted, the other
// 12 → ac-doc-<key>).
for (const theme of ["light", "dark"] as const) {
  test.describe(`glyph resolved fill — ${theme}`, () => {
    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        await page.goto("/dev#glyphs");
        if (theme === "dark") {
          await setTheme(page, "dark");
        }
        const colour = await page
          .locator(`[data-testid="glyph-cell-${docType}-24"] svg`)
          .evaluate((el) => getComputedStyle(el).color);
        expect(colour).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]));
      });
    }
  });
}
