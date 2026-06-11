import { expect, type Page, test } from "@playwright/test";
import {
  DARK_COLOR_TOKENS,
  DARK_SHADOW_TOKENS,
  LIGHT_COLOR_TOKENS,
  LIGHT_SHADOW_TOKENS,
} from "../../src/styles/tokens";
import {
  hexToRgb,
  parseRgb,
  setTheme,
  type Theme,
} from "./lib/expected-colours";

const SHADOW_KEYS = ["ac-shadow-soft", "ac-shadow-lift"] as const;
const COLOR_KEYS = ["ac-accent", "ac-accent-2"] as const;
type ShadowKey = (typeof SHADOW_KEYS)[number];
type ColorKey = (typeof COLOR_KEYS)[number];
type TokenSnapshot = {
  shadows: Record<ShadowKey, string>;
  colors: Record<ColorKey, string>;
};

const EXPECTED_SHADOWS: Record<Theme, Record<ShadowKey, string>> = {
  light: {
    "ac-shadow-soft": LIGHT_SHADOW_TOKENS["ac-shadow-soft"],
    "ac-shadow-lift": LIGHT_SHADOW_TOKENS["ac-shadow-lift"],
  },
  dark: {
    "ac-shadow-soft": DARK_SHADOW_TOKENS["ac-shadow-soft"],
    "ac-shadow-lift": DARK_SHADOW_TOKENS["ac-shadow-lift"],
  },
};

const EXPECTED_COLORS: Record<Theme, Record<ColorKey, string>> = {
  light: {
    "ac-accent": LIGHT_COLOR_TOKENS["ac-accent"],
    "ac-accent-2": LIGHT_COLOR_TOKENS["ac-accent-2"],
  },
  dark: {
    "ac-accent": DARK_COLOR_TOKENS["ac-accent"],
    "ac-accent-2": DARK_COLOR_TOKENS["ac-accent-2"],
  },
};

// Collapse internal whitespace to a single space rather than removing it
// entirely. Tolerates Chromium re-spacing inside `rgba(...)` argument lists
// while preserving the mandatory separator between shadow components, so a
// dropped separator (`0 1px 2px` → `01px2px`) cannot compare equal to a
// broken declaration. Also canonicalises fractional alpha forms: Chromium
// re-serialises `0.X` as `.X` (drops the leading zero) and strips trailing
// zeros (`0.10` → `.1`), so we apply the same canonical form to both sides.
const normaliseShadow = (s: string) =>
  s
    .toLowerCase()
    .replace(/\s+/g, " ")
    .replace(/(^|[^0-9])0\.(\d)/g, "$1.$2")
    .replace(/(\.\d*?)0+(?=\D|$)/g, "$1")
    .trim();

async function readRootTokens(page: Page): Promise<TokenSnapshot> {
  return page.evaluate(
    ({ shadowKeys, colorKeys }) => {
      const root = getComputedStyle(document.documentElement);
      const readShadow = (k: string) => root.getPropertyValue(`--${k}`).trim();
      // Colour tokens in light theme indirect through `var(--atomic-X)`
      // brand-layer references; `getPropertyValue` returns the literal
      // `var(...)` text for unregistered custom properties. Resolving via
      // a throwaway element's `color` property forces Chromium to
      // serialise the resolved rgb. Mirrors `chip-resolved-colours.spec.ts`.
      const resolveColor = (k: string) => {
        const tmp = document.createElement("div");
        tmp.style.color = `var(--${k})`;
        document.body.appendChild(tmp);
        const resolved = getComputedStyle(tmp).color;
        tmp.remove();
        return resolved;
      };
      return {
        shadows: Object.fromEntries(shadowKeys.map((k) => [k, readShadow(k)])),
        colors: Object.fromEntries(colorKeys.map((k) => [k, resolveColor(k)])),
      };
    },
    { shadowKeys: [...SHADOW_KEYS], colorKeys: [...COLOR_KEYS] },
  ) as Promise<TokenSnapshot>;
}

function assertParity(theme: Theme, actual: TokenSnapshot) {
  for (const k of SHADOW_KEYS) {
    expect(normaliseShadow(actual.shadows[k])).toEqual(
      normaliseShadow(EXPECTED_SHADOWS[theme][k]),
    );
  }
  // Expected side is a 6-char hex literal from tokens.ts → `hexToRgb` →
  // `rgb(r, g, b)`. Actual side comes from `getComputedStyle(...).color`,
  // which Chromium serialises as `rgb(r, g, b)` or (rarely) `color(srgb …)`.
  // Route both through `parseRgb` for a tuple comparison that closes AC#3's
  // "normalised to `rgb()` notation" wording on the data-model level.
  for (const k of COLOR_KEYS) {
    expect(parseRgb(actual.colors[k])).toEqual(
      parseRgb(hexToRgb(EXPECTED_COLORS[theme][k])),
    );
  }
}

test.describe("root resolved tokens", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      try {
        localStorage.removeItem("ac-theme");
      } catch {
        /* private-mode SecurityError */
      }
    });
  });

  for (const theme of ["light", "dark"] as const) {
    test(`values resolve under [data-theme="${theme}"]`, async ({ page }) => {
      await page.goto("/library");
      await setTheme(page, theme);
      assertParity(theme, await readRootTokens(page));
    });
  }

  test("values resolve under prefers-color-scheme: dark (no data-theme)", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/library");
    // useTheme's mount effect unconditionally writes data-theme on mount.
    // Under emulated dark + empty localStorage, readInitial resolves to
    // 'dark'. Wait for that write to land BEFORE removing the attribute —
    // the effect's deps are `[theme]`, so once fired it does not re-fire
    // without a state change, and the manual removal sticks.
    await page.waitForFunction(
      () => document.documentElement.getAttribute("data-theme") === "dark",
    );
    await page.evaluate(() => {
      document.documentElement.removeAttribute("data-theme");
    });
    // Invariant guard: confirm MIRROR-B (`:root:not([data-theme="light"])`
    // under `@media (prefers-color-scheme: dark)`) is the cascade source,
    // not MIRROR-A. If React re-applied the attribute, this fails loudly
    // rather than silently degenerating into a MIRROR-A check.
    expect(
      await page.evaluate(() =>
        document.documentElement.hasAttribute("data-theme"),
      ),
    ).toBe(false);
    assertParity("dark", await readRootTokens(page));
  });
});
