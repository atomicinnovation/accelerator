import {
  type AnyRoute,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";

export type CrumbLoaderData = { crumb: string };

type CrumbResolver = (args: { params: Record<string, string> }) => string;

export function resolveCrumb(
  crumbOrResolver: string | CrumbResolver,
  params: Record<string, string>,
): CrumbLoaderData {
  return {
    crumb:
      typeof crumbOrResolver === "string"
        ? crumbOrResolver
        : crumbOrResolver({ params }),
  };
}

// Wraps createRoute with a breadcrumb loader. Generic over TParentRoute and
// TPath so the returned Route carries the correct path literal type — required
// for TanStack Router's module-level type registry to include these routes.
export function withCrumb<TParentRoute extends AnyRoute, TPath extends string>(
  crumbOrResolver: string | CrumbResolver,
  options: { getParentRoute: () => TParentRoute; path: TPath } & Record<
    string,
    unknown
  >,
): ReturnType<typeof createRoute<unknown, TParentRoute, TPath>> {
  const route = createRoute({
    ...options,
    loader: ({ params }: { params: Record<string, string> }) =>
      resolveCrumb(crumbOrResolver, params),
  } as unknown as Parameters<typeof createRoute>[0]);
  // Double-cast through `unknown` bridges TanStack Router's over-constrained
  // createRoute generics, which the structural value cannot satisfy directly.
  return route as unknown as ReturnType<
    typeof createRoute<unknown, TParentRoute, TPath>
  >;
}

import { type DocTypeKey, isDocTypeKey } from "./api/types";
import { DevDesignSystem } from "./components/DevDesignSystem/DevDesignSystem";
import { RootLayout } from "./components/RootLayout/RootLayout";
import { KanbanBoard } from "./routes/kanban/KanbanBoard";
import { LibraryDocView } from "./routes/library/LibraryDocView";
import { LibraryLayout } from "./routes/library/LibraryLayout";
import { LibraryOverviewHub } from "./routes/library/LibraryOverviewHub";
import { LibraryTemplatesIndex } from "./routes/library/LibraryTemplatesIndex";
import { LibraryTemplatesView } from "./routes/library/LibraryTemplatesView";
import { LibraryTypeView } from "./routes/library/LibraryTypeView";
import { CatchAllNotFound } from "./routes/library/recovery/CatchAllNotFound";
import { LifecycleClusterView } from "./routes/lifecycle/LifecycleClusterView";
import { LifecycleIndex } from "./routes/lifecycle/LifecycleIndex";
import { LifecycleLayout } from "./routes/lifecycle/LifecycleLayout";

const rootRoute = createRootRoute({ component: RootLayout });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/library" });
  },
});

const libraryRoute = withCrumb("Library", {
  getParentRoute: () => rootRoute,
  path: "/library",
  component: LibraryLayout,
});

// Landing at /library renders the overview hub.
const libraryIndexRoute = createRoute({
  getParentRoute: () => libraryRoute,
  path: "/",
  component: LibraryOverviewHub,
});

// Dedicated Templates routes — literal paths beat the `/$type` param
// route below, so these are dispatched directly by the router rather
// than via a runtime `if (type === 'templates')` branch inside the
// generic views.
const libraryTemplatesIndexRoute = withCrumb("Templates", {
  getParentRoute: () => libraryRoute,
  path: "/templates",
  component: LibraryTemplatesIndex,
});

const libraryTemplateDetailRoute = withCrumb(({ params }) => params.name, {
  getParentRoute: () => libraryRoute,
  path: "/templates/$name",
  component: LibraryTemplatesView,
});

// `parseParams` narrows `type: string` → `type: DocTypeKey` at the router
// boundary. An unknown type in the URL redirects to /library rather than
// rendering a silently-wrong view.
const libraryTypeRoute = withCrumb(({ params }) => params.type, {
  getParentRoute: () => libraryRoute,
  path: "/$type",
  parseParams: (raw: Record<string, string>): { type: DocTypeKey } => {
    if (!isDocTypeKey(raw.type)) {
      throw redirect({ to: "/library" });
    }
    return { type: raw.type };
  },
  component: LibraryTypeView,
});

const libraryDocRoute = withCrumb(({ params }) => params.fileSlug, {
  getParentRoute: () => libraryTypeRoute,
  path: "/$fileSlug",
  component: LibraryDocView,
});

const lifecycleRoute = withCrumb("Lifecycle", {
  getParentRoute: () => rootRoute,
  path: "/lifecycle",
  component: LifecycleLayout,
});

const lifecycleIndexRoute = createRoute({
  getParentRoute: () => lifecycleRoute,
  path: "/",
  component: LifecycleIndex,
});

export const lifecycleClusterRoute = withCrumb(({ params }) => params.slug, {
  getParentRoute: () => lifecycleRoute,
  path: "/$slug",
  component: LifecycleClusterView,
});

const kanbanRoute = withCrumb("Kanban", {
  getParentRoute: () => rootRoute,
  path: "/kanban",
  component: KanbanBoard,
});

// DevDesignSystem — uncrumbed `/dev` reference page (story 0083). Reached via
// the `#dev` activation aliases (normalised by the hash bridge), the
// Cmd/Ctrl+Shift+L chord, or a sidebar-foot triple-click. Consolidates and
// replaces the five throwaway showcase routes below (retired in a later phase).
const devRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/dev",
  component: DevDesignSystem,
});

// The five developer-only showcase routes (/glyph-showcase, /big-glyph-showcase,
// /chip-showcase, /code-syntax-showcase, /kanban-card-showcase) were retired in
// story 0083: their primitives and visual-regression coverage are consolidated
// into the DevDesignSystem `/dev` reference page (the VR specs are repointed at
// /dev#<section>). A retired path no longer resolves to a showcase — it falls
// through to TanStack's default SPA not-found UI (served over HTTP 200; the app
// has no notFoundComponent and adds no redirect).

// Exported so tests can construct an isolated router with memory history.
export const routeTree = rootRoute.addChildren([
  indexRoute,
  libraryRoute.addChildren([
    libraryIndexRoute,
    // Dedicated Templates routes registered before the generic $type route;
    // literal-path specificity means the router matches these first.
    libraryTemplatesIndexRoute,
    libraryTemplateDetailRoute,
    libraryTypeRoute.addChildren([libraryDocRoute]),
  ]),
  lifecycleRoute.addChildren([lifecycleIndexRoute, lifecycleClusterRoute]),
  kanbanRoute,
  devRoute,
]);

// Shared so the `buildRouter` test fixture (router-fixtures.ts) constructs a
// router carrying the same catch-all the app does — a `defaultNotFoundComponent`
// set only on this instance would never be exercised by the fixture.
export const routerOptions = {
  routeTree,
  defaultNotFoundComponent: CatchAllNotFound,
};

export const router = createRouter(routerOptions);

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
