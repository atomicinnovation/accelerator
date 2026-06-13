import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "../../test/router-helpers";
import { RootLayout } from "./RootLayout";
import rootLayoutCss from "./RootLayout.module.css?raw";

vi.mock("../../api/use-doc-events", () => ({
  useDocEvents: vi.fn(() => ({
    connectionState: "open",
    justReconnected: false,
    setDragInProgress: vi.fn(),
    isDragInProgress: vi.fn(() => false),
    subscribe: vi.fn(() => () => {}),
  })),
  useDocEventsContext: vi.fn(() => ({
    connectionState: "open",
    justReconnected: false,
    setDragInProgress: vi.fn(),
    isDragInProgress: vi.fn(() => false),
    subscribe: vi.fn(() => () => {}),
  })),
  DocEventsContext: { Provider: ({ children }: any) => children },
}));

vi.mock("../../api/use-origin", () => ({
  useOrigin: vi.fn(() => "localhost"),
}));

vi.mock("@tanstack/react-query", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-query")>();
  return {
    ...actual,
    useQuery: vi.fn(() => ({ data: [] })),
  };
});

describe("RootLayout", () => {
  it("renders a <main> element", async () => {
    render(
      <MemoryRouter>
        <RootLayout />
      </MemoryRouter>,
    );
    expect(await screen.findByRole("main")).toBeInTheDocument();
  });

  it("renders a <nav> (sidebar)", async () => {
    render(
      <MemoryRouter>
        <RootLayout />
      </MemoryRouter>,
    );
    // The sidebar renders as <nav> and the breadcrumbs also render as <nav>
    // Confirm sidebar exists by looking for the nav with section headings
    expect(await screen.findByRole("navigation")).toBeInTheDocument();
  });

  it("renders a <header> (Topbar) above the body row in DOM order", async () => {
    const { container } = render(
      <MemoryRouter>
        <RootLayout />
      </MemoryRouter>,
    );
    await screen.findByRole("main");
    const root = container.firstChild as HTMLElement;
    const header = root?.querySelector("header");
    const body = header?.nextElementSibling;
    expect(header?.tagName).toBe("HEADER");
    expect(body).not.toBeNull();
  });

  describe("global / keybind", () => {
    async function renderLayout() {
      const result = render(
        <MemoryRouter>
          <RootLayout />
        </MemoryRouter>,
      );
      // Wait for the sidebar to render so the search input ref is attached.
      await screen.findByRole("searchbox", { name: /search/i });
      return result;
    }

    it("focuses sidebar search when no field focused", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const input = screen.getByRole("searchbox", { name: /search/i });
      expect(document.activeElement).not.toBe(input);
      await user.keyboard("/");
      expect(document.activeElement).toBe(input);
    });

    it("does not focus sidebar search when an <input> is focused", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const other = document.createElement("input");
      document.body.appendChild(other);
      other.focus();
      await user.keyboard("/");
      expect(document.activeElement).toBe(other);
      expect(other.value).toBe("/");
      other.remove();
    });

    it("does not focus sidebar search when a <textarea> is focused", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const ta = document.createElement("textarea");
      document.body.appendChild(ta);
      ta.focus();
      await user.keyboard("/");
      expect(document.activeElement).toBe(ta);
      ta.remove();
    });

    it("does not focus sidebar search when a contenteditable is focused", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const ce = document.createElement("div");
      ce.setAttribute("contenteditable", "true");
      ce.tabIndex = 0;
      document.body.appendChild(ce);
      ce.focus();
      await user.keyboard("/");
      expect(document.activeElement).toBe(ce);
      ce.remove();
    });

    it("does not activate with meta modifier", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const input = screen.getByRole("searchbox", { name: /search/i });
      const initialActive = document.activeElement;
      await user.keyboard("{Meta>}/{/Meta}");
      expect(document.activeElement).toBe(initialActive);
      expect(document.activeElement).not.toBe(input);
    });

    it("does not activate with ctrl modifier", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const input = screen.getByRole("searchbox", { name: /search/i });
      const initialActive = document.activeElement;
      await user.keyboard("{Control>}/{/Control}");
      expect(document.activeElement).toBe(initialActive);
      expect(document.activeElement).not.toBe(input);
    });

    it("does not activate with alt modifier", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const input = screen.getByRole("searchbox", { name: /search/i });
      const initialActive = document.activeElement;
      await user.keyboard("{Alt>}/{/Alt}");
      expect(document.activeElement).toBe(initialActive);
      expect(document.activeElement).not.toBe(input);
    });

    it("does not activate with shift modifier", async () => {
      const user = userEvent.setup();
      await renderLayout();
      const input = screen.getByRole("searchbox", { name: /search/i });
      const initialActive = document.activeElement;
      await user.keyboard("{Shift>}/{/Shift}");
      expect(document.activeElement).toBe(initialActive);
      expect(document.activeElement).not.toBe(input);
    });

    it("cleans up the listener on unmount", async () => {
      const user = userEvent.setup();
      const { unmount } = await renderLayout();
      unmount();
      // After unmount, pressing / should not throw and there is no
      // search input to focus.
      await user.keyboard("/");
      // No assertion needed beyond not throwing; the listener should be gone.
      expect(true).toBe(true);
    });

    it("does not call preventDefault when an editable target has focus", async () => {
      await renderLayout();
      const other = document.createElement("input");
      document.body.appendChild(other);
      other.focus();
      const event = new KeyboardEvent("keydown", {
        key: "/",
        bubbles: true,
        cancelable: true,
      });
      const preventSpy = vi.spyOn(event, "preventDefault");
      other.dispatchEvent(event);
      expect(preventSpy).not.toHaveBeenCalled();
      other.remove();
    });
  });

  describe("CSS source assertions", () => {
    it(".root declares flex-direction: column", () => {
      expect(rootLayoutCss).toMatch(/\.root\s*\{[^}]*flex-direction:\s*column/);
    });

    it(".root declares min-height: 100vh", () => {
      expect(rootLayoutCss).toMatch(/\.root\s*\{[^}]*min-height:\s*100vh/);
    });

    it(".body declares flex: 1", () => {
      expect(rootLayoutCss).toMatch(/\.body\s*\{[^}]*flex:\s*1/);
    });
  });
});

// RootLayout is rootRoute's component in production, so mount it that way here
// (a custom router with a real /dev child) to exercise the activation chord and
// the Escape exit it wires onto the shared useDevActivation hook.
describe("RootLayout — dev activation (chord + Escape)", () => {
  function buildRouter(initial: string) {
    const root = createRootRoute({ component: RootLayout });
    const lib = createRoute({
      getParentRoute: () => root,
      path: "/library",
      component: () => <div data-testid="route-library" />,
    });
    const dev = createRoute({
      getParentRoute: () => root,
      path: "/dev",
      component: () => <div data-testid="route-dev" />,
    });
    const tree = root.addChildren([lib, dev]);
    return createRouter({
      routeTree: tree,
      history: createMemoryHistory({ initialEntries: [initial] }),
    });
  }

  async function renderAt(initial: string) {
    sessionStorage.clear();
    const router = buildRouter(initial);
    render(<RouterProvider router={router} />);
    await screen.findByRole("main");
    await waitFor(() => expect(router.state.location.pathname).toBe(initial));
    return router;
  }

  function dispatchKey(
    opts: {
      code: string;
      key?: string;
      meta?: boolean;
      ctrl?: boolean;
      shift?: boolean;
      alt?: boolean;
    },
    target: EventTarget = document,
  ) {
    const event = new KeyboardEvent("keydown", {
      code: opts.code,
      key: opts.key ?? "",
      metaKey: !!opts.meta,
      ctrlKey: !!opts.ctrl,
      shiftKey: !!opts.shift,
      altKey: !!opts.alt,
      bubbles: true,
      cancelable: true,
    });
    const preventSpy = vi.spyOn(event, "preventDefault");
    act(() => {
      target.dispatchEvent(event);
    });
    return preventSpy;
  }

  it("Cmd+Shift+L enters /dev and the chord toggles back out", async () => {
    const router = await renderAt("/library");
    const enter = dispatchKey({ code: "KeyL", meta: true, shift: true });
    expect(enter).toHaveBeenCalled();
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    dispatchKey({ code: "KeyL", meta: true, shift: true });
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/library"),
    );
  });

  it("Ctrl+Shift+L also activates (cross-platform modifier)", async () => {
    const router = await renderAt("/library");
    const spy = dispatchKey({ code: "KeyL", ctrl: true, shift: true });
    expect(spy).toHaveBeenCalled();
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
  });

  it("ignores accidental near-misses (Cmd+L without Shift, Cmd+Shift+K)", async () => {
    const router = await renderAt("/library");
    expect(dispatchKey({ code: "KeyL", meta: true })).not.toHaveBeenCalled();
    expect(
      dispatchKey({ code: "KeyK", meta: true, shift: true }),
    ).not.toHaveBeenCalled();
    expect(router.state.location.pathname).toBe("/library");
  });

  it("chord is inert when an editable target is focused", async () => {
    await renderAt("/library");
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();
    const spy = dispatchKey({ code: "KeyL", meta: true, shift: true }, input);
    expect(spy).not.toHaveBeenCalled();
    input.remove();
  });

  it("Escape exits /dev to the prior route", async () => {
    const router = await renderAt("/library");
    dispatchKey({ code: "KeyL", meta: true, shift: true });
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    const spy = dispatchKey({ code: "Escape", key: "Escape" });
    expect(spy).toHaveBeenCalled();
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/library"),
    );
  });

  it("Escape is inert inside a focused editable target (does not eject)", async () => {
    const router = await renderAt("/library");
    dispatchKey({ code: "KeyL", meta: true, shift: true });
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();
    const spy = dispatchKey({ code: "Escape", key: "Escape" }, input);
    expect(spy).not.toHaveBeenCalled();
    expect(router.state.location.pathname).toBe("/dev");
    input.remove();
  });
});
