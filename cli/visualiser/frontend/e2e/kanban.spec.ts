import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dndDrag } from "./dnd.js";
import { expect, test } from "./fixtures.js";

const FIXTURES_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../server/tests/fixtures/meta/work",
);
const WORK_ITEM_0001_PATH = resolve(FIXTURES_DIR, "0001-first-work-item.md");
const WORK_ITEM_0005_PATH = resolve(FIXTURES_DIR, "0005-sse-test-work-item.md");

test("drag todo card to in-progress column", async ({ page }) => {
  const original = readFileSync(WORK_ITEM_0001_PATH, "utf-8");
  try {
    await page.goto("/kanban");

    const todoCard = page.locator(
      'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"]',
    );
    const inProgressColumn = page.locator('section[data-column="in-progress"]');

    await expect(todoCard).toBeVisible();

    // Arm before the drag so we don't race the response.
    const patchDone = page.waitForResponse(
      (r) => r.url().includes("/api/docs/") && r.request().method() === "PATCH",
    );

    await dndDrag(
      page,
      'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"] a',
      'section[data-column="in-progress"]',
    );

    // Card should now appear in the in-progress column (optimistic update).
    await expect(
      inProgressColumn.locator(
        'li[data-relpath="tests/fixtures/meta/work/0001-first-work-item.md"]',
      ),
    ).toBeVisible({ timeout: 5000 });

    // A success toast (plain copy, polite region) confirms the move; the card
    // stays put in the target column.
    await expect(
      page
        .getByTestId("toaster-region-polite")
        .getByText(/moved to In progress/i),
    ).toBeVisible({ timeout: 5000 });

    // Wait for the server to finish writing before finally restores the file.
    await patchDone;
  } finally {
    writeFileSync(WORK_ITEM_0001_PATH, original);
  }
});

test("second tab receives SSE update after drag", async ({ page, context }) => {
  test.setTimeout(60_000);
  const original = readFileSync(WORK_ITEM_0005_PATH, "utf-8");
  try {
    const page2 = await context.newPage();
    await page.goto("/kanban");
    await page2.goto("/kanban");

    // Wait until both pages have established their SSE connections so neither
    // misses the doc-changed broadcast.
    await expect(page.locator('[data-sse-state="open"]')).toBeVisible({
      timeout: 5000,
    });
    await expect(page2.locator('[data-sse-state="open"]')).toBeVisible({
      timeout: 5000,
    });

    // Arm the PATCH response interceptor before the drag so we don't race.
    const patchDone = page.waitForResponse(
      (r) => r.url().includes("/api/docs/") && r.request().method() === "PATCH",
    );

    await dndDrag(
      page,
      'li[data-relpath="tests/fixtures/meta/work/0005-sse-test-work-item.md"] a',
      'section[data-column="in-progress"]',
    );

    // Confirm optimistic update on page 1.
    await expect(
      page.locator(
        'section[data-column="in-progress"] li[data-relpath="tests/fixtures/meta/work/0005-sse-test-work-item.md"]',
      ),
    ).toBeVisible({ timeout: 5000 });

    await patchDone;

    // Bring page 2 to the front before asserting to avoid background-tab throttling.
    await page2.bringToFront();

    // Second tab should receive the SSE event and move the card.
    await expect(
      page2.locator(
        'section[data-column="in-progress"] li[data-relpath="tests/fixtures/meta/work/0005-sse-test-work-item.md"]',
      ),
    ).toBeVisible({ timeout: 10000 });
  } finally {
    writeFileSync(WORK_ITEM_0005_PATH, original);
  }
});
