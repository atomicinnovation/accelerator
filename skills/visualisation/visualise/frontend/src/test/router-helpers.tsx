import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
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
  // Mirror the production `/library` overview route so `<Link to="/library">`
  // (the recovery surfaces' always-present back-link) resolves to a real href.
  const libraryRoute = createRoute({
    getParentRoute: () => root,
    path: "/library",
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
    libraryRoute,
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

/** Like `renderWithRouterAt`, but also wraps the tree in a
 *  `QueryClientProvider`. Surfaces that call a `useQuery`/`useQueries` hook
 *  *and* render `<Link>`s (e.g. the recovery surfaces) need both contexts; the
 *  router alone provides no query client, so the hook would throw. The client
 *  disables retries so rejected `fetchDocs` mocks settle immediately. */
export function renderWithRouterAndQueryAt(ui: React.ReactNode, atUrl = "/") {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = buildTestRouter(ui, atUrl);
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

/** Backwards-compatible wrapper used by the Sidebar test suite. */
export function MemoryRouter({ children }: { children: React.ReactNode }) {
  return <RouterProvider router={buildTestRouter(children)} />;
}
