import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { DOC_TYPE_KEYS, WORKFLOW_PIPELINE_STEPS } from "../../api/types";
import {
  DOC_TYPE_HUE,
  RADIUS_TOKENS,
  SPACING_TOKENS,
} from "../../styles/tokens";
import { ICON_NAMES } from "../Icon/Icon";
import { DevDesignSystem } from "./DevDesignSystem";
import { DEV_CHORD_HINT, DEV_SECTIONS } from "./dev-constants";
import {
  type DevActivation,
  DevActivationProvider,
} from "./use-dev-activation";

beforeAll(() => {
  // jsdom does not implement scrollIntoView; the TOC jump uses it.
  Element.prototype.scrollIntoView = vi.fn();
});

function renderPage(overrides: Partial<DevActivation> = {}) {
  const dev: DevActivation = {
    isDevActive: true,
    enterDev: vi.fn(),
    exitDev: vi.fn(),
    toggleDev: vi.fn(),
    getIsDevActive: () => true,
    recordProgrammaticHash: vi.fn(),
    ...overrides,
  };
  // The composites/chrome sections call useWikiLinkResolver (3 useQuery calls),
  // which needs a QueryClient — at runtime DevDesignSystem renders inside
  // RootLayout's provider tree; here we supply a no-retry client (the wiki-link
  // queries stay pending in jsdom, which is fine for presence assertions).
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const result = render(
    <QueryClientProvider client={queryClient}>
      <DevActivationProvider value={dev}>
        <DevDesignSystem />
      </DevActivationProvider>
    </QueryClientProvider>,
  );
  return { ...result, dev };
}

describe("DevDesignSystem chrome", () => {
  it("renders all 24 sections with their ds-<id> anchors", () => {
    const { container } = renderPage();
    const sections = container.querySelectorAll('section[id^="ds-"]');
    expect(sections).toHaveLength(24);
    for (const s of DEV_SECTIONS) {
      expect(container.querySelector(`#ds-${s.id}`)).not.toBeNull();
    }
  });

  it("renders a 24-entry TOC with two-digit numbers", () => {
    const { container } = renderPage();
    const items = container.querySelectorAll('nav a[href^="#"]');
    expect(items).toHaveLength(24);
    expect(container.textContent).toContain("01");
    expect(container.textContent).toContain("24");
  });

  it("binds the marquee + footer hint to DEV_CHORD_HINT, never ⌘⇧D", () => {
    const { container } = renderPage();
    expect(container.textContent).toContain(DEV_CHORD_HINT);
    expect(container.textContent).not.toContain("⌘⇧D");
  });

  it("moves focus to the page heading on activation", () => {
    renderPage();
    expect(
      document.activeElement?.getAttribute("data-dev-focus-anchor"),
    ).not.toBeNull();
  });

  it("marks the overview TOC entry active with aria-current=location on load", () => {
    const { container } = renderPage();
    const overview = container.querySelector('[title="Overview — #overview"]');
    expect(overview?.getAttribute("aria-current")).toBe("location");
  });

  it("moves aria-current to the clicked TOC entry", () => {
    const { container } = renderPage();
    const colours = container.querySelector(
      '[title="Colours — #colors"]',
    ) as HTMLElement;
    fireEvent.click(colours);
    expect(colours.getAttribute("aria-current")).toBe("location");
    const overview = container.querySelector('[title="Overview — #overview"]');
    expect(overview?.getAttribute("aria-current")).toBeNull();
  });

  it("exit-to-app control calls exitDev", () => {
    const { getByRole, dev } = renderPage();
    fireEvent.click(getByRole("button", { name: /exit to app/i }));
    expect(dev.exitDev).toHaveBeenCalledTimes(1);
  });

  it("renders an in-page theme toggle in the chrome", () => {
    // The topbar section also renders a ThemeToggle, so scope to the TOC aside.
    const { container } = renderPage();
    const aside = container.querySelector("aside");
    expect(aside).not.toBeNull();
    expect(
      within(aside as HTMLElement).getByRole("button", { name: /dark theme/i }),
    ).toBeInTheDocument();
  });
});

describe("DevDesignSystem section content — tokens & type (Phase 6)", () => {
  it("overview cards bind their counts to live constants", () => {
    const { getByTestId } = renderPage();
    expect(getByTestId("overview-card-icons").textContent).toContain(
      String(ICON_NAMES.length),
    );
    expect(getByTestId("overview-card-glyphs").textContent).toContain(
      String(DOC_TYPE_KEYS.length),
    );
    expect(getByTestId("overview-card-fonts").textContent).toContain("3");
    expect(getByTestId("overview-card-themes").textContent).toContain("2");
  });

  it("overview renders the deviations aside (the divergences home)", () => {
    const { getByTestId } = renderPage();
    expect(getByTestId("ds-deviations").textContent).toMatch(
      /deviations from the prototype/i,
    );
  });

  it("colours section renders the curated semantic + brand swatch groups", () => {
    const { getByTestId } = renderPage();
    const count = (id: string) =>
      getByTestId(id).querySelectorAll("[data-token]").length;
    expect(count("ds-swatches-surfaces")).toBe(8);
    expect(count("ds-swatches-foreground")).toBe(4);
    expect(count("ds-swatches-accent")).toBe(8);
    expect(count("ds-swatches-stroke")).toBe(3);
    expect(count("ds-swatches-brand")).toBe(19);
    expect(
      getByTestId("ds-swatches-surfaces").querySelector(
        '[data-token="--ac-bg"]',
      ),
    ).not.toBeNull();
  });

  it("doc-type hues bind one chip per DOC_TYPE_HUE entry", () => {
    const { getByTestId } = renderPage();
    const chips =
      getByTestId("ds-typehues").querySelectorAll("[data-doc-type]");
    expect(chips).toHaveLength(Object.keys(DOC_TYPE_HUE).length);
    expect(chips).toHaveLength(DOC_TYPE_KEYS.length);
  });

  it("spacing section binds one bar per SPACING_TOKENS step", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-spacing").querySelectorAll("[data-sp]"),
    ).toHaveLength(Object.keys(SPACING_TOKENS).length);
  });

  it("radii section binds the live radii ladder + the three shadows", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-radii").querySelectorAll("[data-radius]"),
    ).toHaveLength(Object.keys(RADIUS_TOKENS).length);
    expect(
      getByTestId("ds-shadows").querySelectorAll("[data-shadow]"),
    ).toHaveLength(3);
  });

  it("type section renders the seven ramp specimens", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-typesamples").querySelectorAll("[data-type-sample]"),
    ).toHaveLength(7);
  });
});

describe("DevDesignSystem section content — glyphs, mark, icons (Phase 7)", () => {
  it("icons section renders one cell per ICON_NAME", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-icons").querySelectorAll("[data-icon]"),
    ).toHaveLength(ICON_NAMES.length);
  });

  it("doc-type glyphs reproduce the glyph-cell-<type>-<size> contract at 16/24/32/48", () => {
    const { getByTestId, container } = renderPage();
    const grid = getByTestId("ds-glyphs");
    // every doc type renders all four sizes (the migrated VR + resolved-fill
    // contract: the resolved-fill spec reads the 24px cell)
    for (const docType of DOC_TYPE_KEYS) {
      for (const size of [16, 24, 32, 48]) {
        expect(
          grid.querySelector(
            `[data-testid="glyph-cell-${docType}-${size}"] svg`,
          ),
        ).not.toBeNull();
      }
    }
    expect(
      container.querySelectorAll('[data-testid^="glyph-cell-"]'),
    ).toHaveLength(DOC_TYPE_KEYS.length * 4);
  });

  it("empty-state glyphs render a big-glyph-cell-<type> per doc type + a size ramp", () => {
    const { getByTestId } = renderPage();
    const grid = getByTestId("ds-bigglyphs");
    for (const docType of DOC_TYPE_KEYS) {
      expect(
        grid.querySelector(`[data-testid="big-glyph-cell-${docType}"]`),
      ).not.toBeNull();
    }
    expect(
      grid.querySelectorAll('[data-testid^="big-glyph-cell-"]'),
    ).toHaveLength(DOC_TYPE_KEYS.length);
  });

  it("empty-state glyphs render the five-size ramp", () => {
    const { container } = renderPage();
    expect(container.querySelectorAll("[data-big-glyph-size]")).toHaveLength(5);
  });

  it("atomic mark renders five sizes plus an on-night cell", () => {
    const { container } = renderPage();
    expect(container.querySelectorAll("[data-mark]")).toHaveLength(6);
    expect(container.querySelectorAll("[data-mark-night]")).toHaveLength(1);
  });
});

describe("DevDesignSystem section content — interactive primitives (Phase 8)", () => {
  it("chips reproduce the 6×2 chip-cell-<variant>-<size> contract", () => {
    const { getByTestId } = renderPage();
    const grid = getByTestId("ds-chips");
    expect(grid.querySelectorAll('[data-testid^="chip-cell-"]')).toHaveLength(
      12,
    );
    // each cell wraps a live Chip carrying data-variant + data-size
    expect(
      grid.querySelector(
        '[data-testid="chip-cell-indigo-md"] [data-variant="indigo"][data-size="md"]',
      ),
    ).not.toBeNull();
  });

  it("badges route the 12 values through the correct status/verdict/result component", () => {
    const { getByTestId } = renderPage();
    const status = getByTestId("ds-badges-status");
    // the 8 statuses
    expect(
      status.querySelectorAll('[data-testid="status-badge"]'),
    ).toHaveLength(8);
    const verdict = getByTestId("ds-badges-verdict");
    // approve + request-changes → VerdictBadge; approve-with-changes → StatusBadge
    // (amber); pass → ResultBadge
    expect(
      verdict.querySelectorAll('[data-testid="verdict-badge"]'),
    ).toHaveLength(2);
    expect(
      verdict.querySelectorAll('[data-testid="status-badge"]'),
    ).toHaveLength(1);
    expect(
      verdict.querySelectorAll('[data-testid="result-badge"]'),
    ).toHaveLength(1);
    // approve-with-changes resolves to amber via statusToVariant
    expect(
      verdict.querySelector(
        '[data-testid="status-badge"][data-variant="amber"]',
      ),
    ).not.toBeNull();
  });

  it("stage dots render WORKFLOW_PIPELINE_STEPS dots across all/partial/none", () => {
    const { getByTestId } = renderPage();
    const dots = getByTestId("ds-stagedots");
    expect(
      dots.querySelectorAll('[aria-label^="Lifecycle pipeline"]'),
    ).toHaveLength(3);
    expect(dots.querySelectorAll("[data-stage]")).toHaveLength(
      WORKFLOW_PIPELINE_STEPS.length * 3,
    );
    const n = WORKFLOW_PIPELINE_STEPS.length;
    const all = dots.querySelector(
      `[aria-label="Lifecycle pipeline, ${n} of ${n} stages complete"]`,
    );
    expect(all?.querySelectorAll('[data-active="true"]')).toHaveLength(n);
  });

  it("tier pills render the four presence states", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-tierpills").querySelectorAll("[data-tier]"),
    ).toHaveLength(4);
  });

  it("buttons render the seven variants", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-buttons").querySelectorAll("[data-btn]"),
    ).toHaveLength(7);
  });

  it("form renders the search composite + two checkbox rows", () => {
    const { getByTestId } = renderPage();
    expect(getByTestId("ds-form-search")).toBeInTheDocument();
    expect(getByTestId("ds-form-search").querySelector("input")).not.toBeNull();
    expect(document.querySelectorAll("[data-check]")).toHaveLength(2);
  });

  it("sidebar nav renders the six named variants", () => {
    const { getByTestId } = renderPage();
    const nav = getByTestId("ds-nav");
    for (const variant of [
      "label",
      "sublabel",
      "default",
      "active",
      "pulse",
      "faded",
    ]) {
      expect(nav.querySelector(`[data-nav="${variant}"]`)).not.toBeNull();
    }
  });
});

describe("DevDesignSystem section content — composites & chrome (Phase 9)", () => {
  it("cards render the four variants incl. the kanban-card-cell .ac-kcard cells", () => {
    const { getByTestId } = renderPage();
    // kanban: resting / dragging / overlay, each wrapping an .ac-kcard
    for (const state of ["resting", "dragging", "overlay"]) {
      const cell = getByTestId(`kanban-card-cell-${state}`);
      expect(cell.querySelector(".ac-kcard")).not.toBeNull();
    }
    // the other three card variants
    expect(getByTestId("ds-related")).toBeInTheDocument();
    expect(getByTestId("ds-lcard-empty")).toBeInTheDocument();
    expect(getByTestId("ds-table")).toBeInTheDocument();
  });

  it("markdown renders the eight element kinds incl. a wiki-link", () => {
    const { getByTestId } = renderPage();
    const md = getByTestId("ds-markdown");
    expect(md.querySelector("h2")).not.toBeNull();
    expect(md.querySelector("h3")).not.toBeNull();
    expect(md.querySelector("strong")).not.toBeNull();
    expect(md.querySelector("em")).not.toBeNull();
    expect(md.querySelector("code")).not.toBeNull();
    expect(md.querySelector("ul")).not.toBeNull();
    expect(md.querySelector("ol")).not.toBeNull();
    expect(md.querySelector("table")).not.toBeNull();
    // the [[ADR-0001]] wiki-link renders as a wiki-link node (pending in jsdom,
    // a resolved anchor at runtime)
    expect(md.querySelector('[class*="wiki-link"], a')).not.toBeNull();
  });

  it("code blocks render all 8 languages + bash, each with data-language and hljs spans", () => {
    const { getByTestId } = renderPage();
    const code = getByTestId("ds-code");
    for (const lang of [
      "python",
      "typescript",
      "yaml",
      "json",
      "css",
      "html",
      "diff",
      "markdown",
      "bash",
    ]) {
      const cell = getByTestId(`code-syntax-cell-${lang}`);
      expect(cell.querySelector(`[data-language="${lang}"]`)).not.toBeNull();
    }
    // rehype-highlight emits hljs token spans (the resolved-colour spec asserts
    // these resolve to the syntax tokens)
    expect(
      code.querySelector(
        '[data-testid="code-syntax-cell-python"] [class*="hljs-"]',
      ),
    ).not.toBeNull();
  });

  it("frontmatter renders key/value rows incl. a referenced value", () => {
    const { getByTestId } = renderPage();
    const fm = getByTestId("ds-frontmatter");
    expect(fm.querySelectorAll("dt").length).toBeGreaterThan(0);
    expect(fm.querySelectorAll("dd").length).toBeGreaterThan(0);
    // the related ADR id renders (a resolved anchor at runtime; pending/plain in
    // jsdom — either way the reference text is present)
    expect(fm.textContent).toContain("ADR-0001");
  });

  it("empty + banner render two demos", () => {
    const { getByTestId } = renderPage();
    expect(getByTestId("ds-empty")).toBeInTheDocument();
    expect(getByTestId("ds-banner")).toBeInTheDocument();
  });

  it("toasts render three dismissible demos", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-toasts").querySelectorAll("[data-toast]"),
    ).toHaveLength(3);
  });

  it("topbar renders the five parts", () => {
    const { getByTestId } = renderPage();
    expect(
      getByTestId("ds-topbar").querySelectorAll("[data-topbar-part]"),
    ).toHaveLength(5);
  });
});

describe("VR cell-presence gate (Phase 10)", () => {
  // Coverage-preservation enforced by CI: every data-testid cell the migrated
  // visual-regression specs clip must exist on the page, with PINNED dimensions
  // (a glyph size drift, a dropped chip size, or a missing code language fails
  // here rather than slipping past the row-by-row audit).
  it("renders every required migrated VR cell", () => {
    const { container } = renderPage();
    const required: string[] = [];
    for (const docType of DOC_TYPE_KEYS) {
      for (const size of [16, 24, 32, 48]) {
        required.push(`glyph-cell-${docType}-${size}`);
      }
      required.push(`big-glyph-cell-${docType}`);
    }
    for (const variant of [
      "neutral",
      "indigo",
      "green",
      "amber",
      "red",
      "violet",
    ]) {
      for (const size of ["sm", "md"]) {
        required.push(`chip-cell-${variant}-${size}`);
      }
    }
    for (const lang of [
      "python",
      "typescript",
      "yaml",
      "json",
      "css",
      "html",
      "diff",
      "markdown",
      "bash",
    ]) {
      required.push(`code-syntax-cell-${lang}`);
    }
    for (const state of ["resting", "dragging", "overlay"]) {
      required.push(`kanban-card-cell-${state}`);
    }
    const missing = required.filter(
      (id) => container.querySelector(`[data-testid="${id}"]`) === null,
    );
    expect(missing).toEqual([]);
  });
});
