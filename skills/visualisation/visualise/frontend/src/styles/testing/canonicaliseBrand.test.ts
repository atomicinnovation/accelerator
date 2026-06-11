import { describe, expect, it, vi } from "vitest";
import { canonicaliseBrand } from "./canonicaliseBrand";

describe("canonicaliseBrand", () => {
  it("normalises rgb(...) to lowercase six-digit hex", () => {
    expect(canonicaliseBrand("rgb(14, 15, 25)")).toBe("#0e0f19");
  });
  it("lowercases plain hex without changing channels", () => {
    expect(canonicaliseBrand("#0E0F19")).toBe("#0e0f19");
  });
  it("resolves var(--atomic-X) through BRAND_COLOR_TOKENS", () => {
    expect(canonicaliseBrand("var(--atomic-bone)")).toBe("#fbfcfe");
  });
  it("resolves alias to hex via BRAND_COLOR_TOKENS (single hop today; recursion-safe by design)", () => {
    expect(canonicaliseBrand("var(--atomic-violet)")).toBe("#965dd9");
  });
  it("leaves rgba(...) in canonical form", () => {
    expect(canonicaliseBrand("rgba(0, 0, 0, 0.08)")).toBe("rgba(0,0,0,0.08)");
  });
  it("throws on unknown --atomic-* refs with an actionable message", () => {
    expect(() => canonicaliseBrand("var(--atomic-nonexistent)")).toThrow(
      /unknown brand token --atomic-nonexistent/,
    );
  });
  it("passes through non-brand var() refs unchanged (e.g. var(--ac-bg))", () => {
    expect(canonicaliseBrand("var(--ac-bg)")).toBe("var(--ac-bg)");
  });
  it("detects cycles when BRAND_COLOR_TOKENS contains var() strings (defensive)", async () => {
    // Stubs the brand map with self-referential entries so the cycle guard
    // actually fires. Today BRAND_COLOR_TOKENS stores resolved hex (no
    // cycles possible) but the guard exists to defend against a future
    // refactor; this test pins that defence.
    vi.resetModules();
    vi.doMock("../tokens", () => ({
      BRAND_COLOR_TOKENS: {
        "atomic-a": "var(--atomic-b)",
        "atomic-b": "var(--atomic-a)",
      },
    }));
    const { canonicaliseBrand: cycling } = await import("./canonicaliseBrand");
    expect(() => cycling("var(--atomic-a)")).toThrow(/cycle detected/);
    vi.doUnmock("../tokens");
    vi.resetModules();
  });
});
