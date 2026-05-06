import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mergeMaskSelectors, DEFAULT_MASK_SELECTORS } from './mask.js';

test('default selectors present with no extras', () => {
  const result = mergeMaskSelectors();
  assert.ok(result.includes('[type=password]'));
  assert.ok(result.includes('[autocomplete*=token]'));
  assert.ok(result.includes('[data-secret]'));
});

test('caller-supplied selectors are merged in', () => {
  const result = mergeMaskSelectors(['[data-pii]']);
  assert.ok(result.includes('[data-pii]'));
  assert.ok(result.includes('[type=password]'));
});

test('duplicates are deduplicated', () => {
  const result = mergeMaskSelectors(['[type=password]', '[type=password]', '[data-pii]']);
  assert.equal(result.filter(s => s === '[type=password]').length, 1);
});

test('result is a plain array', () => {
  assert.ok(Array.isArray(mergeMaskSelectors()));
  assert.ok(Array.isArray(mergeMaskSelectors(['[x]'])));
});

test('caller cannot remove defaults', () => {
  const result = mergeMaskSelectors([]);
  for (const d of DEFAULT_MASK_SELECTORS) {
    assert.ok(result.includes(d), `default selector ${d} should be present`);
  }
});

test('DEFAULT_MASK_SELECTORS is frozen', () => {
  assert.ok(Object.isFrozen(DEFAULT_MASK_SELECTORS));
});
