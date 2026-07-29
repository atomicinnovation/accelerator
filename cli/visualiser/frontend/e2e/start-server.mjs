/**
 * Spawns the visualiser server against the committed test fixtures.
 *
 * A tiny health HTTP server on E2E_HEALTH_PORT (default 19087) lets
 * Playwright's webServer.url detect readiness without needing to know the
 * real server port upfront. It returns 503 until server-info.json appears,
 * then 200. The real port is written to .e2e-port for the baseURL fixture.
 *
 * Playwright sends SIGTERM when all tests have finished; we forward it.
 */

import { execSync, spawn } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:http";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const frontendDir = resolve(__dirname, "..");
const serverDir = resolve(frontendDir, "../server");
const fixturesDir = join(serverDir, "tests/fixtures/meta");

const HEALTH_PORT = Number(process.env.E2E_HEALTH_PORT ?? 19087);

// ── 1. Start the health server (returns 503 until real server is ready) ──────

let serverReady = false;
const healthServer = createServer((_req, res) => {
  res.writeHead(serverReady ? 200 : 503);
  res.end(serverReady ? "ok" : "starting");
});
healthServer.listen(HEALTH_PORT, "127.0.0.1", () => {
  console.log(
    `[e2e] Health server listening on http://127.0.0.1:${HEALTH_PORT}`,
  );
});

// ── 2. Resolve the server binary ─────────────────────────────────────────────

let bin = process.env.ACCELERATOR_VISUALISER_BIN;
if (!bin) {
  console.log("[e2e] Building server binary (dev-frontend)…");
  execSync("cargo build --no-default-features --features dev-frontend", {
    cwd: serverDir,
    stdio: "inherit",
  });
  bin = join(serverDir, "target/debug/accelerator-visualiser");
}
if (!existsSync(bin)) {
  console.error(`[e2e] Binary not found: ${bin}`);
  process.exit(1);
}

// ── 3. Root the server at the committed fixtures ─────────────────────────────
//
// The Model-1 server reads .accelerator/*.md from the discovered project root
// and derives each doc's API path by stripping that root off the doc's own
// path, so the served docs must live physically under the root. A symlink or an
// out-of-tree copy breaks that: a symlink resolves to a path that escapes the
// root (absolute API paths the doc-read guard 403s), and a copy decouples the
// served files from the on-disk fixtures these specs mutate (breaking the
// fs-driven SSE and per-test reset flow). So the fixtures directory itself is
// the project root — meta/ and templates/ are already real children — and only
// .accelerator (config + git-ignored runtime state) is added. config.md remaps
// the one path that differs from the catalogue default (research_codebase →
// meta/research). globalSetup/globalTeardown snapshot and restore the mutated
// work-item files.

const project = join(serverDir, "tests/fixtures");
// The project root is fixed (not a fresh tempdir), so runtime state from an
// earlier run persists here. Clear it before spawning so the readiness wait
// below blocks on THIS server's server-info.json rather than reading a stale
// port left by a previous run (or a checkout that accidentally tracked one).
rmSync(join(project, ".accelerator", "tmp"), { recursive: true, force: true });
mkdirSync(join(project, ".accelerator"), { recursive: true });
writeFileSync(
  join(project, ".accelerator", "config.md"),
  "---\npaths:\n  research_codebase: meta/research\n---\n",
);
const infoPath = join(project, ".accelerator/tmp/visualiser/server-info.json");

// ── 4. Spawn the visualiser server ────────────────────────────────────────────
//
// E2E_SERVER_HOST (set by the Docker visual-regression task) opts the
// dev-frontend server into a non-loopback bind so the container can reach the
// host over the bridge gateway. --owner-pid 0 disables owner-based shutdown.

const child = spawn(bin, ["serve", "--owner-pid", "0"], {
  cwd: project,
  env: {
    ...process.env,
    ACCELERATOR_PLUGIN_ROOT: project,
    FIXTURES_PATH: fixturesDir,
  },
  stdio: "inherit",
});

child.on("error", (err) => {
  console.error("[e2e] Failed to spawn server:", err);
  process.exit(1);
});

child.on("exit", (code) => {
  if (code !== 0 && code !== null) {
    console.error(`[e2e] Server exited with code ${code}`);
  }
  // The real server is gone, so this wrapper must not linger: a lingering
  // wrapper keeps the health server answering 200 for a now-dead port, and a
  // later run (reuseExistingServer) would silently reuse it and point every
  // test at a dead origin (ERR_CONNECTION_REFUSED). Tear down and exit so the
  // health port is freed and the next run starts a fresh server.
  serverReady = false;
  healthServer.close();
  process.exit(code ?? 0);
});

// ── 5. Wait for server-info.json, publish port, signal health ─────────────────

const deadline = Date.now() + 30_000;
while (!existsSync(infoPath)) {
  if (Date.now() > deadline) {
    console.error("[e2e] server-info.json did not appear within 30s");
    child.kill();
    process.exit(1);
  }
  await new Promise((r) => setTimeout(r, 100));
}

const info = JSON.parse(readFileSync(infoPath, "utf-8"));
const port = info.port;
console.log(`[e2e] Visualiser server ready at http://127.0.0.1:${port}`);
writeFileSync(join(frontendDir, ".e2e-port"), String(port));
serverReady = true;

// ── 6. Stay alive until Playwright sends SIGTERM ─────────────────────────────

process.on("SIGTERM", () => {
  child.kill("SIGTERM");
  healthServer.close();
});
process.on("SIGINT", () => {
  child.kill("SIGTERM");
  healthServer.close();
  process.exit(0);
});

await new Promise(() => {});
