import { describe, expect, it } from "vitest";
import { formatDocId } from "./doc-type-id";

describe("formatDocId", () => {
  it("passes a 4-digit id through unchanged", () => {
    expect(formatDocId("PROJ-0001")).toBe("PROJ-0001");
  });

  it("zero-pads a short numeric id to 4 digits", () => {
    expect(formatDocId("PROJ-1")).toBe("PROJ-0001");
  });

  it("passes longer numeric ids through without truncation", () => {
    expect(formatDocId("PROJ-12345")).toBe("PROJ-12345");
  });

  it("returns empty string for null and undefined", () => {
    expect(formatDocId(null)).toBe("");
    expect(formatDocId(undefined)).toBe("");
  });

  it("returns the original string for malformed prefixes", () => {
    expect(formatDocId("NOTPREFIXED")).toBe("NOTPREFIXED");
  });

  it("accepts alphanumeric prefixes and preserves case", () => {
    expect(formatDocId("proj1-0001")).toBe("proj1-0001");
  });
});
