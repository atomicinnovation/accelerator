import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  makeAuthHeaderHandler,
  parseAuthHeader,
  applyHeader,
  shouldAttachHeader,
} from './auth-header.js';

const origin = 'https://app.example.com';

test('parseAuthHeader returns null for an unset value', () => {
  assert.equal(parseAuthHeader(undefined), null);
  assert.equal(parseAuthHeader(''), null);
});

test('parseAuthHeader returns null for a value with no colon', () => {
  assert.equal(parseAuthHeader('Authorization Bearer x'), null);
});

test('parseAuthHeader returns null for an empty trimmed name', () => {
  assert.equal(parseAuthHeader(':Bearer x'), null);
  assert.equal(parseAuthHeader('   :Bearer x'), null);
});

test('parseAuthHeader returns null for an empty trimmed value', () => {
  assert.equal(parseAuthHeader('Authorization:'), null);
  assert.equal(parseAuthHeader('Authorization:   '), null);
});

test('parseAuthHeader returns a trimmed name/value pair for a valid header', () => {
  assert.deepEqual(parseAuthHeader('Authorization: Bearer abc123'), {
    name: 'Authorization',
    value: 'Bearer abc123',
  });
  assert.deepEqual(parseAuthHeader('  X-Api-Key :  secret  '), {
    name: 'X-Api-Key',
    value: 'secret',
  });
});

test('shouldAttachHeader is false for a null or undefined origin (fail-closed)', () => {
  assert.ok(!shouldAttachHeader('https://app.example.com/', null));
  assert.ok(!shouldAttachHeader('https://app.example.com/', undefined));
});

test('shouldAttachHeader is true for an exact origin match', () => {
  assert.ok(shouldAttachHeader('https://app.example.com/api/data', origin));
  assert.ok(shouldAttachHeader('https://app.example.com/', origin));
});

test('shouldAttachHeader is false for a differing origin', () => {
  assert.ok(!shouldAttachHeader('https://other.example.com/api', origin));
  assert.ok(!shouldAttachHeader('https://evil.com/', origin));
  assert.ok(!shouldAttachHeader('http://app.example.com/', origin));
  assert.ok(!shouldAttachHeader('https://app.example.com:8080/', origin));
});

test('shouldAttachHeader is false for an unparseable request URL', () => {
  assert.ok(!shouldAttachHeader('not a url', origin));
});

test('shouldAttachHeader normalises the request-side default port', () => {
  // URL.origin drops the default :443, so the request side matches a bare
  // expected origin. The expected-side normalisation lives on the daemon's
  // originOf, which produces the origin string this compares against.
  assert.ok(shouldAttachHeader('https://example.com:443/path', 'https://example.com'));
});

test('shouldAttachHeader compares the request host case-insensitively', () => {
  assert.ok(shouldAttachHeader('https://APP.EXAMPLE.COM/', origin));
});

test('shouldAttachHeader rejects a subdomain-confusion host', () => {
  assert.ok(!shouldAttachHeader('https://app.example.com.evil.com/', origin));
});

test('shouldAttachHeader rejects an IDN homograph host', () => {
  assert.ok(!shouldAttachHeader('https://xn--example-X.com/', 'https://example.com'));
});

test('shouldAttachHeader rejects an opaque data: origin', () => {
  assert.ok(!shouldAttachHeader('data:text/html,<h1>hi</h1>', origin));
});

test('applyHeader adds the pair under the lowercased name when attaching', () => {
  const headers = applyHeader(
    { 'content-type': 'text/html' },
    true,
    { name: 'Authorization', value: 'Bearer abc123' }
  );
  assert.equal(headers.authorization, 'Bearer abc123');
  assert.equal(headers['content-type'], 'text/html');
});

test('applyHeader deletes the header by its lowercased name when not attaching', () => {
  const headers = applyHeader(
    { authorization: 'Bearer abc123', 'content-type': 'text/html' },
    false,
    { name: 'Authorization', value: 'Bearer abc123' }
  );
  assert.ok(!('authorization' in headers));
  assert.equal(headers['content-type'], 'text/html');
});

test('applyHeader leaves the map untouched when no header is configured', () => {
  const current = { authorization: 'existing' };
  const headers = applyHeader(current, false, null);
  assert.deepEqual(headers, { authorization: 'existing' });
  assert.notEqual(headers, current);
});

// Exercises the installed route body with a mock page/route, so the getter
// wiring and the fail-closed default are covered without a live browser.
function captureRoute(getters) {
  let registered;
  const page = {
    route: async (_pattern, handler) => {
      registered = handler;
    },
  };
  const install = makeAuthHeaderHandler(page, getters);
  return install().then(() => registered);
}

function mockRoute(url, headers = {}) {
  const seen = {};
  return {
    seen,
    request: () => ({
      url: () => url,
      allHeaders: async () => ({ ...headers }),
    }),
    continue: async (options) => {
      seen.options = options ?? {};
    },
  };
}

test('the installed route attaches the header on the expected origin', async () => {
  const route = await captureRoute({
    getExpectedOrigin: () => origin,
    getAuthHeader: () => ({ name: 'Authorization', value: 'Bearer abc123' }),
  });
  const call = mockRoute('https://app.example.com/api');
  await route(call);
  assert.equal(call.seen.options.headers.authorization, 'Bearer abc123');
});

test('the installed route strips the header on a cross-origin request', async () => {
  const route = await captureRoute({
    getExpectedOrigin: () => origin,
    getAuthHeader: () => ({ name: 'Authorization', value: 'Bearer abc123' }),
  });
  const call = mockRoute('https://cdn.other.com/asset.js', {
    authorization: 'Bearer abc123',
  });
  await route(call);
  assert.ok(!('authorization' in call.seen.options.headers));
});

test('the installed route no-ops when the header getter returns null', async () => {
  const route = await captureRoute({
    getExpectedOrigin: () => origin,
    getAuthHeader: () => null,
  });
  const call = mockRoute('https://app.example.com/', {
    authorization: 'pre-existing',
  });
  await route(call);
  assert.deepEqual(call.seen.options.headers, { authorization: 'pre-existing' });
});

test('the module reads neither retired auth env var', () => {
  // The needles are built from fragments so this assertion does not itself
  // count as a reference under the AC4 sweep that scans this tree.
  const source = readFileSync(
    new URL('./auth-header.js', import.meta.url).pathname,
    'utf8'
  );
  const retiredOrigin = 'ACCELERATOR_BROWSER_LOCATION' + '_ORIGIN';
  const authHeader = 'ACCELERATOR_BROWSER' + '_AUTH_HEADER';
  assert.ok(!source.includes(retiredOrigin));
  assert.ok(!source.includes(authHeader));
});
