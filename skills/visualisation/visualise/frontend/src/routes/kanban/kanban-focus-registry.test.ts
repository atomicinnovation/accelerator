import { afterEach, describe, expect, it } from "vitest";
import { createKanbanFocusRegistry } from "./kanban-focus-registry";

afterEach(() => {
  document.body.innerHTML = "";
});

describe("createKanbanFocusRegistry", () => {
  it("focuses the registered anchor and reports it was present", () => {
    const store = new Map<string, HTMLElement>();
    const registry = createKanbanFocusRegistry(store);
    const anchor = document.createElement("a");
    anchor.href = "#";
    document.body.appendChild(anchor);

    registry.register("meta/work/0001-a.md", anchor);
    const focused = registry.focus("meta/work/0001-a.md");

    expect(focused).toBe(true);
    expect(document.activeElement).toBe(anchor);
  });

  it("returns false when no anchor is registered for the relPath", () => {
    const registry = createKanbanFocusRegistry(new Map());
    expect(registry.focus("meta/work/unknown.md")).toBe(false);
  });

  it("deregisters on null (card unmount) so a stale node is not focused", () => {
    const store = new Map<string, HTMLElement>();
    const registry = createKanbanFocusRegistry(store);
    const anchor = document.createElement("a");
    document.body.appendChild(anchor);

    registry.register("meta/work/0001-a.md", anchor);
    registry.register("meta/work/0001-a.md", null);

    expect(registry.focus("meta/work/0001-a.md")).toBe(false);
  });

  it("re-registration under the same relPath resolves to the live (remounted) node", () => {
    const store = new Map<string, HTMLElement>();
    const registry = createKanbanFocusRegistry(store);
    const stale = document.createElement("a");
    const live = document.createElement("a");
    stale.href = "#";
    live.href = "#";
    document.body.append(stale, live);

    registry.register("meta/work/0001-a.md", stale);
    registry.register("meta/work/0001-a.md", live); // refetch remount

    registry.focus("meta/work/0001-a.md");
    expect(document.activeElement).toBe(live);
  });
});
