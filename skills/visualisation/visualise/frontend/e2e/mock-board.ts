import type { Page } from "@playwright/test";

export interface MockColumn {
  key: string;
  label: string;
}

export interface MockCard {
  relPath: string;
  workItemId: string;
  title: string;
  status: string;
}

export interface MockBoardOptions {
  columns: MockColumn[];
  cards: MockCard[];
}

/**
 * Install stateful mocks for the kanban data path — `/api/kanban/config`,
 * `/api/docs?type=work-items`, and `PATCH …/frontmatter` — so drag and keyboard
 * moves persist purely in-memory. This fully isolates a spec from the shared
 * on-disk fixtures and the server's file-watcher latency (the source of the
 * cross-test races that plague real-file drag specs), giving deterministic
 * cross-config / keyboard coverage. The real-server write path stays covered by
 * kanban.spec / kanban-conflict / project-pattern specs.
 */
export async function installMockBoard(
  page: Page,
  opts: MockBoardOptions,
): Promise<void> {
  const status = new Map(opts.cards.map((c) => [c.relPath, c.status]));
  // Bump on each PATCH so a post-settle refetch differs from the optimistic
  // state (a real server returns a fresh ETag/mtime after a write). Without this
  // React Query's structural sharing keeps the entries array referentially
  // stable and the board's focus-return effect would never fire.
  const mtime = new Map(opts.cards.map((c) => [c.relPath, 0]));
  let tick = 0;

  await page.route("**/api/kanban/config", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ columns: opts.columns }),
    }),
  );

  await page.route(
    (url) =>
      url.pathname.endsWith("/api/docs") &&
      url.searchParams.get("type") === "work-items",
    (route) =>
      route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          docs: opts.cards.map((c) => ({
            type: "work-items",
            path: `/fixtures/${c.relPath}`,
            relPath: c.relPath,
            slug: c.relPath.split("/").pop()!.replace(/\.md$/, ""),
            workItemId: c.workItemId,
            title: c.title,
            frontmatter: { status: status.get(c.relPath) },
            frontmatterState: "parsed",
            workItemRefs: [],
            mtimeMs: mtime.get(c.relPath),
            size: 100,
            etag: `e2e-mock-${mtime.get(c.relPath)}`,
            bodyPreview: "",
          })),
        }),
      }),
  );

  await page.route("**/api/docs/**/frontmatter", async (route) => {
    if (route.request().method() !== "PATCH") {
      await route.continue();
      return;
    }
    const { pathname } = new URL(route.request().url());
    const encoded = pathname
      .replace(/^.*\/api\/docs\//, "")
      .replace(/\/frontmatter$/, "");
    const relPath = encoded.split("/").map(decodeURIComponent).join("/");
    const body = JSON.parse(route.request().postData() ?? "{}");
    const next = body?.patch?.status;
    if (typeof next === "string") status.set(relPath, next);
    mtime.set(relPath, ++tick);
    // Mirror the server's success shape: 200 + a fresh ETag header, empty body.
    await route.fulfill({
      status: 200,
      headers: { etag: `"e2e-mock-${tick}"` },
      body: "",
    });
  });
}
