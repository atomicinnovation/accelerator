import { writeFileSync } from "node:fs";
import type { Locator } from "@playwright/test";
import { expect } from "./fixtures.js";

/** Saturated FSEvents has been seen to take minutes to register watches. */
export const WATCH_REGISTRATION_BUDGET_MS = 300_000;

/**
 * Rewrites `path` with `contents` until `observed` is visible. The server
 * answers HTTP before its filesystem watches are registered, so a single
 * write can land before anything is listening; identical rewrites after the
 * first observed one are suppressed server-side.
 */
export async function writeUntilVisible(
  path: string,
  contents: string,
  observed: Locator,
): Promise<void> {
  await expect(async () => {
    writeFileSync(path, contents);
    await expect(observed).toBeVisible({ timeout: 1_000 });
  }).toPass({ timeout: WATCH_REGISTRATION_BUDGET_MS, intervals: [250] });
}
