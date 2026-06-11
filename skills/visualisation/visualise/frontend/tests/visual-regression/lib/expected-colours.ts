import { expect, type Page } from "@playwright/test";
import { DOC_TYPE_KEYS, type DocTypeKey } from "../../../src/api/types";
import { DOC_TYPE_TOKEN_KEY } from "../../../src/components/Glyph/Glyph.constants";
import {
  DARK_COLOR_TOKENS,
  LIGHT_COLOR_TOKENS,
} from "../../../src/styles/tokens";

export type Theme = "light" | "dark";

export function hexToRgb(hex: string): string {
  const v = hex.replace("#", "");
  const r = parseInt(v.slice(0, 2), 16);
  const g = parseInt(v.slice(2, 4), 16);
  const b = parseInt(v.slice(4, 6), 16);
  return `rgb(${r}, ${g}, ${b})`;
}

export function parseRgb(rgb: string): [number, number, number] {
  // Legacy form: `rgb(r, g, b)` / `rgba(r, g, b, a)` with 0..255 integer
  // channels.
  const legacy = rgb.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/);
  if (legacy) return [Number(legacy[1]), Number(legacy[2]), Number(legacy[3])];

  // CSS Color Level 4: `color(srgb r g b [/ a])` with 0..1 float channels.
  // Chromium serialises `color-mix(in srgb, …)` results in this form. Round
  // to 0..255 so we can range-compare against `hexToRgb` output.
  const modern = rgb.match(
    /color\(\s*srgb\s+([\d.eE+-]+)\s+([\d.eE+-]+)\s+([\d.eE+-]+)/,
  );
  if (modern) {
    const to255 = (s: string) => Math.round(Number(s) * 255);
    return [to255(modern[1]), to255(modern[2]), to255(modern[3])];
  }

  throw new Error(`Cannot parse colour: ${rgb}`);
}

export function expectChannelsBetween(actual: string, a: string, b: string) {
  const [ar, ag, ab] = parseRgb(actual);
  const [xr, xg, xb] = parseRgb(a);
  const [yr, yg, yb] = parseRgb(b);
  expect(ar).toBeGreaterThanOrEqual(Math.min(xr, yr));
  expect(ar).toBeLessThanOrEqual(Math.max(xr, yr));
  expect(ag).toBeGreaterThanOrEqual(Math.min(xg, yg));
  expect(ag).toBeLessThanOrEqual(Math.max(xg, yg));
  expect(ab).toBeGreaterThanOrEqual(Math.min(xb, yb));
  expect(ab).toBeLessThanOrEqual(Math.max(xb, yb));
}

const TOKEN_TABLE: Record<Theme, Record<string, string>> = {
  light: LIGHT_COLOR_TOKENS,
  dark: DARK_COLOR_TOKENS,
};

// Per-doc-type expected hex by theme, derived from the typed
// DOC_TYPE_TOKEN_KEY lookup (templates → ac-fg-muted, the other 12 →
// ac-doc-<key>). No string parsing of var() expressions.
export const EXPECTED_COLOR: Record<
  DocTypeKey,
  Record<Theme, string>
> = Object.fromEntries(
  DOC_TYPE_KEYS.map((key) => {
    const tokenKey = DOC_TYPE_TOKEN_KEY[key];
    return [
      key,
      {
        light: TOKEN_TABLE.light[tokenKey],
        dark: TOKEN_TABLE.dark[tokenKey],
      },
    ];
  }),
) as Record<DocTypeKey, Record<Theme, string>>;

// Resolve a CSS custom property to its concrete computed value by setting
// it as `color` on a throwaway element and reading the serialised rgb/rgba.
// Mirrors the inline pattern in chip-resolved-colours.spec.ts:56-63 and
// root-resolved-tokens.spec.ts:71-78 — works for `var()`-indirected tokens
// (light colours route through the brand layer) and for `rgba(...)` tokens
// like `--ac-stroke-soft` that `hexToRgb` could not handle.
export async function resolveToken(page: Page, token: string): Promise<string> {
  return page.evaluate((t) => {
    const tmp = document.createElement("div");
    tmp.style.color = `var(${t})`;
    document.body.appendChild(tmp);
    const resolved = getComputedStyle(tmp).color;
    tmp.remove();
    return resolved;
  }, token);
}

export async function setTheme(page: Page, theme: Theme): Promise<void> {
  // Matches the convention used by chip-resolved-colours.spec.ts and
  // glyph-resolved-fill.spec.ts.
  await page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
  }, theme);
  await page.waitForFunction(
    (t) => document.documentElement.dataset.theme === t,
    theme,
  );
}
