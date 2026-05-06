import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shouldAttachHeader } from './auth-header.js';

const origin = 'https://app.example.com';

test('same-origin request → header attached', () => {
  assert.ok(shouldAttachHeader('https://app.example.com/api/data', origin));
  assert.ok(shouldAttachHeader('https://app.example.com/', origin));
});

test('cross-origin different host → header not attached', () => {
  assert.ok(!shouldAttachHeader('https://other.example.com/api', origin));
  assert.ok(!shouldAttachHeader('https://evil.com/', origin));
});

test('cross-origin different scheme → header not attached', () => {
  assert.ok(!shouldAttachHeader('http://app.example.com/', origin));
});

test('cross-origin different port → header not attached', () => {
  assert.ok(!shouldAttachHeader('https://app.example.com:8080/', origin));
});

test('default-port equivalence: https://example.com and https://example.com:443 are same origin', () => {
  assert.ok(shouldAttachHeader('https://example.com:443/path', 'https://example.com'));
  assert.ok(shouldAttachHeader('https://example.com/path', 'https://example.com:443'));
});

test('hostname comparison is case-insensitive via URL.origin', () => {
  // URL.origin normalises hostname to lowercase
  assert.ok(shouldAttachHeader('https://APP.EXAMPLE.COM/', 'https://app.example.com'));
});

test('subdomain confusion: app.example.com.evil.com does NOT match app.example.com', () => {
  assert.ok(!shouldAttachHeader('https://app.example.com.evil.com/', 'https://app.example.com'));
});

test('IDN homograph: xn--example-X.com does NOT match example.com', () => {
  assert.ok(!shouldAttachHeader('https://xn--example-X.com/', 'https://example.com'));
});

test('cross-origin redirect target → header not attached', () => {
  // Even though the frame origin matches, the request URL is a CDN
  assert.ok(!shouldAttachHeader('https://cdn.third-party.com/asset.js', 'https://app.example.com'));
});

test('opaque origin (null) → header not attached', () => {
  // data: URIs produce opaque origin "null"
  assert.ok(!shouldAttachHeader('data:text/html,<h1>hi</h1>', 'https://app.example.com'));
});

test('missing ACCELERATOR_BROWSER_AUTH_HEADER env → no-op handler', async () => {
  const { makeAuthHeaderHandler } = await import('./auth-header.js');
  const mockPage = { route: () => { throw new Error('route should not be called'); } };
  const install = makeAuthHeaderHandler(mockPage, { env: {} });
  // Should not throw
  await install();
});
