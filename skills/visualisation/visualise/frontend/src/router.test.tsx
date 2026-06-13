import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "./api/fetch";
import {
  buildRouter,
  setupRouterFixtures,
  waitForPath,
} from "./test/router-fixtures";

setupRouterFixtures();

function renderAt(url: string) {
  const { router, queryClient } = buildRouter(url);
  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

function stubKanbanConfigFetch(
  columns = [
    { key: "draft", label: "Draft" },
    { key: "ready", label: "Ready" },
    { key: "in-progress", label: "In progress" },
  ],
) {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string) => {
      if (url === "/api/kanban/config") {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ columns }),
        });
      }
      return Promise.reject(new Error(`unexpected fetch: ${url}`));
    }),
  );
}

describe("router", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("redirects / to /library (the overview hub)", async () => {
    const router = renderAt("/");
    await waitForPath(router, "/library");
  });

  it("serves the overview hub at bare /library", async () => {
    const router = renderAt("/library");
    await waitForPath(router, "/library");
  });

  it("routes /library/templates to the templates index", async () => {
    const router = renderAt("/library/templates");
    await waitForPath(router, "/library/templates");
    // Heading from LibraryTemplatesIndex — matched via the literal
    // /library/templates route, not the generic /library/$type.
    expect(
      await screen.findByRole("heading", { name: /^Templates$/i }),
    ).toBeInTheDocument();
  });

  it("routes /library/templates/adr to the templates detail view", async () => {
    const router = renderAt("/library/templates/adr");
    await waitForPath(router, "/library/templates/adr");
    // The detail route shows the same index page header plus an additional
    // "TIERS · adr.md" section heading for the selected template.
    expect(
      await screen.findByRole("heading", { name: /TIERS · adr\.md/i }),
    ).toBeInTheDocument();
  });

  it("redirects /library/bogus to /library when the type is unknown", async () => {
    // parseParams on libraryTypeRoute throws redirect({ to: '/library' })
    // for any string that is not a DocTypeKey; /library renders the hub.
    const router = renderAt("/library/bogus");
    await waitForPath(router, "/library");
  });

  it("renders the catch-all not-found surface for a truly-unmatched URL", async () => {
    renderAt("/garbage");
    // Catch-all H1, co-asserted with RootLayout's <main> so the test fails
    // concretely if the surface escapes the root shell rather than passing
    // against a partial tree.
    const h1 = await screen.findByRole("heading", {
      level: 1,
      name: /Page not found/i,
    });
    expect(h1).toBeInTheDocument();
    expect(screen.getByRole("main")).toBeInTheDocument();
    // Back-to-library present; no back-to-type; no eyebrow.
    expect(
      screen
        .getByRole("link", { name: /Back to library/i })
        .getAttribute("href"),
    ).toBe("/library");
    expect(screen.queryByRole("link", { name: /Back to .* list/i })).toBeNull();
    expect(document.querySelector('[data-slot="eyebrow"]')).toBeNull();
  });

  it("routes /lifecycle to the index view", async () => {
    vi.spyOn(fetchModule, "fetchLifecycleClusters").mockResolvedValue([]);
    const router = renderAt("/lifecycle");
    await waitForPath(router, "/lifecycle");
    expect(
      await screen.findByText(/no lifecycle clusters/i),
    ).toBeInTheDocument();
  });

  it("routes /lifecycle/foo to the cluster detail view", async () => {
    const spy = vi
      .spyOn(fetchModule, "fetchLifecycleCluster")
      .mockResolvedValue({
        slug: "foo",
        title: "Foo Cluster",
        entries: [],
        completeness: {
          hasWorkItem: false,
          hasResearch: false,
          hasPlan: false,
          hasPlanReview: false,
          hasValidation: false,
          hasPrDescription: false,
          hasPrReview: false,
          hasDecision: false,
          hasNotes: false,
          hasDesignInventory: false,
          hasDesignGap: false,
          present: [],
        },
        lastChangedMs: 0,
        clusterKey: null,
      });
    const router = renderAt("/lifecycle/foo");
    await waitForPath(router, "/lifecycle/foo");
    expect(
      await screen.findByRole("heading", { name: "Foo Cluster" }),
    ).toBeInTheDocument();
    expect(spy).toHaveBeenCalledWith("foo");
  });

  it("routes /kanban to the kanban board and renders configured columns", async () => {
    stubKanbanConfigFetch();
    vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);
    const router = renderAt("/kanban");
    await waitForPath(router, "/kanban");
    expect(
      await screen.findByRole("region", { name: /draft/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("region", { name: /ready/i })).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: /in progress/i }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: /other/i })).toBeNull();
  });

  it('does not render the legacy "coming in Phase 7" stub copy at /kanban', async () => {
    stubKanbanConfigFetch();
    vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);
    const router = renderAt("/kanban");
    await waitForPath(router, "/kanban");
    expect(screen.queryByText(/coming in phase 7/i)).toBeNull();
  });
});

describe("loader crumbs", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("/library/templates → Templates crumb with Library ancestor", async () => {
    const router = renderAt("/library/templates");
    await waitForPath(router, "/library/templates");
    const matches = router.state.matches;
    const templatesMatch = matches.find(
      (m) => m.routeId.includes("templates") && !m.routeId.includes("$name"),
    );
    const libraryMatch = matches.find((m) => m.routeId === "/library");
    expect((templatesMatch?.loaderData as any)?.crumb).toBe("Templates");
    expect((libraryMatch?.loaderData as any)?.crumb).toBe("Library");
  });

  it("/library/templates/adr → adr crumb", async () => {
    const router = renderAt("/library/templates/adr");
    await waitForPath(router, "/library/templates/adr");
    const matches = router.state.matches;
    const detailMatch = matches.find((m) => m.routeId.includes("$name"));
    expect((detailMatch?.loaderData as any)?.crumb).toBe("adr");
  });

  it("/library/decisions → decisions crumb with Library ancestor", async () => {
    vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);
    const router = renderAt("/library/decisions");
    await waitForPath(router, "/library/decisions");
    const matches = router.state.matches;
    const typeMatch = matches.find((m) => m.routeId.includes("$type"));
    const libraryMatch = matches.find((m) => m.routeId === "/library");
    expect((typeMatch?.loaderData as any)?.crumb).toBe("decisions");
    expect((libraryMatch?.loaderData as any)?.crumb).toBe("Library");
  });

  it("/library/decisions/some-slug → some-slug crumb with Library and decisions ancestors", async () => {
    vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([
      {
        slug: "some-slug",
        title: "Some",
        docType: "decisions",
        tags: [],
        lastModifiedMs: 0,
      } as any,
    ]);
    const router = renderAt("/library/decisions/some-slug");
    await waitForPath(router, "/library/decisions/some-slug");
    const matches = router.state.matches;
    const docMatch = matches.find((m) => m.routeId.includes("$fileSlug"));
    const typeMatch = matches.find((m) => m.routeId.includes("$type"));
    const libraryMatch = matches.find((m) => m.routeId === "/library");
    expect((docMatch?.loaderData as any)?.crumb).toBe("some-slug");
    expect((typeMatch?.loaderData as any)?.crumb).toBe("decisions");
    expect((libraryMatch?.loaderData as any)?.crumb).toBe("Library");
  });

  it("/lifecycle → Lifecycle crumb", async () => {
    vi.spyOn(fetchModule, "fetchLifecycleClusters").mockResolvedValue([]);
    const router = renderAt("/lifecycle");
    await waitForPath(router, "/lifecycle");
    const matches = router.state.matches;
    const lifecycleMatch = matches.find((m) => m.routeId === "/lifecycle");
    expect((lifecycleMatch?.loaderData as any)?.crumb).toBe("Lifecycle");
  });

  it("/lifecycle/some-cluster → some-cluster crumb with Lifecycle ancestor", async () => {
    vi.spyOn(fetchModule, "fetchLifecycleCluster").mockResolvedValue({
      slug: "some-cluster",
      title: "Some Cluster",
      entries: [],
      completeness: {
        hasWorkItem: false,
        hasResearch: false,
        hasPlan: false,
        hasPlanReview: false,
        hasValidation: false,
        hasPrDescription: false,
        hasPrReview: false,
        hasDecision: false,
        hasNotes: false,
        hasDesignInventory: false,
        hasDesignGap: false,
        present: [],
      },
      lastChangedMs: 0,
      clusterKey: null,
    });
    const router = renderAt("/lifecycle/some-cluster");
    await waitForPath(router, "/lifecycle/some-cluster");
    const matches = router.state.matches;
    const clusterMatch = matches.find((m) => m.routeId.includes("$slug"));
    const lifecycleMatch = matches.find((m) => m.routeId === "/lifecycle");
    expect((clusterMatch?.loaderData as any)?.crumb).toBe("some-cluster");
    expect((lifecycleMatch?.loaderData as any)?.crumb).toBe("Lifecycle");
  });

  it("/kanban → Kanban crumb", async () => {
    stubKanbanConfigFetch();
    vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);
    const router = renderAt("/kanban");
    await waitForPath(router, "/kanban");
    const matches = router.state.matches;
    const kanbanMatch = matches.find((m) => m.routeId === "/kanban");
    expect((kanbanMatch?.loaderData as any)?.crumb).toBe("Kanban");
  });

  it("routes /dev to the DevDesignSystem reference page", async () => {
    const router = renderAt("/dev");
    await waitForPath(router, "/dev");
    // The Overview section heading confirms DevDesignSystem rendered.
    expect(
      await screen.findByRole("heading", { name: /^Overview$/ }),
    ).toBeInTheDocument();
  });

  it("registers /dev as an uncrumbed route (no breadcrumb trail)", async () => {
    const router = renderAt("/dev");
    await waitForPath(router, "/dev");
    const devMatch = router.state.matches.find((m) => m.routeId === "/dev");
    // No `withCrumb` loader → no crumb → Breadcrumbs renders nothing.
    expect(
      (devMatch?.loaderData as { crumb?: string } | undefined)?.crumb,
    ).toBeUndefined();
    expect(
      screen.queryByRole("navigation", { name: /breadcrumb/i }),
    ).toBeNull();
  });

  it("no longer resolves the retired showcase paths to a showcase (0083)", async () => {
    // The five showcase routes were retired into /dev. A former path no longer
    // matches a showcase route — it falls through to the default SPA not-found
    // (no notFoundComponent, no redirect), so the path stays put and no match
    // carries it as a route id.
    for (const path of [
      "/glyph-showcase",
      "/big-glyph-showcase",
      "/chip-showcase",
      "/code-syntax-showcase",
      "/kanban-card-showcase",
    ]) {
      const router = renderAt(path);
      await waitForPath(router, path);
      expect(router.state.location.pathname).toBe(path);
      expect(router.state.matches.some((m) => m.routeId === path)).toBe(false);
    }
  });
});
