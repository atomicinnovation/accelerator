import { expect, test } from "@playwright/test";
import { resolveToken, setTheme } from "../lib/expected-colours";

test.use({ viewport: { width: 1280, height: 720 } });

const ROOT = '[class*="markdown"]';
// First task list = the tight one appended for this spec; scoping to it keeps
// the checked/unchecked cardinality at exactly one each.
const TIGHT = `${ROOT} [class*="tasklist"] >> nth=0`;
const px = (s: string) => parseFloat(s); // tolerant numeric compare for sub-px widths

for (const theme of ["light", "dark"] as const) {
  test(`task-list boxes are token-driven (${theme})`, async ({ page }) => {
    await page.goto("/library/plans/first-plan");
    if (theme === "dark") await setTheme(page, "dark");
    const list = page.locator(TIGHT);
    const checkedBox = list.locator('[role="checkbox"][aria-checked="true"]');
    const uncheckedBox = list.locator(
      '[role="checkbox"][aria-checked="false"]',
    );
    const doneLabel = list.locator('[class*="taskDone"] [class*="taskLabel"]');
    const todoLabel = list.locator(
      'li:not([class*="taskDone"]) [class*="taskLabel"]',
    );

    // AC1 / requirement: no native control survives, anywhere (tight + loose).
    await expect(page.locator(`${ROOT} input[type="checkbox"]`)).toHaveCount(0);
    // Cardinality guards so a structural failure reports behaviourally, not as a
    // generic locator timeout, and a duplicate checked box can't pass silently.
    await expect(checkedBox).toHaveCount(1);
    await expect(uncheckedBox).toHaveCount(1);
    await expect(doneLabel).toHaveCount(1);
    await expect(todoLabel).toHaveCount(1);

    // AC3: the task list shows no list-item marker (real-cascade, not text).
    const listStyle = await list.evaluate(
      (n) => getComputedStyle(n).listStyleType,
    );
    expect(listStyle).toBe("none");

    // Unchecked box (AC1): --ac-stroke-strong border on --ac-bg-card, ~1.5px,
    // 4px corner (var(--radius-4)), 17px.
    const u = await uncheckedBox.evaluate((n) => {
      const c = getComputedStyle(n);
      return {
        borderTopColor: c.borderTopColor,
        backgroundColor: c.backgroundColor,
        borderTopWidth: c.borderTopWidth,
        borderTopLeftRadius: c.borderTopLeftRadius,
        width: c.width,
        height: c.height,
      };
    });
    expect(u.borderTopColor).toBe(
      await resolveToken(page, "--ac-stroke-strong"),
    );
    expect(u.backgroundColor).toBe(await resolveToken(page, "--ac-bg-card"));
    // Hairline border: the CSS declares 1.5px (pinned verbatim by the
    // migration CSS-as-text guard), but Chromium reports border-width's
    // computed value rounded to an integer CSS pixel, so the real cascade
    // yields "1px". Assert a non-zero hairline no wider than the declared 1.5.
    expect(px(u.borderTopWidth)).toBeGreaterThanOrEqual(1);
    expect(px(u.borderTopWidth)).toBeLessThanOrEqual(1.5);
    expect(u.borderTopLeftRadius).toBe("4px"); // var(--radius-4)
    expect(u.width).toBe("17px");
    expect(u.height).toBe("17px");

    // Checked box (AC2, softened): fill + border --ac-accent, white tick present.
    const c = await checkedBox.evaluate((n) => {
      const s = getComputedStyle(n);
      return {
        backgroundColor: s.backgroundColor,
        borderTopColor: s.borderTopColor,
      };
    });
    const accent = await resolveToken(page, "--ac-accent");
    expect(c.backgroundColor).toBe(accent);
    expect(c.borderTopColor).toBe(accent);
    // Redundant-cue guarantee the softened AC2 relies on: checked fill differs
    // from the unchecked fill, so state is conveyed independent of tick contrast.
    expect(c.backgroundColor).not.toBe(u.backgroundColor);
    const tickColor = await checkedBox
      .locator("svg")
      .evaluate((n) => getComputedStyle(n).color);
    expect(tickColor).toBe("rgb(255, 255, 255)"); // #ffffff tick; 3:1 is a manual note (dark ≈ 2.9:1)

    // AC4 done half: muted + line-through.
    const d = await doneLabel.evaluate((n) => {
      const s = getComputedStyle(n);
      return { color: s.color, line: s.textDecorationLine };
    });
    expect(d.color).toBe(await resolveToken(page, "--ac-fg-muted"));
    expect(d.line).toContain("line-through");

    // AC4 not-done half: normal body text — positively equals the prose body
    // colour (--ac-fg), not merely "not muted".
    const t = await todoLabel.evaluate((n) => {
      const s = getComputedStyle(n);
      return { color: s.color, line: s.textDecorationLine };
    });
    expect(t.line).not.toContain("line-through");
    expect(t.color).toBe(await resolveToken(page, "--ac-fg"));
  });
}

// AC1/AC2/AC4 divergence: box + label colours must actually change between
// themes, not merely resolve to a token value.
test("task-list colours diverge between light and dark", async ({ page }) => {
  await page.goto("/library/plans/first-plan");
  const lightStroke = await resolveToken(page, "--ac-stroke-strong");
  const lightAccent = await resolveToken(page, "--ac-accent");
  await setTheme(page, "dark");
  expect(await resolveToken(page, "--ac-stroke-strong")).not.toBe(lightStroke);
  expect(await resolveToken(page, "--ac-accent")).not.toBe(lightAccent);
});
