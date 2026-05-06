// Screenshot path constraint — ensures all screenshot paths resolve under the output root.

import { realpathSync, mkdirSync, existsSync } from 'node:fs';
import { resolve, isAbsolute, dirname, basename } from 'node:path';
import { makeError } from './errors.js';

// Resolve path following symlinks where possible. For components that don't
// exist yet (new screenshot files), realpaths the deepest existing ancestor.
function resolveFollowingSymlinks(inputPath, realRoot) {
  const candidate = isAbsolute(inputPath) ? inputPath : resolve(realRoot, inputPath);

  // Try to realpath the full path (works if file already exists)
  try {
    return realpathSync(candidate);
  } catch {
    // File doesn't exist yet — realpath the parent dir (which must exist for
    // the write to succeed) and append the filename. This catches symlink
    // attacks where the parent is a symlink pointing outside the root.
    const parent = dirname(candidate);
    const name = basename(candidate);
    try {
      return resolve(realpathSync(parent), name);
    } catch {
      // Parent doesn't exist either — return normalized path for containment check
      return resolve(candidate);
    }
  }
}

// Guard a screenshot path against the output root.
// Returns {ok: true, path: <absolute-path>} or {ok: false, error: <envelope>}.
// Reads ACCELERATOR_INVENTORY_OUTPUT_ROOT from the provided env object.
export function guardScreenshotPath(inputPath, { env = process.env } = {}) {
  const rawRoot = env.ACCELERATOR_INVENTORY_OUTPUT_ROOT;
  if (!rawRoot || rawRoot.trim() === '') {
    return {
      ok: false,
      error: makeError({
        error: 'screenshot-output-root-unset',
        message: 'ACCELERATOR_INVENTORY_OUTPUT_ROOT is not set. The skill must populate this env var before spawning the agent.',
        category: 'usage',
        retryable: false,
      }),
    };
  }

  if (!existsSync(rawRoot)) {
    try {
      mkdirSync(rawRoot, { recursive: true, mode: 0o700 });
    } catch {
      // Fall through; realpathSync will fail below with a clear message
    }
  }

  let realRoot;
  try {
    realRoot = realpathSync(rawRoot);
  } catch {
    return {
      ok: false,
      error: makeError({
        error: 'screenshot-output-root-invalid',
        message: `ACCELERATOR_INVENTORY_OUTPUT_ROOT is not accessible: ${rawRoot}`,
        category: 'filesystem',
        retryable: false,
      }),
    };
  }

  const realPath = resolveFollowingSymlinks(inputPath, realRoot);

  if (realPath !== realRoot && !realPath.startsWith(realRoot + '/')) {
    return {
      ok: false,
      error: makeError({
        error: 'screenshot-path-outside-output-root',
        message: `Screenshot path resolves to ${realPath}, which is outside the output root ${realRoot}`,
        category: 'usage',
        retryable: false,
      }),
    };
  }

  return { ok: true, path: realPath };
}
