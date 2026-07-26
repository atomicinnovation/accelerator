import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithRouterAt } from "../../test/router-helpers";

// Force useSortable into its active-drag state so we can assert the sortable
// card forwards isDragging to the presentation. jsdom cannot enter dnd-kit's
// real active-drag state (that is the E2E A3 oracle), so the prop wiring is
// verified here by mocking the hook.
vi.mock("@dnd-kit/sortable", async (orig) => {
  const actual = await orig<typeof import("@dnd-kit/sortable")>();
  return {
    ...actual,
    useSortable: () => ({
      attributes: {},
      listeners: {},
      setNodeRef: () => {},
      transform: null,
      transition: undefined,
      isDragging: true,
    }),
  };
});

import { makeIndexEntry } from "../../api/test-fixtures";
import { WorkItemCard } from "./WorkItemCard";

describe("WorkItemCard (active drag)", () => {
  it("forwards useSortable isDragging to the presentation as the dragging state", async () => {
    const entry = makeIndexEntry({
      type: "work-items",
      relPath: "meta/work/0001-a.md",
      workItemId: "0001",
      title: "Dragging card",
    });
    const { container } = renderWithRouterAt(<WorkItemCard entry={entry} />);
    await screen.findByText("Dragging card");
    const card = container.querySelector(".ac-kcard") as HTMLElement;
    expect(card.hasAttribute("data-dragging")).toBe(true);
    // The sortable card is not the overlay clone.
    expect(card.hasAttribute("data-overlay")).toBe(false);
  });
});
