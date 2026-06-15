import { expect, test } from "@playwright/test";

test.use({ viewport: { width: 1280, height: 720 } });

// L1: detail-page action buttons ("Open in editor", "Copy path") must not wrap
// their labels, and the actions row must not be compressed by a long title.
// Asserting the computed `white-space: nowrap` (button) and `flex-shrink: 0`
// (actions row) directly guards the fix deterministically — independent of how
// long the rendered fixture title happens to be, which a pixel diff would
// depend on.
test("L1: action buttons don't wrap and the actions row doesn't shrink", async ({
  page,
}) => {
  await page.goto("/library/plans/first-plan");

  const actions = page.locator('[data-slot="actions"]');
  await actions.waitFor();
  const actionsShrink = await actions.evaluate(
    (n) => getComputedStyle(n).flexShrink,
  );
  expect(actionsShrink).toBe("0");

  const button = actions.locator("button").first();
  await button.waitFor();
  const whiteSpace = await button.evaluate(
    (n) => getComputedStyle(n).whiteSpace,
  );
  expect(whiteSpace).toBe("nowrap");
});
