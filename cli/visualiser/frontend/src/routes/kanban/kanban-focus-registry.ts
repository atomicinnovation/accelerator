import { createContext, useContext } from "react";

/**
 * Focus contract between the board and its cards (C3). Each `WorkItemCard`
 * registers its focusable `<Link>` anchor under its relPath; the board calls
 * `focus(relPath)` after a move settles to return focus to the card in its final
 * resting column. Keyed by relPath (stable across a status move) and owned by
 * the card, so the board depends on this seam rather than the card's DOM
 * structure — focus survives the WorkItemCardPresentation split and refetch
 * remounts without a `querySelector` + `CSS.escape` + rAF chain.
 */
export interface KanbanFocusRegistry {
  /** Register (el) or deregister (null, on unmount) a card's anchor. */
  register(relPath: string, el: HTMLElement | null): void;
  /** Focus the registered anchor; returns whether one was present to focus. */
  focus(relPath: string): boolean;
}

/** Build a registry backed by `store` (the board's relPath → anchor map). */
export function createKanbanFocusRegistry(
  store: Map<string, HTMLElement>,
): KanbanFocusRegistry {
  return {
    register(relPath, el) {
      if (el) store.set(relPath, el);
      else store.delete(relPath);
    },
    focus(relPath) {
      const el = store.get(relPath);
      if (el) {
        el.focus();
        return true;
      }
      return false;
    },
  };
}

const noopRegistry: KanbanFocusRegistry = {
  register: () => {},
  focus: () => false,
};

export const KanbanFocusContext =
  createContext<KanbanFocusRegistry>(noopRegistry);

export function useKanbanFocusRegistry(): KanbanFocusRegistry {
  return useContext(KanbanFocusContext);
}
