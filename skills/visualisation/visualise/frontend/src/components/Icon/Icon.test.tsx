import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Icon } from "./Icon";
import { ICON_NAMES, type IconName } from "./Icon.constants";

// Compile-time type-rejection guard. The @ts-expect-error directive fires
// when `typecheck` (tsc --noEmit) runs; `npm test` alone does not enforce it.
// biome-ignore lint/suspicious/noExportsInTest: the export is load-bearing — it keeps tsc's noUnusedLocals from eliding this compile-time @ts-expect-error type guard; the function is never imported or run
export function _typeContractGuards(): void {
  // @ts-expect-error — "banana" is not a member of IconName.
  void (<Icon name="banana" />);
}

describe("Icon: name registry", () => {
  it("ICON_NAMES has exactly 33 entries", () => {
    expect(ICON_NAMES.length).toBe(33);
  });

  it("every registered name renders an <svg>", () => {
    for (const name of ICON_NAMES) {
      const { container, unmount } = render(<Icon name={name} />);
      const svg = container.querySelector("svg");
      expect(svg, `Icon "${name}" did not render an <svg>`).not.toBeNull();
      unmount();
    }
  });

  it("every registered name renders at least one child shape", () => {
    for (const name of ICON_NAMES) {
      const { container, unmount } = render(<Icon name={name} />);
      const svg = container.querySelector("svg")!;
      expect(
        svg.querySelectorAll("path, circle, rect, line, polygon").length,
        `Icon "${name}" rendered an empty <svg>`,
      ).toBeGreaterThan(0);
      unmount();
    }
  });
});

describe("Icon: runtime DOM shape", () => {
  it("root element is <svg> with viewBox 0 0 24 24", () => {
    const { container } = render(<Icon name="search" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute("viewBox")).toBe("0 0 24 24");
  });

  it("default size is 16 (width and height)", () => {
    const { container } = render(<Icon name="search" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("16");
    expect(svg.getAttribute("height")).toBe("16");
  });

  it("size prop sets both width and height", () => {
    const { container } = render(<Icon name="search" size={28} />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("28");
    expect(svg.getAttribute("height")).toBe("28");
  });

  it('root <svg> strokes via currentColor (tints through CSS "color")', () => {
    const { container } = render(<Icon name="check" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("stroke")).toBe("currentColor");
    expect(svg.getAttribute("fill")).toBe("none");
  });

  it("applies a passed className alongside the module class", () => {
    const { container } = render(<Icon name="check" className="extra" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("class")).toMatch(/\bextra\b/);
  });
});

// Canonical geometry assertions for the app-icon refactor (Phase 2). Baseline
// regeneration blesses whatever rendered — it cannot catch a wrong stroke-width
// or a swapped icon. These pin the path data + the 2px stroke for the icons the
// refactor migrates most widely, so a geometry drift fails in unit CI, not only
// in a human reading regenerated pixel diffs.
describe("Icon: canonical geometry", () => {
  it("root <svg> carries a 2px stroke", () => {
    const { container } = render(<Icon name="chevron-right" />);
    expect(container.querySelector("svg")!.getAttribute("stroke-width")).toBe(
      "2",
    );
  });

  const CANONICAL: ReadonlyArray<[IconName, string]> = [
    ["chevron-right", "m9 6 6 6-6 6"],
    ["chevron-down", "m6 9 6 6 6-6"],
    ["check", "m5 12 5 5L20 7"],
    ["filter", "M4 4h16l-6 8v6l-4 2v-8z"],
  ];

  it.each(CANONICAL)("%s renders its canonical path data", (name, d) => {
    const { container } = render(<Icon name={name} />);
    const paths = Array.from(container.querySelectorAll("path")).map((p) =>
      p.getAttribute("d"),
    );
    expect(paths).toContain(d);
  });

  it("search renders a circle of radius 7 plus the handle path", () => {
    const { container } = render(<Icon name="search" />);
    const circle = container.querySelector("circle")!;
    expect(circle.getAttribute("r")).toBe("7");
    const paths = Array.from(container.querySelectorAll("path")).map((p) =>
      p.getAttribute("d"),
    );
    expect(paths).toContain("m20 20-3.5-3.5");
  });
});

describe("Icon: accessibility branches", () => {
  it('decorative default carries explicit aria-hidden="true" and no role/label', () => {
    const { container } = render(<Icon name="search" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("role")).toBeNull();
    expect(svg.getAttribute("aria-label")).toBeNull();
  });

  it('ariaLabel flips to role="img" + aria-label and drops aria-hidden', () => {
    const { container } = render(<Icon name="search" ariaLabel="Search" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("role")).toBe("img");
    expect(svg.getAttribute("aria-label")).toBe("Search");
    expect(svg.getAttribute("aria-hidden")).toBeNull();
  });

  it("default render is not exposed as an image to assistive tech", () => {
    const { queryByRole } = render(<Icon name="search" />);
    expect(queryByRole("img")).toBeNull();
  });

  it("with ariaLabel, render exposes role=img with the given name", () => {
    const { getByRole } = render(<Icon name="search" ariaLabel="Search" />);
    expect(getByRole("img", { name: "Search" })).toBeTruthy();
  });
});

describe("Icon: runtime guard", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders nothing and warns once for an off-registry name in dev", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    // Force an unknown name past the type system to exercise the dev guard.
    const name = "banana" as unknown as IconName;
    const { container } = render(<Icon name={name} />);
    expect(container.querySelector("svg")).toBeNull();
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0][0]).toMatch(/Unknown name: banana/);
  });
});
