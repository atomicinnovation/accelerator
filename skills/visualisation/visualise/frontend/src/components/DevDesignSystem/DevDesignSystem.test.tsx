import { fireEvent, render } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
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
});
