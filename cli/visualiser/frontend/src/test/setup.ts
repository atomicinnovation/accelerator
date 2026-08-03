import "@testing-library/jest-dom";
// Re-exported from @testing-library/dom, which is not a declared dependency —
// importing it directly would be a phantom. Every spec reaches `screen` and
// `waitFor` through this package for the same reason.
import { configure } from "@testing-library/react";
import { afterAll, vi } from "vitest";

// `waitFor` and every `findBy*` built on it poll against their OWN budget,
// which defaults to 1000ms — `testTimeout` in vite.config.ts does not govern
// them. So the reasoning recorded there applies here and had to be repeated:
// the number is a hang detector, not a latency budget. Vitest sizes its worker
// pool to the CPU count and `mise run` schedules this suite alongside cargo
// builds and the Python suites, so a mocked-promise render that settles in
// ~50ms idle has been measured past 1s on a loaded CI runner — surfacing as a
// findBy* that still sees "Loading…". A genuinely stuck query still fails,
// 5s later, and well inside the 30s testTimeout so it reports as itself.
configure({ asyncUtilTimeout: 5_000 });

// Stub global EventSource as a safety net — prevents any test that
// mounts a component using the production `useDocEvents` hook from
// opening a real network connection. The SSE hook tests themselves do
// NOT depend on this stub; they use `makeUseDocEvents(fakeFactory)` to
// inject their own fake (see use-doc-events.test.ts).
class MockEventSource {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;
  readyState = MockEventSource.OPEN;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onerror: ((e: Event) => void) | null = null;
  close = vi.fn();
}

class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}

// jsdom has no IntersectionObserver; the DevDesignSystem scroll-spy constructs
// one on mount. The mock is a no-op (scroll-spy behaviour is unit-tested via the
// pure `pickActiveSection` helper and confirmed end-to-end in Playwright).
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  takeRecords = vi.fn(() => []);
}

// These are set via Object.defineProperty rather than vi.stubGlobal so that
// they survive vi.unstubAllGlobals() calls in individual test files (e.g.
// router.test.tsx's afterEach). vi.stubGlobal-based stubs are reverted by
// unstubAllGlobals(), which would restore jsdom's "Not implemented" scrollTo
// and its real EventSource (which fires onerror when it can't connect).
Object.defineProperty(window, "scrollTo", { value: vi.fn(), writable: true });
Object.defineProperty(window, "EventSource", {
  value: MockEventSource,
  writable: true,
  configurable: true,
});
Object.defineProperty(window, "ResizeObserver", {
  value: MockResizeObserver,
  writable: true,
  configurable: true,
});
Object.defineProperty(window, "IntersectionObserver", {
  value: MockIntersectionObserver,
  writable: true,
  configurable: true,
});
Object.defineProperty(window, "matchMedia", {
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
});
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = vi.fn();
}

afterAll(() => {
  vi.unstubAllGlobals();
});
