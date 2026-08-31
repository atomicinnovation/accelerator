// Daemon tests that need a real Playwright runtime.
//
// They live at the playwright/ root, outside the `lib/*.test.js` glob the unit
// lane discovers, because `node --test` has no way to run only some of a file's
// cases per lane. A runtime-dependent assertion therefore has to live in a file
// the unit lane never reaches, not merely be labelled opt-in inside one it does.
//
// Nothing here skips. The opt-in task's own preflight guarantees a runtime
// before any of this runs, so an absent one is a visible failure rather than a
// silent pass — the pattern this suite exists to stop reproducing.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, realpathSync } from 'node:fs';
import { fork } from 'node:child_process';
import { createServer, request } from 'node:http';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';

import { readServerInfo } from './lib/state.js';
import { requireRuntime } from './runtime-preflight.js';

const RUN_JS = resolve(import.meta.dirname, 'run.js');
const HANDOFF_FD = 4;
const TEST_TOKEN = 'runtimetokenruntimetokenruntime0';

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'daemon-runtime-')));
  return Promise.resolve(fn(dir)).finally(async () => {
    await new Promise(r => setTimeout(r, 300));
    try { rmSync(dir, { recursive: true, force: true }); } catch {}
  });
}

// The launcher's job in production: the pid is not knowable until the child
// exists, so the record travels down a pipe written after the fork.
function forkDaemon(dir, extraEnv = {}) {
  const child = fork(RUN_JS, ['daemon', '--state-dir', dir], {
    env: {
      ...process.env,
      ACCELERATOR_PLAYWRIGHT_IDENTITY_FD: String(HANDOFF_FD),
      ACCELERATOR_PLAYWRIGHT_IDLE_MS: '20000',
      ...extraEnv,
    },
    detached: false,
    stdio: ['pipe', 'pipe', 'pipe', 'ipc', 'pipe'],
  });
  child.stdio[HANDOFF_FD].end(`${child.pid}\n1700000000\np\n${TEST_TOKEN}\n`);
  return child;
}

async function send(url, body) {
  return new Promise((resolvePromise, reject) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = request(
      {
        hostname: u.hostname,
        port: u.port,
        path: '/',
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'content-length': Buffer.byteLength(data),
          'x-accelerator-token': TEST_TOKEN,
        },
      },
      res => {
        const chunks = [];
        res.on('data', c => chunks.push(c));
        res.on('end', () => {
          try { resolvePromise(JSON.parse(Buffer.concat(chunks).toString('utf8'))); }
          catch (e) { reject(e); }
        });
      }
    );
    req.on('error', reject);
    req.write(data);
    req.end();
  });
}

async function waitForInfo(stateDir, ms = 10000) {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    const info = readServerInfo(stateDir);
    if (info?.url) return info;
    await new Promise(r => setTimeout(r, 50));
  }
  throw new Error(`server-info.json did not appear within ${ms}ms in ${stateDir}`);
}

// Runs `body` against a started daemon, stopping it afterwards.
async function withDaemon(body) {
  const nsRoot = requireRuntime();
  return withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_NS_ROOT: nsRoot });
    try {
      const info = await waitForInfo(dir);
      await body(info, dir);
    } finally {
      child.kill('SIGTERM');
    }
  });
}

test('ping returns ok: true without launching a browser', { timeout: 20000 }, async () => {
  await withDaemon(async info => {
    const res = await send(info.url, { protocol: 1, command: 'ping' });
    assert.equal(res.ok, true);
    assert.equal(typeof res.node, 'string');
    assert.equal(typeof res.chromium, 'string');
  });
});

// A privacy contract over the daemon — what a crawl may hand back about a
// page's anchors. Extracting real anchors from a real page needs a real
// Chromium, so unlike the spawn properties there is no honest way to make this
// runtime-free.

const FIXTURE_URL = `file://${resolve(import.meta.dirname, '__fixtures__/links.html')}`;

async function linksOf(info, url) {
  await send(info.url, { protocol: 1, command: 'navigate', url });
  return send(info.url, { protocol: 1, command: 'links' });
}

test('links names the current page and resolves same-origin paths', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const body = JSON.stringify(res);

    assert.ok(Array.isArray(res.links), body);
    assert.ok(res.url.startsWith('file://'), body);
    assert.ok(body.includes('"pathname":"/work-items"'), body);
    assert.ok(body.includes('/library/work-items'), body);
  });
});

test('links normalises anchor text and preserves role verbatim', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const texts = res.links.map(l => l.text);
    const roles = res.links.map(l => l.role);

    assert.ok(texts.includes('Library Items'), JSON.stringify(texts));
    assert.ok(roles.includes('button'), JSON.stringify(roles));
    assert.ok(roles.includes(null), JSON.stringify(roles));
  });
});

test('every anchor on an opaque-origin page reports same_origin false', { timeout: 30000 }, async () => {
  // A file:// page has an opaque origin, so nothing can be same-origin with
  // it. Reporting otherwise would tell a crawler to follow links it should not.
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    assert.ok(res.links.length > 0);
    assert.ok(
      res.links.every(l => l.same_origin === false),
      JSON.stringify(res.links)
    );
  });
});

test('links reports each anchor scheme', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const schemes = new Set(res.links.map(l => l.scheme));
    for (const scheme of ['file', 'https', 'mailto']) {
      assert.ok(schemes.has(scheme), `${scheme} missing from ${[...schemes]}`);
    }
  });
});

test('links returns no raw href, resolved URL, query string or fragment', { timeout: 30000 }, async () => {
  // The contract that makes `links` safe to hand to a model: it describes where
  // an anchor points without echoing anything the page put in the URL.
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const body = JSON.stringify(res);

    for (const forbidden of ['"href"', '"resolved"', 'q=foo', '#top']) {
      assert.ok(!body.includes(forbidden), `${forbidden} leaked into ${body}`);
    }
  });
});

test('links on about:blank returns an empty list rather than an error', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, 'about:blank');
    assert.equal(res.url, 'about:blank');
    assert.deepEqual(res.links, []);
  });
});

// Navigation classification over a real browser: the route handler must fire per
// redirect hop and per page-initiated navigation, which a mocked route cannot
// prove. A link-local host stands in for the metadata endpoint; the loopback
// server that redirects to it is itself always allowed, so only the hop can
// produce a link-local refusal.

const INTERNAL_URL = 'http://169.254.169.254/';

async function withServer(handler, body) {
  const server = createServer(handler);
  await new Promise(started => server.listen(0, '127.0.0.1', started));
  const url = `http://127.0.0.1:${server.address().port}/`;
  try {
    return await body(url);
  } finally {
    await new Promise(closed => server.close(closed));
  }
}

function redirectTo(target) {
  return (_req, res) => {
    res.writeHead(302, { location: target });
    res.end();
  };
}

function serveHtml(html) {
  return (_req, res) => {
    res.writeHead(200, { 'content-type': 'text/html' });
    res.end(html);
  };
}

test('a redirect to a link-local host is refused, not followed', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    await withServer(redirectTo(INTERNAL_URL), async serverUrl => {
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(res.error, 'navigation-refused', JSON.stringify(res));
      assert.equal(res.retryable, false, JSON.stringify(res));
      assert.equal(res.details.classification, 'link-local');
      assert.ok(!res.message.includes('?'), res.message);
    });
  });
});

test('allow_internal lets a link-local redirect past classification', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    await withServer(redirectTo(INTERNAL_URL), async serverUrl => {
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
        allow_internal: true,
      });
      // Classification allows the hop; the fetch to a dead link-local host then
      // fails, so the outcome is a connection error, never navigation-refused.
      assert.notEqual(res.error, 'navigation-refused', JSON.stringify(res));
    });
  });
});

test('two navigations on one daemon are judged under their own allowances', { timeout: 40000 }, async () => {
  await withDaemon(async info => {
    await withServer(redirectTo(INTERNAL_URL), async serverUrl => {
      const allowed = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
        allow_internal: true,
      });
      assert.notEqual(
        allowed.error,
        'navigation-refused',
        JSON.stringify(allowed)
      );
      const refused = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(refused.error, 'navigation-refused', JSON.stringify(refused));
      assert.equal(refused.details.classification, 'link-local');
    });
  });
});

test('a sub-frame navigation to an internal host does not mask the main frame', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const html = `<!doctype html><meta charset=utf8><iframe src="${INTERNAL_URL}"></iframe>main`;
    await withServer(serveHtml(html), async serverUrl => {
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(res.ok, true, JSON.stringify(res));
    });
  });
});

test('a click that navigates to an internal host is refused', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const html = `<!doctype html><meta charset=utf8><a id="go" href="${INTERNAL_URL}">go</a>`;
    await withServer(serveHtml(html), async serverUrl => {
      await send(info.url, { protocol: 1, command: 'navigate', url: serverUrl });
      const res = await send(info.url, {
        protocol: 1,
        command: 'click',
        ref: '#go',
      });
      assert.equal(res.error, 'navigation-refused', JSON.stringify(res));
      assert.equal(res.details.classification, 'link-local');
    });
  });
});

test('a scripted redirect to an internal host after load is aborted', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const html =
      `<!doctype html><meta charset=utf8><body>ok<script>` +
      `setTimeout(function(){location.href=${JSON.stringify(INTERNAL_URL)};},50);` +
      `</script></body>`;
    await withServer(serveHtml(html), async serverUrl => {
      const nav = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(nav.ok, true, JSON.stringify(nav));
      await new Promise(r => setTimeout(r, 400));
      const location = await send(info.url, {
        protocol: 1,
        command: 'evaluate',
        expression: 'location.href',
      });
      assert.ok(
        !String(location.result).startsWith('http://169.254'),
        JSON.stringify(location)
      );
    });
  });
});
