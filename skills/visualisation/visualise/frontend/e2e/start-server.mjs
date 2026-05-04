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

import { execSync, spawn } from 'node:child_process'
import { createServer } from 'node:http'
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const frontendDir = resolve(__dirname, '..')
const serverDir = resolve(frontendDir, '../server')
const fixturesDir = join(serverDir, 'tests/fixtures/meta')
const templatesDir = join(serverDir, 'tests/fixtures/templates')

const HEALTH_PORT = Number(process.env.E2E_HEALTH_PORT ?? 19087)

// ── 1. Start the health server (returns 503 until real server is ready) ──────

let serverReady = false
const healthServer = createServer((_req, res) => {
  res.writeHead(serverReady ? 200 : 503)
  res.end(serverReady ? 'ok' : 'starting')
})
healthServer.listen(HEALTH_PORT, '127.0.0.1', () => {
  console.log(`[e2e] Health server listening on http://127.0.0.1:${HEALTH_PORT}`)
})

// ── 2. Resolve the server binary ─────────────────────────────────────────────

let bin = process.env.ACCELERATOR_VISUALISER_BIN
if (!bin) {
  console.log('[e2e] Building server binary (dev-frontend)…')
  execSync(
    'cargo build --no-default-features --features dev-frontend',
    { cwd: serverDir, stdio: 'inherit' },
  )
  bin = join(serverDir, 'target/debug/accelerator-visualiser')
}
if (!existsSync(bin)) {
  console.error(`[e2e] Binary not found: ${bin}`)
  process.exit(1)
}

// ── 3. Write a config pointing at the committed fixtures ──────────────────────

const tmpDir = mkdtempSync(join(tmpdir(), 'vis-e2e-'))
const configPath = join(tmpDir, 'config.json')
const infoPath = join(tmpDir, 'server-info.json')

const docPaths = {
  decisions: join(fixturesDir, 'decisions'),
  work: join(fixturesDir, 'work'),
  plans: join(fixturesDir, 'plans'),
  research: join(fixturesDir, 'research'),
  review_plans: join(fixturesDir, 'reviews/plans'),
  review_prs: join(fixturesDir, 'reviews/prs'),
  validations: join(fixturesDir, 'validations'),
  notes: join(fixturesDir, 'notes'),
  prs: join(fixturesDir, 'prs'),
}

const templates = {}
for (const name of ['adr', 'plan', 'research', 'validation', 'pr-description']) {
  templates[name] = {
    config_override: null,
    user_override: join(fixturesDir, `templates/${name}.md`),
    plugin_default: join(templatesDir, `${name}.md`),
  }
}

writeFileSync(configPath, JSON.stringify({
  plugin_root: serverDir,
  plugin_version: '0.0.0-e2e',
  project_root: serverDir,
  tmp_path: tmpDir,
  host: '127.0.0.1',
  owner_pid: 0,
  owner_start_time: null,
  log_path: join(tmpDir, 'server.log'),
  doc_paths: docPaths,
  templates,
}, null, 2))

// ── 4. Spawn the visualiser server ────────────────────────────────────────────

const child = spawn(bin, ['--config', configPath], {
  env: { ...process.env, FIXTURES_PATH: fixturesDir },
  stdio: 'inherit',
})

child.on('error', (err) => {
  console.error('[e2e] Failed to spawn server:', err)
  process.exit(1)
})

child.on('exit', (code) => {
  if (code !== 0 && code !== null) {
    console.error(`[e2e] Server exited with code ${code}`)
  }
})

// ── 5. Wait for server-info.json, publish port, signal health ─────────────────

const deadline = Date.now() + 30_000
while (!existsSync(infoPath)) {
  if (Date.now() > deadline) {
    console.error('[e2e] server-info.json did not appear within 30s')
    child.kill()
    process.exit(1)
  }
  await new Promise((r) => setTimeout(r, 100))
}

const info = JSON.parse(readFileSync(infoPath, 'utf-8'))
const port = info.port
console.log(`[e2e] Visualiser server ready at http://127.0.0.1:${port}`)
writeFileSync(join(frontendDir, '.e2e-port'), String(port))
serverReady = true

// ── 6. Stay alive until Playwright sends SIGTERM ─────────────────────────────

process.on('SIGTERM', () => {
  child.kill('SIGTERM')
  healthServer.close()
})
process.on('SIGINT', () => {
  child.kill('SIGTERM')
  healthServer.close()
  process.exit(0)
})

await new Promise(() => {})
