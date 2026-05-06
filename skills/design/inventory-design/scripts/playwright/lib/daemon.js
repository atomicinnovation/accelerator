// Playwright browser-context daemon server.
// Launched via: node run.js daemon --state-dir <dir> --owner-pid <pid>
//
// Listens on 127.0.0.1:0 (OS-assigned port). Writes server-info.json and
// server.pid atomically once ready. Handles a single sequential JSON
// request per TCP connection. Shuts down on SIGTERM, SIGINT, idle timeout,
// owner-PID exit, or per-op wall-clock timeout.

import { createServer } from 'node:http';
import { writeServerInfo, writeServerStopped, removeServerFiles, ensureStateDir } from './state.js';
import { makeAuthHeaderHandler } from './auth-header.js';
import { mergeMaskSelectors } from './mask.js';
import { guardScreenshotPath } from './path-guard.js';
import { makeError, protocolMismatch, PROTOCOL } from './errors.js';

const IDLE_MS = parseInt(process.env.ACCELERATOR_PLAYWRIGHT_IDLE_MS || '1800000', 10);     // 30 min
const WALL_CLOCK_RAW = parseInt(process.env.ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS || '300000', 10);
const WALL_CLOCK_CEILING = 1800000; // 30 min hard ceiling
const WALL_CLOCK_MS = Math.min(WALL_CLOCK_RAW, WALL_CLOCK_CEILING);
const OWNER_POLL_MS = parseInt(process.env.ACCELERATOR_PLAYWRIGHT_OWNER_POLL_MS || '60000', 10);

const BLOCKING_OPS = new Set(['navigate', 'snapshot', 'screenshot', 'evaluate', 'click', 'type', 'wait_for']);

// ESM import() does not honour NODE_PATH. When run via run.sh the daemon
// receives ACCELERATOR_PLAYWRIGHT_NS_ROOT (the namespaced cache dir) and we
// construct a file:// URL so the import resolver can find playwright.
async function importPlaywright() {
  const nsRoot = process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT;
  if (!nsRoot) return import('playwright');
  const { resolve } = await import('node:path');
  const { pathToFileURL } = await import('node:url');
  let entryFile = 'index.js';
  try {
    const { readFileSync } = await import('node:fs');
    const pkg = JSON.parse(readFileSync(resolve(nsRoot, 'node_modules/playwright/package.json'), 'utf8'));
    if (typeof pkg.main === 'string') entryFile = pkg.main;
  } catch {}
  return import(pathToFileURL(resolve(nsRoot, 'node_modules/playwright', entryFile)).href);
}

export async function startDaemon({ stateDir, ownerPid }) {
  ensureStateDir(stateDir);

  let browser = null;
  let page = null;
  let shutdownInitiated = false;
  let idleTimer = null;
  let wallClockTimer = null;
  let currentOp = null; // { name, res, conn }

  // ------ Shutdown --------------------------------------------------------

  async function shutdown(reason, extra = {}) {
    if (shutdownInitiated) return;
    shutdownInitiated = true;

    clearTimeout(idleTimer);
    clearTimeout(wallClockTimer);
    clearInterval(ownerWatcher);

    try { writeServerStopped(stateDir, reason, extra); } catch {}

    try { await browser?.close(); } catch {}

    server.close(() => {
      removeServerFiles(stateDir);
      process.exit(reason === 'wall-clock' ? 2 : 0);
    });
    setTimeout(() => process.exit(reason === 'wall-clock' ? 2 : 0), 3000).unref();
  }

  // ------ Idle timer ------------------------------------------------------

  function resetIdle() {
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => shutdown('idle'), IDLE_MS).unref();
  }

  // ------ Wall-clock per-op timer ----------------------------------------

  function armWallClock(opName, conn) {
    clearTimeout(wallClockTimer);
    wallClockTimer = setTimeout(async () => {
      const envelope = makeError({
        error: 'wall-clock-exceeded',
        message: `Operation exceeded the ${WALL_CLOCK_MS}ms wall-clock budget.`,
        category: 'browser',
        retryable: false,
        details: { op: opName, wall_clock_ms: WALL_CLOCK_MS },
      });
      try {
        conn.write(JSON.stringify(envelope) + '\n');
        await new Promise(r => setTimeout(r, 500));
      } catch {}
      await shutdown('wall-clock', { op: opName, wall_clock_ms: WALL_CLOCK_MS });
    }, WALL_CLOCK_MS);
  }

  function disarmWallClock() {
    clearTimeout(wallClockTimer);
    wallClockTimer = null;
  }

  // ------ Owner PID watcher -----------------------------------------------

  const ownerWatcher = ownerPid > 0 ? setInterval(() => {
    try {
      process.kill(ownerPid, 0);
    } catch {
      shutdown('owner-exited', { owner_pid: ownerPid });
    }
  }, OWNER_POLL_MS) : null;
  if (ownerWatcher) ownerWatcher.unref();

  // ------ Browser / page --------------------------------------------------

  async function ensureBrowser() {
    if (browser) return;
    const { chromium } = await importPlaywright();
    browser = await chromium.launch({ headless: true });
    const ctx = await browser.newContext();
    page = await ctx.newPage();
  }

  // ------ Request handler -------------------------------------------------

  async function handleRequest(req) {
    if (req.protocol !== PROTOCOL) return protocolMismatch(req.protocol);

    const cmd = req.command;

    if (cmd === 'ping') {
      const { chromium: cr } = await importPlaywright();
      const execPath = cr.executablePath();
      try {
        await import('node:fs').then(({ promises }) => promises.access(execPath));
      } catch {
        return makeError({ error: 'chromium-not-found', message: `Chromium binary not found at ${execPath}. Run ensure-playwright.sh to reinstall.`, category: 'bootstrap', retryable: false, details: { execPath } });
      }
      const nsRoot = process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT;
      let pv = 'unknown';
      try {
        const { resolve } = await import('node:path');
        const { readFile } = await import('node:fs/promises');
        const pkgFile = nsRoot
          ? resolve(nsRoot, 'node_modules/playwright/package.json')
          : new URL('../node_modules/playwright/package.json', import.meta.url).pathname;
        const raw = JSON.parse(await readFile(pkgFile, 'utf8'));
        pv = raw.version;
      } catch {}
      return { protocol: PROTOCOL, ok: true, node: process.version, playwright: pv, chromium: execPath };
    }

    if (cmd === 'daemon-status') {
      return { protocol: PROTOCOL, state: 'running', pid: process.pid };
    }

    if (cmd === 'daemon-stop') {
      setImmediate(() => shutdown('daemon-stop'));
      return { protocol: PROTOCOL, ok: true };
    }

    // All remaining commands require the browser
    await ensureBrowser();

    switch (cmd) {
      case 'navigate': {
        if (!req.url) return makeError({ error: 'missing-url', message: 'navigate requires a "url" field', category: 'usage', retryable: false });
        await page.goto(req.url, { waitUntil: 'domcontentloaded', timeout: WALL_CLOCK_MS });
        return { protocol: PROTOCOL, ok: true, url: page.url() };
      }

      case 'snapshot': {
        const snap = await page.accessibility.snapshot();
        return { protocol: PROTOCOL, snapshot: snap };
      }

      case 'screenshot': {
        const guard = guardScreenshotPath(req.path || 'screenshot.png');
        if (!guard.ok) return guard.error;
        const { mkdirSync } = await import('node:fs');
        const { dirname } = await import('node:path');
        mkdirSync(dirname(guard.path), { recursive: true });
        const masks = mergeMaskSelectors(req.mask || []);
        await page.screenshot({
          path: guard.path,
          mask: masks.map(sel => page.locator(sel)),
          fullPage: req.full_page ?? false,
        });
        return { protocol: PROTOCOL, ok: true, path: guard.path };
      }

      case 'evaluate': {
        if (req.expression === undefined) return makeError({ error: 'missing-expression', message: 'evaluate requires an "expression" field', category: 'usage', retryable: false });
        const result = await page.evaluate(req.expression);
        return { protocol: PROTOCOL, result };
      }

      case 'click': {
        if (!req.ref) return makeError({ error: 'missing-ref', message: 'click requires a "ref" field', category: 'usage', retryable: false });
        await page.click(req.ref, { timeout: WALL_CLOCK_MS });
        return { protocol: PROTOCOL, ok: true };
      }

      case 'type': {
        if (!req.ref) return makeError({ error: 'missing-ref', message: 'type requires a "ref" field', category: 'usage', retryable: false });
        if (req.text === undefined) return makeError({ error: 'missing-text', message: 'type requires a "text" field', category: 'usage', retryable: false });
        await page.fill(req.ref, req.text, { timeout: WALL_CLOCK_MS });
        return { protocol: PROTOCOL, ok: true };
      }

      case 'wait_for': {
        if (!req.text) return makeError({ error: 'missing-text', message: 'wait_for requires a "text" field', category: 'usage', retryable: false });
        const callerTimeout = req.timeout_ms ?? WALL_CLOCK_MS;
        const capped = Math.min(callerTimeout, WALL_CLOCK_MS);
        const truncated = capped < callerTimeout;
        try {
          await page.waitForSelector(`text=${req.text}`, { timeout: capped });
          const base = { protocol: PROTOCOL, ok: true };
          if (truncated) Object.assign(base, { truncated, caller_timeout_ms: callerTimeout });
          return base;
        } catch (e) {
          if (e.name === 'TimeoutError') {
            return makeError({
              error: 'wait-for-timeout',
              message: `wait_for timed out after ${capped}ms waiting for text "${req.text}"`,
              category: 'browser',
              retryable: true,
              details: { text: req.text, timeout_ms: capped, truncated, caller_timeout_ms: callerTimeout },
            });
          }
          throw e;
        }
      }

      default:
        return makeError({ error: 'unknown-command', message: `Unknown command: ${cmd}`, category: 'usage', retryable: false });
    }
  }

  // ------ HTTP server -----------------------------------------------------

  const server = createServer((req, res) => {
    const chunks = [];
    req.on('data', c => chunks.push(c));
    req.on('end', async () => {
      resetIdle();
      let parsed;
      try {
        parsed = JSON.parse(Buffer.concat(chunks).toString('utf8'));
      } catch {
        const err = makeError({ error: 'invalid-json', message: 'Request body is not valid JSON', category: 'protocol', retryable: false });
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(err));
        return;
      }

      const isBlocking = BLOCKING_OPS.has(parsed.command);
      if (isBlocking) armWallClock(parsed.command, res);

      let result;
      try {
        result = await handleRequest(parsed);
      } catch (e) {
        result = makeError({ error: 'internal-error', message: e.message || String(e), category: 'browser', retryable: false });
      }

      if (isBlocking) disarmWallClock();

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(result));
    });
  });

  // ------ Startup ---------------------------------------------------------

  await new Promise((resolve, reject) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      const url = `http://127.0.0.1:${port}/`;
      const startTime = Math.floor(Date.now() / 1000);
      writeServerInfo(stateDir, {
        protocol: PROTOCOL,
        pid: process.pid,
        start_time: startTime,
        host: '127.0.0.1',
        port,
        url,
        wall_clock_ms: WALL_CLOCK_MS,
        ready_at: new Date().toISOString(),
      });
      resolve();
    });
    server.once('error', reject);
  });

  // Redirect stdout/stderr away from terminal once ready (daemon mode)
  if (!process.env.ACCELERATOR_PLAYWRIGHT_KEEP_STDIO) {
    try {
      const { openSync } = await import('node:fs');
      const devNull = openSync('/dev/null', 'r+');
      // Don't redirect; keep logging available
    } catch {}
  }

  resetIdle();

  process.on('SIGTERM', () => shutdown('sigterm'));
  process.on('SIGINT', () => shutdown('sigint'));

  if (WALL_CLOCK_RAW > WALL_CLOCK_CEILING) {
    process.stderr.write(`inventory-design: ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=${WALL_CLOCK_RAW} exceeds ceiling; clamped to ${WALL_CLOCK_CEILING}ms\n`);
  }
}
