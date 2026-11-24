import { Link } from "@tanstack/react-router";
import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithRouterAt } from "../test/router-helpers";
import {
  descriptiveSlugOf,
  documentIdOf,
  libraryHref,
  libraryLink,
  resolveLibraryDocument,
} from "./library-document-address";
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

  it("prefers a stem match over an earlier entry whose slug is the document id", () => {
    const bySlug = makeIndexEntry({
      relPath: "meta/notes/2026-01-01-foo.md",
      id: null,
      slug: "foo",
    });
    const byStem = makeIndexEntry({
      relPath: "meta/notes/foo.md",
      id: null,
      slug: "other",
    });

    expect(resolveLibraryDocument([bySlug, byStem], "foo")).toBe(byStem);
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

describe("documentIdOf", () => {
  it("is the document's id", () => {
    expect(
      documentIdOf({
        type: "plans",
        id: "PP-1",
        relPath: "meta/plans/2026-01-01-foo.md",
      }),
    ).toBe("PP-1");
  });

  it("is the filename stem when the document has no id", () => {
    expect(
      documentIdOf({
        type: "plans",
        id: null,
        relPath: "meta/plans/2026-01-01-foo.md",
      }),
    ).toBe("2026-01-01-foo");
  });
});

describe("libraryLink", () => {
  it("addresses a research set by its id", () => {
    expect(
      libraryLink({
        type: "topic-research",
        id: "b",
        relPath: "meta/research/topics/b/manifest.md",
      }),
    ).toEqual({
      to: "/library/$type/$documentId",
      params: { type: "topic-research", documentId: "b" },
    });
  });

  it("addresses a document with no id by its filename stem", () => {
    expect(
      libraryLink({ type: "plans", id: null, relPath: "meta/plans/foo.md" }),
    ).toEqual({
      to: "/library/$type/$documentId",
      params: { type: "plans", documentId: "foo" },
    });
  });
});

describe("libraryHref", () => {
  it("renders the link as a path", () => {
    expect(
      libraryHref(
        libraryLink({ type: "plans", id: null, relPath: "meta/plans/foo.md" }),
      ),
    ).toBe("/library/plans/foo");
  });

  it("percent-encodes a reserved character in an id", () => {
    expect(
      libraryHref(
        libraryLink({ type: "plans", id: "a b?c", relPath: "meta/plans/x.md" }),
      ),
    ).toBe("/library/plans/a%20b%3Fc");
  });

  it.each([
    ["a plain id", "PP-1"],
    ["an id with a reserved character", "a b?c#d"],
  ])("matches a rendered router link's href for %s", async (_, id) => {
    const link = libraryLink({ type: "plans", id, relPath: "meta/plans/x.md" });

    renderWithRouterAt(<Link {...link}>doc</Link>);

    expect(await screen.findByRole("link", { name: "doc" })).toHaveAttribute(
      "href",
      libraryHref(link),
    );
  });
});

describe("round trip", () => {
  it("resolves every document's link back to that document", () => {
    const entries = [
      makeIndexEntry({
        type: "topic-research",
        relPath: "meta/research/topics/a/manifest.md",
        id: "a",
        slug: null,
      }),
      makeIndexEntry({
        type: "topic-research",
        relPath: "meta/research/topics/b/manifest.md",
        id: "b",
        slug: null,
      }),
      makeIndexEntry({
        relPath: "meta/plans/2026-01-01-manifest.md",
        id: "2026-01-01-manifest",
        slug: "manifest",
      }),
    ];

    for (const entry of entries) {
      expect(resolveLibraryDocument(entries, documentIdOf(entry))).toBe(entry);
    }
  });

  it("resolves a stem link back to its document when another document has that slug", () => {
    const entries = [
      makeIndexEntry({
        relPath: "meta/notes/2026-01-01-foo.md",
        id: null,
        slug: "foo",
      }),
      makeIndexEntry({
        relPath: "meta/notes/foo.md",
        id: null,
        slug: "bar",
      }),
    ];

    for (const entry of entries) {
      expect(resolveLibraryDocument(entries, documentIdOf(entry))).toBe(entry);
    }
  });
});

describe("descriptiveSlugOf", () => {
  it("is the document's slug", () => {
    expect(
      descriptiveSlugOf({
        slug: "foo",
        relPath: "meta/plans/2026-01-01-foo.md",
      }),
    ).toBe("foo");
  });

  it("is the filename stem when the document has no slug", () => {
    expect(
      descriptiveSlugOf({
        slug: null,
        relPath: "meta/research/topics/a/manifest.md",
      }),
    ).toBe("manifest");
  });
});
