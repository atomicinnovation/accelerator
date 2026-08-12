import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { realpathSync } from 'node:fs';
import { createServer } from 'node:http';
import { createServer as createNetServer } from 'node:net';
import { writeServerInfo } from './state.js';
import { callRemote } from './client.js';

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'client-test-')));
  return Promise.resolve(fn(dir)).finally(() => rmSync(dir, { recursive: true, force: true }));
}

async function withMockHttpServer(handler, fn) {
  const server = createServer(handler);
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const { port } = server.address();
  const url = `http://127.0.0.1:${port}/`;
  try {
    await fn(url, port);
  } finally {
    await new Promise(r => server.close(r));
  }
}

// Suppress callRemote's stdout output during tests
function suppressStdout(fn) {
  const original = process.stdout.write.bind(process.stdout);
  process.stdout.write = () => true;
  return Promise.resolve(fn()).finally(() => { process.stdout.write = original; });
}

test('server-info.json present and reachable → client returns response', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    await withMockHttpServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ protocol: 1, ok: true, node: 'v20.0.0', playwright: '1.49.1', chromium: '/path/to/chromium' }));
    }, async (url, port) => {
      writeServerInfo(dir, { protocol: 1, pid: process.pid, start_time: 0, host: '127.0.0.1', port, url, ready_at: new Date().toISOString() });
      const res = await suppressStdout(() => callRemote(dir, 'ping'));
      assert.equal(res.ok, true);
      assert.equal(res.node, 'v20.0.0');
    });
  });
});

test('server-info.json missing → client returns no-daemon error', { timeout: 2000 }, async () => {
  await withTmpDir(async dir => {
    const res = await suppressStdout(() => callRemote(dir, 'ping'));
    assert.equal(res.error, 'no-daemon');
    assert.equal(res.category, 'usage');
  });
});

test('server-info.json present but TCP connect fails → returns connection-failed', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    // Get a free port then close it immediately so the connect will ECONNREFUSED
    const freePort = await new Promise(resolve => {
      const s = createNetServer();
      s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); });
    });
    writeServerInfo(dir, { protocol: 1, pid: 99999, start_time: 0, host: '127.0.0.1', port: freePort, url: `http://127.0.0.1:${freePort}/`, ready_at: new Date().toISOString() });
    // Wait a moment for the port to be fully released
    await new Promise(r => setTimeout(r, 100));
    const res = await suppressStdout(() => callRemote(dir, 'ping'));
    assert.equal(res.error, 'connection-failed');
    assert.equal(res.category, 'protocol');
  });
});

test('a non-loopback daemon URL is refused before the token is sent', async () => {
  // Anything able to write the state dir could otherwise redirect the
  // token-bearing request to a host of its choosing.
  await withTmpDir(async dir => {
    for (const url of [
      'http://evil.example.com:8080/',
      'http://127.0.0.1.evil.example.com/',
      'http://10.0.0.1:8080/',
      'http://[2001:db8::1]:8080/',
    ]) {
      writeServerInfo(dir, { protocol: 1, pid: 1, url, token: 'tok' });
      const result = await callRemote(dir, 'ping', {});
      assert.equal(result.error, 'non-loopback-daemon', url);
    }
  });
});

test('both loopback literals are accepted', async () => {
  await withTmpDir(async dir => {
    await withMockHttpServer(
      (req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true }));
      },
      async url => {
        writeServerInfo(dir, { protocol: 1, pid: 1, url, token: 'tok' });
        const result = await callRemote(dir, 'ping', {});
        assert.equal(result.ok, true);
      }
    );
  });
});

test('the token travels as a header, never as a query parameter', async () => {
  await withTmpDir(async dir => {
    let seenHeader;
    let seenUrl;
    await withMockHttpServer(
      (req, res) => {
        seenHeader = req.headers['x-accelerator-token'];
        seenUrl = req.url;
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true }));
      },
      async url => {
        writeServerInfo(dir, { protocol: 1, pid: 1, url, token: 'sekrit' });
        await callRemote(dir, 'ping', {});
        assert.equal(seenHeader, 'sekrit');
        assert.ok(!seenUrl.includes('sekrit'), seenUrl);
      }
    );
  });
});

test('a payload naming its own command or protocol is refused', async () => {
  // Refused rather than silently overwritten, so a caller cannot believe it
  // ran something it did not.
  await withTmpDir(async dir => {
    writeServerInfo(dir, { protocol: 1, pid: 1, url: 'http://127.0.0.1:1/', token: 'tok' });
    for (const payload of [{ command: 'evaluate' }, { protocol: 99 }]) {
      const result = await callRemote(dir, 'ping', payload);
      assert.equal(result.error, 'payload-rejected', JSON.stringify(payload));
    }
  });
});

test('the validated command wins over anything the payload carries', async () => {
  await withTmpDir(async dir => {
    let body;
    await withMockHttpServer(
      (req, res) => {
        const chunks = [];
        req.on('data', c => chunks.push(c));
        req.on('end', () => {
          body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true }));
        });
      },
      async url => {
        writeServerInfo(dir, { protocol: 1, pid: 1, url, token: 'tok' });
        await callRemote(dir, 'ping', { url: 'https://example.com' });
        assert.equal(body.command, 'ping');
        assert.equal(body.protocol, 1);
        assert.equal(body.url, 'https://example.com');
      }
    );
  });
});
