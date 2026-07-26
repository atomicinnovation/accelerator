import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EditorConfig } from "../../api/types";
import { OpenInEditorButton } from "./OpenInEditorButton";

let editorData: EditorConfig | undefined;

vi.mock("../../api/use-editor-config", () => ({
  useEditorConfig: () => ({ data: editorData }),
}));

beforeEach(() => {
  editorData = undefined;
});

const PATHS = { absPath: "/Users/x/a b.md", relPath: "sub dir/a b.md" };

describe("OpenInEditorButton — configured", () => {
  it('renders an <a rel="noopener noreferrer"> with the computed href', () => {
    editorData = { editor: "vscode", editorProject: "myrepo" };
    render(<OpenInEditorButton {...PATHS} />);
    const link = screen.getByRole("link", { name: "Open in editor" });
    // The label is rendered as visible text, not just an accessible name.
    expect(link).toHaveTextContent("Open in editor");
    expect(link).toHaveAttribute("href", "vscode://file/Users/x/a%20b.md");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
  });

  it("renders the JetBrains href with the resolved project name", () => {
    editorData = { editor: "web-storm", editorProject: "myrepo" };
    render(<OpenInEditorButton {...PATHS} />);
    expect(
      screen.getByRole("link", { name: "Open in editor" }),
    ).toHaveAttribute(
      "href",
      "jetbrains://web-storm/navigate/reference?project=myrepo&path=sub%20dir/a%20b.md",
    );
  });
});

describe("OpenInEditorButton — unconfigured (disabled)", () => {
  beforeEach(() => {
    editorData = { editor: null, editorProject: "repo" };
  });

  it("renders a focusable aria-disabled button (NOT native disabled) wired to a description", () => {
    render(<OpenInEditorButton {...PATHS} />);
    const btn = screen.getByRole("button", { name: "Open in editor" });
    expect(btn).toHaveAttribute("aria-disabled", "true");
    expect(btn).not.toBeDisabled();
    btn.focus();
    expect(btn).toHaveFocus();
    const descId = btn.getAttribute("aria-describedby");
    expect(descId).toBeTruthy();
    expect(document.getElementById(descId!)).not.toBeNull();
  });

  it("names visualiser.editor in both the title and the description", () => {
    render(<OpenInEditorButton {...PATHS} />);
    const btn = screen.getByRole("button", { name: "Open in editor" });
    expect(btn.getAttribute("title")).toContain("visualiser.editor");
    const desc = document.getElementById(btn.getAttribute("aria-describedby")!);
    expect(desc?.textContent).toContain("visualiser.editor");
  });

  it("also renders disabled while the query is still loading (data undefined)", () => {
    editorData = undefined;
    render(<OpenInEditorButton {...PATHS} />);
    const btn = screen.getByRole("button", { name: "Open in editor" });
    expect(btn).toHaveAttribute("aria-disabled", "true");
    expect(btn.getAttribute("title")).toContain("visualiser.editor");
  });
});

describe("OpenInEditorButton — configured but unrecognised (disabled)", () => {
  it("says the value was not recognised and echoes the offending value", () => {
    editorData = { editor: "notaneditor", editorProject: "repo" };
    render(<OpenInEditorButton {...PATHS} />);
    const btn = screen.getByRole("button", { name: "Open in editor" });
    expect(btn).toHaveAttribute("aria-disabled", "true");
    const title = btn.getAttribute("title") ?? "";
    expect(title).toContain("not recognised");
    expect(title).toContain("notaneditor");
  });

  it("truncates a long offending value in the echoed hint", () => {
    const long = "x".repeat(60);
    editorData = { editor: long, editorProject: "repo" };
    render(<OpenInEditorButton {...PATHS} />);
    const title =
      screen
        .getByRole("button", { name: "Open in editor" })
        .getAttribute("title") ?? "";
    expect(title).toContain("not recognised");
    expect(title).toContain(`${"x".repeat(40)}…`);
    expect(title).not.toContain("x".repeat(41));
  });
});
