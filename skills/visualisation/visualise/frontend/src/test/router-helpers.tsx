import React from 'react'
import {
  createRouter, createRootRoute, createRoute,
  createMemoryHistory, RouterProvider, Outlet,
} from '@tanstack/react-router'
import { render } from '@testing-library/react'

function buildTestRouter(ui: React.ReactNode, atUrl = '/') {
  const root = createRootRoute({ component: () => <Outlet /> })
  const indexRoute = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: () => <>{ui}</>,
  })
  const libraryTypeRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type',
    component: () => <>{ui}</>,
  })
  const libraryDocRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type/$fileSlug',
    component: () => <>{ui}</>,
  })
  const tree = root.addChildren([indexRoute, libraryTypeRoute, libraryDocRoute])
  return createRouter({
    routeTree: tree,
    history: createMemoryHistory({ initialEntries: [atUrl] }),
  })
}

export function renderWithRouterAt(ui: React.ReactNode, atUrl = '/') {
  const router = buildTestRouter(ui, atUrl)
  return render(<RouterProvider router={router} />)
}

/** Backwards-compatible wrapper used by the Sidebar test suite. */
export function MemoryRouter({ children }: { children: React.ReactNode }) {
  return <RouterProvider router={buildTestRouter(children)} />
}
