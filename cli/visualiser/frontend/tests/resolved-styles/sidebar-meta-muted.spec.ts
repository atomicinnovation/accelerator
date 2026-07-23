import { expect, test } from "@playwright/test";
import { resolveToken, setTheme } from "../lib/expected-colours";

test.use({ viewport: { width: 1280, height: 720 } });

// L4: the sidebar META block is the quietest nav treatment — block opacity 0.7,
// META heading a further 0.75 (≈0.525 net effective), Templates link reduced to
// 12.5px and recoloured to the faintest fg. Pin the exact values a VR pixel diff
// can't assert precisely.
const META_SECTION = '[class*="metaSection"]';
const META_HEADING = '[class*="metaHeading"]';
const META_LINK = '[class*="metaLink"]';

for (const theme of ["light", "dark"] as const) {
  test(`L4: META block carries the compounded dampening + reduced link (${theme})`, async ({
    page,
  }) => {
    await page.goto("/lifecycle");
    if (theme === "dark") await setTheme(page, "dark");

    await page.locator(META_SECTION).waitFor();
    expect(
      await page
        .locator(META_SECTION)
        .evaluate((n) => getComputedStyle(n).opacity),
    ).toBe("0.7");
    expect(
      await page
        .locator(META_HEADING)
        .evaluate((n) => getComputedStyle(n).opacity),
    ).toBe("0.75");

    const link = await page
      .locator(META_LINK)
      .first()
      .evaluate((n) => {
        const c = getComputedStyle(n);
        return { fontSize: c.fontSize, color: c.color };
      });
    expect(link.fontSize).toBe("12.5px");
    expect(link.color).toBe(await resolveToken(page, "--ac-fg-faint"));
  });
}
