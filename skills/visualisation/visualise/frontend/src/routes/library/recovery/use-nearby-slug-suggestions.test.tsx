import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "../../../api/fetch";
import { makeIndexEntry } from "../../../api/test-fixtures";
import {
  DOC_TYPE_KEYS,
  type DocTypeKey,
  type IndexEntry,
  isPhysicalDocTypeKey,
} from "../../../api/types";
import { useNearbySlugSuggestions } from "./use-nearby-slug-suggestions";

const PHYSICAL_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey);

function makeWrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: qc }, children);
  };
}

function newClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

/** A controllable promise per doc type so the fan-out can be settled
 *  staggered, one query at a time. */
interface Deferred<T> {
  promise: Promise<T>;
  resolve: (v: T) => void;
  reject: (e: unknown) => void;
}
function defer<T>(): Deferred<T> {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("useNearbySlugSuggestions", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("aggregates across types and returns ranked candidates once settled", async () => {
    // Worked example spread across two types; T₂ (2000) newer than T₁ (1000).
    const byType: Partial<Record<DocTypeKey, IndexEntry[]>> = {
      plans: [
        makeIndexEntry({
          type: "plans",
          slug: "error-screen-v2",
          relPath: "meta/plans/error-screen-v2.md",
          mtimeMs: 2000,
        }),
        makeIndexEntry({
          type: "plans",
          slug: "error-screens",
          relPath: "meta/plans/error-screens.md",
          mtimeMs: 1000,
        }),
      ],
      notes: [
        makeIndexEntry({
          type: "notes",
          slug: "legacy-error-screen",
          relPath: "meta/notes/legacy-error-screen.md",
          mtimeMs: 5000,
        }),
        makeIndexEntry({
          type: "notes",
          slug: "error-handling",
          relPath: "meta/notes/error-handling.md",
          mtimeMs: 9000,
        }),
      ],
    };
    vi.spyOn(fetchModule, "fetchDocs").mockImplementation((type) =>
      Promise.resolve(byType[type] ?? []),
    );

    const { result } = renderHook(
      () => useNearbySlugSuggestions("error-screen"),
      { wrapper: makeWrapper(newClient()) },
    );

    await waitFor(() => expect(result.current.isPending).toBe(false));
    expect(result.current.suggestions.map((s) => s.slug)).toEqual([
      "error-screen-v2",
      "error-screens",
      "legacy-error-screen",
    ]);
  });

  it("stays pending with no suggestions until the last query settles (single-pass gate)", async () => {
    const deferreds = new Map<DocTypeKey, Deferred<IndexEntry[]>>();
    for (const type of PHYSICAL_KEYS) deferreds.set(type, defer());
    vi.spyOn(fetchModule, "fetchDocs").mockImplementation(
      (type) => deferreds.get(type)?.promise ?? Promise.resolve([]),
    );

    const { result } = renderHook(
      () => useNearbySlugSuggestions("error-screen"),
      { wrapper: makeWrapper(newClient()) },
    );

    expect(result.current.isPending).toBe(true);
    expect(result.current.suggestions).toEqual([]);

    // Resolve all but the last; the matching candidate is in an early query.
    const last = PHYSICAL_KEYS[PHYSICAL_KEYS.length - 1];
    for (const type of PHYSICAL_KEYS) {
      if (type === last) continue;
      deferreds.get(type)?.resolve(
        type === "plans"
          ? [
              makeIndexEntry({
                type: "plans",
                slug: "error-screen-v2",
                relPath: "meta/plans/error-screen-v2.md",
                mtimeMs: 2000,
              }),
            ]
          : [],
      );
    }

    // Still pending — the list is withheld even though a match has resolved.
    await waitFor(() => expect(result.current.suggestions).toEqual([]));
    expect(result.current.isPending).toBe(true);

    // Settle the last query: now (and only now) the ranked list appears.
    deferreds.get(last)?.resolve([]);
    await waitFor(() => expect(result.current.isPending).toBe(false));
    expect(result.current.suggestions.map((s) => s.slug)).toEqual([
      "error-screen-v2",
    ]);
  });

  it("settles and ranks over the resolved subset when one type rejects", async () => {
    vi.spyOn(fetchModule, "fetchDocs").mockImplementation((type) => {
      if (type === "notes") return Promise.reject(new Error("notes-boom"));
      if (type === "plans")
        return Promise.resolve([
          makeIndexEntry({
            type: "plans",
            slug: "error-screen-v2",
            relPath: "meta/plans/error-screen-v2.md",
            mtimeMs: 2000,
          }),
        ]);
      return Promise.resolve([]);
    });

    const { result } = renderHook(
      () => useNearbySlugSuggestions("error-screen"),
      { wrapper: makeWrapper(newClient()) },
    );

    await waitFor(() => expect(result.current.isPending).toBe(false));
    // Ranks the resolved subset without throwing; the rejected type's
    // would-be slug is absent.
    expect(result.current.suggestions.map((s) => s.slug)).toEqual([
      "error-screen-v2",
    ]);
  });

  it("is disabled for sub-two-char slugs and does not call fetchDocs", async () => {
    const spy = vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);

    const { result } = renderHook(() => useNearbySlugSuggestions("a"), {
      wrapper: makeWrapper(newClient()),
    });

    expect(result.current).toEqual({ suggestions: [], isPending: false });
    // Give any (erroneously) enabled query a tick to fire.
    await new Promise((r) => setTimeout(r, 0));
    expect(spy).not.toHaveBeenCalled();
  });

  it("maps a null-slug entry to its relPath stem", async () => {
    vi.spyOn(fetchModule, "fetchDocs").mockImplementation((type) =>
      Promise.resolve(
        type === "notes"
          ? [
              makeIndexEntry({
                type: "notes",
                slug: null,
                relPath: "meta/notes/2026-error-note.md",
                mtimeMs: 1,
              }),
            ]
          : [],
      ),
    );

    const { result } = renderHook(() => useNearbySlugSuggestions("error"), {
      wrapper: makeWrapper(newClient()),
    });

    await waitFor(() => expect(result.current.isPending).toBe(false));
    expect(result.current.suggestions.map((s) => s.slug)).toEqual([
      "2026-error-note",
    ]);
  });

  it("never queries the virtual templates type", async () => {
    const spy = vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);

    const { result } = renderHook(
      () => useNearbySlugSuggestions("error-screen"),
      { wrapper: makeWrapper(newClient()) },
    );

    await waitFor(() => expect(result.current.isPending).toBe(false));
    expect(spy).not.toHaveBeenCalledWith("templates");
  });
});
