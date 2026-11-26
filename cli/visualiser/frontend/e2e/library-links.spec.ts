import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures.js";
import { FIXTURE_META_DIR } from "./global-setup.js";

const RESEARCH_SETS = [
  "ac2-coverage",
  "mcp-transport-layer",
  "model-context-protocol",
  "prompt-caching-economics",
];

test("the topic-research listing links each set to its own id", async ({
  page,
}) => {
  await page.goto("/library/topic-research");
  const listing = page.locator('[role="table"]');
  await expect(listing).toBeVisible();

  for (const set of RESEARCH_SETS) {
    await expect(
      listing.locator(`a[href="/library/topic-research/${set}"]`),
    ).toHaveCount(1);
  }

  await listing
    .locator('a[href="/library/topic-research/mcp-transport-layer"]')
    .click();
  await expect(page).toHaveURL(
    /\/library\/topic-research\/mcp-transport-layer$/,
  );
  await expect(
    page
      .getByRole("heading", { level: 1, name: "MCP Transport Layer" })
      .first(),
  ).toBeVisible();
});

test("a design-inventory search result links to the inventory's id", async ({
  page,
}) => {
  await page.goto("/library");
  await page.getByRole("searchbox", { name: "Search" }).fill("example");

  const results = page.getByRole("region", { name: "Search results" });
  await expect(
    results.getByRole("option").filter({ hasText: "Example Design Inventory" }),
  ).toHaveAttribute("href", "/library/design-inventories/2026-05-26-example");
});

test("plan reviews sharing a slug each link to their own filename stem", async ({
  page,
}) => {
  await page.goto("/library/plan-reviews");
  const listing = page.locator('[role="table"]');
  await expect(listing).toBeVisible();

  for (const n of [1, 2]) {
    await expect(
      listing.locator(
        `a[href="/library/plan-reviews/2026-01-01-first-plan-review-${n}"]`,
      ),
    ).toHaveCount(1);
  }

  await listing
    .locator('a[href="/library/plan-reviews/2026-01-01-first-plan-review-2"]')
    .click();
  await expect(page.locator("article")).toContainText(
    "Second review of the first plan.",
  );
});

interface EditedDocument {
  file: string;
  href: string;
  title: string;
}

async function editEachAndFollowItsActivityRow(
  page: Page,
  documents: EditedDocument[],
): Promise<void> {
  const originals = documents.map((d) =>
    readFileSync(resolve(FIXTURE_META_DIR, d.file), "utf-8"),
  );
  try {
    await page.goto("/library");
    await expect(page.getByTestId("activity-live-badge")).toBeVisible();
    const activity = page.locator(
      'section[aria-labelledby="activity-heading"]',
    );

    for (const edited of documents) {
      appendFileSync(
        resolve(FIXTURE_META_DIR, edited.file),
        "\nEdited by the library-links e2e spec.\n",
      );
      await expect(
        activity.locator(`a[href="${edited.href}"]`).first(),
      ).toBeVisible();
    }

    for (const edited of documents) {
      await activity.locator(`a[href="${edited.href}"]`).first().click();
      await expect(page).toHaveURL(new RegExp(`${edited.href}$`));
      await expect(
        page.getByRole("heading", { level: 1, name: edited.title }).first(),
      ).toBeVisible();
    }
  } finally {
    documents.forEach((d, i) => {
      writeFileSync(resolve(FIXTURE_META_DIR, d.file), originals[i]);
    });
  }
}

test("activity rows for edited research sets each open their own set", async ({
  page,
}) => {
  await editEachAndFollowItsActivityRow(page, [
    {
      file: "research/topics/mcp-transport-layer/manifest.md",
      href: "/library/topic-research/mcp-transport-layer",
      title: "MCP Transport Layer",
    },
    {
      file: "research/topics/model-context-protocol/manifest.md",
      href: "/library/topic-research/model-context-protocol",
      title: "Model Context Protocol",
    },
  ]);
});

test("activity rows for edited design inventories each open their own inventory", async ({
  page,
}) => {
  await editEachAndFollowItsActivityRow(page, [
    {
      file: "research/design-inventories/2026-05-26-ac2-coverage/inventory.md",
      href: "/library/design-inventories/2026-05-26-ac2-coverage",
      title: "AC2 Coverage (design inventories)",
    },
    {
      file: "research/design-inventories/2026-05-26-example/inventory.md",
      href: "/library/design-inventories/2026-05-26-example",
      title: "Example Design Inventory",
    },
  ]);
});
