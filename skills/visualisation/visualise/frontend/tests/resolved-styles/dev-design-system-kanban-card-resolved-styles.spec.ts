import { expect, test } from "@playwright/test";
import { parseRgb, resolveToken, setTheme } from "../lib/expected-colours";

// Computed-style oracle for the drag affordance (A1). The cross-platform pixel
// oracle (kanban-card-showcase.spec.ts) covers the overall look; these probes
// pin the exact values the prototype specifies, with tolerance where engines
// round sub-pixel (transform matrix, box-shadow) and exact equality where they
// serialise deterministically (opacity, cursor, integer-rgb border).
const card = (state: string) =>
  `[data-testid="kanban-card-cell-${state}"] .ac-kcard`;

// Parse `matrix(a, b, c, d, e, f)` into rotation (deg) and uniform scale.
function decomposeMatrix(transform: string): {
  rotationDeg: number;
  scale: number;
} {
  const m = transform.match(/matrix\(([^)]+)\)/);
  if (!m) throw new Error(`not a 2d matrix: ${transform}`);
  const [a, b] = m[1].split(",").map((s) => Number(s.trim()));
  return {
    rotationDeg: (Math.atan2(b, a) * 180) / Math.PI,
    scale: Math.sqrt(a * a + b * b),
  };
}

for (const theme of ["light", "dark"] as const) {
  test.describe(`kanban-card-resolved-styles (${theme})`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/dev#cards");
      // Disable transitions before any theme flip: `.card` now animates
      // border-color/transform/background (140ms), so a theme switch would
      // otherwise be caught mid-transition and the computed border-color would
      // read an intermediate blend rather than the settled accent.
      await page.addStyleTag({
        content:
          "*, *::before, *::after { transition: none !important; animation: none !important; }",
      });
      if (theme === "dark") await setTheme(page, "dark");
    });

    test("resting card uses the grab cursor", async ({ page }) => {
      const cursor = await page
        .locator(card("resting"))
        .evaluate((el) => getComputedStyle(el).cursor);
      expect(cursor).toBe("grab");
    });

    test("dragging card: rotate(1.5deg) scale(1.02), lift shadow, accent border", async ({
      page,
    }) => {
      const el = page.locator(card("dragging"));
      const { transform, boxShadow, borderColor } = await el.evaluate(
        (node) => {
          const s = getComputedStyle(node);
          return {
            transform: s.transform,
            boxShadow: s.boxShadow,
            borderColor: s.borderTopColor,
          };
        },
      );
      // Rotation/scale within an epsilon — the matrix carries sub-pixel rounding.
      const { rotationDeg, scale } = decomposeMatrix(transform);
      expect(rotationDeg).toBeGreaterThan(1.3);
      expect(rotationDeg).toBeLessThan(1.7);
      expect(scale).toBeGreaterThan(1.015);
      expect(scale).toBeLessThan(1.025);
      // Lift shadow present (exact length/colour expansion varies by engine).
      expect(boxShadow).not.toBe("none");
      expect(boxShadow).toMatch(/rgba?\(/);
      // Accent border — integer rgb, compares exactly.
      const accent = parseRgb(await resolveToken(page, "--ac-accent"));
      expect(parseRgb(borderColor)).toEqual(accent);
    });

    test("overlay clone: opacity 0.8 and grabbing cursor", async ({ page }) => {
      const { opacity, cursor } = await page
        .locator(card("overlay"))
        .evaluate((el) => {
          const s = getComputedStyle(el);
          return { opacity: s.opacity, cursor: s.cursor };
        });
      expect(opacity).toBe("0.8");
      expect(cursor).toBe("grabbing");
    });
  });
}
