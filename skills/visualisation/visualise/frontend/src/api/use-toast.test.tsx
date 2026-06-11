import { act, renderHook } from "@testing-library/react";
import React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  MAX_TOASTS,
  TOAST_AUTO_DISMISS_MS,
  ToastContext,
  useToast,
  useToastDispatcher,
} from "./use-toast";

describe("useToastDispatcher", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("appends a toast with the given heading/message and a unique id", () => {
    const { result } = renderHook(() => useToastDispatcher());
    expect(result.current.toasts).toEqual([]);

    let firstId = 0;
    let secondId = 0;
    act(() => {
      firstId = result.current.showToast({ heading: "A", message: "msg-a" });
    });
    act(() => {
      secondId = result.current.showToast({ heading: "B", message: "msg-b" });
    });
    expect(result.current.toasts).toHaveLength(2);
    expect(result.current.toasts[0]).toMatchObject({
      id: firstId,
      heading: "A",
      message: "msg-a",
    });
    expect(result.current.toasts[1]).toMatchObject({
      id: secondId,
      heading: "B",
      message: "msg-b",
    });
    expect(firstId).not.toBe(secondId);
  });

  it("auto-dismisses after 5s (AC: 4s present, 5.5s removed)", () => {
    const { result } = renderHook(() => useToastDispatcher());
    act(() => {
      result.current.showToast({ heading: "H", message: "M" });
    });
    expect(result.current.toasts).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(4_000);
    });
    expect(result.current.toasts).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(1_500);
    });
    expect(result.current.toasts).toHaveLength(0);
  });

  it("exposes TOAST_AUTO_DISMISS_MS = 5000", () => {
    expect(TOAST_AUTO_DISMISS_MS).toBe(5_000);
  });

  it("dismissToast removes immediately and does not double-remove or throw on later timer fire", () => {
    const { result } = renderHook(() => useToastDispatcher());
    let id = 0;
    act(() => {
      id = result.current.showToast({ heading: "H", message: "M" });
    });
    act(() => {
      result.current.dismissToast(id);
    });
    expect(result.current.toasts).toHaveLength(0);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current.toasts).toHaveLength(0);
  });

  it("dismissing one toast in a stack leaves the other intact and still auto-dismissing", () => {
    const { result } = renderHook(() => useToastDispatcher());
    let idA = 0;
    let idB = 0;
    act(() => {
      idA = result.current.showToast({ heading: "A", message: "a" });
      idB = result.current.showToast({ heading: "B", message: "b" });
    });
    act(() => {
      result.current.dismissToast(idA);
    });
    expect(result.current.toasts.map((t) => t.id)).toEqual([idB]);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current.toasts).toHaveLength(0);
  });

  it("per-toast independent timers (staggered)", () => {
    const { result } = renderHook(() => useToastDispatcher());
    let idA = 0;
    let idB = 0;
    act(() => {
      idA = result.current.showToast({ heading: "A", message: "a" });
    });
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    act(() => {
      idB = result.current.showToast({ heading: "B", message: "b" });
    });
    // A at 5s total
    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(result.current.toasts.map((t) => t.id)).toEqual([idB]);
    // B at its own 5s
    act(() => {
      vi.advanceTimersByTime(3_000);
    });
    expect(result.current.toasts).toHaveLength(0);
    // both ids referenced (keep vars used)
    expect(idA).not.toBe(idB);
  });

  it("pauseToast clears the timer; resumeToast starts a fresh window", () => {
    const { result } = renderHook(() => useToastDispatcher());
    let id = 0;
    act(() => {
      id = result.current.showToast({ heading: "H", message: "M" });
    });
    act(() => {
      result.current.pauseToast(id);
    });
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(result.current.toasts).toHaveLength(1);

    act(() => {
      result.current.resumeToast(id);
    });
    act(() => {
      vi.advanceTimersByTime(4_000);
    });
    expect(result.current.toasts).toHaveLength(1);
    act(() => {
      vi.advanceTimersByTime(1_500);
    });
    expect(result.current.toasts).toHaveLength(0);
  });

  it("resumeToast is a no-op for an unknown / already-dismissed id", () => {
    const { result } = renderHook(() => useToastDispatcher());
    expect(() => {
      act(() => {
        result.current.resumeToast(999);
      });
    }).not.toThrow();
    expect(result.current.toasts).toHaveLength(0);

    // dismissed toast does not resurrect
    let id = 0;
    act(() => {
      id = result.current.showToast({ heading: "H", message: "M" });
    });
    act(() => {
      result.current.dismissToast(id);
    });
    act(() => {
      result.current.resumeToast(id);
    });
    expect(result.current.toasts).toHaveLength(0);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(result.current.toasts).toHaveLength(0);
  });

  it("caps the stack at MAX_TOASTS and drops the OLDEST", () => {
    const { result } = renderHook(() => useToastDispatcher());
    const ids: number[] = [];
    act(() => {
      for (let i = 0; i < MAX_TOASTS + 1; i++) {
        ids.push(
          result.current.showToast({ heading: `H${i}`, message: `M${i}` }),
        );
      }
    });
    expect(result.current.toasts).toHaveLength(MAX_TOASTS);
    // Oldest id (ids[0]) should be gone
    expect(result.current.toasts.find((t) => t.id === ids[0])).toBeUndefined();
    // Newest id should be present
    expect(
      result.current.toasts.find((t) => t.id === ids[ids.length - 1]),
    ).toBeDefined();

    // Advancing time should not throw — the dropped toast's timer was cleared
    expect(() => {
      act(() => {
        vi.advanceTimersByTime(5_500);
      });
    }).not.toThrow();
    expect(result.current.toasts).toHaveLength(0);
  });

  it("clears all outstanding timers on unmount (no post-unmount setState)", () => {
    const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const { result, unmount } = renderHook(() => useToastDispatcher());
    act(() => {
      result.current.showToast({ heading: "H", message: "M" });
    });
    unmount();
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    // No "state update on unmounted component" warning
    expect(
      errSpy.mock.calls.find((c) => String(c[0] ?? "").includes("unmounted")),
    ).toBeUndefined();
    errSpy.mockRestore();
  });

  it("injectable autoDismissMs", () => {
    const { result } = renderHook(() => useToastDispatcher(1_000));
    act(() => {
      result.current.showToast({ heading: "H", message: "M" });
    });
    act(() => {
      vi.advanceTimersByTime(900);
    });
    expect(result.current.toasts).toHaveLength(1);
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(result.current.toasts).toHaveLength(0);
  });

  it('defaults kind to "info" and threads an explicit kind through', () => {
    const { result } = renderHook(() => useToastDispatcher());
    act(() => {
      result.current.showToast({ heading: "A", message: "a" });
    });
    act(() => {
      result.current.showToast({ heading: "B", message: "b", kind: "ok" });
    });
    act(() => {
      result.current.showToast({ heading: "C", message: "c", kind: "error" });
    });
    expect(result.current.toasts.map((t) => t.kind)).toEqual([
      "info",
      "ok",
      "error",
    ]);
  });

  it("error toasts persist (never auto-dismiss); info/ok still auto-dismiss", () => {
    const { result } = renderHook(() => useToastDispatcher());
    act(() => {
      result.current.showToast({ heading: "E", message: "e", kind: "error" });
      result.current.showToast({ heading: "O", message: "o", kind: "ok" });
    });
    expect(result.current.toasts).toHaveLength(2);
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    // The ok toast auto-dismissed; the error toast survives indefinitely.
    expect(result.current.toasts.map((t) => t.kind)).toEqual(["error"]);
  });

  it("error toasts are EXEMPT from the MAX_TOASTS cap while info/ok are capped", () => {
    const { result } = renderHook(() => useToastDispatcher());
    let errorId = 0;
    const okIds: number[] = [];
    act(() => {
      errorId = result.current.showToast({
        heading: "E",
        message: "e",
        kind: "error",
      });
    });
    act(() => {
      for (let i = 0; i < MAX_TOASTS + 1; i++) {
        okIds.push(
          result.current.showToast({
            heading: `O${i}`,
            message: `o${i}`,
            kind: "ok",
          }),
        );
      }
    });
    // The persistent error survives, the info/ok kinds are capped at MAX_TOASTS,
    // and the oldest ok is dropped — so the total is MAX_TOASTS + 1 (the error).
    expect(result.current.toasts).toHaveLength(MAX_TOASTS + 1);
    expect(result.current.toasts.find((t) => t.id === errorId)).toBeDefined();
    expect(
      result.current.toasts.find((t) => t.id === okIds[0]),
    ).toBeUndefined();
    expect(
      result.current.toasts.find((t) => t.id === okIds[okIds.length - 1]),
    ).toBeDefined();
  });
});

describe("useToast (consumer)", () => {
  it("returns a no-op handle when no provider is mounted", () => {
    const { result } = renderHook(() => useToast());
    expect(result.current.toasts).toEqual([]);
    expect(() => {
      result.current.showToast({ heading: "H", message: "M" });
      result.current.dismissToast(1);
      result.current.pauseToast(1);
      result.current.resumeToast(1);
    }).not.toThrow();
  });

  it("reads the provided context value", () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => {
      const handle = useToastDispatcher();
      return React.createElement(
        ToastContext.Provider,
        { value: handle },
        children,
      );
    };
    const { result } = renderHook(() => useToast(), { wrapper });
    expect(result.current.toasts).toEqual([]);
    expect(typeof result.current.showToast).toBe("function");
  });
});
