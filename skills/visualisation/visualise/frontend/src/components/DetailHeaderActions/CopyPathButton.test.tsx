import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CopyPathButton } from "./CopyPathButton";

const copyText = vi.fn();
const showToast = vi.fn();

vi.mock("../../api/clipboard", () => ({
  copyText: (text: string) => copyText(text),
}));
vi.mock("../../api/use-toast", () => ({
  useToast: () => ({ showToast }),
}));

beforeEach(() => {
  copyText.mockReset();
  showToast.mockReset();
});

describe("CopyPathButton", () => {
  it("copies the raw relPath (no scheme/host/encoding) and shows an ok toast", async () => {
    copyText.mockResolvedValue(true);
    render(
      <CopyPathButton relPath="meta/work/0080-detail-page-header-actions.md" />,
    );

    const btn = screen.getByRole("button", { name: "Copy path" });
    // The label is rendered as visible text, not just an accessible name.
    expect(btn).toHaveTextContent("Copy path");
    fireEvent.click(btn);

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(copyText).toHaveBeenCalledWith(
      "meta/work/0080-detail-page-header-actions.md",
    );
    const arg = showToast.mock.calls[0][0];
    expect(arg.kind).toBe("ok");
    // The message wraps the raw path in backticks so the Toaster renders it as <code>.
    expect(arg.message).toBe("`meta/work/0080-detail-page-header-actions.md`");
  });

  it("shows an error toast (not the success toast) when the copy fails", async () => {
    copyText.mockResolvedValue(false);
    render(<CopyPathButton relPath="a/b.md" />);

    fireEvent.click(screen.getByRole("button", { name: "Copy path" }));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    const arg = showToast.mock.calls[0][0];
    expect(arg.kind).toBe("error");
    expect(showToast.mock.calls.every((c) => c[0].kind !== "ok")).toBe(true);
  });
});
