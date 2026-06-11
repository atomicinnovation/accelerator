import { describe, expect, it } from "vitest";
import { normaliseSelection, queryKeys } from "./query-keys";

describe("queryKeys", () => {
  it("returns stable arrays for the same inputs", () => {
    expect(queryKeys.docs("plans")).toEqual(["docs", "plans"]);
    expect(queryKeys.docContent("meta/plans/foo.md")).toEqual([
      "doc-content",
      "meta/plans/foo.md",
    ]);
    expect(queryKeys.templateDetail("adr")).toEqual(["template-detail", "adr"]);
  });

  it("types key is a singleton", () => {
    expect(queryKeys.types()).toEqual(["types"]);
  });

  // ── Step 5.4 ────────────────────────────────────────────────────────
  it("related and relatedPrefix have stable shapes that nest under the prefix", () => {
    expect(queryKeys.related("meta/plans/foo.md")).toEqual([
      "related",
      "meta/plans/foo.md",
    ]);
    expect(queryKeys.relatedPrefix()).toEqual(["related"]);
  });

  it("libraryStructure with no selection produces the canonical empty key", () => {
    expect(queryKeys.libraryStructure()).toEqual(["library-structure", {}]);
    expect(queryKeys.libraryStructure({})).toEqual(["library-structure", {}]);
  });

  it("libraryStructure normalises empty option arrays to the canonical empty key", () => {
    expect(queryKeys.libraryStructure({ decisions: { status: [] } })).toEqual([
      "library-structure",
      {},
    ]);
    expect(queryKeys.libraryStructure({ decisions: {} })).toEqual([
      "library-structure",
      {},
    ]);
    expect(queryKeys.libraryStructure({ decisions: undefined })).toEqual([
      "library-structure",
      {},
    ]);
  });

  it("libraryStructure produces a distinct key for non-empty selection", () => {
    const empty = queryKeys.libraryStructure();
    const populated = queryKeys.libraryStructure({
      decisions: { status: ["open"] },
    });
    expect(empty).not.toEqual(populated);
  });

  it("libraryStructure canonicalises option order (set semantics)", () => {
    const a = queryKeys.libraryStructure({
      decisions: { status: ["open", "blocked"] },
    });
    const b = queryKeys.libraryStructure({
      decisions: { status: ["blocked", "open"] },
    });
    expect(a).toEqual(b);
  });

  it("libraryStructure canonicalises facet-key order", () => {
    const a = queryKeys.libraryStructure({
      decisions: { status: ["open"], clusterSlug: ["foo"] },
    });
    const b = queryKeys.libraryStructure({
      decisions: { clusterSlug: ["foo"], status: ["open"] },
    });
    expect(a).toEqual(b);
  });

  it("normaliseSelection returns {} on undefined and {}", () => {
    expect(normaliseSelection(undefined)).toEqual({});
    expect(normaliseSelection({})).toEqual({});
  });

  it('search(q) returns ["search", q] and distinct queries produce distinct tuples', () => {
    expect(queryKeys.search("foo")).toEqual(["search", "foo"]);
    expect(queryKeys.search("foo")).not.toEqual(queryKeys.search("bar"));
  });

  it("disabled(prefix) cannot collide with related(<relPath>)", () => {
    // The sentinel uses a doubled-underscore token that cannot appear
    // as a relPath. Even if a relPath equalled '__disabled__' the keys
    // still differ in their prefix shape (`related(...)` vs
    // `disabled('related')` both collapse to ['related', '__disabled__']
    // — so the only case that *would* collide is a doc literally named
    // '__disabled__', which is not a legal filename slug for any
    // doc-type in this project. Locked here as a contract.
    expect(queryKeys.disabled("related")).toEqual(["related", "__disabled__"]);
    expect(queryKeys.disabled("related")).not.toEqual(queryKeys.related("foo"));
  });
});
