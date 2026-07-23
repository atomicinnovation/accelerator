import { expect, test } from "@playwright/test";
import { DOC_TYPE_KEYS, isPhysicalDocTypeKey } from "../../src/api/types";
import { EXPECTED_COLOR, hexToRgb, setTheme } from "../lib/expected-colours";

// 12 non-virtual doc types. AC #2 templates row is descoped — the backend
// doesn't emit a templates row in any RelatedArtifacts response (templates
// have no on-disk fixture). Reuse the production predicate rather than an
// inline VIRTUAL_DOC_TYPE_KEYS.includes filter — after Phase 1 narrowed
// VIRTUAL_DOC_TYPE_KEYS to readonly ['templates'], the inline form rejects a
// DocTypeKey-typed argument at typecheck.
const PHYSICAL_DOC_TYPE_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey);

// Navigate to any one of the 12 sibling 'ac2-coverage' fixtures — the other
// 11 surface as inferred rows in the single Related artifacts list.
const ANCHOR_URL = "/library/work-items/0099-ac2-coverage";

const THEMES = ["light", "dark"] as const;

// Up-front coverage guard. If a sibling fixture renames or breaks its shared
// slug, this fails with a clear count signal. Scope to icons within *inferred*
// rows via the stable `data-kind` hook — Option B merges declared and inferred
// into one <ul>, so an unscoped count would over-count any declared-row icons
// the anchor fixture carries. `data-kind` is a stable attribute (the module
// class `.tagInferred` is hashed and unselectable in the built bundle).
test("inferred rows surface one row per other non-virtual doc type", async ({
  page,
}) => {
  await page.goto(ANCHOR_URL);
  const rowIcons = page.locator(
    '[data-testid="related-row"][data-kind="inferred"] svg[data-doc-type]',
  );
  // -1 because the anchor doc itself is not listed as its own cluster
  // sibling. The remaining 11 cover the other non-virtual types.
  await expect(rowIcons).toHaveCount(PHYSICAL_DOC_TYPE_KEYS.length - 1, {
    timeout: 5000,
  });
});

for (const theme of THEMES) {
  test.describe(`related-artifacts row icon colour — ${theme}`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(ANCHOR_URL);
      await setTheme(page, theme);
    });

    // The 11 expected sibling types (all non-virtual except 'work-items',
    // which is the anchor doc's own type and so not in its own cluster).
    for (const target of PHYSICAL_DOC_TYPE_KEYS.filter(
      (k) => k !== "work-items",
    )) {
      test(`row icon for ${target}`, async ({ page }) => {
        const icon = page.locator(
          `[data-testid="related-row"][data-kind="inferred"] svg[data-doc-type="${target}"]`,
        );
        await expect(icon).toBeVisible();
        const color = await icon.evaluate((el) => getComputedStyle(el).color);
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[target][theme]));

        // Aria-hidden contract: row icons are decorative; the adjacent
        // anchor text carries the row's accessible name.
        expect(await icon.getAttribute("aria-hidden")).toBe("true");
        expect(await icon.getAttribute("role")).toBeNull();
      });
    }

    // Cover work-items by navigating to a sibling anchor of a different
    // doc type, so the work-items ac2-coverage fixture appears as one of
    // its cluster siblings.
    test("row icon for work-items (via sibling anchor)", async ({ page }) => {
      await page.goto("/library/decisions/ADR-0099-ac2-coverage");
      await setTheme(page, theme);
      const icon = page.locator(
        '[data-testid="related-row"][data-kind="inferred"] svg[data-doc-type="work-items"]',
      );
      await expect(icon).toBeVisible();
      const color = await icon.evaluate((el) => getComputedStyle(el).color);
      expect(color).toBe(hexToRgb(EXPECTED_COLOR["work-items"][theme]));
    });
  });
}

// Row-container layout invariance: AC #2 requires that adding the icon does
// not change the row container's background/border styling.
test.describe("related-artifacts container invariance", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(ANCHOR_URL);
    await setTheme(page, "light");
  });

  test("row hover-card reads as chrome-free at rest", async ({ page }) => {
    // The row is now an `.ac-related__item`-style hover-card: at rest it
    // carries a 1px *transparent* border (so hover can fill the colour in
    // without a layout shift) and no background. Assert both read as
    // chrome-free — transparent border colour + transparent background —
    // rather than a literal 0px border.
    const row = page.locator('[data-testid="related-row"] a').first();
    const bg = await row.evaluate((el) => getComputedStyle(el).backgroundColor);
    expect(bg).toBe("rgba(0, 0, 0, 0)");
    const borderColor = await row.evaluate(
      (el) => getComputedStyle(el).borderTopColor,
    );
    expect(borderColor).toBe("rgba(0, 0, 0, 0)");
  });

  test("row uses align-items: center for icon/text/chevron alignment", async ({
    page,
  }) => {
    const row = page.locator('[data-testid="related-row"] a').first();
    expect(await row.evaluate((el) => getComputedStyle(el).alignItems)).toBe(
      "center",
    );
  });
});
