// Reads the identity record the launcher hands over an inherited pipe.
//
// The launcher learns this daemon's pid the moment spawn() returns — too late
// to pass it through the environment, since a child's envp is fixed at exec and
// built before the fork. So the values arrive on a descriptor instead, named by
// ACCELERATOR_PLAYWRIGHT_IDENTITY_FD.
//
// This is read to end of input and parsed BEFORE the daemon publishes anything
// or opens its listening socket. That ordering is the whole point: readiness IS
// the record appearing, so a daemon that published first could be caught with a
// record carrying no start time — which the launcher's reuse verdict reads as
// stale, deletes, and respawns around while this process still holds a port, a
// browser and the crawl's page state.
//
// A short or malformed read is a failure, never a default. A launcher killed
// after spawning but before writing leaves this reading immediate EOF, and the
// daemon must exit rather than run on unsupervised and un-recorded.

import { readFileSync } from 'node:fs';

export const IDENTITY_FD_VAR = 'ACCELERATOR_PLAYWRIGHT_IDENTITY_FD';

// The single tag byte naming where the launcher's start time came from.
const SOURCES = { p: 'probe', w: 'wallclock', u: 'writer-unavailable' };

export class MalformedIdentity extends Error {}

// Parse the wire form: pid\nstart_time\nsource\ntoken\n
//
// Exactly one trailing newline is stripped, not every one: trimming greedily
// would make an empty token indistinguishable from a field that never arrived,
// collapsing two different failures into one.
export function parseIdentity(raw) {
  const body = raw.endsWith('\n') ? raw.slice(0, -1) : raw;
  const fields = raw === '' ? [] : body.split('\n');
  if (fields.length !== 4) {
    throw new MalformedIdentity(
      `identity handoff carried ${fields.length} of 4 fields; the launcher did not complete its write`
    );
  }

  const [rawPid, rawSeconds, rawSource, token] = fields;
  if (!/^-?\d+$/.test(rawPid)) {
    throw new MalformedIdentity(`identity pid ${JSON.stringify(rawPid)} is not an integer`);
  }
  const source = SOURCES[rawSource];
  if (!source) {
    throw new MalformedIdentity(
      `identity start-time source ${JSON.stringify(rawSource)} is not one of p, w, u`
    );
  }
  if (source !== 'writer-unavailable' && !/^\d+$/.test(rawSeconds)) {
    throw new MalformedIdentity(
      `identity start time ${JSON.stringify(rawSeconds)} is not an integer`
    );
  }
  if (token === '') {
    throw new MalformedIdentity('identity carried an empty token');
  }

  return {
    pid: Number(rawPid),
    // The launcher could not probe one, so there is no value to record. The
    // reader treats this the same as a wallclock value: liveness alone, with no
    // PID-recycle guard.
    start_time: source === 'writer-unavailable' ? null : Number(rawSeconds),
    start_time_source: source,
    token,
  };
}

// Read the whole descriptor named by the environment.
//
// The read terminates at EOF rather than blocking: the child's inherited copy
// of the pipe's write end closed when the kernel exec'd this process, and the
// launcher closes its own immediately after writing, so no writable copy
// remains anywhere.
export function readIdentity(env = process.env) {
  const raw = env[IDENTITY_FD_VAR];
  if (!raw) {
    throw new MalformedIdentity(
      `${IDENTITY_FD_VAR} is not set; the daemon is started by the launcher, not directly`
    );
  }
  const fd = Number(raw);
  if (!Number.isInteger(fd) || fd < 0) {
    throw new MalformedIdentity(`${IDENTITY_FD_VAR}=${JSON.stringify(raw)} is not a descriptor`);
  }
  return parseIdentity(readFileSync(fd, 'utf8'));
}
