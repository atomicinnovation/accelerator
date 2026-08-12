import { test } from 'node:test';
import assert from 'node:assert/strict';
import { writeFileSync, openSync, closeSync, readFileSync } from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';

import {
  parseIdentity,
  readIdentity,
  MalformedIdentity,
  IDENTITY_FD_VAR,
} from './identity-handoff.js';

const TOKEN = '0123456789abcdef0123456789abcdef';

test('the wire format matches the launcher byte for byte', () => {
  assert.deepEqual(parseIdentity(`4242\n1700145620\np\n${TOKEN}\n`), {
    pid: 4242,
    start_time: 1700145620,
    start_time_source: 'probe',
    token: TOKEN,
  });
});

test('each start-time source tag is recognised', () => {
  assert.equal(parseIdentity(`1\n17\np\n${TOKEN}\n`).start_time_source, 'probe');
  assert.equal(parseIdentity(`1\n17\nw\n${TOKEN}\n`).start_time_source, 'wallclock');
  assert.equal(
    parseIdentity(`1\n0\nu\n${TOKEN}\n`).start_time_source,
    'writer-unavailable'
  );
});

test('a writer-unavailable record carries no start time rather than a placeholder', () => {
  // Reading the padding zero as a real value would hold the daemon to the
  // launcher's one-second tolerance against a number nobody measured.
  assert.equal(parseIdentity(`1\n999\nu\n${TOKEN}\n`).start_time, null);
});

test('an empty read is a truncation, not a set of defaults', () => {
  // The launcher was killed after spawning but before writing.
  assert.throws(() => parseIdentity(''), MalformedIdentity);
  assert.throws(() => parseIdentity(''), /carried 0 of 4 fields/);
});

test('a short read names how many fields arrived', () => {
  assert.throws(() => parseIdentity('4242\n1700145620\n'), /carried 2 of 4 fields/);
});

test('an empty token is distinguishable from a missing field', () => {
  // Trimming trailing newlines greedily would collapse these two.
  assert.throws(() => parseIdentity('1\n1\np\n\n'), /empty token/);
});

test('each unparseable field is named', () => {
  assert.throws(() => parseIdentity(`x\n1\np\n${TOKEN}\n`), /pid "x" is not an integer/);
  assert.throws(() => parseIdentity(`1\nx\np\n${TOKEN}\n`), /start time "x" is not an integer/);
  assert.throws(() => parseIdentity(`1\n1\nz\n${TOKEN}\n`), /source "z" is not one of/);
});

test('an unset descriptor variable says the daemon is launcher-started', () => {
  assert.throws(() => readIdentity({}), /is not set/);
  assert.throws(() => readIdentity({ [IDENTITY_FD_VAR]: 'x' }), /is not a descriptor/);
});

test('the record is read from the descriptor the environment names', () => {
  const dir = mkdtempSync(resolve(tmpdir(), 'identity-handoff-'));
  const path = resolve(dir, 'handoff');
  writeFileSync(path, `77\n1700145620\np\n${TOKEN}\n`);
  const fd = openSync(path, 'r');
  try {
    const identity = readIdentity({ [IDENTITY_FD_VAR]: String(fd) });
    assert.equal(identity.pid, 77);
    assert.equal(identity.token, TOKEN);
  } finally {
    closeSync(fd);
  }
});

test('the shared fixture is the record the Rust launcher renders', () => {
  // The one wire format, read from the same bytes by both sides. The Rust test
  // that renders it reads this very file, so a change on either side of the
  // language boundary fails here rather than at run time.
  const fixture = readFileSync(
    new URL('./__fixtures__/identity-handoff.txt', import.meta.url),
    'utf8'
  );
  assert.deepEqual(parseIdentity(fixture), {
    pid: 4242,
    start_time: 1700145620,
    start_time_source: 'probe',
    token: '0123456789abcdef0123456789abcdef',
  });
});
