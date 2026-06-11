import { describe, expect, it } from "vitest";
import { extractAllAcDeclarations } from "./extractAcDeclarations";

describe("extractAllAcDeclarations", () => {
  it("returns one entry per declaration across all three blocks tagged correctly", () => {
    const css = `
      :root {
        --ac-bg: #fff;
        --ac-fg: #000;
      }
      [data-theme="dark"] {
        --ac-bg: #000;
      }
      @media (prefers-color-scheme: dark) {
        :root:not([data-theme="light"]) {
          --ac-bg: #000;
        }
      }
    `;
    const decls = extractAllAcDeclarations(css);
    expect(decls).toEqual([
      { name: "ac-bg", value: "#fff", block: "root" },
      { name: "ac-fg", value: "#000", block: "root" },
      { name: "ac-bg", value: "#000", block: "data-dark" },
      { name: "ac-bg", value: "#000", block: "media-dark" },
    ]);
  });

  it("skips non-`--ac-*` declarations", () => {
    const css = `
      :root {
        --ac-bg: #fff;
        --atomic-night: rgb(14, 15, 25);
        --tk-com: #6f7796;
        --size-md: 18px;
      }
    `;
    const decls = extractAllAcDeclarations(css);
    expect(decls).toEqual([{ name: "ac-bg", value: "#fff", block: "root" }]);
  });

  it("handles values with embedded parentheses (e.g. var(...) and rgba(...))", () => {
    const css = `
      :root {
        --ac-bg: var(--atomic-bone);
        --ac-stroke: rgba(32, 34, 49, 0.10);
      }
    `;
    const decls = extractAllAcDeclarations(css);
    expect(decls).toEqual([
      { name: "ac-bg", value: "var(--atomic-bone)", block: "root" },
      { name: "ac-stroke", value: "rgba(32, 34, 49, 0.10)", block: "root" },
    ]);
  });

  it("returns empty when no blocks are present", () => {
    expect(extractAllAcDeclarations("body { color: red; }")).toEqual([]);
  });
});
