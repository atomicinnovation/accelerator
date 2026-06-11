import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import prototypeTokens from "./fixtures/prototype-tokens.json";
import { canonicaliseBrand } from "./testing/canonicaliseBrand";
import { extractBlockBody } from "./testing/cssBlocks";

// Drift detector: the committed fixture must remain byte-equivalent
// (case- and whitespace-normalised) to the prototype's .ac-codeblock
// and :root brand-palette blocks. If the prototype changes a colour,
// this spec surfaces the drift instead of silently accepting it. Four
// `..` walks from `skills/visualisation/visualise/frontend/` reach the
// repo root; mirrors the cwd-relative pattern in fonts.test.ts.
const PROTOTYPE_PATH = resolve(
  process.cwd(),
  "..",
  "..",
  "..",
  "..",
  "meta",
  "research",
  "design-inventories",
  "2026-05-21-015231-claude-design-prototype",
  "prototype-standalone.html",
);
const source = readFileSync(PROTOTYPE_PATH, "utf-8");

// Extract the `.ac-codeblock { ... }` block via brace-balanced
// scanning so nested rules (if any) do not truncate the match.
function extractAcCodeblockBlock(html: string): string {
  const sel = ".ac-codeblock";
  let i = 0;
  while (i < html.length) {
    const idx = html.indexOf(sel, i);
    if (idx === -1) break;
    // The matched substring must be followed by whitespace then `{`
    // (not by `__head` or another suffix).
    const after = html.slice(idx + sel.length);
    const m = /^\s*\{/.exec(after);
    if (m) {
      const body = extractBlockBody(html, idx + sel.length);
      if (body !== undefined) return body;
    }
    i = idx + sel.length;
  }
  throw new Error(
    "extractAcCodeblockBlock: could not locate .ac-codeblock rule",
  );
}

// The prototype has TWO top-level (non-`@media`) `:root` blocks: the
// first declares `--atomic-*` (the brand palette) and the second
// declares `--ac-*` light defaults. Disambiguation is by **content**:
// returns the first `:root { ... }` block whose body contains
// `--atomic-night:`. This makes selection resilient to future
// reordering of the prototype's `:root` blocks and produces a precise
// failure mode ("brand block not found") if the prototype is
// restructured.
function extractRootBlockBody(html: string): string | undefined {
  const re = /:root\s*\{/g;
  let m = re.exec(html);
  while (m !== null) {
    const body = extractBlockBody(html, m.index);
    if (body !== undefined && /--atomic-night\s*:/.test(body)) {
      return body;
    }
    m = re.exec(html);
  }
  return undefined;
}

// Parse a CSS block body into a name→value map for the --code-*,
// --tk-* and --atomic-* tokens only.
function declarationsOf(block: string): Map<string, string> {
  const out = new Map<string, string>();
  // The block may contain `\n` literally in the embedded HTML string —
  // treat both real newlines and escaped sequences as separators.
  const normalised = block.replace(/\\n/g, "\n");
  for (const m of normalised.matchAll(
    /--((?:code|tk|atomic)-[\w-]+):\s*([^;]+);/g,
  )) {
    out.set(`--${m[1]}`, m[2].trim());
  }
  return out;
}

const codeBlock = extractAcCodeblockBlock(source);
const rootBlock = extractRootBlockBody(source);
if (rootBlock === undefined) {
  throw new Error(
    "prototype-tokens.fixture.test: failed to locate brand-palette :root block in prototype",
  );
}
const protoMap = new Map<string, string>([
  ...declarationsOf(codeBlock),
  ...declarationsOf(rootBlock),
]);
const fixtureMap = new Map<string, string>(
  Object.entries(prototypeTokens) as ReadonlyArray<[string, string]>,
);

describe("prototype-tokens.json ↔ prototype-standalone.html drift detector", () => {
  it("every prototype token is captured in the fixture", () => {
    const missing: string[] = [];
    for (const name of protoMap.keys()) {
      if (!fixtureMap.has(name)) missing.push(name);
    }
    expect(missing).toEqual([]);
  });

  it("fixture introduces no token absent from the prototype", () => {
    const extra: string[] = [];
    for (const name of fixtureMap.keys()) {
      if (!protoMap.has(name)) extra.push(name);
    }
    expect(extra).toEqual([]);
  });

  for (const [name, value] of fixtureMap) {
    it(`${name}: fixture value matches prototype source`, () => {
      const proto = protoMap.get(name);
      expect(proto, `prototype source missing ${name}`).toBeDefined();
      expect(canonicaliseBrand(value)).toBe(canonicaliseBrand(proto!));
    });
  }
});

describe("extractRootBlockBody", () => {
  it("returns the first :root block containing --atomic-night", () => {
    const html = `
      :root { --ac-bg: #fff; }
      :root { --atomic-night: rgb(14, 15, 25); --atomic-bone: rgb(251, 252, 254); }
    `;
    const body = extractRootBlockBody(html);
    expect(body).toBeDefined();
    expect(body!).toMatch(/--atomic-night/);
    expect(body!).not.toMatch(/--ac-bg/);
  });
  it("skips :root blocks without --atomic-night (e.g. --ac-* defaults)", () => {
    const html = `:root { --ac-bg: #fff; --ac-fg: #000; }`;
    expect(extractRootBlockBody(html)).toBeUndefined();
  });
  it("returns undefined when no qualifying :root exists", () => {
    expect(extractRootBlockBody(`body { color: red; }`)).toBeUndefined();
  });
  it("handles balanced-brace pathological input (e.g. nested rules)", () => {
    const html = `
      :root {
        --atomic-night: rgb(14, 15, 25);
        @supports (display: grid) { color: red; }
      }
    `;
    const body = extractRootBlockBody(html);
    expect(body).toBeDefined();
    expect(body!).toMatch(/--atomic-night/);
    expect(body!).toMatch(/@supports/);
  });
});
