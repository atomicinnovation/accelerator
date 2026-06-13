import { describe, expect, it } from "vitest";
import { errorMessage } from "./error-message";

describe("errorMessage", () => {
  it("returns the message of an Error", () => {
    expect(errorMessage(new Error("boom"))).toBe("boom");
  });

  it("returns a string value verbatim", () => {
    expect(errorMessage("boom")).toBe("boom");
  });

  it("returns a stable fallback for undefined and null without throwing", () => {
    expect(errorMessage(undefined)).toBe("An unknown error occurred.");
    expect(errorMessage(null)).toBe("An unknown error occurred.");
  });
});
