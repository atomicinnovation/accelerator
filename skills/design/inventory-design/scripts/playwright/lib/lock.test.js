import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { realpathSync } from 'node:fs';
import { acquireLock, isLocked } from './lock.js';

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'lock-test-')));
  return Promise.resolve(fn(dir)).finally(() => rmSync(dir, { recursive: true, force: true }));
}

test('acquires immediately when lock is free', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const lockFile = resolve(dir, 'launcher.lock');
    const release = await acquireLock(lockFile);
    assert.ok(typeof release === 'function');
    release();
  });
});

test('lock is held during callback, released after', { timeout: 5000 }, async () => {
  await withTmpDir(async dir => {
    const lockFile = resolve(dir, 'launcher.lock');
    const release = await acquireLock(lockFile);
    assert.ok(isLocked(lockFile), 'should be locked during hold');
    release();
    assert.ok(!isLocked(lockFile), 'should be unlocked after release');
  });
});

test('second acquireLock waits until first is released', { timeout: 10000 }, async () => {
  await withTmpDir(async dir => {
    const lockFile = resolve(dir, 'launcher.lock');
    const release1 = await acquireLock(lockFile);

    let release2Resolved = false;
    const p2 = acquireLock(lockFile).then(rel => { release2Resolved = true; return rel; });

    // Give p2 a chance to try — it should be waiting
    await new Promise(res => setTimeout(res, 200));
    assert.ok(!release2Resolved, 'second lock should still be waiting');

    release1();
    const release2 = await p2;
    assert.ok(release2Resolved, 'second lock should now be acquired');
    release2();
  });
});

test('ACCELERATOR_LOCK_FORCE_MKDIR=1: same acquisition semantics', { timeout: 5000 }, async () => {
  // ACCELERATOR_LOCK_FORCE_MKDIR is a no-op in the JS layer (already uses mkdir)
  // but must not break acquisition
  const original = process.env.ACCELERATOR_LOCK_FORCE_MKDIR;
  process.env.ACCELERATOR_LOCK_FORCE_MKDIR = '1';
  try {
    await withTmpDir(async dir => {
      const lockFile = resolve(dir, 'launcher.lock');
      const release = await acquireLock(lockFile);
      assert.ok(isLocked(lockFile));
      release();
      assert.ok(!isLocked(lockFile));
    });
  } finally {
    if (original === undefined) delete process.env.ACCELERATOR_LOCK_FORCE_MKDIR;
    else process.env.ACCELERATOR_LOCK_FORCE_MKDIR = original;
  }
});

test('contention timeout when lock dir is stale', { timeout: 5000 }, async () => {
  const original = process.env.ACCELERATOR_LOCK_TIMEOUT_MS;
  process.env.ACCELERATOR_LOCK_TIMEOUT_MS = '300';
  try {
    await withTmpDir(async dir => {
      const lockFile = resolve(dir, 'launcher.lock');
      // Pre-create the lock dir (simulates stale lock from killed prior holder)
      mkdirSync(lockFile + '.d', { recursive: false });
      await assert.rejects(
        () => acquireLock(lockFile),
        { code: 'ELOCKTIMEOUT' },
      );
    });
  } finally {
    if (original === undefined) delete process.env.ACCELERATOR_LOCK_TIMEOUT_MS;
    else process.env.ACCELERATOR_LOCK_TIMEOUT_MS = original;
  }
});
