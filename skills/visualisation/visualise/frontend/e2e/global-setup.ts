/**
 * Playwright global setup — runs after webServer is ready.
 * Reads the real server port from .e2e-port and publishes it as BASE_URL
 * so playwright.config.ts (re-evaluated after globalSetup) and all tests
 * can use the correct origin.
 */
import { existsSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const portFile = join(dirname(fileURLToPath(import.meta.url)), '..', '.e2e-port')

export default function globalSetup() {
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
