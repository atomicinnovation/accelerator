/**
 * A2: a drag must never trigger the card's <Link> navigation, while a genuine
 * click always navigates. jsdom cannot reproduce the PointerSensor 5px
 * activation or the synthetic post-drag click, so this is the authoritative
 * oracle for the click-vs-drag guard.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { startDrag } from "./dnd.js";
import { expect, test } from "./fixtures.js";

const WORK_ITEM_0001_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../server/tests/fixtures/meta/work/0001-first-work-item.md",
);
const CARD =
  'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"]';

test("dragging a card and releasing does NOT navigate", async ({ page }) => {
  // Restore the fixture even though this drag stays within the card's own column
  // (threshold move only, no status change) — a defensive guard so a geometry
  // change can never silently pollute later specs' starting state.
  const original = readFileSync(WORK_ITEM_0001_PATH, "utf-8");
  try {
    await page.goto("/kanban");
    await expect(page.locator(CARD)).toBeVisible();

    // Cross the 5px activation threshold and release in place — a real drag
    // whose synthetic post-drag click would, without the guard, follow the
    // anchor. Staying in the same column means no status change / file mutation.
    const drag = await startDrag(page, `${CARD} a`);
    await drag.drop();

    // Still on the board — navigation was suppressed.
    await expect(page).toHaveURL(/\/kanban$/);
    await expect(page.locator(CARD)).toBeVisible();
  } finally {
    writeFileSync(WORK_ITEM_0001_PATH, original);
  }
});

test("clicking a card (no drag) navigates to its library page", async ({
  page,
}) => {
  await page.goto("/kanban");
  await expect(page.locator(CARD)).toBeVisible();

  await page.locator(`${CARD} a`).click();

  await expect(page).toHaveURL(/\/library\/work-items\/0001-first-work-item$/);
});
