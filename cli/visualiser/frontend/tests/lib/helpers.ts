import type { Page } from "@playwright/test";

// Relative timestamps ("57s ago", "2m ago") change between baseline
// capture and test runs. Mask any <span> whose text ends with " ago"
// so pixel differences in card headers / article chrome don't cause
// spurious failures.
export const relativeTimeMask = (page: Page) =>
  page.locator("span").filter({ hasText: / ago$/ });

// Apply a theme by mutating `data-theme` on <html>, then wait for a
// rAF so the browser commits the style recalculation before the next
// observable read or screenshot. Light is the default theme and
// requires no mutation.
export const applyTheme = async (page: Page, theme: "light" | "dark") => {
  if (theme === "light") return;
  await page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        document.documentElement.dataset.theme = "dark";
        requestAnimationFrame(() => resolve());
      }),
  );
};
