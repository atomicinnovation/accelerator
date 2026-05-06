// Process start-time detection for PID identity checks (reuse short-circuit).
// Mirrors launcher-helpers.sh start_time_of() logic exactly.
// Both implementations emit UTC seconds since epoch.

import { readFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

// Parse the tail (after removing "pid (comm) ") of a /proc/<pid>/stat line.
// Returns starttime in ticks (field 20 of the tail = field 22 of the full stat line).
export function parseProcStatTail(tail) {
  const fields = tail.trim().split(/\s+/);
  const starttime = parseInt(fields[19], 10); // 0-indexed field 19 = field 20 = starttime
  if (!Number.isFinite(starttime)) throw new Error('Cannot parse starttime from /proc stat tail');
  return starttime;
}

// Parse btime from a /proc/stat file content.
export function parseProcStatBtime(statContent) {
  const m = statContent.match(/^btime\s+(\d+)/m);
  if (!m) throw new Error('Cannot find btime in /proc/stat');
  return parseInt(m[1], 10);
}

// Parse macOS ps lstart output: "Wed Jan 15 12:00:00 2025" → UTC epoch seconds.
// Parses assuming UTC (cross-validation always runs under TZ=UTC).
export function parsePsLstart(lstartOutput) {
  const MONTHS = { Jan: 0, Feb: 1, Mar: 2, Apr: 3, May: 4, Jun: 5, Jul: 6, Aug: 7, Sep: 8, Oct: 9, Nov: 10, Dec: 11 };
  const parts = lstartOutput.trim().split(/\s+/);
  if (parts.length < 5) throw new Error(`Cannot parse ps lstart output: ${JSON.stringify(lstartOutput)}`);
  // parts[0]=weekday parts[1]=month parts[2]=day parts[3]=HH:MM:SS parts[4]=year
  const month = MONTHS[parts[1]];
  if (month === undefined) throw new Error(`Unknown month: ${parts[1]}`);
  const day = parseInt(parts[2], 10);
  const [hours, minutes, seconds] = parts[3].split(':').map(Number);
  const year = parseInt(parts[4], 10);
  return Math.floor(Date.UTC(year, month, day, hours, minutes, seconds) / 1000);
}

// Read the start time of a process. Returns UTC epoch seconds, or null.
// hz: optional override for CLK_TCK (for testing)
export function startTimeOf(pid, { hz: hzOverride } = {}) {
  if (existsSync(`/proc/${pid}/stat`) && existsSync('/proc/stat')) {
    try {
      const statRaw = readFileSync(`/proc/${pid}/stat`, 'utf8');
      // Remove "pid (comm) " prefix — the comm may contain spaces
      const tail = statRaw.replace(/^\d+ \(.*?\) /, '');
      const starttime = parseProcStatTail(tail);
      const btimeContent = readFileSync('/proc/stat', 'utf8');
      const btime = parseProcStatBtime(btimeContent);
      const hz = hzOverride ?? clkTck();
      if (hz <= 0) return null;
      return btime + Math.floor(starttime / hz);
    } catch {
      return null;
    }
  }

  // macOS path
  try {
    const raw = execFileSync('ps', ['-p', String(pid), '-o', 'lstart='], {
      // TZ=UTC so ps formats lstart in UTC, matching parsePsLstart's Date.UTC() parser.
      // This ensures identity.js and launcher-helpers.sh start_time_of() agree when
      // both run under TZ=UTC (cross-validation requirement).
      env: { ...process.env, LC_ALL: 'C', TZ: 'UTC' },
      encoding: 'utf8',
      timeout: 5000,
    });
    const collapsed = raw.replace(/  +/g, ' ').trim();
    if (!collapsed) return null;
    return parsePsLstart(collapsed);
  } catch {
    return null;
  }
}

function clkTck() {
  try {
    const out = execFileSync('getconf', ['CLK_TCK'], { encoding: 'utf8', timeout: 2000 });
    const n = parseInt(out.trim(), 10);
    return Number.isFinite(n) && n > 0 ? n : 100;
  } catch {
    return 100;
  }
}
