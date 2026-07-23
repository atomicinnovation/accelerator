import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import type { LibraryFacet, LibrarySelectionPerType } from "../../api/types";
import { FilterPill } from "./FilterPill";

const STATUS_FACET: LibraryFacet = {
  id: "status",
  label: "Status",
  options: [
    { id: "open", label: "Open", count: 3 },
    { id: "blocked", label: "Blocked", count: 1 },
  ],
};

const LONG_FACET: LibraryFacet = {
  id: "clusterSlug",
  label: "Cluster",
  options: Array.from({ length: 11 }, (_, i) => ({
    id: `cluster-${i}`,
    label: `cluster-${i}`,
    count: 1,
  })),
};

function Controlled({
  facets,
  initial,
  onChange,
  isFetching,
}: {
  facets: LibraryFacet[];
  initial?: LibrarySelectionPerType;
  onChange?: (next: LibrarySelectionPerType) => void;
  isFetching?: boolean;
}) {
  const [sel, setSel] = useState<LibrarySelectionPerType>(initial ?? {});
  return (
    <FilterPill
      facets={facets}
      selection={sel}
      onChange={(next) => {
        setSel(next);
        onChange?.(next);
      }}
      isFetching={isFetching}
    />
  );
}

describe("FilterPill", () => {
  it("renders the trigger labelled Filter", () => {
    render(<Controlled facets={[STATUS_FACET]} />);
    expect(screen.getByRole("button", { name: /filter/i })).toBeInTheDocument();
  });

  it("opens a menu listing the facet sections and options", async () => {
    const user = userEvent.setup();
    render(<Controlled facets={[STATUS_FACET]} />);
    await user.click(screen.getByRole("button", { name: /filter/i }));
    expect(screen.getByText("Status")).toBeInTheDocument();
    expect(screen.getAllByRole("menuitemcheckbox")).toHaveLength(2);
  });

  it("renders option counts next to each option", async () => {
    const user = userEvent.setup();
    render(<Controlled facets={[STATUS_FACET]} />);
    await user.click(screen.getByRole("button", { name: /filter/i }));
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("toggling an option fires onChange with the new selection", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Controlled facets={[STATUS_FACET]} onChange={onChange} />);
    await user.click(screen.getByRole("button", { name: /filter/i }));
    const items = screen.getAllByRole("menuitemcheckbox");
    await user.click(items[0]);
    expect(onChange).toHaveBeenCalledWith({ status: ["open"] });
  });

  it("toggling an already-selected option removes it", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <Controlled
        facets={[STATUS_FACET]}
        initial={{ status: ["open"] }}
        onChange={onChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: /filter/i }));
    const items = screen.getAllByRole("menuitemcheckbox");
    await user.click(items[0]);
    expect(onChange).toHaveBeenLastCalledWith({});
  });

  it("renders a Clear filters button when any option is selected", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <Controlled
        facets={[STATUS_FACET]}
        initial={{ status: ["open"] }}
        onChange={onChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: /filter/i }));
    const clear = screen.getByRole("button", { name: /clear all/i });
    await user.click(clear);
    expect(onChange).toHaveBeenLastCalledWith({});
  });

  it("shows a search input for facets with more than 8 options", async () => {
    const user = userEvent.setup();
    render(<Controlled facets={[LONG_FACET]} />);
    await user.click(screen.getByRole("button", { name: /filter/i }));
    const search = screen.getByPlaceholderText(/filter cluster…/i);
    expect(search).toBeInTheDocument();
    await user.type(search, "cluster-3");
    const items = screen.getAllByRole("menuitemcheckbox");
    expect(items).toHaveLength(1);
  });

  it("marks selected options with aria-checked=true", async () => {
    const user = userEvent.setup();
    render(
      <Controlled facets={[STATUS_FACET]} initial={{ status: ["open"] }} />,
    );
    await user.click(screen.getByRole("button", { name: /filter/i }));
    const items = screen.getAllByRole("menuitemcheckbox");
    expect(items[0]).toHaveAttribute("aria-checked", "true");
    expect(items[1]).toHaveAttribute("aria-checked", "false");
  });

  it("renders the fetching indicator when isFetching is true", () => {
    render(<Controlled facets={[STATUS_FACET]} isFetching />);
    expect(screen.getByTestId("filter-pill-fetching")).toBeInTheDocument();
  });

  it("omits the fetching indicator when isFetching is false", () => {
    render(<Controlled facets={[STATUS_FACET]} isFetching={false} />);
    expect(screen.queryByTestId("filter-pill-fetching")).toBeNull();
  });
});
