import { useNavigate, useRouter, useRouterState } from "@tanstack/react-router";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
} from "react";
import { safeSessionGetItem, safeSessionSetItem } from "../../api/safe-storage";
import { DEV_PRIOR_PATH_STORAGE_KEY } from "../../api/storage-keys";
import { DEV_CHORD_HINT, devAliasTarget } from "./dev-constants";

const DEV_PATH = "/dev";
const FALLBACK_EXIT_PATH = "/library";
/** Focus restore target on exit — a stable element present on every app route
 *  (the scroll-root `<main>`), so keyboard users are not stranded on `<body>`. */
const APP_FOCUS_ANCHOR_SELECTOR = "[data-app-focus-anchor]";

export interface DevActivation {
  /** Reactive — true while the `/dev` route is mounted. */
  isDevActive: boolean;
  /** Chord + triple-click: navigate directly to `/dev` (no hash round-trip). */
  enterDev: () => void;
  /** Exit-to-app / Escape / chord-toggle: restore the prior route, clear the
   *  section hash, and move focus back to the app focus anchor. */
  exitDev: () => void;
  /** Chord: exit when on `/dev`, else enter. Reads the live route, not a stale
   *  closure, so the chord toggles reliably however the page was reached. */
  toggleDev: () => void;
  /** Fresh (non-reactive) read of "am I on /dev?" for keydown handlers. */
  getIsDevActive: () => boolean;
  /** Register a hash the page itself writes (the scroll-spy's `replaceState`)
   *  so a same-value `hashchange` cannot re-enter the activation bridge. */
  recordProgrammaticHash: (hash: string) => void;
}

/**
 * The single owner of "am I in dev, and how do I get in/out?". Uses the
 * in-context `useRouter()`/`useNavigate()` hooks — NOT the exported `router`
 * singleton — because this hook is mounted by `RootLayout` (rootRoute's
 * component); importing the singleton here would form a
 * `router.ts → RootLayout → use-dev-activation → router.ts` cycle.
 *
 * The `#dev` / `#dev/<section>` activation aliases are bridged to the real
 * `/dev` route and normalised to the canonical `/dev#<section>` form; the prior
 * path is captured on every non-`/dev` navigation for exit-restore.
 */
export function useDevActivation(): DevActivation {
  const navigate = useNavigate();
  const router = useRouter();
  const isDevActive = useRouterState({
    select: (s) => s.location.pathname === DEV_PATH,
  });

  // In-memory value is authoritative while mounted; the per-tab session store
  // only seeds it on cold load (so a deep-link's exit target survives a reload
  // but never leaks across tabs/windows).
  const priorPathRef = useRef<string | null>(
    safeSessionGetItem(DEV_PRIOR_PATH_STORAGE_KEY),
  );
  const lastProgrammaticHashRef = useRef<string | null>(null);

  const getIsDevActive = useCallback(
    () => router.state.location.pathname === DEV_PATH,
    [router],
  );

  const recordProgrammaticHash = useCallback((hash: string) => {
    lastProgrammaticHashRef.current = hash;
  }, []);

  // Capture the prior (non-/dev) path on EVERY resolved navigation — not only on
  // alias entry — so exit-restore is correct for a direct `/dev` entry too.
  useEffect(() => {
    const capture = (pathname: string) => {
      if (pathname !== DEV_PATH) {
        priorPathRef.current = pathname;
        safeSessionSetItem(DEV_PRIOR_PATH_STORAGE_KEY, pathname);
      }
    };
    capture(router.state.location.pathname);
    return router.subscribe("onResolved", ({ toLocation }) => {
      capture(toLocation.pathname);
    });
  }, [router]);

  const resolvablePrior = useCallback((): string => {
    const prior = priorPathRef.current;
    if (prior && prior !== DEV_PATH) {
      try {
        // Validate the stored path still resolves to a route (e.g. a retired
        // showcase path no longer does) before restoring it.
        if (router.getMatchedRoutes(prior).foundRoute) return prior;
      } catch {
        /* fall through to the fallback */
      }
    }
    return FALLBACK_EXIT_PATH;
  }, [router]);

  const enterDev = useCallback(() => {
    navigate({ to: DEV_PATH });
  }, [navigate]);

  const exitDev = useCallback(() => {
    const prior = resolvablePrior();
    navigate({ to: prior as never, hash: undefined });
    // Restore focus to a stable anchor after the route swaps in.
    requestAnimationFrame(() => {
      document.querySelector<HTMLElement>(APP_FOCUS_ANCHOR_SELECTOR)?.focus();
    });
  }, [navigate, resolvablePrior]);

  const toggleDev = useCallback(() => {
    if (getIsDevActive()) exitDev();
    else enterDev();
  }, [getIsDevActive, enterDev, exitDev]);

  // Bridge EXTERNAL alias URLs (typed/pasted/bookmarked `#dev` / `#dev/<section>`)
  // to the real `/dev` route. `enterDev()` navigates directly, and navigate's
  // pushState/replaceState do not fire `hashchange`, so this only ever handles
  // genuinely external alias hashes.
  const sync = useCallback(() => {
    const { activate, section } = devAliasTarget(
      window.location.hash,
      lastProgrammaticHashRef.current,
    );
    if (!activate) return;
    // replace:true → the alias URL never enters history, so Back does not bounce
    // back through the bridge.
    navigate({ to: DEV_PATH, hash: section || undefined, replace: true });
  }, [navigate]);

  useEffect(() => {
    sync(); // cold-load alias URL
    const onHashChange = () => sync();
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, [sync]);

  // One-line DEV-build discoverability hint — the page has no production nav
  // entry, so a maintainer who opens devtools can still find it.
  useEffect(() => {
    if (import.meta.env.DEV) {
      console.info(
        `[dev] DesignSystem reference — open via the #dev hash, ${DEV_CHORD_HINT}, ` +
          "or a triple-click on the sidebar-foot version label.",
      );
    }
  }, []);

  // Stable identity (callbacks are useCallback-stable; recomputes only when the
  // reactive `isDevActive` flips) so the context value doesn't re-render every
  // consumer on each RootLayout render.
  return useMemo(
    () => ({
      isDevActive,
      enterDev,
      exitDev,
      toggleDev,
      getIsDevActive,
      recordProgrammaticHash,
    }),
    [
      isDevActive,
      enterDev,
      exitDev,
      toggleDev,
      getIsDevActive,
      recordProgrammaticHash,
    ],
  );
}

// Shared so the single RootLayout-owned hook instance reaches the sidebar-foot
// triple-click and the dev page's exit control without duplicate subscriptions.
const DevActivationContext = createContext<DevActivation | null>(null);
export const DevActivationProvider = DevActivationContext.Provider;

/** Read the shared activation surface. Returns `null` outside a provider (e.g.
 *  the Sidebar rendered standalone in tests), so consumers call it null-safely. */
export function useDevActivationContext(): DevActivation | null {
  return useContext(DevActivationContext);
}
