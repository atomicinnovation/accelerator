import {
  type ReactNode,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
} from "react";
import styles from "./Popover.module.css";
import { useDismiss } from "./use-dismiss";

export interface PopoverTriggerProps {
  ref: React.RefCallback<HTMLElement>;
  "aria-haspopup": "menu";
  "aria-expanded": boolean;
  "aria-controls": string;
  onClick: () => void;
}

export interface PopoverProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  trigger: (triggerProps: PopoverTriggerProps) => ReactNode;
  children: ReactNode;
  ariaLabel?: string;
}

// Module-level singleton: when one Popover opens, it dismisses any prior open one.
let activeDismiss: (() => void) | null = null;

const FOCUSABLE_ITEM = '[role="menuitem"],[role="menuitemcheckbox"]';

export function Popover({
  open,
  onOpenChange,
  trigger,
  children,
  ariaLabel,
}: PopoverProps) {
  const shellRef = useRef<HTMLSpanElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLElement | null>(null);
  const setTriggerRef = useCallback((el: HTMLElement | null) => {
    triggerRef.current = el;
  }, []);
  const wasOpenRef = useRef<boolean>(false);
  // Our most recently registered dismiss closure — used to identify
  // ourselves in the module-level singleton check so we don't dismiss
  // our own previous closure when `onOpenChange` re-references and the
  // effect re-runs while still open.
  const ourDismissRef = useRef<(() => void) | null>(null);
  const panelId = useId();

  useDismiss(
    open,
    shellRef,
    useCallback(() => onOpenChange(false), [onOpenChange]),
  );

  // Manage module-level active singleton: when open transitions to true,
  // dismiss any previously-active popover (but not our own previous
  // closure) and register self.
  useEffect(() => {
    if (open) {
      const dismiss = () => onOpenChange(false);
      if (activeDismiss && activeDismiss !== ourDismissRef.current) {
        activeDismiss();
      }
      activeDismiss = dismiss;
      ourDismissRef.current = dismiss;
      return () => {
        if (activeDismiss === dismiss) activeDismiss = null;
        if (ourDismissRef.current === dismiss) ourDismissRef.current = null;
      };
    }
    return undefined;
  }, [open, onOpenChange]);

  // Focus the first menu item on open. Panel positioning is handled in CSS
  // (top: calc(100% + 4px); right: 0) relative to the popover shell — no
  // JS positioning needed.
  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (open && panel) {
      const firstItem = panel.querySelector<HTMLElement>(FOCUSABLE_ITEM);
      if (firstItem) firstItem.focus();
    }
  }, [open]);

  useEffect(() => {
    if (!open && wasOpenRef.current) {
      if (triggerRef.current) {
        try {
          triggerRef.current.focus();
        } catch {
          // ignore
        }
      }
    }
    wasOpenRef.current = open;
  }, [open]);

  const onPanelKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const panel = panelRef.current;
    if (!panel) return;
    const items = Array.from(
      panel.querySelectorAll<HTMLElement>(FOCUSABLE_ITEM),
    );
    if (items.length === 0) return;
    const active = document.activeElement as HTMLElement | null;
    const currentIndex = active ? items.indexOf(active) : -1;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      const next = items[(currentIndex + 1) % items.length];
      next?.focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const prev = items[(currentIndex - 1 + items.length) % items.length];
      prev?.focus();
    } else if (event.key === "Home") {
      event.preventDefault();
      items[0]?.focus();
    } else if (event.key === "End") {
      event.preventDefault();
      items[items.length - 1]?.focus();
    } else if (event.key === "Enter" || event.key === " ") {
      if (active && items.includes(active)) {
        event.preventDefault();
        active.click();
      }
    } else if (event.key === "Tab") {
      onOpenChange(false);
    }
  };

  const triggerProps: PopoverTriggerProps = {
    ref: setTriggerRef,
    "aria-haspopup": "menu",
    "aria-expanded": open,
    "aria-controls": panelId,
    onClick: () => onOpenChange(!open),
  };

  return (
    <span ref={shellRef} className={styles.popover}>
      {trigger(triggerProps)}
      <div
        ref={panelRef}
        id={panelId}
        role="menu"
        aria-label={ariaLabel}
        className={styles.panel}
        hidden={!open}
        onKeyDown={onPanelKeyDown}
      >
        {children}
      </div>
    </span>
  );
}
