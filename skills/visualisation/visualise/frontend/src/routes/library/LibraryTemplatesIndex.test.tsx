import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import type React from "react";
import { describe, expect, it, vi } from "vitest";
import * as fetchModule from "../../api/fetch";
import type { TemplateSummary } from "../../api/types";
import { MemoryRouter } from "../../test/router-helpers";
import { LibraryTemplatesIndex } from "./LibraryTemplatesIndex";
import indexCss from "./LibraryTemplatesIndex.module.css?raw";
import { glyphKeyForTemplate } from "./template-tier";

const mockTemplates: TemplateSummary[] = [
  {
    name: "adr",
    activeTier: "plugin-default",
    tiers: [
      { source: "config-override", path: "/x", present: false, active: false },
      { source: "user-override", path: "/y", present: false, active: false },
      { source: "plugin-default", path: "/z", present: true, active: true },
    ],
  },
  {
    name: "plan",
    activeTier: "plugin-default",
    tiers: [
      { source: "config-override", path: "/x", present: false, active: false },
      { source: "user-override", path: "/y", present: false, active: false },
      { source: "plugin-default", path: "/z", present: true, active: true },
    ],
  },
];

const mockWithVariety: TemplateSummary[] = [
  {
    name: "adr",
    activeTier: "plugin-default",
    tiers: [
      { source: "config-override", path: "/x", present: false, active: false },
      { source: "user-override", path: "/y", present: false, active: false },
      { source: "plugin-default", path: "/z", present: true, active: true },
    ],
  },
  {
    name: "plan",
    activeTier: "user-override",
    tiers: [
      { source: "config-override", path: "/x", present: false, active: false },
      { source: "user-override", path: "/y", present: true, active: true },
      { source: "plugin-default", path: "/z", present: true, active: false },
    ],
  },
  {
    name: "research",
    activeTier: "config-override",
    tiers: [
      { source: "config-override", path: "/x", present: true, active: true },
      { source: "user-override", path: "/y", present: true, active: false },
      { source: "plugin-default", path: "/z", present: true, active: false },
    ],
  },
];

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  );
}

describe("glyphKeyForTemplate", () => {
  it("maps exact template names to their doc-type glyph", () => {
    expect(glyphKeyForTemplate("adr")).toBe("decisions");
    expect(glyphKeyForTemplate("research")).toBe("research");
    expect(glyphKeyForTemplate("plan")).toBe("plans");
    expect(glyphKeyForTemplate("validation")).toBe("validations");
    expect(glyphKeyForTemplate("pr-description")).toBe("pr-descriptions");
  });

  it("falls back to a matching stem inside compound template names", () => {
    expect(glyphKeyForTemplate("codebase-research")).toBe("research");
    expect(glyphKeyForTemplate("feature-plan")).toBe("plans");
    expect(glyphKeyForTemplate("something-decision")).toBe("decisions");
  });

  it("returns null when no stem matches", () => {
    expect(glyphKeyForTemplate("totally-unknown")).toBeNull();
  });

  it("maps rca to the glyph-only root-cause-analyses key", () => {
    // `root-cause-analyses` is not a server DocTypeKey — it is a glyph-only
    // key so the rca template row shows the prototype's RCA glyph instead of
    // the blank fallback.
    expect(glyphKeyForTemplate("rca")).toBe("root-cause-analyses");
  });
});

describe("LibraryTemplatesIndex", () => {
  it('renders the page title "Templates" with a "TEMPLATES" eyebrow', async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockTemplates,
    });
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    expect(
      await screen.findByRole("heading", { name: /^Templates$/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/^TEMPLATES$/)).toBeInTheDocument();
    // VIRTUAL marker has been dropped — make sure it doesn't reappear.
    expect(screen.queryByText(/VIRTUAL/i)).toBeNull();
  });

  it("renders the long-form subtitle below the title", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockTemplates,
    });
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    await screen.findByRole("heading", { name: /Templates/i });
    expect(
      screen.getByText(/The starting shape for every new doc\./i),
    ).toBeInTheDocument();
  });

  it("renders a clickable row for each template, with the filename in monospace", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockTemplates,
    });
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    expect(
      await screen.findByRole("link", { name: /adr\.md/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /plan\.md/i })).toBeInTheDocument();
  });

  it("renders a framed doc-type glyph svg per row", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockTemplates,
    });
    const { container } = render(<LibraryTemplatesIndex />, {
      wrapper: Wrapper,
    });
    await screen.findByRole("link", { name: /adr\.md/i });
    // Framed glyph wraps the <svg> in a <span data-doc-type="..."> with a
    // tinted background; verify both that the wrapper exists and the svg
    // is inside it.
    const frame = container.querySelector('span[data-doc-type="decisions"]');
    expect(frame).not.toBeNull();
    expect(
      frame?.querySelector('svg[data-doc-type="decisions"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('span[data-doc-type="plans"]'),
    ).not.toBeNull();
  });

  it("shows loading state while fetching", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockImplementation(
      () =>
        new Promise(() => {
          /* pending forever */
        }),
    );
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    expect(await screen.findByText(/Loading…/i)).toBeInTheDocument();
  });

  it("renders an error alert when fetchTemplates rejects", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockRejectedValue(
      new Error("boom"),
    );
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    expect(await screen.findByRole("alert")).toHaveTextContent(
      /Failed to load templates/i,
    );
  });

  it("renders three tier pills per row in the fixed order default → user → config", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockWithVariety,
    });
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    await screen.findByRole("link", { name: /adr\.md/i });
    for (const name of ["adr", "plan", "research"]) {
      const row = screen.getByRole("link", {
        name: new RegExp(`${name}\\.md`),
      });
      const labels = within(row).getAllByText(/^(default|user|config)$/);
      expect(labels.map((c) => c.textContent)).toEqual([
        "default",
        "user",
        "config",
      ]);
    }
  });

  it("renders the inter-pill separator and row disclosure as right-chevron SVGs", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockTemplates,
    });
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    const row = await screen.findByRole("link", { name: /adr\.md/i });
    // Three pills → two inter-pill separators; +1 disclosure chevron on the row.
    // All four chevrons are SVG <path d="m9 6 6 6-6 6"/>.
    const chevronPaths = row.querySelectorAll('svg path[d="m9 6 6 6-6 6"]');
    expect(chevronPaths.length).toBe(3);
    // No legacy text-based "→"/"›" remnants.
    expect(within(row).queryAllByText("→")).toEqual([]);
    expect(within(row).queryAllByText("›")).toEqual([]);
  });

  it("renders a bullet shape (no plus sign) as the leading icon on each tier pill", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockTemplates,
    });
    const { container } = render(<LibraryTemplatesIndex />, {
      wrapper: Wrapper,
    });
    await screen.findByRole("link", { name: /adr\.md/i });
    // Bullet is a styled <span/> per pill — three pills × two rows.
    // Use a class-presence check rather than a glyph-text match because
    // the bullet is now a CSS circle rather than a literal "•".
    expect(container.querySelectorAll(`[class*="tierPillBullet"]`).length).toBe(
      6,
    );
    // The legacy "+" leading is gone.
    expect(screen.queryAllByText("+")).toEqual([]);
  });

  it("maps tier (active, present, absent) state to data-state on the pill", async () => {
    vi.spyOn(fetchModule, "fetchTemplates").mockResolvedValue({
      templates: mockWithVariety,
    });
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper });
    await screen.findByRole("link", { name: /adr\.md/i });
    const stateFor = (
      rowName: string,
      label: "default" | "user" | "config",
    ) => {
      const row = screen.getByRole("link", {
        name: new RegExp(`${rowName}\\.md`),
      });
      return within(row)
        .getByText(label)
        .closest("[data-state]")!
        .getAttribute("data-state");
    };
    expect(stateFor("adr", "default")).toBe("active");
    expect(stateFor("adr", "user")).toBe("absent");
    expect(stateFor("adr", "config")).toBe("absent");
    expect(stateFor("plan", "default")).toBe("present");
    expect(stateFor("plan", "user")).toBe("active");
    expect(stateFor("plan", "config")).toBe("absent");
    expect(stateFor("research", "default")).toBe("present");
    expect(stateFor("research", "user")).toBe("present");
    expect(stateFor("research", "config")).toBe("active");
  });

  it("rows in the list share borders rather than gap-separated cards", () => {
    // The connected-table look-and-feel is driven by `border-top` on each
    // row in a shared container, not per-row `border` + `gap`. Anchor a
    // CSS regression test so a future refactor that re-introduces gap
    // styling fails loudly.
    expect(indexCss).toMatch(/\.row\s*\{[^}]*border-top:/m);
    expect(indexCss).not.toMatch(/\.list\s*\{[^}]*gap:/m);
  });

  it("tier pills column-align across rows via subgrid (.list owns the column tracks)", () => {
    // Cross-row alignment requires the column tracks to live on the
    // outer container so all rows share them. Anchor a CSS regression
    // so a future refactor that moves the tracks back onto per-row
    // grids (which would let the name column drift) fails loudly.
    expect(indexCss).toMatch(/\.list\s*\{[^}]*display:\s*grid/m);
    expect(indexCss).toMatch(/\.list\s*\{[^}]*grid-template-columns:/m);
    expect(indexCss).toMatch(
      /\.row\s*\{[^}]*grid-template-columns:\s*subgrid/m,
    );
    expect(indexCss).toMatch(
      /\.rowLink\s*\{[^}]*grid-template-columns:\s*subgrid/m,
    );
  });

  it("CSS module no longer defines legacy .winning or .active rules", () => {
    expect(indexCss).not.toMatch(/\.winning\b/);
    expect(indexCss).not.toMatch(/\.active\b/);
  });
});
