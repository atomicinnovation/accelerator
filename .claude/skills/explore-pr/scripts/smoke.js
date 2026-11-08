// Usage: node smoke.js PAGE.html [--shots DIR] [--chrome PATH]
//
// Opens a built explore-pr page from file:// in headless Chrome, visits every
// route the page registers, and fails on any uncaught error, unresolved code
// slice, or sideways scroll at phone width. It then reads a tour stop and checks
// that All files counts its lines as read, and marks that stop's file reviewed
// and checks the stop shows as already read. Needs Node 22+ (global WebSocket).

const { spawn, execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

const CHROME_CANDIDATES = [
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
  'google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser',
];
const sleep = ms => new Promise(r => setTimeout(r, ms));

function args(argv) {
  const out = { page: null, shots: null, chrome: process.env.CHROME || null };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--shots') out.shots = argv[++i];
    else if (argv[i] === '--chrome') out.chrome = argv[++i];
    else out.page = argv[i];
  }
  if (!out.page) throw new Error('usage: node smoke.js PAGE.html [--shots DIR] [--chrome PATH]');
  return out;
}

function findChrome(preferred) {
  for (const candidate of [preferred, ...CHROME_CANDIDATES].filter(Boolean)) {
    if (candidate.includes('/')) { if (fs.existsSync(candidate)) return candidate; continue; }
    try { return execFileSync('which', [candidate], { encoding: 'utf8' }).trim(); } catch { /* not on PATH */ }
  }
  return null;
}

async function devtools(chrome) {
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'explore-pr-smoke-'));
  const proc = spawn(chrome, ['--headless=new', '--disable-gpu', '--no-first-run', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
  const portFile = path.join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 80 && !fs.existsSync(portFile); i++) await sleep(100);
  const port = fs.readFileSync(portFile, 'utf8').split('\n')[0];
  let targets = [];
  for (let i = 0; i < 40 && !targets.some(t => t.type === 'page'); i++) {
    try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); } catch { await sleep(100); }
  }
  const ws = new WebSocket(targets.find(t => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  let id = 0;
  const pending = new Map();
  const errors = [];
  ws.onmessage = message => {
    const data = JSON.parse(message.data);
    if (data.id && pending.has(data.id)) { pending.get(data.id)(data); pending.delete(data.id); }
    if (data.method === 'Runtime.exceptionThrown') errors.push(data.params.exceptionDetails.exception?.description || data.params.exceptionDetails.text);
    if (data.method === 'Runtime.consoleAPICalled' && data.params.type === 'error') errors.push(data.params.args.map(a => a.value ?? a.description).join(' '));
  };
  const send = (method, params = {}) => new Promise(resolve => { const n = ++id; pending.set(n, resolve); ws.send(JSON.stringify({ id: n, method, params })); });
  const evaluate = async expression => (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.result.value;
  const close = async () => {
    ws.close();
    const exited = new Promise(resolve => proc.once('exit', resolve));
    proc.kill();
    await exited;
    try { fs.rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }); } catch { /* a leftover temp profile is harmless */ }
  };
  return { send, evaluate, errors, close };
}

const COVERAGE_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = () => new Promise(r => setTimeout(r, 500));
  const changed = (file, stop) => (api.diff[file]?.hunks || []).some(h => {
    let o = h.o, n = h.n;
    return h.l.some(l => { const hit = l[0] === '+' ? stop.side !== 'old' && n >= stop.start && n <= stop.end : l[0] === '-' ? stop.side === 'old' && o >= stop.start && o <= stop.end : false; if (l[0] !== '+') o++; if (l[0] !== '-') n++; return hit; });
  });
  const chapter = api.content.chapters.find(c => (c.stops || []).some(s => changed(s.file, s)));
  if (!chapter) return { skipped: 'no stop spans a changed line' };
  const stop = chapter.stops.find(s => changed(s.file, s));
  const forget = target => api.store.put(target, target.split(':')[0], { status: '' }, {});
  api.store.put('stop:' + stop.id, 'stop', { status: 'ok' }, {});
  api.openFile(stop.file, stop.start, stop.side || 'new');
  await settle();
  const bar = document.querySelector('[data-coverage-of="' + CSS.escape(stop.file) + '"]');
  const stopReachesFiles = !!bar && +bar.dataset.read > 0;
  forget('stop:' + stop.id);
  api.store.put('file:' + stop.file, 'file', { status: 'ok' }, {});
  location.hash = 'tour-' + chapter.id;
  await settle();
  const control = document.querySelector('.ctl[data-ctl="' + CSS.escape('stop:' + stop.id) + '"]');
  const fileReachesStops = !!control && control.classList.contains('via');
  forget('file:' + stop.file);
  return { stop: stop.id, stopReachesFiles, fileReachesStops };
})()`;

async function main() {
  const options = args(process.argv.slice(2));
  const chrome = findChrome(options.chrome);
  if (!chrome) { console.log(JSON.stringify({ skipped: 'no Chrome or Chromium found; set CHROME' })); return; }
  const page = path.resolve(options.page);
  const cdp = await devtools(chrome);
  const report = { page, routes: {}, overflow: [], errors: cdp.errors };
  try {
    await cdp.send('Runtime.enable');
    const viewport = width => cdp.send('Emulation.setDeviceMetricsOverride', { width, height: 1000, deviceScaleFactor: 1, mobile: width < 500 });
    await viewport(1440);
    await cdp.send('Page.navigate', { url: pathToFileURL(page).href + '#overview' });
    for (let i = 0; i < 50 && !(await cdp.evaluate('!!(window.ExplorePR && window.ExplorePR.routes().length)')); i++) await sleep(100);
    const routes = await cdp.evaluate('window.ExplorePR ? window.ExplorePR.routes() : []');
    if (options.shots) fs.mkdirSync(options.shots, { recursive: true });
    for (const route of routes) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(700);
      report.routes[route] = await cdp.evaluate(`({ heading: (document.querySelector('main h1') || {}).textContent || null, unresolvedSlices: document.querySelectorAll('[data-pending-slice]').length, highlighted: document.querySelectorAll('[class^="hljs-"]').length })`);
      if (options.shots) {
        const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
        fs.writeFileSync(path.join(options.shots, `${route}.png`), Buffer.from(shot.result.data, 'base64'));
      }
    }
    report.coverage = await cdp.evaluate(COVERAGE_PROBE);
    await viewport(400);
    for (const route of routes) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(400);
      const width = await cdp.evaluate('document.documentElement.scrollWidth');
      if (width > 400) report.overflow.push({ route, scrollWidth: width });
    }
    if (options.shots) {
      await cdp.evaluate("location.hash = 'overview'");
      await sleep(500);
      const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
      fs.writeFileSync(path.join(options.shots, 'overview-phone.png'), Buffer.from(shot.result.data, 'base64'));
    }
  } finally {
    await cdp.close();
  }
  const missing = Object.entries(report.routes).filter(([, r]) => !r.heading || r.unresolvedSlices).map(([route]) => route);
  const coverage = report.coverage || {};
  const coverageHolds = !!coverage.skipped || (coverage.stopReachesFiles && coverage.fileReachesStops);
  report.ok = !report.errors.length && !report.overflow.length && !missing.length && Object.keys(report.routes).length > 0 && coverageHolds;
  if (missing.length) report.incomplete = missing;
  console.log(JSON.stringify(report, null, 1));
  process.exitCode = report.ok ? 0 : 1;
}

main().catch(error => { console.error(error); process.exitCode = 2; });
