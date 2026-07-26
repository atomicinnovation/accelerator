import { expect, test } from "@playwright/test";
import { applyTheme, relativeTimeMask } from "../lib/helpers";

const ROUTES = [
  ["library-doc-view", "/library/plans/first-plan"],
  // RCA detail + listing (Operate category, work item 0110). The RCA empty
  // state / BigGlyph hero is covered by big-glyph-showcase.spec.ts; the
  // fixture dir is non-empty so it can't render here.
  ["library-doc-view-rca", "/library/root-cause-analyses/example-rca"],
  ["library-list-view-rca", "/library/root-cause-analyses"],
] as const;

const VIEWPORT = { width: 1440, height: 900 };

for (const [id, path] of ROUTES) {
  for (const theme of ["light", "dark"] as const) {
    test(`${id} (${theme})`, async ({ page }) => {
      await page.setViewportSize(VIEWPORT);
      await page.goto(path);
      await applyTheme(page, theme);
      // Both body and aside render <p>Loading…</p> placeholders;
      // wait for every Loading text node to detach before capture.
      await expect(page.getByText("Loading…")).toHaveCount(0);
      await page.evaluate(() => document.fonts.ready.then(() => undefined));
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: "disabled",
        mask: [relativeTimeMask(page)],
      });
    });
  }
}
