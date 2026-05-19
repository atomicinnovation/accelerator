// State directory management and atomic file-write helpers.

import { writeFileSync, renameSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import { execSync } from 'node:child_process';

export const SERVER_INFO_FILE = 'server-info.json';
export const SERVER_PID_FILE = 'server.pid';
export const SERVER_LOG_FILE = 'server.log';
export const BOOTSTRAP_LOG_FILE = 'server.bootstrap.log';
export const SERVER_STOPPED_FILE = 'server-stopped.json';
export const LAUNCHER_LOCK_FILE = 'launcher.lock';

// Atomically write data to path via a tmp sibling file + rename.
export function atomicWrite(filePath, content, mode = 0o600) {
  const dir = resolve(filePath, '..');
  const tmp = resolve(dir, `.tmp-${randomBytes(6).toString('hex')}`);
  try {
    writeFileSync(tmp, content, { mode });
    renameSync(tmp, filePath);
  } catch (err) {
    try { rmSync(tmp, { force: true }); } catch {}
    throw err;
  }
}

export function ensureStateDir(stateDir) {
  mkdirSync(stateDir, { recursive: true, mode: 0o700 });
}

// Return seconds-resolution kernel-recorded start time for our own
// process — matching what launcher-helpers.sh `start_time_of` reads.
// Using Date.now() at server.listen() time can drift a whole second
// past `ps lstart` on busy systems (module loading takes time), which
// then makes the launcher's reuse short-circuit declare a live daemon
// stale and respawn a fresh one. Reading from the same source as the
// launcher eliminates the race.
export function processStartSeconds() {
  try {
    if (process.platform === 'linux') {
      const stat = readFileSync('/proc/self/stat', 'utf8');
      const fields = stat.replace(/^.*\) /, '').split(' ');
      const startticks = parseInt(fields[19], 10);
      const btimeMatch = readFileSync('/proc/stat', 'utf8').match(/^btime (\d+)/m);
      const hz = parseInt(execSync('getconf CLK_TCK', { encoding: 'utf8' }).trim(), 10);
      if (btimeMatch && hz > 0 && Number.isFinite(startticks)) {
        return parseInt(btimeMatch[1], 10) + Math.floor(startticks / hz);
      }
    } else if (process.platform === 'darwin') {
      const out = execSync(`ps -p ${process.pid} -o lstart=`, {
        env: { ...process.env, LANG: 'C', LC_ALL: 'C' },
        encoding: 'utf8',
      }).trim();
      const d = new Date(out);
      if (!isNaN(d.getTime())) return Math.floor(d.getTime() / 1000);
    }
  } catch {}
  return Math.floor(Date.now() / 1000);
}

export function writeServerInfo(stateDir, info) {
  atomicWrite(resolve(stateDir, SERVER_INFO_FILE), JSON.stringify(info));
  atomicWrite(resolve(stateDir, SERVER_PID_FILE), String(info.pid));
}

export function writeServerStopped(stateDir, reason, extra = {}) {
  const data = { protocol: 1, reason, stopped_at: new Date().toISOString(), ...extra };
  atomicWrite(resolve(stateDir, SERVER_STOPPED_FILE), JSON.stringify(data));
}

export function readServerInfo(stateDir) {
  const p = resolve(stateDir, SERVER_INFO_FILE);
  if (!existsSync(p)) return null;
  try {
    return JSON.parse(readFileSync(p, 'utf8'));
  } catch {
    return null;
  }
}

export function readServerPid(stateDir) {
  const p = resolve(stateDir, SERVER_PID_FILE);
  if (!existsSync(p)) return null;
  try {
    const raw = readFileSync(p, 'utf8').trim();
    const pid = parseInt(raw, 10);
    return Number.isFinite(pid) && pid > 0 ? pid : null;
  } catch {
    return null;
  }
}

export function removeServerFiles(stateDir) {
  for (const name of [SERVER_INFO_FILE, SERVER_PID_FILE]) {
    try { rmSync(resolve(stateDir, name), { force: true }); } catch {}
  }
}
