import { describe, expect, it } from "vitest";
import { contrastRatio, hslFromHex } from "./contrast";
import { DARK_COLOR_TOKENS, LIGHT_COLOR_TOKENS } from "./tokens";

// The topic-research lifecycle chips carry the accessibility gate on their own
// numeric bounds (they deliberately deviate from the semantic ok/warn/err
// scale). The fills are plain hex tokens (no color-mix), so a direct
// sRGB→HSL resolver is enough — no CSS `color-mix` resolution needed.

describe("hslFromHex golden values", () => {
  it.each([
    ["#ffffff", 0, 0, 100],
    ["#000000", 0, 0, 0],
    ["#808080", 0, 0, 50.2],
    ["#4cb7cd", 190.23, 56.33, 55.1],
  ])("%s → h≈%d s≈%d l≈%d", (hex, h, s, l) => {
    const got = hslFromHex(hex as string);
    expect(got.h).toBeCloseTo(h as number, 0);
    expect(got.s).toBeCloseTo(s as number, 0);
    expect(got.l).toBeCloseTo(l as number, 0);
  });
});

const STEPS = [
  "briefed",
  "outlined",
  "researching",
  "synthesised",
  "complete",
] as const;

// A lone chip is read with no reference ramp beside it, so adjacent steps
// carry a generous lightness separation rather than the bare minimum.
const MIN_ADJACENT_LIGHTNESS_DELTA = 5;

describe.each([
  ["light", LIGHT_COLOR_TOKENS],
  ["dark", DARK_COLOR_TOKENS],
] as const)("lifecycle chip ramp (%s theme)", (_theme, tokens) => {
  const fg = (tokens as Record<string, string>)["ac-lifecycle-fg"];
  const fills = STEPS.map(
    (step) => (tokens as Record<string, string>)[`ac-lifecycle-${step}`],
  );

  it.each(
    STEPS.map((step, i) => [step, fills[i]] as const),
  )("%s (%s) clears >=15%% saturation and >=3:1 text contrast", (_step, fill) => {
    expect(hslFromHex(fill).s).toBeGreaterThanOrEqual(15);
    expect(contrastRatio(fill, fg)).toBeGreaterThanOrEqual(3);
  });

  it("adjacent steps differ in lightness by a generous margin", () => {
    const lightness = fills.map((fill) => hslFromHex(fill).l);
    for (let i = 1; i < lightness.length; i++) {
      expect(Math.abs(lightness[i] - lightness[i - 1])).toBeGreaterThanOrEqual(
        MIN_ADJACENT_LIGHTNESS_DELTA,
      );
    }
  });

  it("briefed carries enough chroma and lightness to read as active, not grey", () => {
    expect(hslFromHex(fills[0]).s).toBeGreaterThanOrEqual(15);
  });
});
