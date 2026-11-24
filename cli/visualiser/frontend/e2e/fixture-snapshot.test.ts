import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { restoreFixtures, snapshotFixtures } from "./fixture-snapshot";

const WORK_ITEM = join("work", "0001-x.md");
const RESEARCH_SET = join("research", "topics", "a", "manifest.md");
const INVENTORY = join(
  "research",
  "design-inventories",
  "2026-01-01-a",
  "inventory.md",
);

let metaDir: string;

function writeFixture(file: string, content: string): void {
  mkdirSync(dirname(join(metaDir, file)), { recursive: true });
  writeFileSync(join(metaDir, file), content);
}

function readFixture(file: string): string {
  return readFileSync(join(metaDir, file), "utf-8");
}

beforeEach(() => {
  metaDir = mkdtempSync(join(tmpdir(), "fixture-snapshot-"));
  writeFixture(WORK_ITEM, "original work item");
  writeFixture(RESEARCH_SET, "original research set");
  writeFixture(INVENTORY, "original inventory");
});

afterEach(() => {
  rmSync(metaDir, { recursive: true, force: true });
});

describe("restoreFixtures", () => {
  it("puts back every edited fixture a snapshot holds", () => {
    const snapshot = snapshotFixtures(metaDir);
    writeFixture(WORK_ITEM, "edited");
    writeFixture(RESEARCH_SET, "edited");
    writeFixture(INVENTORY, "edited");

    restoreFixtures(metaDir, snapshot);

    expect(readFixture(WORK_ITEM)).toBe("original work item");
    expect(readFixture(RESEARCH_SET)).toBe("original research set");
    expect(readFixture(INVENTORY)).toBe("original inventory");
  });

  it("rejects a snapshot naming a file that is not an editable fixture, writing nothing", () => {
    writeFixture(WORK_ITEM, "edited");
    const staleSnapshot = {
      [WORK_ITEM]: "original work item",
      "0001-x.md": "from an older snapshot layout",
    };

    expect(() => restoreFixtures(metaDir, staleSnapshot)).toThrow(/0001-x\.md/);
    expect(readFixture(WORK_ITEM)).toBe("edited");
    expect(existsSync(join(metaDir, "0001-x.md"))).toBe(false);
  });
});
