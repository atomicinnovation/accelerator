import { existsSync, readFileSync, rmSync } from "node:fs";
import { restoreFixtures, SNAPSHOT_FILE } from "./global-setup.js";

export default function globalTeardown() {
  if (!existsSync(SNAPSHOT_FILE)) return;
  restoreFixtures(JSON.parse(readFileSync(SNAPSHOT_FILE, "utf-8")));
  rmSync(SNAPSHOT_FILE);
}
