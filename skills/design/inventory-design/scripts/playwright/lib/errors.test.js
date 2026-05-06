import { test } from 'node:test';
import assert from 'node:assert/strict';
import { makeError, protocolMismatch, PROTOCOL } from './errors.js';

test('every category produces a well-formed envelope', () => {
  for (const category of ['usage', 'protocol', 'browser', 'bootstrap', 'filesystem']) {
    const env = makeError({ error: 'some-error', message: 'Something happened', category, retryable: false });
    assert.equal(env.protocol, PROTOCOL);
    assert.equal(typeof env.error, 'string');
    assert.match(env.error, /^[a-z][a-z0-9-]*$/);
    assert.ok(env.message.length > 0);
    assert.equal(env.category, category);
    assert.equal(typeof env.retryable, 'boolean');
    assert.ok(!('details' in env), 'no details field when not provided');
  }
});

test('optional details round-trips arbitrary JSON-serialisable values', () => {
  const details = { foo: 'bar', n: 42, arr: [1, 2], nested: { x: true } };
  const env = makeError({ error: 'test-error', message: 'Test', category: 'usage', retryable: false, details });
  assert.deepEqual(env.details, details);
  assert.doesNotThrow(() => JSON.stringify(env));
});

test('protocol-mismatch populates message with "protocol": 1', () => {
  const env = protocolMismatch(999);
  assert.equal(env.protocol, PROTOCOL);
  assert.equal(env.error, 'protocol-mismatch');
  assert.ok(env.message.includes(`"protocol": ${PROTOCOL}`), `message should mention "protocol": ${PROTOCOL}`);
  assert.equal(env.details.expected, PROTOCOL);
  assert.equal(env.details.got, 999);
  assert.equal(env.category, 'protocol');
  assert.equal(env.retryable, false);
});

test('unknown category throws', () => {
  assert.throws(
    () => makeError({ error: 'x', message: 'y', category: 'invalid', retryable: false }),
    /Unknown error category/,
  );
});

test('retryable defaults to false', () => {
  const env = makeError({ error: 'x', message: 'y', category: 'usage' });
  assert.equal(env.retryable, false);
});
