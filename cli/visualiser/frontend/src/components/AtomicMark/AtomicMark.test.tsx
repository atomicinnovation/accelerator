import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AtomicMark } from "./AtomicMark";

describe("AtomicMark", () => {
  it("renders an aria-hidden SVG with the gradient stops and accent-2 dot", () => {
    const { container } = render(<AtomicMark />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("aria-hidden")).toBe("true");

    const stopColors = Array.from(container.querySelectorAll("stop")).map((s) =>
      s.getAttribute("stop-color"),
    );
    expect(stopColors).toContain("var(--ac-accent)");
    expect(stopColors).toContain("var(--ac-accent-2)");

    const dotFills = Array.from(container.querySelectorAll("circle"))
      .map((c) => c.getAttribute("fill"))
      .filter(Boolean);
    expect(dotFills).toContain("var(--ac-accent-2)");
  });

  it("defaults to size 28 and honours an explicit size", () => {
    const { container: a } = render(<AtomicMark />);
    const svgA = a.querySelector("svg")!;
    expect(svgA.getAttribute("width")).toBe("28");
    expect(svgA.getAttribute("height")).toBe("28");

    const { container: b } = render(<AtomicMark size={72} />);
    const svgB = b.querySelector("svg")!;
    expect(svgB.getAttribute("width")).toBe("72");
    expect(svgB.getAttribute("height")).toBe("72");
  });

  it("mints a distinct gradient id per instance (no collision on one page)", () => {
    const { container } = render(
      <>
        <AtomicMark size={24} />
        <AtomicMark size={48} />
      </>,
    );
    const ids = Array.from(container.querySelectorAll("linearGradient")).map(
      (g) => g.getAttribute("id"),
    );
    expect(ids).toHaveLength(2);
    expect(ids[0]).not.toBe(ids[1]);
    // Each id must be a valid SVG fragment target (no useId colons).
    for (const id of ids) {
      expect(id).not.toContain(":");
    }
  });

  it("wires the hexagon stroke to its own gradient id", () => {
    const { container } = render(<AtomicMark />);
    const gradientId = container
      .querySelector("linearGradient")!
      .getAttribute("id");
    const stroke = container.querySelector("path")!.getAttribute("stroke");
    expect(stroke).toBe(`url(#${gradientId})`);
  });
});
