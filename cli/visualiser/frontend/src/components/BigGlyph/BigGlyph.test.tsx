import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DOC_TYPE_KEYS, type DocTypeKey } from "../../api/types";
import { DOC_TYPE_HUE } from "../../styles/tokens";
import { BIG_GLYPHS, BigGlyph, DEFAULT_BIG_HUE } from "./BigGlyph";
import { PR_REVIEW_DIFF_TINTS } from "./BigGlyph.constants";
import { bigPalette } from "./bigPalette";

// Collect every fill/stroke attribute value across an <svg> subtree, lowercased
// and whitespace-trimmed for case-insensitive set membership comparison.
function collectColours(svg: SVGElement): string[] {
  const out: string[] = [];
  for (const node of Array.from(svg.querySelectorAll("*"))) {
    for (const attr of ["fill", "stroke"] as const) {
      const v = node.getAttribute(attr);
      if (v !== null) out.push(v.trim().toLowerCase());
    }
  }
  return out;
}

// Parse the leading hue out of an `hsl(<hue> <s>% <l>%)` string.
function hslHue(tone: string): number {
  const m = tone.match(/^hsl\(\s*([\d.]+)/);
  if (!m) throw new Error(`Not an hsl() tone: ${tone}`);
  return Number(m[1]);
}

describe("bigPalette", () => {
  it("returns exactly seven tones with white fixed to #ffffff", () => {
    const p = bigPalette(200);
    expect(Object.keys(p).sort()).toEqual([
      "accent",
      "deep",
      "fill",
      "fold",
      "line",
      "stroke",
      "white",
    ]);
    expect(p.white).toBe("#ffffff");
  });

  // Boundary + sample hues. `0` confirms the parse yields exactly 0 (not
  // empty/NaN); 215 is the fallback default; 12/280 are in-union samples.
  it.each([
    0, 12, 215, 280,
  ])("derives all six hue tones from the input hue %i", (hue) => {
    const p = bigPalette(hue);
    for (const tone of [p.stroke, p.fill, p.fold, p.line, p.accent, p.deep]) {
      expect(hslHue(tone)).toBe(hue);
    }
  });
});

describe("PR_REVIEW_DIFF_TINTS", () => {
  it("equals the four exact prototype diff-tint constants", () => {
    expect(PR_REVIEW_DIFF_TINTS).toEqual({
      addedBg: "hsl(140 60% 85%)",
      addedMarker: "hsl(140 50% 40%)",
      removedBg: "hsl(0 65% 88%)",
      removedMarker: "hsl(0 55% 45%)",
    });
  });
});

describe("BIG_GLYPHS dispatch", () => {
  it("has exactly 14 entries, one per DocTypeKey", () => {
    expect(Object.keys(BIG_GLYPHS).length).toBe(14);
    for (const key of DOC_TYPE_KEYS) {
      expect(BIG_GLYPHS[key], `missing dispatch entry for ${key}`).toBeTypeOf(
        "function",
      );
    }
  });

  it("maps every key to a referentially distinct illustration function", () => {
    // Catches a dispatch copy-paste error (two keys pointing at the same
    // illustration) deterministically — a per-cell baseline could not, since
    // distinct hues render distinct bytes even for a shared function.
    expect(new Set(Object.values(BIG_GLYPHS)).size).toBe(14);
  });
});

describe("BigGlyph: DOM + a11y contract", () => {
  it("renders a single <svg> with viewBox 0 0 80 80", () => {
    const { container } = render(<BigGlyph docType="plans" />);
    const svgs = container.querySelectorAll("svg");
    expect(svgs.length).toBe(1);
    expect(svgs[0].getAttribute("viewBox")).toBe("0 0 80 80");
  });

  it("defaults size to 96 (width + height) when omitted", () => {
    const { container } = render(<BigGlyph docType="plans" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("96");
    expect(svg.getAttribute("height")).toBe("96");
  });

  it("honours an explicit size on width + height", () => {
    const { container } = render(<BigGlyph docType="plans" size={120} />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("120");
    expect(svg.getAttribute("height")).toBe("120");
  });

  it('is decorative: aria-hidden="true" and no role', () => {
    const { container } = render(<BigGlyph docType="decisions" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("role")).toBeNull();
  });
});

describe("BigGlyph: hue resolution", () => {
  it("resolves the doc-type hue by default", () => {
    const { container } = render(<BigGlyph docType="pr-reviews" />);
    const svg = container.querySelector("svg")!;
    const expected = `hsl(${DOC_TYPE_HUE["pr-reviews"]} `;
    expect(collectColours(svg).some((c) => c.startsWith(expected))).toBe(true);
  });

  it("an explicit hue prop overrides the doc-type hue", () => {
    const { container } = render(<BigGlyph docType="plans" hue={280} />);
    const svg = container.querySelector("svg")!;
    expect(collectColours(svg).some((c) => c.startsWith("hsl(280 "))).toBe(
      true,
    );
  });

  it("honours the boundary hue={0} via ?? (not ||, which would discard 0)", () => {
    const { container } = render(<BigGlyph docType="plans" hue={0} />);
    const svg = container.querySelector("svg")!;
    expect(collectColours(svg).some((c) => c.startsWith("hsl(0 "))).toBe(true);
  });
});

describe("BigGlyph: off-union fallback", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders the DEFAULT_BIG shape (marked) at the 215 guard hue and warns once", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const docType = "banana" as unknown as DocTypeKey;
    const { container } = render(<BigGlyph docType={docType} />);
    // DEFAULT_BIG shape specifically, not merely "an <svg>".
    const marker = container.querySelector('[data-testid="default-big-glyph"]');
    expect(marker).not.toBeNull();
    // The guard hue (215) is applied to the fallback art.
    const svg = container.querySelector("svg")!;
    expect(
      collectColours(svg).some((c) => c.startsWith(`hsl(${DEFAULT_BIG_HUE} `)),
    ).toBe(true);
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0][0]).toMatch(/Unknown docType "banana"/);
  });

  it("does not warn for an in-union docType", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    render(<BigGlyph docType="plans" />);
    expect(warn).not.toHaveBeenCalled();
  });
});

describe("BigGlyph: source-walk literal guard", () => {
  // Every descendant fill/stroke across all illustrations (plus the
  // fallback) must be a bigPalette tone, `none`, or a TYPE-SCOPED sanctioned
  // constant. Match by exact set membership after lowercasing — so a
  // wrong-lightness tone or a misplaced sanctioned constant (e.g. a diff tint
  // on a non-pr-reviews type) is rejected, while a verbatim `#ffffff` is not.
  const SHADOW = "rgba(0,0,0,0.08)";

  function allowedFor(docType: DocTypeKey): Set<string> {
    const hue = DOC_TYPE_HUE[docType] ?? DEFAULT_BIG_HUE;
    const set = new Set(
      Object.values(bigPalette(hue)).map((c) => c.toLowerCase()),
    );
    set.add("none");
    if (docType === "pr-reviews") {
      for (const tint of Object.values(PR_REVIEW_DIFF_TINTS)) {
        set.add(tint.toLowerCase());
      }
    }
    if (docType === "notes") set.add(SHADOW.toLowerCase());
    return set;
  }

  it.each(
    DOC_TYPE_KEYS,
  )("uses only sanctioned colours in the %s illustration", (docType) => {
    const allowed = allowedFor(docType);
    const { container } = render(<BigGlyph docType={docType} />);
    const svg = container.querySelector("svg")!;
    for (const colour of collectColours(svg)) {
      expect(
        allowed.has(colour),
        `${docType}: stray colour "${colour}" not in the sanctioned set`,
      ).toBe(true);
    }
  });
});
