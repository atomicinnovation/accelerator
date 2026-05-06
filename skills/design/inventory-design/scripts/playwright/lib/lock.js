// Mutex using mkdir-based locking (always atomic on POSIX).
// Mirrors the bash flock-or-mkdir fallback from ensure-playwright.sh.
// In the JS layer we always use mkdir since we can't hold a file descriptor
// across exec boundaries the way bash's `exec 9>$FILE; flock -n 9` does.
// ACCELERATOR_LOCK_FORCE_MKDIR=1 is accepted for parity with the bash scripts
// but has no effect here (mkdir is already the mechanism).

import { mkdirSync, rmdirSync, existsSync } from 'node:fs';

const LOCK_TIMEOUT_MS = parseInt(process.env.ACCELERATOR_LOCK_TIMEOUT_MS || '300000', 10);
const LOCK_POLL_MS = 100;

// Acquire a lock on lockFile. Returns a release() function.
// The lock dir is lockFile + '.d' (mirrors bash mkdir fallback convention).
// Throws { code: 'ELOCKTIMEOUT' } if lock cannot be acquired within timeout.
export async function acquireLock(lockFile) {
  const lockDir = lockFile + '.d';
  const timeoutMs = parseInt(process.env.ACCELERATOR_LOCK_TIMEOUT_MS || '300000', 10);
  const startMs = Date.now();

  while (true) {
    try {
      mkdirSync(lockDir, { recursive: false });
      let released = false;
      return function release() {
        if (released) return;
        released = true;
        try { rmdirSync(lockDir); } catch {}
      };
    } catch (e) {
      if (e.code !== 'EEXIST') throw e;
      if (Date.now() - startMs > timeoutMs) {
        throw Object.assign(
          new Error(`Lock timeout after ${timeoutMs}ms waiting for ${lockDir}`),
          { code: 'ELOCKTIMEOUT', lockDir },
        );
      }
      await sleep(LOCK_POLL_MS);
    }
  }
}

export function isLocked(lockFile) {
  return existsSync(lockFile + '.d');
}

function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}
