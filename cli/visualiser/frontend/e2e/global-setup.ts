/**
 * Playwright global setup — runs after webServer is ready.
 * Reads the real server port from .e2e-port and publishes it as BASE_URL
 * so playwright.config.ts (re-evaluated after globalSetup) and all tests
 * can use the correct origin.
 *
 * Also snapshots fixture files so globalTeardown can restore them,
 * guarding against interrupted runs leaving modified fixtures on disk.
 */
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const E2E_DIR = dirname(fileURLToPath(import.meta.url));
const portFile = join(E2E_DIR, "..", ".e2e-port");
export const FIXTURE_META_DIR = join(
  E2E_DIR,
  "..",
  "..",
  "server",
  "tests",
  "fixtures",
  "meta",
);
export const SNAPSHOT_FILE = join(E2E_DIR, "..", ".e2e-fixture-snapshot.json");

function workItemFixtures(): string[] {
  return readdirSync(join(FIXTURE_META_DIR, "work"))
    .filter((file) => file.endsWith(".md"))
    .map((file) => join("work", file));
}

function setManifestFixtures(setsDir: string, manifest: string): string[] {
  return readdirSync(join(FIXTURE_META_DIR, setsDir), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(setsDir, entry.name, manifest))
    .filter((file) => existsSync(join(FIXTURE_META_DIR, file)));
}

function editableFixtures(): string[] {
  return [
    ...workItemFixtures(),
    ...setManifestFixtures(join("research", "topics"), "manifest.md"),
    ...setManifestFixtures(
      join("research", "design-inventories"),
      "inventory.md",
    ),
  ];
}

function readFixtures(): Record<string, string> {
  const snapshot: Record<string, string> = {};
  for (const file of editableFixtures()) {
    snapshot[file] = readFileSync(join(FIXTURE_META_DIR, file), "utf-8");
  }
  return snapshot;
}

export function restoreFixtures(snapshot: Record<string, string>): void {
  for (const [file, content] of Object.entries(snapshot)) {
    writeFileSync(join(FIXTURE_META_DIR, file), content);
  }
}

// Probe that something is actually listening on the origin. A successful TCP
// connect is exactly the condition whose absence surfaces as
// net::ERR_CONNECTION_REFUSED in the browser, so this catches a dead origin
// before 344 tests fail one-by-one with no hint why.
function probeOrigin(port: number): Promise<void> {
  return new Promise((resolve, reject) => {
    const socket = connect({ host: "127.0.0.1", port }, () => {
      socket.end();
      resolve();
    });
    socket.once("error", reject);
    socket.setTimeout(1_000, () => {
      socket.destroy();
      reject(new Error("connect timed out"));
    });
  });
}

export default async function globalSetup() {
  // If a snapshot exists from a previous interrupted run, restore fixture
  // files to their pre-run state before snapshotting again.
  if (existsSync(SNAPSHOT_FILE)) {
    restoreFixtures(JSON.parse(readFileSync(SNAPSHOT_FILE, "utf-8")));
  }

  // Snapshot current fixture state so globalTeardown can restore it.
  writeFileSync(SNAPSHOT_FILE, JSON.stringify(readFixtures(), null, 2));

  if (!existsSync(portFile)) {
    throw new Error(
      "[e2e] .e2e-port not found after webServer started — " +
        "check that start-server.mjs wrote the file successfully.",
    );
  }
  const port = readFileSync(portFile, "utf-8").trim();

  // .e2e-port is trusted blindly, and with reuseExistingServer (local runs)
  // Playwright may reuse a previous run's still-listening health server while
  // its real server is dead or hung — pointing every test at a stale port. A
  // single liveness probe converts that into one actionable failure here
  // rather than a storm of net::ERR_CONNECTION_REFUSED across the whole suite.
  try {
    await probeOrigin(Number(port));
  } catch (err) {
    throw new Error(
      `[e2e] origin http://127.0.0.1:${port} (from .e2e-port) is not ` +
        `reachable: ${String(err)}. This usually means a stale .e2e-port ` +
        "plus an orphaned 'node e2e/start-server.mjs' still answering the " +
        "health port, so Playwright reused a dead server. Kill the orphaned " +
        "wrapper and remove " +
        "cli/visualiser/frontend/.e2e-port, then re-run.",
    );
  }

  process.env.BASE_URL = `http://127.0.0.1:${port}`;
  console.log(`[e2e] BASE_URL set to ${process.env.BASE_URL}`);
}
