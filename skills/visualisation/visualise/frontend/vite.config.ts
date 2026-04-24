import { readFileSync } from 'node:fs'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

/**
 * Resolve the dev API port in this order:
 *   1. `VISUALISER_API_PORT` env var (explicit override).
 *   2. `VISUALISER_INFO_PATH` env var → read `{ port }` from that JSON file.
 *   3. Give up and fall back to port 0 (ECONNREFUSED loudly).
 */
function resolveApiPort(): number {
  const fromEnv = process.env.VISUALISER_API_PORT
  if (fromEnv && Number.isFinite(Number(fromEnv))) return Number(fromEnv)

  const infoPath = process.env.VISUALISER_INFO_PATH
  if (infoPath) {
    try {
      const info = JSON.parse(readFileSync(infoPath, 'utf-8')) as { port?: number }
      if (typeof info.port === 'number') return info.port
    } catch (err) {
      console.warn(
        `[vite.config] Failed to read port from VISUALISER_INFO_PATH=${infoPath}:`,
        err,
      )
    }
  }

  console.warn(
    '[vite.config] Dev API port not resolved — set VISUALISER_API_PORT=<port> ' +
    'or VISUALISER_INFO_PATH=<path to server-info.json> before `npm run dev`. ' +
    'Falling back to port 0, which will ECONNREFUSED loudly.',
  )
  return 0
}

const apiPort = resolveApiPort()

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': {
        target: `http://127.0.0.1:${apiPort}`,
        changeOrigin: true,
      },
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: true,
    restoreMocks: true,
  },
})
