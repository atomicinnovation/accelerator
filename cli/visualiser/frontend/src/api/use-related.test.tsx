import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "./fetch";
import { useRelated } from "./use-related";

function makeWrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: qc }, children);
  };
}

describe("useRelated", () => {
  beforeEach(() => vi.restoreAllMocks());

  // ── Step 5.6 ────────────────────────────────────────────────────────
  it("does not fire while relPath is undefined", () => {
    const qc = new QueryClient();
    const spy = vi.spyOn(fetchModule, "fetchRelated").mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    });

    const initialProps: { p: string | undefined } = { p: undefined };
    const { rerender } = renderHook(
      ({ p }: { p: string | undefined }) => useRelated(p),
      { wrapper: makeWrapper(qc), initialProps },
    );
    expect(spy).not.toHaveBeenCalled();

    rerender({ p: "meta/plans/foo.md" });
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith("meta/plans/foo.md");
  });

  it("returns the response data after the query settles", async () => {
    const qc = new QueryClient();
    const payload = {
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    };
    vi.spyOn(fetchModule, "fetchRelated").mockResolvedValue(payload);

    const { result } = renderHook(() => useRelated("meta/plans/foo.md"), {
      wrapper: makeWrapper(qc),
    });

    await waitFor(() => expect(result.current.data).toEqual(payload));
  });
});
