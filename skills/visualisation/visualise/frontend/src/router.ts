import {
  createRouter,
  createRoute,
  createRootRoute,
  redirect,
} from '@tanstack/react-router'

export type CrumbLoaderData = { crumb: string }

type CrumbResolver = (args: { params: Record<string, string> }) => string

export function resolveCrumb(
  crumbOrResolver: string | CrumbResolver,
  params: Record<string, string>,
): CrumbLoaderData {
  return {
    crumb:
      typeof crumbOrResolver === 'string'
        ? crumbOrResolver
        : crumbOrResolver({ params }),
  }
}

export function withCrumb(
  crumbOrResolver: string | CrumbResolver,
  options: Omit<Parameters<typeof createRoute>[0], 'loader'>,
) {
  return createRoute({
    ...options,
    loader: ({ params }) =>
      resolveCrumb(crumbOrResolver, params as Record<string, string>),
  })
}
import { RootLayout } from './components/RootLayout/RootLayout'
import { LibraryLayout } from './routes/library/LibraryLayout'
import { LibraryTypeView } from './routes/library/LibraryTypeView'
import { LibraryDocView } from './routes/library/LibraryDocView'
import { LibraryTemplatesIndex } from './routes/library/LibraryTemplatesIndex'
import { LibraryTemplatesView } from './routes/library/LibraryTemplatesView'
import { LifecycleLayout } from './routes/lifecycle/LifecycleLayout'
import { LifecycleIndex } from './routes/lifecycle/LifecycleIndex'
import { LifecycleClusterView } from './routes/lifecycle/LifecycleClusterView'
import { KanbanBoard } from './routes/kanban/KanbanBoard'
import { isDocTypeKey, type DocTypeKey } from './api/types'

const rootRoute = createRootRoute({ component: RootLayout })

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  beforeLoad: () => { throw redirect({ to: '/library' }) },
})

const libraryRoute = withCrumb('Library', {
  getParentRoute: () => rootRoute,
  path: '/library',
  component: LibraryLayout,
})

// Landing at /library redirects to the Decisions index so users see
// content rather than an empty main pane.
const libraryIndexRoute = createRoute({
  getParentRoute: () => libraryRoute,
  path: '/',
  beforeLoad: () => {
    throw redirect({ to: '/library/$type', params: { type: 'decisions' } })
  },
})

// Dedicated Templates routes — literal paths beat the `/$type` param
// route below, so these are dispatched directly by the router rather
// than via a runtime `if (type === 'templates')` branch inside the
// generic views.
const libraryTemplatesIndexRoute = withCrumb('Templates', {
  getParentRoute: () => libraryRoute,
  path: '/templates',
  component: LibraryTemplatesIndex,
})

const libraryTemplateDetailRoute = withCrumb(({ params }) => params.name, {
  getParentRoute: () => libraryRoute,
  path: '/templates/$name',
  component: LibraryTemplatesView,
})

// `parseParams` narrows `type: string` → `type: DocTypeKey` at the router
// boundary. An unknown type in the URL redirects to /library rather than
// rendering a silently-wrong view.
const libraryTypeRoute = withCrumb(({ params }) => params.type, {
  getParentRoute: () => libraryRoute,
  path: '/$type',
  parseParams: (raw: Record<string, string>): { type: DocTypeKey } => {
    if (!isDocTypeKey(raw.type)) {
      throw redirect({ to: '/library' })
    }
    return { type: raw.type }
  },
  component: LibraryTypeView,
})

const libraryDocRoute = withCrumb(({ params }) => params.fileSlug, {
  getParentRoute: () => libraryTypeRoute,
  path: '/$fileSlug',
  component: LibraryDocView,
})

const lifecycleRoute = withCrumb('Lifecycle', {
  getParentRoute: () => rootRoute,
  path: '/lifecycle',
  component: LifecycleLayout,
})

const lifecycleIndexRoute = createRoute({
  getParentRoute: () => lifecycleRoute,
  path: '/',
  component: LifecycleIndex,
})

export const lifecycleClusterRoute = withCrumb(({ params }) => params.slug, {
  getParentRoute: () => lifecycleRoute,
  path: '/$slug',
  component: LifecycleClusterView,
})

const kanbanRoute = withCrumb('Kanban', {
  getParentRoute: () => rootRoute,
  path: '/kanban',
  component: KanbanBoard,
})

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
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
