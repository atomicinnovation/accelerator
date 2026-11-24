import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export type FixtureSnapshot = Record<string, string>;

function workItemFixtures(metaDir: string): string[] {
  return readdirSync(join(metaDir, "work"))
    .filter((file) => file.endsWith(".md"))
    .map((file) => join("work", file));
}

function setManifestFixtures(
  metaDir: string,
  setsDir: string,
  manifest: string,
): string[] {
  return readdirSync(join(metaDir, setsDir), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(setsDir, entry.name, manifest))
    .filter((file) => existsSync(join(metaDir, file)));
}

function editableFixtures(metaDir: string): string[] {
  return [
    ...workItemFixtures(metaDir),
    ...setManifestFixtures(metaDir, join("research", "topics"), "manifest.md"),
    ...setManifestFixtures(
      metaDir,
      join("research", "design-inventories"),
      "inventory.md",
    ),
  ];
}

export function snapshotFixtures(metaDir: string): FixtureSnapshot {
  const snapshot: FixtureSnapshot = {};
  for (const file of editableFixtures(metaDir)) {
    snapshot[file] = readFileSync(join(metaDir, file), "utf-8");
  }
  return snapshot;
}

export function restoreFixtures(
  metaDir: string,
  snapshot: FixtureSnapshot,
): void {
  const editable = new Set(editableFixtures(metaDir));
  const unknown = Object.keys(snapshot).filter((file) => !editable.has(file));
  if (unknown.length > 0) {
    throw new Error(
      `[e2e] The fixture snapshot names files that are not editable ` +
        `fixtures: ${unknown.join(", ")}. It predates the current fixture ` +
        `layout; delete .e2e-fixture-snapshot.json and rerun.`,
    );
  }
  for (const [file, content] of Object.entries(snapshot)) {
    writeFileSync(join(metaDir, file), content);
  }
}
