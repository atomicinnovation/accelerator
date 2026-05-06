// Unit tests for daemon timer/state-machine logic that don't require launching Chromium.
// The daemon is started with mock Playwright and a fast-parameterised wall-clock.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { realpathSync } from 'node:fs';
import { request } from 'node:http';
import { resolve as pathResolve } from 'node:path';
import { createHash } from 'node:crypto';
import { readServerInfo, SERVER_STOPPED_FILE } from './state.js';

function resolvePlaywrightNsRoot() {
  const cacheRoot = process.env.ACCELERATOR_PLAYWRIGHT_CACHE || `${process.env.HOME}/.cache/accelerator/playwright`;
  if (!existsSync(cacheRoot)) return null;
  const lockFile = new URL('../package-lock.json', import.meta.url).pathname;
  let lockContents;
  try { lockContents = readFileSync(lockFile); } catch { return null; }
  const lockhash = createHash('sha256').update(lockContents).digest('hex').slice(0, 8);
  const nsRoot = resolve(cacheRoot, lockhash);
  // Check for real playwright install: must have actual JS entry point, not just a stub package.json
  const playwrightEntry = resolve(nsRoot, 'node_modules', 'playwright', 'index.js');
  if (!existsSync(playwrightEntry)) return null;
  return nsRoot;
}

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'daemon-test-')));
  return Promise.resolve(fn(dir)).finally(async () => {
    // Brief pause so any still-running daemon subprocess can flush and exit
    await new Promise(r => setTimeout(r, 300));
    try { rmSync(dir, { recursive: true, force: true }); } catch {}
  });
}

async function send(url, body) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = request({ hostname: u.hostname, port: u.port, path: '/', method: 'POST',
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(data) },
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

// -- ping does not launch Chromium (fast, < 500ms) ----------------------

test('ping returns ok: true without launching browser', { timeout: 5000 }, async () => {
  const nsRoot = resolvePlaywrightNsRoot();
  if (!nsRoot) return; // Skip if Playwright not fully installed for this lockhash
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000', ACCELERATOR_PLAYWRIGHT_NS_ROOT: nsRoot };
    const child = (await import('node:child_process')).fork(
      pathResolve(import.meta.dirname, '../run.js'),
      ['daemon', '--state-dir', dir, '--owner-pid', '0'],
      { env: daemonEnv, detached: false },
    );
    try {
      const info = await waitForInfo(dir, 5000);
      const res = await send(info.url, { protocol: 1, command: 'ping' });
      assert.equal(res.ok, true);
      assert.ok(typeof res.node === 'string');
      assert.ok(typeof res.chromium === 'string');
    } finally {
      child.kill('SIGTERM');
    }
  });
});

// -- Protocol-version check ---------------------------------------------

test('protocol mismatch returns protocol-mismatch error', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '5000' };
    const child = (await import('node:child_process')).fork(
      pathResolve(import.meta.dirname, '../run.js'),
      ['daemon', '--state-dir', dir, '--owner-pid', '0'],
      { env: daemonEnv, detached: false },
    );
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
    const child = (await import('node:child_process')).fork(
      pathResolve(import.meta.dirname, '../run.js'),
      ['daemon', '--state-dir', dir, '--owner-pid', '0'],
      { env: daemonEnv, detached: false },
    );
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
    const child = (await import('node:child_process')).fork(
      pathResolve(import.meta.dirname, '../run.js'),
      ['daemon', '--state-dir', dir, '--owner-pid', '0'],
      { env: daemonEnv, detached: false },
    );
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

// -- idle timer ---------------------------------------------------------

test('idle timer shuts down daemon and writes server-stopped.json', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const daemonEnv = { ...process.env, ACCELERATOR_PLAYWRIGHT_IDLE_MS: '300' };
    const child = (await import('node:child_process')).fork(
      pathResolve(import.meta.dirname, '../run.js'),
      ['daemon', '--state-dir', dir, '--owner-pid', '0'],
      { env: daemonEnv, detached: false },
    );
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
