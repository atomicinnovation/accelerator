import { vi, beforeEach } from 'vitest'
import { waitFor } from '@testing-library/react'
import { createRouter, createMemoryHistory } from '@tanstack/react-router'
import { QueryClient } from '@tanstack/react-query'
import { routeTree } from '../router'
import * as fetchModule from '../api/fetch'

export function setupRouterFixtures() {
  beforeEach(() => {
    vi.spyOn(fetchModule, 'fetchTypes').mockResolvedValue([])
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: [] })
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue({
      name: 'adr', activeTier: 'plugin-default', tiers: [],
    })
  })
}

export function buildRouter(url: string) {
  return {
    router: createRouter({
      routeTree,
      history: createMemoryHistory({ initialEntries: [url] }),
    }),
    queryClient: new QueryClient(),
  }
}

export async function waitForPath(
  router: { state: { location: { pathname: string } } },
  expected: string,
): Promise<void> {
  await waitFor(() => {
    expect(router.state.location.pathname).toBe(expected)
  })
}
