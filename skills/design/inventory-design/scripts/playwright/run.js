#!/usr/bin/env node
// CLI dispatch. Routes to daemon.start() or client.callRemote().
// run.sh calls: node run.js <command> [json-args]
// run.sh daemon launch: node run.js daemon --state-dir <dir> --owner-pid <pid>

import { callRemote } from './lib/client.js';

const args = process.argv.slice(2);
const command = args[0];

if (!command) {
  process.stderr.write('usage: run.js <command> [json-args]\n');
  process.exit(2);
}

const stateDir = process.env.ACCELERATOR_PLAYWRIGHT_STATE_DIR;

if (command === 'daemon') {
  // Internal subcommand: start the daemon server
  let stateDirArg = stateDir;
  let ownerPid = 0;
  for (let i = 1; i < args.length; i++) {
    if (args[i] === '--state-dir') stateDirArg = args[++i];
    else if (args[i] === '--owner-pid') ownerPid = parseInt(args[++i], 10);
  }
  if (!stateDirArg) {
    process.stderr.write('run.js daemon: --state-dir is required\n');
    process.exit(2);
  }
  const { startDaemon } = await import('./lib/daemon.js');
  await startDaemon({ stateDir: stateDirArg, ownerPid: ownerPid || 0 });
} else {
  // Client subcommands: forward to the running daemon
  if (!stateDir) {
    process.stderr.write('run.js: ACCELERATOR_PLAYWRIGHT_STATE_DIR is not set (use run.sh, not run.js directly)\n');
    process.exit(2);
  }
  let extraArgs = {};
  if (args[1]) {
    try {
      extraArgs = JSON.parse(args[1]);
    } catch {
      process.stderr.write(`run.js: second argument must be valid JSON, got: ${args[1]}\n`);
      process.exit(2);
    }
  }
  await callRemote(stateDir, command, extraArgs);
}
