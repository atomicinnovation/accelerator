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
import { request } from 'node:http';
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
