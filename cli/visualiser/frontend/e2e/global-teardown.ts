import { existsSync, readFileSync, rmSync } from "node:fs";
import { restoreFixtures } from "./fixture-snapshot.js";
import { FIXTURE_META_DIR, SNAPSHOT_FILE } from "./global-setup.js";

export default function globalTeardown() {
  if (!existsSync(SNAPSHOT_FILE)) return;
  restoreFixtures(
    FIXTURE_META_DIR,
    JSON.parse(readFileSync(SNAPSHOT_FILE, "utf-8")),
  );
  rmSync(SNAPSHOT_FILE);
}
