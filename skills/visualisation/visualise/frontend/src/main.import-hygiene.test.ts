import { describe, expect, it } from "vitest";
import mainSource from "./main.tsx?raw";

describe("main.tsx import hygiene", () => {
  it("does not import highlight.js/styles/github.css (replaced by code-syntax.global.css in story 0076)", () => {
    expect(mainSource).not.toMatch(/highlight\.js\/styles\/github\.css/);
  });

  it("imports code-syntax.global.css after global.css (load-order contract)", () => {
    const globalIdx = mainSource.indexOf('import "./styles/global.css"');
    const syntaxIdx = mainSource.indexOf(
      'import "./styles/code-syntax.global.css"',
    );
    expect(globalIdx).toBeGreaterThanOrEqual(0);
    expect(syntaxIdx).toBeGreaterThan(globalIdx);
  });
});
