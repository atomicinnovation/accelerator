import { describe, expect, it } from "vitest";
import type { IndexEntry } from "../../api/types";
import {
  buildKanbanAnnouncements,
  describeEntry,
  workItemIdFromRelPath,
} from "./announcements";

const fooEntry = {
  type: "work-items" as const,
  path: "/abs/meta/work/0001-foo.md",
  relPath: "meta/work/0001-foo.md",
  slug: "foo",
  frontmatter: {},
  frontmatterState: "parsed" as const,
  workItemRefs: [],
  title: "Foo",
  mtimeMs: 0,
  size: 0,
  etag: "sha256-x",
  bodyPreview: "",
} as unknown as IndexEntry;

describe("workItemIdFromRelPath", () => {
  it("extracts the NNNN- prefix", () => {
    expect(workItemIdFromRelPath("meta/work/0001-foo.md")).toBe("0001");
    expect(workItemIdFromRelPath("meta/work/0042-bar.md")).toBe("0042");
  });

  it("returns null when the prefix is missing", () => {
    expect(workItemIdFromRelPath("meta/work/foo.md")).toBeNull();
  });
});

describe("buildKanbanAnnouncements", () => {
  const entriesMap = new Map([[fooEntry.relPath, fooEntry]]);
  const cols = [{ key: "in-progress", label: "In progress" }];
  const a = buildKanbanAnnouncements({
    entries: () => entriesMap,
    columns: () => cols,
  });

  it("onDragStart includes the work item number and title (colon separator)", () => {
    const msg = a.onDragStart!({
      active: { id: "meta/work/0001-foo.md" },
    } as any);
    expect(msg).toBe("Picked up work item 0001: Foo.");
  });

  it("onDragEnd maps column id to its display label", () => {
    const msg = a.onDragEnd!({
      active: { id: "meta/work/0001-foo.md" },
      over: { id: "in-progress" },
    } as any);
    expect(msg).toBe("Moved work item 0001: Foo to In progress.");
  });

  it("onDragOver announces the card over the target column label (C2)", () => {
    const msg = a.onDragOver!({
      active: { id: "meta/work/0001-foo.md" },
      over: { id: "in-progress" },
    } as any);
    expect(msg).toBe("work item 0001: Foo is over In progress.");
  });

  it("onDragOver omits announcement when there is no over target", () => {
    const msg = a.onDragOver!({
      active: { id: "meta/work/0001-foo.md" },
      over: null,
    } as any);
    expect(msg).toBeUndefined();
  });

  it("onDragEnd with no target announces a cancelled drop (C2)", () => {
    const msg = a.onDragEnd!({
      active: { id: "meta/work/0001-foo.md" },
      over: null,
    } as any);
    expect(msg).toBe("Drop of work item 0001: Foo cancelled, no target.");
  });

  it("onDragCancel labels the cancellation", () => {
    const msg = a.onDragCancel!({
      active: { id: "meta/work/0001-foo.md" },
    } as any);
    expect(msg).toBe("Drag of work item 0001: Foo cancelled.");
  });

  it("announcements use describeEntry — the same card name the toast uses", () => {
    // One source of truth: the pick-up announcement embeds exactly
    // describeEntry(entry), so the toast heading and announcements cannot drift.
    expect(describeEntry(fooEntry)).toBe("work item 0001: Foo");
    const msg = a.onDragStart!({
      active: { id: "meta/work/0001-foo.md" },
    } as any);
    expect(msg).toBe(`Picked up ${describeEntry(fooEntry)}.`);
  });
});

describe("describeEntry", () => {
  it('returns the bare "work item" fallback for a missing (undefined) entry', () => {
    expect(describeEntry(undefined)).toBe("work item");
  });
});
