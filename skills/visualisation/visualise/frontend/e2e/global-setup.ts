/**
 * Playwright global setup — runs after webServer is ready.
 * Reads the real server port from .e2e-port and publishes it as BASE_URL
 * so playwright.config.ts (re-evaluated after globalSetup) and all tests
 * can use the correct origin.
 *
 * Also snapshots fixture files so globalTeardown can restore them,
 * guarding against interrupted runs leaving modified fixtures on disk.
 */
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const E2E_DIR = dirname(fileURLToPath(import.meta.url))
const portFile = join(E2E_DIR, '..', '.e2e-port')
const FIXTURES_DIR = join(E2E_DIR, '..', '..', 'server', 'tests', 'fixtures', 'meta', 'work')
export const SNAPSHOT_FILE = join(E2E_DIR, '..', '.e2e-fixture-snapshot.json')

function readFixtures(): Record<string, string> {
  const snapshot: Record<string, string> = {}
  for (const file of readdirSync(FIXTURES_DIR)) {
    if (file.endsWith('.md')) {
      snapshot[file] = readFileSync(join(FIXTURES_DIR, file), 'utf-8')
    }
  }
  return snapshot
}

export function restoreFixtures(snapshot: Record<string, string>): void {
  for (const [file, content] of Object.entries(snapshot)) {
    writeFileSync(join(FIXTURES_DIR, file), content)
  }
}

export default function globalSetup() {
  // If a snapshot exists from a previous interrupted run, restore fixture
  // files to their pre-run state before snapshotting again.
  if (existsSync(SNAPSHOT_FILE)) {
    restoreFixtures(JSON.parse(readFileSync(SNAPSHOT_FILE, 'utf-8')))
  }

  // Snapshot current fixture state so globalTeardown can restore it.
  writeFileSync(SNAPSHOT_FILE, JSON.stringify(readFixtures(), null, 2))

  if (!existsSync(portFile)) {
    throw new Error(
      '[e2e] .e2e-port not found after webServer started — ' +
      'check that start-server.mjs wrote the file successfully.',
    )
  }
  const port = readFileSync(portFile, 'utf-8').trim()
  process.env.BASE_URL = `http://127.0.0.1:${port}`
  console.log(`[e2e] BASE_URL set to ${process.env.BASE_URL}`)
}
