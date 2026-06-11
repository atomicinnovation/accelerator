import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Chip } from "./Chip";
import chipCss from "./Chip.module.css?raw";

const Q = `['"]`;

describe("Chip", () => {
  describe("rendering", () => {
    it("renders its children", () => {
      render(<Chip variant="neutral">Done</Chip>);
      expect(screen.getByText("Done")).toBeInTheDocument();
    });

    it("renders the leading slot before the children", () => {
      render(
        <Chip variant="indigo" leading={<span data-testid="lead" />}>
          live
        </Chip>,
      );
      const lead = screen.getByTestId("lead");
      const text = screen.getByText("live");
      expect(
        lead.compareDocumentPosition(text) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    });

    it.each([
      ["undefined", undefined],
      ["null", null],
      ["false", false],
    ] as const)("does not render a leading wrapper when leading=%s", (_label, value) => {
      const { container } = render(
        <Chip variant="neutral" leading={value as never}>
          x
        </Chip>,
      );
      expect(container.querySelector('[data-slot="leading"]')).toBeNull();
    });

    it("forwards aria-label to the chip element", () => {
      const { container } = render(
        <Chip variant="green" aria-label="status: accepted">
          accepted
        </Chip>,
      );
      expect(
        container.querySelector('[aria-label="status: accepted"]'),
      ).not.toBeNull();
    });
  });

  describe("data-testid forwarding", () => {
    it("forwards data-testid to the root element", () => {
      const { container } = render(
        <Chip variant="neutral" data-testid="status-badge">
          x
        </Chip>,
      );
      expect(
        container.querySelector('[data-testid="status-badge"]'),
      ).not.toBeNull();
    });

    it("omits the data-testid attribute when none is passed", () => {
      const { container } = render(<Chip variant="neutral">x</Chip>);
      expect(container.querySelector("[data-testid]")).toBeNull();
    });
  });

  describe("variants", () => {
    it.each([
      ["neutral"],
      ["indigo"],
      ["green"],
      ["amber"],
      ["red"],
      ["violet"],
    ] as const)("renders variant=%s with the matching data-variant attribute", (variant) => {
      const { container } = render(<Chip variant={variant}>x</Chip>);
      expect(
        container.querySelector(`[data-variant="${variant}"]`),
      ).not.toBeNull();
    });
  });

  describe("sizes", () => {
    it("defaults to size sm", () => {
      const { container } = render(<Chip variant="neutral">x</Chip>);
      expect(container.querySelector('[data-size="sm"]')).not.toBeNull();
    });

    it.each([
      ["sm"],
      ["md"],
    ] as const)("renders size=%s with the matching data-size attribute", (size) => {
      const { container } = render(
        <Chip variant="neutral" size={size}>
          x
        </Chip>,
      );
      expect(container.querySelector(`[data-size="${size}"]`)).not.toBeNull();
    });
  });

  describe("CSS source assertions", () => {
    it("binds base font-family to --ac-font-mono", () => {
      expect(chipCss).toMatch(
        /\.chip\s*\{[^}]*font-family:\s*var\(--ac-font-mono\)/,
      );
    });
    it("binds base border-radius to --radius-pill", () => {
      expect(chipCss).toMatch(
        /\.chip\s*\{[^}]*border-radius:\s*var\(--radius-pill\)/,
      );
    });
    it("binds base font-size to --size-3xs-lg", () => {
      expect(chipCss).toMatch(
        /\.chip\s*\{[^}]*font-size:\s*var\(--size-3xs-lg\)/,
      );
    });

    it(`[data-variant=…neutral…] binds color to --ac-fg-muted`, () => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}neutral${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-fg-muted\\)`,
        ),
      );
    });
    it("[data-variant=indigo] binds color to --ac-accent", () => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}indigo${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-accent\\)`,
        ),
      );
    });
    it("[data-variant=green] binds color to --ac-ok", () => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}green${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-ok\\)`,
        ),
      );
    });
    it("[data-variant=amber] binds color to --ac-warn", () => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}amber${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-warn\\)`,
        ),
      );
    });
    it("[data-variant=red] binds color to --ac-err", () => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}red${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-err\\)`,
        ),
      );
    });
    it("[data-variant=violet] binds color to --ac-violet", () => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}violet${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-violet\\)`,
        ),
      );
    });

    it.each([
      ["green", "--ac-ok"],
      ["amber", "--ac-warn"],
      ["red", "--ac-err"],
      ["violet", "--ac-violet"],
    ] as const)("[data-variant=%s] background composes at 8%% against --ac-bg", (variant, token) => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}${variant}${Q}\\][^{]*\\{[^}]*background:\\s*color-mix\\(\\s*in\\s+srgb\\s*,\\s*var\\(\\${token}\\)\\s+8%\\s*,\\s*var\\(--ac-bg\\)\\s*\\)`,
        ),
      );
    });

    it.each([
      ["green", "--ac-ok"],
      ["amber", "--ac-warn"],
      ["red", "--ac-err"],
      ["violet", "--ac-violet"],
    ] as const)("[data-variant=%s] border-color composes at 30%% against --ac-bg", (variant, token) => {
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-variant=${Q}${variant}${Q}\\][^{]*\\{[^}]*border-color:\\s*color-mix\\(\\s*in\\s+srgb\\s*,\\s*var\\(\\${token}\\)\\s+30%\\s*,\\s*var\\(--ac-bg\\)\\s*\\)`,
        ),
      );
    });

    it("[data-size=md] overrides padding and font-size", () => {
      expect(chipCss).toMatch(
        new RegExp(`\\[data-size=${Q}md${Q}\\][^{]*\\{[^}]*padding:`),
      );
      expect(chipCss).toMatch(
        new RegExp(
          `\\[data-size=${Q}md${Q}\\][^{]*\\{[^}]*font-size:\\s*var\\(--size-xxs-sm\\)`,
        ),
      );
    });
  });
});
