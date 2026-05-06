import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { realpathSync } from 'node:fs';
import {
  atomicWrite, writeServerInfo, writeServerStopped,
  readServerInfo, readServerPid, removeServerFiles,
  SERVER_INFO_FILE, SERVER_PID_FILE, SERVER_STOPPED_FILE,
} from './state.js';

function withTmp(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'state-test-')));
  try { fn(dir); }
  finally { rmSync(dir, { recursive: true, force: true }); }
}

test('atomicWrite writes via tmp+rename, no partial file visible', () => {
  withTmp(dir => {
    const target = resolve(dir, 'out.json');
    atomicWrite(target, '{"ok":true}');
    assert.ok(existsSync(target));
    assert.equal(readFileSync(target, 'utf8'), '{"ok":true}');
    const files = readdirSync(dir);
    assert.ok(!files.some(f => f.startsWith('.tmp-')), 'no tmp files left behind');
  });
});

test('writeServerInfo writes server-info.json and server.pid', () => {
  withTmp(dir => {
    const info = { protocol: 1, pid: 99999, start_time: 1700000000, host: '127.0.0.1', port: 54321, url: 'http://127.0.0.1:54321/', ready_at: new Date().toISOString() };
    writeServerInfo(dir, info);
    const parsed = JSON.parse(readFileSync(resolve(dir, SERVER_INFO_FILE), 'utf8'));
    assert.deepEqual(parsed, info);
    const pid = readFileSync(resolve(dir, SERVER_PID_FILE), 'utf8').trim();
    assert.equal(pid, '99999');
  });
});

test('readServerInfo returns null when file missing', () => {
  withTmp(dir => {
    assert.equal(readServerInfo(dir), null);
  });
});

test('readServerInfo returns parsed JSON when file present', () => {
  withTmp(dir => {
    const info = { protocol: 1, pid: 1, url: 'http://127.0.0.1:9/' };
    writeServerInfo(dir, info);
    assert.deepEqual(readServerInfo(dir), info);
  });
});

test('readServerPid returns null when file missing', () => {
  withTmp(dir => {
    assert.equal(readServerPid(dir), null);
  });
});

test('readServerPid returns integer pid', () => {
  withTmp(dir => {
    const info = { protocol: 1, pid: 12345, url: 'http://127.0.0.1:1/' };
    writeServerInfo(dir, info);
    assert.equal(readServerPid(dir), 12345);
  });
});

test('writeServerStopped includes reason, stopped_at, and extra fields', () => {
  withTmp(dir => {
    writeServerStopped(dir, 'idle', { extra: 'value' });
    const parsed = JSON.parse(readFileSync(resolve(dir, SERVER_STOPPED_FILE), 'utf8'));
    assert.equal(parsed.reason, 'idle');
    assert.ok(typeof parsed.stopped_at === 'string');
    assert.equal(parsed.extra, 'value');
    assert.equal(parsed.protocol, 1);
  });
});

test('removeServerFiles removes info.json and pid file', () => {
  withTmp(dir => {
    writeServerInfo(dir, { protocol: 1, pid: 1, url: 'http://127.0.0.1:1/' });
    removeServerFiles(dir);
    assert.ok(!existsSync(resolve(dir, SERVER_INFO_FILE)));
    assert.ok(!existsSync(resolve(dir, SERVER_PID_FILE)));
  });
});
