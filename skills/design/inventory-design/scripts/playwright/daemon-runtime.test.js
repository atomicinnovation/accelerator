// Daemon tests that need a real Playwright runtime.
//
// They live at the playwright/ root, outside the `lib/*.test.js` glob the unit
// lane discovers, because `node --test` has no way to run only some of a file's
// cases per lane. A runtime-dependent assertion therefore has to live in a file
// the unit lane never reaches, not merely be labelled opt-in inside one it does.
//
// Nothing here skips. The opt-in task's own preflight guarantees a runtime
// before any of this runs, so an absent one is a visible failure rather than a
// silent pass — the pattern this suite exists to stop reproducing.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, realpathSync } from 'node:fs';
import { fork } from 'node:child_process';
import { createServer, request } from 'node:http';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';

import { readServerInfo } from './lib/state.js';
import { requireRuntime } from './runtime-preflight.js';

const RUN_JS = resolve(import.meta.dirname, 'run.js');
const HANDOFF_FD = 4;
const TEST_TOKEN = 'runtimetokenruntimetokenruntime0';

function withTmpDir(fn) {
  const dir = realpathSync(mkdtempSync(resolve(tmpdir(), 'daemon-runtime-')));
  return Promise.resolve(fn(dir)).finally(async () => {
    await new Promise(r => setTimeout(r, 300));
    try { rmSync(dir, { recursive: true, force: true }); } catch {}
  });
}

// The launcher's job in production: the pid is not knowable until the child
// exists, so the record travels down a pipe written after the fork.
function forkDaemon(dir, extraEnv = {}) {
  const child = fork(RUN_JS, ['daemon', '--state-dir', dir], {
    env: {
      ...process.env,
      ACCELERATOR_PLAYWRIGHT_IDENTITY_FD: String(HANDOFF_FD),
      ACCELERATOR_PLAYWRIGHT_IDLE_MS: '20000',
      ...extraEnv,
    },
    detached: false,
    stdio: ['pipe', 'pipe', 'pipe', 'ipc', 'pipe'],
  });
  child.stdio[HANDOFF_FD].end(`${child.pid}\n1700000000\np\n${TEST_TOKEN}\n`);
  return child;
}

async function send(url, body) {
  return new Promise((resolvePromise, reject) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = request(
      {
        hostname: u.hostname,
        port: u.port,
        path: '/',
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'content-length': Buffer.byteLength(data),
          'x-accelerator-token': TEST_TOKEN,
        },
      },
      res => {
        const chunks = [];
        res.on('data', c => chunks.push(c));
        res.on('end', () => {
          try { resolvePromise(JSON.parse(Buffer.concat(chunks).toString('utf8'))); }
          catch (e) { reject(e); }
        });
      }
    );
    req.on('error', reject);
    req.write(data);
    req.end();
  });
}

async function waitForInfo(stateDir, ms = 10000) {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    const info = readServerInfo(stateDir);
    if (info?.url) return info;
    await new Promise(r => setTimeout(r, 50));
  }
  throw new Error(`server-info.json did not appear within ${ms}ms in ${stateDir}`);
}

// Runs `body` against a started daemon, stopping it afterwards. The third
// argument exposes the daemon's accumulated stderr — where its warnings land in
// production via the bootstrap log — so a test can assert a warning without a
// header value in it.
async function withDaemon(body) {
  const nsRoot = requireRuntime();
  return withTmpDir(async dir => {
    const child = forkDaemon(dir, { ACCELERATOR_PLAYWRIGHT_NS_ROOT: nsRoot });
    let stderr = '';
    child.stderr?.on('data', chunk => {
      stderr += chunk.toString('utf8');
    });
    try {
      const info = await waitForInfo(dir);
      await body(info, dir, { stderr: () => stderr });
    } finally {
      child.kill('SIGTERM');
    }
  });
}

test('ping returns ok: true without launching a browser', { timeout: 20000 }, async () => {
  await withDaemon(async info => {
    const res = await send(info.url, { protocol: 1, command: 'ping' });
    assert.equal(res.ok, true);
    assert.equal(typeof res.node, 'string');
    assert.equal(typeof res.chromium, 'string');
  });
});

// A privacy contract over the daemon — what a crawl may hand back about a
// page's anchors. Extracting real anchors from a real page needs a real
// Chromium, so unlike the spawn properties there is no honest way to make this
// runtime-free.

const FIXTURE_URL = `file://${resolve(import.meta.dirname, '__fixtures__/links.html')}`;

async function linksOf(info, url) {
  await send(info.url, { protocol: 1, command: 'navigate', url });
  return send(info.url, { protocol: 1, command: 'links' });
}

test('links names the current page and resolves same-origin paths', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const body = JSON.stringify(res);

    assert.ok(Array.isArray(res.links), body);
    assert.ok(res.url.startsWith('file://'), body);
    assert.ok(body.includes('"pathname":"/work-items"'), body);
    assert.ok(body.includes('/library/work-items'), body);
  });
});

test('links normalises anchor text and preserves role verbatim', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const texts = res.links.map(l => l.text);
    const roles = res.links.map(l => l.role);

    assert.ok(texts.includes('Library Items'), JSON.stringify(texts));
    assert.ok(roles.includes('button'), JSON.stringify(roles));
    assert.ok(roles.includes(null), JSON.stringify(roles));
  });
});

test('every anchor on an opaque-origin page reports same_origin false', { timeout: 30000 }, async () => {
  // A file:// page has an opaque origin, so nothing can be same-origin with
  // it. Reporting otherwise would tell a crawler to follow links it should not.
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    assert.ok(res.links.length > 0);
    assert.ok(
      res.links.every(l => l.same_origin === false),
      JSON.stringify(res.links)
    );
  });
});

test('links reports each anchor scheme', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const schemes = new Set(res.links.map(l => l.scheme));
    for (const scheme of ['file', 'https', 'mailto']) {
      assert.ok(schemes.has(scheme), `${scheme} missing from ${[...schemes]}`);
    }
  });
});

test('links returns no raw href, resolved URL, query string or fragment', { timeout: 30000 }, async () => {
  // The contract that makes `links` safe to hand to a model: it describes where
  // an anchor points without echoing anything the page put in the URL.
  await withDaemon(async info => {
    const res = await linksOf(info, FIXTURE_URL);
    const body = JSON.stringify(res);

    for (const forbidden of [
      '"href"',
      '"resolved"',
      '"host"',
      '"sameOriginRaw"',
      'q=foo',
      '#top',
    ]) {
      assert.ok(!body.includes(forbidden), `${forbidden} leaked into ${body}`);
    }
  });
});

test('links on about:blank returns an empty list rather than an error', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await linksOf(info, 'about:blank');
    assert.equal(res.url, 'about:blank');
    assert.deepEqual(res.links, []);
  });
});

// Navigation classification over a real browser: the route handler must fire per
// redirect hop and per page-initiated navigation, which a mocked route cannot
// prove. A link-local host stands in for the metadata endpoint; the loopback
// server that redirects to it is itself always allowed, so only the hop can
// produce a link-local refusal.

const INTERNAL_URL = 'http://169.254.169.254/';

async function withServer(handler, body) {
  const server = createServer(handler);
  await new Promise(started => server.listen(0, '127.0.0.1', started));
  const url = `http://127.0.0.1:${server.address().port}/`;
  try {
    return await body(url);
  } finally {
    await new Promise(closed => server.close(closed));
  }
}

function redirectTo(target) {
  return (_req, res) => {
    res.writeHead(302, { location: target });
    res.end();
  };
}

function serveHtml(html) {
  return (_req, res) => {
    res.writeHead(200, { 'content-type': 'text/html' });
    res.end(html);
  };
}

test('a redirect to a link-local host is refused, not followed', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    await withServer(redirectTo(INTERNAL_URL), async serverUrl => {
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(res.error, 'navigation-refused', JSON.stringify(res));
      assert.equal(res.retryable, false, JSON.stringify(res));
      assert.equal(res.details.classification, 'link-local');
      assert.ok(!res.message.includes('?'), res.message);
    });
  });
});

test('allow_internal lets a link-local redirect past classification', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    await withServer(redirectTo(INTERNAL_URL), async serverUrl => {
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
        allow_internal: true,
      });
      // Classification allows the hop; the fetch to a dead link-local host then
      // fails, so the outcome is a connection error, never navigation-refused.
      assert.notEqual(res.error, 'navigation-refused', JSON.stringify(res));
    });
  });
});

test('two navigations on one daemon are judged under their own allowances', { timeout: 40000 }, async () => {
  await withDaemon(async info => {
    await withServer(redirectTo(INTERNAL_URL), async serverUrl => {
      const allowed = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
        allow_internal: true,
      });
      assert.notEqual(
        allowed.error,
        'navigation-refused',
        JSON.stringify(allowed)
      );
      const refused = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(refused.error, 'navigation-refused', JSON.stringify(refused));
      assert.equal(refused.details.classification, 'link-local');
    });
  });
});

test('a sub-frame navigation to an internal host does not mask the main frame', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const html = `<!doctype html><meta charset=utf8><iframe src="${INTERNAL_URL}"></iframe>main`;
    await withServer(serveHtml(html), async serverUrl => {
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(res.ok, true, JSON.stringify(res));
    });
  });
});

test('a click that navigates to an internal host is refused', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const html = `<!doctype html><meta charset=utf8><a id="go" href="${INTERNAL_URL}">go</a>`;
    await withServer(serveHtml(html), async serverUrl => {
      await send(info.url, { protocol: 1, command: 'navigate', url: serverUrl });
      const res = await send(info.url, {
        protocol: 1,
        command: 'click',
        ref: '#go',
      });
      assert.equal(res.error, 'navigation-refused', JSON.stringify(res));
      assert.equal(res.details.classification, 'link-local');
    });
  });
});

test('a scripted redirect to an internal host after load is aborted', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const html =
      `<!doctype html><meta charset=utf8><body>ok<script>` +
      `setTimeout(function(){location.href=${JSON.stringify(INTERNAL_URL)};},50);` +
      `</script></body>`;
    await withServer(serveHtml(html), async serverUrl => {
      const nav = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: serverUrl,
      });
      assert.equal(nav.ok, true, JSON.stringify(nav));
      await new Promise(r => setTimeout(r, 400));
      const location = await send(info.url, {
        protocol: 1,
        command: 'evaluate',
        expression: 'location.href',
      });
      assert.ok(
        !String(location.result).startsWith('http://169.254'),
        JSON.stringify(location)
      );
    });
  });
});

// The auth-header path, end to end over a real browser: the header is attached
// on the crawl's declared location origin and stripped on every other. The tests
// play the client's role, putting location_url and auth_header in the request
// body directly; the daemon reads no environment for auth.

const BEARER = 'design-crawl-bearer-value';
const AUTH_HEADER = `Authorization: Bearer ${BEARER}`;

function authFor(locationUrl) {
  return { location_url: locationUrl, auth_header: AUTH_HEADER };
}

function gated(secret) {
  return (req, res) => {
    if (req.headers.authorization === `Bearer ${BEARER}`) {
      res.writeHead(200, { 'content-type': 'text/html' });
      res.end(`<!doctype html><meta charset=utf8><body>${secret}</body>`);
    } else {
      res.writeHead(401, { 'content-type': 'text/html' });
      res.end('<!doctype html><meta charset=utf8><body>denied</body>');
    }
  };
}

function recorder(seen, html = '<!doctype html><meta charset=utf8><body>ok</body>') {
  return (req, res) => {
    seen.push({ path: req.url, authorization: req.headers.authorization });
    res.writeHead(200, { 'content-type': 'text/html' });
    res.end(html);
  };
}

// Answers the CORS preflight for a cross-origin fetch that carries Authorization,
// so the actual GET fires and a deleted header is observable as a real absence
// on the target rather than a request that never left.
function corsRecorder(seen) {
  const cors = {
    'access-control-allow-origin': '*',
    'access-control-allow-headers': 'authorization',
  };
  return (req, res) => {
    seen.push({ method: req.method, authorization: req.headers.authorization });
    if (req.method === 'OPTIONS') {
      res.writeHead(204, cors);
      res.end();
    } else {
      res.writeHead(200, { ...cors, 'content-type': 'text/plain' });
      res.end('asset');
    }
  };
}

function callsB(bUrl) {
  return serveHtml(
    `<!doctype html><meta charset=utf8><body>A<script>` +
      `window.fetchB=function(){return fetch(${JSON.stringify(bUrl)},` +
      `{headers:{Authorization:'Bearer ${BEARER}'}}).then(function(r){return r.status;});};` +
      `</script></body>`
  );
}

function firesLate(seen, latePath) {
  return recorder(
    seen,
    `<!doctype html><meta charset=utf8><body>ready<script>` +
      `window.fireLate=function(){return fetch(${JSON.stringify(latePath)})` +
      `.then(function(r){return r.status;});};` +
      `</script></body>`
  );
}

test('a gated page loads with the bearer attached on the location origin', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    await withServer(gated('SECRET-CONTENT'), async aUrl => {
      const nav = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: aUrl,
        ...authFor(aUrl),
      });
      assert.equal(nav.ok, true, JSON.stringify(nav));
      const body = await send(info.url, {
        protocol: 1,
        command: 'evaluate',
        expression: 'document.body.textContent',
        ...authFor(aUrl),
      });
      assert.ok(String(body.result).includes('SECRET-CONTENT'), JSON.stringify(body));
    });
  });
});

test('a cross-origin fetch is stripped of the bearer yet completes', { timeout: 30000 }, async () => {
  // Mutation-resistant: the page sets Authorization itself and fixture B answers
  // the CORS preflight, so a deleted strip branch would let the bearer reach B.
  await withDaemon(async info => {
    const bSeen = [];
    await withServer(corsRecorder(bSeen), async bUrl => {
      await withServer(callsB(bUrl), async aUrl => {
        await send(info.url, {
          protocol: 1,
          command: 'navigate',
          url: aUrl,
          ...authFor(aUrl),
        });
        const res = await send(info.url, {
          protocol: 1,
          command: 'evaluate',
          expression: 'window.fetchB()',
          ...authFor(aUrl),
        });
        assert.equal(res.result, 200, JSON.stringify(res));
        const gets = bSeen.filter(entry => entry.method === 'GET');
        assert.ok(gets.length > 0, `B saw no GET: ${JSON.stringify(bSeen)}`);
        assert.ok(
          gets.every(entry => entry.authorization !== `Bearer ${BEARER}`),
          JSON.stringify(bSeen)
        );
      });
    });
  });
});

test('a cross-origin navigate carries no bearer to the off-site origin', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const bSeen = [];
    await withServer(recorder(bSeen), async bUrl => {
      // The location is declared as a different origin; navigating straight to B
      // plays a followed external link. The header keys to the declared location,
      // not the navigate target, so B is reached without it.
      const declaredLocation = 'http://127.0.0.1:1/';
      const res = await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: bUrl,
        ...authFor(declaredLocation),
      });
      assert.equal(res.ok, true, JSON.stringify(res));
      assert.ok(
        bSeen.every(entry => entry.authorization !== `Bearer ${BEARER}`),
        JSON.stringify(bSeen)
      );
    });
  });
});

test('a cross-origin redirect does not leak the header to the target', { timeout: 30000 }, async () => {
  // A boundary assertion, not mutation-resistant: the browser does not forward
  // Authorization across origins on a redirect, so no bearer is present on the B
  // hop to strip. It still catches an origin-blind injection mutant.
  await withDaemon(async info => {
    const bSeen = [];
    await withServer(recorder(bSeen), async bUrl => {
      await withServer(redirectTo(bUrl), async aUrl => {
        const res = await send(info.url, {
          protocol: 1,
          command: 'navigate',
          url: aUrl,
          ...authFor(aUrl),
        });
        assert.equal(res.ok, true, JSON.stringify(res));
        assert.ok(
          bSeen.every(entry => entry.authorization !== `Bearer ${BEARER}`),
          JSON.stringify(bSeen)
        );
      });
    });
  });
});

test('a same-origin subresource fired during a following command stays authenticated', { timeout: 30000 }, async () => {
  // The subresource is dispatched inside a following `evaluate` that carries the
  // auth fields, so it fires deterministically while the daemon's per-request
  // auth state is set. Because that state is cleared at request end, a mutant
  // that authenticated `navigate` only would strip this fetch.
  await withDaemon(async info => {
    const aSeen = [];
    await withServer(firesLate(aSeen, '/late'), async aUrl => {
      await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: aUrl,
        ...authFor(aUrl),
      });
      const res = await send(info.url, {
        protocol: 1,
        command: 'evaluate',
        expression: 'window.fireLate()',
        ...authFor(aUrl),
      });
      assert.equal(res.result, 200, JSON.stringify(res));
      const late = aSeen.filter(entry => entry.path === '/late');
      assert.ok(late.length > 0, `no /late request recorded: ${JSON.stringify(aSeen)}`);
      assert.ok(
        late.every(entry => entry.authorization === `Bearer ${BEARER}`),
        JSON.stringify(aSeen)
      );
    });
  });
});

test('a header declared without a location warns and strips', { timeout: 30000 }, async () => {
  await withDaemon(async (info, _dir, ctx) => {
    const aSeen = [];
    await withServer(recorder(aSeen), async aUrl => {
      await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: aUrl,
        auth_header: AUTH_HEADER,
      });
      assert.ok(
        aSeen.every(entry => entry.authorization !== `Bearer ${BEARER}`),
        JSON.stringify(aSeen)
      );
      await new Promise(r => setTimeout(r, 150));
      const log = ctx.stderr();
      assert.ok(log.includes('no location origin resolved'), log);
      assert.ok(!log.includes(BEARER), 'the warning must not carry the header value');
    });
  });
});

test('a warm daemon does not bleed a prior crawl auth onto an unconfigured re-crawl', { timeout: 30000 }, async () => {
  // Same origin both times: a design that pinned origin and header across
  // requests would still attach on the second, unconfigured crawl.
  await withDaemon(async info => {
    const aSeen = [];
    await withServer(recorder(aSeen), async aUrl => {
      await send(info.url, {
        protocol: 1,
        command: 'navigate',
        url: aUrl,
        ...authFor(aUrl),
      });
      const afterFirst = aSeen.length;
      assert.ok(
        aSeen.slice(0, afterFirst).some(entry => entry.authorization === `Bearer ${BEARER}`),
        `first crawl should attach: ${JSON.stringify(aSeen)}`
      );
      await send(info.url, { protocol: 1, command: 'navigate', url: aUrl });
      const second = aSeen.slice(afterFirst);
      assert.ok(second.length > 0, 'second navigate recorded nothing');
      assert.ok(
        second.every(entry => entry.authorization !== `Bearer ${BEARER}`),
        JSON.stringify(second)
      );
    });
  });
});

test('a refused origin is still blocked with the auth-header route installed', { timeout: 30000 }, async () => {
  await withDaemon(async info => {
    const res = await send(info.url, {
      protocol: 1,
      command: 'navigate',
      url: INTERNAL_URL,
      ...authFor(INTERNAL_URL),
    });
    assert.equal(res.error, 'navigation-refused', JSON.stringify(res));
    assert.equal(res.details.classification, 'link-local');
  });
});
