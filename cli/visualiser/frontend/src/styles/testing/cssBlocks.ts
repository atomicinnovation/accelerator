// Test-only helper. Consumers: global.test.ts,
// prototype-tokens.fixture.test.ts, extractAcDeclarations.ts.
//
// Do not import from production code.
//
// `extractBlockBody(source, index)` scans the next `{ ... }` block starting
// at `index` using brace-balanced scanning so nested rules do not truncate
// the match. Returns the body (without the enclosing braces) or `undefined`
// if no balanced block exists at that position. Resilient to formatter
// changes (no column-0 anchor required).

export function extractBlockBody(
  source: string,
  index: number,
): string | undefined {
  const open = source.indexOf("{", index);
  if (open === -1) return undefined;
  let depth = 1;
  for (let i = open + 1; i < source.length; i++) {
    if (source[i] === "{") depth++;
    else if (source[i] === "}") {
      depth--;
      if (depth === 0) return source.slice(open + 1, i);
    }
  }
  return undefined;
}
