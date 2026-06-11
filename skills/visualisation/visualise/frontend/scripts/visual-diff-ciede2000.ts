/**
 * Visual diff using CIEDE2000 colour-difference metric.
 *
 * Usage: npx tsx scripts/visual-diff-ciede2000.ts <baseline> <actual> [--out <heatmap.png>]
 *
 * Exit codes:
 *   0 — max ΔE2000 < 5 across all changed pixels (AC5 threshold from
 *       meta/work/0073-atomic-brand-layer-palette.md)
 *   1 — at least one pixel exceeds the threshold
 *   2 — invalid input (missing file, decode error, etc.)
 *
 * See AC5 in 0073 for the threshold rationale; the script is intended
 * to be reusable across future visual-regression PRs.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { differenceCiede2000, parse } from "culori";
import { PNG } from "pngjs";

export const THRESHOLD_DELTA_E = 5;

export interface Pixel {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface DiffReport {
  width: number;
  height: number;
  changedPixels: number;
  maxDeltaE: number;
  meanDeltaE: number;
  p95DeltaE: number;
  boundingBox: {
    minX: number;
    minY: number;
    maxX: number;
    maxY: number;
  } | null;
}

export interface DimensionMismatch {
  baseline: { width: number; height: number };
  actual: { width: number; height: number };
}

export function decodePng(buf: Buffer): PNG {
  return PNG.sync.read(buf);
}

const dE = differenceCiede2000();

export function pixelDeltaE(a: Pixel, b: Pixel): number {
  // Use culori's parse so the math operates in OKLab/Lab space.
  const aRef = parse(`rgb(${a.r}, ${a.g}, ${a.b})`);
  const bRef = parse(`rgb(${b.r}, ${b.g}, ${b.b})`);
  if (!aRef || !bRef) return 0;
  return dE(aRef, bRef);
}

export function diffImages(
  baseline: PNG,
  actual: PNG,
): DiffReport | DimensionMismatch {
  if (baseline.width !== actual.width || baseline.height !== actual.height) {
    return {
      baseline: { width: baseline.width, height: baseline.height },
      actual: { width: actual.width, height: actual.height },
    };
  }
  const w = baseline.width;
  const h = baseline.height;
  const deltas: number[] = [];
  let minX = w;
  let minY = h;
  let maxX = -1;
  let maxY = -1;
  let max = 0;
  let sum = 0;

  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) << 2;
      const a: Pixel = {
        r: baseline.data[i],
        g: baseline.data[i + 1],
        b: baseline.data[i + 2],
        a: baseline.data[i + 3],
      };
      const b: Pixel = {
        r: actual.data[i],
        g: actual.data[i + 1],
        b: actual.data[i + 2],
        a: actual.data[i + 3],
      };
      // Skip pixels that are byte-identical (also skips alpha-only changes
      // where RGB matches; AC5 cares about colour drift, not alpha).
      if (a.r === b.r && a.g === b.g && a.b === b.b) continue;
      const d = pixelDeltaE(a, b);
      if (d <= 0) continue;
      deltas.push(d);
      sum += d;
      if (d > max) max = d;
      if (x < minX) minX = x;
      if (y < minY) minY = y;
      if (x > maxX) maxX = x;
      if (y > maxY) maxY = y;
    }
  }

  const sorted = deltas.slice().sort((p, q) => p - q);
  const p95Idx = Math.floor(sorted.length * 0.95);
  const p95 =
    sorted.length > 0 ? sorted[Math.min(p95Idx, sorted.length - 1)] : 0;
  return {
    width: w,
    height: h,
    changedPixels: deltas.length,
    maxDeltaE: max,
    meanDeltaE: deltas.length > 0 ? sum / deltas.length : 0,
    p95DeltaE: p95,
    boundingBox: maxX >= 0 ? { minX, minY, maxX, maxY } : null,
  };
}

export function formatReport(r: DiffReport): string {
  const bb = r.boundingBox
    ? `(${r.boundingBox.minX},${r.boundingBox.minY})-(${r.boundingBox.maxX},${r.boundingBox.maxY})`
    : "n/a";
  return [
    `dimensions:    ${r.width}x${r.height}`,
    `changed:       ${r.changedPixels} pixels`,
    `max ΔE2000:    ${r.maxDeltaE.toFixed(3)}`,
    `mean ΔE2000:   ${r.meanDeltaE.toFixed(3)}`,
    `p95 ΔE2000:    ${r.p95DeltaE.toFixed(3)}`,
    `bounding box:  ${bb}`,
  ].join("\n");
}

function isDimensionMismatch(
  r: DiffReport | DimensionMismatch,
): r is DimensionMismatch {
  return (r as DimensionMismatch).baseline !== undefined;
}

function parseArgs(argv: ReadonlyArray<string>): {
  baseline: string;
  actual: string;
  out?: string;
} | null {
  const positional: string[] = [];
  let out: string | undefined;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--out") {
      out = argv[++i];
    } else {
      positional.push(a);
    }
  }
  if (positional.length !== 2) return null;
  return { baseline: positional[0], actual: positional[1], out };
}

function writeHeatmap(baseline: PNG, actual: PNG, outPath: string): void {
  const w = baseline.width;
  const h = baseline.height;
  const png = new PNG({ width: w, height: h });
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) << 2;
      const same =
        baseline.data[i] === actual.data[i] &&
        baseline.data[i + 1] === actual.data[i + 1] &&
        baseline.data[i + 2] === actual.data[i + 2];
      if (same) {
        png.data[i] = 240;
        png.data[i + 1] = 240;
        png.data[i + 2] = 240;
        png.data[i + 3] = 255;
      } else {
        png.data[i] = 255;
        png.data[i + 1] = 0;
        png.data[i + 2] = 0;
        png.data[i + 3] = 255;
      }
    }
  }
  writeFileSync(outPath, PNG.sync.write(png));
}

export function main(argv: ReadonlyArray<string>): number {
  const args = parseArgs(argv);
  if (!args) {
    console.error(
      "usage: visual-diff-ciede2000.ts <baseline.png> <actual.png> [--out <heatmap.png>]",
    );
    return 2;
  }
  let baseline: PNG;
  let actual: PNG;
  try {
    baseline = decodePng(readFileSync(args.baseline));
    actual = decodePng(readFileSync(args.actual));
  } catch (err) {
    console.error(`error reading inputs: ${(err as Error).message}`);
    return 2;
  }
  const result = diffImages(baseline, actual);
  if (isDimensionMismatch(result)) {
    console.error(
      `dimension mismatch: baseline ${result.baseline.width}x${result.baseline.height} ` +
        `vs actual ${result.actual.width}x${result.actual.height}`,
    );
    return 2;
  }
  console.log(formatReport(result));
  if (args.out) writeHeatmap(baseline, actual, args.out);
  return result.maxDeltaE < THRESHOLD_DELTA_E ? 0 : 1;
}

// Only run main when invoked as a script (not when imported by tests).
// process.argv[1] ends with the script path; check via simple substring.
const isMain =
  typeof process !== "undefined" &&
  process.argv[1] !== undefined &&
  /visual-diff-ciede2000(?:\.[jt]s)?$/.test(process.argv[1]);
if (isMain) {
  process.exit(main(process.argv.slice(2)));
}
