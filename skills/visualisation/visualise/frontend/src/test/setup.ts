import '@testing-library/jest-dom'
import { vi, beforeAll, afterAll } from 'vitest'

// Stub global EventSource as a safety net — prevents any test that
// mounts a component using the production `useDocEvents` hook from
// opening a real network connection. The SSE hook tests themselves do
// NOT depend on this stub; they use `makeUseDocEvents(fakeFactory)` to
// inject their own fake (see use-doc-events.test.ts).
class MockEventSource {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSED = 2
  readyState = MockEventSource.OPEN
  onmessage: ((e: MessageEvent) => void) | null = null
  onerror: ((e: Event) => void) | null = null
  close = vi.fn()
  constructor(_url: string) {}
}

beforeAll(() => {
  vi.stubGlobal('EventSource', MockEventSource)
})

afterAll(() => {
  vi.unstubAllGlobals()
})
