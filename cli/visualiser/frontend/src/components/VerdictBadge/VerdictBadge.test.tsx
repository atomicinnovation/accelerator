import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VerdictBadge } from "./VerdictBadge";

describe("VerdictBadge", () => {
  describe("observable hook", () => {
    it('renders with data-testid="verdict-badge"', () => {
      const { container } = render(<VerdictBadge value="APPROVE" />);
      expect(
        container.querySelector('[data-testid="verdict-badge"]'),
      ).not.toBeNull();
    });
  });

  describe("aria-label (inherited via composition)", () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: literal describing the aria-label format in the test name — not a template
    it('renders aria-label of "${key}: ${value}"', () => {
      const { container } = render(<VerdictBadge value="APPROVE" />);
      expect(
        container.querySelector('[aria-label="verdict: APPROVE"]'),
      ).not.toBeNull();
    });
  });

  describe("plan-review verdict vocabulary", () => {
    it.each([
      ["APPROVE", "green"],
      ["REVISE", "amber"],
      ["REQUEST_CHANGES", "red"],
      ["COMMENT", "neutral"],
    ])("verdict %s → %s", (value, expected) => {
      const { container } = render(<VerdictBadge value={value} />);
      expect(
        container.querySelector(`[data-variant="${expected}"]`),
      ).not.toBeNull();
    });
  });

  describe("case insensitivity", () => {
    it.each([
      ["approve", "green"],
      ["Approve", "green"],
      ["APPROVE", "green"],
    ])("verdict %s → green", (value) => {
      const { container } = render(<VerdictBadge value={value} />);
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull();
    });
  });

  describe("neutral fallback", () => {
    it.each(["xyz", "", "undecided"])("unmapped %s → neutral", (value) => {
      const { container } = render(<VerdictBadge value={value} />);
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
      const { container } = render(<VerdictBadge value={value} />);
      expect(
        container.querySelector('[data-variant="neutral"]'),
      ).not.toBeNull();
    });
  });

  describe("vocabulary isolation", () => {
    it.each([
      "done",
      "accepted",
      "blocked",
      "rejected",
    ])("status-shaped %s under verdict → neutral", (value) => {
      const { container } = render(<VerdictBadge value={value} />);
      expect(
        container.querySelector('[data-variant="neutral"]'),
      ).not.toBeNull();
    });

    it.each([
      "pass",
      "fail",
      "partial",
    ])("result-shaped %s under verdict → neutral (handled by ResultBadge only)", (value) => {
      const { container } = render(<VerdictBadge value={value} />);
      expect(
        container.querySelector('[data-variant="neutral"]'),
      ).not.toBeNull();
    });
  });
});
