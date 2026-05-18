// Loads the `playwright` package from a namespaced cache root when given,
// otherwise falls back to normal package resolution.
//
// We must select the ESM entry (advertised by pkg.exports['.'].import,
// typically ./index.mjs). pkg.main points at the CJS shim, and a dynamic
// import() of CJS wraps the entire module.exports under `default` — so
// the named exports chromium/firefox/webkit/devices/selectors/errors
// would all be undefined on the returned namespace.
//
// Trust boundary: `nsRoot` is treated as fully trusted. In production it
// is set by run.sh from a lockhash digest under
// $HOME/.cache/accelerator/playwright. The value drives an arbitrary
// import() call; any new caller must derive it from an equally trusted
// source.
//
// Layout contract: the path `node_modules/playwright/package.json`
// relative to `nsRoot` is owned by ensure-playwright.sh — keep in sync.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function resolvePlaywrightPkgPath(nsRoot) {
  return resolve(nsRoot, 'node_modules/playwright/package.json');
}

export async function importPlaywright({ nsRoot } = {}) {
  if (!nsRoot) return import('playwright');

  const pkgPath = resolvePlaywrightPkgPath(nsRoot);
  let pkg;
  try {
    pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
  } catch (err) {
    if (err.code !== 'ENOENT') {
      throw new Error(
        `playwright-loader: failed to read ${pkgPath}: ${err.message}`
      );
    }
  }

  let entryFile = 'index.mjs';
  if (pkg) {
    const subpath = pkg.exports?.['.'];
    if (subpath !== undefined && subpath !== null && typeof subpath === 'object') {
      if (typeof subpath.import === 'string') {
        entryFile = subpath.import;
      } else {
        // exports['.'] declares conditions but no string `import` entry is
        // reachable (e.g. nested condition tree, removed condition). Fail
        // loudly rather than silently falling through to pkg.main (CJS) —
        // that fall-through is what produced the original bug.
        throw new Error(
          `playwright-loader: pkg.exports['.'].import is not a string at ${pkgPath}; ` +
          `the exports map shape may have changed. Update the loader.`
        );
      }
    } else if (typeof pkg.main === 'string') {
      entryFile = pkg.main;
    }
  }

  const entryUrl = pathToFileURL(
    resolve(nsRoot, 'node_modules/playwright', entryFile)
  ).href;
  return import(entryUrl);
}
