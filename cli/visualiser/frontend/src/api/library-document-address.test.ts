import { describe, expect, it } from "vitest";
import { resolveLibraryDocument } from "./library-document-address";
import { makeIndexEntry } from "./test-fixtures";

describe("resolveLibraryDocument", () => {
  it("matches the entry whose id is the document id", () => {
    const set = makeIndexEntry({
      type: "topic-research",
      relPath: "meta/research/topics/prompt-caching/manifest.md",
      id: "prompt-caching",
      slug: null,
    });
    const other = makeIndexEntry({
      type: "topic-research",
      relPath: "meta/research/topics/mcp/manifest.md",
      id: "mcp",
      slug: null,
    });

    expect(resolveLibraryDocument([other, set], "prompt-caching")).toBe(set);
  });

  it("prefers an id match over an earlier entry whose slug is the document id", () => {
    const bySlug = makeIndexEntry({
      relPath: "meta/plans/2026-01-01-foo.md",
      id: "2026-01-01-foo",
      slug: "x",
    });
    const byId = makeIndexEntry({
      relPath: "meta/plans/2026-02-02-bar.md",
      id: "x",
      slug: "bar",
    });

    expect(resolveLibraryDocument([bySlug, byId], "x")).toBe(byId);
  });

  it("matches a slug when no id matches", () => {
    const plan = makeIndexEntry({
      relPath: "meta/plans/2026-01-01-foo.md",
      id: "2026-01-01-foo",
      slug: "foo",
    });

    expect(resolveLibraryDocument([plan], "foo")).toBe(plan);
  });

  it("matches a filename stem when neither an id nor a slug matches", () => {
    const plan = makeIndexEntry({
      relPath: "meta/plans/2026-01-01-foo.md",
      id: null,
      slug: "foo",
    });

    expect(resolveLibraryDocument([plan], "2026-01-01-foo")).toBe(plan);
  });

  it("prefers a slug match over an earlier entry whose stem is the document id", () => {
    const byStem = makeIndexEntry({
      relPath: "meta/notes/foo.md",
      id: null,
      slug: "other",
    });
    const bySlug = makeIndexEntry({
      relPath: "meta/notes/2026-01-01-foo.md",
      id: null,
      slug: "foo",
    });

    expect(resolveLibraryDocument([byStem, bySlug], "foo")).toBe(bySlug);
  });

  it("opens a review by its stem when two reviews share a slug", () => {
    const first = makeIndexEntry({
      type: "plan-reviews",
      relPath: "meta/reviews/plans/2026-01-01-x-review-1.md",
      id: null,
      slug: "x-review",
    });
    const second = makeIndexEntry({
      type: "plan-reviews",
      relPath: "meta/reviews/plans/2026-01-01-x-review-2.md",
      id: null,
      slug: "x-review",
    });

    expect(
      resolveLibraryDocument([first, second], "2026-01-01-x-review-2"),
    ).toBe(second);
  });

  it("finds nothing when no entry matches", () => {
    const plan = makeIndexEntry({
      relPath: "meta/plans/foo.md",
      id: "foo",
      slug: "foo",
    });

    expect(resolveLibraryDocument([plan], "missing")).toBeUndefined();
  });
});
