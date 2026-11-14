// Usage: node smoke.js PAGE.html [--shots DIR] [--chrome PATH]
//
// Opens a built explore-pr page from file:// in headless Chrome, visits every
// route the page registers, and fails on any uncaught error, unresolved code
// slice, or sideways scroll at phone width. It then reads a tour stop and checks
// that All files counts and marks its lines as read and the rest as unread, and
// marks that stop's file reviewed and checks the stop shows as already read.
// With commits, it marks a line in By commit and finds it read in All files,
// gives a file a verdict and finds its lines read in By commit, and marks one
// line at a time in All files, checking a mark link sits above each run of
// unread lines and that the n key jumps to the next unread line. Needs Node
// 22+ (global WebSocket). It unfolds a run of unchanged lines in a file
// opened by link and in one picked from the file list. Last, it renders prose
// naming a stop and its chapter, and checks both become links and that the
// stop's link opens its chapter at that stop. It checks that a stop names
// everything it carries, that each name opens what it names, and that every
// line a stop says to focus on is marked in its code. It also records a file verdict,
// pretends it was given at an earlier head, and checks that rewritten lines are
// unread again and announced, that untouched ones stay read, and that only a
// new verdict, not a note, moves the record to this head. Every diagram the
// kit drew is put through the kit's own checks: text inside its box, no box
// across another or across an arrow, tracing, and a linked box opening its link.

const { spawn, execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { DRAWING_CHECKS, SELF_DRAWN_CHECKS, TRACING_CHECK } = require('../../diagram-kit/lib/page-checks.js');

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
  const changedRows = document.querySelectorAll('#file-code tr.add, #file-code tr.del').length;
  const readRows = document.querySelectorAll('#file-code tr.add.read, #file-code tr.del.read').length;
  const unreadRows = document.querySelectorAll('#file-code tr.add.unread, #file-code tr.del.unread').length;
  const linesMarked = readRows > 0 && readRows + unreadRows === changedRows;
  forget('stop:' + stop.id);
  api.store.put('file:' + stop.file, 'file', { status: 'ok' }, {});
  location.hash = 'tour-' + chapter.id;
  await settle();
  const control = document.querySelector('.ctl[data-ctl="' + CSS.escape('stop:' + stop.id) + '"]');
  const fileReachesStops = !!control && control.classList.contains('via');
  forget('file:' + stop.file);
  return { stop: stop.id, stopReachesFiles, linesMarked, fileReachesStops };
})()`;

const COMMITS_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = () => new Promise(r => setTimeout(r, 600));
  if (!(api.content.commits || []).length) return { skipped: 'no commits' };
  for (let i = 0; i < 30 && !Object.keys(api.commits || {}).length; i++) await settle();
  const commits = Object.values(window.ExplorePR.api.commits);
  const row = commits.flatMap(c => window.ReviewCoverage.commitLines(c).filter(r => r.counted && api.diff[r.path]).map(r => ({ ...r, sha: c.sha })))[0];
  if (!row) return { skipped: 'no commit line in the whole diff' };
  const sel = (lp, lk) => 'tr[data-lp="' + CSS.escape(lp) + '"][data-lk="' + lk + '"]';
  const forget = target => api.store.put(target, target.split(':')[0], { status: '', lines: [] }, {});
  location.hash = 'commits'; await settle();
  document.querySelector('[data-commit="' + row.sha + '"]').click(); await settle();
  document.querySelector('#commit-code ' + sel(row.path, row.key) + ' td.ln').click(); await settle();
  api.openFile(row.path, +row.key.split(':')[1], row.key.startsWith('old') ? 'old' : 'new'); await settle();
  const commitReachesFiles = !!document.querySelector('#file-code ' + sel(row.path, row.key) + '.read');
  forget('lines:' + row.path);
  api.store.put('file:' + row.path, 'file', { status: 'ok' }, {});
  location.hash = 'commits'; await settle();
  document.querySelector('[data-commit="' + row.sha + '"]').click(); await settle();
  const filesReachCommits = !!document.querySelector('#commit-code ' + sel(row.path, row.key) + '.read');
  forget('file:' + row.path);
  api.openFile(row.path, null, 'new'); await settle();
  const before = document.querySelectorAll('#file-code tr.read').length;
  const unread = document.querySelector('#file-code tr.unread[data-lk] td.ln');
  if (unread) unread.click();
  await settle();
  const linesMarkOneByOne = document.querySelectorAll('#file-code tr.read').length === before + 1;
  const marked = [...document.querySelectorAll('#file-code tr[data-lk]')];
  const middle = marked.find((tr, i) => tr.classList.contains('unread') && marked[i - 1]?.classList.contains('unread') && marked[i + 1]?.classList.contains('unread') && tr.previousElementSibling === marked[i - 1] && tr.nextElementSibling === marked[i + 1]);
  if (middle) { middle.querySelector('td.ln').click(); await settle(); }
  const rows = [...document.querySelectorAll('#file-code tr')];
  const unreadRun = i => rows[i].matches('tr.unread[data-lk]');
  const runStarts = rows.map((_, i) => i).filter(i => unreadRun(i) && !(i > 0 && (unreadRun(i - 1) || rows[i - 1].matches('tr.anno') && i > 1 && unreadRun(i - 2))));
  const linkAboveEachUnreadRun = !!middle && runStarts.every(i => rows[i - 1] && rows[i - 1].matches('tr.blk'))
    && document.querySelectorAll('#file-code tr.blk').length === runStarts.length;
  document.querySelectorAll('tr.flash').forEach(tr => tr.classList.remove('flash'));
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true }));
  await new Promise(r => setTimeout(r, 50));
  const nextUnreadKey = !!document.querySelector('#file-code tr.unread.flash');
  forget('lines:' + row.path);
  return { sha: row.sha, line: row.path + ' ' + row.key, commitReachesFiles, filesReachCommits, linesMarkOneByOne, linkAboveEachUnreadRun, nextUnreadKey };
})()`;

const REFERENCES_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const api = window.ExplorePR.api;
  const chapters = api.content.chapters || [];
  const last = [...chapters].reverse().find(chapter => (chapter.stops || []).length);
  if (!last) return { skipped: 'no chapter with a stop' };
  const stop = last.stops[last.stops.length - 1];
  location.hash = 'overview'; await settle();
  const holder = document.createElement('p');
  holder.innerHTML = api.md('See \`' + stop.id + '\` in chapter \`' + last.id + '\`.', true);
  document.querySelector('#main .view').prepend(holder);
  const [toStop, toChapter] = holder.querySelectorAll('a.ref');
  const named = !!toStop && !!toChapter && !holder.querySelector('code') && toStop.textContent.includes(stop.title || stop.id) && toChapter.textContent === (last.title || last.id);
  if (!named) return { named, html: holder.innerHTML };
  toStop.click(); await settle();
  const target = document.getElementById('stop-' + stop.id);
  const top = target ? target.getBoundingClientRect().top : null;
  return { named, opensChapter: location.hash === '#tour-' + last.id, reachesStop: top !== null && top >= -2 && top < innerHeight };
})()`;

const EVIDENCE_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const api = window.ExplorePR.api;
  const chapters = api.content.chapters || [];
  const evidence = window.StopEvidence.index(api.content);
  const chapter = chapters.find(each => (each.stops || []).some(stop => evidence.carriedBy(stop).length));
  if (!chapter) return { skipped: 'no stop carries anything the page declares' };
  const stop = chapter.stops.find(each => evidence.carriedBy(each).length);
  location.hash = 'tour-' + chapter.id; await settle();
  const shown = document.getElementById('stop-' + stop.id);
  const links = shown ? [...shown.querySelectorAll('a.carried')] : [];
  const named = links.length === evidence.carriedBy(stop).length;
  const pointed = chapters.flatMap(each => each.stops || []).filter(each => (each.focus || []).some(focus => focus.why));
  const focusMarked = pointed.every(each => {
    const there = document.getElementById('stop-' + each.id);
    return !there || !there.querySelector('.code table') || !!there.querySelector('tr.focus + tr.point');
  });
  if (!named) return { named, focusMarked };
  links[0].click(); await settle();
  const target = document.getElementById(links[0].dataset.scrollTo);
  const top = target ? target.getBoundingClientRect().top : null;
  return { stop: stop.id, named, focusMarked, reachesWhatItCarries: top !== null && top >= -2 && top < innerHeight };
})()`;

const HEADS_PROBE = `(async () => {
  const api = window.ExplorePR.api, store = api.store, content = api.content;
  const settle = () => new Promise(r => setTimeout(r, 500));
  const file = content.files.find(f => window.ReviewCoverage.changedLinesOf((api.diff || {})[f.path]).length);
  if (!file || !content.pr.head) return { skipped: 'no head, or no file with a changed line' };
  const path = file.path, target = 'file:' + path, earlier = 'earlier-head';
  const bar = () => document.querySelector('[data-coverage-of="' + CSS.escape(path) + '"]');
  const since = () => document.querySelector('.ctl[data-ctl="' + CSS.escape(target) + '"] [data-since-note]');
  const recordedAt = (head, lineage) => {
    content.lineage = { ...(content.lineage || {}), [earlier]: { [path]: lineage } };
    store.docs.set(target, { ...store.get(target), head });
    store.version++;
    api.rerender();
  };
  store.put(target, 'file', { status: 'ok' }, {});
  const saved = store.get(target);
  const stamped = saved.head === content.pr.head && Array.isArray(saved.claimed) && saved.claimed.length > 0;
  api.openFile(path); await settle();
  const readAtHead = +bar().dataset.read;
  recordedAt(earlier, { new: [], old: [], changed: true }); api.openFile(path); await settle();
  const rewrittenUnread = readAtHead > 0 && +bar().dataset.read === 0;
  const marked = !!since() && !since().hidden && since().textContent.includes(earlier.slice(0, 7));
  const banner = document.querySelector('#store-banner');
  const announced = !!banner && !banner.hidden && /moved/.test(banner.textContent);
  recordedAt(earlier, 'same'); api.openFile(path); await settle();
  const untouchedCarried = +bar().dataset.read === readAtHead && !!since() && since().hidden;
  store.put(target, 'file', { note: 'checked' }, {});
  const noteKeepsHead = store.get(target).head === earlier;
  store.put(target, 'file', { status: 'issue' }, {});
  const verdictRestamps = store.get(target).head === content.pr.head;
  store.put(target, 'file', { status: '', note: '' }, {});
  delete content.lineage[earlier];
  return { stamped, rewrittenUnread, marked, announced, untouchedCarried, noteKeepsHead, verdictRestamps };
})()`;

const DIAGRAMS_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const seen = {};
  let tracing = { skipped: 'no diagram with an arrow' };
  for (const route of [...window.ExplorePR.figureViews(), ...window.ExplorePR.customViews()]) {
    location.hash = route; await settle();
    for (let waited = 0; waited < 40 && document.querySelector('[data-drawing]'); waited++) await settle();
    if (document.querySelector('[data-drawing]')) { seen[route] = ['a diagram was still being drawn after 12 seconds']; continue; }
    if (!document.querySelector('svg.dk, .dk-drawn')) continue;
    const checked = await ${DRAWING_CHECKS};
    [...checked.diagrams, ...${SELF_DRAWN_CHECKS}].forEach(each => { seen[each.id] = each.problems; });
    const traced = await ${TRACING_CHECK};
    if (!traced.skipped) tracing = traced;
    const link = document.querySelector('svg.dk .node-link');
    if (link && !('linkOpens' in seen)) {
      const before = location.hash;
      link.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      await settle();
      seen.linkOpens = location.hash !== before || !!document.querySelector('#file-pane');
    }
  }
  const { linkOpens = null, ...problems } = seen;
  if (!Object.keys(problems).length) return { skipped: 'no kit diagram on the page' };
  return { problems, tracing, linkOpens };
})()`;

const UNFOLD_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = () => new Promise(r => setTimeout(r, 500));
  const folded = api.content.files.find(f => f.status !== 'added' && window.ReviewCoverage && api.diff[f.path] && (api.diff[f.path].hunks[0]?.l.length || 0) > 30);
  if (!folded) return { skipped: 'no file long enough to fold' };
  const unfolds = async () => {
    const button = document.querySelector('#file-code [data-unfold]');
    if (!button) return null;
    const before = document.querySelectorAll('#file-code tr').length;
    button.click(); await settle();
    return document.querySelectorAll('#file-code tr').length > before;
  };
  api.openFile(folded.path, null, 'new'); await settle();
  const afterOpening = await unfolds();
  const other = api.content.files.find(f => f.path !== folded.path);
  if (other) { document.querySelector('.fl-item[data-file="' + CSS.escape(other.path) + '"]').click(); await settle(); }
  document.querySelector('.fl-item[data-file="' + CSS.escape(folded.path) + '"]').click(); await settle();
  const afterPicking = await unfolds();
  return { file: folded.path, afterOpening, afterPicking };
})()`;

async function wholePage(cdp) {
  const { result: { cssContentSize: size } } = await cdp.send('Page.getLayoutMetrics');
  return { format: 'png', captureBeyondViewport: true, clip: { x: 0, y: 0, width: size.width, height: size.height, scale: 1 } };
}

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
    const figureViews = await cdp.evaluate('window.ExplorePR.figureViews()');
    const chapters = figureViews.filter(view => !routes.includes(view));
    for (const route of [...routes, ...chapters]) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(700);
      report.routes[route] = await cdp.evaluate(`({ heading: (document.querySelector('main h1') || {}).textContent || null, unresolvedSlices: document.querySelectorAll('[data-pending-slice]').length, highlighted: document.querySelectorAll('[class^="hljs-"]').length })`);
      if (options.shots) {
        const shot = await cdp.send('Page.captureScreenshot', figureViews.includes(route) ? await wholePage(cdp) : { format: 'png' });
        fs.writeFileSync(path.join(options.shots, `${route}.png`), Buffer.from(shot.result.data, 'base64'));
      }
    }
    report.coverage = await cdp.evaluate(COVERAGE_PROBE);
    report.commits = await cdp.evaluate(COMMITS_PROBE);
    report.unfold = await cdp.evaluate(UNFOLD_PROBE);
    report.references = await cdp.evaluate(REFERENCES_PROBE);
    report.evidence = await cdp.evaluate(EVIDENCE_PROBE);
    report.heads = await cdp.evaluate(HEADS_PROBE);
    report.diagrams = await cdp.evaluate(DIAGRAMS_PROBE);
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
  const coverageHolds = !!coverage.skipped || (coverage.stopReachesFiles && coverage.linesMarked && coverage.fileReachesStops);
  const commits = report.commits || {};
  const commitsHold = !!commits.skipped || !!(commits.commitReachesFiles && commits.filesReachCommits && commits.linesMarkOneByOne && commits.linkAboveEachUnreadRun && commits.nextUnreadKey);
  const unfold = report.unfold || {};
  const unfoldHolds = !!unfold.skipped || (unfold.afterOpening !== false && unfold.afterPicking !== false && (unfold.afterOpening || unfold.afterPicking) !== null);
  const references = report.references || {};
  const referencesHold = !!references.skipped || !!(references.named && references.opensChapter && references.reachesStop);
  const evidence = report.evidence || {};
  const evidenceHolds = !!evidence.skipped || !!(evidence.named && evidence.focusMarked && evidence.reachesWhatItCarries);
  const heads = report.heads || {};
  const headsHold = !!heads.skipped || !!(heads.stamped && heads.rewrittenUnread && heads.marked && heads.announced && heads.untouchedCarried && heads.noteKeepsHead && heads.verdictRestamps);
  const diagrams = report.diagrams || {};
  const tracesHold = !diagrams.tracing || !!diagrams.tracing.skipped || !!(diagrams.tracing.traces && diagrams.tracing.clears);
  const diagramsHold = !!diagrams.skipped || (Object.values(diagrams.problems || { none: ['not checked'] }).every(found => !found.length) && tracesHold && diagrams.linkOpens !== false);
  report.ok = diagramsHold && headsHold && evidenceHolds && referencesHold && commitsHold && unfoldHolds && !report.errors.length && !report.overflow.length && !missing.length && Object.keys(report.routes).length > 0 && coverageHolds;
  if (missing.length) report.incomplete = missing;
  console.log(JSON.stringify(report, null, 1));
  process.exitCode = report.ok ? 0 : 1;
}

main().catch(error => { console.error(error); process.exitCode = 2; });
