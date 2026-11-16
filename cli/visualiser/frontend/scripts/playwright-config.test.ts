import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@playwright/test", () => ({ defineConfig: <T>(c: T) => c }));

async function loadConfig(ci: string | undefined) {
  vi.resetModules();
  vi.stubEnv("CI", ci);
  return (await import("../playwright.config")).default;
}

afterEach(() => vi.unstubAllEnvs());

describe.each([undefined, "true"])("with CI=%s", (ci) => {
  it("never retries a failing test", async () => {
    expect((await loadConfig(ci)).retries).toBe(0);
  });

  it("keeps the trace of a failing test", async () => {
    expect((await loadConfig(ci)).use?.trace).toBe("retain-on-failure");
  });
});
