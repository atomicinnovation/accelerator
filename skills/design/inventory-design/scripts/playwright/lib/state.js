// State directory management and atomic file-write helpers.

import { writeFileSync, renameSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';

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
