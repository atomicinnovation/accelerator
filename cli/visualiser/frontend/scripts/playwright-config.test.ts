import os from "node:os";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@playwright/test", () => ({ defineConfig: <T>(c: T) => c }));

async function loadConfig(ci: string | undefined) {
  vi.resetModules();
  vi.stubEnv("CI", ci);
  return (await import("../playwright.config")).default;
}

function projectWorkers(
  config: Awaited<ReturnType<typeof loadConfig>>,
  name: string,
) {
  return config.projects?.find((project) => project.name === name)?.workers;
}

beforeEach(() => {
  vi.spyOn(os, "availableParallelism").mockReturnValue(4);
});

afterEach(() => vi.unstubAllEnvs());

describe.each([undefined, "true"])("with CI=%s", (ci) => {
  it("never retries a failing test", async () => {
    expect((await loadConfig(ci)).retries).toBe(0);
  });

  it("keeps the trace of a failing test", async () => {
    expect((await loadConfig(ci)).use?.trace).toBe("retain-on-failure");
  });

  describe("on a host with 4 logical CPUs", () => {
    it("caps the run at 3 workers", async () => {
      expect((await loadConfig(ci)).workers).toBe(3);
    });

    it("gives resolved-styles 2 workers", async () => {
      expect(projectWorkers(await loadConfig(ci), "resolved-styles")).toBe(2);
    });

    it("gives chromium 1 worker", async () => {
      expect(projectWorkers(await loadConfig(ci), "chromium")).toBe(1);
    });
  });
});
