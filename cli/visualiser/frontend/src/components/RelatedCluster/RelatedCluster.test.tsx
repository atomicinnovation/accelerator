import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { makeIndexEntry, makeLifecycleCluster } from "../../api/test-fixtures";
import { renderWithRouterAt } from "../../test/router-helpers";
import { RelatedCluster } from "./RelatedCluster";

describe("RelatedCluster", () => {
  it("renders the cluster title", async () => {
    renderWithRouterAt(
      <RelatedCluster
        cluster={makeLifecycleCluster({ title: "Config layer", slug: "0001" })}
      />,
    );
    expect(await screen.findByText("Config layer")).toBeInTheDocument();
  });

  it("renders <n> artifacts (pluralised) and the updated time", async () => {
    renderWithRouterAt(
      <RelatedCluster
        cluster={makeLifecycleCluster({
          title: "Foo cluster",
          slug: "0001",
          entries: [makeIndexEntry(), makeIndexEntry(), makeIndexEntry()],
          lastChangedMs: 0,
        })}
      />,
    );
    // lastChangedMs 0 → formatMtime renders the em-dash sentinel. The meta
    // row splits across text nodes + a <time>, so assert on the link text.
    const link = await screen.findByRole("link", { name: /Foo cluster/ });
    expect(link.textContent).toContain("3 artifacts · —");
  });

  it("uses the singular form for a one-artifact cluster", async () => {
    renderWithRouterAt(
      <RelatedCluster
        cluster={makeLifecycleCluster({
          title: "Foo cluster",
          slug: "0001",
          entries: [makeIndexEntry()],
        })}
      />,
    );
    const link = await screen.findByRole("link", { name: /Foo cluster/ });
    expect(link.textContent).toContain("1 artifact ·");
    expect(link.textContent).not.toContain("1 artifacts");
  });

  it("links to /lifecycle/<slug>", async () => {
    renderWithRouterAt(
      <RelatedCluster
        cluster={makeLifecycleCluster({ title: "Config layer", slug: "0042" })}
      />,
    );
    const link = await screen.findByRole("link", { name: /Config layer/ });
    expect(link.getAttribute("href")).toBe("/lifecycle/0042");
  });
});
