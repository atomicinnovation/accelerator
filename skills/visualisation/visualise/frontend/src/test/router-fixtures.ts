import { QueryClient } from "@tanstack/react-query";
import { createMemoryHistory, createRouter } from "@tanstack/react-router";
import { waitFor } from "@testing-library/react";
import { beforeEach, expect, vi } from "vitest";
import * as fetchModule from "../api/fetch";
import { routeTree } from "../router";

export function setupRouterFixtures() {
  beforeEach(() => {
    vi.spyOn(fetchModule, "fetchTypes").mockResolvedValue([]);
    vi.spyOn(fetchModule, "fetchLibraryStructure").mockResolvedValue({
      phases: [],
      templates: {
        id: "templates",
        label: "Templates",
        count: 0,
        filteredCount: 0,
        latest: null,
        filterFacets: [],
      },
    });
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: [],
    });
    vi.spyOn(fetchModule, "fetchTemplateDetail").mockResolvedValue({
      name: "adr",
      activeTier: "plugin-default",
      tiers: [],
    });
  });
}

export function buildRouter(url: string) {
  return {
    router: createRouter({
      routeTree,
      history: createMemoryHistory({ initialEntries: [url] }),
    }),
    queryClient: new QueryClient(),
  };
}

export async function waitForPath(
  router: { state: { location: { pathname: string } } },
  expected: string,
): Promise<void> {
  await waitFor(() => {
    expect(router.state.location.pathname).toBe(expected);
  });
}
