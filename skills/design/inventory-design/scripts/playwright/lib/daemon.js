// Playwright browser-context daemon server.
// Launched via: node run.js daemon --state-dir <dir>
//
// Listens on 127.0.0.1:0 (OS-assigned port). Writes server-info.json and
// server.pid atomically once ready. Handles a single sequential JSON
// request per TCP connection. Shuts down on SIGTERM, SIGINT, idle timeout,
// or per-op wall-clock timeout.

import { createServer } from 'node:http';
import { timingSafeEqual } from 'node:crypto';
import { writeServerInfo, writeServerStopped, removeServerFiles, ensureStateDir } from './state.js';
import { readIdentity } from './identity-handoff.js';
import { makeAuthHeaderHandler } from './auth-header.js';
import { mergeMaskSelectors } from './mask.js';
import { guardScreenshotPath } from './path-guard.js';
import { makeError, protocolMismatch, PROTOCOL, TOKEN_HEADER } from './errors.js';
import { importPlaywright, resolvePlaywrightPkgPath } from './playwright-loader.js';

// 10-min default balances cross-turn daemon reuse (Claude Code sessions
// typically span minutes) against bounding the lifetime of an
// auth-bearing browser context held in memory. Override via
// ACCELERATOR_PLAYWRIGHT_IDLE_MS.
const IDLE_MS = parseInt(process.env.ACCELERATOR_PLAYWRIGHT_IDLE_MS || '600000', 10);
const WALL_CLOCK_RAW = parseInt(process.env.ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS || '300000', 10);
const WALL_CLOCK_CEILING = 1800000; // 30 min hard ceiling
const WALL_CLOCK_MS = Math.min(WALL_CLOCK_RAW, WALL_CLOCK_CEILING);

// How long past the budget the backstop waits. It sits deliberately outside
// WALL_CLOCK_MS — and so outside the ceiling — because it guards the operations
// that ignore their own timeout entirely, `page.evaluate` above all, rather than
// the ones that honour it.
const WALL_CLOCK_GRACE_MS = 2000;

// BLOCKING_OPS membership: operations that perform browser I/O and must
// be wall-clock bounded. `links` joins the set because page.evaluate()
// can hang on a hostile page, mirroring the existing `evaluate` and
// `snapshot` entries.
const BLOCKING_OPS = new Set(['navigate', 'snapshot', 'links', 'screenshot', 'evaluate', 'click', 'type', 'wait_for']);

export async function startDaemon({ stateDir }) {
  // Before the state dir, before any browser, before the listening socket: if
  // the launcher did not complete its handoff there is nothing to publish, and
  // a daemon that ran on anyway would be unsupervised and un-recorded.
  const identity = readIdentity();

  ensureStateDir(stateDir);

  let browser = null;
  let page = null;
  let shutdownInitiated = false;
  let idleTimer = null;
  let wallClockTimer = null;

  // ------ Shutdown --------------------------------------------------------

  async function shutdown(reason, extra = {}) {
    if (shutdownInitiated) return;
    shutdownInitiated = true;

    clearTimeout(idleTimer);
    clearTimeout(wallClockTimer);

    try { writeServerStopped(stateDir, reason, extra); } catch {}

    // Stop being discoverable before the browser goes away. Closing the
    // browser first leaves a window as long as Chromium takes to exit in
    // which the launcher's reuse check still passes — the pid is live and
    // server-info.json is present — so the next launcher dispatches onto a
    // dead page and the caller sees `Target page, context or browser has
    // been closed` instead of a clean respawn.
    removeServerFiles(stateDir);

    try { await browser?.close(); } catch {}

    server.close(() => process.exit(reason === 'wall-clock' ? 2 : 0));
    setTimeout(() => process.exit(reason === 'wall-clock' ? 2 : 0), 3000).unref();
  }

  // ------ Idle timer ------------------------------------------------------

  function resetIdle() {
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => shutdown('idle'), IDLE_MS).unref();
  }

  // ------ Responding ------------------------------------------------------

  // Both the operation and its backstop can reach the same response, and only
  // one of them may answer: the backstop fires precisely when the operation is
  // still running, so a later completion must not write a second body onto a
  // finished response.
  function respond(res, status, body) {
    if (res.writableEnded || res.headersSent) return false;
    res.writeHead(status, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(body));
    return true;
  }

  // ------ Wall-clock per-op timer ----------------------------------------

  // The backstop fires after the budget, not on it. Every bounded operation
  // passes WALL_CLOCK_MS to Playwright as its own timeout, so a backstop armed
  // for the same instant pre-empts the graceful path it exists to protect: the
  // caller gets a dead daemon where the operation was about to return a typed
  // envelope. The grace is what leaves room for that envelope.
  function armWallClock(opName, res) {
    clearTimeout(wallClockTimer);
    wallClockTimer = setTimeout(async () => {
      const envelope = makeError({
        error: 'wall-clock-exceeded',
        message: `Operation exceeded the ${WALL_CLOCK_MS}ms wall-clock budget.`,
        category: 'browser',
        retryable: false,
        details: { op: opName, wall_clock_ms: WALL_CLOCK_MS },
      });
      // A complete response, not a raw write. Writing the envelope onto the
      // socket and then exiting leaves an HTTP client waiting on a chunked body
      // that never ends — so the one caller who needs to hear about this is the
      // one caller who never does.
      respond(res, 200, envelope);
      await shutdown('wall-clock', { op: opName, wall_clock_ms: WALL_CLOCK_MS });
    }, WALL_CLOCK_MS + WALL_CLOCK_GRACE_MS);
  }

  function disarmWallClock() {
    clearTimeout(wallClockTimer);
    wallClockTimer = null;
  }

  // ------ Browser / page --------------------------------------------------

  async function ensureBrowser() {
    if (browser) return;
    const { chromium } = await importPlaywright({
      nsRoot: process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT,
    });
    browser = await chromium.launch({ headless: true });
    const ctx = await browser.newContext();
    page = await ctx.newPage();
  }

  // ------ Request handler -------------------------------------------------

  async function handleRequest(req, { onBrowserReady } = {}) {
    if (req.protocol !== PROTOCOL) return protocolMismatch(req.protocol);

    const cmd = req.command;

    if (cmd === 'ping') {
      const nsRoot = process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT;
      const { chromium: cr } = await importPlaywright({ nsRoot });
      const execPath = cr.executablePath();
      try {
        await import('node:fs').then(({ promises }) => promises.access(execPath));
      } catch {
        return makeError({ error: 'chromium-not-found', message: `Chromium binary not found at ${execPath}. Run ensure-playwright.sh to reinstall.`, category: 'bootstrap', retryable: false, details: { execPath } });
      }
      let pv = 'unknown';
      try {
        const { readFile } = await import('node:fs/promises');
        const pkgFile = nsRoot
          ? resolvePlaywrightPkgPath(nsRoot)
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

    // A launcher that cleared the reuse check moments before shutdown began
    // can still arrive on an already-open connection. Answer with a typed,
    // retryable envelope rather than letting the closed browser surface as a
    // Playwright exception.
    if (shutdownInitiated) {
      return makeError({
        error: 'daemon-stopping',
        message: 'The daemon is shutting down. Run the command again; the launcher will spawn a new one.',
        category: 'browser',
        retryable: true,
      });
    }

    // All remaining commands require the browser
    await ensureBrowser();
    onBrowserReady?.();

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

      case 'links': {
        const pageUrl = page.url();
        const links = await page.evaluate(() => {
          const pageOrigin = location.origin;
          const pageProtocol = location.protocol;
          // Opaque-origin schemes per HTML spec: file:, data:, javascript:,
          // blob:. Chromium reports varying origin strings for these
          // (e.g. 'null', 'file://') depending on the build, so use the
          // scheme directly as the opaque-origin signal rather than
          // relying on origin-string equality.
          const OPAQUE = new Set(['file:', 'data:', 'javascript:', 'blob:']);
          return Array.from(document.querySelectorAll('a[href]')).map(a => {
            let pathname = null;
            let sameOrigin = false;
            let scheme = null;
            try {
              const u = new URL(a.href);
              pathname = u.pathname;
              scheme = u.protocol.replace(':', '');
              // Same-origin check: both origins must match AND neither
              // side may be an opaque-origin scheme. Without the opaque
              // guard a `mailto:` or `javascript:void(0)` anchor on a
              // file:// page would be marked as same-origin, which is
              // wrong both semantically and as a security signal (the
              // locator's route-following rule trusts same_origin).
              sameOrigin = u.origin === pageOrigin
                && u.origin !== 'null'
                && !OPAQUE.has(u.protocol)
                && !OPAQUE.has(pageProtocol);
            } catch {
              // href could not be resolved against the current document
              // (e.g. malformed). Leave the derived fields null.
            }
            return {
              text: (a.textContent || '').replace(/\s+/g, ' ').trim(),
              pathname,
              same_origin: sameOrigin,
              scheme,
              role: a.getAttribute('role'),
            };
          });
        });
        return { protocol: PROTOCOL, url: pageUrl, links };
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

  // ------ Request authentication ------------------------------------------

  // A loopback TCP socket is not a uid boundary on Unix. A different local
  // user on a shared host cannot read the 0600 server-info.json holding this
  // token, but could otherwise connect to this port and drive the crawl's
  // browser session — and the pages the crawl itself visits could reach it by
  // CSRF or DNS rebinding. The token closes both.
  //
  // It does NOT defend against a same-uid process, which can simply read the
  // token file. That is stated rather than implied, so nobody mistakes this
  // for a sandbox.
  function authenticate(req) {
    // A legitimate client has no reason to send one, and a browser cannot
    // suppress it — so its presence means the request came from a page.
    if (req.headers.origin !== undefined) {
      return makeError({
        error: 'origin-rejected',
        message: 'Requests carrying an Origin header are refused: this daemon serves no browser origin.',
        category: 'usage',
        retryable: false,
      });
    }

    // Header only. A query parameter lands in logs, referrers and process
    // listings, so a valid token presented that way is still refused.
    const url = new URL(req.url || '/', 'http://127.0.0.1');
    if (url.searchParams.has(TOKEN_HEADER) || url.searchParams.has('token')) {
      return makeError({
        error: 'token-rejected',
        message: 'The request token must be sent as a header, never as a query parameter.',
        category: 'usage',
        retryable: false,
      });
    }

    if (!tokenMatches(req.headers[TOKEN_HEADER])) {
      return makeError({
        error: 'unauthenticated',
        message: 'Missing or invalid request token.',
        category: 'usage',
        retryable: false,
      });
    }
    return null;
  }

  function tokenMatches(presented) {
    if (typeof presented !== 'string') return false;
    const expected = Buffer.from(identity.token, 'utf8');
    const actual = Buffer.from(presented, 'utf8');
    // timingSafeEqual throws on a length mismatch, so the lengths are compared
    // first. That leaks only the length, which is fixed and public anyway.
    if (expected.length !== actual.length) return false;
    return timingSafeEqual(expected, actual);
  }

  // ------ HTTP server -----------------------------------------------------

  const server = createServer((req, res) => {
    // Refused before the body is read, from the very first accepted
    // connection.
    const refusal = authenticate(req);
    if (refusal) {
      res.writeHead(403, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(refusal));
      req.resume();
      return;
    }

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

      let result;
      try {
        result = await handleRequest(parsed, {
          // Armed once the browser exists, not before the call: acquiring
          // Chromium can outrun any sane per-op budget, and charging a cold
          // start to the operation kills the daemon on its first browser
          // command. The hook fires after `ensureBrowser`, so the commands that
          // return before it — and a request arriving mid-shutdown — arm nothing.
          onBrowserReady: () => {
            if (isBlocking) armWallClock(parsed.command, res);
          },
        });
      } catch (e) {
        result = makeError({ error: 'internal-error', message: e.message || String(e), category: 'browser', retryable: false });
      }

      if (isBlocking) disarmWallClock();

      respond(res, 200, result);
    });
  });

  // ------ Startup ---------------------------------------------------------

  await new Promise((resolve, reject) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      const url = `http://127.0.0.1:${port}/`;
      writeServerInfo(stateDir, {
        protocol: PROTOCOL,
        // The launcher's own observation, relayed rather than recomputed.
        pid: identity.pid,
        start_time: identity.start_time,
        start_time_source: identity.start_time_source,
        token: identity.token,
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

  resetIdle();

  process.on('SIGTERM', () => shutdown('sigterm'));
  process.on('SIGINT', () => shutdown('sigint'));

  if (WALL_CLOCK_RAW > WALL_CLOCK_CEILING) {
    process.stderr.write(`inventory-design: ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=${WALL_CLOCK_RAW} exceeds ceiling; clamped to ${WALL_CLOCK_CEILING}ms\n`);
  }
}
