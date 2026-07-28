/**
 * C1 (keyboard cross-column move) and C3 (focus returns to the moved card's
 * anchor after the move settles). dnd-kit's sortableKeyboardCoordinates needs
 * real layout boxes jsdom does not provide, so these are E2E.
 *
 * The data path is mocked (see installMockBoard) so the move is deterministic
 * and isolated from on-disk fixture state; the real KeyboardSensor geometry and
 * the real DOM are still exercised in a real browser. The real-server write path
 * is covered by kanban.spec / kanban-conflict.
 */

import { dndDrag } from "./dnd.js";
import { expect, test } from "./fixtures.js";
import { installMockBoard } from "./mock-board.js";

const RELPATH = "meta/work/0001-keyboard.md";
const CARD = `li[data-relpath="${RELPATH}"]`;
const COLUMNS = [
  { key: "draft", label: "Draft" },
  { key: "in-progress", label: "In progress" },
  { key: "done", label: "Done" },
];
const CARD_FIXTURE = {
  relPath: RELPATH,
  workItemId: "0001",
  title: "Keyboard card",
  status: "draft",
};

test("keyboard: Space → arrows → Space completes a cross-column move (C1)", async ({
  page,
}) => {
  await installMockBoard(page, { columns: COLUMNS, cards: [CARD_FIXTURE] });
  await page.goto("/kanban");
  await expect(
    page.locator(`section[data-column="draft"] ${CARD}`),
  ).toBeVisible();

  // Focus the draggable's listeners element (the card anchor) and drive the
  // KeyboardSensor: Space to pick up, ArrowRight to cross to the next column,
  // Space to drop. Space (not Enter) avoids the anchor's native activation.
  // Assert focus landed and pace the presses so the first event is not dropped
  // before the sensor activates.
  const anchor = page.locator(`${CARD} a`);
  await anchor.focus();
  await expect(anchor).toBeFocused();
  await page.keyboard.press("Space");
  await page.waitForTimeout(150);
  await page.keyboard.press("ArrowRight");
  await page.waitForTimeout(150);
  await page.keyboard.press("Space");

  // The card left the draft column and persists in a new column.
  await expect(
    page.locator(`section[data-column="draft"] ${CARD}`),
  ).toHaveCount(0, {
    timeout: 5000,
  });
  await expect(page.locator(CARD)).toBeVisible();
});

test("focus returns to the moved card anchor in its resting column after settle (C3)", async ({
  page,
}) => {
  await installMockBoard(page, { columns: COLUMNS, cards: [CARD_FIXTURE] });
  await page.goto("/kanban");
  await expect(page.locator(CARD)).toBeVisible();

  await dndDrag(page, `${CARD} a`, 'section[data-column="in-progress"]');

  // After the onSettled invalidation resolves and the node remounts, focus is
  // restored to the card's <Link> anchor (relPath-keyed, so it resolves to the
  // live node in its resting column).
  await expect(
    page.locator(`section[data-column="in-progress"] ${CARD}`),
  ).toBeVisible({
    timeout: 5000,
  });
  await expect(page.locator(`${CARD} a`)).toBeFocused({ timeout: 5000 });
});
