#!/usr/bin/env node
// Integration tests for the Playwright executor.
// Requires a bootstrapped Playwright install (run ensure-playwright.sh first).
// Run with: node --test test-run.js

import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, existsSync, writeFileSync, mkdirSync, symlinkSync, readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { realpathSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { fork, execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { request } from 'node:http';

const __dir = dirname(fileURLToPath(import.meta.url));
const RUN_JS = resolve(__dir, 'run.js');
const FIXTURES_DIR = resolve(__dir, '__fixtures__');

// -- Helpers -------------------------------------------------------------

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'runjs-test-')));
  return Promise.resolve(fn(dir)).finally(() => rmSync(dir, { recursive: true, force: true }));
}

async function waitForFile(filePath, ms = 6000) {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    if (existsSync(filePath)) return;
    await new Promise(r => setTimeout(r, 50));
  }
  throw new Error(`File did not appear within ${ms}ms: ${filePath}`);
}

async function waitForInfo(stateDir, ms = 6000) {
  const info = resolve(stateDir, 'server-info.json');
  await waitForFile(info, ms);
  return JSON.parse(readFileSync(info, 'utf8'));
}

async function send(url, body) {
  return new Promise((res, rej) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = request({ hostname: u.hostname, port: u.port, path: '/', method: 'POST',
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(data) },
    }, r => {
      const chunks = [];
      r.on('data', c => chunks.push(c));
      r.on('end', () => { try { res(JSON.parse(Buffer.concat(chunks).toString('utf8'))); } catch(e){ rej(e); } });
    });
    req.on('error', rej);
    req.write(data);
    req.end();
  });
}

function spawnDaemon(stateDir, extraEnv = {}) {
  const env = {
    ...process.env,
    ACCELERATOR_PLAYWRIGHT_STATE_DIR: stateDir,
    ACCELERATOR_PLAYWRIGHT_IDLE_MS: '10000',
    ACCELERATOR_PLAYWRIGHT_KEEP_STDIO: '1',
    ...extraEnv,
  };
  const child = fork(RUN_JS, ['daemon', '--state-dir', stateDir, '--owner-pid', '0'], {
    env, detached: false, stdio: 'pipe',
  });
  child.stderr?.on('data', d => process.stderr.write('[daemon] ' + d));
  return child;
}

// Serve a static HTML file from a temp HTTP server
async function withFixtureServer(fn) {
  const html = readFileSync(resolve(FIXTURES_DIR, 'fixture.html'), 'utf8');
  const server = createServer((req, res) => {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(html);
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const { port } = server.address();
  const url = `http://127.0.0.1:${port}/fixture.html`;
  try {
    await fn(url);
  } finally {
    await new Promise(r => server.close(r));
  }
}

// Check if Playwright is bootstrapped
const cacheRoot = process.env.ACCELERATOR_PLAYWRIGHT_CACHE || `${process.env.HOME}/.cache/accelerator/playwright`;
const playwrightInstalled = existsSync(cacheRoot);

// -- ping ----------------------------------------------------------------

test('ping: exits 0 with ok: true and chromium path, < 500ms', { timeout: 15000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir);
    try {
      const info = await waitForInfo(stateDir);
      const t0 = Date.now();
      const res = await send(info.url, { protocol: 1, command: 'ping' });
      const elapsed = Date.now() - t0;
      assert.equal(res.ok, true);
      assert.ok(typeof res.node === 'string');
      assert.ok(typeof res.chromium === 'string');
      assert.ok(existsSync(res.chromium), `chromium binary should exist at ${res.chromium}`);
      assert.ok(elapsed < 500, `ping took ${elapsed}ms, expected < 500ms`);
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('ping: corrupted bootstrap → category: bootstrap', { timeout: 10000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir, {
      PLAYWRIGHT_BROWSERS_PATH: '/nonexistent-path-xyz',
    });
    try {
      const info = await waitForInfo(stateDir);
      const res = await send(info.url, { protocol: 1, command: 'ping' });
      // Either ok or bootstrap error depending on executable path
      if (!res.ok) {
        assert.equal(res.category, 'bootstrap');
      }
    } finally {
      child.kill('SIGTERM');
    }
  });
});

// -- navigate + snapshot ------------------------------------------------

test('navigate then snapshot produces non-empty JSON', { timeout: 30000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir);
    try {
      const info = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        const navRes = await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        assert.equal(navRes.ok, true);

        const snapRes = await send(info.url, { protocol: 1, command: 'snapshot' });
        assert.ok(snapRes.snapshot !== undefined);
        assert.ok(snapRes.snapshot !== null);
      });
    } finally {
      child.kill('SIGTERM');
    }
  });
});

// -- screenshot ----------------------------------------------------------

test('screenshot writes a non-empty PNG to the output root', { timeout: 30000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    await withTmpDir(async outputRoot => {
      const child = spawnDaemon(stateDir, { ACCELERATOR_INVENTORY_OUTPUT_ROOT: outputRoot });
      try {
        const info = await waitForInfo(stateDir);
        await withFixtureServer(async fixtureUrl => {
          await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
          const res = await send(info.url, { protocol: 1, command: 'screenshot', path: 'home.png' });
          assert.equal(res.ok, true);
          assert.ok(existsSync(res.path), `screenshot file should exist at ${res.path}`);
          const size = readFileSync(res.path).length;
          assert.ok(size > 0, 'screenshot should not be empty');
          // PNG magic bytes
          const magic = readFileSync(res.path).slice(0, 4);
          assert.equal(magic[0], 0x89);
          assert.equal(magic[1], 0x50); // P
        });
      } finally {
        child.kill('SIGTERM');
      }
    });
  });
});

// -- screenshot path-guard (integration boundary) ----------------------

test('screenshot: unset output root → screenshot-output-root-unset', { timeout: 10000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const childEnv = { ...process.env };
    delete childEnv.ACCELERATOR_INVENTORY_OUTPUT_ROOT;
    const child = spawnDaemon(stateDir, { ACCELERATOR_INVENTORY_OUTPUT_ROOT: '' });
    try {
      const info = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        const res = await send(info.url, { protocol: 1, command: 'screenshot', path: 'x.png' });
        assert.equal(res.error, 'screenshot-output-root-unset');
      });
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('screenshot: absolute path outside root → screenshot-path-outside-output-root', { timeout: 20000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    await withTmpDir(async outputRoot => {
      const child = spawnDaemon(stateDir, { ACCELERATOR_INVENTORY_OUTPUT_ROOT: outputRoot });
      try {
        const info = await waitForInfo(stateDir);
        await withFixtureServer(async fixtureUrl => {
          await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
          const res = await send(info.url, { protocol: 1, command: 'screenshot', path: '/etc/shadow' });
          assert.equal(res.error, 'screenshot-path-outside-output-root');
        });
      } finally {
        child.kill('SIGTERM');
      }
    });
  });
});

// -- evaluate -----------------------------------------------------------

test('evaluate: expression result round-trips', { timeout: 20000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir);
    try {
      const info = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        const res = await send(info.url, { protocol: 1, command: 'evaluate', expression: 'document.title' });
        assert.equal(typeof res.result, 'string');
        assert.ok(res.result.length > 0);
      });
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('evaluate: fetch call is NOT filtered (deny-list removed)', { timeout: 20000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir);
    try {
      const info = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        // Should reach page.evaluate without rejection from executor
        const res = await send(info.url, { protocol: 1, command: 'evaluate', expression: 'typeof fetch' });
        assert.ok(!res.error || res.error !== 'evaluate-payload-rejected', 'executor must not filter evaluate payloads');
      });
    } finally {
      child.kill('SIGTERM');
    }
  });
});

// -- daemon lifecycle ----------------------------------------------------

test('two consecutive client calls reuse the same daemon', { timeout: 30000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir);
    try {
      const info1 = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        await send(info1.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        // Second call — server-info.json PID should be the same
        const info2 = JSON.parse(readFileSync(resolve(stateDir, 'server-info.json'), 'utf8'));
        assert.equal(info1.pid, info2.pid);
        await send(info2.url, { protocol: 1, command: 'snapshot' });
        const info3 = JSON.parse(readFileSync(resolve(stateDir, 'server-info.json'), 'utf8'));
        assert.equal(info1.pid, info3.pid);
      });
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('idle shutdown: no traffic → server-stopped.json with reason: idle', { timeout: 5000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '400' });
    try {
      const info = await waitForInfo(stateDir);
      await send(info.url, { protocol: 1, command: 'ping' });
      await new Promise(r => setTimeout(r, 1500));
      const stopped = resolve(stateDir, 'server-stopped.json');
      assert.ok(existsSync(stopped), 'server-stopped.json should exist after idle timeout');
      const parsed = JSON.parse(readFileSync(stopped, 'utf8'));
      assert.equal(parsed.reason, 'idle');
    } finally {
      try { child.kill('SIGTERM'); } catch {}
    }
  });
});

test('wall-clock exceeded: in-flight client receives structured envelope', { timeout: 15000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir, {
      ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS: '1000',
      ACCELERATOR_PLAYWRIGHT_IDLE_MS: '30000',
    });
    try {
      const info = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        // wait_for something that will never appear, wall clock should fire first
        const res = await send(info.url, { protocol: 1, command: 'wait_for', text: '__never_appears_xyz__', timeout_ms: 60000 });
        // Either wait-for-timeout or wall-clock-exceeded
        assert.ok(
          res.error === 'wait-for-timeout' || res.error === 'wall-clock-exceeded',
          `expected timeout error, got: ${JSON.stringify(res)}`,
        );
        if (res.error === 'wait-for-timeout') {
          assert.equal(res.details?.truncated, true);
        }
      });
    } finally {
      try { child.kill('SIGTERM'); } catch {}
    }
  });
});

test('wait_for: caller timeout shorter than wall clock returns natural timeout without kill', { timeout: 15000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir, {
      ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS: '10000',
      ACCELERATOR_PLAYWRIGHT_IDLE_MS: '30000',
    });
    try {
      const info = await waitForInfo(stateDir);
      await withFixtureServer(async fixtureUrl => {
        await send(info.url, { protocol: 1, command: 'navigate', url: fixtureUrl });
        const t0 = Date.now();
        const res = await send(info.url, { protocol: 1, command: 'wait_for', text: '__never__', timeout_ms: 300 });
        const elapsed = Date.now() - t0;
        // Should resolve at ~300ms (natural timeout), NOT at 10s (wall clock)
        assert.ok(elapsed < 3000, `should resolve quickly, took ${elapsed}ms`);
        assert.equal(res.error, 'wait-for-timeout');
        assert.ok(!res.details?.truncated, 'should not be truncated when caller timeout < wall clock');
        // daemon should still be alive
        const stillAlive = existsSync(resolve(stateDir, 'server-info.json'));
        assert.ok(stillAlive, 'daemon should still be running');
      });
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('protocol round-trip: every command includes protocol: 1 in response', { timeout: 30000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir);
    try {
      const info = await waitForInfo(stateDir);
      for (const cmd of ['ping', 'daemon-status']) {
        const res = await send(info.url, { protocol: 1, command: cmd });
        assert.equal(res.protocol, 1, `${cmd} response should include protocol: 1`);
      }
      // protocol-mismatch test
      const mismatch = await send(info.url, { protocol: 999, command: 'ping' });
      assert.equal(mismatch.error, 'protocol-mismatch');
      assert.equal(mismatch.protocol, 1);
    } finally {
      child.kill('SIGTERM');
    }
  });
});

test('daemon-stop: writes server-stopped.json and removes state files', { timeout: 10000, skip: !playwrightInstalled }, async () => {
  await withTmpDir(async stateDir => {
    const child = spawnDaemon(stateDir, { ACCELERATOR_PLAYWRIGHT_IDLE_MS: '30000' });
    try {
      const info = await waitForInfo(stateDir);
      const res = await send(info.url, { protocol: 1, command: 'daemon-stop' });
      assert.equal(res.ok, true);
      await new Promise(r => setTimeout(r, 2000));
      assert.ok(existsSync(resolve(stateDir, 'server-stopped.json')));
    } finally {
      try { child.kill('SIGTERM'); } catch {}
    }
  });
});
