import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, symlinkSync, mkdirSync, rmdirSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir, platform } from 'node:os';
import { realpathSync } from 'node:fs';
import { guardScreenshotPath } from './path-guard.js';

// Helpers

function withTmpRoot(fn) {
  const root = realpathSync(mkdtempSync(resolve(tmpdir(), 'pg-test-')));
  try {
    fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function env(root) {
  return { ACCELERATOR_INVENTORY_OUTPUT_ROOT: root };
}

// Tests

test('unset ACCELERATOR_INVENTORY_OUTPUT_ROOT → screenshot-output-root-unset', () => {
  const result = guardScreenshotPath('foo.png', { env: {} });
  assert.equal(result.ok, false);
  assert.equal(result.error.error, 'screenshot-output-root-unset');
  assert.equal(result.error.category, 'usage');
});

test('empty ACCELERATOR_INVENTORY_OUTPUT_ROOT → screenshot-output-root-unset', () => {
  const result = guardScreenshotPath('foo.png', { env: { ACCELERATOR_INVENTORY_OUTPUT_ROOT: '   ' } });
  assert.equal(result.ok, false);
  assert.equal(result.error.error, 'screenshot-output-root-unset');
});

test('valid relative path → resolves to root/path', () => {
  withTmpRoot(root => {
    const result = guardScreenshotPath('home.png', { env: env(root) });
    assert.equal(result.ok, true);
    assert.equal(result.path, resolve(root, 'home.png'));
  });
});

test('valid absolute path inside root → returns path', () => {
  withTmpRoot(root => {
    const absPath = resolve(root, 'screens/page.png');
    mkdirSync(resolve(root, 'screens'), { recursive: true });
    const result = guardScreenshotPath(absPath, { env: env(root) });
    assert.equal(result.ok, true);
    assert.equal(result.path, absPath);
  });
});

test('absolute path outside root → screenshot-path-outside-output-root', () => {
  withTmpRoot(root => {
    const result = guardScreenshotPath('/etc/shadow', { env: env(root) });
    assert.equal(result.ok, false);
    assert.equal(result.error.error, 'screenshot-path-outside-output-root');
    assert.equal(result.error.category, 'usage');
  });
});

test('absolute path with .. that resolves outside root → rejected', () => {
  withTmpRoot(root => {
    const outsidePath = resolve(root, '..', 'evil.png');
    const result = guardScreenshotPath(outsidePath, { env: env(root) });
    assert.equal(result.ok, false);
    assert.equal(result.error.error, 'screenshot-path-outside-output-root');
  });
});

test('relative path with .. that escapes root → rejected', () => {
  withTmpRoot(root => {
    const result = guardScreenshotPath('../../etc/x.png', { env: env(root) });
    assert.equal(result.ok, false);
    assert.equal(result.error.error, 'screenshot-path-outside-output-root');
  });
});

test('symlink under root pointing outside → rejected', () => {
  withTmpRoot(root => {
    const target = realpathSync(mkdtempSync(resolve(tmpdir(), 'pg-ext-')));
    try {
      const linkPath = resolve(root, 'escape');
      symlinkSync(target, linkPath);
      const result = guardScreenshotPath('escape/x.png', { env: env(root) });
      assert.equal(result.ok, false);
      assert.equal(result.error.error, 'screenshot-path-outside-output-root');
    } finally {
      rmSync(target, { recursive: true, force: true });
    }
  });
});

test('overwriting an existing file inside root is permitted', () => {
  withTmpRoot(root => {
    // First write (file doesn't exist)
    const r1 = guardScreenshotPath('same.png', { env: env(root) });
    assert.equal(r1.ok, true);
    // Second "write" (same path)
    const r2 = guardScreenshotPath('same.png', { env: env(root) });
    assert.equal(r2.ok, true);
    assert.equal(r1.path, r2.path);
  });
});
