import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { makeIndexEntry } from "../../api/test-fixtures";
import type { RelatedArtifactsResponse } from "../../api/types";
import { RelatedArtifacts } from "./RelatedArtifacts";

const empty: RelatedArtifactsResponse = {
  inferredCluster: [],
  declaredOutbound: [],
  declaredInbound: [],
};

const examplePlan = makeIndexEntry({
  type: "plans",
  relPath: "meta/plans/2026-04-18-foo.md",
  path: "/x/meta/plans/2026-04-18-foo.md",
  title: "Foo Plan",
});
const exampleReview = makeIndexEntry({
  type: "plan-reviews",
  relPath: "meta/reviews/plans/2026-04-18-foo-review-1.md",
  path: "/x/meta/reviews/plans/2026-04-18-foo-review-1.md",
  title: "Foo review",
});
const exampleAdr = makeIndexEntry({
  type: "decisions",
  relPath: "meta/decisions/ADR-0001-example.md",
  path: "/x/meta/decisions/ADR-0001-example.md",
  title: "Example decision",
});

describe("RelatedArtifacts", () => {
  it("shows all-empty message when all three arrays are empty", () => {
    render(<RelatedArtifacts related={empty} />);
    expect(
      screen.getByText("This document has no declared or inferred relations."),
    ).toBeInTheDocument();
    // Option B has no sub-group headings.
    expect(screen.queryByRole("heading", { level: 4 })).toBeNull();
  });

  it("renders one list with no sub-group headings", () => {
    render(
      <RelatedArtifacts
        related={{
          inferredCluster: [exampleAdr],
          declaredOutbound: [examplePlan],
          declaredInbound: [exampleReview],
        }}
      />,
    );
    // Single list, no level-4 group headings.
    expect(screen.queryByRole("heading", { level: 4 })).toBeNull();
    expect(screen.getAllByTestId("related-list")).toHaveLength(1);
  });

  it("tags declared rows (declared) and inferred rows (inferred)", () => {
    render(
      <RelatedArtifacts
        related={{
          inferredCluster: [exampleAdr],
          declaredOutbound: [examplePlan],
          declaredInbound: [exampleReview],
        }}
      />,
    );
    const rows = screen.getAllByTestId("related-row");
    const declaredRows = rows.filter((r) => r.dataset.kind === "declared");
    const inferredRows = rows.filter((r) => r.dataset.kind === "inferred");
    expect(declaredRows).toHaveLength(2);
    expect(inferredRows).toHaveLength(1);
    for (const row of declaredRows) {
      expect(within(row).getByText("(declared)")).toBeInTheDocument();
    }
    for (const row of inferredRows) {
      expect(within(row).getByText("(inferred)")).toBeInTheDocument();
    }
  });

  it("orders rows declared-first then inferred (outbound, inbound, cluster)", () => {
    // Distinct paths so dedup cannot shorten the list and shift indices.
    const outbound = makeIndexEntry({
      type: "plans",
      relPath: "meta/plans/a.md",
      path: "/x/a.md",
      title: "Alpha outbound",
    });
    const inbound = makeIndexEntry({
      type: "plan-reviews",
      relPath: "meta/reviews/b.md",
      path: "/x/b.md",
      title: "Bravo inbound",
    });
    const cluster = makeIndexEntry({
      type: "decisions",
      relPath: "meta/decisions/c.md",
      path: "/x/c.md",
      title: "Charlie cluster",
    });
    render(
      <RelatedArtifacts
        related={{
          declaredOutbound: [outbound],
          declaredInbound: [inbound],
          inferredCluster: [cluster],
        }}
      />,
    );
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Alpha outbound");
    expect(rows[0]).toHaveTextContent("(declared)");
    expect(rows[1]).toHaveTextContent("Bravo inbound");
    expect(rows[1]).toHaveTextContent("(declared)");
    expect(rows[2]).toHaveTextContent("Charlie cluster");
    expect(rows[2]).toHaveTextContent("(inferred)");
  });

  it("dedupes a bidirectional declared relation to a single row", () => {
    // Same entry (same path) in both outbound and inbound renders once.
    render(
      <RelatedArtifacts
        related={{
          declaredOutbound: [examplePlan],
          declaredInbound: [examplePlan],
          inferredCluster: [],
        }}
      />,
    );
    const rows = screen.getAllByTestId("related-row");
    expect(rows).toHaveLength(1);
    expect(screen.getAllByText("(declared)")).toHaveLength(1);
    expect(screen.getByRole("link", { name: "Foo Plan" })).toBeInTheDocument();
  });

  it("renders correctly with inferred relations only", () => {
    render(
      <RelatedArtifacts
        related={{ ...empty, inferredCluster: [exampleAdr] }}
      />,
    );
    const rows = screen.getAllByTestId("related-row");
    expect(rows).toHaveLength(1);
    expect(rows[0].dataset.kind).toBe("inferred");
    expect(
      screen.queryByText(
        "This document has no declared or inferred relations.",
      ),
    ).toBeNull();
  });

  it("each row links to /library/{type}/{slug}", () => {
    render(
      <RelatedArtifacts
        related={{
          declaredOutbound: [examplePlan],
          declaredInbound: [exampleReview],
          inferredCluster: [exampleAdr],
        }}
      />,
    );
    expect(
      screen.getByRole("link", { name: "Foo Plan" }).getAttribute("href"),
    ).toBe("/library/plans/2026-04-18-foo");
    expect(
      screen.getByRole("link", { name: "Foo review" }).getAttribute("href"),
    ).toBe("/library/plan-reviews/2026-04-18-foo-review-1");
    expect(
      screen
        .getByRole("link", { name: "Example decision" })
        .getAttribute("href"),
    ).toBe("/library/decisions/ADR-0001-example");
  });

  it("routes a related RCA row to the RCA detail page with the RCA singular label", () => {
    const exampleRca = makeIndexEntry({
      type: "root-cause-analyses",
      relPath: "meta/research/issues/2026-06-10-example-rca.md",
      path: "/x/meta/research/issues/2026-06-10-example-rca.md",
      title: "Example RCA",
    });
    render(
      <RelatedArtifacts
        related={{ ...empty, declaredInbound: [exampleRca] }}
      />,
    );
    const link = screen.getByRole("link", { name: "Example RCA" });
    expect(link.getAttribute("href")).toBe(
      "/library/root-cause-analyses/2026-06-10-example-rca",
    );
    const row = screen.getByTestId("related-row");
    expect(within(row).getByText("Root cause analysis")).toBeInTheDocument();
    expect(row.querySelector("svg")?.getAttribute("data-doc-type")).toBe(
      "root-cause-analyses",
    );
  });

  it("renders no legend", () => {
    render(
      <RelatedArtifacts
        related={{ ...empty, declaredOutbound: [examplePlan] }}
      />,
    );
    expect(screen.queryByText("Declared")).toBeNull();
    expect(screen.queryByText("Inferred")).toBeNull();
  });

  it("shows Updating hint only when showUpdatingHint is true", () => {
    const populated = { ...empty, declaredOutbound: [examplePlan] };
    const { rerender } = render(
      <RelatedArtifacts related={populated} showUpdatingHint />,
    );
    const hint = screen.getByText("Updating…");
    expect(hint).toBeInTheDocument();
    expect(hint.getAttribute("aria-live")).toBe("polite");

    rerender(<RelatedArtifacts related={populated} showUpdatingHint={false} />);
    expect(screen.queryByText("Updating…")).toBeNull();
  });

  it("each row renders a decorative Glyph matching the row doc type", () => {
    render(
      <RelatedArtifacts
        related={{
          inferredCluster: [exampleAdr],
          declaredOutbound: [examplePlan],
          declaredInbound: [],
        }}
      />,
    );
    for (const [kind, docType] of [
      ["declared", "plans"],
      ["inferred", "decisions"],
    ] as const) {
      const row = screen
        .getAllByTestId("related-row")
        .find((r) => r.dataset.kind === kind);
      expect(row, `missing ${kind} row`).toBeDefined();
      const svg = row!.querySelector("svg");
      expect(svg, `missing row icon in ${kind} row`).not.toBeNull();
      expect(svg!.getAttribute("data-doc-type")).toBe(docType);
      expect(svg!.getAttribute("aria-hidden")).toBe("true");
      expect(svg!.getAttribute("role")).toBeNull();
    }
  });
});
