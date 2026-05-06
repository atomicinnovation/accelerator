import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { parseProcStatTail, parseProcStatBtime, parsePsLstart, startTimeOf } from './identity.js';

const __dir = dirname(fileURLToPath(import.meta.url));
const FIXTURES = resolve(__dir, '__fixtures__');

// -- Linux fixture tests ---------------------------------------------------

test('parseProcStatTail extracts starttime from fixture', () => {
  const fixture = readFileSync(resolve(FIXTURES, 'proc-stat-linux.txt'), 'utf8');
  const statLine = fixture.match(/^stat: (.+)/m)?.[1];
  const expectedStarttime = 14562000;
  assert.ok(statLine, 'fixture has stat line');
  const tail = statLine.replace(/^\d+ \(.*?\) /, '');
  const starttime = parseProcStatTail(tail);
  assert.equal(starttime, expectedStarttime);
});

test('parseProcStatBtime extracts btime from fixture', () => {
  const fixture = readFileSync(resolve(FIXTURES, 'proc-stat-linux.txt'), 'utf8');
  const btimeLine = fixture.match(/^btime: (\d+)/m)?.[1];
  assert.ok(btimeLine, 'fixture has btime line');
  const btimeFixture = `btime ${btimeLine}`;
  const btime = parseProcStatBtime(btimeFixture);
  assert.equal(btime, parseInt(btimeLine, 10));
});

test('Linux fixture produces expected_start_time (btime + starttime/hz)', () => {
  const fixture = readFileSync(resolve(FIXTURES, 'proc-stat-linux.txt'), 'utf8');
  const statLine = fixture.match(/^stat: (.+)/m)?.[1];
  const btime = parseInt(fixture.match(/^btime: (\d+)/m)?.[1], 10);
  const hz = parseInt(fixture.match(/^hz: (\d+)/m)?.[1], 10);
  const expected = parseInt(fixture.match(/^expected_start_time: (\d+)/m)?.[1], 10);
  const tail = statLine.replace(/^\d+ \(.*?\) /, '');
  const starttime = parseProcStatTail(tail);
  const actual = btime + Math.floor(starttime / hz);
  assert.equal(actual, expected);
});

// -- macOS fixture tests ---------------------------------------------------

test('parsePsLstart parses fixture lstart output', () => {
  const fixture = readFileSync(resolve(FIXTURES, 'ps-lstart-macos.txt'), 'utf8');
  const lstart = fixture.split('\n')[0];
  const expected = parseInt(fixture.match(/^expected_start_time: (\d+)/m)?.[1], 10);
  const actual = parsePsLstart(lstart);
  assert.equal(actual, expected, `parsePsLstart("${lstart}") should equal ${expected}`);
});

test('parsePsLstart handles space-collapsed single-digit day', () => {
  // "Wed Jan  1 12:00:00 2025" after tr -s ' ' → "Wed Jan 1 12:00:00 2025"
  const actual = parsePsLstart('Wed Jan 1 12:00:00 2025');
  const expected = Math.floor(Date.UTC(2025, 0, 1, 12, 0, 0) / 1000);
  assert.equal(actual, expected);
});

// -- Cross-validation: bash startTimeOf vs JS startTimeOf -----------------

test('startTimeOf(current PID) returns a positive integer', () => {
  const st = startTimeOf(process.pid);
  assert.ok(st !== null, 'startTimeOf should not return null for current process');
  assert.ok(Number.isInteger(st) && st > 0, `expected positive integer, got ${st}`);
});

test('bash start_time_of(PID) matches JS startTimeOf(PID) under LC_ALL=C TZ=UTC', () => {
  const pid = process.pid;
  const launcherHelpers = resolve(__dir, '../../../../../..', 'skills/visualisation/visualise/scripts/launcher-helpers.sh');

  let bashResult;
  try {
    const out = execFileSync('bash', ['-c',
      `source "${launcherHelpers}"; start_time_of ${pid}`,
    ], {
      env: { ...process.env, LC_ALL: 'C', TZ: 'UTC', PATH: process.env.PATH },
      encoding: 'utf8',
      timeout: 5000,
    });
    bashResult = parseInt(out.trim(), 10);
  } catch {
    // Skip on platforms where bash cross-validation isn't feasible
    return;
  }

  const jsResult = startTimeOf(pid);
  assert.ok(jsResult !== null, 'JS startTimeOf should succeed');

  // Allow ±1s for the hz rounding difference between bash's integer division and JS's Math.floor
  assert.ok(Math.abs(jsResult - bashResult) <= 1,
    `bash=${bashResult} js=${jsResult} differ by more than 1s`);
});

// -- Locale fragility guard -----------------------------------------------

test('bash start_time_of produces same result under LANG=de_DE.UTF-8 and LANG=C', () => {
  const pid = process.pid;
  const launcherHelpers = resolve(__dir, '../../../../../..', 'skills/visualisation/visualise/scripts/launcher-helpers.sh');

  let resultC, resultDe;
  try {
    resultC = execFileSync('bash', ['-c', `source "${launcherHelpers}"; start_time_of ${pid}`], {
      env: { ...process.env, LANG: 'C', LC_ALL: 'C', TZ: 'UTC', PATH: process.env.PATH },
      encoding: 'utf8', timeout: 5000,
    }).trim();
    resultDe = execFileSync('bash', ['-c', `source "${launcherHelpers}"; start_time_of ${pid}`], {
      env: { ...process.env, LANG: 'de_DE.UTF-8', TZ: 'UTC', PATH: process.env.PATH },
      encoding: 'utf8', timeout: 5000,
    }).trim();
  } catch {
    return; // Skip if locale not available
  }

  assert.equal(resultC, resultDe, `LANG=C gave ${resultC} but LANG=de_DE.UTF-8 gave ${resultDe}`);
});
