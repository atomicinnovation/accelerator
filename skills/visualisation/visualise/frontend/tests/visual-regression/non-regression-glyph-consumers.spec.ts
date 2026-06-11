import { expect, test } from "@playwright/test";
import { DOC_TYPE_KEYS, isPhysicalDocTypeKey } from "../../src/api/types";
import { EXPECTED_COLOR, hexToRgb, setTheme } from "./lib/expected-colours";

const THEMES = ["light", "dark"] as const;
// Reuse the production predicate (avoids the narrowed-`includes` typecheck
// failure noted in the aside-row spec).
const NON_VIRTUAL_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey);

for (const theme of THEMES) {
  test.describe(`hub-card glyph colour — ${theme}`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/library");
      await setTheme(page, theme);
    });

    for (const docType of NON_VIRTUAL_KEYS) {
      test(`${docType}`, async ({ page }) => {
        // Target the SVG inside the framed HubCard Glyph. Framed Glyphs
        // put data-doc-type on both wrapper span and svg; svg[data-doc-type]
        // disambiguates.
        const glyph = page.locator(
          `[data-testid="hub-grid"] svg[data-doc-type="${docType}"]`,
        );
        await expect(glyph).toBeVisible();
        const color = await glyph.evaluate((el) => getComputedStyle(el).color);
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]));
      });
    }

    test("templates hub card has NO glyph", async ({ page }) => {
      // Templates is intentionally gated out of HubCard rendering by Phase 1.
      const glyph = page.locator(
        '[data-testid="hub-grid"] svg[data-doc-type="templates"]',
      );
      await expect(glyph).toHaveCount(0);
    });
  });

  test.describe(`listing-route eyebrow glyph colour — ${theme}`, () => {
    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        await page.goto(`/library/${docType}`);
        await setTheme(page, theme);
        const glyph = page.locator(
          `[data-slot="eyebrow"] svg[data-doc-type="${docType}"]`,
        );
        await expect(glyph).toBeVisible();
        const color = await glyph.evaluate((el) => getComputedStyle(el).color);
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]));
      });
    }
  });
}
