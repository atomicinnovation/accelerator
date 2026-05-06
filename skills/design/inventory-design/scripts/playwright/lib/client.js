// HTTP client that connects to a running daemon and sends a single command.
// Reads server-info.json from the state dir; returns the response on stdout.

import { request } from 'node:http';
import { readServerInfo } from './state.js';
import { makeError, PROTOCOL } from './errors.js';

// Send one command to the daemon and print the JSON response to stdout.
// Returns the parsed response object.
export async function callRemote(stateDir, command, args = {}) {
  const info = readServerInfo(stateDir);
  if (!info?.url) {
    const err = makeError({ error: 'no-daemon', message: 'No running daemon found. Run the command again; run.sh will spawn one.', category: 'usage', retryable: false });
    process.stdout.write(JSON.stringify(err) + '\n');
    return err;
  }

  const body = JSON.stringify({ protocol: PROTOCOL, command, ...args });
  return new Promise((resolve, reject) => {
    const u = new URL(info.url);
    const req = request({
      hostname: u.hostname,
      port: u.port,
      path: '/',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body),
      },
    }, res => {
      const chunks = [];
      res.on('data', c => chunks.push(c));
      res.on('end', () => {
        const raw = Buffer.concat(chunks).toString('utf8');
        let parsed;
        try { parsed = JSON.parse(raw); }
        catch {
          parsed = makeError({ error: 'invalid-response', message: 'Daemon returned non-JSON response', category: 'protocol', retryable: false, details: { raw: raw.slice(0, 200) } });
        }
        process.stdout.write(JSON.stringify(parsed) + '\n');
        resolve(parsed);
      });
    });
    req.on('error', err => {
      const envelope = makeError({ error: 'connection-failed', message: `Cannot connect to daemon at ${info.url}: ${err.message}`, category: 'protocol', retryable: false });
      process.stdout.write(JSON.stringify(envelope) + '\n');
      resolve(envelope);
    });
    req.write(body);
    req.end();
  });
}
