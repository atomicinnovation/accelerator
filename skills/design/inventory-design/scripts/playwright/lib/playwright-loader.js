// Imports playwright-core from a driver tree by absolute path.
//
// Node's ESM resolver ignores NODE_PATH, so a bare `import 'playwright-core'`
// would walk node_modules upward from the plugin tree and never reach the
// sealed driver tree. The daemon therefore resolves the ESM entry by absolute
// path under the tree root the executor supplies.
//
// Trust boundary: `nsRoot` is treated as fully trusted. In production it is the
// launcher-resolved, signature-verified driver tree. The value drives an
// arbitrary import() call; any new caller must derive it from an equally
// trusted source.

import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const PACKAGE = 'node_modules/playwright-core';
const ESM_ENTRY = `${PACKAGE}/index.mjs`;

export function resolvePlaywrightPkgPath(nsRoot) {
  return resolve(nsRoot, `${PACKAGE}/package.json`);
}

export async function importPlaywright({ nsRoot } = {}) {
  if (!nsRoot) {
    throw new Error(
      'playwright-loader: a driver tree root is required; the executor did ' +
      'not resolve one'
    );
  }
  const entryUrl = pathToFileURL(resolve(nsRoot, ESM_ENTRY)).href;
  return import(entryUrl);
}
