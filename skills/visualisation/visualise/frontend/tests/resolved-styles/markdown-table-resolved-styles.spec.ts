import { expect, test } from "@playwright/test";
import { resolveToken, setTheme } from "../lib/expected-colours";

test.use({ viewport: { width: 1280, height: 720 } });

// Structural complements to the dev-design-system-markdown VR snapshot: pin the
// load-bearing M1/M3 facts a pixel diff can miss under its maxDiffPixelRatio
// budget — that the table wrapper actually carries `overflow: hidden` (the
// border-collapse rounding workaround + wide-table clip, the literal M1 AC), and
// that the `hr` resolves to the muted `--ac-stroke` token (M3).
const TABLE_WRAP = '[data-testid="ds-markdown"] [class*="tableWrap"]';
const MD_HR = '[data-testid="ds-markdown"] hr';

test("M1: table wrapper clips with overflow:hidden + a rounded, bordered frame", async ({
  page,
}) => {
  await page.goto("/dev#markdown");
  const wrap = page.locator(TABLE_WRAP).first();
  await wrap.waitFor();

  const s = await wrap.evaluate((n) => {
    const c = getComputedStyle(n);
    return {
      overflowX: c.overflowX,
      overflowY: c.overflowY,
      borderTopLeftRadius: c.borderTopLeftRadius,
      borderTopWidth: c.borderTopWidth,
    };
  });

  // The literal M1 AC: the wrapper clips (overflow hidden — so a wide table is
  // clipped, never horizontally scrolled) and frames the table with a rounded
  // 1px border (the border-collapse rounding workaround). Asserting overflowX
  // directly catches a dropped overflow:hidden regression regardless of how wide
  // the fixture table happens to render; the visual clip itself is covered by
  // the dev-design-system-markdown VR snapshot.
  expect(s.overflowX).toBe("hidden");
  expect(s.overflowY).toBe("hidden");
  expect(s.borderTopLeftRadius).not.toBe("0px");
  expect(s.borderTopWidth).toBe("1px");
});

for (const theme of ["light", "dark"] as const) {
  test(`M3: horizontal rule resolves to --ac-stroke (${theme})`, async ({
    page,
  }) => {
    await page.goto("/dev#markdown");
    if (theme === "dark") await setTheme(page, "dark");

    const hr = page.locator(MD_HR).first();
    await hr.waitFor();
    const s = await hr.evaluate((n) => {
      const c = getComputedStyle(n);
      return { backgroundColor: c.backgroundColor, height: c.height };
    });

    expect(s.height).toBe("1px");
    expect(s.backgroundColor).toBe(await resolveToken(page, "--ac-stroke"));
  });
}
