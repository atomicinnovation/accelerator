import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CodeSyntaxShowcase, FIXTURES } from "./CodeSyntaxShowcase";

describe("CodeSyntaxShowcase", () => {
  it("renders the top-level showcase container", () => {
    const { container } = render(<CodeSyntaxShowcase />);
    expect(
      container.querySelector('[data-testid="code-syntax-showcase"]'),
    ).not.toBeNull();
  });

  it.each(
    FIXTURES.map(({ lang }) => ({ lang })),
  )("renders a cell with a fenced code block for $lang", ({ lang }) => {
    const { container } = render(<CodeSyntaxShowcase />);
    const cell = container.querySelector(
      `[data-testid="code-syntax-cell-${lang}"]`,
    );
    expect(cell).not.toBeNull();
    const pre = cell!.querySelector("pre code");
    expect(pre).not.toBeNull();
    expect(pre!.className).toMatch(new RegExp(`\\blanguage-${lang}\\b`));
    expect(pre!.className).toMatch(/\bhljs\b/);
  });
});
