import React from 'react'
import { createRouter, createRootRoute, createMemoryHistory, RouterProvider } from '@tanstack/react-router'

export function MemoryRouter({ children }: { children: React.ReactNode }) {
  const root = createRootRoute({ component: () => <>{children}</> })
  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ['/'] }),
  })
  return <RouterProvider router={router} />
}
