import { readdirSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { DOC_TYPE_KEYS } from "./types";

// In-loop guard for the out-of-loop Docker VR lane: the committed glyph and
// big-glyph baselines are named from `DOC_TYPE_KEYS`, so a stale or missing
// filename (a rename that left the old key behind, or a new type with no
// baseline yet) is caught by `mise run check` rather than only by the Docker
// compare. cwd is the frontend package root under vitest.
const SNAPSHOTS = resolve(
  process.cwd(),
  "tests",
  "visual-regression",
  "__screenshots__",
);
const SIZES = [16, 24, 32, 48] as const;
const THEMES = ["light", "dark"] as const;

// Doc types whose baselines are not yet committed because they render only in
// the pinned Docker/Linux VR harness. `topic-research` landed its glyph and
// big-glyph showcases here; its 10 baselines are generated and committed by
// `mise run test:e2e:visualiser:docker:update`. Once committed, the
// exact-match assertions below fail (extra files) until the key is removed
// from this set — a self-cleaning reminder.
const PENDING_VR_BASELINES = new Set<string>(["topic-research"]);
const COVERED_KEYS = DOC_TYPE_KEYS.filter(
  (key) => !PENDING_VR_BASELINES.has(key),
);

function pngStems(dir: string): Set<string> {
  return new Set(
    readdirSync(resolve(SNAPSHOTS, dir)).filter((name) =>
      name.endsWith("-visual-regression.png"),
    ),
  );
}

describe("visual-regression baselines cover every DocTypeKey", () => {
  it("glyph baselines are exactly the covered DOC_TYPE_KEYS × sizes × themes", () => {
    const expected = new Set<string>();
    for (const key of COVERED_KEYS) {
      for (const size of SIZES) {
        for (const theme of THEMES) {
          expected.add(`${key}-${size}-${theme}-visual-regression.png`);
        }
      }
    }
    const actual = pngStems("dev-design-system-glyph.spec.ts-snapshots");
    expect(actual).toEqual(expected);
  });

  it("big-glyph baselines are exactly the covered DOC_TYPE_KEYS × themes", () => {
    const expected = new Set<string>();
    for (const key of COVERED_KEYS) {
      for (const theme of THEMES) {
        expected.add(`${key}-${theme}-visual-regression.png`);
      }
    }
    const actual = pngStems("dev-design-system-big-glyph.spec.ts-snapshots");
    expect(actual).toEqual(expected);
  });
});
