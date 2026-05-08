import '@testing-library/jest-dom'
import { vi, afterAll } from 'vitest'

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

class MockResizeObserver {
  observe = vi.fn()
  unobserve = vi.fn()
  disconnect = vi.fn()
}

// These are set via Object.defineProperty rather than vi.stubGlobal so that
// they survive vi.unstubAllGlobals() calls in individual test files (e.g.
// router.test.tsx's afterEach). vi.stubGlobal-based stubs are reverted by
// unstubAllGlobals(), which would restore jsdom's "Not implemented" scrollTo
// and its real EventSource (which fires onerror when it can't connect).
Object.defineProperty(window, 'scrollTo', { value: vi.fn(), writable: true })
Object.defineProperty(window, 'EventSource', { value: MockEventSource, writable: true, configurable: true })
Object.defineProperty(window, 'ResizeObserver', { value: MockResizeObserver, writable: true, configurable: true })
Object.defineProperty(window, 'matchMedia', {
  value: vi.fn((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
  writable: true,
  configurable: true,
})
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = vi.fn()
}

afterAll(() => {
  vi.unstubAllGlobals()
})
