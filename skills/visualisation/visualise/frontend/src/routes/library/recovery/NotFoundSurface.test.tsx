import { act, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "../../../api/fetch";
import { makeIndexEntry } from "../../../api/test-fixtures";
import type { DocTypeKey, IndexEntry } from "../../../api/types";
import { renderWithRouterAndQueryAt } from "../../../test/router-helpers";
import { NotFoundSurface } from "./NotFoundSurface";

/** Stub `fetchDocs` so each doc type resolves the entries supplied for it (or
 *  an empty list). */
function mockDocs(byType: Partial<Record<DocTypeKey, IndexEntry[]>> = {}) {
  return vi
    .spyOn(fetchModule, "fetchDocs")
    .mockImplementation((type) => Promise.resolve(byType[type] ?? []));
}

const WORKED_EXAMPLE: Partial<Record<DocTypeKey, IndexEntry[]>> = {
  plans: [
    makeIndexEntry({
      type: "plans",
      slug: "error-screen-v2",
      relPath: "meta/plans/error-screen-v2.md",
      title: "Error screen v2",
      mtimeMs: 2000,
    }),
    makeIndexEntry({
      type: "plans",
      slug: "error-screens",
      relPath: "meta/plans/error-screens.md",
      title: "Error screens",
      mtimeMs: 1000,
    }),
  ],
  notes: [
    makeIndexEntry({
      type: "notes",
      slug: "legacy-error-screen",
      relPath: "meta/notes/legacy-error-screen.md",
      title: "Legacy error screen",
      mtimeMs: 5000,
    }),
    makeIndexEntry({
      type: "notes",
      slug: "error-handling",
      relPath: "meta/notes/error-handling.md",
      title: "Error handling",
      mtimeMs: 9000,
    }),
  ],
};

describe("NotFoundSurface", () => {
  beforeEach(() => vi.restoreAllMocks());
  afterEach(() => vi.useRealTimers());

  it("renders the known-type 404 chrome and affordances", async () => {
    mockDocs();
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="work-items" />,
    );

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: /Document not found/i,
      }),
    ).toBeInTheDocument();
    // Eyebrow present for a known type.
    expect(screen.getByTestId("eyebrow-label")).toBeInTheDocument();
    // Always-present back-to-library.
    const backLib = screen.getByRole("link", { name: /Back to library/i });
    expect(backLib.getAttribute("href")).toBe("/library");
    // Conditional back-to-type.
    const backType = screen.getByRole("link", {
      name: /Back to work item list/i,
    });
    expect(backType.getAttribute("href")).toBe("/library/work-items");
  });

  it("quotes the missing slug in a mono element and ends the body with a period", async () => {
    mockDocs();
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="plans" />,
    );
    await screen.findByRole("heading", {
      level: 1,
      name: /Document not found/i,
    });

    const body = screen.getByTestId("not-found-body");
    const mono = body.querySelector("code");
    expect(mono?.textContent).toBe("error-screen");
    expect(body.textContent?.endsWith(".")).toBe(true);
    // One intentionally-exact assertion so copy edits are a deliberate update.
    expect(body.textContent).toBe(
      "We couldn’t find a document with the slug error-screen in this library.",
    );
  });

  it("renders the catch-all variant with no eyebrow and no back-to-type", async () => {
    const spy = mockDocs();
    const { container } = renderWithRouterAndQueryAt(<NotFoundSurface />);

    expect(
      await screen.findByRole("heading", { level: 1, name: /Page not found/i }),
    ).toBeInTheDocument();
    expect(container.querySelector('[data-slot="eyebrow"]')).toBeNull();
    expect(
      screen.getByRole("link", { name: /Back to library/i }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Back to .* list/i })).toBeNull();
    // No slug ⇒ no suggestion fan-out.
    expect(spy).not.toHaveBeenCalled();
  });

  it("defers the working hint until ~250ms (no warm-cache flash)", async () => {
    vi.useFakeTimers();
    // Never resolves: the fan-out stays pending so the deferral can be observed.
    vi.spyOn(fetchModule, "fetchDocs").mockReturnValue(new Promise(() => {}));

    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="plans" />,
    );
    // Flush the router's initial render without crossing the 250ms threshold.
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    const status = screen.getByRole("status");
    expect(status.textContent).not.toMatch(/Looking for similar documents/i);
    // No suggestion links while pending.
    expect(screen.queryByRole("link", { name: /v2/i })).toBeNull();

    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    expect(screen.getByRole("status").textContent).toMatch(
      /Looking for similar documents/i,
    );
  });

  it("announces only a short status string in the live region; links sit outside it", async () => {
    // Five distinct prefix matches.
    mockDocs({
      plans: [1, 2, 3, 4, 5].map((n) =>
        makeIndexEntry({
          type: "plans",
          slug: `error-screen-${n}`,
          relPath: `meta/plans/error-screen-${n}.md`,
          title: `Error screen ${n}`,
          mtimeMs: n,
        }),
      ),
    });
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="plans" />,
    );

    await screen.findByRole("heading", { name: /Did you mean/i, level: 2 });
    const status = screen.getByRole("status");
    expect(status.textContent).toBe("5 similar documents found");
    // The links are NOT descendants of the live region.
    expect(status.querySelector("a")).toBeNull();
    // The heading is rendered outside the live region.
    expect(status.querySelector("h2")).toBeNull();
  });

  it("lists Did you mean… suggestions in ranked order as plain list links", async () => {
    mockDocs(WORKED_EXAMPLE);
    const { container } = renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="plans" />,
    );

    const heading = await screen.findByRole("heading", {
      name: /Did you mean/i,
      level: 2,
    });
    const block = heading.closest("div") as HTMLElement;
    const links = within(block).getAllByRole("link");
    expect(links.map((l) => l.textContent)).toEqual([
      // title + type label + path are concatenated in textContent; assert order
      // via hrefs below, and the titles here.
      expect.stringContaining("Error screen v2"),
      expect.stringContaining("Error screens"),
      expect.stringContaining("Legacy error screen"),
    ]);
    expect(links.map((l) => l.getAttribute("href"))).toEqual([
      "/library/plans/error-screen-v2",
      "/library/plans/error-screens",
      "/library/notes/legacy-error-screen",
    ]);
    // Plain list links — not a composite listbox/option widget.
    expect(container.querySelector('[role="listbox"]')).toBeNull();
    expect(container.querySelector('[role="option"]')).toBeNull();
  });

  it("renders at most five suggestions when more than five match", async () => {
    mockDocs({
      plans: [1, 2, 3, 4, 5, 6, 7].map((n) =>
        makeIndexEntry({
          type: "plans",
          slug: `error-screen-${n}`,
          relPath: `meta/plans/error-screen-${n}.md`,
          title: `Error screen ${n}`,
          mtimeMs: n,
        }),
      ),
    });
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="plans" />,
    );

    const heading = await screen.findByRole("heading", {
      name: /Did you mean/i,
      level: 2,
    });
    const block = heading.closest("div") as HTMLElement;
    expect(within(block).getAllByRole("link")).toHaveLength(5);
  });

  it("matches a mixed-case missing slug case-insensitively", async () => {
    mockDocs({
      plans: [
        makeIndexEntry({
          type: "plans",
          slug: "error-screen-v2",
          relPath: "meta/plans/error-screen-v2.md",
          title: "Error screen v2",
          mtimeMs: 1,
        }),
      ],
    });
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="Error-Screen" knownType="plans" />,
    );

    expect(
      await screen.findByRole("link", { name: /Error screen v2/i }),
    ).toBeInTheDocument();
  });

  it("omits the Did you mean… block entirely when there are no matches", async () => {
    mockDocs(); // all empty
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="error-screen" knownType="plans" />,
    );

    await screen.findByRole("heading", {
      level: 1,
      name: /Document not found/i,
    });
    await waitFor(() =>
      expect(
        screen.queryByRole("heading", { name: /Did you mean/i }),
      ).toBeNull(),
    );
  });

  it("omits suggestions when the missing slug is shorter than two characters", async () => {
    const spy = mockDocs();
    renderWithRouterAndQueryAt(
      <NotFoundSurface missingSlug="a" knownType="plans" />,
    );

    await screen.findByRole("heading", {
      level: 1,
      name: /Document not found/i,
    });
    expect(screen.queryByRole("heading", { name: /Did you mean/i })).toBeNull();
    expect(spy).not.toHaveBeenCalled();
  });
});
