import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
  canonicalise,
  classifyHost,
  classifyNavigationRequest,
  classifyUrl,
} from './access-policy.js';

// The corpus lives in the Rust domain crate; the Rust classifier is held to the
// same cases, so a failure on either side means the two implementations drifted.
const CORPUS_PATH = resolve(
  import.meta.dirname,
  '../../../../../..',
  'cli/design/tests/fixtures/host-classification-vectors.json'
);
const CORPUS = JSON.parse(readFileSync(CORPUS_PATH, 'utf8'));

function allowancesOf(policyCase) {
  return {
    allowInternal: policyCase.allow_internal === true,
    allowInsecureScheme: policyCase.allow_insecure_scheme === true,
  };
}

test('the corpus is found and carries reach and policy cases', () => {
  assert.ok(CORPUS.reach.length > 0, 'no reach cases loaded');
  assert.ok(CORPUS.policy.length > 0, 'no policy cases loaded');
});

test('every reach and error case classifies as the corpus says', () => {
  for (const item of CORPUS.reach) {
    const outcome = canonicalise(item.authority);
    if (item.reach !== undefined) {
      assert.ok(outcome.ok, `${item.authority} failed to canonicalise`);
      assert.equal(classifyHost(outcome.host), item.reach, item.authority);
    } else {
      assert.equal(outcome.ok, false, `${item.authority} should not parse`);
    }
  }
});

test('every policy case judges as the corpus says', () => {
  for (const item of CORPUS.policy) {
    const outcome = classifyUrl(item.url, allowancesOf(item));
    if (item.verdict === 'accepted') {
      assert.deepEqual(outcome, { ok: true }, item.url);
    } else {
      assert.equal(outcome.ok, false, item.url);
      assert.equal(outcome.classification, item.classification, item.url);
    }
  }
});

test('a missing allowances argument fails closed to deny', () => {
  assert.deepEqual(classifyUrl('http://10.0.0.1'), {
    ok: false,
    classification: 'private',
  });
  assert.deepEqual(classifyUrl('http://10.0.0.1', undefined), {
    ok: false,
    classification: 'private',
  });
});

test('an unjudgeable input is refused as malformed, never accepted', () => {
  for (const url of [
    'http://user@example.com',
    'ftp://example.com',
    'http://0x7f000001',
    'file:///etc/passwd',
  ]) {
    assert.deepEqual(
      classifyUrl(url, { allowInternal: true, allowInsecureScheme: true }),
      { ok: false, classification: 'malformed' },
      url
    );
  }
});

test('every wire classification token is reachable through classifyUrl', () => {
  const tokens = new Set();
  for (const item of CORPUS.policy) {
    const outcome = classifyUrl(item.url, allowancesOf(item));
    if (!outcome.ok) tokens.add(outcome.classification);
  }
  for (const token of [
    'private',
    'link-local',
    'reserved',
    'unspecified',
    'insecure-scheme',
    'malformed',
  ]) {
    assert.ok(tokens.has(token), `no policy case yields ${token}`);
  }
});

function fakeRequest(url, isNavigation = true) {
  return { url: () => url, isNavigationRequest: () => isNavigation };
}

test('classifyNavigationRequest aborts a refused navigation with its class', () => {
  for (const item of CORPUS.policy) {
    const decision = classifyNavigationRequest(
      fakeRequest(item.url),
      allowancesOf(item)
    );
    if (item.verdict === 'accepted') {
      assert.deepEqual(decision, { continue: true }, item.url);
    } else {
      assert.equal(decision.abort, true, item.url);
      assert.equal(decision.classification, item.classification, item.url);
      assert.equal(decision.url, item.url, item.url);
    }
  }
});

test('classifyNavigationRequest continues every non-navigation request', () => {
  // A subresource to an internal host is not a navigation, so it is not the
  // route handler's concern — only navigations and followed links are classified.
  for (const url of ['http://169.254.169.254/', 'https://example.com/app.js']) {
    assert.deepEqual(
      classifyNavigationRequest(fakeRequest(url, false), {
        allowInternal: false,
        allowInsecureScheme: false,
      }),
      { continue: true },
      url
    );
  }
});
