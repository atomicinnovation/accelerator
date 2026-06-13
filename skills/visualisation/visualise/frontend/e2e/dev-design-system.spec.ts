import { DARK_COLOR_TOKENS, LIGHT_COLOR_TOKENS } from "../src/styles/tokens";
import { hexToRgb } from "../tests/lib/expected-colours";
import { expect, test } from "./fixtures.js";

// Behavioural e2e for the DevDesignSystem (/dev) reference page — the scroll-spy,
// deep-link landing, hash sync, and in-page theme toggle that were deferred from
// Phases 5–6 (empty stub sections had no height to scroll between; the sections
// now carry content). The scroll container is RootLayout's <main>, marked with
// `data-scroll-root`.
const SCROLL_ROOT = "[data-scroll-root]";

const hash = (page: import("@playwright/test").Page) =>
  page.evaluate(() => window.location.hash);
const theme = (page: import("@playwright/test").Page) =>
  page.evaluate(() => document.documentElement.dataset.theme ?? "");
const bgColour = (chip: import("@playwright/test").Locator) =>
  chip.evaluate((el) => getComputedStyle(el).backgroundColor);

test.describe("DevDesignSystem (/dev)", () => {
  test("deep-link /dev#colors lands on the page with Colours active", async ({
    page,
  }) => {
    await page.goto("/dev#colors");
    await expect(page).toHaveURL(/\/dev#colors$/);
    // The DEV marquee marks the page (rendered twice for the seamless loop).
    await expect(
      page.getByText("Design system reference").first(),
    ).toBeVisible();
    // The Colours TOC entry is active and exposes it to assistive tech.
    await expect(page.locator('a[title="Colours — #colors"]')).toHaveAttribute(
      "aria-current",
      "location",
    );
  });

  test("scroll-spy advances the active section + hash, never pinned to Colours", async ({
    page,
  }) => {
    await page.goto("/dev");
    // Overview ⇒ bare /dev (no hash written).
    await expect.poll(() => hash(page)).toBe("");

    // Scroll deep past the tall Colours section.
    await page.locator(SCROLL_ROOT).evaluate((el) => {
      el.scrollTo({ top: el.scrollHeight * 0.6, behavior: "auto" });
    });

    // The hash advances to a later section — never stuck on overview, and
    // crucially never pinned to #colors (the prototype's single-dispatch bug).
    await expect.poll(() => hash(page), { timeout: 5000 }).not.toBe("");
    expect(await hash(page)).not.toBe("#colors");

    // Exactly one TOC entry is active, and it matches the written hash.
    const active = page.locator('nav a[aria-current="location"]');
    await expect(active).toHaveCount(1);
    const activeHref = await active.getAttribute("href");
    expect(activeHref).toBe(await hash(page));
  });

  test("scrolling back to the top clears the hash to bare /dev", async ({
    page,
  }) => {
    await page.goto("/dev#code");
    await page.locator(SCROLL_ROOT).evaluate((el) => {
      el.scrollTo({ top: 0, behavior: "auto" });
    });
    await expect.poll(() => hash(page), { timeout: 5000 }).toBe("");
  });

  test("in-page theme toggle flips data-theme and the --ac-bg surface swatch", async ({
    page,
  }) => {
    await page.goto("/dev#colors");
    // Force a known starting theme.
    await page.evaluate(() => {
      document.documentElement.dataset.theme = "light";
    });
    const chip = page.locator('[data-token="--ac-bg"] > div').first();
    await expect
      .poll(() => bgColour(chip))
      .toBe(hexToRgb(LIGHT_COLOR_TOKENS["ac-bg"]));

    // Flip via the in-page control (the chrome toggle; the topbar section has a
    // second one — take the first).
    await page
      .getByRole("button", { name: /dark theme/i })
      .first()
      .click();
    await expect.poll(() => theme(page)).toBe("dark");

    // The surface swatch's computed colour now resolves to the dark token — and
    // the two values differ (the colours section is theme-responsive).
    await expect
      .poll(() => bgColour(chip))
      .toBe(hexToRgb(DARK_COLOR_TOKENS["ac-bg"]));
    expect(hexToRgb(LIGHT_COLOR_TOKENS["ac-bg"])).not.toBe(
      hexToRgb(DARK_COLOR_TOKENS["ac-bg"]),
    );
  });
});
