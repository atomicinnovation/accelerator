// HTTP client that connects to a running daemon and sends a single command.
// Reads server-info.json from the state dir; returns the response on stdout.

import { request } from 'node:http';
import { isIP } from 'node:net';
import { readServerInfo } from './state.js';
import { makeError, PROTOCOL, TOKEN_HEADER } from './errors.js';

// Anything able to write the state dir could otherwise redirect a
// token-bearing request to a host of its choosing, so the recorded URL is
// checked before it is dialled. A literal loopback address, parsed — not a
// string comparison, which `127.0.0.1.evil.com` would pass.
function isLoopbackUrl(raw) {
  let parsed;
  try {
    parsed = new URL(raw);
  } catch {
    return false;
  }
  // A bracketed IPv6 literal arrives with the brackets still attached.
  const host = parsed.hostname.replace(/^\[|\]$/g, '');
  if (!isIP(host)) return false;
  return host === '127.0.0.1' || host === '::1';
}

// Send one command to the daemon and print the JSON response to stdout.
// Returns the parsed response object.
export async function callRemote(stateDir, command, args = {}) {
  const info = readServerInfo(stateDir);
  if (!info?.url) {
    const err = makeError({ error: 'no-daemon', message: 'No running daemon found. Run the command again; the launcher will spawn one.', category: 'usage', retryable: false });
    process.stdout.write(JSON.stringify(err) + '\n');
    return err;
  }

  if (!isLoopbackUrl(info.url)) {
    const err = makeError({ error: 'non-loopback-daemon', message: 'server-info.json names a non-loopback daemon URL; refusing to send the request token to it.', category: 'usage', retryable: false });
    process.stdout.write(JSON.stringify(err) + '\n');
    return err;
  }

  // A caller-supplied payload must not decide which command runs: the spread
  // comes FIRST so the validated command and protocol overwrite anything it
  // carries, and a payload naming either is refused outright rather than
  // silently ignored.
  if (Object.hasOwn(args, 'command') || Object.hasOwn(args, 'protocol')) {
    const err = makeError({ error: 'payload-rejected', message: 'The command payload must not carry its own `command` or `protocol` key.', category: 'usage', retryable: false });
    process.stdout.write(JSON.stringify(err) + '\n');
    return err;
  }

  const body = JSON.stringify({ ...args, protocol: PROTOCOL, command });
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
        // Header, never a query parameter: the daemon refuses the latter even
        // when the token is valid.
        [TOKEN_HEADER]: info.token || '',
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
