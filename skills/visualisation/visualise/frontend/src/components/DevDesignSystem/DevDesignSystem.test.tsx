import { fireEvent, render } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { DOC_TYPE_KEYS } from "../../api/types";
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
  const result = render(
    <DevActivationProvider value={dev}>
      <DevDesignSystem />
    </DevActivationProvider>,
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
    const { getByRole } = renderPage();
    expect(getByRole("button", { name: /dark theme/i })).toBeInTheDocument();
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
