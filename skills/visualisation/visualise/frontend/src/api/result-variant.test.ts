import { describe, expect, it } from "vitest";
import { normaliseValue } from "./normalise-value";
import { __SETS_FOR_TEST, resultToVariant } from "./result-variant";

describe("resultToVariant", () => {
  describe("internal invariants", () => {
    it("all Set keys are in normalised form", () => {
      expect(__SETS_FOR_TEST).toBeDefined();
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0);
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0);
        for (const k of s) {
          expect(normaliseValue(k)).toBe(k);
        }
      }
    });
  });

  describe("validation result vocabulary", () => {
    it.each([
      ["pass", "green"],
      ["partial", "amber"],
      ["fail", "red"],
    ])("maps %s → %s", (v, expected) => {
      expect(resultToVariant(v)).toBe(expected);
    });
  });

  describe("case insensitivity", () => {
    it.each([
      ["pass", "green"],
      ["Pass", "green"],
      ["PASS", "green"],
      ["fail", "red"],
      ["FAIL", "red"],
      ["partial", "amber"],
      ["Partial", "amber"],
    ])("maps %s → %s", (v, expected) => {
      expect(resultToVariant(v)).toBe(expected);
    });
  });

  describe("neutral fallback", () => {
    it.each(["xyz", "", "undecided", "unknown"])("unmapped %s → neutral", (v) =>
      expect(resultToVariant(v)).toBe("neutral"));

    it.each([
      null,
      undefined,
      42,
      true,
      ["a"],
      { x: 1 },
    ] as const)("non-string → neutral", (v) =>
      expect(resultToVariant(v as unknown)).toBe("neutral"));
  });

  describe("vocabulary isolation", () => {
    it.each([
      ["APPROVE", "neutral"],
      ["REVISE", "neutral"],
      ["REQUEST_CHANGES", "neutral"],
      ["COMMENT", "neutral"],
      ["done", "neutral"],
      ["accepted", "neutral"],
    ])("non-result-vocab %s → %s", (v, expected) => {
      expect(resultToVariant(v)).toBe(expected);
    });
  });
});
