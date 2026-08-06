import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DEV_PRIOR_PATH_STORAGE_KEY } from "../../api/storage-keys";
import { type DevActivation, useDevActivation } from "./use-dev-activation";

// Latest hook return, captured each render so tests can drive enter/exit.
const captured: { current: DevActivation | null } = { current: null };

function Root() {
  captured.current = useDevActivation();
  return <Outlet />;
}

/** Rendered marker per route, so a wait can key on the tree, not the router. */
const LABELS: Record<string, string> = {
  "/library": "library",
  "/kanban": "kanban",
  "/dev": "dev",
};

function buildRouter(initial: string) {
  const root = createRootRoute({ component: Root });
  const mk = (path: string, label: string) =>
    createRoute({
      getParentRoute: () => root,
      path,
      component: () => <div>{label}</div>,
    });
  const tree = root.addChildren([
    mk("/library", LABELS["/library"]),
    mk("/kanban", LABELS["/kanban"]),
    mk("/dev", LABELS["/dev"]),
    mk("/library/$type/$fileSlug", "doc"),
  ]);
  return createRouter({
    routeTree: tree,
    history: createMemoryHistory({ initialEntries: [initial] }),
  });
}

/**
 * Wait until the hook has genuinely *observed* `pathname`.
 *
 * `router.state.location.pathname` is the wrong thing to wait on by itself:
 * the memory history reports the target the instant the router is built, so
 * the assertion is already true before `render` — which returns before the
 * async initial load has rendered anything at all. A test can therefore sail
 * past it with the tree unmounted and `useDevActivation`'s prior-path capture
 * effect never run, and then drive `exitDev` against an unseeded ref. That
 * restores the `/library` fallback instead of the real prior, which is exactly
 * the intermittent failure this suite showed under parallel load — and it is
 * invisible when the test's own starting path is also `/library`.
 *
 * So: wait for the route's rendered marker (proves the tree mounted), then for
 * the session write, which is the capture effect's only observable trace.
 */
async function settleAt(
  router: ReturnType<typeof buildRouter>,
  pathname: string,
) {
  await screen.findByText(LABELS[pathname]);
  await waitFor(() => expect(router.state.location.pathname).toBe(pathname));
  if (pathname !== "/dev") {
    await waitFor(() =>
      expect(sessionStorage.getItem(DEV_PRIOR_PATH_STORAGE_KEY)).toBe(pathname),
    );
  }
}

async function renderAt(initial: string) {
  const router = buildRouter(initial);
  render(<RouterProvider router={router} />);
  await settleAt(router, initial.split("#")[0]);
  return router;
}

function fireHashChange(hash: string) {
  act(() => {
    window.location.hash = hash;
    window.dispatchEvent(new Event("hashchange"));
  });
}

beforeEach(() => {
  sessionStorage.clear();
  window.location.hash = "";
  captured.current = null;
});

afterEach(() => {
  window.location.hash = "";
});

describe("useDevActivation — alias bridge", () => {
  it("bridges #dev/colors to /dev#colors via replace (no Back trap)", async () => {
    const router = await renderAt("/library");
    expect(router.history.length).toBe(1);

    fireHashChange("#dev/colors");

    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    expect(router.state.location.hash).toContain("colors");
    // replace:true — the alias did not push a new history entry, so Back does
    // not bounce back through the bridge.
    expect(router.history.length).toBe(1);
  });

  it("bridges bare #dev to /dev with no section", async () => {
    const router = await renderAt("/library");
    fireHashChange("#dev");
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    expect(router.state.location.hash).toBe("");
  });

  it("self-heals a stray /dev#dev/colors to /dev#colors", async () => {
    const router = await renderAt("/dev");
    fireHashChange("#dev/colors");
    await waitFor(() => expect(router.state.location.hash).toContain("colors"));
    expect(router.state.location.pathname).toBe("/dev");
  });

  it("leaves a bare #colors hash untouched (not an alias)", async () => {
    const router = await renderAt("/dev");
    fireHashChange("#colors");
    // No navigation away from /dev; the bare section hash is not an alias.
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
  });
});

describe("useDevActivation — enter / exit", () => {
  it("enterDev navigates directly to /dev (a normal push)", async () => {
    const router = await renderAt("/library");
    act(() => captured.current?.enterDev());
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
  });

  it("exitDev restores the captured prior and clears the section hash", async () => {
    const router = await renderAt("/library");
    act(() => captured.current?.enterDev());
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    act(() => captured.current?.exitDev());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/library"),
    );
    expect(router.state.location.hash).toBe("");
  });

  it("restores the LATEST non-/dev path, not the first", async () => {
    const router = await renderAt("/library");
    act(() => {
      router.navigate({ to: "/kanban" });
    });
    // Must settle, not merely arrive: the assertion below distinguishes the
    // captured prior from the fallback only if the capture actually happened.
    await settleAt(router, "/kanban");
    act(() => captured.current?.enterDev());
    await waitFor(() => expect(router.state.location.pathname).toBe("/dev"));
    act(() => captured.current?.exitDev());
    await waitFor(() => expect(router.state.location.pathname).toBe("/kanban"));
  });
});

describe("useDevActivation — exit-restore on cold load", () => {
  it("restores a session-seeded prior on a cold-load deep-link", async () => {
    sessionStorage.setItem(DEV_PRIOR_PATH_STORAGE_KEY, "/kanban");
    const router = await renderAt("/dev");
    act(() => captured.current?.exitDev());
    await waitFor(() => expect(router.state.location.pathname).toBe("/kanban"));
  });

  it("falls back to /library when no prior is stored", async () => {
    const router = await renderAt("/dev");
    act(() => captured.current?.exitDev());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/library"),
    );
  });

  it("falls back to /library when the stored prior no longer resolves", async () => {
    sessionStorage.setItem(DEV_PRIOR_PATH_STORAGE_KEY, "/glyph-showcase");
    const router = await renderAt("/dev");
    act(() => captured.current?.exitDev());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/library"),
    );
  });
});
