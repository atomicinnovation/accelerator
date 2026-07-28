import { describe, expect, it } from "vitest";
import css from "./code-syntax.global.css?raw";
import { assertSelectorColorIs, selectorOffset } from "./testing/cssRules";

const REQUIRED_MAPPINGS: ReadonlyArray<[selector: string, token: string]> = [
  [".hljs-comment", "tk-com"],
  [".hljs-quote", "tk-com"],
  [".hljs-code", "tk-com"],
  [".hljs-bullet", "tk-com"],
  [".hljs-string", "tk-str"],
  [".hljs-number", "tk-num"],
  [".hljs-keyword", "tk-kw"],
  [".hljs-literal", "tk-lit"],
  [".hljs-type", "tk-typ"],
  [".hljs-class", "tk-typ"],
  [".hljs-function", "tk-fn"],
  [".hljs-title.function_", "tk-fn"],
  [".hljs-attr", "tk-attr"],
  [".hljs-attribute", "tk-attr"],
  [".hljs-meta", "tk-deco"],
  [".hljs-built_in", "tk-bn"],
  [".hljs-variable", "tk-var"],
  [".hljs-template-variable", "tk-var"],
  [".hljs-property", "tk-prop"],
  [".hljs-selector-class", "tk-sel"],
  [".hljs-selector-id", "tk-sel"],
  [".hljs-selector-tag", "tk-sel"],
  [".hljs-selector-pseudo", "tk-sel"],
  [".hljs-tag", "tk-tag"],
  [".hljs-name", "tk-tag"],
  [".hljs-section", "tk-header"],
  [".hljs-link", "tk-anchor"],
  [".hljs-symbol", "tk-anchor"],
  [".hljs-punctuation", "tk-pun"],
  [".hljs-addition", "tk-dadd"],
  [".hljs-diff-added", "tk-dadd"],
  [".hljs-deletion", "tk-ddel"],
  [".hljs-diff-deleted", "tk-ddel"],
  [".language-diff .hljs-meta", "tk-dhdr"],
  [".language-diff .hljs-comment", "tk-dhunk"],
];

describe("code-syntax.global.css", () => {
  it("declares a .hljs base rule resetting background to transparent", () => {
    expect(css).toMatch(/\.hljs\s*\{[^}]*background:\s*transparent/);
    expect(css).toMatch(/\.hljs\s*\{[^}]*color:\s*inherit/);
  });

  for (const [selector, token] of REQUIRED_MAPPINGS) {
    it(`${selector} → var(--${token})`, () => {
      assertSelectorColorIs(css, selector, token);
    });
  }

  it(".hljs-section declares font-weight: 600 (preserved from templates preview)", () => {
    expect(css).toMatch(/\.hljs-section[^{]*\{[^}]*font-weight:\s*600/);
  });

  it(".hljs-emphasis declares font-style: italic (no colour override — inherits surrounding fg)", () => {
    expect(css).toMatch(/\.hljs-emphasis\s*\{[^}]*font-style:\s*italic/);
    const block = /\.hljs-emphasis\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(block).not.toMatch(/color\s*:/);
  });

  it(".hljs-strong declares font-weight: 600 (no colour override)", () => {
    expect(css).toMatch(/\.hljs-strong\s*\{[^}]*font-weight:\s*600/);
    const block = /\.hljs-strong\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(block).not.toMatch(/color\s*:/);
  });

  it(".language-diff .hljs-meta rule appears AFTER the general .hljs-meta rule in source", () => {
    const general = selectorOffset(css, ".hljs-meta");
    const override = selectorOffset(css, ".language-diff .hljs-meta");
    expect(general).not.toBeNull();
    expect(override).not.toBeNull();
    expect(override!).toBeGreaterThan(general!);
  });

  it(".language-diff .hljs-comment rule appears AFTER the general .hljs-comment rule in source", () => {
    const general = selectorOffset(css, ".hljs-comment");
    const override = selectorOffset(css, ".language-diff .hljs-comment");
    expect(general).not.toBeNull();
    expect(override).not.toBeNull();
    expect(override!).toBeGreaterThan(general!);
  });
});
