import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Page } from "./Page";
import pageCss from "./Page.module.css?raw";

describe("Page", () => {
  it("renders the required title as an h1", () => {
    render(
      <Page title="Library">
        <p>body</p>
      </Page>,
    );
    expect(
      screen.getByRole("heading", { name: "Library", level: 1 }),
    ).toBeInTheDocument();
  });

  it("renders children inside the content area", () => {
    render(
      <Page title="Library">
        <p>body</p>
      </Page>,
    );
    expect(screen.getByText("body")).toBeInTheDocument();
  });

  it("renders the eyebrow when provided", () => {
    render(
      <Page eyebrow="LIBRARY" title="Library">
        x
      </Page>,
    );
    expect(screen.getByText("LIBRARY")).toBeInTheDocument();
  });

  it("omits the eyebrow slot when not provided", () => {
    const { container } = render(<Page title="Library">x</Page>);
    expect(container.querySelector('[data-slot="eyebrow"]')).toBeNull();
  });

  it("renders the subtitle when provided", () => {
    render(
      <Page title="Library" subtitle="12 documents">
        x
      </Page>,
    );
    expect(screen.getByText("12 documents")).toBeInTheDocument();
  });

  it("omits the subtitle slot when not provided", () => {
    const { container } = render(<Page title="Library">x</Page>);
    expect(container.querySelector('[data-slot="subtitle"]')).toBeNull();
  });

  it("renders the actions slot when provided", () => {
    render(
      <Page title="Library" actions={<button type="button">Action</button>}>
        x
      </Page>,
    );
    expect(screen.getByRole("button", { name: "Action" })).toBeInTheDocument();
  });

  it("omits the actions slot when not provided", () => {
    const { container } = render(<Page title="Library">x</Page>);
    expect(container.querySelector('[data-slot="actions"]')).toBeNull();
  });

  it("renders a divider between header and content", () => {
    const { container } = render(<Page title="Library">x</Page>);
    expect(container.querySelector("hr")).not.toBeNull();
  });

  it("binds default max-width to --ac-content-max-width via CSS module source", () => {
    expect(pageCss).toMatch(
      /\.page\s*\{[^}]*max-width:\s*var\(--ac-content-max-width\)/,
    );
  });

  it("binds the narrow variant to --ac-content-max-width-narrow", () => {
    expect(pageCss).toMatch(
      /\.page\.narrow\s*\{[^}]*max-width:\s*var\(--ac-content-max-width-narrow\)/,
    );
  });

  it("applies horizontal padding via --sp-7 (40px) — gives 1120px content at 1200px max-width", () => {
    expect(pageCss).toMatch(/\.page\s*\{[^}]*padding:\s*0\s+var\(--sp-7\)/);
  });

  it('applies the narrow class only when maxWidth="narrow"', () => {
    const { container, rerender } = render(<Page title="t">x</Page>);
    const section = container.querySelector("section")!;
    expect(section.className).not.toMatch(/narrow/);
    rerender(
      <Page title="t" maxWidth="narrow">
        x
      </Page>,
    );
    expect(container.querySelector("section")!.className).toMatch(/narrow/);
  });

  it("has bottom padding so page content does not touch the viewport edge", () => {
    // CSS regression: the .page rule must declare a non-zero bottom
    // padding so the last content block has breathing room. Anchored to
    // the .page selector specifically — a future refactor that moves
    // the padding into a wrapper must also keep this guard happy.
    expect(pageCss).toMatch(
      /\.page\s*\{[^}]*padding:[^;}]*var\(--sp-[0-9]+\)\s*;?\s*\}/m,
    );
  });
});
