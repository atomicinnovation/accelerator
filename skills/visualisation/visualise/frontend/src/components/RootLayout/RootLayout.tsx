import { useQuery } from "@tanstack/react-query";
import { Outlet } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { fetchLibraryStructure, fetchTypes } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import { DocEventsContext, useDocEvents } from "../../api/use-doc-events";
import { FontModeContext, useFontMode } from "../../api/use-font-mode";
import { ThemeContext, useTheme } from "../../api/use-theme";
import { ToastContext, useToastDispatcher } from "../../api/use-toast";
import {
  UnseenDocTypesContext,
  useUnseenDocTypes,
} from "../../api/use-unseen-doc-types";
import { DEV_CHORD } from "../DevDesignSystem/dev-constants";
import {
  DevActivationProvider,
  useDevActivation,
} from "../DevDesignSystem/use-dev-activation";
import { Sidebar } from "../Sidebar/Sidebar";
import { ExternalEditToast } from "../Toaster/ExternalEditToast";
import { Toaster } from "../Toaster/Toaster";
import { Topbar } from "../Topbar/Topbar";
import styles from "./RootLayout.module.css";

function isPlainSlashKey(event: KeyboardEvent): boolean {
  return (
    event.key === "/" &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey
  );
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target instanceof HTMLInputElement) return true;
  if (target instanceof HTMLTextAreaElement) return true;
  if (target.isContentEditable) return true;
  // JSDOM doesn't always set isContentEditable from the attribute. Fall back
  // to the attribute so the test environment matches browser behaviour.
  const attr = target.getAttribute("contenteditable");
  return attr !== null && attr !== "false";
}

export function RootLayout() {
  const unseen = useUnseenDocTypes();
  const docEvents = useDocEvents({
    onEvent: unseen.onEvent,
    onReconnect: unseen.onReconnect,
  });
  const theme = useTheme();
  const fontMode = useFontMode();
  const toast = useToastDispatcher();
  const searchInputRef = useRef<HTMLInputElement>(null);
  const devActivation = useDevActivation();
  const { toggleDev, exitDev, getIsDevActive } = devActivation;

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!isPlainSlashKey(e)) return;
      if (isEditableTarget(e.target)) return;
      const input = searchInputRef.current;
      if (!input) return;
      e.preventDefault();
      input.focus();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  // DevDesignSystem activation chord + Escape exit. The chord matches on
  // `e.code` (physical key position) — a net-new pattern here (every other
  // handler keys off `e.key`) — so it survives non-US layouts and the Shift
  // uppercasing of "l"/"L". Escape only ejects from /dev and only when no
  // editable target is focused (so it clears/blurs a demo input first).
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (
        (e.metaKey || e.ctrlKey) &&
        e.shiftKey &&
        !e.altKey &&
        e.code === DEV_CHORD.code
      ) {
        if (isEditableTarget(e.target)) return;
        e.preventDefault();
        toggleDev();
        return;
      }
      if (
        e.key === "Escape" &&
        getIsDevActive() &&
        !isEditableTarget(e.target)
      ) {
        e.preventDefault();
        exitDev();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [toggleDev, exitDev, getIsDevActive]);

  const { data: docTypes = [] } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  });

  // Shared library-structure fetch — `LibraryOverviewHub` and any
  // selection-aware list view re-use the same query key (no selection arg
  // here), so React Query deduplicates and only one network request is made
  // per page load. See `normaliseSelection` in api/query-keys.ts.
  const { data: libraryStructure } = useQuery({
    queryKey: queryKeys.libraryStructure(),
    queryFn: () => fetchLibraryStructure(),
  });

  return (
    <ThemeContext.Provider value={theme}>
      <FontModeContext.Provider value={fontMode}>
        <DocEventsContext.Provider value={docEvents}>
          <UnseenDocTypesContext.Provider value={unseen}>
            <ToastContext.Provider value={toast}>
              <DevActivationProvider value={devActivation}>
                <div className={styles.root}>
                  <Topbar />
                  <div className={styles.body}>
                    <Sidebar
                      docTypes={docTypes}
                      phases={libraryStructure?.phases ?? []}
                      templates={libraryStructure?.templates ?? null}
                      searchInputRef={searchInputRef}
                    />
                    {/* `data-scroll-root` is the explicit scroll-spy root (Phase
                        5 binds the IntersectionObserver to it rather than
                        `closest("main")`); `data-app-focus-anchor` + tabIndex
                        give exit-from-dev a stable focus target. */}
                    <main
                      className={styles.main}
                      data-scroll-root
                      data-app-focus-anchor
                      tabIndex={-1}
                    >
                      <Outlet />
                    </main>
                  </div>
                </div>
              </DevActivationProvider>
              {/* INVARIANT: <ExternalEditToast/> and <Toaster/> must stay inside
            <ToastContext.Provider>. Toaster portals to document.body, so its DOM
            position is irrelevant, but if it falls outside this provider it
            silently reads the no-op handle and all toasts vanish with no type
            error. Keep these two adjacent and inside the provider. */}
              <ExternalEditToast />
              <Toaster />
            </ToastContext.Provider>
          </UnseenDocTypesContext.Provider>
        </DocEventsContext.Provider>
      </FontModeContext.Provider>
    </ThemeContext.Provider>
  );
}
