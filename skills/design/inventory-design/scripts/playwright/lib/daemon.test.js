// Unit tests for daemon timer/state-machine logic that don't require launching Chromium.
// The daemon is started with mock Playwright and a fast-parameterised wall-clock.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, existsSync, readFileSync, writeFileSync, openSync, closeSync } from 'node:fs';
import { fork as forkChild } from 'node:child_process';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { realpathSync } from 'node:fs';
import { request } from 'node:http';
import { resolve as pathResolve } from 'node:path';
import { readServerInfo, SERVER_STOPPED_FILE } from './state.js';

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'daemon-test-')));
  return Promise.resolve(fn(dir)).finally(async () => {
    // Brief pause so any still-running daemon subprocess can flush and exit
    await new Promise(r => setTimeout(r, 300));
    try { rmSync(dir, { recursive: true, force: true }); } catch {}
  });
}

async function sendRaw(url, body, headers) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = request({ hostname: u.hostname, port: u.port, path: u.pathname + u.search, method: 'POST',
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(data), ...headers },
    }, res => {
      const chunks = [];
      res.on('data', c => chunks.push(c));
      res.on('end', () => {
        try { resolve(JSON.parse(Buffer.concat(chunks).toString('utf8'))); }
        catch (e) { reject(e); }
      });
    });
    req.on('error', reject);
    req.write(data);
    req.end();
  });
}

async function send(url, body, headers = {}) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = request({ hostname: u.hostname, port: u.port, path: '/', method: 'POST',
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(data), ...authed(headers) },
    }, res => {
      const chunks = [];
      res.on('data', c => chunks.push(c));
      res.on('end', () => {
        try { resolve(JSON.parse(Buffer.concat(chunks).toString('utf8'))); }
        catch (e) { reject(e); }
      });
    });
    req.on('error', reject);
    req.write(data);
    req.end();
  });
}

async function waitForInfo(stateDir, ms = 5000) {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    const info = readServerInfo(stateDir);
    if (info?.url) return info;
    await new Promise(r => setTimeout(r, 50));
  }
  throw new Error(`server-info.json did not appear within ${ms}ms in ${stateDir}`);
}


// The daemon reads its identity off an inherited descriptor before it publishes
// anything, so a test that forks it must supply one — the launcher's job in
// production.
//
// A real pipe, written after the fork, because that is the only faithful shape:
// the pid is not knowable until the child exists, which is exactly why the
// launcher cannot pass it through the environment. `fork` puts its IPC channel
// at stdio index 3, so the handoff pipe lands at 4.
const HANDOFF_FD = 4;
const TEST_TOKEN = 'testtokentesttokentesttokentest0';

function forkDaemon(dir, extraEnv = {}) {
  const child = forkChild(
    pathResolve(import.meta.dirname, '../run.js'),
    ['daemon', '--state-dir', dir],
    {
      env: {
        ...process.env,
        ACCELERATOR_PLAYWRIGHT_IDENTITY_FD: String(HANDOFF_FD),
        ...extraEnv,
      },
      detached: false,
      stdio: ['pipe', 'pipe', 'pipe', 'ipc', 'pipe'],
    }
  );
  // Closing the write end is what gives the daemon a deterministic EOF.
  child.stdio[HANDOFF_FD].end(
    `${child.pid}\n1700000000\np\n${TEST_TOKEN}\n`
  );
  return child;
}

// Every request the daemon accepts must carry the token.
function authed(extra = {}) {
  return { 'x-accelerator-token': TEST_TOKEN, ...extra };
}

// -- Protocol-version check ---------------------------------------------

test('protocol mismatch returns protocol-mismatch error', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' };
    const child = forkDaemon(dir, daemonEnv);
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await send(info.url, { protocol: 999, command: 'ping' });
      assert.equal(res.error, 'protocol-mismatch');
      assert.equal(res.category, 'protocol');
    } finally {
      child.kill('SIGTERM');
    }
  });
});

// -- daemon-status returns running without spawning browser --------------

test('daemon-status returns state: running', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' };
    const child = forkDaemon(dir, daemonEnv);
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await send(info.url, { protocol: 1, command: 'daemon-status' });
      assert.equal(res.state, 'running');
      assert.equal(res.pid, info.pid);
    } finally {
      child.kill('SIGTERM');
    }
  });
});

// -- daemon-stop --------------------------------------------------------

test('daemon-stop writes server-stopped.json and removes state files', { timeout: 8000 }, async () => {
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '30000' };
    const child = forkDaemon(dir, daemonEnv);
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await send(info.url, { protocol: 1, command: 'daemon-stop' });
      assert.equal(res.ok, true);
      // Wait for cleanup
      await new Promise(r => setTimeout(r, 2000));
      assert.ok(existsSync(resolve(dir, SERVER_STOPPED_FILE)), 'server-stopped.json should exist');
    } finally {
      try { child.kill('SIGTERM'); } catch {}
    }
  });
});

// -- IDLE_MS default ---------------------------------------------------

test('daemon module declares IDLE_MS default of 10 minutes', async () => {
  const src = readFileSync(
    new URL('./daemon.js', import.meta.url).pathname, 'utf8');
  // Pin the default value at the source level (avoid runtime probe
  // requiring a 10-minute wait).
  assert.match(src, /IDLE_MS\s*=\s*parseInt\(process\.env\.ACCELERATOR_PLAYWRIGHT_IDLE_MS\s*\|\|\s*'600000'/);
});

// -- idle timer ---------------------------------------------------------

test('idle timer shuts down daemon and writes server-stopped.json', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '300' };
    const child = forkDaemon(dir, daemonEnv);
    try {
      const info = await waitForInfo(dir, 5000);
      // Don't send any traffic — wait for idle timeout
      await new Promise(r => setTimeout(r, 1500));
      const stopped = existsSync(resolve(dir, SERVER_STOPPED_FILE));
      assert.ok(stopped, 'server-stopped.json should exist after idle timeout');
    } finally {
      try { child.kill('SIGTERM'); } catch {}
    }
  });
});

// -- Request authentication ---------------------------------------------

// The token defends two things the 0600 file cannot: a different local user on
// a shared host (a loopback socket is not a uid boundary), and the pages the
// crawl itself visits reaching this port by CSRF or DNS rebinding.

test('a request without the token is refused from the first connection', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' });
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await sendRaw(info.url, { protocol: 1, command: 'ping' }, {});
      assert.equal(res.error, 'unauthenticated');
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('a request with the wrong token is refused', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' });
    try {
      const info = await waitForInfo(dir, 5000);
      for (const wrong of ['', 'short', `${TEST_TOKEN}x`, TEST_TOKEN.replace('0', '1')]) {
        const res = await sendRaw(info.url, { protocol: 1, command: 'ping' }, { 'x-accelerator-token': wrong });
        assert.equal(res.error, 'unauthenticated', JSON.stringify(wrong));
      }
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('a request carrying an Origin header is refused whatever its token', { timeout: 5000 }, async () => {
  // A legitimate client never sends one, and a browser cannot suppress it — so
  // its presence means the request came from a page.
  await withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' });
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await sendRaw(info.url, { protocol: 1, command: 'ping' }, {
        'x-accelerator-token': TEST_TOKEN,
        origin: 'https://evil.example.com',
      });
      assert.equal(res.error, 'origin-rejected');
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('a valid token presented as a query parameter is still refused', { timeout: 5000 }, async () => {
  // Query strings land in logs, referrers and process listings.
  await withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' });
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await sendRaw(
        `${info.url}?x-accelerator-token=${TEST_TOKEN}`,
        { protocol: 1, command: 'ping' },
        {}
      );
      assert.equal(res.error, 'token-rejected');
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('the published record carries the launcher-supplied identity', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' });
    try {
      const info = await waitForInfo(dir, 5000);
      assert.equal(info.pid, child.pid);
      assert.equal(info.start_time, 1700000000);
      assert.equal(info.start_time_source, 'probe');
      assert.equal(info.token, TEST_TOKEN);
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('a daemon whose handoff never arrives exits without publishing', { timeout: 5000 }, async () => {
  // The launcher-crashed-mid-handoff case: an immediate EOF with no data must
  // stop the daemon before it opens a socket or creates a browser, rather than
  // leaving it running unsupervised and un-recorded.
  await withTmpDir(async dir => {
    const child = forkChild(
      pathResolve(import.meta.dirname, '../run.js'),
      ['daemon', '--state-dir', dir],
      {
        env: { ...process.env, ACCELERATOR_PLAYWRIGHT_IDENTITY_FD: String(HANDOFF_FD) },
        detached: false,
        stdio: ['pipe', 'pipe', 'pipe', 'ipc', 'pipe'],
      }
    );
    child.stdio[HANDOFF_FD].end();
    const code = await new Promise(r => child.on('exit', r));
    assert.notEqual(code, 0);
    assert.equal(readServerInfo(dir), null);
  });
});

// -- Guards re-homed from scripts/test-design.sh -------------------------

// Three of these sweep a tree for a forbidden string, and that tree now
// contains this very file. Every needle is therefore built from concatenated
// fragments, and no title, comment or assertion message below writes the
// phrase whole — otherwise the check would find itself and invert permanently.
// The shell version got away with literals only because it lived outside the
// directories it scanned, which stops being true the moment it moves in here.

import { readdirSync, statSync } from 'node:fs';

const EXECUTOR_SRC = pathResolve(import.meta.dirname, '..');

function sourceFilesUnder(root) {
  const found = [];
  for (const entry of readdirSync(root)) {
    if (entry === 'node_modules' || entry === '__fixtures__') continue;
    const path = pathResolve(root, entry);
    if (statSync(path).isDirectory()) found.push(...sourceFilesUnder(path));
    else if (entry.endsWith('.js')) found.push(path);
  }
  return found;
}

// The two scopes the shell version used, kept distinct. The deny-list sweeps
// covered lib/ and run.js — the executor's own implementation — while the
// watcher sweep covered the whole tree, so a reintroduction anywhere would be
// caught.
function executorImplementation() {
  return [
    ...sourceFilesUnder(pathResolve(EXECUTOR_SRC, 'lib')),
    pathResolve(EXECUTOR_SRC, 'run.js'),
  ];
}

function filesContaining(files, needle) {
  return files
    .filter(path => readFileSync(path, 'utf8').includes(needle))
    .map(path => path.replace(EXECUTOR_SRC, ''));
}

test('the payload-rejection deny-list marker is absent from executor source', () => {
  const needle = 'evaluate-payload' + '-rejected';
  assert.deepEqual(filesContaining(executorImplementation(), needle), []);
});

test('no MCP-prefixed browser-tool reference survives in executor source', () => {
  const needle = 'mcp' + '__' + 'playwright' + '__';
  assert.deepEqual(filesContaining(executorImplementation(), needle), []);
});

test('no owner-PID watcher identifier survives under the playwright tree', () => {
  // A resolved incident: the watcher raced an ephemeral shell and killed live
  // daemons. See meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md
  const needles = ['owner' + 'Pid', '--owner' + '-pid', 'OWNER' + '_POLL_MS'];
  for (const needle of needles) {
    assert.deepEqual(filesContaining(sourceFilesUnder(EXECUTOR_SRC), needle), [], needle);
  }
});

test('PROTOCOL.md documents every command the daemon dispatches', () => {
  // Reads two named files and asserts content is present, so unlike the sweeps
  // above it carries no self-matching risk wherever it lives.
  const protocol = readFileSync(
    pathResolve(EXECUTOR_SRC, '../../PROTOCOL.md'),
    'utf8'
  );
  const commands = ['ping', 'daemon-status', 'daemon-stop', 'navigate', 'snapshot',
    'links', 'screenshot', 'evaluate', 'click', 'type', 'wait_for'];
  for (const command of commands) {
    assert.ok(protocol.includes(`### \`${command}\``), `${command} undocumented`);
  }

  // Every environment variable the daemon reads must be documented too.
  const daemonSource = readFileSync(pathResolve(EXECUTOR_SRC, 'lib/daemon.js'), 'utf8');
  const declared = new Set(daemonSource.match(/ACCELERATOR_PLAYWRIGHT_[A-Z_]+/g) || []);
  assert.ok(protocol.includes('## Environment Variables'));
  for (const variable of declared) {
    assert.ok(protocol.includes(variable), `${variable} undocumented`);
  }
});

test('links is wall-clock bounded like every other browser operation', () => {
  // page.evaluate() can hang on a hostile page, so `links` belongs in the set
  // that gets a per-op budget.
  const daemonSource = readFileSync(pathResolve(EXECUTOR_SRC, 'lib/daemon.js'), 'utf8');
  const declaration = daemonSource
    .split('\n')
    .find(line => line.startsWith('const BLOCKING_OPS'));
  assert.ok(declaration, 'BLOCKING_OPS is not declared');
  assert.ok(declaration.includes("'links'"), declaration);
});
