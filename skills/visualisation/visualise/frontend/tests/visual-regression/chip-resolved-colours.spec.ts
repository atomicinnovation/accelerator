import { expect, test } from "@playwright/test";
import { DARK_COLOR_TOKENS, LIGHT_COLOR_TOKENS } from "../../src/styles/tokens";
import {
  expectChannelsBetween,
  hexToRgb,
  setTheme,
} from "./lib/expected-colours";

const COLOUR_CODED = ["indigo", "green", "amber", "red", "violet"] as const;

const EXPECTED_FG_LIGHT: Record<(typeof COLOUR_CODED)[number], string> = {
  indigo: hexToRgb(LIGHT_COLOR_TOKENS["ac-accent"]),
  green: hexToRgb(LIGHT_COLOR_TOKENS["ac-ok"]),
  amber: hexToRgb(LIGHT_COLOR_TOKENS["ac-warn"]),
  red: hexToRgb(LIGHT_COLOR_TOKENS["ac-err"]),
  violet: hexToRgb(LIGHT_COLOR_TOKENS["ac-violet"]),
};

const EXPECTED_FG_DARK: Record<(typeof COLOUR_CODED)[number], string> = {
  indigo: hexToRgb(DARK_COLOR_TOKENS["ac-accent"]),
  green: hexToRgb(DARK_COLOR_TOKENS["ac-ok"]),
  amber: hexToRgb(DARK_COLOR_TOKENS["ac-warn"]),
  red: hexToRgb(DARK_COLOR_TOKENS["ac-err"]),
  // --ac-violet has no dark override; remains theme-invariant.
  violet: hexToRgb(LIGHT_COLOR_TOKENS["ac-violet"]),
};

for (const theme of ["light", "dark"] as const) {
  test.describe(`chip-resolved-colours (${theme})`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/chip-showcase");
      if (theme === "dark") {
        await setTheme(page, "dark");
      }
    });

    for (const variant of COLOUR_CODED) {
      test(`${variant}: foreground matches the resolved token hex`, async ({
        page,
      }) => {
        const chip = page.locator(
          `[data-testid="chip-cell-${variant}-sm"] [data-variant="${variant}"]`,
        );
        const fg = await chip.evaluate((el) => getComputedStyle(el).color);
        const expected =
          theme === "light"
            ? EXPECTED_FG_LIGHT[variant]
            : EXPECTED_FG_DARK[variant];
        expect(fg).toBe(expected);
      });

      test(`${variant}: background sits between --ac-bg and the semantic colour`, async ({
        page,
      }) => {
        const chip = page.locator(
          `[data-testid="chip-cell-${variant}-sm"] [data-variant="${variant}"]`,
        );
        const bg = await chip.evaluate(
          (el) => getComputedStyle(el).backgroundColor,
        );
        // Resolve --ac-bg through the cascade (it may indirect through the
        // brand layer as var(--atomic-X)) by setting it as a color on a
        // throwaway element and reading the computed value. Returns an
        // `rgb(...)` string directly, which parseRgb handles.
        const acBg = await page.evaluate(() => {
          const tmp = document.createElement("div");
          tmp.style.color = "var(--ac-bg)";
          document.body.appendChild(tmp);
          const resolved = getComputedStyle(tmp).color;
          tmp.remove();
          return resolved;
        });
        const expectedFg =
          theme === "light"
            ? EXPECTED_FG_LIGHT[variant]
            : EXPECTED_FG_DARK[variant];
        expectChannelsBetween(bg, expectedFg, acBg);
      });
    }
  });
}
