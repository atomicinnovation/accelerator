// Test-only helper. Consumers: global.test.ts,
// prototype-tokens.fixture.test.ts.
//
// Do not import from production code.

import { BRAND_COLOR_TOKENS } from "../tokens";

function rgbToHex(r: string, g: string, b: string): string {
  const hex = (n: string) => Number(n).toString(16).padStart(2, "0");
  return `#${hex(r)}${hex(g)}${hex(b)}`;
}

/**
 * Normalise a CSS colour value for parity comparison. Handles:
 *  - whitespace and case (lowercase + strip whitespace)
 *  - rgb(r, g, b) → #rrggbb (six-digit lowercase hex)
 *  - var(--atomic-X) → look through BRAND_COLOR_TOKENS, recur
 *    (alias chains followed; cycle guard prevents infinite recursion)
 *  - rgba(...) and #XXXXXX pass through unchanged after stripping
 *  - var() refs whose name is not an --atomic-* prefix pass through
 *    unchanged (consumer's semantic-layer refs, not brand-layer)
 *
 * Throws if a var(--atomic-X) ref names an --atomic-* token that
 * does not exist in BRAND_COLOR_TOKENS — this is a bug, not a soft
 * mismatch, and a hard failure produces an actionable error message
 * rather than an opaque string-mismatch test failure downstream.
 *
 * Domain assumption: the prototype uses comma-separated integer
 * rgb() with 0-255 channels. Whitespace-separated rgb(), percentage
 * channels, and 4-channel rgba() shapes are out of scope; values
 * outside that domain fall through to the lowercased/stripped form.
 */
export function canonicaliseBrand(v: string): string {
  return resolve(v, new Set());
}

function resolve(v: string, seen: Set<string>): string {
  const s = v.toLowerCase().replace(/\s+/g, "");
  const rgb = /^rgb\((\d{1,3}),(\d{1,3}),(\d{1,3})\)$/.exec(s);
  if (rgb) return rgbToHex(rgb[1], rgb[2], rgb[3]);

  const ref = /^var\(--(atomic-[\w-]+)\)$/.exec(s);
  if (ref) {
    const name = ref[1];
    if (seen.has(name)) {
      throw new Error(`canonicaliseBrand: cycle detected at --${name}`);
    }
    const target = (BRAND_COLOR_TOKENS as Record<string, string | undefined>)[
      name
    ];
    if (target === undefined) {
      throw new Error(
        `canonicaliseBrand: unknown brand token --${name}; ` +
          `check spelling or add to BRAND_COLOR_TOKENS`,
      );
    }
    return resolve(target, new Set(seen).add(name));
  }

  // Non-brand var() refs (e.g. var(--ac-bg)) pass through unchanged
  // so callers can detect them via string mismatch rather than
  // exception. Only --atomic-* refs are resolved here.
  return s;
}

export { rgbToHex };
