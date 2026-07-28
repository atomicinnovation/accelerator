import { describe, expect, it } from "vitest";
import type { DocTypeKey } from "../../../api/types";
import {
  isSuggestible,
  rankSlugSuggestions,
  type SlugCandidate,
} from "./rank-slug-suggestions";

/** Build a SlugCandidate with sensible defaults; the `slug` doubles as the
 *  default `relPath` stem and `title` so cases stay terse. */
function cand(
  overrides: Partial<SlugCandidate> & { slug: string },
): SlugCandidate {
  return {
    type: "plans" as DocTypeKey,
    title: overrides.slug,
    mtimeMs: 0,
    relPath: `meta/plans/${overrides.slug}.md`,
    ...overrides,
  };
}

describe("rankSlugSuggestions", () => {
  it("ranks the worked example in the AC-specified order", () => {
    // T₂ newer than T₁.
    const candidates: SlugCandidate[] = [
      cand({ slug: "error-screen-v2", mtimeMs: 2000 }),
      cand({ slug: "error-screens", mtimeMs: 1000 }),
      cand({ slug: "legacy-error-screen", mtimeMs: 5000 }), // interior, newest
      cand({ slug: "error-handling", mtimeMs: 9000 }), // no match
    ];
    const ranked = rankSlugSuggestions("error-screen", candidates);
    expect(ranked.map((c) => c.slug)).toEqual([
      "error-screen-v2",
      "error-screens",
      "legacy-error-screen",
    ]);
  });

  it("ranks prefix above interior regardless of mtime", () => {
    const candidates: SlugCandidate[] = [
      cand({ slug: "x-error-y", mtimeMs: 9999 }), // interior, very new
      cand({ slug: "error-thing", mtimeMs: 1 }), // prefix, very old
    ];
    const ranked = rankSlugSuggestions("error", candidates);
    expect(ranked.map((c) => c.slug)).toEqual(["error-thing", "x-error-y"]);
  });

  it("orders by mtime descending within a bucket", () => {
    const candidates: SlugCandidate[] = [
      cand({ slug: "error-old", mtimeMs: 100 }),
      cand({ slug: "error-new", mtimeMs: 900 }),
      cand({ slug: "error-mid", mtimeMs: 500 }),
    ];
    const ranked = rankSlugSuggestions("error", candidates);
    expect(ranked.map((c) => c.slug)).toEqual([
      "error-new",
      "error-mid",
      "error-old",
    ]);
  });

  it("breaks ties on relPath ascending when bucket and mtime are equal", () => {
    const candidates: SlugCandidate[] = [
      cand({
        slug: "error-b",
        mtimeMs: 500,
        relPath: "meta/plans/zzz-error-b.md",
      }),
      cand({
        slug: "error-a",
        mtimeMs: 500,
        relPath: "meta/plans/aaa-error-a.md",
      }),
    ];
    const ranked = rankSlugSuggestions("error", candidates);
    expect(ranked.map((c) => c.relPath)).toEqual([
      "meta/plans/aaa-error-a.md",
      "meta/plans/zzz-error-b.md",
    ]);
  });

  it("returns exactly the top five when more than five candidates match", () => {
    const candidates: SlugCandidate[] = [
      cand({ slug: "error-1", mtimeMs: 6 }),
      cand({ slug: "error-2", mtimeMs: 5 }),
      cand({ slug: "error-3", mtimeMs: 4 }),
      cand({ slug: "error-4", mtimeMs: 3 }),
      cand({ slug: "error-5", mtimeMs: 2 }),
      cand({ slug: "error-6", mtimeMs: 1 }),
    ];
    const ranked = rankSlugSuggestions("error", candidates);
    expect(ranked).toHaveLength(5);
    expect(ranked.map((c) => c.slug)).toEqual([
      "error-1",
      "error-2",
      "error-3",
      "error-4",
      "error-5",
    ]);
  });

  it("matches case-insensitively on both prefix and interior branches", () => {
    const candidates: SlugCandidate[] = [
      cand({ slug: "error-screen-v2", mtimeMs: 2 }), // prefix
      cand({ slug: "legacy-error-screen", mtimeMs: 1 }), // interior
    ];
    const ranked = rankSlugSuggestions("Error-Screen", candidates);
    expect(ranked.map((c) => c.slug)).toEqual([
      "error-screen-v2",
      "legacy-error-screen",
    ]);
  });

  it("returns [] when the missing slug is shorter than two characters", () => {
    expect(rankSlugSuggestions("a", [cand({ slug: "a-thing" })])).toEqual([]);
  });

  it("normalises a whitespace-padded missing slug before matching", () => {
    const candidates: SlugCandidate[] = [cand({ slug: "error-thing" })];
    expect(rankSlugSuggestions("  er", candidates).map((c) => c.slug)).toEqual([
      "error-thing",
    ]);
  });

  it("ranks a candidate whose slug was derived from its relPath stem", () => {
    // Mirrors the hook's `slug ?? fileSlugFromRelPath(relPath)` mapping: the
    // candidate arrives with a slug taken from the file stem.
    const candidates: SlugCandidate[] = [
      cand({
        slug: "2026-error-note",
        relPath: "meta/notes/2026-error-note.md",
      }),
    ];
    expect(rankSlugSuggestions("error", candidates).map((c) => c.slug)).toEqual(
      ["2026-error-note"],
    );
  });

  it("excludes an exact-slug match (never suggests a found document)", () => {
    const candidates: SlugCandidate[] = [
      cand({ slug: "error-screen" }), // exact
      cand({ slug: "error-screen-v2", mtimeMs: 5 }), // prefix
    ];
    const ranked = rankSlugSuggestions("error-screen", candidates);
    expect(ranked.map((c) => c.slug)).toEqual(["error-screen-v2"]);
  });

  it("returns [] when no candidate matches", () => {
    expect(
      rankSlugSuggestions("error", [cand({ slug: "totally-unrelated" })]),
    ).toEqual([]);
  });
});

describe("isSuggestible", () => {
  it("is false for inputs shorter than two chars after normalisation", () => {
    expect(isSuggestible("a")).toBe(false);
    expect(isSuggestible(" ")).toBe(false);
    expect(isSuggestible("")).toBe(false);
  });

  it("is true for two-or-more chars, including whitespace-padded", () => {
    expect(isSuggestible("ab")).toBe(true);
    expect(isSuggestible("  ab ")).toBe(true);
  });
});
