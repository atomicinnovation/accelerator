import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { type SortOption, SortPill } from "./SortPill";

function ControlledPill({
  initial = "recently-modified" as SortOption,
  onChange,
}: {
  initial?: SortOption;
  onChange?: (next: SortOption) => void;
}) {
  const [value, setValue] = useState<SortOption>(initial);
  return (
    <SortPill
      value={value}
      onChange={(next) => {
        setValue(next);
        onChange?.(next);
      }}
    />
  );
}

describe("SortPill", () => {
  it("renders the active option label on the closed trigger", () => {
    render(<ControlledPill />);
    expect(
      screen.getByRole("button", { name: /recently modified/i }),
    ).toBeInTheDocument();
  });

  it("exposes aria-haspopup / -expanded / -controls on the trigger", () => {
    render(<ControlledPill />);
    const trigger = screen.getByRole("button", { name: /recently modified/i });
    expect(trigger).toHaveAttribute("aria-haspopup", "menu");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveAttribute("aria-controls");
  });

  it("opens a menu showing five options", async () => {
    const user = userEvent.setup();
    render(<ControlledPill />);
    await user.click(screen.getByRole("button"));
    expect(screen.getByText(/sort by/i)).toBeInTheDocument();
    expect(screen.getAllByRole("menuitem")).toHaveLength(5);
    expect(screen.getByText("Title (A → Z)")).toBeInTheDocument();
  });

  it("selecting an option fires onChange and closes the menu", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ControlledPill onChange={onChange} />);
    await user.click(screen.getByRole("button"));
    await user.click(screen.getByText("Title (A → Z)"));
    expect(onChange).toHaveBeenCalledWith("title-asc");
    expect(screen.getByRole("button")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("marks the active option with aria-checked=true", async () => {
    const user = userEvent.setup();
    render(<ControlledPill initial="title-asc" />);
    await user.click(screen.getByRole("button"));
    const items = screen.getAllByRole("menuitem");
    const titleAsc = items.find((el) =>
      el.textContent?.includes("Title (A → Z)"),
    );
    expect(titleAsc).toHaveAttribute("aria-checked", "true");
  });
});
