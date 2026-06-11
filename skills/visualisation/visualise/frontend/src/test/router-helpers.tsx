import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { render } from "@testing-library/react";
import type React from "react";

function buildTestRouter(ui: React.ReactNode, atUrl = "/") {
  const root = createRootRoute({ component: () => <Outlet /> });
  const indexRoute = createRoute({
    getParentRoute: () => root,
    path: "/",
    component: () => <>{ui}</>,
  });
  const libraryTypeRoute = createRoute({
    getParentRoute: () => root,
    path: "/library/$type",
    component: () => <>{ui}</>,
  });
  const libraryDocRoute = createRoute({
    getParentRoute: () => root,
    path: "/library/$type/$fileSlug",
    component: () => <>{ui}</>,
  });
  // Mirror the production `/lifecycle/$slug` route so `<Link to="/lifecycle/$slug">`
  // resolves to the expected href in tests (e.g. the Cluster block).
  const lifecycleClusterRoute = createRoute({
    getParentRoute: () => root,
    path: "/lifecycle/$slug",
    component: () => <>{ui}</>,
  });
  const tree = root.addChildren([
    indexRoute,
    libraryTypeRoute,
    libraryDocRoute,
    lifecycleClusterRoute,
  ]);
  return createRouter({
    routeTree: tree,
    history: createMemoryHistory({ initialEntries: [atUrl] }),
  });
}

export function renderWithRouterAt(ui: React.ReactNode, atUrl = "/") {
  const router = buildTestRouter(ui, atUrl);
  return render(<RouterProvider router={router} />);
}

/** Backwards-compatible wrapper used by the Sidebar test suite. */
export function MemoryRouter({ children }: { children: React.ReactNode }) {
  return <RouterProvider router={buildTestRouter(children)} />;
}
