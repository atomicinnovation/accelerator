import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PNG } from "pngjs";
import { describe, expect, it } from "vitest";
import {
  decodePng,
  diffImages,
  main,
  pixelDeltaE,
} from "./visual-diff-ciede2000";

function makePng(
  width: number,
  height: number,
  fill: [number, number, number, number],
): PNG {
  const png = new PNG({ width, height });
  for (let i = 0; i < png.data.length; i += 4) {
    png.data[i] = fill[0];
    png.data[i + 1] = fill[1];
    png.data[i + 2] = fill[2];
    png.data[i + 3] = fill[3];
  }
  return png;
}

function writePngToTmp(png: PNG, name: string): string {
  const dir = mkdtempSync(join(tmpdir(), "vd-"));
  const path = join(dir, name);
  writeFileSync(path, PNG.sync.write(png));
  return path;
}

describe("diffImages", () => {
  it("returns max ΔE = 0 for identical buffers", () => {
    const a = makePng(2, 2, [10, 20, 30, 255]);
    const b = makePng(2, 2, [10, 20, 30, 255]);
    const r = diffImages(a, b);
    expect("baseline" in r).toBe(false);
    if ("baseline" in r) return;
    expect(r.maxDeltaE).toBe(0);
    expect(r.changedPixels).toBe(0);
    expect(r.boundingBox).toBeNull();
  });

  it("reports per-pixel ΔE for a one-channel-off pair", () => {
    const a = makePng(1, 1, [128, 128, 128, 255]);
    const b = makePng(1, 1, [128, 128, 130, 255]);
    const r = diffImages(a, b);
    expect("baseline" in r).toBe(false);
    if ("baseline" in r) return;
    expect(r.changedPixels).toBe(1);
    expect(r.maxDeltaE).toBeGreaterThan(0);
    expect(r.maxDeltaE).toBeLessThan(2);
    expect(r.boundingBox).toEqual({ minX: 0, minY: 0, maxX: 0, maxY: 0 });
  });

  it("produces a max ΔE > threshold for a clearly divergent pair", () => {
    const a = makePng(1, 1, [0, 0, 0, 255]);
    const b = makePng(1, 1, [255, 0, 0, 255]);
    const r = diffImages(a, b);
    expect("baseline" in r).toBe(false);
    if ("baseline" in r) return;
    expect(r.maxDeltaE).toBeGreaterThan(5);
  });

  it("returns a DimensionMismatch for differently sized inputs", () => {
    const a = makePng(2, 2, [0, 0, 0, 255]);
    const b = makePng(3, 2, [0, 0, 0, 255]);
    const r = diffImages(a, b);
    expect("baseline" in r).toBe(true);
    if (!("baseline" in r)) return;
    expect(r.baseline).toEqual({ width: 2, height: 2 });
    expect(r.actual).toEqual({ width: 3, height: 2 });
  });
});

describe("pixelDeltaE", () => {
  it("is zero for identical pixels", () => {
    expect(
      pixelDeltaE(
        { r: 100, g: 100, b: 100, a: 255 },
        { r: 100, g: 100, b: 100, a: 255 },
      ),
    ).toBe(0);
  });
  it("is positive for differing pixels", () => {
    expect(
      pixelDeltaE(
        { r: 0, g: 0, b: 0, a: 255 },
        { r: 255, g: 255, b: 255, a: 255 },
      ),
    ).toBeGreaterThan(0);
  });
});

describe("decodePng roundtrip", () => {
  it("encodes and decodes back to the same pixel values", () => {
    const png = makePng(2, 1, [10, 20, 30, 255]);
    const buf = PNG.sync.write(png);
    const decoded = decodePng(buf);
    expect(decoded.width).toBe(2);
    expect(decoded.height).toBe(1);
    expect(decoded.data[0]).toBe(10);
    expect(decoded.data[1]).toBe(20);
    expect(decoded.data[2]).toBe(30);
  });
});

describe("main", () => {
  it("exits 0 for identical PNGs on disk", () => {
    const png = makePng(2, 2, [50, 60, 70, 255]);
    const a = writePngToTmp(png, "a.png");
    const b = writePngToTmp(png, "b.png");
    expect(main([a, b])).toBe(0);
  });

  it("exits 1 for over-threshold divergence", () => {
    const a = writePngToTmp(makePng(1, 1, [0, 0, 0, 255]), "a.png");
    const b = writePngToTmp(makePng(1, 1, [255, 0, 0, 255]), "b.png");
    expect(main([a, b])).toBe(1);
  });

  it("exits 2 for dimension mismatch", () => {
    const a = writePngToTmp(makePng(2, 2, [0, 0, 0, 255]), "a.png");
    const b = writePngToTmp(makePng(3, 2, [0, 0, 0, 255]), "b.png");
    expect(main([a, b])).toBe(2);
  });

  it("exits 2 for missing args", () => {
    expect(main([])).toBe(2);
  });

  it("exits 2 for missing files", () => {
    expect(main(["/no/such/baseline.png", "/no/such/actual.png"])).toBe(2);
  });
});
