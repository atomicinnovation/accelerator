import { expect, test } from "@playwright/test";
import { resolveToken, setTheme } from "../lib/expected-colours";

test.use({ viewport: { width: 1280, height: 720 } });

// `p > code` (not `:not(pre) > code`) so the prose locator can never resolve
// to a table-cell `<code>` regardless of fixture append order.
const PROSE_CODE = '[class*="markdown"] p > code';

for (const theme of ["light", "dark"] as const) {
  test(`inline code is a monospace pill (${theme})`, async ({ page }) => {
    await page.goto("/library/plans/first-plan");
    if (theme === "dark") await setTheme(page, "dark");

    const el = page.locator(PROSE_CODE).first();
    const s = await el.evaluate((n) => {
      const c = getComputedStyle(n);
      return {
        fontFamily: c.fontFamily,
        fontSize: c.fontSize,
        backgroundColor: c.backgroundColor,
        borderTopWidth: c.borderTopWidth,
        borderTopStyle: c.borderTopStyle,
        borderTopColor: c.borderTopColor,
        borderTopLeftRadius: c.borderTopLeftRadius,
        paddingTop: c.paddingTop,
        paddingLeft: c.paddingLeft,
      };
    });

    // Theme-invariant chrome (AC2 dimensions + AC3 size + AC1 face).
    expect(s.fontFamily).toContain("Fira Code");
    expect(s.fontSize).toBe("11.5px");
    expect(s.borderTopWidth).toBe("1px");
    expect(s.borderTopStyle).toBe("solid");
    expect(s.borderTopLeftRadius).toBe("3px");
    expect(s.paddingTop).toBe("1px");
    expect(s.paddingLeft).toBe("5px");

    // Theme-varying colours (AC2 colour + AC5): resolve the tokens through the
    // cascade and compare exactly. Light bg #f4f6fa / dark #070b12; light
    // stroke rgba(32,34,49,0.06) / dark rgba(255,255,255,0.04).
    expect(s.backgroundColor).toBe(await resolveToken(page, "--ac-bg-sunken"));
    expect(s.borderTopColor).toBe(await resolveToken(page, "--ac-stroke-soft"));

    // AC1 contrast: the prose body must NOT be the mono face. Guard the
    // precondition that the default font mode is active — [data-font="mono"]
    // repoints --ac-font-body to mono and would collapse the contrast.
    const fontMode = await page.evaluate(
      () => document.documentElement.dataset.font ?? "default",
    );
    expect(fontMode).not.toBe("mono");
    const proseFont = await page
      .locator('[class*="markdown"] p')
      .first()
      .evaluate((n) => getComputedStyle(n).fontFamily);
    expect(proseFont).not.toContain("Fira Code");
  });
}

const sizeOf = (page: import("@playwright/test").Page, sel: string) =>
  page
    .locator(sel)
    .first()
    .evaluate((n) => getComputedStyle(n).fontSize);

test("table-body inline code is 11px, header + prose stay at the 11.5px base", async ({
  page,
}) => {
  await page.goto("/library/plans/first-plan");
  // `th code` requires the GFM delimiter row (|---|) so the first table row
  // renders as <thead><th>; without it the header collapses to <td>.
  const [td, th, prose] = await Promise.all([
    sizeOf(page, '[class*="markdown"] td code'),
    sizeOf(page, '[class*="markdown"] th code'),
    sizeOf(page, '[class*="markdown"] p > code'),
  ]);
  expect(td).toBe("11px"); // fails first if the (0,1,4) specificity is wrong
  // Assert the *intent* (header code == prose code, strictly above body code),
  // not just the bare 11.5px literal, so a future base-size change can't
  // silently void the td-only scoping.
  expect(th).toBe("11.5px");
  expect(th).toBe(prose);
});

test("fenced code is not pill-styled", async ({ page }) => {
  await page.goto("/library/plans/first-plan");
  const s = await page
    .locator('[class*="markdown"] pre code')
    .first()
    .evaluate((n) => {
      const c = getComputedStyle(n);
      return {
        fontSize: c.fontSize,
        borderTopWidth: c.borderTopWidth,
        backgroundColor: c.backgroundColor,
      };
    });
  expect(s.fontSize).toBe("14px"); // inherits .markdown pre var(--size-xs), not 11.5px
  expect(s.borderTopWidth).toBe("0px"); // no inline pill border leaked in
  // Anchor 'unchanged' on a second, independent property: the inner <code> has
  // no pill background of its own (the .markdown pre wrapper owns the surface),
  // so the sunken pill background must NOT have leaked onto it.
  expect(s.backgroundColor).toBe("rgba(0, 0, 0, 0)");
});

// AC5 divergence: the pill colours must actually change between themes, not
// merely resolve to *a* token value — otherwise a theme-invariant token would
// pass both per-theme branches above trivially.
test("inline-code pill colours diverge between light and dark", async ({
  page,
}) => {
  await page.goto("/library/plans/first-plan");
  const lightBg = await resolveToken(page, "--ac-bg-sunken");
  const lightBorder = await resolveToken(page, "--ac-stroke-soft");
  await setTheme(page, "dark");
  expect(await resolveToken(page, "--ac-bg-sunken")).not.toBe(lightBg);
  expect(await resolveToken(page, "--ac-stroke-soft")).not.toBe(lightBorder);
});
