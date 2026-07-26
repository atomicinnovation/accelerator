import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ResultBadge } from "./ResultBadge";

describe("ResultBadge", () => {
  describe("observable hook", () => {
    it('renders with data-testid="result-badge"', () => {
      const { container } = render(<ResultBadge value="pass" />);
      expect(
        container.querySelector('[data-testid="result-badge"]'),
      ).not.toBeNull();
    });
  });

  describe("aria-label (inherited via composition)", () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: literal describing the aria-label format in the test name — not a template
    it('renders aria-label of "${key}: ${value}"', () => {
      const { container } = render(<ResultBadge value="pass" />);
      expect(
        container.querySelector('[aria-label="result: pass"]'),
      ).not.toBeNull();
    });
  });

  describe("result vocabulary", () => {
    it.each([
      ["pass", "green"],
      ["partial", "amber"],
      ["fail", "red"],
    ])("result %s → %s", (value, expected) => {
      const { container } = render(<ResultBadge value={value} />);
      expect(
        container.querySelector(`[data-variant="${expected}"]`),
      ).not.toBeNull();
    });
  });

  describe("case insensitivity", () => {
    it.each([
      ["pass", "green"],
      ["Pass", "green"],
      ["PASS", "green"],
      ["fail", "red"],
      ["FAIL", "red"],
    ])("result %s → %s", (value, expected) => {
      const { container } = render(<ResultBadge value={value} />);
      expect(
        container.querySelector(`[data-variant="${expected}"]`),
      ).not.toBeNull();
    });
  });

  describe("neutral fallback", () => {
    it.each(["xyz", "", "undecided"])("unmapped %s → neutral", (value) => {
      const { container } = render(<ResultBadge value={value} />);
      expect(
        container.querySelector('[data-variant="neutral"]'),
      ).not.toBeNull();
    });

    it.each([
      null,
      undefined,
      42,
      true,
    ] as const)("non-string → neutral", (value) => {
      const { container } = render(<ResultBadge value={value} />);
      expect(
        container.querySelector('[data-variant="neutral"]'),
      ).not.toBeNull();
    });
  });

  describe("vocabulary isolation", () => {
    it.each([
      "APPROVE",
      "REVISE",
      "REQUEST_CHANGES",
      "COMMENT",
      "done",
      "accepted",
      "blocked",
    ])("non-result-vocab %s under result → neutral", (value) => {
      const { container } = render(<ResultBadge value={value} />);
      expect(
        container.querySelector('[data-variant="neutral"]'),
      ).not.toBeNull();
    });
  });
});
