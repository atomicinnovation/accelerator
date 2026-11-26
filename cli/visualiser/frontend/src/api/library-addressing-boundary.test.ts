import { describe, expect, it } from "vitest";
import {
  isLibraryDocumentAddressModule,
  libraryAddressingViolations,
} from "./testing/library-addressing-boundary";

const productionModules = import.meta.glob(
  [
    "../**/*.{ts,tsx}",
    "!../**/*.test.{ts,tsx}",
    "!../test/**",
    "!../**/testing/**",
    "!../**/test-fixtures.ts",
  ],
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;

const scannedPaths = Object.keys(productionModules);

describe("library addressing boundary", () => {
  it("scans the address module and the modules that link to the library", () => {
    expect(scannedPaths.some(isLibraryDocumentAddressModule)).toBe(true);
    expect(
      scannedPaths.some((p) => p.endsWith("/ActivityFeed/ActivityFeed.tsx")),
    ).toBe(true);
  });

  it("does not scan the boundary rules themselves", () => {
    expect(
      scannedPaths.some((p) => p.endsWith("/library-addressing-boundary.ts")),
    ).toBe(false);
  });

  it("keeps library addressing inside the address module", () => {
    expect(libraryAddressingViolations(productionModules)).toEqual([]);
  });
});
